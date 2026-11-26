//! Drill Builder: turns a numbered, plain-text description of balls into a drill.
//!
//! Each ball goes on its own line and starts with its number, 1, 2, 3, … in order:
//!
//! ```text
//! 1. topspin deep left
//! 2. slice short centre right
//! 3. lob point 25
//! ```
//!
//! A line names one shot (what the machine feeds) and where the ball first bounces: a
//! depth and a side, or a grid point. A missing depth means deep, a missing side the
//! centre. English and Dutch words both work; small words like "to the" are skipped.
//!
//! In doubles the lines describe the right-hand player's balls, and each is followed by
//! its mirror image for the left-hand player (right 1, left 1, right 2, left 2, …). There
//! the side words mean a place within that player's own half (see [`own_half`]); a grid
//! point is still a fixed spot on the court.

use crate::drills::{self, Ball, Drill, Level, Spot};
use crate::protocol::{DrillPause, MAX_DRILL_SHOTS};

/// What the machine feeds. Each maps to a height, a speed change from the grid point's
/// default, an interval and a spin, in the same terms as the drill library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shot {
    Flat,
    Drive,
    Topspin,
    Slice,
    Lob,
    HighBall,
    DropShot,
    Volley,
}

impl Shot {
    /// Height (`ud`), speed change, seconds × 10 until the next ball, spin −6..=6.
    fn shape(self) -> (u16, i8, u8, i8) {
        match self {
            Shot::Flat => (2000, 0, 40, 0),
            Shot::Drive => (2000, 8, 35, 0),
            Shot::Topspin => (2200, 0, 40, 3),
            Shot::Slice => (1600, 0, 45, -4),
            Shot::Lob => (4000, -6, 55, 0),
            Shot::HighBall => (2800, -6, 45, 2),
            Shot::DropShot => (1500, -20, 35, -3),
            Shot::Volley => (1500, 10, 35, 0),
        }
    }

    /// English and Dutch name.
    pub fn name(self) -> (&'static str, &'static str) {
        match self {
            Shot::Flat => ("Flat", "Vlak"),
            Shot::Drive => ("Drive", "Drive"),
            Shot::Topspin => ("Topspin", "Topspin"),
            Shot::Slice => ("Slice", "Slice"),
            Shot::Lob => ("Lob", "Lob"),
            Shot::HighBall => ("High ball", "Hoge bal"),
            Shot::DropShot => ("Drop shot", "Dropshot"),
            Shot::Volley => ("Volley ball", "Volleybal"),
        }
    }
}

/// Offensive gives the player more time and a slower ball to attack; defensive less time
/// and a faster ball to defend against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Offensive,
    Defensive,
}

/// Speed change and interval change for a mode, and how far from the net players start.
fn mode_shape(mode: Option<Mode>) -> (i8, i8, f32) {
    match mode {
        Some(Mode::Offensive) => (-4, 5, 11.5),
        Some(Mode::Defensive) => (6, -5, 13.5),
        None => (0, 0, 12.5),
    }
}

/// One ball as read from the text, before mirroring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Line {
    pub shot: Shot,
    /// Grid point 1..=28: rows of seven from short (1–7) to deep (22–28), column 1 far left.
    pub point: u8,
}

/// Why the text could not be read. `ball` is the number on the line, 0 for the whole text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub ball: usize,
    pub en: String,
    pub nl: String,
}

fn problem(ball: usize, en: impl Into<String>, nl: impl Into<String>) -> Problem {
    Problem { ball, en: en.into(), nl: nl.into() }
}

/// Words and phrases, longest first so "drop shot" wins over "drop".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Word {
    Shot(Shot),
    /// Row 0 (short) ..= 3 (deep).
    Depth(u8),
    /// Column 1 (far left) ..= 7 (far right).
    Side(u8),
    Point,
    Skip,
}

const PHRASES: &[(&str, Word)] = &[
    ("center left", Word::Side(3)),
    ("centre left", Word::Side(3)),
    ("midden links", Word::Side(3)),
    ("center right", Word::Side(5)),
    ("centre right", Word::Side(5)),
    ("midden rechts", Word::Side(5)),
    ("wide left", Word::Side(1)),
    ("far left", Word::Side(1)),
    ("breed links", Word::Side(1)),
    ("ver links", Word::Side(1)),
    ("wide right", Word::Side(7)),
    ("far right", Word::Side(7)),
    ("breed rechts", Word::Side(7)),
    ("ver rechts", Word::Side(7)),
    ("service line", Word::Depth(1)),
    ("mid court", Word::Depth(2)),
    ("drop shot", Word::Shot(Shot::DropShot)),
    ("high ball", Word::Shot(Shot::HighBall)),
    ("hoge bal", Word::Shot(Shot::HighBall)),
];

const WORDS: &[(&str, Word)] = &[
    ("flat", Word::Shot(Shot::Flat)),
    ("vlak", Word::Shot(Shot::Flat)),
    ("drive", Word::Shot(Shot::Drive)),
    ("topspin", Word::Shot(Shot::Topspin)),
    ("slice", Word::Shot(Shot::Slice)),
    ("backspin", Word::Shot(Shot::Slice)),
    ("lob", Word::Shot(Shot::Lob)),
    ("moonball", Word::Shot(Shot::HighBall)),
    ("dropshot", Word::Shot(Shot::DropShot)),
    ("stopbal", Word::Shot(Shot::DropShot)),
    ("volley", Word::Shot(Shot::Volley)),
    ("volleybal", Word::Shot(Shot::Volley)),
    ("short", Word::Depth(0)),
    ("kort", Word::Depth(0)),
    ("service", Word::Depth(1)),
    ("servicelijn", Word::Depth(1)),
    ("midcourt", Word::Depth(2)),
    ("middenveld", Word::Depth(2)),
    ("deep", Word::Depth(3)),
    ("diep", Word::Depth(3)),
    ("baseline", Word::Depth(3)),
    ("achterlijn", Word::Depth(3)),
    ("left", Word::Side(2)),
    ("links", Word::Side(2)),
    ("centre", Word::Side(4)),
    ("center", Word::Side(4)),
    ("middle", Word::Side(4)),
    ("midden", Word::Side(4)),
    ("right", Word::Side(6)),
    ("rechts", Word::Side(6)),
    ("point", Word::Point),
    ("punt", Word::Point),
    ("to", Word::Skip),
    ("the", Word::Skip),
    ("a", Word::Skip),
    ("ball", Word::Skip),
    ("bounce", Word::Skip),
    ("bounces", Word::Skip),
    ("first", Word::Skip),
    ("at", Word::Skip),
    ("on", Word::Skip),
    ("in", Word::Skip),
    ("of", Word::Skip),
    ("and", Word::Skip),
    ("with", Word::Skip),
    ("shot", Word::Skip),
    ("naar", Word::Skip),
    ("de", Word::Skip),
    ("het", Word::Skip),
    ("een", Word::Skip),
    ("bal", Word::Skip),
    ("stuit", Word::Skip),
    ("stuitert", Word::Skip),
    ("eerst", Word::Skip),
    ("op", Word::Skip),
    ("met", Word::Skip),
    ("en", Word::Skip),
    ("van", Word::Skip),
    ("slag", Word::Skip),
];

/// Most balls the text may describe: the machine plays 15, and in doubles each line
/// becomes two balls.
pub fn max_lines(doubles: bool) -> usize {
    if doubles { MAX_DRILL_SHOTS / 2 } else { MAX_DRILL_SHOTS }
}

/// Reads the numbered lines. Blank lines are skipped.
pub fn parse(text: &str, doubles: bool) -> Result<Vec<Line>, Vec<Problem>> {
    let mut lines = Vec::new();
    let mut problems = Vec::new();
    let mut count = 0;
    for raw in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        count += 1;
        let expected = count;
        let digits: String = raw.chars().take_while(char::is_ascii_digit).collect();
        let Ok(number) = digits.parse::<usize>() else {
            problems.push(problem(expected, format!("Start this line with its number, {expected}: “{raw}”."),
                format!("Begin deze regel met het nummer {expected}: “{raw}”.")));
            continue;
        };
        if number != expected {
            problems.push(problem(number, format!("Expected ball {expected} here, found {number}. Number the balls 1, 2, 3, … in order."),
                format!("Hier werd bal {expected} verwacht, maar er staat {number}. Nummer de ballen 1, 2, 3, … op volgorde.")));
            continue;
        }
        match parse_line(&raw[digits.len()..], number, doubles) {
            Ok(line) => lines.push(line),
            Err(p) => problems.push(p),
        }
    }
    if lines.is_empty() && problems.is_empty() {
        problems.push(problem(0, "Describe at least one ball, for example “1. topspin deep left”.",
            "Beschrijf minstens één bal, bijvoorbeeld “1. topspin diep links”."));
    }
    let max = max_lines(doubles);
    if count > max {
        problems.push(if doubles {
            problem(0, format!("A doubles drill can describe up to {max} balls; each is mirrored for the left player and the machine plays {MAX_DRILL_SHOTS} at most."),
                format!("Een dubbelspeloefening kan maximaal {max} ballen beschrijven; elke bal wordt gespiegeld voor de linkerspeler en de machine speelt er hoogstens {MAX_DRILL_SHOTS}."))
        } else {
            problem(0, format!("The machine plays up to {max} balls in one drill; this text describes {count}."),
                format!("De machine speelt maximaal {max} ballen per oefening; deze tekst beschrijft er {count}."))
        });
    }
    if problems.is_empty() { Ok(lines) } else { Err(problems) }
}

fn parse_line(rest: &str, n: usize, doubles: bool) -> Result<Line, Problem> {
    let cleaned: String = rest
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_lowercase().next().unwrap_or(c) } else { ' ' })
        .collect();
    let tokens: Vec<&str> = cleaned.split_whitespace().collect();
    let (mut shot, mut depth, mut side, mut point) = (None, None, None, None);
    let mut i = 0;
    while i < tokens.len() {
        let pair = tokens.get(i + 1).map(|next| format!("{} {next}", tokens[i]));
        let (word, used) = match pair.as_deref().and_then(|p| PHRASES.iter().find(|(w, _)| *w == p)) {
            Some(&(_, w)) => (w, 2),
            None => match WORDS.iter().find(|(w, _)| *w == tokens[i]) {
                Some(&(_, w)) => (w, 1),
                None => {
                    return Err(problem(n, format!("Ball {n}: “{}” is not a shot or a place I know. {}", tokens[i], VOCAB_EN),
                        format!("Bal {n}: “{}” is geen slag of plek die ik ken. {}", tokens[i], VOCAB_NL)));
                }
            },
        };
        let twice = || problem(n, format!("Ball {n} names more than one {}.", what(word).0), format!("Bal {n} noemt meer dan één {}.", what(word).1));
        match word {
            Word::Shot(s) => if shot.replace(s).is_some() { return Err(twice()) },
            Word::Depth(d) => if depth.replace(d).is_some() { return Err(twice()) },
            Word::Side(c) => if side.replace(c).is_some() { return Err(twice()) },
            Word::Point => {
                let p = tokens.get(i + 1).and_then(|t| t.parse::<u8>().ok()).filter(|p| (1..=28).contains(p));
                let Some(p) = p else {
                    return Err(problem(n, format!("Ball {n}: “point” needs a number from 1 to 28 after it."),
                        format!("Bal {n}: na “punt” moet een getal van 1 tot 28 staan.")));
                };
                if point.replace(p).is_some() { return Err(twice()) }
                i += 1;
            }
            Word::Skip => {}
        }
        i += used;
    }
    let Some(shot) = shot else {
        return Err(problem(n, format!("Ball {n} needs a shot: {SHOTS_EN}."), format!("Bal {n} heeft een slag nodig: {SHOTS_NL}.")));
    };
    let point = match (point, depth, side) {
        (Some(_), Some(_), _) | (Some(_), _, Some(_)) => {
            return Err(problem(n, format!("Ball {n}: give either a point number or a depth and side, not both."),
                format!("Bal {n}: geef een puntnummer óf een diepte en kant, niet allebei.")));
        }
        (Some(p), ..) => p,
        (None, d, c) => {
            let column = c.unwrap_or(4);
            d.unwrap_or(3) * 7 + if doubles { own_half(column) } else { column }
        }
    };
    Ok(Line { shot, point })
}

fn what(word: Word) -> (&'static str, &'static str) {
    match word {
        Word::Shot(_) => ("shot", "slag"),
        Word::Depth(_) => ("depth", "diepte"),
        Word::Side(_) => ("side", "kant"),
        _ => ("point", "punt"),
    }
}

const SHOTS_EN: &str = "flat, drive, topspin, slice, lob, high ball, drop shot or volley";
const SHOTS_NL: &str = "vlak, drive, topspin, slice, lob, hoge bal, dropshot of volley";
const VOCAB_EN: &str = "Shots: flat, drive, topspin, slice, lob, high ball, drop shot, volley. Depth: short, service line, midcourt, deep. Side: wide left, left, centre left, centre, centre right, right, wide right. Or “point 1” to “point 28”.";
const VOCAB_NL: &str = "Slagen: vlak, drive, topspin, slice, lob, hoge bal, dropshot, volley. Diepte: kort, servicelijn, middenveld, diep. Kant: breed links, links, midden links, midden, midden rechts, rechts, breed rechts. Of “punt 1” tot “punt 28”.";

/// In doubles, the court column (1..=7) for a side word (column 1..=7 as in singles) said
/// of the right-hand player: the seven words squeeze into that player's half, from the
/// centre line (column 4, "wide left") out to the right sideline (column 7, "right" and
/// "wide right"). Mirroring then gives the left-hand player the same place in theirs.
pub fn own_half(column: u8) -> u8 {
    4 + column / 2
}

/// Grid point mirrored left to right.
pub fn mirror(point: u8) -> u8 {
    let i = point - 1;
    (i / 7) * 7 + (6 - i % 7) + 1
}

/// Where a grid point is, in words, for the ball list.
pub fn describe_point(point: u8) -> ((&'static str, &'static str), (&'static str, &'static str)) {
    const DEPTHS: [(&str, &str); 4] = [("short", "kort"), ("service line", "servicelijn"), ("midcourt", "middenveld"), ("deep", "diep")];
    const SIDES: [(&str, &str); 7] = [("wide left", "breed links"), ("left", "links"), ("centre left", "midden links"), ("centre", "midden"),
        ("centre right", "midden rechts"), ("right", "rechts"), ("wide right", "breed rechts")];
    let i = (point.clamp(1, 28) - 1) as usize;
    (DEPTHS[i / 7], SIDES[i % 7])
}

/// Where a ball bounces, in the words of the player who takes it: in singles as on the
/// whole court; in doubles within that player's half, mirrored for the left-hand player.
pub fn describe_planned(p: &Planned, doubles: bool) -> ((&'static str, &'static str), (&'static str, &'static str)) {
    let (depth, side) = describe_point(p.point);
    if !doubles {
        return (depth, side);
    }
    const RIGHT: [(&str, &str); 4] = [("wide left", "breed links"), ("left", "links"), ("centre", "midden"), ("right", "rechts")];
    const LEFT: [(&str, &str); 4] = [("wide right", "breed rechts"), ("right", "rechts"), ("centre", "midden"), ("left", "links")];
    let column = (p.point - 1) % 7 + 1;
    let side = if p.player == 0 {
        column.checked_sub(4).map_or(side, |i| RIGHT[i as usize])
    } else {
        4u8.checked_sub(column).map_or(side, |i| LEFT[i as usize])
    };
    (depth, side)
}

/// What a saved drill keeps: the text and choices it was generated from, so loading it
/// fills the panel again and regenerates the same drill.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saved {
    pub name: String,
    pub text: String,
    pub mode: Option<Mode>,
    pub doubles: bool,
}

/// The stored fields after the name: `mode<TAB>players<TAB>text`, with backslashes,
/// tabs and line breaks in the text escaped so each drill stays on one line.
pub fn encode_saved(d: &Saved) -> String {
    let mode = match d.mode {
        Some(Mode::Offensive) => "offensive",
        Some(Mode::Defensive) => "defensive",
        None => "",
    };
    let mut text = String::with_capacity(d.text.len());
    for c in d.text.chars() {
        match c {
            '\\' => text.push_str("\\\\"),
            '\n' => text.push_str("\\n"),
            '\t' => text.push_str("\\t"),
            '\r' => {}
            c => text.push(c),
        }
    }
    format!("{mode}\t{}\t{text}", if d.doubles { "doubles" } else { "singles" })
}

/// Reads what [`encode_saved`] wrote; `None` for a damaged record.
pub fn decode_saved(name: &str, fields: &str) -> Option<Saved> {
    let mut parts = fields.splitn(3, '\t');
    let mode = match parts.next()? {
        "offensive" => Some(Mode::Offensive),
        "defensive" => Some(Mode::Defensive),
        "" => None,
        _ => return None,
    };
    let doubles = match parts.next()? {
        "doubles" => true,
        "singles" => false,
        _ => return None,
    };
    let mut text = String::new();
    let mut chars = parts.next()?.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next()? {
                'n' => text.push('\n'),
                't' => text.push('\t'),
                '\\' => text.push('\\'),
                _ => return None,
            }
        } else {
            text.push(c);
        }
    }
    Some(Saved { name: name.to_string(), text, mode, doubles })
}

/// Marks an exported file as ours, so importing some other JSON file fails clearly.
const FILE_FORMAT: &str = "pusun-web drills";
const FILE_VERSION: u32 = 1;
/// Largest file import reads, in bytes, and most drills it takes from one file.
pub const MAX_IMPORT_BYTES: usize = 1_000_000;
pub const MAX_IMPORT_DRILLS: usize = 500;
/// Longest drill text and name kept from a file, in characters.
pub const MAX_TEXT: usize = 2000;
const MAX_NAME: usize = 80;

#[derive(serde::Serialize, serde::Deserialize)]
struct DrillFile {
    format: String,
    version: u32,
    drills: Vec<FileDrill>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct FileDrill {
    name: String,
    /// "singles" or "doubles".
    players: String,
    /// "offensive", "defensive" or absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    style: Option<String>,
    text: String,
}

/// Saved drills as a JSON file for download.
pub fn export_file(drills: &[Saved]) -> String {
    let file = DrillFile {
        format: FILE_FORMAT.into(),
        version: FILE_VERSION,
        drills: drills
            .iter()
            .map(|d| FileDrill {
                name: d.name.clone(),
                players: if d.doubles { "doubles" } else { "singles" }.into(),
                style: d.mode.map(|m| if m == Mode::Offensive { "offensive" } else { "defensive" }.into()),
                text: d.text.clone(),
            })
            .collect(),
    };
    serde_json::to_string_pretty(&file).unwrap_or_default()
}

/// What an import did, for the message under the list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub added: usize,
    /// Added under a new name because a different drill already had theirs.
    pub renamed: usize,
    /// Already saved with the same name and content.
    pub unchanged: usize,
    /// Damaged entries left out.
    pub skipped: usize,
}

/// Keeps a name usable as a stored field: no tabs or line breaks, trimmed and capped.
fn clean_name(name: &str) -> String {
    let name: String = name.chars().map(|c| if c.is_control() { ' ' } else { c }).take(MAX_NAME).collect();
    name.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Keeps text plain, as the text box does: line breaks and tabs, no other control characters.
pub fn clean_text(text: &str) -> String {
    text.chars().filter(|c| !c.is_control() || *c == '\n' || *c == '\t').take(MAX_TEXT).collect()
}

/// Reads an exported file and merges it into `existing`. A drill whose name is taken by a
/// different drill is added as "Name (2)", "Name (3)", …; an identical one is left alone.
/// The file is untrusted: entries are checked and cleaned, and damaged ones skipped.
pub fn import_file(json: &str, existing: &[Saved]) -> Result<(Vec<Saved>, ImportReport), Problem> {
    let unreadable = || problem(0, "This is not a drill file exported from this app.", "Dit is geen oefeningenbestand dat met deze app is geëxporteerd.");
    if json.len() > MAX_IMPORT_BYTES {
        return Err(problem(0, "The file is too large to be a drill file (over 1 MB).", "Het bestand is te groot voor een oefeningenbestand (meer dan 1 MB)."));
    }
    let file: DrillFile = serde_json::from_str(json).map_err(|_| unreadable())?;
    if file.format != FILE_FORMAT {
        return Err(unreadable());
    }
    if file.version > FILE_VERSION {
        return Err(problem(0, "This drill file comes from a newer version of the app. Reload the page and try again.",
            "Dit oefeningenbestand komt van een nieuwere versie van de app. Laad de pagina opnieuw en probeer het nog eens."));
    }
    let mut all = existing.to_vec();
    let mut report = ImportReport::default();
    for (i, d) in file.drills.into_iter().enumerate() {
        if i >= MAX_IMPORT_DRILLS {
            report.skipped += 1;
            continue;
        }
        let doubles = match d.players.as_str() {
            "doubles" => true,
            "singles" => false,
            _ => { report.skipped += 1; continue; }
        };
        let mode = match d.style.as_deref() {
            None | Some("") => None,
            Some("offensive") => Some(Mode::Offensive),
            Some("defensive") => Some(Mode::Defensive),
            Some(_) => { report.skipped += 1; continue; }
        };
        let name = clean_name(&d.name);
        if name.is_empty() {
            report.skipped += 1;
            continue;
        }
        let drill = Saved { name: name.clone(), text: clean_text(&d.text.replace('\r', "")), mode, doubles };
        let same_content = |x: &Saved| x.text == drill.text && x.mode == drill.mode && x.doubles == drill.doubles;
        match all.iter().find(|x| x.name == name) {
            Some(x) if same_content(x) => report.unchanged += 1,
            Some(_) => {
                let mut n = 2;
                let new_name = loop {
                    let candidate = format!("{name} ({n})");
                    match all.iter().find(|x| x.name == candidate) {
                        None => break Some(candidate),
                        Some(x) if same_content(x) => break None,
                        Some(_) => n += 1,
                    }
                };
                match new_name {
                    Some(new_name) => {
                        all.push(Saved { name: new_name, ..drill });
                        report.renamed += 1;
                    }
                    None => report.unchanged += 1,
                }
            }
            None => {
                all.push(drill);
                report.added += 1;
            }
        }
    }
    Ok((all, report))
}

/// One ball of a generated drill, for the list under the court.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Planned {
    pub shot: Shot,
    pub point: u8,
    /// 0 for the right-hand player (or the only player), 1 for the left-hand player.
    pub player: usize,
}

/// A drill made from text; [`Generated::drill`] lends it out as a [`Drill`].
#[derive(Clone, Debug, PartialEq)]
pub struct Generated {
    pub doubles: bool,
    pub mode: Option<Mode>,
    pub planned: Vec<Planned>,
    balls: Vec<Ball>,
    players: Vec<Spot>,
    receivers: Vec<usize>,
}

impl Generated {
    pub fn drill(&self) -> Drill<'_> {
        Drill {
            key: "builder",
            name: ("Your drill", "Jouw oefening"),
            description: ("", ""),
            level: Level::Intermediate,
            doubles: self.doubles,
            pause: DrillPause::None,
            balls: &self.balls,
            machine: drills::MACHINE,
            players: &self.players,
            receivers: &self.receivers,
            strokes: &[],
            hit_depths: &[],
        }
    }
}

/// Builds the drill: mirrors for doubles, applies the mode, and checks that no grid point
/// is asked for two different shots (the machine keeps one setting per point).
pub fn generate(lines: &[Line], doubles: bool, mode: Option<Mode>) -> Result<Generated, Vec<Problem>> {
    let planned: Vec<Planned> = lines
        .iter()
        .flat_map(|l| {
            let right = Planned { shot: l.shot, point: l.point, player: 0 };
            let left = doubles.then(|| Planned { shot: l.shot, point: mirror(l.point), player: 1 });
            std::iter::once(right).chain(left)
        })
        .collect();
    let mut problems = Vec::new();
    for (j, a) in planned.iter().enumerate() {
        if let Some(b) = planned[..j].iter().find(|b| b.point == a.point && b.shot != a.shot) {
            let ((d_en, d_nl), (s_en, s_nl)) = describe_point(a.point);
            let (first, second) = (b.shot.name(), a.shot.name());
            let p = problem(0,
                format!("{} and {} both bounce {d_en} {s_en} (point {}). The machine keeps one setting per point, so use the same shot there or move one ball.", first.0, second.0, a.point),
                format!("{} en {} stuiten allebei {d_nl} {s_nl} (punt {}). De machine bewaart één instelling per punt, dus gebruik daar dezelfde slag of verplaats een bal.", first.1, second.1, a.point));
            if !problems.contains(&p) {
                problems.push(p);
            }
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    let (speed_change, interval_change, y) = mode_shape(mode);
    let balls = planned
        .iter()
        .map(|p| {
            let (height, speed, interval, spin) = p.shot.shape();
            drills::shaped(p.point, height, speed + speed_change, (interval as i8 + interval_change) as u8, spin)
        })
        .collect();
    let (players, receivers) = if doubles {
        (vec![drills::at(2.7, y), drills::at(-2.7, y)], planned.iter().map(|p| p.player).collect())
    } else {
        (vec![drills::at(0.0, y)], Vec::new())
    };
    Ok(Generated { doubles, mode, planned, balls, players, receivers })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drills::NORMAL;

    #[test]
    fn reads_words_points_and_defaults() {
        let lines = parse("1. topspin deep left\n\n2) Slice, short, centre-right\n3: lob to point 25\n4 drop shot\n5. volley wide right service line", false).unwrap();
        assert_eq!(lines, vec![
            Line { shot: Shot::Topspin, point: 23 },
            Line { shot: Shot::Slice, point: 5 },
            Line { shot: Shot::Lob, point: 25 },
            Line { shot: Shot::DropShot, point: 25 },
            Line { shot: Shot::Volley, point: 14 },
        ]);
        let nl = parse("1. hoge bal diep breed links\n2. stopbal kort midden", false).unwrap();
        assert_eq!(nl, vec![Line { shot: Shot::HighBall, point: 22 }, Line { shot: Shot::DropShot, point: 4 }]);
    }

    #[test]
    fn reports_problems_by_ball() {
        let err = parse("1. topspin deep left\n3. lob", false).unwrap_err();
        assert_eq!(err[0].ball, 3);
        let err = parse("1. topspin sideways", false).unwrap_err();
        assert!(err[0].en.contains("“sideways”"));
        assert!(parse("1. deep left", false).unwrap_err()[0].en.contains("needs a shot"));
        assert!(parse("1. topspin lob", false).unwrap_err()[0].en.contains("more than one shot"));
        assert!(parse("1. lob point 29", false).is_err());
        assert!(parse("1. lob point 3 deep", false).is_err());
        assert!(parse("topspin deep", false).is_err());
        assert_eq!(parse("   \n", false).unwrap_err()[0].ball, 0);
    }

    #[test]
    fn limits_follow_the_machine() {
        let text = |n: usize| (1..=n).map(|i| format!("{i}. flat deep")).collect::<Vec<_>>().join("\n");
        assert!(parse(&text(15), false).is_ok());
        assert!(parse(&text(16), false).is_err());
        assert!(parse(&text(7), true).is_ok());
        assert!(parse(&text(8), true).is_err());
    }

    #[test]
    fn mirrors_columns() {
        assert_eq!(mirror(22), 28);
        assert_eq!(mirror(4), 4);
        assert_eq!(mirror(9), 13);
        assert!((1..=28).all(|p| mirror(mirror(p)) == p));
    }

    #[test]
    fn doubles_alternates_right_and_left() {
        let lines = parse("1. topspin deep left\n2. slice short right", true).unwrap();
        let g = generate(&lines, true, None).unwrap();
        let order: Vec<(u8, usize)> = g.planned.iter().map(|p| (p.point, p.player)).collect();
        // Sides are within each player's half: the right-hand player's "left" is just right
        // of the centre line, and its mirror just left of it for the left-hand player.
        assert_eq!(order, vec![(26, 0), (24, 1), (7, 0), (1, 1)]);
        assert_eq!(g.drill().points(), vec![26, 24, 7, 1]);
        assert_eq!(g.receivers, vec![0, 1, 0, 1]);
        assert!(g.players[0].x > 0.0 && g.players[1].x < 0.0, "right player first");
    }

    #[test]
    fn doubles_sides_stay_in_each_half() {
        assert_eq!((1..=7).map(own_half).collect::<Vec<_>>(), vec![4, 5, 5, 6, 6, 7, 7]);
        // No side word sends the right-hand player into the left half.
        let words = ["wide left", "left", "centre left", "centre", "centre right", "right", "wide right"];
        for w in words {
            let line = parse(&format!("1. flat deep {w}"), true).unwrap()[0];
            assert!((line.point - 1) % 7 + 1 >= 4, "{w}");
        }
        assert_eq!(parse("1. flat deep", true).unwrap()[0].point, 27, "missing side: centre of the half");
        assert_eq!(parse("1. flat point 22", true).unwrap()[0].point, 22, "points stay fixed");
        let g = generate(&parse("1. flat deep left\n2. flat short right", true).unwrap(), true, None).unwrap();
        let said: Vec<&str> = g.planned.iter().map(|p| describe_planned(p, true).1 .0).collect();
        assert_eq!(said, vec!["left", "right", "right", "left"]);
    }

    #[test]
    fn saved_drills_round_trip() {
        let tricky = Saved {
            name: "Warm-up".into(),
            text: "1. topspin deep left\r\n2. slice\tshort right\n3. lob \\ point 25 \\n not a break".into(),
            mode: Some(Mode::Defensive),
            doubles: true,
        };
        let stored = encode_saved(&tricky);
        assert!(!stored.contains('\n'), "one line per drill");
        assert_eq!(stored.matches('\t').count(), 2, "only the two field separators");
        let back = decode_saved("Warm-up", &stored).unwrap();
        assert_eq!(back, Saved { text: tricky.text.replace('\r', ""), ..tricky });
        for mode in [None, Some(Mode::Offensive)] {
            let d = Saved { name: "x".into(), text: String::new(), mode, doubles: false };
            assert_eq!(decode_saved("x", &encode_saved(&d)), Some(d));
        }
        assert_eq!(decode_saved("x", "attacking\tsingles\t1. lob"), None);
        assert_eq!(decode_saved("x", "\ttriples\t1. lob"), None);
        assert_eq!(decode_saved("x", "\tsingles\t1. lob \\q"), None);
        assert_eq!(decode_saved("x", "\tsingles"), None);
    }

    #[test]
    fn export_then_import_round_trips() {
        let drills = vec![
            Saved { name: "Net attack".into(), text: "1. volley short left\n2. lob deep".into(), mode: Some(Mode::Defensive), doubles: true },
            Saved { name: "Rally".into(), text: "1. topspin deep\t\"quoted\" \\ ok".into(), mode: None, doubles: false },
        ];
        let file = export_file(&drills);
        assert!(file.contains("\"format\": \"pusun-web drills\""));
        let (all, report) = import_file(&file, &[]).unwrap();
        assert_eq!(all, drills);
        assert_eq!(report, ImportReport { added: 2, ..Default::default() });
        // Importing again changes nothing.
        let (again, report) = import_file(&file, &all).unwrap();
        assert_eq!(again, drills);
        assert_eq!(report, ImportReport { unchanged: 2, ..Default::default() });
    }

    #[test]
    fn import_renames_clashes_and_cleans_untrusted_entries() {
        let mine = vec![Saved { name: "Rally".into(), text: "1. flat deep".into(), mode: None, doubles: false }];
        let json = r#"{"format":"pusun-web drills","version":1,"drills":[
            {"name":"Rally","players":"singles","text":"1. lob deep"},
            {"name":"Rally","players":"singles","text":"1. slice deep"},
            {"name":"  Tab\there\nand\u0007bell  ","players":"doubles","style":"offensive","text":"1. lob\r\n2. flat\u0000"},
            {"name":"","players":"singles","text":"x"},
            {"name":"Bad players","players":"triples","text":"x"},
            {"name":"Bad style","players":"singles","style":"sideways","text":"x"}
        ]}"#;
        let (all, report) = import_file(json, &mine).unwrap();
        let names: Vec<&str> = all.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, vec!["Rally", "Rally (2)", "Rally (3)", "Tab here and bell"]);
        assert_eq!(all[3].text, "1. lob\n2. flat");
        assert_eq!(all[3].mode, Some(Mode::Offensive));
        assert_eq!(report, ImportReport { added: 1, renamed: 2, unchanged: 0, skipped: 3 });
        // Every imported drill survives the browser-storage format.
        for d in &all {
            assert_eq!(decode_saved(&d.name, &encode_saved(d)).as_ref(), Some(d));
            assert!(!d.name.contains(['\t', '\n']));
        }
    }

    #[test]
    fn import_rejects_other_files() {
        assert!(import_file("not json", &[]).is_err());
        assert!(import_file(r#"{"format":"something else","version":1,"drills":[]}"#, &[]).is_err());
        assert!(import_file(r#"{"format":"pusun-web drills","version":2,"drills":[]}"#, &[]).unwrap_err().en.contains("newer"));
        assert!(import_file(&" ".repeat(MAX_IMPORT_BYTES + 1), &[]).unwrap_err().en.contains("too large"));
        let many = format!(r#"{{"format":"pusun-web drills","version":1,"drills":[{}]}}"#,
            (0..MAX_IMPORT_DRILLS + 3).map(|i| format!(r#"{{"name":"d{i}","players":"singles","text":""}}"#)).collect::<Vec<_>>().join(","));
        let (all, report) = import_file(&many, &[]).unwrap();
        assert_eq!((all.len(), report.skipped), (MAX_IMPORT_DRILLS, 3));
    }

    #[test]
    fn same_point_needs_same_shot() {
        let lines = parse("1. topspin deep\n2. lob deep", false).unwrap();
        let err = generate(&lines, false, None).unwrap_err();
        assert!(err[0].en.contains("point 25"));
        let same = parse("1. topspin deep\n2. topspin deep", false).unwrap();
        assert!(generate(&same, false, None).is_ok());
        // The centre column mirrors onto itself with the same shot: fine.
        assert!(generate(&parse("1. lob deep centre", true).unwrap(), true, None).is_ok());
    }

    #[test]
    fn modes_change_pace_and_stay_in_range() {
        let lines = parse("1. flat deep left", false).unwrap();
        let at = |mode| generate(&lines, false, mode).unwrap().drill().params_at(NORMAL)[0].1;
        let (off, none, def) = (at(Some(Mode::Offensive)), at(None), at(Some(Mode::Defensive)));
        assert!(off.speed < none.speed && none.speed < def.speed);
        assert!(off.interval > none.interval && none.interval > def.interval);
        let all = [Shot::Flat, Shot::Drive, Shot::Topspin, Shot::Slice, Shot::Lob, Shot::HighBall, Shot::DropShot, Shot::Volley];
        for shot in all {
            for mode in [None, Some(Mode::Offensive), Some(Mode::Defensive)] {
                for point in 1..=28 {
                    let g = generate(&[Line { shot, point }], false, mode).unwrap();
                    for (_, p) in g.drill().params_at(NORMAL) {
                        assert_eq!(p, p.clamped(), "{shot:?} {mode:?} point {point}");
                    }
                    assert!(drills::court_svg(&g.drill(), false).ends_with("</svg>"));
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use view::DrillBuilder;

#[cfg(target_arch = "wasm32")]
mod view {
    use super::*;
    use crate::app::{SavedList, load_named, save_named};
    use crate::ui::{self, Help, tr};
    use leptos::prelude::*;

    /// Browser storage key for saved drills, one `name<TAB>fields` line each.
    const SAVED_KEY: &str = "pusun-web.builder-drills";

    fn load_saved() -> Vec<Saved> {
        load_named(SAVED_KEY).into_iter().filter_map(|(name, fields)| decode_saved(&name, &fields)).collect()
    }

    fn store_saved(all: &[Saved]) {
        save_named(SAVED_KEY, all.iter().map(|d| (d.name.clone(), encode_saved(d))));
    }

    /// Hands the browser a file to save, named after today's date.
    fn download(contents: &str) -> Result<(), String> {
        use wasm_bindgen::JsCast;
        let parts = js_sys::Array::of1(&contents.into());
        let options = web_sys::BlobPropertyBag::new();
        options.set_type("application/json");
        let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &options).map_err(|_| "blob")?;
        let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(|_| "url")?;
        let date = js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default();
        let a: web_sys::HtmlAnchorElement = document().create_element("a").map_err(|_| "a")?.unchecked_into();
        a.set_href(&url);
        a.set_download(&format!("pusun-drills-{}.json", date.get(..10).unwrap_or("export")));
        a.click();
        // Give the download a moment to start before releasing the blob.
        set_timeout(move || { let _ = web_sys::Url::revoke_object_url(&url); }, std::time::Duration::from_secs(10));
        Ok(())
    }

    fn report_text(r: &ImportReport) -> (String, String) {
        let n = r.added + r.renamed;
        let mut en = format!("Imported {n} {}.", if n == 1 { "drill" } else { "drills" });
        let mut nl = format!("{n} {} geïmporteerd.", if n == 1 { "oefening" } else { "oefeningen" });
        if r.renamed > 0 {
            en += &format!(" {} got a number after the name because that name was taken.", r.renamed);
            nl += &format!(" {} kreeg een nummer achter de naam omdat die naam al bestond.", r.renamed);
        }
        if r.unchanged > 0 {
            en += &format!(" {} {} already saved.", r.unchanged, if r.unchanged == 1 { "was" } else { "were" });
            nl += &format!(" {} {} al opgeslagen.", r.unchanged, if r.unchanged == 1 { "was" } else { "waren" });
        }
        if r.skipped > 0 {
            en += &format!(" {} damaged {} left out.", r.skipped, if r.skipped == 1 { "entry was" } else { "entries were" });
            nl += &format!(" {} beschadigde {} overgeslagen.", r.skipped, if r.skipped == 1 { "invoer is" } else { "invoeren zijn" });
        }
        (en, nl)
    }

    fn say(p: &Problem) -> String {
        if ui::dutch() { p.nl.clone() } else { p.en.clone() }
    }

    /// The Drill Builder panel. `on_start` loads and starts a drill, `on_edit` loads it into
    /// Vary Speed; both are disabled while no machine is connected, like the library's.
    #[component]
    pub fn DrillBuilder(
        on_start: impl Fn(&Generated) + Copy + Send + Sync + 'static,
        on_edit: impl Fn(&Generated) + Copy + Send + Sync + 'static,
        on_stop: impl Fn() + Copy + Send + Sync + 'static,
        connected: Signal<bool>,
        playing: Signal<bool>,
        difficulty: Signal<u8>,
    ) -> impl IntoView {
        let text = RwSignal::new(String::new());
        let mode = RwSignal::new(None::<Mode>);
        // None until a box is ticked; Singles is ticked at first.
        let doubles = RwSignal::new(Some(false));
        let problems = RwSignal::new(Vec::<Problem>::new());
        let result = RwSignal::new(None::<Generated>);
        // The text and choices the shown drill was generated from; this is what Save keeps,
        // even if the text box has been changed since.
        let source = RwSignal::new(None::<Saved>);
        let saved = RwSignal::new(load_saved());

        let run = move || {
            let Some(doubles) = doubles.get_untracked() else {
                problems.set(vec![problem(0, "Tick Singles or Doubles first.", "Vink eerst Enkelspel of Dubbelspel aan.")]);
                result.set(None);
                source.set(None);
                return;
            };
            let (text, mode) = (text.get_untracked(), mode.get_untracked());
            match parse(&text, doubles).and_then(|lines| generate(&lines, doubles, mode)) {
                Ok(g) => {
                    problems.set(Vec::new());
                    result.set(Some(g));
                    source.set(Some(Saved { name: String::new(), text, mode, doubles }));
                }
                Err(p) => {
                    problems.set(p);
                    result.set(None);
                    source.set(None);
                }
            }
        };
        let save = move |name: String| {
            let Some(d) = source.get_untracked() else { return };
            saved.update(|all| {
                all.retain(|x| x.name != name);
                all.push(Saved { name, ..d });
                store_saved(all);
            });
        };
        let load = move |name: String| {
            let Some(d) = saved.get_untracked().into_iter().find(|x| x.name == name) else { return };
            text.set(d.text);
            mode.set(d.mode);
            doubles.set(Some(d.doubles));
            run();
        };
        let delete = move |name: String| {
            saved.update(|all| {
                all.retain(|x| x.name != name);
                store_saved(all);
            });
        };
        let names = Signal::derive(move || saved.get().into_iter().map(|d| d.name).collect::<Vec<_>>());
        // Plain text only: drop control characters other than line breaks and tabs.
        let on_input = move |ev| text.set(clean_text(&event_target_value(&ev)));

        // Export and import of saved drills as a file. `file_note` is (is_error, en, nl).
        let file_note = RwSignal::new(None::<(bool, String, String)>);
        let file_input = NodeRef::<leptos::html::Input>::new();
        let export = move |_| {
            let ok = download(&export_file(&saved.get_untracked())).is_ok();
            file_note.set((!ok).then(|| (true, "The browser did not start the download.".into(), "De browser startte de download niet.".into())));
        };
        let import = move |_| {
            let Some(input) = file_input.get_untracked() else { return };
            let Some(file) = input.files().and_then(|f| f.get(0)) else { return };
            // Let the same file be chosen again later.
            input.set_value("");
            if file.size() > MAX_IMPORT_BYTES as f64 {
                file_note.set(Some((true, "The file is too large to be a drill file (over 1 MB).".into(),
                    "Het bestand is te groot voor een oefeningenbestand (meer dan 1 MB).".into())));
                return;
            }
            leptos::task::spawn_local(async move {
                let read = wasm_bindgen_futures::JsFuture::from(file.text()).await.ok().and_then(|v| v.as_string());
                let Some(json) = read else {
                    file_note.set(Some((true, "The file could not be read.".into(), "Het bestand kon niet worden gelezen.".into())));
                    return;
                };
                match import_file(&json, &saved.get_untracked()) {
                    Ok((all, report)) => {
                        store_saved(&all);
                        saved.set(all);
                        let (en, nl) = report_text(&report);
                        file_note.set(Some((false, en, nl)));
                    }
                    Err(p) => file_note.set(Some((true, p.en, p.nl))),
                }
            });
        };
        let mode_box = move |m: Mode, label: &'static str| {
            let id = if m == Mode::Offensive { "builder-offensive" } else { "builder-defensive" };
            view! {
                <label class="check" for=id>
                    <input type="checkbox" id=id prop:checked=move || mode.get() == Some(m)
                        on:change=move |ev| mode.set(event_target_checked(&ev).then_some(m)) />
                    <span>{move || tr(label)}</span>
                </label>
            }
        };
        let players_box = move |d: bool, label: &'static str| {
            let id = if d { "builder-doubles" } else { "builder-singles" };
            view! {
                <label class="check" for=id>
                    <input type="checkbox" id=id prop:checked=move || doubles.get() == Some(d)
                        on:change=move |ev| doubles.set(event_target_checked(&ev).then_some(d)) />
                    <span>{move || tr(label)}</span>
                </label>
            }
        };

        view! {
            <section class="drill-builder">
                <h2>{move || tr("Drill Builder")}</h2>
                <p class="hint">{move || tr("Describe each ball on its own line, numbered from 1: the shot the machine feeds and where the ball first bounces. In doubles, describe the right-hand player's balls, with left and right meaning their own half; each ball is mirrored for the left-hand player.")}</p>
                <div class="with-help builder-label">
                    <label for="builder-text">{move || tr("Balls")}</label><Help topic="Drill text" />
                </div>
                <textarea id="builder-text" rows="7" maxlength=MAX_TEXT.to_string() spellcheck="false" autocomplete="off"
                    placeholder=move || if ui::dutch() { "1. topspin diep links\n2. slice kort rechts\n3. lob diep midden" } else { "1. topspin deep left\n2. slice short right\n3. lob deep centre" }
                    prop:value=move || text.get() on:input=on_input></textarea>
                <div class="builder-options">
                    <fieldset>
                        <legend>{move || tr("Play style")}</legend>
                        {mode_box(Mode::Offensive, "Offensive")}
                        {mode_box(Mode::Defensive, "Defensive")}
                    </fieldset>
                    <fieldset>
                        <legend>{move || tr("Players")}</legend>
                        {players_box(false, "Singles")}
                        {players_box(true, "Doubles")}
                    </fieldset>
                </div>
                <span class="with-help"><button class="primary" on:click=move |_| run()>{move || tr("Generate Drill")}</button><Help topic="Generate Drill" /></span>
                <Show when=move || !problems.get().is_empty()>
                    <ul class="error builder-problems" role="alert">
                        {move || problems.get().iter().map(|p| view! { <li>{say(p)}</li> }).collect_view()}
                    </ul>
                </Show>
                {move || result.get().map(|g| result_card(g, on_start, on_edit, on_stop, connected, playing, difficulty))}
                <div class="builder-saved">
                    <SavedList title="Saved drills" noun="drill" names=names can_save=Signal::derive(move || source.get().is_some())
                        on_load=load on_delete=delete on_save=save />
                    <div class="row">
                        <span class="with-help"><button on:click=export disabled=move || saved.get().is_empty()>{move || tr("Export file")}</button><Help topic="Export file" /></span>
                        <span class="with-help"><button on:click=move |_| { if let Some(i) = file_input.get_untracked() { i.click() } }>{move || tr("Import file")}</button><Help topic="Import file" /></span>
                        <input type="file" accept=".json,application/json" hidden node_ref=file_input on:change=import
                            aria-label=move || tr("Import file") />
                    </div>
                    {move || file_note.get().map(|(error, en, nl)| view! {
                        <p class=if error { "error" } else { "hint" } role="status">{if ui::dutch() { nl } else { en }}</p>
                    })}
                </div>
            </section>
        }
    }

    fn result_card(
        g: Generated,
        on_start: impl Fn(&Generated) + Copy + Send + Sync + 'static,
        on_edit: impl Fn(&Generated) + Copy + Send + Sync + 'static,
        on_stop: impl Fn() + Copy + Send + Sync + 'static,
        connected: Signal<bool>,
        playing: Signal<bool>,
        difficulty: Signal<u8>,
    ) -> impl IntoView {
        let doubles = g.doubles;
        let balls = g.planned.len();
        let mode = g.mode;
        let planned = g.planned.clone();
        let g = StoredValue::new(g);
        let svg = move || g.with_value(|g| drills::court_svg(&g.drill(), ui::dutch()));
        let summary = move || {
            let (lo, hi) = g.with_value(|g| drills::interval_range(&g.drill().params_at(difficulty.get())));
            let secs = |t: u8| {
                let s = format!("{:.1}", t as f32 / 10.0);
                if ui::dutch() { s.replace('.', ",") } else { s }
            };
            let gap = if lo == hi { secs(lo) } else { format!("{}–{}", secs(lo), secs(hi)) };
            let style = match mode {
                Some(Mode::Offensive) => format!(" · {}", tr("Offensive")),
                Some(Mode::Defensive) => format!(" · {}", tr("Defensive")),
                None => String::new(),
            };
            format!("{} · {balls} {} · {} {gap} s{style}", tr(if doubles { "Doubles" } else { "Singles" }),
                if balls == 1 { tr("ball") } else { tr("balls") }, tr("every"))
        };
        view! {
            <article class="drill builder-result" class:playing=move || playing.get()>
                <div class="drill-head"><strong>{move || if ui::dutch() { "Jouw oefening" } else { "Your drill" }}</strong></div>
                <small class="hint">{summary}</small>
                <div class="builder-court" inner_html=svg></div>
                <ol class="builder-balls">
                    {planned.into_iter().map(|p| {
                        let ((d_en, d_nl), (s_en, s_nl)) = describe_planned(&p, doubles);
                        let (n_en, n_nl) = p.shot.name();
                        let who = doubles.then(|| if p.player == 0 {
                            view! { <i class="key-p1" title=move || tr("Right-hand player")>"1"</i> }.into_any()
                        } else {
                            view! { <i class="key-p2" title=move || tr("Left-hand player")>"2"</i> }.into_any()
                        });
                        view! {
                            <li>{who}{move || if ui::dutch() { format!("{n_nl} · {d_nl} {s_nl} (punt {})", p.point) } else { format!("{n_en} · {d_en} {s_en} (point {})", p.point) }}</li>
                        }
                    }).collect_view()}
                </ol>
                <div class="row">
                    <span class="with-help"><button class="primary" disabled=move || !connected.get()
                        on:click=move |_| g.with_value(|g| on_start(g))>{move || tr("Start")}</button><Help topic="Drill start" /></span>
                    <span class="with-help"><button class="stop" disabled=move || !connected.get()
                        on:click=move |_| on_stop()>{move || tr("Stop")}</button><Help topic="Drill stop" /></span>
                    <span class="with-help"><button on:click=move |_| g.with_value(|g| on_edit(g))>{move || tr("Edit")}</button><Help topic="Drill edit" /></span>
                </div>
            </article>
        }
    }
}
