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
    use crate::ui::{self, Help, tr};
    use leptos::prelude::*;

    /// Longest text accepted, in characters; 15 lines of description fit easily.
    const MAX_TEXT: usize = 2000;

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

        let generate = move |_| {
            let Some(doubles) = doubles.get_untracked() else {
                problems.set(vec![problem(0, "Tick Singles or Doubles first.", "Vink eerst Enkelspel of Dubbelspel aan.")]);
                result.set(None);
                return;
            };
            match parse(&text.get_untracked(), doubles).and_then(|lines| generate(&lines, doubles, mode.get_untracked())) {
                Ok(g) => {
                    problems.set(Vec::new());
                    result.set(Some(g));
                }
                Err(p) => {
                    problems.set(p);
                    result.set(None);
                }
            }
        };
        // Plain text only: drop control characters other than line breaks and tabs.
        let on_input = move |ev| {
            let v: String = event_target_value(&ev)
                .chars()
                .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
                .take(MAX_TEXT)
                .collect();
            text.set(v);
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
                <span class="with-help"><button class="primary" on:click=generate>{move || tr("Generate Drill")}</button><Help topic="Generate Drill" /></span>
                <Show when=move || !problems.get().is_empty()>
                    <ul class="error builder-problems" role="alert">
                        {move || problems.get().iter().map(|p| view! { <li>{say(p)}</li> }).collect_view()}
                    </ul>
                </Show>
                {move || result.get().map(|g| result_card(g, on_start, on_edit, on_stop, connected, playing, difficulty))}
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
