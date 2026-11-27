use crate::ui::{self, Help, tr};
use std::rc::Rc;

use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::ble::Machine;
use crate::builder::{self, DrillBuilder};
use crate::courses::Courses;
use crate::drills::{self, DIFFICULTIES, Drill, LIBRARY};
use crate::protocol::{self as p, AiOptions, CustomPoint, DrillPause, DrillPoint, Frame, Height, Notification, Pattern, PatternDefault, ProgramSettings, RawPoint, Sport};

/// A frame and the delay before sending it, mirroring the app's `postDelayed` gaps.
type Step = (u32, Frame);

const CUSTOM_KEY: &str = "pusun-web.custom-points";
const ROUTES_KEY: &str = "pusun-web.routes";
const DEFAULTS_KEY: &str = "pusun-web.pattern-defaults";
const ADVANCED_KEY: &str = "pusun-web.advanced";
const SPORT_KEY: &str = "pusun-web.sport";
/// Renamed when the default became 5, so a level chosen under the old default of 3
/// does not keep the app below the new default.
const DIFFICULTY_KEY: &str = "pusun-web.drill-difficulty.v2";
const PADEL_CUSTOM_KEY: &str = "pusun-web.padel.custom-points";
/// Bluetooth id of the machine used last, to find it again after a reload.
const DEVICE_KEY: &str = "pusun-web.device-id";
/// How long the link may stay idle before a keepalive.
const KEEPALIVE_MS: u32 = 15_000;
/// How often the watchdog looks at the link. While the machine is not answering it
/// probes at this pace, so a dead link is restarted within about 15–20 s.
const WATCHDOG_MS: u32 = 5_000;
/// Unanswered keepalives in a row after which the link counts as dead.
const MAX_MISSED: u32 = 3;

/// Sleeps `ms`, or less when `poke` is set (which it clears). Returns whether poked.
async fn nap(poke: StoredValue<bool>, ms: u32) -> bool {
    let mut left = ms;
    while left > 0 {
        if poke.get_value() {
            poke.set_value(false);
            return true;
        }
        let step = left.min(250);
        TimeoutFuture::new(step).await;
        left -= step;
    }
    false
}

fn custom_key(sport: Sport) -> &'static str {
    match sport {
        Sport::Tennis => CUSTOM_KEY,
        Sport::Padel => PADEL_CUSTOM_KEY,
    }
}

/// Saved defaults are kept apart per screen. The app shares one table between its
/// tennis and padel screens, which would apply tennis aims to padel and vice versa.
fn default_key(sport: Sport, pat: &Pattern) -> Option<String> {
    let k = pat.default_key()?;
    Some(match sport {
        Sport::Tennis => k,
        Sport::Padel => format!("padel:{k}"),
    })
}

/// Saved per-pattern defaults, keyed like the app (`fix1`, `lr3`, `moon`, ...).
fn load_defaults() -> Vec<(String, PatternDefault)> {
    let Some(s) = storage().and_then(|s| s.get_item(DEFAULTS_KEY).ok().flatten()) else { return Vec::new() };
    s.lines()
        .filter_map(|line| {
            let (key, vals) = line.split_once('=')?;
            let v: Vec<i32> = vals.split(',').filter_map(|x| x.parse().ok()).collect();
            let [lr, ud, freq, speed, spin, level] = v[..] else { return None };
            Some((
                key.to_string(),
                PatternDefault {
                    lr: (lr as u16).clamp(p::LR_MIN, p::LR_MAX),
                    ud: (ud as u16).min(p::UD_MAX),
                    freq: (freq as u8).clamp(1, 10),
                    speed: (speed as u8).clamp(1, 13),
                    spin: (spin as i8).clamp(-p::SPIN_MAX, p::SPIN_MAX),
                    height_level: if level == 0 { 0 } else { (level as u8).clamp(p::HEIGHT_LEVEL_MIN, p::PADEL_HEIGHT_LEVEL_MAX) },
                },
            ))
        })
        .collect()
}

fn save_defaults(d: &[(String, PatternDefault)]) {
    let s = d
        .iter()
        .map(|(k, v)| format!("{k}={},{},{},{},{},{}", v.lr, v.ud, v.freq, v.speed, v.spin, v.height_level))
        .collect::<Vec<_>>()
        .join("\n");
    if let Some(st) = storage() {
        let _ = st.set_item(DEFAULTS_KEY, &s);
    }
}

/// A saved Vary Speed drill: name, point order, pause and each shot's point parameters.
#[derive(Clone, Debug, PartialEq)]
struct Route {
    name: String,
    points: Vec<u8>,
    pause: DrillPause,
    params: Vec<(u8, DrillPoint)>,
}

fn pause_code(p: DrillPause) -> u8 {
    match p {
        DrillPause::None => 0,
        DrillPause::Short => 1,
        DrillPause::Long => 2,
    }
}

fn pause_from(c: &str) -> DrillPause {
    match c {
        "1" | "short" => DrillPause::Short,
        "2" | "long" => DrillPause::Long,
        _ => DrillPause::None,
    }
}

const PROGRAMS_KEY: &str = "pusun-web.programs";
const CUSTOM_SETS_KEY: &str = "pusun-web.custom-sets";

/// Named records stored one per line as `name<TAB>fields`.
pub(crate) fn load_named(key: &str) -> Vec<(String, String)> {
    let Some(s) = storage().and_then(|s| s.get_item(key).ok().flatten()) else { return Vec::new() };
    s.lines().filter_map(|l| l.split_once('\t')).map(|(n, v)| (n.to_string(), v.to_string())).collect()
}

pub(crate) fn save_named(key: &str, items: impl Iterator<Item = (String, String)>) {
    let s = items.map(|(n, v)| format!("{n}\t{v}")).collect::<Vec<_>>().join("\n");
    if let Some(st) = storage() {
        let _ = st.set_item(key, &s);
    }
}

fn nums<T: std::str::FromStr>(s: &str) -> Vec<T> {
    s.split(',').filter_map(|x| x.trim().parse().ok()).collect()
}

fn join<T: ToString>(v: &[T]) -> String {
    v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}

fn load_routes() -> Vec<Route> {
    load_named(ROUTES_KEY)
        .into_iter()
        .filter_map(|(name, v)| {
            let f: Vec<&str> = v.split('\t').collect();
            let [pts, pause, params] = f[..] else { return None };
            let points = nums::<u8>(pts).into_iter().filter(|x| (1..=28).contains(x)).collect();
            let params = params
                .split(';')
                .filter_map(|r| {
                    let [lr, ud, speed, interval, spin, id] = nums::<u16>(r)[..] else { return None };
                    (1..=28).contains(&id).then(|| {
                        (id as u8, DrillPoint { lr, ud, speed: speed as u8, interval: interval as u8, spin: spin as u8 }.clamped())
                    })
                })
                .collect();
            Some(Route { name, points, pause: pause_from(pause), params })
        })
        .collect()
}

fn save_routes(routes: &[Route]) {
    save_named(
        ROUTES_KEY,
        routes.iter().map(|r| {
            let params = r
                .params
                .iter()
                .map(|(id, d)| format!("{},{},{},{},{},{id}", d.lr, d.ud, d.speed, d.interval, d.spin))
                .collect::<Vec<_>>()
                .join(";");
            (r.name.clone(), format!("{}\t{}\t{params}", join(&r.points), pause_code(r.pause)))
        }),
    );
}

/// A named Program-panel program (`SeleDao`).
#[derive(Clone, Debug, PartialEq)]
struct SavedProgram {
    name: String,
    points: Vec<u8>,
    settings: ProgramSettings,
}

fn height_code(h: Height) -> u8 {
    match h {
        Height::Low => 0,
        Height::Mid => 1,
        Height::High => 2,
    }
}

fn load_programs() -> Vec<SavedProgram> {
    load_named(PROGRAMS_KEY)
        .into_iter()
        .filter_map(|(name, v)| {
            let (pts, rest) = v.split_once('\t')?;
            let [freq, height, level, speed, spin] = nums::<i32>(rest)[..] else { return None };
            Some(SavedProgram {
                name,
                points: nums::<u8>(pts).into_iter().filter(|x| (1..=p::ALL_POINTS).contains(x)).take(p::MAX_SEQUENCE).collect(),
                settings: ProgramSettings {
                    freq: (freq as u8).clamp(1, 10),
                    speed: (speed as u8).clamp(1, 13),
                    spin: (spin as i8).clamp(-p::SPIN_MAX, p::SPIN_MAX),
                    height: match height { 0 => Height::Low, 2 => Height::High, _ => Height::Mid },
                    height_level: if level == 0 { 0 } else { (level as u8).clamp(p::HEIGHT_LEVEL_MIN, p::PADEL_HEIGHT_LEVEL_MAX) },
                },
            })
        })
        .collect()
}

fn save_programs(list: &[SavedProgram]) {
    save_named(
        PROGRAMS_KEY,
        list.iter().map(|x| {
            let s = x.settings;
            (x.name.clone(), format!("{}\t{},{},{},{},{}", join(&x.points), s.freq, height_code(s.height), s.height_level, s.speed, s.spin))
        }),
    );
}

/// A named custom-point set (`VariDao`): play order and all six points.
#[derive(Clone, Debug, PartialEq)]
struct CustomSet {
    name: String,
    order: Vec<u8>,
    points: [CustomPoint; p::CUSTOM_POINTS],
}

fn load_custom_sets() -> Vec<CustomSet> {
    load_named(CUSTOM_SETS_KEY)
        .into_iter()
        .filter_map(|(name, v)| {
            let (order, rest) = v.split_once('\t')?;
            let mut points = CustomPoint::defaults(Sport::Tennis);
            for (i, row) in rest.split(';').take(p::CUSTOM_POINTS).enumerate() {
                let [lr, ud, speed, interval] = nums::<u16>(row)[..] else { return None };
                points[i] = CustomPoint { lr, ud, speed, interval: interval as u8 }.clamped();
            }
            let order = nums::<u8>(order).into_iter().filter(|x| (1..=6).contains(x)).take(p::MAX_SEQUENCE).collect();
            Some(CustomSet { name, order, points })
        })
        .collect()
}

fn save_custom_sets(list: &[CustomSet]) {
    save_named(
        CUSTOM_SETS_KEY,
        list.iter().map(|x| {
            let pts = x.points.iter().map(|c| format!("{},{},{},{}", c.lr, c.ud, c.speed, c.interval)).collect::<Vec<_>>().join(";");
            (x.name.clone(), format!("{}\t{pts}", join(&x.order)))
        }),
    );
}

/// Asks for a name; tabs and newlines are replaced because they separate stored fields.
fn prompt_name(label: &str) -> Option<String> {
    let name = web_sys::window()?.prompt_with_message(label).ok()??;
    let name = name.trim().replace(['\t', '\n', '\r'], " ");
    (!name.is_empty()).then_some(name)
}


#[derive(Clone, Copy)]
struct Ctx {
    machine: StoredValue<Option<Rc<Machine>>, LocalStorage>,
    name: RwSignal<Option<String>>,
    log: RwSignal<Vec<String>>,
    error: RwSignal<Option<String>>,
    /// True from a successful connect until Disconnect: while set, a lost link is
    /// connected again on its own and commands wait in `pending` instead of being lost.
    keep: StoredValue<bool>,
    /// Commands not sent yet because the link was down, and when the first was queued.
    pending: StoredValue<(f64, Vec<Step>)>,
    /// Restarts the link; set by the app. Called after a failed write.
    recover: StoredValue<Option<Rc<dyn Fn(&str)>>, LocalStorage>,
    /// When a frame was last written, so the keepalive only sends when the link is idle.
    last_tx: StoredValue<f64>,
    /// Link restarts after failed writes since the last write that worked.
    failures: StoredValue<u32>,
    /// When the machine last sent anything (a notification).
    last_rx: StoredValue<f64>,
}

/// After this many restarts in a row without one good write, a failing command is
/// reported instead of restarting the link forever.
const MAX_WRITE_RESTARTS: u32 = 3;

/// Queued commands older than this are dropped on reconnect rather than sent late:
/// a drill that starts by itself long after it was asked for would be a surprise.
const PENDING_MAX_AGE_MS: f64 = 15_000.0;

impl Ctx {
    fn push_log(&self, line: String) {
        let d = js_sys::Date::new_0();
        let line = format!("{:02}:{:02}:{:02} {line}", d.get_hours(), d.get_minutes(), d.get_seconds());
        self.log.update(|l| {
            l.push(line);
            let n = l.len();
            if n > 300 {
                l.drain(..n - 300);
            }
        });
    }

    /// Sends steps in order in the background. While the link is being restored they
    /// are queued and sent once it is back; when a write fails, the link is restarted
    /// and the rest is sent after it.
    fn run(&self, steps: Vec<Step>) {
        self.run_logged(steps, true);
    }

    fn run_logged(&self, steps: Vec<Step>, log: bool) {
        let Some(m) = self.machine.get_value() else {
            if self.keep.get_value() && !steps.is_empty() {
                self.queue(steps);
            }
            return;
        };
        let ctx = *self;
        spawn_local(async move {
            for (i, (delay, f)) in steps.iter().enumerate() {
                if *delay > 0 {
                    TimeoutFuture::new(*delay).await;
                }
                // The link was restarted meanwhile: hand the rest to the new one.
                if !ctx.machine.get_value().is_some_and(|cur| Rc::ptr_eq(&cur, &m)) {
                    ctx.run_logged(steps[i..].to_vec(), log);
                    return;
                }
                if log {
                    ctx.push_log(format!("TX {}", p::hex(f)));
                }
                match m.send(f).await {
                    Ok(()) => {
                        ctx.last_tx.set_value(js_sys::Date::now());
                        ctx.failures.set_value(0);
                    }
                    Err(e) => {
                        ctx.push_log(format!("Write failed: {e}"));
                        let recover = ctx.recover.get_value();
                        let failures = ctx.failures.get_value() + 1;
                        ctx.failures.set_value(failures);
                        if ctx.keep.get_value() && recover.is_some() && failures <= MAX_WRITE_RESTARTS {
                            ctx.queue(steps[i..].to_vec());
                            recover.unwrap()(&e);
                        } else {
                            ctx.pending.set_value((0.0, Vec::new()));
                            ctx.failures.set_value(0);
                            ctx.error.set(Some(format!("Write failed: {e}")));
                        }
                        return;
                    }
                }
            }
        });
    }

    fn queue(&self, steps: Vec<Step>) {
        self.pending.update_value(|(since, q)| {
            if q.is_empty() {
                *since = js_sys::Date::now();
            }
            q.extend(steps);
        });
    }

    /// The queued commands, if they are recent enough to send now.
    fn take_pending(&self) -> Vec<Step> {
        let (since, q) = self.pending.with_value(|(s, q)| (*s, q.clone()));
        self.pending.set_value((0.0, Vec::new()));
        if q.is_empty() {
            return q;
        }
        if js_sys::Date::now() - since > PENDING_MAX_AGE_MS {
            self.push_log(format!("Dropped {} queued commands: too old to send now", q.len()));
            return Vec::new();
        }
        q
    }
}

/// Scrolls to Vary Speed with its heading just below the sticky Start/Stop bar, which
/// would otherwise cover it.
fn show_vary_speed() {
    let Some(el) = document().get_element_by_id("vary-speed") else { return };
    let bar = document().query_selector("main .sticky").ok().flatten().map_or(0.0, |s| s.get_bounding_client_rect().height());
    let top = el.get_bounding_client_rect().top() + window().scroll_y().unwrap_or(0.0) - bar - 8.0;
    window().scroll_to_with_x_and_y(0.0, top.max(0.0));
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

fn load_custom(sport: Sport) -> [CustomPoint; p::CUSTOM_POINTS] {
    let mut pts = CustomPoint::defaults(sport);
    if let Some(s) = storage().and_then(|s| s.get_item(custom_key(sport)).ok().flatten()) {
        for (i, row) in s.split(';').take(p::CUSTOM_POINTS).enumerate() {
            let v: Vec<u16> = row.split(',').filter_map(|x| x.parse().ok()).collect();
            if let [lr, ud, speed, interval] = v[..] {
                pts[i] = CustomPoint { lr, ud, speed, interval: interval as u8 }.clamped();
            }
        }
    }
    pts
}

fn save_custom(sport: Sport, pts: &[CustomPoint; p::CUSTOM_POINTS]) {
    let s = pts.iter().map(|c| format!("{},{},{},{}", c.lr, c.ud, c.speed, c.interval)).collect::<Vec<_>>().join(";");
    if let Some(st) = storage() {
        let _ = st.set_item(custom_key(sport), &s);
    }
}

/// Gap between calibration rows, as in `sendBaseData`.
fn table_steps(frames: Vec<Frame>, first_delay: u32) -> Vec<Step> {
    frames.into_iter().enumerate().map(|(i, f)| (if i == 0 { first_delay } else { 10 }, f)).collect()
}

#[component]
pub fn App() -> impl IntoView {
    let dutch = ui::language();
    let ctx = Ctx {
        machine: StoredValue::new_local(None),
        name: RwSignal::new(None),
        log: RwSignal::new(Vec::new()),
        error: RwSignal::new(None),
        keep: StoredValue::new(false),
        pending: StoredValue::new((0.0, Vec::new())),
        recover: StoredValue::new_local(None),
        last_tx: StoredValue::new(0.0),
        failures: StoredValue::new(0),
        last_rx: StoredValue::new(0.0),
    };
    let battery = RwSignal::new(None::<u8>);
    let fault = RwSignal::new(None::<String>);
    let running = RwSignal::new(false);
    // Key of the library drill started last, so its card can show it is playing.
    let active_drill = RwSignal::new(None::<&'static str>);
    // The drill whose court popup is open. Tracked here instead of left to CSS :hover and
    // :focus, which some phone browsers keep after a tap elsewhere, so a click or tap
    // outside the popup (or Escape) always closes it.
    let open_pop = RwSignal::new(None::<&'static str>);
    let close_unless_inside = move |target: Option<web_sys::EventTarget>| {
        use wasm_bindgen::JsCast;
        let Some(open) = open_pop.get_untracked() else { return };
        let inside = target
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|el| el.closest(".drill-name").ok().flatten())
            .and_then(|name| name.get_attribute("data-drill"))
            .is_some_and(|key| key == open);
        if !inside {
            open_pop.set(None);
        }
    };
    let _ = window_event_listener(leptos::ev::pointerdown, move |e| close_unless_inside(e.target()));
    let _ = window_event_listener(leptos::ev::focusin, move |e| close_unless_inside(e.target()));
    let _ = window_event_listener(leptos::ev::keydown, move |e| if e.key() == "Escape" { open_pop.set(None) });
    let connecting = RwSignal::new(false);

    let sport = RwSignal::new(match storage().and_then(|s| s.get_item(SPORT_KEY).ok().flatten()).as_deref() {
        Some("padel") => Sport::Padel,
        _ => Sport::Tennis,
    });
    let speed = RwSignal::new(5u8);
    let freq = RwSignal::new(5u8);
    let spin = RwSignal::new(0i8);
    let lr = RwSignal::new(1140u16);
    let ud = RwSignal::new(1000u16);
    // Court height always starts on Low.
    let height = RwSignal::new(Height::Low);
    let height_level = RwSignal::new(p::HEIGHT_LEVEL_DEFAULT);

    let pattern = RwSignal::new(Pattern::Fixed(1));
    let program = RwSignal::new(Vec::<u8>::new());
    let custom_seq = RwSignal::new(Vec::<u8>::new());
    let custom = RwSignal::new(load_custom(sport.get_untracked()));
    let editing = RwSignal::new(1u8);
    let defaults = RwSignal::new(load_defaults());
    let notice = RwSignal::new(None::<String>);
    let advanced = RwSignal::new(storage().and_then(|s| s.get_item(ADVANCED_KEY).ok().flatten()).as_deref() == Some("1"));
    let status = RwSignal::new(None::<(u8, u8)>);

    // Whether the machine answered a battery query on this link: only then does a
    // missing answer mean the link is dead (see the watchdog below).
    let answers = StoredValue::new(false);
    let on_notify = move |bytes: Vec<u8>| {
        ctx.last_rx.set_value(js_sys::Date::now());
        ctx.push_log(format!("RX {}", p::hex(&bytes)));
        match p::parse_notification(&bytes) {
            Notification::Battery(v) => {
                answers.set_value(true);
                battery.set(Some(v));
            }
            Notification::Fault(f) => {
                if f == p::Fault::OutOfBalls {
                    running.set(false);
                }
                fault.set(Some(f.describe()));
            }
            Notification::Status(a, b) => status.set(Some((a, b))),
            _ => {}
        }
    };

    // Court table for the current height setting, plus the saved custom points.
    let table_upload = move |first_delay: u32| {
        let mut steps = table_steps(p::calibration(sport.get_untracked(), height.get_untracked(), height_level.get_untracked()), first_delay);
        if height.get_untracked() == Height::Mid {
            steps.extend(custom.get_untracked().iter().enumerate().map(|(i, c)| (10, p::custom_point(i as u8 + 1, *c))));
        }
        steps
    };

    // What the app sends after every connect, in the order and timing of its onConnectSuccess.
    let setup_steps = move || {
        let mut steps = table_upload(1500);
        steps.extend([
            (300, p::speed(sport.get_untracked(), speed.get_untracked())),
            (150, p::frequency(freq.get_untracked())),
            (50, p::lr_limits()),
            (50, p::ud_limits()),
            // The aim and spin shown, which may come from a saved default.
            (50, p::position(lr.get_untracked(), ud.get_untracked())),
            (50, p::spin(spin.get_untracked())),
            (50, p::apply()),
            (100, p::battery_query()),
        ]);
        steps
    };

    // Keeping the link alive. `device` is the machine to come back to; `reconnecting`
    // is true while a lost link is being restored; `wake` holds the screen wake lock.
    let device = StoredValue::new_local(None::<web_sys::BluetoothDevice>);
    let reconnecting = RwSignal::new(false);
    let wake = StoredValue::new_local(None::<wasm_bindgen::JsValue>);
    let reconnect_hook = StoredValue::new_local(None::<Rc<dyn Fn()>>);
    // Set to cut a reconnect pause short: the page was shown again or Bluetooth came back.
    let poke = StoredValue::new(false);
    // Whether the phone's Bluetooth is off (from navigator.bluetooth.getAvailability).
    let bt_off = RwSignal::new(false);
    // Watchdog state for this link: when the last keepalive went out still unanswered,
    // and how many keepalives in a row got no answer.
    let probe = StoredValue::new(None::<f64>);
    let missed = StoredValue::new(0u32);
    let reconnect = move || {
        if let Some(f) = reconnect_hook.get_value() {
            f();
        }
    };
    let screen_on = move || {
        if wake.with_value(Option::is_some) {
            return;
        }
        spawn_local(async move {
            if let Some(lock) = crate::ble::keep_screen_on().await {
                // Disconnected while the request was pending: let the screen sleep again.
                if ctx.keep.get_value() {
                    wake.set_value(Some(lock));
                } else {
                    crate::ble::release_screen(&lock);
                }
            }
        });
    };
    let screen_off = move || {
        if let Some(lock) = wake.get_value() {
            crate::ble::release_screen(&lock);
        }
        wake.set_value(None);
    };

    // One connect: through the device chooser when `dev` is None, otherwise straight to
    // that machine. `report` shows a failure on screen; reconnect tries only log it.
    let open = move |dev: Option<web_sys::BluetoothDevice>, report: bool| async move {
        let on_disconnect = move || {
            // Later, not inside the event: dropping the Machine drops this handler.
            spawn_local(async move {
                ctx.machine.set_value(None);
                ctx.name.set(None);
                ctx.push_log("Disconnected".into());
                if running.get_untracked() {
                    running.set(false);
                    notice.set(Some("The connection dropped while the machine was playing. Press Start again once it is back.".into()));
                }
                if ctx.keep.get_value() {
                    reconnect();
                } else {
                    screen_off();
                }
            });
        };
        match Machine::connect(dev, on_notify, on_disconnect, move |line| ctx.push_log(line)).await {
            Ok(m) => {
                let name = m.name.clone();
                ctx.push_log(format!("Connected to {name}"));
                if !(name.starts_with("PT7") || name.starts_with("PT8")) {
                    ctx.error.set(Some(format!(
                        "{name} is not a PT7/PT8 device. This protocol was decoded for PT7/PT8; other models may react differently."
                    )));
                }
                let d = m.device();
                if let Some(st) = storage() {
                    let _ = st.set_item(DEVICE_KEY, &d.id());
                }
                device.set_value(Some(d));
                ctx.keep.set_value(true);
                answers.set_value(false);
                probe.set_value(None);
                missed.set_value(0);
                ctx.machine.set_value(Some(m));
                ctx.name.set(Some(name));
                let mut steps = setup_steps();
                let queued = ctx.take_pending();
                if !queued.is_empty() {
                    ctx.push_log(format!("Sending {} commands queued while the link was down", queued.len()));
                    steps.extend(queued);
                }
                ctx.run(steps);
                screen_on();
                true
            }
            Err(e) => {
                ctx.push_log(format!("Connect failed: {e}"));
                if report {
                    ctx.error.set(Some(e));
                }
                false
            }
        }
    };

    // Restores a lost link to the same machine without the chooser, trying again with
    // growing pauses until it is back or Disconnect is pressed. Turning the machine off
    // and on, or walking out of range and back, therefore needs no tap on the phone.
    reconnect_hook.set_value(Some(Rc::new(move || {
        if reconnecting.get_untracked() || ctx.machine.with_value(Option::is_some) {
            return;
        }
        let Some(dev) = device.get_value() else { return };
        reconnecting.set(true);
        spawn_local(async move {
            let mut pause = 500;
            let mut tries = 0;
            while ctx.keep.get_value() && ctx.machine.with_value(Option::is_none) {
                // No use trying while the phone's Bluetooth is off; turning it on pokes.
                if bt_off.get_untracked() {
                    nap(poke, 5_000).await;
                    continue;
                }
                tries += 1;
                ctx.push_log(format!("Reconnecting (round {tries})"));
                // On success the loop ends, unless the link already dropped again.
                if open(Some(dev.clone()), false).await {
                    continue;
                }
                // Slower while the page is hidden, to spare the phone's battery.
                let max = if document().hidden() { 30_000 } else { 8_000 };
                if nap(poke, pause).await {
                    pause = 500;
                } else {
                    pause = (pause * 2).min(max);
                }
            }
            reconnecting.set(false);
        });
    })));

    // Drops the link and connects again, keeping queued commands. Used by the Reboot
    // button, after a failed write and when the link died without telling us.
    let restart_link = move |why: &str| {
        let Some(m) = ctx.machine.get_value() else { return };
        ctx.push_log(format!("Restarting the connection: {why}"));
        m.close_for_reboot();
        ctx.machine.set_value(None);
        ctx.name.set(None);
        spawn_local(async move {
            // Give the browser and the machine a moment to drop the old link.
            TimeoutFuture::new(1000).await;
            reconnect();
        });
    };
    ctx.recover.set_value(Some(Rc::new(move |e: &str| restart_link(&format!("write failed ({e})")))));

    let connect = move |_| {
        connecting.set(true);
        ctx.error.set(None);
        spawn_local(async move {
            open(None, true).await;
            connecting.set(false);
        });
    };

    let reboot = move |_| {
        running.set(false);
        restart_link("Reboot pressed");
    };

    // Ends the session: no reconnecting, nothing queued, the screen may sleep again.
    let disconnect = move |_| {
        ctx.keep.set_value(false);
        ctx.pending.set_value((0.0, Vec::new()));
        screen_off();
        match ctx.machine.get_value() {
            Some(m) => m.disconnect(),
            None => ctx.push_log("Stopped reconnecting".into()),
        }
    };

    // Keepalive and watchdog. While idle, ask for the battery level now and then: that
    // keeps the link (and perhaps the machine) awake and the battery shown up to date.
    // Commands are written without response, so a write "succeeds" even when the
    // machine no longer hears anything. The answer to the battery query is what proves
    // the link works: once the machine has answered on this link, several unanswered
    // queries in a row restart the link.
    {
        let handle = set_interval_with_handle(
            move || {
                let Some(m) = ctx.machine.get_value() else { return };
                if !m.is_connected() {
                    restart_link("the link was gone without a disconnect event");
                    return;
                }
                let now = js_sys::Date::now();
                if let Some(sent) = probe.get_value().filter(|t| now - t >= 3_000.0) {
                    if ctx.last_rx.get_value() >= sent {
                        missed.set_value(0);
                    } else if m.notifications && answers.get_value() {
                        let n = missed.get_value() + 1;
                        missed.set_value(n);
                        ctx.push_log(format!("No answer from the machine ({n}/{MAX_MISSED})"));
                        if n >= MAX_MISSED {
                            restart_link("the machine stopped answering");
                            return;
                        }
                    }
                    probe.set_value(None);
                }
                // Probe when idle, and also while unanswered, to settle it quickly.
                if probe.get_value().is_none() && (now - ctx.last_tx.get_value() > KEEPALIVE_MS as f64 || missed.get_value() > 0) {
                    probe.set_value(Some(now));
                    ctx.run_logged(vec![(0, p::battery_query())], false);
                }
            },
            std::time::Duration::from_millis(WATCHDOG_MS as u64),
        );
        on_cleanup(move || {
            if let Ok(h) = handle {
                h.clear();
            }
        });
    }

    // Follow the phone's Bluetooth switch: say so when it is off, and reconnect at once
    // when it comes back on instead of waiting out the pause.
    if let Some(bt) = window().navigator().bluetooth() {
        let check = {
            let bt = bt.clone();
            move || {
                let p = bt.get_availability();
                spawn_local(async move {
                    if let Ok(v) = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(wasm_bindgen::JsValue::from(p))).await {
                        let off = v.as_bool() == Some(false);
                        if bt_off.get_untracked() != off {
                            bt_off.set(off);
                            ctx.push_log(if off { "Bluetooth is off on this phone".into() } else { "Bluetooth is on again".into() });
                            if !off {
                                poke.set_value(true);
                                reconnect();
                            }
                        }
                    }
                });
            }
        };
        check();
        let on_change = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(check);
        bt.set_onavailabilitychanged(Some(wasm_bindgen::JsCast::unchecked_ref(on_change.as_ref())));
        on_change.forget();
    }

    // Coming back to the page (screen on, app switched back): the browser dropped the
    // wake lock while hidden and may have lost the link without an event, so check both.
    let _ = window_event_listener(leptos::ev::visibilitychange, move |_| {
        if document().hidden() || !ctx.keep.get_value() {
            return;
        }
        wake.set_value(None);
        match ctx.machine.get_value() {
            Some(m) if !m.is_connected() => restart_link("the link was lost while the page was hidden"),
            Some(_) => screen_on(),
            None => {
                poke.set_value(true);
                reconnect();
            }
        }
    });

    // After a reload, connect to the machine used last without the chooser, where the
    // browser can list the devices this site may use (navigator.bluetooth.getDevices).
    spawn_local(async move {
        let Some(id) = storage().and_then(|s| s.get_item(DEVICE_KEY).ok().flatten()) else { return };
        let Some(dev) = crate::ble::known_devices().await.into_iter().find(|d| d.id() == id) else { return };
        ctx.push_log(format!("Found {} from last time; connecting", dev.name().unwrap_or_default()));
        device.set_value(Some(dev));
        ctx.keep.set_value(true);
        reconnect();
    });

    let send_speed = move || ctx.run(vec![(0, p::speed(sport.get_untracked(), speed.get_untracked())), (50, p::apply())]);
    let send_freq = move || ctx.run(vec![(0, p::frequency(freq.get_untracked())), (50, p::apply())]);
    let send_spin = move || {
        let s = spin.get_untracked();
        let mut steps = Vec::new();
        // The app raises speed to level 5 before applying any spin.
        if s != 0 && speed.get_untracked() < 5 {
            speed.set(5);
            steps.push((0, p::speed(sport.get_untracked(), 5)));
        }
        if s == 0 {
            steps.extend([(0, p::spin(0)), (50, p::apply())]);
        } else {
            steps.extend([(50, p::apply()), (50, p::spin(s))]);
        }
        ctx.run(steps);
    };
    // The app caps height at 2000 in fixed-point and horizontal patterns.
    let ud_cap = move || match pattern.get() {
        Pattern::Fixed(_) | Pattern::Horizontal(_) => p::UD_MAX_LOW_MODES,
        _ => p::UD_MAX,
    };
    let position_frame = move || {
        let pat = pattern.get_untracked();
        let x = if matches!(pat, Pattern::Horizontal(_)) { 0 } else { lr.get_untracked() };
        let cap = if matches!(pat, Pattern::Fixed(_) | Pattern::Horizontal(_)) { p::UD_MAX_LOW_MODES } else { p::UD_MAX };
        let y = if matches!(pat, Pattern::Vertical(_)) { 0 } else { ud.get_untracked().min(cap) };
        p::position(x, y)
    };
    let send_position = move || ctx.run(vec![(0, position_frame()), (50, p::apply())]);

    let current_pattern = move || match pattern.get_untracked() {
        Pattern::Program(_) => Pattern::Program(program.get_untracked()),
        Pattern::Custom(_) => Pattern::Custom(custom_seq.get_untracked()),
        other => other,
    };

    let start = move || {
        let pat = current_pattern();
        if !pat.is_ready() {
            ctx.error.set(Some("Select at least one point first.".into()));
            return;
        }
        fault.set(None);
        running.set(true);
        active_drill.set(None);
        ctx.run(pat.start());
    };
    let stop = move || {
        running.set(false);
        // Stop wins over anything still waiting for the link (a drill start, say).
        ctx.pending.set_value((0.0, Vec::new()));
        // Stop is sent twice: a write without response that the phone drops gives no
        // error, and a lost Stop leaves the machine firing. A second stop does nothing.
        let mut steps = p::stop_sequence();
        steps.insert(1, (60, p::stop()));
        ctx.run(steps);
    };

    // Selecting a pattern applies its preset like the app does, then its saved default,
    // and restarts if running. Returns the frames, so callers can queue them after others.
    let select_steps = move |pat: Pattern| -> Vec<Step> {
        pattern.set(pat.clone());
        let sp = sport.get_untracked();
        let pre = pat.preset(sp);
        let mut steps: Vec<Step> = Vec::new();
        // The app sends the preset aim as is, including on horizontal and vertical patterns.
        if let Some((x, y)) = pre.aim {
            lr.set(x);
            ud.set(y);
            steps.push((0, p::position(x, y)));
        }
        if let Some(s) = pre.speed {
            speed.set(s);
            steps.push((50, p::speed(sp, s)));
        }
        if let Some((threshold, level)) = pre.speed_max {
            if speed.get_untracked() > threshold {
                speed.set(level);
                steps.push((50, p::speed(sp, level)));
            }
        }
        if let Some((level, f)) = pre.spin {
            spin.set(level);
            steps.push((50, f));
        }
        // A saved default for this pattern overrides the preset, as in setDefaultMode.
        let saved = default_key(sp, &pat).and_then(|k| defaults.get_untracked().into_iter().find(|(key, _)| *key == k));
        if let Some((_, d)) = saved {
            lr.set(d.lr);
            ud.set(d.ud);
            freq.set(d.freq);
            speed.set(d.speed);
            spin.set(d.spin);
            // The court height stays as chosen (Low by default); a stored fine height level
            // is kept for Mid and only sent as a table while Mid is chosen.
            if d.height_level != 0 {
                height_level.set(d.height_level);
            }
            let table = if height.get_untracked() == Height::Mid { d.height_level } else { 0 };
            steps.extend(PatternDefault { height_level: table, ..d }.apply_steps(sp));
        }
        notice.set(None);
        let reselect = sp == Sport::Padel && running.get_untracked() && matches!(pat, Pattern::Fixed(_) | Pattern::Volley(_));
        steps.push((50, if reselect { p::apply_padel_reselect() } else { p::apply() }));
        if running.get_untracked() && pat.is_ready() {
            steps.extend(current_pattern().start());
        }
        steps
    };
    let select = move |pat: Pattern| ctx.run(select_steps(pat));
    // Start with pattern 1's preset and saved default, so a reload shows the saved values
    // (nothing is sent while disconnected; connecting sends them).
    let _ = select_steps(Pattern::Fixed(1));

    let current_default = move || PatternDefault {
        lr: lr.get(),
        ud: ud.get(),
        freq: freq.get(),
        speed: speed.get(),
        spin: spin.get(),
        height_level: height_level.get(),
    };
    let saved_default = move || {
        let key = default_key(sport.get(), &pattern.get())?;
        defaults.get().into_iter().find(|(k, _)| *k == key).map(|(_, d)| d)
    };
    // Like the app's SaveDefault button: highlighted when there is something new to save.
    let default_dirty = move || match saved_default() {
        Some(d) => d.differs(&pattern.get(), &current_default()),
        None => true,
    };
    let save_default = move |_| {
        let Some(key) = default_key(sport.get_untracked(), &pattern.get_untracked()) else { return };
        if !default_dirty() {
            notice.set(Some("No change for default value.".into()));
            return;
        }
        let cur = current_default();
        defaults.update(|d| {
            d.retain(|(k, _)| *k != key);
            d.push((key, cur));
            save_defaults(d);
        });
        notice.set(Some("Current values have been saved as default.".into()));
    };
    let forget_default = move |_| {
        let Some(key) = default_key(sport.get_untracked(), &pattern.get_untracked()) else { return };
        defaults.update(|d| {
            d.retain(|(k, _)| *k != key);
            save_defaults(d);
        });
        notice.set(Some("Default removed; the pattern's built-in preset applies next time.".into()));
    };

    let set_height = move |h: Height| {
        height.set(h);
        ctx.run(table_upload(0));
    };
    let change_level = move |delta: i8| {
        let lvl = (height_level.get_untracked() as i8 + delta).clamp(p::HEIGHT_LEVEL_MIN as i8, sport.get_untracked().height_level_max() as i8) as u8;
        if lvl == height_level.get_untracked() {
            return;
        }
        height_level.set(lvl);
        let mut steps = vec![(5, p::apply())];
        steps.extend(table_upload(45));
        ctx.run(steps);
    };

    let edit_custom = move |f: &dyn Fn(&mut CustomPoint)| {
        let n = editing.get_untracked();
        custom.update(|c| {
            let pt = &mut c[(n - 1) as usize];
            f(pt);
            *pt = pt.clamped();
        });
        let pts = custom.get_untracked();
        save_custom(sport.get_untracked(), &pts);
        ctx.run(vec![(0, p::custom_point(n, pts[(n - 1) as usize])), (50, p::apply())]);
    };

    // Vary Speed drills.
    let drill = RwSignal::new(Vec::<u8>::new());
    // Names for the balls of a drill made in the Drill Builder: (point, player in doubles,
    // English, Dutch) per position. Shown while the route still starts with those points.
    let drill_labels = RwSignal::new(Vec::<(u8, Option<usize>, String, String)>::new());
    let pause = RwSignal::new(DrillPause::None);
    let drill_params = RwSignal::new(DrillPoint::defaults());
    let routes = RwSignal::new(load_routes());
    let ai = RwSignal::new(AiOptions { left: true, level: 2, balls: 3 });
    let date = move || ctx.name.get_untracked().as_deref().and_then(p::date_code);

    let start_current_drill = move || {
        let pts = drill.get_untracked();
        if pts.is_empty() {
            return;
        }
        fault.set(None);
        running.set(true);
        let frames = p::drill(&pts, &drill_params.get_untracked(), pause.get_untracked(), date());
        ctx.run(frames.into_iter().map(|f| (50, f)).collect());
    };
    let start_drill = move |_| {
        active_drill.set(None);
        start_current_drill();
    };
    // Built-in drills load into Vary Speed like a saved route, at the chosen difficulty.
    let difficulty = RwSignal::new(
        storage()
            .and_then(|s| s.get_item(DIFFICULTY_KEY).ok().flatten())
            .and_then(|v| v.parse::<u8>().ok())
            .filter(|v| (1..=5).contains(v))
            .unwrap_or(5),
    );
    let set_difficulty = move |level: u8| {
        difficulty.set(level);
        if let Some(st) = storage() {
            let _ = st.set_item(DIFFICULTY_KEY, &level.to_string());
        }
    };
    // Loads a drill's points and per-point settings into Vary Speed at the chosen difficulty.
    let load_drill = move |params: Vec<(u8, DrillPoint)>, drill_pause: DrillPause, points: Vec<u8>| {
        drill_params.update(|all| {
            for (id, pt) in params {
                all[(id - 1) as usize] = pt;
            }
        });
        pause.set(drill_pause);
        drill.set(points);
        drill_labels.set(Vec::new());
    };
    let load_library_drill = move |d: &'static Drill| load_drill(d.params_at(difficulty.get_untracked()), d.pause, d.points());
    let load_generated = move |g: &builder::Generated| {
        let d = g.drill();
        load_drill(d.params_at(difficulty.get_untracked()), d.pause, d.points());
        drill_labels.set(g.planned.iter().map(|p| {
            let (en, nl) = builder::ball_words(p, g.doubles);
            (p.point, g.doubles.then_some(p.player), en, nl)
        }).collect());
    };
    // Editing a shot changes its point and is sent to the machine straight away, like the app.
    let edit_shot = move |position: usize, f: &dyn Fn(&mut DrillPoint)| {
        let pts = drill.get_untracked();
        let Some(&id) = pts.get(position) else { return };
        drill_params.update(|d| {
            let pt = &mut d[(id - 1) as usize];
            f(pt);
            *pt = pt.clamped();
        });
        let pt = drill_params.get_untracked()[(id - 1) as usize];
        let last = position + 1 == pts.len();
        ctx.run(p::drill_edit(position as u8 + 1, pt, last, pause.get_untracked(), date()));
    };
    let generate = move |_| {
        let mut params = drill_params.get_untracked();
        let pts = p::ai_drill(ai.get_untracked(), &mut params, |n| (js_sys::Math::random() * n as f64) as usize % n);
        drill_params.set(params);
        drill.set(pts);
        drill_labels.set(Vec::new());
    };
    let save_route = move |name: String| {
        let pts = drill.get_untracked();
        let params = drill_params.get_untracked();
        let route = Route {
            name: name.clone(),
            points: pts.clone(),
            pause: pause.get_untracked(),
            params: pts.iter().map(|&id| (id, params[(id - 1) as usize])).collect(),
        };
        routes.update(|r| {
            r.retain(|x| x.name != name);
            r.push(route);
            save_routes(r);
        });
    };
    let load_route = move |name: String| {
        let Some(r) = routes.get_untracked().into_iter().find(|r| r.name == name) else { return };
        drill_params.update(|d| {
            for (id, pt) in &r.params {
                d[(*id - 1) as usize] = *pt;
            }
        });
        pause.set(r.pause);
        drill.set(r.points);
        drill_labels.set(Vec::new());
    };
    let delete_route = move |name: String| {
        routes.update(|r| {
            r.retain(|x| x.name != name);
            save_routes(r);
        });
    };
    let route_names = Signal::derive(move || routes.get().into_iter().map(|r| r.name).collect::<Vec<_>>());

    // Named programs and custom-point sets.
    let programs = RwSignal::new(load_programs());
    let custom_sets = RwSignal::new(load_custom_sets());
    let save_program = move |name: String| {
        let entry = SavedProgram {
            name: name.clone(),
            points: program.get_untracked(),
            settings: ProgramSettings {
                freq: freq.get_untracked(),
                speed: speed.get_untracked(),
                spin: spin.get_untracked(),
                height: height.get_untracked(),
                height_level: height_level.get_untracked(),
            },
        };
        programs.update(|l| {
            l.retain(|x| x.name != name);
            l.push(entry);
            save_programs(l);
        });
    };
    let load_program = move |name: String| {
        let Some(x) = programs.get_untracked().into_iter().find(|x| x.name == name) else { return };
        let s = x.settings;
        program.set(x.points);
        freq.set(s.freq);
        speed.set(s.speed);
        spin.set(s.spin);
        // The app always loads the mid table here, whatever was saved.
        height.set(Height::Mid);
        if s.height_level != 0 {
            height_level.set(s.height_level);
        }
        ctx.run(s.load_steps(sport.get_untracked(), height_level.get_untracked()));
    };
    let delete_program = move |name: String| {
        programs.update(|l| {
            l.retain(|x| x.name != name);
            save_programs(l);
        });
    };
    let save_custom_set = move |name: String| {
        let entry = CustomSet { name: name.clone(), order: custom_seq.get_untracked(), points: custom.get_untracked() };
        custom_sets.update(|l| {
            l.retain(|x| x.name != name);
            l.push(entry);
            save_custom_sets(l);
        });
    };
    let load_custom_set = move |name: String| {
        let Some(x) = custom_sets.get_untracked().into_iter().find(|x| x.name == name) else { return };
        custom_seq.set(x.order);
        custom.set(x.points);
        save_custom(sport.get_untracked(), &x.points);
        ctx.run(p::custom_set_load_steps(&x.points));
    };
    let delete_custom_set = move |name: String| {
        custom_sets.update(|l| {
            l.retain(|x| x.name != name);
            save_custom_sets(l);
        });
    };
    let program_names = Signal::derive(move || programs.get().into_iter().map(|x| x.name).collect::<Vec<_>>());
    let custom_set_names = Signal::derive(move || custom_sets.get().into_iter().map(|x| x.name).collect::<Vec<_>>());

    // A course's Start opens Vary Speed with its points; the screen resets every interval.
    let practice_course = move |points: Vec<u8>| {
        drill_params.update(|d| d.iter_mut().for_each(|p| p.interval = p::DRILL_INTERVAL_DEFAULT));
        pause.set(DrillPause::None);
        drill.set(points);
        drill_labels.set(Vec::new());
        show_vary_speed();
    };

    // The app's "Paddle Tennis" / "Tennis" button opens the other screen, which
    // reconnects and uploads its own court table. Here the table is re-sent in place.
    let switch_sport = move |s: Sport| {
        if sport.get_untracked() == s {
            return;
        }
        let mut steps: Vec<Step> = Vec::new();
        if running.get_untracked() {
            running.set(false);
            steps.extend(p::stop_sequence());
        }
        sport.set(s);
        if let Some(st) = storage() {
            let _ = st.set_item(SPORT_KEY, if s == Sport::Padel { "padel" } else { "tennis" });
        }
        custom.set(load_custom(s));
        height_level.update(|l| *l = (*l).min(s.height_level_max()));
        // Pattern 1 with this sport's saved default; its height level goes into the table.
        let selected = select_steps(Pattern::Fixed(1));
        steps.extend(table_upload(300));
        steps.push((300, p::speed(s, speed.get_untracked())));
        steps.extend(selected);
        ctx.run(steps);
    };
    let padel = move || sport.get() == Sport::Padel;
    let pt7_padel = move || padel() && ctx.name.get().is_some_and(|n| n.starts_with("PT7"));

    let connected = move || ctx.name.get().is_some();
    let builder_open = RwSignal::new(false);
    let quick_open = RwSignal::new(false);
    let is_program = move || matches!(pattern.get(), Pattern::Program(_));
    let is_custom = move || matches!(pattern.get(), Pattern::Custom(_));

    view! {
        <header>
            <h1>{move || tr("PT-9001 control")}</h1>
            <label class="language">
                <span>{move || tr("Language")}</span>
                <select prop:value=move || if dutch.get() { "nl" } else { "en" }
                    on:change=move |ev| dutch.set(event_target_value(&ev) == "nl")>
                    <option value="en">"English"</option>
                    <option value="nl">"Nederlands"</option>
                </select>
            </label>
            <div class="sport" role="group" aria-label="Sport">
                <span class="with-help"><button class="mode" class:selected=move || !padel() on:click=move |_| switch_sport(Sport::Tennis)>"Tennis"</button><Help topic="Tennis" /></span>
                <span class="with-help"><button class="mode" class:selected=padel on:click=move |_| switch_sport(Sport::Padel)>"Padel"</button><Help topic="Padel" /></span>
            </div>
            <span class="with-help"><button class="quick-drill" class:selected=move || quick_open.get()
                aria-expanded=move || if quick_open.get() { "true" } else { "false" } aria-controls="quick-drill"
                on:click=move |_| quick_open.update(|open| *open = !*open)>{move || tr("Quick Drill")}</button><Help topic="Quick Drill" /></span>
            <span class="with-help"><button class="mode" class:selected=move || builder_open.get()
                aria-expanded=move || if builder_open.get() { "true" } else { "false" } aria-controls="drill-builder"
                on:click=move |_| builder_open.update(|open| *open = !*open)>{move || tr("Drill Builder")}</button><Help topic="Drill Builder" /></span>
            <Show
                when=connected
                fallback=move || view! {
                    <Show when=move || reconnecting.get()
                        fallback=move || view! {
                            <span class="with-help"><button class="primary" on:click=connect disabled=move || connecting.get()>
                                {move || if connecting.get() { tr("Connecting…") } else { tr("Connect machine") }}
                            </button><Help topic="Connect machine" /></span>
                        }>
                        <span class="device reconnecting" role="status">{move || tr("Reconnecting…")}</span>
                        <span class="with-help"><button on:click=disconnect>{move || tr("Cancel")}</button><Help topic="Cancel reconnect" /></span>
                    </Show>
                }
            >
                <span class="device">{move || ctx.name.get()}</span>
                <span class="battery">
                    {move || match battery.get() {
                        Some(v) => format!("{} {} ({v})", tr("Battery"), "▮".repeat(p::battery_bars(v) as usize) + &"▯".repeat(4 - p::battery_bars(v) as usize)),
                        None => format!("{} –", tr("Battery")),
                    }}
                </span>
                <span class="with-help"><button on:click=move |_| ctx.run(vec![(0, p::battery_query())])>"↻"</button><Help topic="↻" /></span>
                <span class="with-help"><button on:click=reboot>{move || tr("Reboot")}</button><Help topic="Reboot" /></span>
                <span class="with-help"><button on:click=disconnect>{move || tr("Disconnect")}</button><Help topic="Disconnect" /></span>
            </Show>
        </header>

        // One tap starts a library drill at the chosen difficulty and closes the menu.
        <Show when=move || quick_open.get()>
            <nav id="quick-drill" aria-label="Quick Drill">
                <Show when=move || !connected()>
                    <p class="hint">{move || tr("Connect the machine to start a drill.")}</p>
                </Show>
                {[(false, "Singles"), (true, "Doubles")].into_iter().map(|(doubles, heading)| view! {
                    <h3>{move || tr(heading)}</h3>
                    <div class="quick-list">
                        {LIBRARY.iter().filter(move |d| d.doubles == doubles).map(|d| view! {
                            <button class="quick-item" disabled=move || !connected()
                                class:playing=move || running.get() && active_drill.get() == Some(d.key)
                                on:click=move |_| {
                                    load_library_drill(d);
                                    start_current_drill();
                                    active_drill.set(Some(d.key));
                                    quick_open.set(false);
                                }>
                                <strong>{move || if ui::dutch() { d.name.1 } else { d.name.0 }}</strong>
                                <small>{move || { let (en, nl) = d.level.label(); if ui::dutch() { nl } else { en } }}</small>
                            </button>
                        }).collect_view()}
                    </div>
                }).collect_view()}
                <div class="row">
                    <button class="stop" disabled=move || !connected() on:click=move |_| stop()>{move || tr("Stop")}</button>
                    <button on:click=move |_| quick_open.set(false)>{move || tr("Close")}</button>
                </div>
            </nav>
        </Show>

        // Outside <main>, so it can be used before a machine is connected. Hidden rather
        // than removed when closed, so the text and choices survive closing the panel.
        <div id="drill-builder" hidden=move || !builder_open.get()>
            <DrillBuilder
                on_generate=load_generated
                on_start=move |g: &builder::Generated| {
                    load_generated(g);
                    start_current_drill();
                    active_drill.set(Some("builder"));
                }
                on_edit=move |g: &builder::Generated| {
                    load_generated(g);
                    show_vary_speed();
                }
                on_stop=move || stop()
                connected=Signal::derive(connected)
                playing=Signal::derive(move || running.get() && active_drill.get() == Some("builder"))
                difficulty=difficulty.into() />
        </div>

        <Show when=move || bt_off.get()>
            <p class="error" role="status">{move || tr("Bluetooth is off on this phone. Turn it on; the app reconnects by itself.")}</p>
        </Show>
        <Show when=move || ctx.error.get().is_some()>
            <div class="error">{move || ctx.error.get().map(|msg| ui::message(&msg))} " " <span class="with-help"><button on:click=move |_| ctx.error.set(None)>{move || tr("Dismiss")}</button><Help topic="Dismiss" /></span></div>
        </Show>
        <Show when=pt7_padel>
            <p class="error">{move || tr("This machine's name starts with PT7. The official app sends nothing to PT7 machines from its padel screen, so padel mode may not be supported on this model. Untested.")}</p>
        </Show>
        <Show when=move || fault.get().is_some()>
            <div class="fault">{move || fault.get().map(|msg| ui::message(&msg))} " " <span class="with-help"><button on:click=move |_| fault.set(None)>{move || tr("Dismiss")}</button><Help topic="Dismiss" /></span></div>
        </Show>

        <main class:disabled=move || !connected()>
            <section class="sticky">
                <div class="run">
                    <span class="current">{move || pattern_name(&pattern.get())}</span>
                    <Show when=move || default_key(sport.get(), &pattern.get()).is_some()>
                        <span class="with-help"><button class:dirty=default_dirty on:click=save_default
                            title=move || tr("Save the current aim, speed, frequency, spin and height as this pattern's default")>{move || tr("Save default")}</button><Help topic="Save default" /></span>
                        <Show when=move || saved_default().is_some()>
                            <span class="with-help"><button on:click=forget_default title=move || tr("Remove this pattern's saved default")>"✕"</button><Help topic="✕" /></span>
                        </Show>
                    </Show>
                    <span class="with-help"><button class="primary" on:click=move |_| start() disabled=move || running.get()>{move || tr("Start")}</button></span>
                    <span class="with-help"><button class="stop" on:click=move |_| stop()>{move || tr("Stop")}</button></span>
                </div>
                <Show when=move || notice.get().is_some()>
                    <p class="hint">{move || notice.get().map(|msg| tr(&msg).to_owned())}</p>
                </Show>
            </section>

            <section>
                <h2>{move || tr("Shot")}</h2>
                <Slider label="Speed" min=1 max=13 value=speed on_commit=send_speed />
                <Slider label="Frequency" min=1 max=10 value=freq on_commit=send_freq />
                <Slider label="Spin (− back / + top)" min=-6 max=6 value=spin on_commit=send_spin />
                <label>
                    <span>{move || tr("Aim ")} {move || {
                        let v = lr.get();
                        if v > 1140 { tr("left") } else if v < 1140 { tr("right") } else { tr("center") }
                    }} " (" {move || lr.get()} ")"</span>
                    <input type="range" min=p::LR_MIN max=p::LR_MAX step="10"
                        prop:value=move || p::LR_MAX + p::LR_MIN - lr.get()
                        on:input=move |ev| lr.set(p::LR_MAX + p::LR_MIN - event_target_value(&ev).parse().unwrap_or(1140))
                        on:change=move |_| send_position() />
                </label>
                <label>
                    <span>{move || tr("Height ")} {move || ud.get().min(ud_cap())}</span>
                    <input type="range" min="0" max=move || ud_cap() step="100"
                        prop:value=move || ud.get().min(ud_cap())
                        on:input=move |ev| ud.set(event_target_value(&ev).parse().unwrap_or(0))
                        on:change=move |_| send_position() />
                </label>
                <div class="row">
                    <span>{move || tr("Court height")}</span>
                    {[(Height::Low, "Low"), (Height::Mid, "Mid"), (Height::High, "High")].into_iter().map(|(h, l)| view! {
                        <span class="with-help"><button class="mode" class:selected=move || height.get() == h on:click=move |_| set_height(h)>{move || tr(l)}</button></span>
                    }).collect_view()}
                    <Show when=move || height.get() == Height::Mid>
                        <span class="with-help"><button on:click=move |_| change_level(-1)>"−"</button><Help topic="Height −" /></span>
                        <span class="value">{move || height_level.get()}</span>
                        <span class="with-help"><button on:click=move |_| change_level(1)>"+"</button><Help topic="Height +" /></span>
                    </Show>
                </div>
            </section>

            <section>
                <h2>{move || tr("Patterns")}</h2>
                <PatternGroup title="Basics" pattern=pattern select=select options=vec![
                    Pattern::Fixed(1), Pattern::Fixed(2), Pattern::Fixed(3), Pattern::Random,
                    Pattern::Primary, Pattern::Smash, Pattern::Moon,
                    Pattern::Volley(1), Pattern::Volley(2), Pattern::Volley(3),
                ] />
                <div style:display=move || if padel() { "" } else { "none" }>
                    <PatternGroup title="Padel shots" pattern=pattern select=select options=vec![
                        Pattern::Bandeja, Pattern::Bajada, Pattern::ServeReturn,
                    ] />
                </div>
                <PatternGroup title="Horizontal" pattern=pattern select=select options=(1..=5).map(Pattern::Horizontal).collect() />
                <PatternGroup title="Vertical" pattern=pattern select=select options=(1..=3).map(Pattern::Vertical).collect() />
                <PatternGroup title="Crosscourt" pattern=pattern select=select options=(1..=6).map(Pattern::Cross).collect() />
                <PatternGroup title="Your own" pattern=pattern select=select options=vec![Pattern::Program(vec![]), Pattern::Custom(vec![])] />

                <Show when=is_program>
                    <p class="hint">{move || tr("Tap court points in play order (max ")} {p::MAX_SEQUENCE} {move || tr("). Far side is at the top. 36–41 are your custom points.")}</p>
                    <div class="court">
                        {(0..5u8).rev().map(|row| view! {
                            <div class="court-row">
                                {(1..=7u8).map(|col| view! { <PointButton id=row * 7 + col list=program max=p::MAX_SEQUENCE /> }).collect_view()}
                            </div>
                        }).collect_view()}
                        <div class="court-row custom-row">
                            {(36..=41u8).map(|id| view! { <PointButton id=id list=program max=p::MAX_SEQUENCE /> }).collect_view()}
                        </div>
                    </div>
                    <div class="run">
                        <span class="with-help"><button on:click=move |_| program.update(|d| { d.pop(); })>{move || tr("Undo")}</button><Help topic="Undo" /></span>
                        <span class="with-help"><button on:click=move |_| program.set(Vec::new())>{move || tr("Clear")}</button><Help topic="Clear" /></span>
                    </div>
                    <SavedList title="Saved programs" noun="program" names=program_names can_save=Signal::derive(move || !program.get().is_empty())
                        on_load=load_program on_delete=delete_program on_save=save_program />
                </Show>

                <Show when=is_custom>
                    <p class="hint">{move || tr("Edit a point (changes are sent to the machine and saved in this browser), then tap points in play order.")}</p>
                    <div class="row">
                        {(1..=6u8).map(|n| view! {
                            <span class="with-help"><button class="mode" class:selected=move || editing.get() == n on:click=move |_| editing.set(n)>{move || format!("{} {n}", tr("Point"))}</button><Help topic="Edit point" /></span>
                        }).collect_view()}
                    </div>
                    {move || {
                        let n = editing.get();
                        let c = custom.get()[(n - 1) as usize];
                        view! {
                            <div class="editor">
                                <Stepper label="Left ↔ right" value=p::lr_percent_label(p::custom_lr_percent(c.lr)) dec_label="◀" inc_label="▶"
                                    dec=move || edit_custom(&|c| c.lr = c.lr.saturating_add(30))
                                    inc=move || edit_custom(&|c| c.lr = c.lr.saturating_sub(30)) />
                                <Stepper label="Height" value=c.ud.to_string()
                                    dec=move || edit_custom(&|c| c.ud = c.ud.saturating_sub(100))
                                    inc=move || edit_custom(&|c| c.ud = c.ud.saturating_add(100)) />
                                <Stepper label="Speed" value=(c.speed as i32 - 30).to_string()
                                    dec=move || edit_custom(&|c| c.speed = c.speed.saturating_sub(5))
                                    inc=move || edit_custom(&|c| c.speed += 5) />
                                <Stepper label="Interval" value=format!("{:.1}", c.interval as f32 / 10.0)
                                    dec=move || edit_custom(&|c| c.interval = c.interval.saturating_sub(5))
                                    inc=move || edit_custom(&|c| c.interval += 5) />
                            </div>
                        }
                    }}
                    <div class="row">
                        <span>{move || tr("Play order:")}</span>
                        {(1..=6u8).map(|id| view! { <PointButton id=id list=custom_seq max=p::MAX_SEQUENCE /> }).collect_view()}
                    </div>
                    <div class="run">
                        <span class="with-help"><button on:click=move |_| custom_seq.update(|d| { d.pop(); })>{move || tr("Undo")}</button><Help topic="Undo" /></span>
                        <span class="with-help"><button on:click=move |_| custom_seq.set(Vec::new())>{move || tr("Clear")}</button><Help topic="Clear" /></span>
                        <span class="with-help"><button on:click=move |_| {
                            let d = CustomPoint::defaults(sport.get_untracked());
                            custom.set(d);
                            save_custom(sport.get_untracked(), &d);
                            ctx.run(d.iter().enumerate().map(|(i, c)| (10, p::custom_point(i as u8 + 1, *c))).chain([(50, p::apply())]).collect());
                        }>{move || tr("Reset points")}</button><Help topic="Reset points" /></span>
                    </div>
                    <SavedList title="Saved point sets" noun="point set" names=custom_set_names can_save=Signal::derive(|| true)
                        on_load=load_custom_set on_delete=delete_custom_set on_save=save_custom_set />
                </Show>
            </section>

            <section id="vary-speed" style:display=move || if padel() { "none" } else { "" }>
                <h2>{move || tr("Vary Speed")}</h2>
                <p class="hint">{move || tr("Tap targets in play order (max ")} {p::MAX_DRILL_SHOTS} {move || tr("), far side at the top. Each shot can be tuned below; settings belong to the point, so a repeated point plays the same.")}</p>
                <div class="court">
                    {(0..4u8).rev().map(|row| view! {
                        <div class="court-row">
                            {(1..=7u8).map(|col| view! { <PointButton id=row * 7 + col list=drill max=p::MAX_DRILL_SHOTS /> }).collect_view()}
                        </div>
                    }).collect_view()}
                </div>
                <div class="run">
                    <select prop:value=move || match pause.get() { DrillPause::None => "none", DrillPause::Short => "short", DrillPause::Long => "long" }
                        on:change=move |ev| pause.set(pause_from(&event_target_value(&ev)))>
                        <option value="none">{move || tr("No pause")}</option>
                        <option value="short">{move || tr("Short pause")}</option>
                        <option value="long">{move || tr("Long pause")}</option>
                    </select>
                    <span class="with-help"><button on:click=move |_| drill.update(|d| { d.pop(); })>{move || tr("Undo")}</button><Help topic="Undo" /></span>
                    <span class="with-help"><button on:click=move |_| drill.set(Vec::new())>{move || tr("Clear")}</button><Help topic="Clear" /></span>
                    <span class="with-help"><button class="primary" on:click=start_drill disabled=move || drill.get().is_empty() || !connected()>{move || tr("Start drill")}</button></span>
                </div>

                <div class="shots">
                    {move || {
                        let pts = drill.get();
                        let labels = drill_labels.get();
                        // Labels hold only for the positions where the route still matches them.
                        let named = pts.iter().zip(&labels).take_while(|(id, l)| **id == l.0).count();
                        pts.into_iter().enumerate().map(|(i, id)| {
                        let d = drill_params.get()[(id - 1) as usize];
                        let label = labels.get(i).filter(|_| i < named).cloned();
                        let heading = match label {
                            Some((_, player, en, nl)) => {
                                let badge = player.map(|pl| if pl == 0 {
                                    view! { <i class="key-p1" title=move || tr("Right-hand player")>"1"</i> }.into_any()
                                } else {
                                    view! { <i class="key-p2" title=move || tr("Left-hand player")>"2"</i> }.into_any()
                                });
                                view! { <strong>{format!("#{} · ", i + 1)}{badge}{move || if ui::dutch() { nl.clone() } else { en.clone() }}</strong> }.into_any()
                            }
                            None => view! { <strong>{move || format!("#{} · {} {id}", i + 1, tr("point"))}</strong> }.into_any(),
                        };
                        view! {
                            <div class="shot">
                                {heading}
                                <Stepper label="Left ↔ right" value=p::lr_percent_label(p::drill_lr_percent(d.lr)) dec_label="◀" inc_label="▶"
                                    dec=move || edit_shot(i, &|p| p.lr = p.lr.saturating_add(30))
                                    inc=move || edit_shot(i, &|p| p.lr = p.lr.saturating_sub(30)) />
                                <Stepper label="Height" value=(d.ud / 100).to_string()
                                    dec=move || edit_shot(i, &|p| p.ud = p.ud.saturating_sub(100))
                                    inc=move || edit_shot(i, &|p| p.ud = p.ud.saturating_add(100)) />
                                <Stepper label="Speed" value=(d.speed as i32 - 30).to_string()
                                    dec=move || edit_shot(i, &|p| p.speed = p.speed.saturating_sub(5))
                                    inc=move || edit_shot(i, &|p| p.speed = p.speed.saturating_add(5)) />
                                <Stepper label="Interval (s)" value=format!("{:.1}", d.interval as f32 / 10.0)
                                    dec=move || edit_shot(i, &|p| p.interval = p.interval.saturating_sub(5))
                                    inc=move || edit_shot(i, &|p| p.interval = p.interval.saturating_add(5)) />
                                <Stepper label="Spin (− back / + top)" value=((d.spin as i32 - 50) / 5).to_string()
                                    dec=move || edit_shot(i, &|p| p.spin = p.spin.saturating_sub(5))
                                    inc=move || edit_shot(i, &|p| p.spin = p.spin.saturating_add(5)) />
                            </div>
                        }
                    }).collect_view()
                    }}
                </div>

                <SavedList title="Saved routes" noun="route" names=route_names can_save=Signal::derive(move || !drill.get().is_empty())
                    on_load=load_route on_delete=delete_route on_save=save_route />

                <h3>{move || tr("AI Coach")}</h3>
                <p class="hint">{move || tr("Generates a random drill: the first ball goes deep to the chosen side, the rest anywhere. Level low adds 0.5 s between balls, high removes 0.5 s.")}</p>
                <div class="row">
                    <span>{move || tr("First ball")}</span>
                    <span class="with-help"><button class="mode" class:selected=move || ai.get().left on:click=move |_| ai.update(|a| a.left = true)>{move || tr("Left")}</button><Help topic="Left" /></span>
                    <span class="with-help"><button class="mode" class:selected=move || !ai.get().left on:click=move |_| ai.update(|a| a.left = false)>{move || tr("Right")}</button><Help topic="Right" /></span>
                </div>
                <div class="row">
                    <span>{move || tr("Level")}</span>
                    {[(0u8, "Low"), (1, "Mid"), (2, "High")].into_iter().map(|(lv, l)| view! {
                        <span class="with-help"><button class="mode" class:selected=move || ai.get().level == lv on:click=move |_| ai.update(|a| a.level = lv)>{move || tr(l)}</button></span>
                    }).collect_view()}
                </div>
                <div class="row">
                    <span>{move || tr("Balls")}</span>
                    <span class="with-help"><button on:click=move |_| ai.update(|a| a.balls = a.balls.saturating_sub(1).max(2))>"−"</button><Help topic="Balls −" /></span>
                    <span class="value">{move || ai.get().balls}</span>
                    <span class="with-help"><button on:click=move |_| ai.update(|a| a.balls = (a.balls + 1).min(9))>"+"</button><Help topic="Balls +" /></span>
                    <span class="with-help"><button class="primary" on:click=generate>{move || tr("Generate")}</button><Help topic="Generate" /></span>
                </div>
            </section>
        </main>

        <section id="drills" style:display=move || if padel() { "none" } else { "" }>
            <h2>{move || tr("Drills")}</h2>
            <p class="hint">{move || tr("Ready-made tennis drills for one player or a doubles pair. Start plays one straight away; Edit opens it in Vary Speed so you can tune each ball first.")}</p>
            <div class="row difficulty" role="group" aria-label=move || tr("Difficulty")>
                <span class="with-help">{move || tr("Difficulty")} <Help topic="Difficulty" /></span>
                {DIFFICULTIES.into_iter().map(|(level, en, nl)| view! {
                    <button class="mode" class:selected=move || difficulty.get() == level
                        aria-pressed=move || (difficulty.get() == level).to_string()
                        title=move || if ui::dutch() { nl } else { en }
                        on:click=move |_| set_difficulty(level)>{level}</button>
                }).collect_view()}
                <strong class="difficulty-name">{move || {
                    let (_, en, nl) = DIFFICULTIES[(difficulty.get() - 1) as usize];
                    if ui::dutch() { nl } else { en }
                }}</strong>
            </div>
            {[(false, "Singles"), (true, "Doubles")].into_iter().map(|(doubles, heading)| view! {
                <h3>{move || tr(heading)}</h3>
                <div class="drills">
                    {LIBRARY.iter().filter(move |d| d.doubles == doubles).map(|d| {
                        let name = move || if ui::dutch() { d.name.1 } else { d.name.0 };
                        let level = move || { let (en, nl) = d.level.label(); if ui::dutch() { nl } else { en } };
                        let balls = d.points().len();
                        view! {
                            <article class="drill" id=format!("drill-{}", d.key)
                                class:playing=move || running.get() && active_drill.get() == Some(d.key)>
                                <div class="drill-head">
                                    <span class="drill-name" tabindex="0" data-drill=d.key
                                        class:open=move || open_pop.get() == Some(d.key)
                                        on:mouseenter=move |e| restart_animation(&e)
                                        on:focusin=move |e| { open_pop.set(Some(d.key)); restart_animation(&e) }
                                        on:click=move |e| { open_pop.set(Some(d.key)); restart_on_name_tap(&e) }>
                                        <strong>{name}</strong>
                                        <span class="court-pop" role="tooltip">
                                            <b>{move || tr("Setup")}</b>
                                            <span inner_html=move || drills::court_svg(d, ui::dutch())></span>
                                            <span class="court-key"><i class="key-machine"></i>{move || tr("Pusun 9001 Pro, centre, about 1 m behind the baseline")}</span>
                                            <span class="court-key"><i class="key-p1">"1"</i>{move || tr(if d.doubles { "Player 1, first ball" } else { "Player, first ball" })}</span>
                                            {d.doubles.then(|| view! {
                                                <span class="court-key"><i class="key-p2">"2"</i>{move || tr("Player 2, partner’s first ball")}</span>
                                            })}
                                            <span class="court-key"><i class="key-ball">"1"</i>{move || tr(if d.doubles { "Where each ball lands, in order; ring colour shows the player" } else { "Where each ball lands, in order" })}</span>
                                        </span>
                                    </span>
                                    <span class=format!("level level-{}", d.level.label().0.to_lowercase())>{level}</span>
                                </div>
                                <p>{move || if ui::dutch() { d.description.1 } else { d.description.0 }}</p>
                                <small class="hint">{move || {
                                    let (lo, hi) = drills::interval_range(&d.params_at(difficulty.get()));
                                    let secs = |t: u8| {
                                        let s = format!("{:.1}", t as f32 / 10.0);
                                        if ui::dutch() { s.replace('.', ",") } else { s }
                                    };
                                    let gap = if lo == hi { secs(lo) } else { format!("{}–{}", secs(lo), secs(hi)) };
                                    format!("{balls} {} · {} {gap} s", if balls == 1 { tr("ball") } else { tr("balls") }, tr("every"))
                                }}</small>
                                <div class="row">
                                    <span class="with-help"><button class="primary" disabled=move || !connected()
                                        on:click=move |_| {
                                        load_library_drill(d);
                                        start_current_drill();
                                        active_drill.set(Some(d.key));
                                    }>{move || tr("Start")}</button></span>
                                <span class="with-help"><button class="stop" disabled=move || !connected()
                                    on:click=move |_| stop()>{move || tr("Stop")}</button></span>
                                    <span class="with-help"><button on:click=move |_| {
                                        load_library_drill(d);
                                        show_vary_speed();
                                    }>{move || tr("Edit")}</button><Help topic="Drill edit" /></span>
                                </div>
                            </article>
                        }
                    }).collect_view()}
                </div>
            }).collect_view()}
        </section>

        <div style:display=move || if padel() { "none" } else { "" }>
            <Courses on_practice=practice_course />
        </div>

        <label class="toggle">
            <input type="checkbox" prop:checked=move || advanced.get()
                on:change=move |ev| {
                    let on = event_target_checked(&ev);
                    advanced.set(on);
                    if let Some(st) = storage() { let _ = st.set_item(ADVANCED_KEY, if on { "1" } else { "0" }); }
                } />
            <span>{move || tr("Show advanced (factory) controls")}</span>
        </label>
        <Show when=move || advanced.get()>
            <Advanced ctx=ctx sport=sport status=status height=height height_level=height_level custom=custom />
        </Show>

        <details>
            <summary>{move || tr("BLE log")}</summary>
            <pre>{move || ctx.log.get().join("\n")}</pre>
        </details>
    }
}

#[component]
fn Slider<T>(
    label: &'static str,
    min: i32,
    max: i32,
    #[prop(default = 1)] step: i32,
    value: RwSignal<T>,
    on_commit: impl Fn() + Copy + 'static,
) -> impl IntoView
where
    T: Copy + std::fmt::Display + std::str::FromStr + Send + Sync + 'static,
{
    view! {
        <label>
            <span>{move || tr(label)} " " {move || value.get().to_string()}</span>
            <input type="range" min=min max=max step=step
                prop:value=move || value.get().to_string()
                on:input=move |ev| { if let Ok(v) = event_target_value(&ev).parse() { value.set(v) } }
                on:change=move |_| on_commit() />
        </label>
    }
}

#[component]
fn PatternGroup(
    title: &'static str,
    pattern: RwSignal<Pattern>,
    select: impl Fn(Pattern) + Copy + 'static,
    options: Vec<Pattern>,
) -> impl IntoView {
    view! {
        <h3>{move || tr(title)}</h3>
        <div class="modes">
            {options.into_iter().map(|opt| {
                let label = opt.clone();
                let same = opt.clone();
                let same_help = opt.clone();
                view! {
                    <span class="with-help"><button class="mode"
                        class:selected=move || std::mem::discriminant(&pattern.get()) == std::mem::discriminant(&same) && (matches!(same, Pattern::Program(_) | Pattern::Custom(_)) || pattern.get() == same)
                        on:click=move |_| select(opt.clone())>
                        {move || pattern_name(&label)}
                    </button><Help topic=pattern_help(&same_help) /></span>
                }
            }).collect_view()}
        </div>
    }
}

/// Help topic for a pattern button (see `ui::Help`).
fn pattern_help(p: &Pattern) -> &'static str {
    const LR: [&str; 5] = ["pat:lr1", "pat:lr2", "pat:lr3", "pat:lr4", "pat:lr5"];
    const UD: [&str; 3] = ["pat:ud1", "pat:ud2", "pat:ud3"];
    const CROSS: [&str; 6] = ["pat:cross1", "pat:cross2", "pat:cross3", "pat:cross4", "pat:cross5", "pat:cross6"];
    let idx = |v: u8, n: usize| (v as usize).clamp(1, n) - 1;
    match p {
        Pattern::Fixed(v) => ["pat:fix1", "pat:fix2", "pat:fix3"][idx(*v, 3)],
        Pattern::Volley(v) => ["pat:volley1", "pat:volley2", "pat:volley3"][idx(*v, 3)],
        Pattern::Random => "pat:random",
        Pattern::Primary => "pat:primary",
        Pattern::Smash => "pat:smash",
        Pattern::Moon => "pat:moon",
        Pattern::Bandeja => "pat:bandeja",
        Pattern::Bajada => "pat:bajada",
        Pattern::ServeReturn => "pat:sreturn",
        Pattern::Horizontal(v) => LR[idx(*v, 5)],
        Pattern::Vertical(v) => UD[idx(*v, 3)],
        Pattern::Cross(v) => CROSS[idx(*v, 6)],
        Pattern::Program(_) => "pat:program",
        Pattern::Custom(_) => "pat:custom",
    }
}

/// A numbered target that appends itself to `list`; shows its positions in the order.
#[component]
fn PointButton(id: u8, list: RwSignal<Vec<u8>>, max: usize) -> impl IntoView {
    view! {
        <span class="with-help"><button class="target"
            class:picked=move || list.get().contains(&id)
            on:click=move |_| list.update(|d| if d.len() < max { d.push(id) })>
            <small>{id}</small>
            {move || {
                let order: Vec<String> = list.get().iter().enumerate()
                    .filter(|(_, q)| **q == id).map(|(i, _)| (i + 1).to_string()).collect();
                if order.is_empty() { String::new() } else { format!("#{}", order.join(",")) }
            }}
        </button><Help topic="Target" /></span>
    }
}

#[component]
fn Stepper(
    label: &'static str,
    value: String,
    #[prop(default = "−")] dec_label: &'static str,
    #[prop(default = "+")] inc_label: &'static str,
    dec: impl Fn() + 'static,
    inc: impl Fn() + 'static,
) -> impl IntoView {
    view! {
        <div class="stepper">
            <span>{move || tr(label)}</span>
            <span class="with-help"><button on:click=move |_| dec()>{dec_label}</button><Help topic={if dec_label == "◀" { "Aim left" } else { "Decrease" }} /></span>
            <span class="value">{value}</span>
            <span class="with-help"><button on:click=move |_| inc()>{inc_label}</button><Help topic={if inc_label == "▶" { "Aim right" } else { "Increase" }} /></span>
        </div>
    }
}

/// Choose / Load / Delete / Save row shared by routes, programs and custom-point sets.
#[component]
pub(crate) fn SavedList(
    title: &'static str,
    noun: &'static str,
    names: Signal<Vec<String>>,
    can_save: Signal<bool>,
    on_load: impl Fn(String) + Copy + 'static,
    on_delete: impl Fn(String) + Copy + 'static,
    on_save: impl Fn(String) + Copy + 'static,
) -> impl IntoView {
    let selected = RwSignal::new(String::new());
    view! {
        <h3>{move || tr(title)}</h3>
        <div class="row">
            <select prop:value=move || selected.get() on:change=move |ev| selected.set(event_target_value(&ev))>
                <option value="">{move || match (ui::dutch(), names.get().is_empty()) {
                    (true, true) => format!("Geen opgeslagen {}", saved_noun(noun, true)),
                    (true, false) => format!("Kies een {}…", saved_noun(noun, false)),
                    (false, true) => format!("No saved {noun}s"),
                    (false, false) => format!("Choose a {noun}…"),
                }}</option>
                {move || names.get().into_iter().map(|n| view! { <option value=n.clone()>{n.clone()}</option> }).collect_view()}
            </select>
            <span class="with-help"><button on:click=move |_| on_load(selected.get_untracked()) disabled=move || selected.get().is_empty()>{move || tr("Load")}</button><Help topic="Load" /></span>
            <span class="with-help"><button disabled=move || selected.get().is_empty()
                on:click=move |_| { on_delete(selected.get_untracked()); selected.set(String::new()); }>{move || tr("Delete")}</button><Help topic="Delete" /></span>
            <span class="with-help"><button disabled=move || !can_save.get()
                on:click=move |_| {
                    let prompt = if ui::dutch() { format!("Naam voor dit item ({})", saved_noun(noun, false)) } else { format!("Name for this {noun}") };
                    if let Some(n) = prompt_name(&prompt) { on_save(n.clone()); selected.set(n); }
                }>{move || tr("Save current")}</button><Help topic="Save current" /></span>
        </div>
    }
}

fn saved_noun(noun: &str, plural: bool) -> &str {
    match (noun, plural) {
        ("program", true) => "programma’s",
        ("program", false) => "programma",
        ("point set", true) => "puntensets",
        ("point set", false) => "puntenset",
        ("route", true) => "routes",
        ("drill", true) => "oefeningen",
        ("drill", false) => "oefening",
        _ => "route",
    }
}

fn pattern_name(pattern: &Pattern) -> String {
    match pattern {
        Pattern::Fixed(n) => format!("{} {n}", tr("Fixed point")),
        _ => tr(&pattern.name()).to_owned(),
    }
}

/// Point as currently uploaded: the height table, or the saved custom point on mid.
fn current_point(id: u8, sport: Sport, height: Height, level: u8, custom: &[CustomPoint; p::CUSTOM_POINTS]) -> RawPoint {
    let id = id.clamp(1, p::ALL_POINTS);
    if height == Height::Mid && id > 35 {
        RawPoint::from_frame(&p::custom_point(id - 35, custom[(id - 36) as usize]))
    } else {
        RawPoint::from_frame(&p::calibration(sport, height, level)[(id - 1) as usize])
    }
}

/// Hidden factory controls from the app: "read value" (`0x71`) and the raw point editor.
#[component]
fn Advanced(
    ctx: Ctx,
    sport: RwSignal<Sport>,
    status: RwSignal<Option<(u8, u8)>>,
    height: RwSignal<Height>,
    height_level: RwSignal<u8>,
    custom: RwSignal<[CustomPoint; p::CUSTOM_POINTS]>,
) -> impl IntoView {
    let connected = move || ctx.name.get().is_some();
    let current = move |id: u8| current_point(id, sport.get_untracked(), height.get_untracked(), height_level.get_untracked(), &custom.get_untracked());
    let point = RwSignal::new(current(1));
    let load = move |id: u8| point.set(current(id));
    let read = move |_| {
        status.set(None);
        ctx.run(vec![(0, p::read_value())]);
    };
    let write = move |_| {
        let pt = point.get_untracked();
        ctx.push_log(format!("Point editor: writing point {}", pt.id));
        ctx.run(vec![(0, pt.frame()), (50, p::apply())]);
    };
    let field = move |label: &'static str, max: u32, get: fn(&RawPoint) -> u32, set: fn(&mut RawPoint, u32)| view! {
        <label class="field">
            <span>{move || tr(label)}</span>
            <input type="number" min=0 max=max
                prop:value=move || get(&point.get()).to_string()
                on:change=move |ev| {
                    let v = event_target_value(&ev).trim().parse::<u32>().unwrap_or(0).min(max);
                    point.update(|pt| set(pt, v));
                } />
        </label>
    };
    view! {
        <section class="advanced" class:disabled=move || !connected()>
            <h2>{move || tr("Advanced")}</h2>
            <p class="warn">{move || tr("Hidden factory controls from the official app. They were never shown to users and have not been tested on a machine. Use at your own risk.")}</p>

            <h3>{move || tr("Read value")}</h3>
            <div class="row">
                <span class="with-help"><button on:click=read>{move || tr("Read value (0x71)")}</button><Help topic="Read value (0x71)" /></span>
                <span class="value">{move || match status.get() {
                    Some((a, b)) => format!("va1 = {a}, va2 = {b}"),
                    None => tr("No 0x30 reply yet").into(),
                }}</span>
            </div>

            <h3>{move || tr("Point editor")}</h3>
            <p class="hint">{move || tr("Writes one stored court point with unchecked values, then sends apply. Points 1–35 are rewritten from the height table on reconnect or height change; 36–41 from the custom points.")}</p>
            <div class="row">
                <label class="field">
                    <span>{move || tr("Point (1–41)")}</span>
                    <input type="number" min=1 max=p::ALL_POINTS
                        prop:value=move || point.get().id.to_string()
                        on:change=move |ev| load(event_target_value(&ev).trim().parse::<u8>().unwrap_or(1).clamp(1, p::ALL_POINTS)) />
                </label>
                <span class="with-help"><button on:click=move |_| load(point.get_untracked().id) title=move || tr("Reload the values currently uploaded for this point")>{move || tr("Load current")}</button><Help topic="Load current" /></span>
            </div>
            <div class="fields">
                {field("Left ↔ right", u16::MAX as u32, |pt| pt.lr as u32, |pt, v| pt.lr = v as u16)}
                {field("Height", u16::MAX as u32, |pt| pt.ud as u32, |pt, v| pt.ud = v as u16)}
                {field("Speed", u16::MAX as u32, |pt| pt.speed as u32, |pt, v| pt.speed = v as u16)}
                {field("Interval", u8::MAX as u32, |pt| pt.interval as u32, |pt, v| pt.interval = v as u8)}
            </div>
            <p class="hint mono">{move || p::hex(&point.get().frame())}</p>
            <div class="row">
                <span class="with-help"><button class="primary" on:click=write>{move || tr("Write point")}</button><Help topic="Write point" /></span>
            </div>
        </section>
    }
}

/// Rewinds the court animation inside the event's target, so the setup popup always
/// plays its drill from the first ball. The popup's SVG keeps running while hidden.
fn restart_animation(e: &web_sys::Event) {
    use wasm_bindgen::JsCast;
    let Some(el) = e.current_target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
    let Ok(Some(svg)) = el.query_selector("svg") else { return };
    if let Ok(f) = js_sys::Reflect::get(&svg, &"setCurrentTime".into()) {
        if let Some(f) = f.dyn_ref::<js_sys::Function>() {
            let _ = f.call1(&svg, &0.into());
        }
    }
}

/// A second tap on the name of an open setup popup restarts it too; taps inside the
/// popup leave it running.
fn restart_on_name_tap(e: &web_sys::Event) {
    use wasm_bindgen::JsCast;
    let in_popup = e
        .target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .and_then(|el| el.closest(".court-pop").ok().flatten())
        .is_some();
    if !in_popup {
        restart_animation(e);
    }
}
