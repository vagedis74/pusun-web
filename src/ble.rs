//! Web Bluetooth transport. Requires Chrome/Edge (desktop or Android) over
//! HTTPS or http://localhost. Safari and iOS browsers do not implement it.

use std::cell::Cell;
use std::rc::Rc;

use futures::lock::Mutex;
use js_sys::{DataView, JsString, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
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

pub struct Machine {
    pub name: String,
    device: BluetoothDevice,
    write: BluetoothRemoteGattCharacteristic,
    lock: Mutex<()>,
    with_response: Cell<bool>,
    _on_notify: Closure<dyn FnMut(web_sys::Event)>,
    _on_disconnect: Closure<dyn FnMut(web_sys::Event)>,
}

impl Machine {
    /// Shows the browser's device picker and connects to the chosen machine.
    pub async fn connect(
        on_notify: impl Fn(Vec<u8>) + 'static,
        on_disconnect: impl Fn() + 'static,
    ) -> Result<Rc<Machine>, String> {
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

        let device: BluetoothDevice = bluetooth.request_device(&options).await.map_err(js_err)?;
        let name = device.name().unwrap_or_default();
        let uuids = if protocol::uses_alt_services(&name) { ALT } else { MAIN };

        let server = device.gatt().ok_or("device has no GATT server")?;
        let server: web_sys::BluetoothRemoteGattServer = server.connect().await.map_err(js_err)?;

        let wsvc: web_sys::BluetoothRemoteGattService =
            server.get_primary_service_with_str(&uuid16(uuids.write_service)).await.map_err(js_err)?;
        let write: BluetoothRemoteGattCharacteristic =
            wsvc.get_characteristic_with_str(&uuid16(uuids.write_char)).await.map_err(js_err)?;
        let nsvc: web_sys::BluetoothRemoteGattService =
            server.get_primary_service_with_str(&uuid16(uuids.notify_service)).await.map_err(js_err)?;
        let notify: BluetoothRemoteGattCharacteristic =
            nsvc.get_characteristic_with_str(&uuid16(uuids.notify_char)).await.map_err(js_err)?;

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
        let _: BluetoothRemoteGattCharacteristic = notify.start_notifications().await.map_err(js_err)?;

        let on_disconnect = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| on_disconnect());
        device.set_ongattserverdisconnected(Some(on_disconnect.as_ref().unchecked_ref()));

        Ok(Rc::new(Machine {
            name,
            device,
            write,
            lock: Mutex::new(()),
            with_response: Cell::new(true),
            _on_notify: on_notify,
            _on_disconnect: on_disconnect,
        }))
    }

    /// Writes one frame. Writes are serialized; GATT rejects overlapping operations.
    pub async fn send(&self, f: &Frame) -> Result<(), String> {
        let _guard = self.lock.lock().await;
        if self.with_response.get() {
            match self.write.write_value_with_response_with_u8_slice(f) {
                Ok(p) => match p.await {
                    Ok(_) => return Ok(()),
                    Err(e) if js_err(e.clone()).contains("not supported") => self.with_response.set(false),
                    Err(e) => return Err(js_err(e)),
                },
                Err(_) => self.with_response.set(false),
            }
        }
        let p = self.write.write_value_without_response_with_u8_slice(f).map_err(js_err)?;
        p.await.map(|_| ()).map_err(js_err)
    }


    pub fn disconnect(&self) {
        if let Some(s) = self.device.gatt() {
            s.disconnect();
        }
    }
}
