//! Built-in tennis drill library. Each drill is a Vary Speed drill: grid points 1..=28
//! (rows of seven, short to deep; column 1 is the far left, which the app's help treats
//! as the backhand side of a right-hander) with per-point aim, height, speed, interval and spin.

use crate::protocol::{DrillPause, DrillPoint};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Beginner,
    Intermediate,
    Advanced,
}

impl Level {
    pub fn label(self) -> (&'static str, &'static str) {
        match self {
            Level::Beginner => ("Beginner", "Beginner"),
            Level::Intermediate => ("Intermediate", "Gevorderd"),
            Level::Advanced => ("Advanced", "Expert"),
        }
    }
}

/// One ball of a drill, as a change from the grid point's default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Ball {
    point: u8,
    /// Height (`ud`); `None` keeps the default 2000.
    height: Option<u16>,
    /// Added to the point's default speed (98, 106, 114 or 122 by row).
    speed: i8,
    /// Seconds × 10 until the next ball.
    interval: u8,
    /// −6..=+6; negative is backspin.
    spin: i8,
}

const fn ball(point: u8, interval: u8) -> Ball {
    Ball { point, height: None, speed: 0, interval, spin: 0 }
}

const fn shaped(point: u8, height: u16, speed: i8, interval: u8, spin: i8) -> Ball {
    Ball { point, height: Some(height), speed, interval, spin }
}

/// A spot on the court in metres, as seen by the players facing the net: `x` from the
/// centre line (negative is left; doubles sidelines at ±5.49), `y` from the net (baseline
/// at 11.89). For the machine, `y` is measured from the net into the far half.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub x: f32,
    pub y: f32,
}

const fn at(x: f32, y: f32) -> Spot {
    Spot { x, y }
}

/// Centre of the far baseline, about 1 m behind it. The grid's aim values assume this
/// spot: moved sideways, every ball lands off target.
pub const MACHINE: Spot = at(0.0, 12.9);

const HALF_LENGTH: f32 = 11.885;
const HALF_DOUBLES: f32 = 5.485;
const HALF_SINGLES: f32 = 4.115;
const SERVICE: f32 = 6.4;

/// The court seen from above, machine side at the top, as an SVG document.
pub fn court_svg(d: &Drill) -> String {
    let line = |x1: f32, y1: f32, x2: f32, y2: f32| {
        format!(r#"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}"/>"#)
    };
    let mut s = format!(
        r##"<svg class="court" viewBox="-7 -15 14 30" role="img" xmlns="http://www.w3.org/2000/svg"><rect x="-7" y="-15" width="14" height="30" rx=".4" fill="#2f6e3f"/><g stroke="#fff" stroke-width=".1" fill="none"><rect x="{}" y="{}" width="{}" height="{}"/>"##,
        -HALF_DOUBLES, -HALF_LENGTH, 2.0 * HALF_DOUBLES, 2.0 * HALF_LENGTH
    );
    for x in [-HALF_SINGLES, HALF_SINGLES] {
        s += &line(x, -HALF_LENGTH, x, HALF_LENGTH);
    }
    for y in [-SERVICE, SERVICE] {
        s += &line(-HALF_SINGLES, y, HALF_SINGLES, y);
    }
    s += &line(0.0, -SERVICE, 0.0, SERVICE);
    s += &line(0.0, -HALF_LENGTH, 0.0, -HALF_LENGTH + 0.3);
    s += &line(0.0, HALF_LENGTH, 0.0, HALF_LENGTH - 0.3);
    s += r##"</g><line x1="-6.2" y1="0" x2="6.2" y2="0" stroke="#ddd" stroke-width=".2" stroke-dasharray=".25 .15"/>"##;
    let m = d.machine;
    s += &format!(
        r##"<rect x="{}" y="{}" width="1.4" height="1" rx=".15" fill="#e67e22" stroke="#000" stroke-width=".08"/>"##,
        m.x - 0.7,
        -m.y - 0.5
    );
    s += &ball_path(d);
    let timing = Timing::of(d);
    let who = receivers(d);
    for (i, p) in d.players.iter().enumerate() {
        let fill = PLAYER_COLOURS[i.min(PLAYER_COLOURS.len() - 1)];
        // Keyframes (time, spot): start, then for each of this player's balls run from
        // where they last hit to where they hit this one, and back to the start at the end.
        let mut keys = vec![(0.0, *p)];
        let mut here = *p;
        let mut free = 0.0f32;
        for (j, b) in d.balls.iter().enumerate().filter(|(j, _)| who[*j] == i) {
            let hit_at = timing.starts[j] + FLIGHT;
            let depart = (free + 0.15).min(hit_at - 0.4).max(keys.last().unwrap().0 + 0.01);
            let spot = hit_spot(b);
            keys.push((depart, here));
            keys.push((hit_at.max(depart + 0.01), spot));
            here = spot;
            free = hit_at;
        }
        let home = timing.total - LOOP_REST + 0.3;
        let last_t = keys.last().unwrap().0;
        keys.push((home.max(last_t + 0.01), here));
        keys.push(((timing.total - 0.2).max(home + 0.01), *p));
        keys.push((timing.total, *p));
        let values: Vec<String> = keys.iter().map(|(_, q)| format!("{} {}", q.x, q.y)).collect();
        let times: Vec<String> =
            keys.iter().map(|(t, _)| format!("{:.4}", (t / timing.total).clamp(0.0, 1.0))).collect();
        s += &format!(
            r##"<g transform="translate({x} {y})"><animateTransform attributeName="transform" type="translate" dur="{dur}s" repeatCount="indefinite" values="{values}" keyTimes="{times}"/><circle r=".6" fill="{fill}" stroke="#fff" stroke-width=".08"/><text font-size=".8" font-family="system-ui,sans-serif" font-weight="700" fill="#fff" text-anchor="middle" dominant-baseline="central">{n}</text></g>"##,
            x = p.x,
            y = p.y,
            dur = timing.total,
            values = values.join(";"),
            times = times.join(";"),
            n = i + 1
        );
    }
    s + "</svg>"
}

/// Where a grid point (1..=28) lands, roughly: seven columns across the singles court,
/// four rows from short to deep. Only for the drawing; the machine aims by its own values.
fn landing(point: u8) -> Spot {
    const DEPTHS: [f32; 4] = [3.0, 5.5, 8.0, 10.5];
    let i = point.clamp(1, 28) - 1;
    at((i % 7) as f32 * 1.2 - 3.6, DEPTHS[(i / 7) as usize])
}

/// Marker colours of player 1 and player 2.
pub const PLAYER_COLOURS: [&str; 2] = ["#2b6cb0", "#b7791f"];

/// Which player (index into `players`) each ball is for: as listed in the drill, if it is;
/// otherwise in singles always the one player;
/// in doubles the player starting on the side the ball lands, and a ball through the middle
/// goes to whoever had the ball before (the first ball to player 1).
fn receivers(d: &Drill) -> Vec<usize> {
    if !d.receivers.is_empty() {
        return d.receivers.to_vec();
    }
    let mut prev = 0;
    d.balls
        .iter()
        .map(|b| {
            let x = landing(b.point).x;
            if d.players.len() > 1 && x.abs() > 0.1 {
                prev = d
                    .players
                    .iter()
                    .position(|p| p.x.signum() == x.signum())
                    .unwrap_or(prev);
            }
            prev
        })
        .collect()
}

/// Seconds of animation per second of drill interval.
const ANIMATION_PACE: f32 = 0.6;
/// Flight time of the yellow ball from the machine to where it lands.
const FLIGHT: f32 = 0.7;
/// Rest at the end of each loop, so the order restarts visibly.
const LOOP_REST: f32 = 1.5;

/// Animated yellow balls flying from the machine to each landing spot in play order, each
/// leaving a numbered mark, looping forever.
struct Timing {
    /// When the machine fires each ball, in seconds of animation.
    starts: Vec<f32>,
    /// Length of one loop.
    total: f32,
}

impl Timing {
    fn of(d: &Drill) -> Timing {
        let gap = |b: &Ball| (b.interval as f32 / 10.0 * ANIMATION_PACE).max(FLIGHT + 0.3);
        let starts: Vec<f32> = d
            .balls
            .iter()
            .scan(0.0, |t, b| {
                let start = *t;
                *t += gap(b);
                Some(start)
            })
            .collect();
        let last = d.balls.last().map_or(0.0, gap);
        let total = starts.last().copied().unwrap_or(0.0) + last + LOOP_REST;
        Timing { starts, total }
    }
}

/// Roughly where the player hits ball `b`: low, firm balls are volleyed near the net,
/// high balls (lobs, floaters, shoulder-high balls) are taken out of the air in front of
/// where they would land, and everything else is played after the bounce, behind it.
fn hit_spot(b: &Ball) -> Spot {
    let l = landing(b.point);
    let height = b.height.unwrap_or(2000);
    let y = if height <= 1600 && b.speed > 0 {
        2.5
    } else if height >= 2500 {
        (l.y - 2.0).max(2.0)
    } else {
        (l.y + 2.5).min(12.5)
    };
    at(l.x, y)
}

fn ball_path(d: &Drill) -> String {
    let Timing { starts, total } = Timing::of(d);
    let k = |t: f32| format!("{:.4}", (t / total).clamp(0.0, 1.0));
    let eps = 0.01;
    let (mx, my) = (d.machine.x, -d.machine.y);
    let mut s = String::new();
    for (i, ((b, &start), &who)) in d.balls.iter().zip(&starts).zip(&receivers(d)).enumerate() {
        let l = landing(b.point);
        let ring = PLAYER_COLOURS[who.min(PLAYER_COLOURS.len() - 1)];
        let land = start + FLIGHT;
        let (a, b_, c, e) = (k(start), k(land), k(land + 0.35), k(land + eps));
        s += &format!(
            r##"<g opacity="0"><animate attributeName="opacity" dur="{total}s" repeatCount="indefinite" values="0;0;1;1" keyTimes="0;{b_};{e};1"/><circle cx="{x}" cy="{y}" r=".42" fill="#f4e04d" stroke="{ring}" stroke-width=".14"/><text x="{x}" y="{y}" font-size=".55" font-family="system-ui,sans-serif" font-weight="700" fill="#000" text-anchor="middle" dominant-baseline="central">{n}</text></g>"##,
            x = l.x,
            y = l.y,
            n = i + 1
        );
        s += &format!(
            r##"<circle r=".3" fill="#ffeb3b" stroke="#000" stroke-width=".05" opacity="0" cx="{mx}" cy="{my}"><animate attributeName="cx" dur="{total}s" repeatCount="indefinite" values="{mx};{mx};{x};{x}" keyTimes="0;{a};{b_};1"/><animate attributeName="cy" dur="{total}s" repeatCount="indefinite" values="{my};{my};{y};{y}" keyTimes="0;{a};{b_};1"/><animate attributeName="opacity" dur="{total}s" repeatCount="indefinite" values="0;0;1;1;0;0" keyTimes="0;{a};{a2};{b_};{c};1"/></circle>"##,
            x = l.x,
            y = l.y,
            a2 = k(start + eps),
        );
    }
    s
}

/// Difficulty 1..=5. Each drill is written at [`NORMAL`]; every step up feeds the
/// next ball 0.5 s sooner and a little faster, every step down the opposite.
pub const DIFFICULTIES: [(u8, &str, &str); 5] = [
    (1, "Very easy", "Heel makkelijk"),
    (2, "Easy", "Makkelijk"),
    (3, "Normal", "Normaal"),
    (4, "Hard", "Moeilijk"),
    (5, "Very hard", "Heel moeilijk"),
];
pub const NORMAL: u8 = 3;
/// Interval change per difficulty step, in tenths of a second.
const INTERVAL_STEP: i16 = 5;
/// Speed change per difficulty step. Kept small: speed also sets how deep a ball lands.
const SPEED_STEP: i16 = 3;

pub struct Drill {
    pub key: &'static str,
    pub name: (&'static str, &'static str),
    pub description: (&'static str, &'static str),
    pub level: Level,
    /// Written for two players on one side of the net.
    pub doubles: bool,
    pub pause: DrillPause,
    balls: &'static [Ball],
    /// Where to put the machine on the far side.
    pub machine: Spot,
    /// Where each player starts: the player who gets the first ball, then the partner.
    pub players: &'static [Spot],
    /// Which player (index into `players`) takes each ball; empty to work it out from
    /// where the balls land (see [`receivers`]).
    receivers: &'static [usize],
}

impl Drill {
    /// Play order as grid points.
    pub fn points(&self) -> Vec<u8> {
        self.balls.iter().map(|b| b.point).collect()
    }

    /// Settings for each point the drill uses at `difficulty` (1..=5), clamped to the machine's ranges.
    pub fn params_at(&self, difficulty: u8) -> Vec<(u8, DrillPoint)> {
        let step = difficulty.clamp(1, 5) as i16 - NORMAL as i16;
        let defaults = DrillPoint::defaults();
        self.balls
            .iter()
            .map(|b| {
                let d = defaults[(b.point - 1) as usize];
                let p = DrillPoint {
                    lr: d.lr,
                    ud: b.height.unwrap_or(d.ud),
                    speed: (d.speed as i16 + b.speed as i16 + SPEED_STEP * step).max(0) as u8,
                    interval: (b.interval as i16 - INTERVAL_STEP * step).max(0) as u8,
                    spin: (50 + 5 * b.spin as i16) as u8,
                };
                (b.point, p.clamped())
            })
            .collect()
    }
}

/// Shortest and longest gap between balls in `params`, in tenths of a second.
pub fn interval_range(params: &[(u8, DrillPoint)]) -> (u8, u8) {
    let it = params.iter().map(|(_, p)| p.interval);
    (it.clone().min().unwrap_or(0), it.max().unwrap_or(0))
}

pub const LIBRARY: &[Drill] = &[
    Drill {
        key: "warmup",
        name: ("Warm-up rally", "Warming-up rally"),
        description: (
            "Calm balls just left and right of centre. Find your rhythm on both sides before anything faster.",
            "Rustige ballen net links en rechts van het midden. Vind je ritme aan beide kanten voordat het sneller gaat.",
        ),
        level: Level::Beginner,
        doubles: false,
        pause: DrillPause::None,
        machine: MACHINE,
        players: &[at(0.0, 12.5)],
        receivers: &[],
        balls: &[ball(17, 50), ball(19, 50)],
    },
    Drill {
        key: "deep",
        name: ("Deep consistency", "Diep en constant"),
        description: (
            "Deep balls with light topspin to both halves of the baseline. Aim every reply deep crosscourt.",
            "Diepe ballen met lichte topspin naar beide helften van de achterlijn. Speel elke bal diep cross terug.",
        ),
        level: Level::Intermediate,
        doubles: false,
        pause: DrillPause::None,
        machine: MACHINE,
        players: &[at(0.0, 12.5)],
        receivers: &[],
        balls: &[shaped(23, 2000, 0, 40, 2), shaped(27, 2000, 0, 40, 2)],
    },
    Drill {
        key: "corners",
        name: ("Corner to corner", "Hoek naar hoek"),
        description: (
            "Deep balls to both corners. Recover to the centre mark after every shot.",
            "Diepe ballen naar beide hoeken. Herstel na elke slag naar het midden.",
        ),
        level: Level::Advanced,
        doubles: false,
        pause: DrillPause::Short,
        machine: MACHINE,
        players: &[at(0.0, 12.5)],
        receivers: &[],
        balls: &[shaped(22, 2000, 0, 35, 1), shaped(28, 2000, 0, 35, 1)],
    },
    Drill {
        key: "runaround",
        name: ("Run-around forehand", "Rondlopen voor de forehand"),
        description: (
            "Two balls to the left of centre, then one wide right. Right-handers: run around the backhand for inside-out forehands, then chase the wide forehand.",
            "Twee ballen links van het midden, dan één wijd rechts. Rechtshandigen: loop om de backhand heen voor inside-out forehands en haal dan de wijde forehand.",
        ),
        level: Level::Intermediate,
        doubles: false,
        pause: DrillPause::Short,
        machine: MACHINE,
        players: &[at(0.0, 12.5)],
        receivers: &[],
        balls: &[ball(24, 45), ball(24, 45), ball(28, 45)],
    },
    Drill {
        key: "approach",
        name: ("Approach and volley", "Aanvallen en volleren"),
        description: (
            "A short sliced ball to approach on, then two firm, low balls to volley left and right. A long pause gives time to get back to the baseline.",
            "Een korte slice om op aan te vallen, dan twee stevige, lage ballen om links en rechts te volleren. Een lange pauze geeft tijd om terug te gaan naar de achterlijn.",
        ),
        level: Level::Intermediate,
        doubles: false,
        pause: DrillPause::Long,
        machine: MACHINE,
        players: &[at(0.0, 12.5)],
        receivers: &[],
        balls: &[shaped(4, 2000, 0, 50, -3), shaped(16, 1500, 10, 30, 0), shaped(20, 1500, 10, 30, 0)],
    },
    Drill {
        key: "firststrike",
        name: ("First strike", "Eerste klap"),
        description: (
            "Start on the baseline and work your way in. A deep ball in the middle: hit it early to a corner. A shorter, slower ball to the forehand: step in and go for the winner. A floater in the middle: take it out of the air with a swing volley. A short lob: smash it away. A long pause gives time to walk back.",
            "Begin op de achterlijn en werk naar voren. Een diepe bal in het midden: neem hem vroeg en speel hem naar een hoek. Een kortere, tragere bal op de forehand: stap in en ga voor de winner. Een zwevende bal in het midden: sla hem uit de lucht met een swing volley. Een korte lob: smash hem weg. Een lange pauze geeft tijd om terug te lopen.",
        ),
        level: Level::Advanced,
        doubles: false,
        pause: DrillPause::Long,
        machine: MACHINE,
        players: &[at(0.0, 12.5)],
        receivers: &[],
        balls: &[
            shaped(25, 2000, 0, 40, 1),
            shaped(13, 2000, -4, 35, 0),
            shaped(18, 2800, -6, 35, 0),
            shaped(11, 4000, -6, 50, 0),
        ],
    },
    Drill {
        key: "janny",
        name: ("Janny drill", "Janny-drill"),
        description: (
            "Play close to the net. A very short ball on the left: run in and play it before it bounces twice, then volley the next ball on the left right at the net. Then the same on the right: a very short ball, followed by a volley close to the net.",
            "Speel dicht bij het net. Een heel korte bal links: ren erop af en speel hem voordat hij twee keer stuit, volleer dan de volgende bal links vlak bij het net. Daarna hetzelfde rechts: een heel korte bal, gevolgd door een volley dicht op het net.",
        ),
        level: Level::Intermediate,
        doubles: false,
        pause: DrillPause::Short,
        machine: MACHINE,
        players: &[at(0.0, 6.4)],
        receivers: &[],
        balls: &[
            shaped(2, 1500, -20, 30, -3),
            shaped(9, 1500, 5, 35, 0),
            shaped(6, 1500, -20, 30, -3),
            shaped(13, 1500, 5, 35, 0),
        ],
    },
    Drill {
        key: "lobsmash",
        name: ("Lob and smash", "Lob en smash"),
        description: (
            "High, slow lobs left and right of centre. Stand near the net, turn sideways and smash.",
            "Hoge, langzame lobs links en rechts van het midden. Sta dicht bij het net, draai zijwaarts en smash.",
        ),
        level: Level::Intermediate,
        doubles: false,
        pause: DrillPause::Short,
        machine: MACHINE,
        players: &[at(0.0, 3.0)],
        receivers: &[],
        balls: &[shaped(10, 4000, -6, 60, 0), shaped(12, 4000, -6, 60, 0)],
    },
    Drill {
        key: "topspin",
        name: ("Heavy topspin defence", "Verdedigen tegen topspin"),
        description: (
            "High, deep balls with heavy topspin to left, centre and right. Hold your ground and take the ball before it climbs out of reach.",
            "Hoge, diepe ballen met veel topspin naar links, midden en rechts. Blijf staan en neem de bal voordat hij te hoog opspringt.",
        ),
        level: Level::Advanced,
        doubles: false,
        pause: DrillPause::Short,
        machine: MACHINE,
        players: &[at(0.0, 13.0)],
        receivers: &[],
        balls: &[shaped(22, 2600, 0, 45, 5), shaped(25, 2600, 0, 45, 5), shaped(28, 2600, 0, 45, 5)],
    },
    Drill {
        key: "slice",
        name: ("Low slice handling", "Lage slice verwerken"),
        description: (
            "Low, skidding backspin balls to mid-court left and right. Bend your knees and lift the ball over the net.",
            "Lage, doorschietende ballen met backspin naar het midden van de baan, links en rechts. Buig door je knieën en til de bal over het net.",
        ),
        level: Level::Intermediate,
        doubles: false,
        pause: DrillPause::None,
        machine: MACHINE,
        players: &[at(0.0, 12.5)],
        receivers: &[],
        balls: &[shaped(9, 1600, 0, 45, -4), shaped(13, 1600, 0, 45, -4)],
    },
    Drill {
        key: "figure8",
        name: ("Figure of eight", "Achtje"),
        description: (
            "Deep left, short centre, deep right, short centre. Train moving forward and back as well as side to side.",
            "Diep links, kort midden, diep rechts, kort midden. Train naar voren en naar achteren, en van links naar rechts.",
        ),
        level: Level::Advanced,
        doubles: false,
        pause: DrillPause::Short,
        machine: MACHINE,
        players: &[at(0.0, 12.5)],
        receivers: &[],
        balls: &[ball(22, 40), ball(4, 40), ball(28, 40), ball(4, 40)],
    },
    Drill {
        key: "pressure",
        name: ("Pressure feed", "Onder druk"),
        description: (
            "Six faster balls all over the court, then a long pause. Play every ball back deep, however you get there.",
            "Zes snellere ballen over de hele baan, dan een lange pauze. Speel elke bal diep terug, hoe je er ook komt.",
        ),
        level: Level::Advanced,
        doubles: false,
        pause: DrillPause::Long,
        machine: MACHINE,
        players: &[at(0.0, 12.5)],
        receivers: &[],
        balls: &[
            shaped(22, 2000, 5, 30, 1),
            shaped(26, 2000, 5, 30, 1),
            shaped(17, 2000, 5, 30, 1),
            shaped(28, 2000, 5, 30, 1),
            shaped(24, 2000, 5, 30, 1),
            shaped(20, 2000, 5, 30, 1),
        ],
    },
    // Doubles: two players on the receiving side. Left and right are the halves of the court.
    Drill {
        key: "doubleswarmup",
        name: ("Doubles warm-up", "Warming-up dubbelspel"),
        description: (
            "Both players on the baseline, one per half. Calm, slower balls alternate to the left and right player, each to the middle of their half. Rally them back easily to find your rhythm.",
            "Beide spelers op de achterlijn, één per helft. Rustige, langzamere ballen wisselen naar de linker- en rechterspeler, elk naar het midden van zijn helft. Speel ze ontspannen terug om in je ritme te komen.",
        ),
        level: Level::Beginner,
        doubles: true,
        pause: DrillPause::None,
        machine: MACHINE,
        players: &[at(-2.7, 12.5), at(2.7, 12.5)],
        receivers: &[],
        balls: &[shaped(16, 2000, -6, 50, 0), shaped(20, 2000, -6, 50, 0)],
    },
    Drill {
        key: "middle",
        name: ("Down the middle", "Door het midden"),
        description: (
            "Both players on the baseline. Calm balls through the middle. Call who takes each one; usually the player with the forehand in the middle.",
            "Beide spelers op de achterlijn. Rustige ballen door het midden. Roep wie hem neemt; meestal de speler met de forehand in het midden.",
        ),
        level: Level::Beginner,
        doubles: true,
        pause: DrillPause::None,
        machine: MACHINE,
        players: &[at(-2.7, 12.5), at(2.7, 12.5)],
        receivers: &[],
        balls: &[ball(25, 50), ball(18, 50)],
    },
    Drill {
        key: "crosscourt",
        name: ("Crosscourt pair", "Cross met z’n tweeën"),
        description: (
            "Both players on the baseline, one per half. Deep balls alternate right and left; each player drives their ball back crosscourt, as on a return of serve.",
            "Beide spelers op de achterlijn, één per helft. Diepe ballen wisselen rechts en links; elke speler slaat zijn bal cross terug, zoals bij een return.",
        ),
        level: Level::Intermediate,
        doubles: true,
        pause: DrillPause::None,
        machine: MACHINE,
        players: &[at(2.7, 12.5), at(-2.7, 12.5)],
        receivers: &[],
        balls: &[shaped(27, 2000, 0, 40, 1), shaped(23, 2000, 0, 40, 1)],
    },
    Drill {
        key: "oneupback",
        name: ("One up, one back", "Eén voor, één achter"),
        description: (
            "Baseline player on the right, net player on the left. A deep ball for the baseline player, then a firm, low ball for the net player to volley. Swap sides after each set.",
            "Achterspeler rechts, netspeler links. Een diepe bal voor de achterspeler, dan een stevige, lage bal voor de netspeler om te volleren. Wissel na elke serie van kant.",
        ),
        level: Level::Intermediate,
        doubles: true,
        pause: DrillPause::None,
        machine: MACHINE,
        players: &[at(2.7, 12.5), at(-2.7, 3.0)],
        receivers: &[],
        balls: &[shaped(27, 2000, 0, 35, 1), shaped(16, 1500, 10, 40, 0)],
    },
    Drill {
        key: "poach",
        name: ("Poach and cover", "Onderscheppen en dekken"),
        description: (
            "Net player starts on the left and crosses to poach a firm ball through the middle. The partner switches behind them and covers the deep ball to the left that follows.",
            "De netspeler begint links en steekt over om een stevige bal door het midden te onderscheppen. De partner wisselt erachter langs en dekt de diepe bal naar links die volgt.",
        ),
        level: Level::Advanced,
        doubles: true,
        pause: DrillPause::Short,
        machine: MACHINE,
        players: &[at(-2.7, 3.0), at(2.7, 12.5)],
        receivers: &[0, 1],
        balls: &[shaped(18, 1500, 10, 40, 0), shaped(22, 2000, 0, 45, 1)],
    },
    Drill {
        key: "netpair",
        name: ("Both at the net", "Samen aan het net"),
        description: (
            "Both players at the net. Fast, low balls left, middle, right, middle. Keep the volleys low and agree who takes the middle ball.",
            "Beide spelers aan het net. Snelle, lage ballen links, midden, rechts, midden. Houd de volleys laag en spreek af wie de middelste bal neemt.",
        ),
        level: Level::Advanced,
        doubles: true,
        pause: DrillPause::Short,
        machine: MACHINE,
        players: &[at(-2.7, 3.0), at(2.7, 3.0)],
        receivers: &[],
        balls: &[
            shaped(16, 1500, 10, 25, 0),
            shaped(18, 1500, 10, 25, 0),
            shaped(20, 1500, 10, 25, 0),
            shaped(18, 1500, 10, 25, 0),
        ],
    },
    Drill {
        key: "turbo",
        name: ("Turbo doubles", "Turbo dubbel"),
        description: (
            "Both players start on the baseline, one per half. The right player gets a deep forehand, sprints to the net for a low or normal volley in the middle, then backpedals for a smash. Then the left player gets the same three balls.",
            "Beide spelers beginnen op de achterlijn, één per helft. De rechterspeler krijgt een diepe forehand, sprint naar het net voor een lage of normale volley in het midden en loopt dan achteruit terug voor een smash. Daarna krijgt de linkerspeler dezelfde drie ballen.",
        ),
        level: Level::Advanced,
        doubles: true,
        pause: DrillPause::Short,
        machine: MACHINE,
        players: &[at(2.7, 12.5), at(-2.7, 12.5)],
        receivers: &[],
        balls: &[
            shaped(27, 2000, 0, 50, 1),
            shaped(18, 1500, 5, 40, 0),
            shaped(20, 4000, -6, 45, 0),
            shaped(24, 2000, 0, 50, 1),
            shaped(18, 1500, 5, 40, 0),
            shaped(16, 4000, -6, 45, 0),
        ],
    },
    Drill {
        key: "storm",
        name: ("Storm the net", "Stormloop naar het net"),
        description: (
            "Both players on the baseline, one per half. A short, sliced ball in the middle: attack it with the forehand and both move in together. Then a firm volley to the right, a low volley to the left, and a high, soft ball in the middle to put away. Never back off; finish every point at the net.",
            "Beide spelers op de achterlijn, één per helft. Een korte bal met slice in het midden: val aan met de forehand en ga samen naar voren. Dan een stevige volley rechts, een lage volley links en een hoge, zachte bal in het midden om af te maken. Nooit terugwijken; maak elk punt af aan het net.",
        ),
        level: Level::Advanced,
        doubles: true,
        pause: DrillPause::Short,
        machine: MACHINE,
        players: &[at(-2.7, 12.5), at(2.7, 12.5)],
        receivers: &[],
        balls: &[
            shaped(11, 2000, -6, 35, -2),
            shaped(20, 1500, 5, 30, 0),
            shaped(16, 1500, 5, 30, 0),
            shaped(18, 3200, -8, 45, 0),
        ],
    },
    Drill {
        key: "kanon",
        name: ("The Cannon of Steenwijk", "Het Kanon van Steenwijk"),
        description: (
            "Both players on the baseline, one per half. The left player first: a deep ball on the left, backhand crosscourt. Then a shorter ball: move in, take it about 1.5 m inside the baseline and attack with a forehand crosscourt from left to right. Then close in to 2–3 m from the net for a ball through the middle at shoulder height: hammer it out of the air into a corner or straight ahead. Then the right player gets the same three balls, mirrored on the right half.",
            "Beide spelers op de achterlijn, één per helft. Eerst de linkerspeler: een diepe bal links, backhand cross. Dan een kortere bal: loop in, neem hem zo’n 1,5 meter voor de baseline en val aan met een forehand cross van links naar rechts. Loop daarna op tot 2 à 3 meter voor het net voor een bal door het midden op schouderhoogte: ram die in één keer uit de lucht naar een hoek of rechtdoor. Daarna krijgt de rechterspeler dezelfde drie ballen, gespiegeld op de rechterhelft.",
        ),
        level: Level::Advanced,
        doubles: true,
        pause: DrillPause::Long,
        machine: MACHINE,
        players: &[at(-2.7, 12.5), at(2.7, 12.5)],
        receivers: &[],
        balls: &[
            shaped(23, 2000, 0, 40, 1),
            shaped(17, 2000, -4, 35, 0),
            shaped(11, 2500, 0, 50, 0),
            shaped(27, 2000, 0, 40, 1),
            shaped(19, 2000, -4, 35, 0),
            shaped(11, 2500, 0, 50, 0),
        ],
    },
    Drill {
        key: "lobswitch",
        name: ("Lob and switch", "Lob en wissel"),
        description: (
            "Both players at the net. High lobs over the left and right player. The partner crosses behind to chase the lob while the lobbed player switches sides.",
            "Beide spelers aan het net. Hoge lobs over de linker- en rechterspeler. De partner loopt erachter langs om de lob te halen en de gelobde speler wisselt van kant.",
        ),
        level: Level::Intermediate,
        doubles: true,
        pause: DrillPause::Short,
        machine: MACHINE,
        players: &[at(-2.7, 3.0), at(2.7, 3.0)],
        receivers: &[],
        balls: &[shaped(23, 3800, -10, 60, 0), shaped(27, 3800, -10, 60, 0)],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::MAX_DRILL_SHOTS;

    #[test]
    fn drills_fit_the_machine() {
        for d in LIBRARY {
            assert!(!d.balls.is_empty() && d.balls.len() <= MAX_DRILL_SHOTS, "{}: ball count", d.key);
            for b in d.balls {
                assert!((1..=28).contains(&b.point), "{}: point {}", d.key, b.point);
                assert!((-6..=6).contains(&b.spin), "{}: spin", d.key);
            }
            // Out-of-range values would be clamped silently; keep the library exact.
            for (id, p) in d.params_at(NORMAL) {
                assert_eq!(p, p.clamped(), "{}: point {id} out of range", d.key);
                let grid = DrillPoint::defaults()[(id - 1) as usize];
                assert_eq!(p.lr, grid.lr, "{}: aim comes from the grid", d.key);
            }
        }
    }

    /// Settings belong to the point, so a point used twice must be described the same way.
    #[test]
    fn repeated_points_agree() {
        for d in LIBRARY {
            for (level, ..) in DIFFICULTIES {
                let params = d.params_at(level);
                for (id, p) in &params {
                    assert!(params.iter().all(|(j, q)| j != id || q == p), "{}: point {id} differs", d.key);
                }
            }
        }
    }

    #[test]
    fn difficulty_never_eases_with_level() {
        for d in LIBRARY {
            for w in DIFFICULTIES.windows(2) {
                for ((_, easier), (_, harder)) in d.params_at(w[0].0).iter().zip(d.params_at(w[1].0)) {
                    assert!(harder.interval <= easier.interval && harder.speed >= easier.speed, "{}", d.key);
                    assert_eq!(harder, harder.clamped(), "{}: clamped", d.key);
                }
            }
            // Easiest and hardest must actually differ in tempo.
            let (easy, hard) = (d.params_at(1), d.params_at(5));
            assert!(easy.iter().zip(&hard).any(|((_, a), (_, b))| a.interval > b.interval), "{}", d.key);
        }
    }

    #[test]
    fn has_singles_and_doubles() {
        assert!(LIBRARY.iter().any(|d| d.doubles) && LIBRARY.iter().any(|d| !d.doubles));
    }

    #[test]
    fn spots_are_on_the_court() {
        for d in LIBRARY {
            assert_eq!(d.players.len(), if d.doubles { 2 } else { 1 }, "{}: players", d.key);
            assert!(d.receivers.is_empty() || d.receivers.len() == d.balls.len(), "{}: receivers", d.key);
            assert!(d.receivers.iter().all(|&r| r < d.players.len()), "{}: receivers", d.key);
            for p in d.players.iter().chain([&d.machine]) {
                assert!(p.x.abs() <= HALF_DOUBLES && (0.0..=14.0).contains(&p.y), "{}: {p:?}", d.key);
            }
        }
    }

    #[test]
    fn animations_are_well_formed() {
        for d in LIBRARY {
            let svg = court_svg(d);
            for part in svg.split("keyTimes=\"").skip(1) {
                let times: Vec<f32> =
                    part.split('"').next().unwrap().split(';').map(|t| t.parse().unwrap()).collect();
                assert_eq!(times[0], 0.0, "{}", d.key);
                assert_eq!(*times.last().unwrap(), 1.0, "{}", d.key);
                assert!(times.windows(2).all(|w| w[0] <= w[1]), "{}: {times:?}", d.key);
            }
            for part in svg.split(" values=\"").skip(1).zip(svg.split("keyTimes=\"").skip(1)) {
                let n = |s: &str| s.split('"').next().unwrap().split(';').count();
                assert_eq!(n(part.0), n(part.1), "{}: values vs keyTimes", d.key);
            }
        }
    }

    #[test]
    fn keys_are_unique() {
        for (i, d) in LIBRARY.iter().enumerate() {
            assert!(LIBRARY[i + 1..].iter().all(|e| e.key != d.key), "duplicate key {}", d.key);
        }
    }
}
