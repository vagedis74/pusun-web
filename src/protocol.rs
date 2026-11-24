//! Pusun PT-9001 (PT7/PT8 BLE name family) wire protocol.
//!
//! Recovered from `com.pusun.pusuntennis.MainActivity7New` in the Pusun Android
//! app v2.3. Every frame is 10 bytes: `AA cmd d0 d1 d2 d3 d4 d5 chk A5`.
//! `chk` is normally the XOR of bytes 1..=7, but the app sends a few frames
//! with fixed trailing bytes that do not follow that rule. Those are
//! reproduced byte-for-byte (see the `quirk` notes) because the firmware's
//! checksum handling is unknown.

use crate::tables::{CALIBRATION, DRILL_POINTS, HEIGHT_HIGH, HEIGHT_LOW, PADEL_CALIBRATION};

pub type Frame = [u8; 10];

pub const HEAD: u8 = 0xAA;
pub const TAIL: u8 = 0xA5;

/// Machine speed for slider levels 1..=13 (`veloNums`).
pub const SPEED_TABLE: [u8; 13] = [76, 83, 90, 95, 105, 115, 125, 135, 145, 155, 160, 170, 180];
/// Padel screen (`MainActivityPad2.veloNums`): slower balls on the same 13 levels.
pub const PADEL_SPEED_TABLE: [u8; 13] = [66, 76, 80, 83, 87, 91, 95, 100, 105, 115, 125, 135, 145];

/// Which of the app's two control screens for this machine is being reproduced:
/// tennis (`MainActivity7New`) or padel ("Paddle Tennis", `MainActivityPad2`).
/// They share the frame format and differ in speed table, mid court table,
/// fine-height range and pattern presets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sport {
    Tennis,
    Padel,
}

impl Sport {
    fn speed_table(self) -> &'static [u8; 13] {
        match self {
            Sport::Tennis => &SPEED_TABLE,
            Sport::Padel => &PADEL_SPEED_TABLE,
        }
    }

    fn mid_table(self) -> &'static [[i16; 4]; 41] {
        match self {
            Sport::Tennis => &CALIBRATION,
            Sport::Padel => &PADEL_CALIBRATION,
        }
    }

    /// Highest fine height level: 16 on the tennis screen, 30 on the padel screen.
    pub fn height_level_max(self) -> u8 {
        match self {
            Sport::Tennis => HEIGHT_LEVEL_MAX,
            Sport::Padel => PADEL_HEIGHT_LEVEL_MAX,
        }
    }

    /// Height of the aim the app sends when a horizontal or vertical pattern is
    /// selected; left/right is `BasicData3.a11[0]` (1320) on both screens.
    fn sweep_aim(self) -> (u16, u16) {
        match self {
            Sport::Tennis => (1320, 900),
            Sport::Padel => (1320, 2000),
        }
    }
}
/// Feed interval for slider levels 1..=10 (`frequentNums`); higher level = faster feed.
pub const FREQ_TABLE: [u8; 10] = [88, 78, 68, 58, 48, 38, 33, 28, 23, 18];

pub const SPIN_MAX: i8 = 6;
pub const LR_MIN: u16 = 210;
pub const LR_MAX: u16 = 2070;
pub const UD_MAX: u16 = 4300;
/// The app caps height at 2000 in the fixed and horizontal modes.
pub const UD_MAX_LOW_MODES: u16 = 2000;
pub const MAX_DRILL_SHOTS: usize = 15;

/// Builds a frame whose checksum is the XOR of bytes 1..=7.
pub fn frame(cmd: u8, d: [u8; 6]) -> Frame {
    let chk = d.iter().fold(cmd, |acc, b| acc ^ b);
    [HEAD, cmd, d[0], d[1], d[2], d[3], d[4], d[5], chk, TAIL]
}

/// Builds a frame with an explicit trailing byte (app quirks and data frames).
fn frame_raw(cmd: u8, d: [u8; 6], trailer: u8) -> Frame {
    [HEAD, cmd, d[0], d[1], d[2], d[3], d[4], d[5], trailer, TAIL]
}

fn be(v: i16) -> [u8; 2] {
    v.to_be_bytes()
}

/// `0x66`: sent after every parameter change and at start. quirk: trailer 0x67.
pub fn apply() -> Frame {
    frame_raw(0x66, [0; 6], 0x67)
}

/// `0x66` with trailer 0x66: what the padel screen sends instead of [`apply`] when
/// fixed point or volley is reselected while running.
pub fn apply_padel_reselect() -> Frame {
    frame_raw(0x66, [0; 6], 0x66)
}

/// `0x67`: request battery level (answered by a `0x03` notification).
pub fn battery_query() -> Frame {
    frame(0x67, [0; 6])
}

/// `0x6B`: stop feeding balls.
pub fn stop() -> Frame {
    frame(0x6B, [0; 6])
}

/// `0x63`: ball speed, level 1..=13, through the screen's speed table.
pub fn speed(sport: Sport, level: u8) -> Frame {
    let v = sport.speed_table()[(level.clamp(1, 13) - 1) as usize];
    frame(0x63, [v, 0, 0, 0, 0, 0])
}

/// `0x61`: feed frequency, level 1..=10.
pub fn frequency(level: u8) -> Frame {
    let v = FREQ_TABLE[(level.clamp(1, 10) - 1) as usize];
    frame(0x61, [v, 0, 0, 0, 0, 0])
}

/// `0x62`: spin, level -6..=6. Positive = topspin, negative = backspin.
pub fn spin(level: i8) -> Frame {
    let level = level.clamp(-SPIN_MAX, SPIN_MAX);
    match level {
        0 => frame(0x62, [0; 6]),
        l if l > 0 => frame(0x62, [1, (l as u8) * 5, 0, 0, 0, 0]),
        l => frame(0x62, [2, (-l as u8) * 5, 0, 0, 0, 0]),
    }
}

/// `0x6C`: aim position. quirk: trailer is always 0x01.
pub fn position(lr: u16, ud: u16) -> Frame {
    let [a, b] = be(lr as i16);
    let [c, d] = be(ud as i16);
    frame_raw(0x6C, [a, b, c, d, 0, 0], 0x01)
}

/// `0x64`: horizontal travel limits sent on connect (max 2070, min 210). quirk: trailer 0x01.
pub fn lr_limits() -> Frame {
    let [a, b] = be(LR_MAX as i16);
    let [c, d] = be(LR_MIN as i16);
    frame_raw(0x64, [a, b, c, d, 0, 0], 0x01)
}

/// `0x65`: vertical limits sent on connect (1000 and 700). quirk: trailer 0x01.
pub fn ud_limits() -> Frame {
    let [a, b] = be(1000);
    let [c, d] = be(700);
    frame_raw(0x65, [a, b, c, d, 0, 0], 0x01)
}

/// Height setting (the app's "Height: low / mid / high" radio buttons).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Height {
    Low,
    Mid,
    High,
}

/// Fine height level for the mid table (`valueSelect`), 4..=16. The app's default is 10.
pub const HEIGHT_LEVEL_DEFAULT: u8 = 10;
pub const HEIGHT_LEVEL_MIN: u8 = 4;
pub const HEIGHT_LEVEL_MAX: u8 = 16;
pub const PADEL_HEIGHT_LEVEL_MAX: u8 = 30;

/// Court-point table upload (`sendBaseData`). The app sends the mid table ~1.5 s
/// after connecting and whenever the height setting changes.
///
/// Mid uploads all 41 points (grid + custom) and shifts the 35 grid points by
/// `(10 - level) * 100`. Low and high upload only the 35 grid points.
/// The padel screen uses its own mid table and the same low and high tables.
pub fn calibration(sport: Sport, height: Height, level: u8) -> Vec<Frame> {
    match height {
        Height::Mid => {
            let adjust = (HEIGHT_LEVEL_DEFAULT as i16 - level.clamp(HEIGHT_LEVEL_MIN, sport.height_level_max()) as i16) * 100;
            sport
                .mid_table()
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let ud = if i < 35 { r[1] - adjust } else { r[1] };
                    point_frame(i as u8 + 1, [r[0], ud, r[2], r[3]])
                })
                .collect()
        }
        Height::Low | Height::High => {
            let table = if height == Height::Low { &HEIGHT_LOW } else { &HEIGHT_HIGH };
            table
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let [a, b] = be(r[0]);
                    let [c, d] = be(r[1]);
                    frame((i + 1) as u8, [a, b, c, d, 0, 0])
                })
                .collect()
        }
    }
}

/// One stored court point: `AA id lr lr ud ud spd spd interval A5`.
fn point_frame(id: u8, [lr, ud, speed, interval]: [i16; 4]) -> Frame {
    let [a, b] = be(lr);
    let [c, d] = be(ud);
    let [e, f] = be(speed);
    frame_raw(id, [a, b, c, d, e, f], interval as u8)
}

/// A user-editable custom point (ids 36..=41 on the machine).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CustomPoint {
    pub lr: u16,
    pub ud: u16,
    /// 40..=140 in steps of 5; the app shows `speed - 30`.
    pub speed: u16,
    /// 25..=65 in steps of 5; the app shows `interval / 10`.
    pub interval: u8,
}

pub const CUSTOM_POINTS: usize = 6;
pub const CUSTOM_SPEED: (u16, u16) = (40, 140);
pub const CUSTOM_INTERVAL: (u8, u8) = (25, 65);

impl CustomPoint {
    pub fn defaults(sport: Sport) -> [CustomPoint; CUSTOM_POINTS] {
        std::array::from_fn(|i| {
            let r = sport.mid_table()[35 + i];
            CustomPoint { lr: r[0] as u16, ud: r[1] as u16, speed: r[2] as u16, interval: r[3] as u8 }
        })
    }

    pub fn clamped(self) -> CustomPoint {
        CustomPoint {
            lr: self.lr.clamp(LR_MIN, LR_MAX),
            ud: self.ud.min(UD_MAX),
            speed: self.speed.clamp(CUSTOM_SPEED.0, CUSTOM_SPEED.1),
            interval: self.interval.clamp(CUSTOM_INTERVAL.0, CUSTOM_INTERVAL.1),
        }
    }
}

/// Uploads custom point `n` (1..=6) to machine id `n + 35`.
pub fn custom_point(n: u8, p: CustomPoint) -> Frame {
    let p = p.clamped();
    point_frame(n.clamp(1, 6) + 35, [p.lr as i16, p.ud as i16, p.speed as i16, p.interval as i16])
}

/// A court point as stored on the machine (ids 1..=41).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawPoint {
    pub id: u8,
    pub lr: u16,
    pub ud: u16,
    pub speed: u16,
    pub interval: u8,
}

impl RawPoint {
    /// Decodes a point frame (`AA id lr lr ud ud spd spd interval A5`).
    pub fn from_frame(f: &Frame) -> RawPoint {
        RawPoint {
            id: f[1],
            lr: u16::from_be_bytes([f[2], f[3]]),
            ud: u16::from_be_bytes([f[4], f[5]]),
            speed: u16::from_be_bytes([f[6], f[7]]),
            interval: f[8],
        }
    }

    /// Hidden factory point editor (`u_dian`): writes any point with unchecked values.
    /// Only the id is clamped to 1..=41 so the frame cannot collide with a command.
    pub fn frame(self) -> Frame {
        let [a, b] = self.lr.to_be_bytes();
        let [c, d] = self.ud.to_be_bytes();
        let [e, f] = self.speed.to_be_bytes();
        frame_raw(self.id.clamp(1, ALL_POINTS), [a, b, c, d, e, f], self.interval)
    }
}

/// `0x71`: hidden factory "read value" (读值) button. quirk: trailer 0x01.
/// The reply is presumably the `0x30` notification, which the app only logs.
pub fn read_value() -> Frame {
    frame_raw(0x71, [0; 6], 0x01)
}

/// `0x6D` pages: up to 28 ids in play order, five per page, six pages.
pub fn sequence_pages(ids: &[u8]) -> [Frame; 6] {
    let mut s = [0u8; 30];
    for (i, &id) in ids.iter().take(MAX_SEQUENCE).enumerate() {
        s[i] = id;
    }
    std::array::from_fn(|page| {
        let o = page * 5;
        let d = if page == 5 { [6, s[o], s[o + 1], s[o + 2], 0, 0] } else { [page as u8 + 1, s[o], s[o + 1], s[o + 2], s[o + 3], s[o + 4]] };
        frame(0x6D, d)
    })
}

pub const MAX_SEQUENCE: usize = 28;
pub const ALL_POINTS: u8 = 41;

/// The app's training patterns (main screen of the PT7/PT8 control page).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pattern {
    /// Fixed point 1/2/3: center, right, left.
    Fixed(u8),
    Smash,
    /// Volley 1/2/3: center, right, left.
    Volley(u8),
    /// "Primary": slow beginner balls.
    Primary,
    Moon,
    Random,
    /// 1 random, 2 wide line, 3 middle line, 4 narrow line, 5 three line.
    Horizontal(u8),
    /// 1 random, 2 circulation, 3 deep and light.
    Vertical(u8),
    /// Crosscourt 1..=6, see [`CROSS_NAMES`].
    Cross(u8),
    /// Program panel: court point ids 1..=41 in play order.
    Program(Vec<u8>),
    /// Custom points 1..=6 in play order.
    Custom(Vec<u8>),
    /// Padel only: overhead bandeja, fixed mode.
    Bandeja,
    /// Padel only: bajada (off the back wall), fixed mode.
    Bajada,
    /// Padel only: serve return, fixed mode.
    ServeReturn,
}

pub const HORIZONTAL_NAMES: [&str; 5] = ["Horizontal random", "Wide line", "Middle line", "Narrow line", "Three line"];
pub const VERTICAL_NAMES: [&str; 3] = ["Vertical random", "Vertical circulation", "Deep and light"];
pub const CROSS_NAMES: [&str; 6] = [
    "Left deep & middle light",
    "Right deep & middle light",
    "Left light & middle deep",
    "Middle deep & right light",
    "Left deep & right light",
    "Left light & right deep",
];
/// Court point pairs for each crosscourt variant (`$96`/`$119`).
const CROSS_POINTS: [(u8, u8); 6] = [(22, 4), (28, 4), (25, 1), (25, 7), (22, 7), (28, 1)];
const SIDE_LR: [u16; 3] = [1140, 210, 2070];

impl Pattern {
    pub fn name(&self) -> String {
        match self {
            Pattern::Fixed(v) => format!("Fixed point {v}"),
            Pattern::Smash => "Smash".into(),
            Pattern::Volley(v) => format!("Volley {v}"),
            Pattern::Primary => "Primary".into(),
            Pattern::Moon => "Moon".into(),
            Pattern::Random => "Random".into(),
            Pattern::Horizontal(v) => HORIZONTAL_NAMES[(*v).clamp(1, 5) as usize - 1].into(),
            Pattern::Vertical(v) => VERTICAL_NAMES[(*v).clamp(1, 3) as usize - 1].into(),
            Pattern::Cross(v) => CROSS_NAMES[(*v).clamp(1, 6) as usize - 1].into(),
            Pattern::Program(_) => "Program panel".into(),
            Pattern::Custom(_) => "Custom points".into(),
            Pattern::Bandeja => "Bandeja".into(),
            Pattern::Bajada => "Bajada".into(),
            Pattern::ServeReturn => "Serve return".into(),
        }
    }

    /// Settings the app applies when the pattern is selected, per screen.
    pub fn preset(&self, sport: Sport) -> Preset {
        let side = |v: u8| SIDE_LR[(v.clamp(1, 3) - 1) as usize];
        let padel = sport == Sport::Padel;
        let pick = |tennis: u16, padel_ud: u16| if padel { padel_ud } else { tennis };
        // quirk: trailer 0x63 instead of 0x62.
        let no_spin = (0, frame_raw(0x62, [0; 6], 0x63));
        // quirk: trailer 0x7A instead of 0x7D.
        let moon_spin = (6, frame_raw(0x62, [1, 30, 0, 0, 0, 0], 0x7A));
        let aim = |x: u16, y: u16, speed: u8| Preset { aim: Some((x, y)), speed: Some(speed), ..Preset::default() };
        match self {
            Pattern::Fixed(v) => aim(side(*v), pick(1200, 2000), 7),
            Pattern::Smash => aim(1140, pick(4200, 3600), 4),
            Pattern::Volley(v) => aim(side(*v), pick(1000, 1800), 6),
            // Speed only drops to 1 if above 2.
            Pattern::Primary => Preset { aim: Some((1140, pick(1600, 2300))), speed_max: Some((2, 1)), spin: Some(no_spin), ..Preset::default() },
            Pattern::Moon => Preset { spin: Some(moon_spin), ..aim(1140, pick(1600, 3000), 5) },
            Pattern::Horizontal(_) | Pattern::Vertical(_) => {
                let (x, y) = sport.sweep_aim();
                aim(x, y, 7)
            }
            Pattern::Bandeja => aim(1140, 4200, 3),
            Pattern::Bajada => aim(1140, 3800, 4),
            Pattern::ServeReturn => aim(1140, 2000, 2),
            _ => Preset { speed: Some(7), ..Preset::default() },
        }
    }

    /// Frames sent by Start, with the delay before each (ms).
    pub fn start(&self) -> Vec<(u32, Frame)> {
        let fixed = frame(0x6A, [1, 0, 0, 0, 0, 0]);
        let mut steps: Vec<(u32, Frame)> = match self {
            Pattern::Fixed(_) | Pattern::Smash | Pattern::Volley(_) | Pattern::Primary | Pattern::Moon | Pattern::Bandeja | Pattern::Bajada | Pattern::ServeReturn => {
                vec![(200, fixed)]
            }
            Pattern::Random => vec![(0, frame(0x6A, [4, 0, 0, 0, 0, 0]))],
            Pattern::Horizontal(v) => vec![(0, HORIZONTAL_FRAMES[((*v).clamp(1, 5) - 1) as usize])],
            Pattern::Vertical(v) => vec![(0, VERTICAL_FRAMES[((*v).clamp(1, 3) - 1) as usize])],
            Pattern::Cross(v) => {
                let (a, b) = CROSS_POINTS[((*v).clamp(1, 6) - 1) as usize];
                sequence_with_mode(&[a, b], frame(0x6A, [5, 0, 0, 0, 0, 0]))
            }
            Pattern::Program(ids) => {
                let ids: Vec<u8> = ids.iter().copied().filter(|&p| (1..=ALL_POINTS).contains(&p)).collect();
                sequence_with_mode(&ids, frame(0x6A, [5, 0, 0, 0, 0, 0]))
            }
            Pattern::Custom(ids) => {
                let ids: Vec<u8> = ids.iter().filter(|&&p| (1..=6).contains(&p)).map(|p| p + 35).collect();
                // quirk: trailer 0x6F instead of 0x6C.
                sequence_with_mode(&ids, frame_raw(0x6A, [6, 0, 0, 0, 0, 0], 0x6F))
            }
        };
        let elapsed: u32 = steps.iter().map(|s| s.0).sum();
        steps.push((270u32.saturating_sub(elapsed).max(30), apply()));
        steps
    }

    /// Key the app stores this pattern's saved default under (`DefaultDao.daoName`).
    /// Program and custom patterns have no defaults.
    pub fn default_key(&self) -> Option<String> {
        Some(match self {
            Pattern::Fixed(v) => format!("fix{v}"),
            Pattern::Volley(v) => format!("hit{v}"),
            Pattern::Smash => "high".into(),
            Pattern::Random => "whole".into(),
            Pattern::Horizontal(v) => format!("lr{v}"),
            Pattern::Vertical(v) => format!("ud{v}"),
            Pattern::Cross(v) => format!("cross{v}"),
            Pattern::Primary => "place".into(),
            Pattern::Moon => "moon".into(),
            Pattern::Bandeja => "bandeja".into(),
            Pattern::Bajada => "bajada".into(),
            Pattern::ServeReturn => "sreturn".into(),
            Pattern::Program(_) | Pattern::Custom(_) => return None,
        })
    }

    /// Program and custom patterns need at least one point.
    pub fn is_ready(&self) -> bool {
        match self {
            Pattern::Program(p) | Pattern::Custom(p) => !p.is_empty(),
            _ => true,
        }
    }
}

fn sequence_with_mode(ids: &[u8], mode: Frame) -> Vec<(u32, Frame)> {
    let mut v: Vec<(u32, Frame)> = sequence_pages(ids).into_iter().enumerate().map(|(i, f)| (if i == 0 { 5 } else { 40 }, f)).collect();
    v.push((40, mode));
    v
}

/// Settings applied when a pattern is selected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Preset {
    pub aim: Option<(u16, u16)>,
    pub speed: Option<u8>,
    /// `(threshold, level)`: lower speed to `level` if it is above `threshold`.
    pub speed_max: Option<(u8, u8)>,
    /// `(slider level, exact frame the app sends)`.
    pub spin: Option<(i8, Frame)>,
}

/// A pattern's saved default (`DefaultDao`): aim, slider levels and fine height.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PatternDefault {
    pub lr: u16,
    pub ud: u16,
    pub freq: u8,
    pub speed: u8,
    pub spin: i8,
    /// Fine height level (`valueSelect`); 0 means not set.
    pub height_level: u8,
}

impl PatternDefault {
    /// Frames the app sends when a pattern with a saved default is selected
    /// (`setDefaultMode`): aim, then frequency, speed and spin, then the mid court
    /// table if a height level is stored. Delays are relative to the previous step.
    /// quirk: the aim is sent as stored, without the per-pattern masking.
    pub fn apply_steps(&self, sport: Sport) -> Vec<(u32, Frame)> {
        let mut v = vec![
            (0, position(self.lr, self.ud)),
            (30, frequency(self.freq)),
            (50, speed(sport, self.speed)),
            (40, spin(self.spin)),
        ];
        if self.height_level != 0 {
            let mut table = calibration(sport, Height::Mid, self.height_level).into_iter();
            v.push((40, table.next().unwrap()));
            v.extend(table.map(|f| (10, f)));
        }
        v
    }

    /// Whether the current values differ from this default, ignoring aim on
    /// horizontal patterns and height on vertical ones (the app hides those controls).
    pub fn differs(&self, pattern: &Pattern, current: &PatternDefault) -> bool {
        let horizontal = matches!(pattern, Pattern::Horizontal(_));
        let vertical = matches!(pattern, Pattern::Vertical(_));
        (!horizontal && self.lr != current.lr)
            || (!vertical && self.ud != current.ud)
            || self.freq != current.freq
            || self.speed != current.speed
            || self.spin != current.spin
            || self.height_level != current.height_level
    }
}

/// Settings stored with a named Program-panel program (`SeleDao`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgramSettings {
    pub freq: u8,
    pub speed: u8,
    pub spin: i8,
    /// Height table active when saved. quirk: the app stores it but always loads mid.
    pub height: Height,
    /// Fine height level; 0 means keep the current level.
    pub height_level: u8,
}

impl ProgramSettings {
    /// Frames sent when a saved program is loaded: apply, frequency, speed and spin,
    /// then the mid court table at `level` (the saved level, or the current one if 0).
    pub fn load_steps(&self, sport: Sport, current_level: u8) -> Vec<(u32, Frame)> {
        let level = if self.height_level == 0 { current_level } else { self.height_level };
        let mut v = vec![(5, apply()), (25, frequency(self.freq)), (50, speed(sport, self.speed)), (40, spin(self.spin))];
        let mut table = calibration(sport, Height::Mid, level).into_iter();
        v.push((40, table.next().unwrap()));
        v.extend(table.map(|f| (10, f)));
        v
    }
}

/// Frames sent when a named custom-point set (`VariDao`) is loaded: apply, then all
/// six custom points.
pub fn custom_set_load_steps(points: &[CustomPoint; CUSTOM_POINTS]) -> Vec<(u32, Frame)> {
    let mut v = vec![(5, apply())];
    v.extend(points.iter().enumerate().map(|(i, c)| (if i == 0 { 55 } else { 10 }, custom_point(i as u8 + 1, *c))));
    v
}

/// Stop: `6B`, battery query after 190 ms, apply after 260 ms.
pub fn stop_sequence() -> Vec<(u32, Frame)> {
    vec![(0, stop()), (190, battery_query()), (70, apply())]
}

/// `0x6A` mode frames exactly as the app sends them. quirk: `02 10 14` carries
/// trailer 0x6E although the XOR rule gives 0x6C.
const HORIZONTAL_FRAMES: [Frame; 5] = [
    [HEAD, 0x6A, 2, 0, 0, 0, 0, 0, 104, TAIL],
    [HEAD, 0x6A, 2, 15, 21, 0, 0, 0, 114, TAIL],
    [HEAD, 0x6A, 2, 16, 20, 0, 0, 0, 110, TAIL],
    [HEAD, 0x6A, 2, 17, 19, 0, 0, 0, 106, TAIL],
    [HEAD, 0x6A, 2, 15, 18, 21, 0, 0, 96, TAIL],
];
const VERTICAL_FRAMES: [Frame; 3] = [
    [HEAD, 0x6A, 3, 0, 0, 0, 0, 0, 105, TAIL],
    [HEAD, 0x6A, 3, 4, 11, 18, 25, 32, 77, TAIL],
    [HEAD, 0x6A, 3, 4, 25, 0, 0, 0, 116, TAIL],
];

/// Pause after the last shot of a drill before it repeats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrillPause {
    None,
    Short,
    Long,
}

impl DrillPause {
    fn extra(self) -> u8 {
        match self {
            DrillPause::None => 0,
            DrillPause::Short => 20,
            DrillPause::Long => 40,
        }
    }
}

/// Parameters of one "Vary Speed" target point (`BasicData30.b3` row).
/// Parameters belong to the point: a point used twice in a drill plays the same way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrillPoint {
    /// 90..=2370, steps of 30; higher is further left.
    pub lr: u16,
    /// 300..=4500, steps of 100.
    pub ud: u16,
    /// 50..=140, steps of 5; the app shows `speed - 30`.
    pub speed: u8,
    /// Feed interval in tenths of a second, 25..=65, steps of 5.
    pub interval: u8,
    /// 20..=80, steps of 5; 50 = none. The app shows `(spin - 50) / 5`.
    pub spin: u8,
}

pub const DRILL_LR: (u16, u16) = (90, 2370);
pub const DRILL_UD: (u16, u16) = (300, 4500);
pub const DRILL_SPEED: (u8, u8) = (50, 140);
pub const DRILL_INTERVAL: (u8, u8) = (25, 65);
pub const DRILL_SPIN: (u8, u8) = (20, 80);
/// Interval the app resets every point to when the Vary Speed screen opens.
pub const DRILL_INTERVAL_DEFAULT: u8 = 40;

impl DrillPoint {
    pub fn defaults() -> [DrillPoint; 28] {
        std::array::from_fn(|i| {
            let r = DRILL_POINTS[i];
            DrillPoint { lr: r[0] as u16, ud: r[1] as u16, speed: r[2] as u8, interval: r[3] as u8, spin: r[4] as u8 }
        })
    }

    /// Clamps to the ranges the app's editor allows.
    pub fn clamped(self) -> DrillPoint {
        DrillPoint {
            lr: self.lr.clamp(DRILL_LR.0, DRILL_LR.1),
            ud: self.ud.clamp(DRILL_UD.0, DRILL_UD.1),
            speed: self.speed.clamp(DRILL_SPEED.0, DRILL_SPEED.1),
            interval: self.interval.clamp(DRILL_INTERVAL.0, DRILL_INTERVAL.1),
            spin: self.spin.clamp(DRILL_SPIN.0, DRILL_SPIN.1),
        }
    }
}

/// Shot slot `position` (1-based) of a drill: `AA 23+pos lr lr ud ud spin speed interval A5`.
/// The last shot carries the pause in its interval byte.
pub fn drill_slot(position: u8, p: DrillPoint, last: bool, pause: DrillPause) -> Frame {
    let [a, b] = be(p.lr as i16);
    let [c, d] = be(p.ud as i16);
    let interval = p.interval.wrapping_add(if last { pause.extra() } else { 0 });
    frame_raw(0x23 + position, [a, b, c, d, p.spin, p.speed], interval)
}

/// Full frame sequence that uploads and starts a drill (`dealSpinvaryStartEvent`).
/// `points` are grid indices 1..=28 (at most 15 used), `params` the per-point settings.
/// `date_code` comes from [`date_code`]; firmware before 250101 gets mode 6, later mode 8.
pub fn drill(points: &[u8], params: &[DrillPoint; 28], pause: DrillPause, date_code: Option<u32>) -> Vec<Frame> {
    let points = &points[..points.len().min(MAX_DRILL_SHOTS)];
    let mut out = Vec::with_capacity(points.len() + 8);
    for (i, &p) in points.iter().enumerate() {
        let last = i == points.len() - 1;
        out.push(drill_slot(i as u8 + 1, params[(p.clamp(1, 28) - 1) as usize], last, pause));
    }
    let slots: Vec<u8> = (0..points.len()).map(|i| 0x24 + i as u8).collect();
    out.extend(sequence_pages(&slots));
    // quirk: the app sends trailer 0x6F for both values.
    let m = if is_old_firmware(date_code) { 6 } else { 8 };
    out.push(frame_raw(0x6A, [m, 0, 0, 0, 0, 0], 0x6F));
    out.push(apply());
    out
}

fn is_old_firmware(date_code: Option<u32>) -> bool {
    date_code.is_some_and(|c| c < 250101)
}

/// Live update after editing a shot (`dealSpinvaryEvent`): the slot, apply after
/// 50 ms, and on firmware before 250101 a separate spin command after 100 ms.
/// quirk: that spin command has trailer 0.
pub fn drill_edit(position: u8, p: DrillPoint, last: bool, pause: DrillPause, date_code: Option<u32>) -> Vec<(u32, Frame)> {
    let mut v = vec![(0, drill_slot(position, p, last, pause)), (50, apply())];
    if is_old_firmware(date_code) && p.spin != 50 {
        let (dir, amount) = if p.spin > 50 { (1, p.spin - 50) } else { (2, 50 - p.spin) };
        v.push((50, frame_raw(0x62, [dir, amount, 0, 0, 0, 0], 0)));
    }
    v
}

/// AI Coach options (`AIDrillActivity`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AiOptions {
    /// First ball side.
    pub left: bool,
    /// 0 low, 1 mid, 2 high. Low adds 0.5 s to each picked point's interval, high removes it.
    pub level: u8,
    /// 2..=9.
    pub balls: u8,
}

const AI_LEFT: [u8; 6] = [17, 16, 15, 24, 23, 22];
const AI_RIGHT: [u8; 6] = [19, 20, 21, 26, 27, 28];

/// Generates an AI Coach drill like the app: resets every interval to 4.0 s, picks the
/// first ball from the chosen side's deep zone and the rest from all 28 points, and
/// shifts each pick's interval by the level (repeated picks shift again, as in the app).
/// `rand(n)` returns a value in `0..n`.
pub fn ai_drill(o: AiOptions, params: &mut [DrillPoint; 28], mut rand: impl FnMut(usize) -> usize) -> Vec<u8> {
    for p in params.iter_mut() {
        p.interval = DRILL_INTERVAL_DEFAULT;
    }
    let side = if o.left { &AI_LEFT } else { &AI_RIGHT };
    (0..o.balls.clamp(2, 9))
        .map(|i| {
            let id = if i == 0 { side[rand(side.len())] } else { rand(28) as u8 + 1 };
            let p = &mut params[id as usize - 1];
            match o.level {
                0 => p.interval = p.interval.wrapping_add(5),
                2 => p.interval = p.interval.wrapping_sub(5),
                _ => {}
            }
            id
        })
        .collect()
}

/// Firmware/production date (YYMMDD) encoded in characters 3..9 of the BLE name.
pub fn date_code(name: &str) -> Option<u32> {
    name.trim().get(3..9)?.parse().ok()
}

/// Whether a BLE name uses the `FFE5`/`FFE0` services instead of `FFF0`.
pub fn uses_alt_services(name: &str) -> bool {
    name.starts_with("PT0") || name.starts_with("PT1")
}

/// Decoded notification from the machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notification {
    /// Raw battery value; the app buckets it at 45/55/61/68.
    Battery(u8),
    Fault(Fault),
    /// `0x30`: two values the app only logs.
    Status(u8, u8),
    Unknown(Vec<u8>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    Wheel,
    BallGate,
    OutOfBalls,
    Other(u8),
}

impl Fault {
    pub fn describe(self) -> String {
        match self {
            Fault::Wheel => "Wheel fault".into(),
            Fault::BallGate => "Ball gate fault".into(),
            Fault::OutOfBalls => "Out of balls".into(),
            Fault::Other(c) => format!("Fault code {c}"),
        }
    }
}

pub fn parse_notification(b: &[u8]) -> Notification {
    match b {
        [_, 0x03, v, ..] => Notification::Battery(*v),
        [_, 0x5E, c, ..] => Notification::Fault(match c {
            1 => Fault::Wheel,
            2 => Fault::BallGate,
            3 => Fault::OutOfBalls,
            c => Fault::Other(*c),
        }),
        [_, 0x30, a, b, ..] => Notification::Status(*a, *b),
        _ => Notification::Unknown(b.to_vec()),
    }
}

/// Battery bars 0..=4 using the app's thresholds.
pub fn battery_bars(v: u8) -> u8 {
    match v {
        0..=44 => 0,
        45..=54 => 1,
        55..=60 => 2,
        61..=67 => 3,
        _ => 4,
    }
}

pub fn hex(f: &[u8]) -> String {
    f.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Expected bytes are literal arrays from MainActivity7New.
    #[test]
    fn fixed_frames_match_app() {
        assert_eq!(apply(), [0xAA, 0x66, 0, 0, 0, 0, 0, 0, 0x67, 0xA5]);
        assert_eq!(battery_query(), [0xAA, 0x67, 0, 0, 0, 0, 0, 0, 0x67, 0xA5]);
        assert_eq!(stop(), [0xAA, 0x6B, 0, 0, 0, 0, 0, 0, 0x6B, 0xA5]);
        assert_eq!(lr_limits(), [0xAA, 100, 8, 0x16, 0, 0xD2, 0, 0, 1, 0xA5]);
        assert_eq!(ud_limits(), [0xAA, 101, 3, 0xE8, 2, 0xBC, 0, 0, 1, 0xA5]);
        assert_eq!(read_value(), [0xAA, 113, 0, 0, 0, 0, 0, 0, 1, 0xA5]);
    }

    #[test]
    fn raw_point_round_trip() {
        let pt = RawPoint { id: 17, lr: 1140, ud: 2600, speed: 105, interval: 40 };
        assert_eq!(pt.frame(), [0xAA, 17, 0x04, 0x74, 0x0A, 0x28, 0, 105, 40, 0xA5]);
        assert_eq!(RawPoint::from_frame(&pt.frame()), pt);
        for f in calibration(Sport::Tennis, Height::Mid, 10) {
            assert_eq!(RawPoint::from_frame(&f).frame(), f);
        }
        assert_eq!(RawPoint { id: 0, ..pt }.frame()[1], 1);
        assert_eq!(RawPoint { id: 0x66, ..pt }.frame()[1], 41);
    }

    fn sent(p: &Pattern) -> Vec<Frame> {
        p.start().into_iter().map(|s| s.1).collect()
    }

    #[test]
    fn patterns_match_app() {
        let fixed = [0xAA, 106, 1, 0, 0, 0, 0, 0, 107, 0xA5];
        for p in [Pattern::Fixed(2), Pattern::Smash, Pattern::Volley(1), Pattern::Primary, Pattern::Moon] {
            assert_eq!(sent(&p), vec![fixed, apply()]);
        }
        assert_eq!(sent(&Pattern::Random)[0], [0xAA, 106, 4, 0, 0, 0, 0, 0, 110, 0xA5]);
        assert_eq!(sent(&Pattern::Horizontal(1))[0], [0xAA, 106, 2, 0, 0, 0, 0, 0, 104, 0xA5]);
        assert_eq!(sent(&Pattern::Horizontal(3))[0], [0xAA, 106, 2, 16, 20, 0, 0, 0, 110, 0xA5]);
        assert_eq!(sent(&Pattern::Horizontal(5))[0], [0xAA, 106, 2, 15, 18, 21, 0, 0, 96, 0xA5]);
        assert_eq!(sent(&Pattern::Vertical(2))[0], [0xAA, 106, 3, 4, 11, 18, 25, 32, 77, 0xA5]);
        assert_eq!(sent(&Pattern::Vertical(3))[0], [0xAA, 106, 3, 4, 25, 0, 0, 0, 116, 0xA5]);
        // Every start ends with apply, and the whole sequence spans at least 270 ms.
        for p in [Pattern::Random, Pattern::Fixed(1), Pattern::Cross(1), Pattern::Program(vec![1])] {
            let s = p.start();
            assert_eq!(s.last().unwrap().1, apply());
            assert!(s.iter().map(|x| x.0).sum::<u32>() >= 270);
        }
    }

    #[test]
    fn cross_and_programs() {
        // $96 with tagC = 1: AA 6D 01 16 04 00 00 00 chk A5, chk = 0x16 ^ 0x6C ^ 0x04.
        let c = sent(&Pattern::Cross(1));
        assert_eq!(c[0], [0xAA, 0x6D, 1, 22, 4, 0, 0, 0, 22 ^ 0x6C ^ 4, 0xA5]);
        assert_eq!(c[1], [0xAA, 0x6D, 2, 0, 0, 0, 0, 0, 0x6F, 0xA5]);
        assert_eq!(c[5], [0xAA, 0x6D, 6, 0, 0, 0, 0, 0, 0x6B, 0xA5]);
        assert_eq!(c[6], [0xAA, 0x6A, 5, 0, 0, 0, 0, 0, 0x6F, 0xA5]);
        assert_eq!(sent(&Pattern::Cross(6))[0][3..5], [28, 1]);
        let pr = sent(&Pattern::Program(vec![1, 41, 99]));
        assert_eq!(pr[0][2..6], [1, 1, 41, 0]);
        assert_eq!(pr[6][2], 5);
        let cu = sent(&Pattern::Custom(vec![1, 6]));
        assert_eq!(cu[0][2..6], [1, 36, 41, 0]);
        assert_eq!(cu[6], [0xAA, 0x6A, 6, 0, 0, 0, 0, 0, 0x6F, 0xA5]);
        let many: Vec<u8> = (1..=30).collect();
        let pages = sequence_pages(&many);
        assert_eq!(pages[5][2..8], [6, 26, 27, 28, 0, 0]);
    }

    #[test]
    fn presets_match_app() {
        assert_eq!(Pattern::Fixed(3).preset(Sport::Tennis).aim, Some((2070, 1200)));
        assert_eq!(Pattern::Volley(2).preset(Sport::Tennis).aim, Some((210, 1000)));
        assert_eq!(Pattern::Smash.preset(Sport::Tennis).speed, Some(4));
        assert_eq!(Pattern::Moon.preset(Sport::Tennis).spin.unwrap().1, [0xAA, 0x62, 1, 0x1E, 0, 0, 0, 0, 0x7A, 0xA5]);
        assert_eq!(Pattern::Primary.preset(Sport::Tennis).spin.unwrap().1, [0xAA, 0x62, 0, 0, 0, 0, 0, 0, 0x63, 0xA5]);
        assert_eq!(stop_sequence().iter().map(|s| s.1[1]).collect::<Vec<_>>(), vec![0x6B, 0x67, 0x66]);
    }

    #[test]
    fn controls_match_app_formulas() {
        assert_eq!(speed(Sport::Tennis, 5), [0xAA, 99, 105, 0, 0, 0, 0, 0, 105 ^ 99, 0xA5]);
        assert_eq!(frequency(1), [0xAA, 97, 88, 0, 0, 0, 0, 0, 88 ^ 97, 0xA5]);
        assert_eq!(spin(0), [0xAA, 98, 0, 0, 0, 0, 0, 0, 98, 0xA5]);
        assert_eq!(spin(3), [0xAA, 98, 1, 15, 0, 0, 0, 0, 15 ^ 99, 0xA5]);
        assert_eq!(spin(-2), [0xAA, 98, 2, 10, 0, 0, 0, 0, 10 ^ 96, 0xA5]);
        assert_eq!(position(1140, 1500), [0xAA, 108, 0x04, 0x74, 0x05, 0xDC, 0, 0, 1, 0xA5]);
    }

    #[test]
    fn drill_sequence() {
        let d = DrillPoint::defaults();
        let f = drill(&[1, 28], &d, DrillPause::Short, Some(260830));
        assert_eq!(f[0], [0xAA, 0x24, 0x08, 0x52, 0x07, 0xD0, 50, 98, 40, 0xA5]);
        assert_eq!(f[1], [0xAA, 0x25, 0x01, 0x0E, 0x07, 0xD0, 50, 122, 60, 0xA5]);
        assert_eq!(f[2], frame(0x6D, [1, 0x24, 0x25, 0, 0, 0]));
        assert_eq!(f[7], [0xAA, 0x6D, 6, 0, 0, 0, 0, 0, 107, 0xA5]);
        assert_eq!(f[8], [0xAA, 106, 8, 0, 0, 0, 0, 0, 111, 0xA5]);
        assert_eq!(f[9], apply());
        assert_eq!(drill(&[1], &d, DrillPause::None, Some(241231))[7][2], 6);
        // Edited parameters are what gets sent.
        let mut e = d;
        e[0] = DrillPoint { lr: 2370, ud: 4500, speed: 140, interval: 25, spin: 80 };
        assert_eq!(drill(&[1], &e, DrillPause::None, None)[0], [0xAA, 0x24, 0x09, 0x42, 0x11, 0x94, 80, 140, 25, 0xA5]);
    }

    #[test]
    fn drill_live_edit_and_ai() {
        let mut p = DrillPoint::defaults()[3];
        p.spin = 65;
        let new_fw = drill_edit(2, p, true, DrillPause::Long, Some(260830));
        assert_eq!(new_fw.iter().map(|s| s.1).collect::<Vec<_>>(), vec![drill_slot(2, p, true, DrillPause::Long), apply()]);
        assert_eq!(new_fw[0].1[1], 0x25);
        assert_eq!(new_fw[0].1[8], 80);
        let old_fw = drill_edit(1, p, false, DrillPause::None, Some(240101));
        assert_eq!(old_fw[2].1, [0xAA, 0x62, 1, 15, 0, 0, 0, 0, 0, 0xA5]);
        assert_eq!(DrillPoint { lr: 0, ud: 9999, speed: 0, interval: 99, spin: 0 }.clamped(),
            DrillPoint { lr: 90, ud: 4500, speed: 50, interval: 65, spin: 20 });

        let mut params = DrillPoint::defaults();
        params[0].interval = 60;
        // rand always 0: first ball = 17 (left zone), the rest = point 1.
        let pts = ai_drill(AiOptions { left: true, level: 0, balls: 4 }, &mut params, |_| 0);
        assert_eq!(pts, vec![17, 1, 1, 1]);
        assert_eq!(params[16].interval, 45);
        assert_eq!(params[0].interval, 55);
        assert_eq!(params[1].interval, 40);
        let right = ai_drill(AiOptions { left: false, level: 2, balls: 1 }, &mut params, |n| n - 1);
        assert_eq!(right, vec![28, 28]);
        assert_eq!(params[27].interval, 30);
    }

    #[test]
    fn pattern_defaults() {
        assert_eq!(Pattern::Fixed(2).default_key().as_deref(), Some("fix2"));
        assert_eq!(Pattern::Volley(3).default_key().as_deref(), Some("hit3"));
        assert_eq!(Pattern::Smash.default_key().as_deref(), Some("high"));
        assert_eq!(Pattern::Random.default_key().as_deref(), Some("whole"));
        assert_eq!(Pattern::Horizontal(5).default_key().as_deref(), Some("lr5"));
        assert_eq!(Pattern::Vertical(1).default_key().as_deref(), Some("ud1"));
        assert_eq!(Pattern::Cross(4).default_key().as_deref(), Some("cross4"));
        assert_eq!(Pattern::Primary.default_key().as_deref(), Some("place"));
        assert_eq!(Pattern::Program(vec![1]).default_key(), None);

        let d = PatternDefault { lr: 2070, ud: 1200, freq: 3, speed: 8, spin: -2, height_level: 12 };
        let s = d.apply_steps(Sport::Tennis);
        assert_eq!(s[0].1, position(2070, 1200));
        assert_eq!(s[1].1, frequency(3));
        assert_eq!(s[2].1, speed(Sport::Tennis, 8));
        assert_eq!(s[3].1, [0xAA, 98, 2, 10, 0, 0, 0, 0, 10 ^ 96, 0xA5]);
        assert_eq!(s.len(), 4 + 41);
        assert_eq!(s[4].1, calibration(Sport::Tennis, Height::Mid, 12)[0]);
        assert_eq!(PatternDefault { height_level: 0, ..d }.apply_steps(Sport::Tennis).len(), 4);

        let moved = PatternDefault { lr: 210, ..d };
        assert!(d.differs(&Pattern::Fixed(1), &moved));
        assert!(!d.differs(&Pattern::Horizontal(1), &moved));
        let higher = PatternDefault { ud: 1500, ..d };
        assert!(!d.differs(&Pattern::Vertical(1), &higher));
        assert!(d.differs(&Pattern::Vertical(1), &PatternDefault { speed: 9, ..d }));
    }

    #[test]
    fn saved_programs_and_sets() {
        let prog = ProgramSettings { freq: 2, speed: 9, spin: 3, height: Height::Low, height_level: 13 };
        let st = prog.load_steps(Sport::Tennis, 10);
        assert_eq!(st[0].1, apply());
        assert_eq!(st[1].1, frequency(2));
        assert_eq!(st[2].1, speed(Sport::Tennis, 9));
        assert_eq!(st[3].1, spin(3));
        // Always the mid table, at the saved level.
        assert_eq!(st[4].1, calibration(Sport::Tennis, Height::Mid, 13)[0]);
        assert_eq!(st.len(), 4 + 41);
        assert_eq!(ProgramSettings { height_level: 0, ..prog }.load_steps(Sport::Tennis, 7)[4].1, calibration(Sport::Tennis, Height::Mid, 7)[0]);
        assert_eq!(st.iter().map(|x| x.0).take(5).collect::<Vec<_>>(), vec![5, 25, 50, 40, 40]);

        let mut pts = CustomPoint::defaults(Sport::Tennis);
        pts[5].lr = 300;
        let cs = custom_set_load_steps(&pts);
        assert_eq!(cs.len(), 7);
        assert_eq!(cs[0].1, apply());
        assert_eq!(cs[1].1[1], 36);
        assert_eq!(cs[6].1, custom_point(6, pts[5]));
    }

    #[test]
    fn calibration_upload() {
        let f = calibration(Sport::Tennis, Height::Mid, 10);
        assert_eq!(f.len(), 41);
        assert_eq!(f[0], [0xAA, 1, 0x08, 0x16, 0x03, 0xE8, 0, 125, 33, 0xA5]);
        // Level 11 raises grid points by 100 and leaves the custom points alone.
        assert_eq!(calibration(Sport::Tennis, Height::Mid, 11)[0][4..6], [0x04, 0x4C]);
        assert_eq!(calibration(Sport::Tennis, Height::Mid, 11)[40], f[40]);
        // Low/high: 35 rows with XOR checksums, as in sendBaseData(0/2).
        let low = calibration(Sport::Tennis, Height::Low, 10);
        assert_eq!(low.len(), 35);
        assert_eq!(low[0], frame(1, [0x08, 0x16, 0x03, 0x20, 0, 0]));
        assert_eq!(calibration(Sport::Tennis, Height::High, 10)[34][4..6], [0x06, 0xA4]);
        assert_eq!(custom_point(1, CustomPoint::defaults(Sport::Tennis)[0]), f[35]);
        assert_eq!(custom_point(6, CustomPoint { lr: 5000, ud: 9000, speed: 1, interval: 99 })[2..9], [0x08, 0x16, 0x10, 0xCC, 0, 40, 65]);
    }

    #[test]
    fn names_and_notifications() {
        assert_eq!(date_code("PT7250301"), Some(250301));
        assert_eq!(date_code("PT7"), None);
        assert_eq!(date_code("PT8260830001"), Some(260830));
        assert!(!uses_alt_services("PT8260830001"));
        assert!(uses_alt_services("PT1xyz"));
        assert!(!uses_alt_services("PT7xyz"));
        assert_eq!(parse_notification(&[0xAA, 3, 60]), Notification::Battery(60));
        assert_eq!(parse_notification(&[0xAA, 0x5E, 3]), Notification::Fault(Fault::OutOfBalls));
        assert_eq!(battery_bars(44), 0);
        assert_eq!(battery_bars(68), 4);
    }

    // Expected values from MainActivityPad2 (the app's "Paddle Tennis" screen).
    #[test]
    fn padel_matches_app() {
        assert_eq!(speed(Sport::Padel, 1), [0xAA, 99, 66, 0, 0, 0, 0, 0, 66 ^ 99, 0xA5]);
        assert_eq!(speed(Sport::Padel, 7)[2], 95);
        assert_eq!(speed(Sport::Padel, 13)[2], 145);

        let mid = calibration(Sport::Padel, Height::Mid, 10);
        assert_eq!(mid.len(), 41);
        assert_eq!(mid[0], [0xAA, 1, 0x08, 0x16, 0x06, 0x40, 0, 125, 33, 0xA5]);
        // Level 30 raises the grid by 2000 but leaves the custom points alone.
        let top = calibration(Sport::Padel, Height::Mid, 30);
        assert_eq!(RawPoint::from_frame(&top[34]).ud, 2100 + 2000);
        assert_eq!(top[40], mid[40]);
        assert_eq!(calibration(Sport::Tennis, Height::Mid, 30), calibration(Sport::Tennis, Height::Mid, 16));
        // Low and high tables are shared with tennis.
        assert_eq!(calibration(Sport::Padel, Height::High, 10), calibration(Sport::Tennis, Height::High, 10));

        let c = CustomPoint::defaults(Sport::Padel);
        assert_eq!(c[0], CustomPoint { lr: 1860, ud: 2200, speed: 55, interval: 40 });
        assert_eq!(c[5], CustomPoint { lr: 520, ud: 4200, speed: 88, interval: 40 });
        assert_eq!(custom_point(1, c[0]), mid[35]);

        let pre = |p: Pattern| p.preset(Sport::Padel);
        assert_eq!(pre(Pattern::Fixed(2)).aim, Some((210, 2000)));
        assert_eq!(pre(Pattern::Volley(3)).aim, Some((2070, 1800)));
        assert_eq!((pre(Pattern::Smash).aim, pre(Pattern::Smash).speed), (Some((1140, 3600)), Some(4)));
        assert_eq!(pre(Pattern::Primary).aim, Some((1140, 2300)));
        assert_eq!(pre(Pattern::Moon).aim, Some((1140, 3000)));
        assert_eq!((pre(Pattern::Bandeja).aim, pre(Pattern::Bandeja).speed), (Some((1140, 4200)), Some(3)));
        assert_eq!((pre(Pattern::Bajada).aim, pre(Pattern::Bajada).speed), (Some((1140, 3800)), Some(4)));
        assert_eq!((pre(Pattern::ServeReturn).aim, pre(Pattern::ServeReturn).speed), (Some((1140, 2000)), Some(2)));
        assert_eq!(pre(Pattern::Horizontal(2)).aim, Some((1320, 2000)));
        assert_eq!(Pattern::Vertical(1).preset(Sport::Tennis).aim, Some((1320, 900)));

        for p in [Pattern::Bandeja, Pattern::Bajada, Pattern::ServeReturn] {
            assert_eq!(p.start()[0], (200, [0xAA, 106, 1, 0, 0, 0, 0, 0, 107, 0xA5]));
        }
        assert_eq!(Pattern::ServeReturn.default_key().as_deref(), Some("sreturn"));
        assert_eq!(Pattern::Bandeja.default_key().as_deref(), Some("bandeja"));
        assert_eq!(apply_padel_reselect(), [0xAA, 102, 0, 0, 0, 0, 0, 0, 102, 0xA5]);
    }
}
