//! Web Bluetooth transport. Requires Chrome/Edge (desktop or Android) over
//! HTTPS or http://localhost. Safari and iOS browsers do not implement it.

use std::cell::Cell;
use std::rc::Rc;

use futures::FutureExt;
use futures::lock::Mutex;
use gloo_timers::future::TimeoutFuture;
use js_sys::{DataView, JsString, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    BluetoothDevice, BluetoothLeScanFilterInit, BluetoothRemoteGattCharacteristic, RequestDeviceOptions,
};

use crate::protocol::{self, Frame};

fn uuid16(short: u16) -> String {
    format!("0000{short:04x}-0000-1000-8000-00805f9b34fb")
}

struct Uuids {
    write_service: u16,
    write_char: u16,
    notify_service: u16,
    notify_char: u16,
}

const MAIN: Uuids = Uuids { write_service: 0xFFF0, write_char: 0xFFF2, notify_service: 0xFFF0, notify_char: 0xFFF1 };
const ALT: Uuids = Uuids { write_service: 0xFFE5, write_char: 0xFFE9, notify_service: 0xFFE0, notify_char: 0xFFE4 };

fn js_err(e: JsValue) -> String {
    e.dyn_ref::<js_sys::Error>()
        .map(|e| String::from(e.message()))
        .or_else(|| e.as_string())
        .unwrap_or_else(|| format!("{e:?}"))
}

const CONNECT_ATTEMPTS: u32 = 3;
const CONNECT_TIMEOUT_MS: u32 = 15_000;
const NOTIFY_TIMEOUT_MS: u32 = 4_000;
/// A write that has not settled by then is reported as failed instead of blocking
/// every later write (they are serialized).
const WRITE_TIMEOUT_MS: u32 = 3_000;
/// How long a write keeps retrying while another GATT operation is in progress,
/// starting with short waits.
const BUSY_WAIT_MS: u32 = 5_000;
const BUSY_STEP_MS: u32 = 25;

/// Chrome's error when a characteristic is asked to start an operation while
/// the previous one has not finished ("GATT operation already in progress.").
fn is_busy(e: &str) -> bool {
    e.contains("already in progress")
}

/// Connects GATT and looks up the write and notify characteristics.
async fn open_gatt(
    device: &BluetoothDevice,
    uuids: &Uuids,
    log: &impl Fn(String),
) -> Result<(BluetoothRemoteGattCharacteristic, BluetoothRemoteGattCharacteristic), String> {
    let server = device.gatt().ok_or("device has no GATT server")?;
    let server: web_sys::BluetoothRemoteGattServer =
        server.connect().await.map_err(|e| format!("GATT connect: {}", js_err(e)))?;
    log("GATT connected; looking up services".into());
    let step = |what: &str, e: JsValue| format!("{what}: {}", js_err(e));
    let wsvc: web_sys::BluetoothRemoteGattService = server
        .get_primary_service_with_str(&uuid16(uuids.write_service))
        .await
        .map_err(|e| step(&format!("service {:04X}", uuids.write_service), e))?;
    let write: BluetoothRemoteGattCharacteristic = wsvc
        .get_characteristic_with_str(&uuid16(uuids.write_char))
        .await
        .map_err(|e| step(&format!("characteristic {:04X}", uuids.write_char), e))?;
    let nsvc: web_sys::BluetoothRemoteGattService = server
        .get_primary_service_with_str(&uuid16(uuids.notify_service))
        .await
        .map_err(|e| step(&format!("service {:04X}", uuids.notify_service), e))?;
    let notify: BluetoothRemoteGattCharacteristic = nsvc
        .get_characteristic_with_str(&uuid16(uuids.notify_char))
        .await
        .map_err(|e| step(&format!("characteristic {:04X}", uuids.notify_char), e))?;
    Ok((write, notify))
}

/// Awaits a write, giving up after [`WRITE_TIMEOUT_MS`].
async fn with_timeout<T>(p: impl std::future::IntoFuture<Output = Result<T, JsValue>>) -> Result<(), String> {
    let write = p.into_future().map(|r| r.map(|_| ()).map_err(js_err));
    let timeout = TimeoutFuture::new(WRITE_TIMEOUT_MS)
        .map(|_| Err(format!("no reply from the machine within {} s", WRITE_TIMEOUT_MS / 1000)));
    futures::select! {
        r = write.fuse() => r,
        r = timeout.fuse() => r,
    }
}

pub struct Machine {
    pub name: String,
    device: BluetoothDevice,
    write: BluetoothRemoteGattCharacteristic,
    lock: Mutex<()>,
    with_response: Cell<bool>,
    can_write_without_response: bool,
    _on_notify: Closure<dyn FnMut(web_sys::Event)>,
    _on_disconnect: Closure<dyn FnMut(web_sys::Event)>,
}

impl Machine {
    /// Connects to a machine: to `device` when given (a reboot of the link to the same
    /// machine), otherwise to the one picked in the browser's device chooser.
    /// `log` receives each step, so a failure on the phone shows where it stopped.
    pub async fn connect(
        device: Option<BluetoothDevice>,
        on_notify: impl Fn(Vec<u8>) + 'static,
        on_disconnect: impl Fn() + 'static,
        log: impl Fn(String),
    ) -> Result<Rc<Machine>, String> {
        let device = match device {
            Some(d) => d,
            None => {
                let bluetooth = web_sys::window()
                    .and_then(|w| w.navigator().bluetooth())
                    .ok_or("This browser has no Web Bluetooth. Use Chrome or Edge on desktop or Android.")?;

                let filter = BluetoothLeScanFilterInit::new();
                filter.set_name_prefix("PT");
                let options = RequestDeviceOptions::new();
                options.set_filters(&[filter]);
                let services: Vec<JsString> =
                    [0xFFF0, 0xFFE0, 0xFFE5].iter().map(|s| JsString::from(uuid16(*s))).collect();
                options.set_optional_services(&services);

                log("Opening device chooser".into());
                bluetooth.request_device(&options).await.map_err(js_err)?
            }
        };
        let name = device.name().unwrap_or_default();
        let uuids = if protocol::uses_alt_services(&name) { ALT } else { MAIN };
        log(format!(
            "Selected {name}; using services {:04X}/{:04X}",
            uuids.write_service, uuids.notify_service
        ));

        // Android often fails the first GATT connection right after the chooser closes
        // (status 133), and a connect can hang without ever settling, so retry with a timeout.
        let mut last_err = String::new();
        let mut chars = None;
        for attempt in 1..=CONNECT_ATTEMPTS {
            log(format!("Connecting (attempt {attempt}/{CONNECT_ATTEMPTS})"));
            let timeout = TimeoutFuture::new(CONNECT_TIMEOUT_MS).map(|_| Err(format!("timed out after {} s", CONNECT_TIMEOUT_MS / 1000)));
            let result = futures::select! {
                r = open_gatt(&device, &uuids, &log).fuse() => r,
                r = timeout.fuse() => r,
            };
            match result {
                Ok(c) => {
                    chars = Some(c);
                    break;
                }
                Err(e) => {
                    log(format!("Attempt {attempt} failed: {e}"));
                    last_err = e;
                    if let Some(s) = device.gatt() {
                        s.disconnect();
                    }
                    if attempt < CONNECT_ATTEMPTS {
                        TimeoutFuture::new(1000).await;
                    }
                }
            }
        }
        let (write, notify) = chars.ok_or_else(|| format!("Could not connect to {name}: {last_err}"))?;

        let on_notify = Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| {
            let Some(c) = e.target().and_then(|t| t.dyn_into::<BluetoothRemoteGattCharacteristic>().ok()) else {
                return;
            };
            if let Some(v) = c.value() {
                let v: DataView = v;
                on_notify(Uint8Array::new_with_byte_offset_and_length(&v.buffer(), v.byte_offset() as u32, v.byte_length() as u32).to_vec());
            }
        });
        notify.set_oncharacteristicvaluechanged(Some(on_notify.as_ref().unchecked_ref()));
        log("Starting notifications".into());
        // On Android, Chrome can wait forever for the machine to confirm the notification
        // subscription. Commands do not depend on it (only battery and fault reports do),
        // so stop waiting after a few seconds and keep the connection.
        let started = JsFuture::from(notify.start_notifications()).map(|r| match r {
            Ok(_) => "Notifications on".to_string(),
            Err(e) => format!("Notifications failed: {}; battery and fault reports unavailable", js_err(e)),
        });
        let timeout = TimeoutFuture::new(NOTIFY_TIMEOUT_MS).map(|_| {
            format!(
                "No reply when starting notifications after {} s; continuing without battery and fault reports",
                NOTIFY_TIMEOUT_MS / 1000
            )
        });
        let line = futures::select! {
            l = started.fuse() => l,
            l = timeout.fuse() => l,
        };
        log(line);

        // Prefer writes without response: some machines never confirm a write with
        // response on Android, which used to leave every later command waiting.
        let props = write.properties();
        let (with, without) = (props.write(), props.write_without_response());
        log(format!(
            "Write supports: with response {}, without response {}",
            if with { "yes" } else { "no" },
            if without { "yes" } else { "no" }
        ));

        let on_disconnect = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| on_disconnect());
        device.set_ongattserverdisconnected(Some(on_disconnect.as_ref().unchecked_ref()));

        Ok(Rc::new(Machine {
            name,
            device,
            write,
            lock: Mutex::new(()),
            with_response: Cell::new(!without),
            can_write_without_response: without,
            _on_notify: on_notify,
            _on_disconnect: on_disconnect,
        }))
    }

    /// Writes one frame. Writes are serialized; GATT rejects overlapping operations.
    pub async fn send(&self, f: &Frame) -> Result<(), String> {
        let _guard = self.lock.lock().await;
        if self.with_response.get() {
            match self.write_retrying(f, true).await {
                Ok(()) => return Ok(()),
                Err(e) if self.can_write_without_response => {
                    // Retry this frame without response and keep using that from now on.
                    self.with_response.set(false);
                    return self.write_retrying(f, false).await.map_err(|e2| format!("{e}; without response: {e2}"));
                }
                Err(e) => return Err(e),
            }
        }
        self.write_retrying(f, false).await
    }

    /// Writes a frame, waiting and trying again while the browser reports that another
    /// GATT operation is still running. That happens when an earlier operation outlived
    /// its timeout here (a write, or the notification start on Android) but has not
    /// settled in the browser yet, or while Android is still busy with the connection.
    async fn write_retrying(&self, f: &Frame, with_response: bool) -> Result<(), String> {
        let mut waited = 0;
        loop {
            let p = if with_response {
                self.write.write_value_with_response_with_u8_slice(f)
            } else {
                self.write.write_value_without_response_with_u8_slice(f)
            };
            let result = match p {
                Ok(p) => with_timeout(p).await,
                Err(e) => Err(js_err(e)),
            };
            match result {
                Err(e) if is_busy(&e) && waited < BUSY_WAIT_MS => {
                    let step = (BUSY_STEP_MS * (1 + waited / 500)).min(250);
                    TimeoutFuture::new(step).await;
                    waited += step;
                }
                r => return r,
            }
        }
    }

    /// Drops the link for a reboot: without the disconnect handler, so the app does not
    /// treat it as the machine going away, and hands back the device to connect again.
    pub fn close_for_reboot(&self) -> BluetoothDevice {
        self.device.set_ongattserverdisconnected(None);
        self.disconnect();
        self.device.clone()
    }

    pub fn disconnect(&self) {
        if let Some(s) = self.device.gatt() {
            s.disconnect();
        }
    }
}
