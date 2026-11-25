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

pub struct Drill {
    pub key: &'static str,
    pub name: (&'static str, &'static str),
    pub description: (&'static str, &'static str),
    pub level: Level,
    pub pause: DrillPause,
    balls: &'static [Ball],
}

impl Drill {
    /// Play order as grid points.
    pub fn points(&self) -> Vec<u8> {
        self.balls.iter().map(|b| b.point).collect()
    }

    /// Settings for each point the drill uses.
    pub fn params(&self) -> Vec<(u8, DrillPoint)> {
        let defaults = DrillPoint::defaults();
        self.balls
            .iter()
            .map(|b| {
                let d = defaults[(b.point - 1) as usize];
                let p = DrillPoint {
                    lr: d.lr,
                    ud: b.height.unwrap_or(d.ud),
                    speed: (d.speed as i16 + b.speed as i16) as u8,
                    interval: b.interval,
                    spin: (50 + 5 * b.spin as i16) as u8,
                };
                (b.point, p.clamped())
            })
            .collect()
    }
}

pub const LIBRARY: &[Drill] = &[
    Drill {
        key: "warmup",
        name: ("Warm-up rally", "Warming-up rally"),
        description: (
            "Calm balls just left and right of centre, 5 s apart. Find your rhythm on both sides before anything faster.",
            "Rustige ballen net links en rechts van het midden, 5 s na elkaar. Vind je ritme aan beide kanten voordat het sneller gaat.",
        ),
        level: Level::Beginner,
        pause: DrillPause::None,
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
        pause: DrillPause::None,
        balls: &[shaped(23, 2000, 0, 40, 2), shaped(27, 2000, 0, 40, 2)],
    },
    Drill {
        key: "corners",
        name: ("Corner to corner", "Hoek naar hoek"),
        description: (
            "Deep balls to both corners, 3.5 s apart. Recover to the centre mark after every shot.",
            "Diepe ballen naar beide hoeken, 3,5 s na elkaar. Herstel na elke slag naar het midden.",
        ),
        level: Level::Advanced,
        pause: DrillPause::Short,
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
        pause: DrillPause::Short,
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
        pause: DrillPause::Long,
        balls: &[shaped(4, 2000, 0, 50, -3), shaped(16, 1500, 10, 30, 0), shaped(20, 1500, 10, 30, 0)],
    },
    Drill {
        key: "lobsmash",
        name: ("Lob and smash", "Lob en smash"),
        description: (
            "High, slow lobs left and right of centre, 6 s apart. Stand near the net, turn sideways and smash.",
            "Hoge, langzame lobs links en rechts van het midden, 6 s na elkaar. Sta dicht bij het net, draai zijwaarts en smash.",
        ),
        level: Level::Intermediate,
        pause: DrillPause::Short,
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
        pause: DrillPause::Short,
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
        pause: DrillPause::None,
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
        pause: DrillPause::Short,
        balls: &[ball(22, 40), ball(4, 40), ball(28, 40), ball(4, 40)],
    },
    Drill {
        key: "pressure",
        name: ("Pressure feed", "Onder druk"),
        description: (
            "Six faster balls all over the court, 3 s apart, then a long pause. Play every ball back deep, however you get there.",
            "Zes snellere ballen over de hele baan, 3 s na elkaar, dan een lange pauze. Speel elke bal diep terug, hoe je er ook komt.",
        ),
        level: Level::Advanced,
        pause: DrillPause::Long,
        balls: &[
            shaped(22, 2000, 5, 30, 1),
            shaped(26, 2000, 5, 30, 1),
            shaped(17, 2000, 5, 30, 1),
            shaped(28, 2000, 5, 30, 1),
            shaped(24, 2000, 5, 30, 1),
            shaped(20, 2000, 5, 30, 1),
        ],
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
            for (id, p) in d.params() {
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
            let params = d.params();
            for (id, p) in &params {
                assert!(params.iter().all(|(j, q)| j != id || q == p), "{}: point {id} differs", d.key);
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
