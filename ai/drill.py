"""Drill generation for the Pusun PT-9001: the prompt sent to Claude, and strict
validation of what comes back, so every drill can be played as written.

A drill is a Vary Speed route: up to 15 balls, each aimed at one of the 28 grid points.
The machine stores settings per grid point, so a point used twice plays the same way
both times. The route format matches `save_routes` in `src/app.rs`.
"""
from __future__ import annotations

import json
import re
from dataclasses import dataclass, field

# --- machine limits (src/protocol.rs, src/tables.rs) ----------------------------------

GRID_LR = [2130, 1820, 1510, 1200, 890, 580, 270]  # columns 1..7, left to right
UD_RANGE = (300, 4500)        # steps of 100, default 2000
SPEED_RANGE = (50, 140)       # steps of 5; the app shows speed - 30 as km/h
INTERVAL_RANGE = (25, 65)     # tenths of a second, steps of 5
SPIN_RANGE = (-6, 6)          # stored as 50 + 5 * spin
MAX_BALLS = 15               # what Vary Speed can hold
# Balls per loop before it repeats: 8 for one player, 4 each for a doubles pair.
BALLS_PER_PLAYER = {"singles": 8, "doubles": 4}
PAUSES = {"none": 0, "short": 1, "long": 2}

# Generated drills are meant to be hard: keep them in the fast end of the ranges.
SPEED_FLOOR = 110             # 80 km/h shown
INTERVAL_CEIL = 40            # 4.0 s

FORMATS = ("singles", "doubles")
SIDES = ("forehand", "backhand", "both")

STROKES = (
    "forehand", "backhand",
    "forehand volley", "backhand volley",
    "forehand swing volley", "backhand swing volley",
    "smash",
)
FOREHAND_STROKES = {"forehand", "forehand volley", "forehand swing volley"}
BACKHAND_STROKES = {"backhand", "backhand volley", "backhand swing volley"}


class DrillError(ValueError):
    """The model's answer can't be turned into a playable drill."""


# --- prompt ----------------------------------------------------------------------------

SYSTEM_PROMPT = f"""You are an elite tennis coach who designs ball-machine drills for a
Pusun PT-9001. You design very intense, very difficult drills for strong players:
high tempo, wide court coverage, changes of height, depth and spin, and decisions
under fatigue. Never design an easy drill.

THE MACHINE
- It stands in the centre, about 1 m behind the far baseline, and feeds balls in a
  loop: ball 1, 2, ... N, then the pause, then ball 1 again.
- Each ball aims at one of 28 grid points on the player's half: 4 rows x 7 columns.
  Points 1-7 are short (about 3 m from the net), 8-14 are 5.5 m, 15-21 are 8 m
  (behind the service line), 22-28 are deep (about 10.5 m, near the baseline).
  Within a row the column runs from 1 (far LEFT as the player faces the net) to 7
  (far RIGHT). Point = (row - 1) * 7 + column; for example 22 = deep far left,
  25 = deep centre, 28 = deep far right, 4 = short centre.
- Players are RIGHT-HANDED: the left columns (1-3) are the backhand side and the
  right columns (5-7) the forehand side. Column 4 is the middle, where the player
  chooses (or runs around the ball).
- Per ball you set:
  - height: {UD_RANGE[0]}-{UD_RANGE[1]} in steps of 100. 2000 is a normal rally arc;
    1200-1600 is low and flat (for volleys, or skidding slice); 3000-3600 is a
    high, looping ball; 3600-4500 is a lob (for smashes).
  - speed: {SPEED_FLOOR}-{SPEED_RANGE[1]} in steps of 5 (machine units; km/h is speed - 30).
    125-140 is hard pace.
  - interval: seconds x 10 until the NEXT ball, {INTERVAL_RANGE[0]}-{INTERVAL_CEIL} in steps
    of 5 (25 = 2.5 s, the fastest the machine can feed). Give a long run across the
    court enough time to be just barely possible; that is what makes it intense.
  - spin: {SPIN_RANGE[0]} to +{SPIN_RANGE[1]}; positive is topspin (heavy, kicks up),
    negative is backspin/slice (stays low), 0 is flat.
- IMPORTANT: settings belong to the grid point. If you use the same point twice in
  one drill, both balls must have identical height, speed, interval and spin. Use a
  different point if you want a different ball.
- A loop is a fixed number of balls (given in the request), then the machine repeats
  it. pause is "none" or "short" (adds 2 s before the loop repeats).

OUTPUT
Reply with ONE JSON object and nothing else (no markdown, no comments):
{{
  "name": "short, punchy drill name (max 40 chars)",
  "summary": "max 50 words: what the drill trains and how it flows",
  "why_hard": "max 35 words on what makes it brutal",
  "players": [{{"label": "Player 1", "start": "where to start, e.g. centre mark behind the baseline"}}],
  "pause": "none" | "short",
  "balls": [
    {{
      "point": 1-28,
      "height": int,
      "speed": int,
      "interval": int,
      "spin": int,
      "player": 1 | 2,
      "stroke": one of {list(STROKES)},
      "target": "where to hit the reply, e.g. deep crosscourt",
      "cue": "max 12 words of coaching for this ball"
    }}
  ],
  "structure": {{"sets": int, "loops_per_set": int, "rest_seconds": int}},
  "coaching_points": ["3-5 short points"],
  "safety": "one sentence: when to stop or what to watch"
}}"""


def user_prompt(fmt: str, side: str) -> str:
    if fmt == "singles":
        who = ("SINGLES, one player. Exactly 8 balls per loop, then it repeats. Every "
               "ball has \"player\": 1 and players has one entry. Use the full singles court.")
    else:
        who = ("DOUBLES, two players on the same side (one team). Exactly 8 balls per "
               "loop: 4 for player 1 and 4 for player 2, in any order, then it repeats. "
               "players has two entries; each ball says who takes it (1 or 2). Use "
               "doubles patterns: one up one back, both at the net, poach and cover, "
               "switching, lobs over the net player, balls down the middle.")
    focus = {
        "forehand": ("FOREHAND only. Every stroke must be a forehand, forehand volley or "
                     "forehand swing volley (run-around forehands from the backhand "
                     "side are encouraged). No backhands and no smashes."),
        "backhand": ("BACKHAND only. Every stroke must be a backhand, backhand volley or "
                     "backhand swing volley (balls to the forehand side must be taken "
                     "with a backhand only if that is realistic; mostly feed the "
                     "backhand side and the middle). No forehands and no smashes."),
        "both": ("Forehand AND backhand. Mix both sides so the player must change "
                 "grips and direction under pressure: at least a third of the balls "
                 "must be forehands and at least a third backhands (volleys count). "
                 "Smashes are allowed."),
    }[side]
    return (f"Design a new, very intense and difficult drill.\nFormat: {who}\n"
            f"Focus: {focus}\nBe inventive; don't just alternate two corners.")


# --- validation ------------------------------------------------------------------------

@dataclass
class Ball:
    point: int
    height: int
    speed: int
    interval: int
    spin: int
    player: int
    stroke: str
    target: str
    cue: str

    @property
    def row(self) -> int:
        return (self.point - 1) // 7 + 1

    @property
    def column(self) -> int:
        return (self.point - 1) % 7 + 1


@dataclass
class Drill:
    fmt: str
    side: str
    name: str
    summary: str
    why_hard: str
    players: list[dict]
    pause: str
    balls: list[Ball]
    structure: dict
    coaching_points: list[str]
    safety: str
    notes: list[str] = field(default_factory=list)

    def route_line(self) -> str:
        """One `pusun-web.routes` record: name TAB points TAB pause TAB params."""
        points = ",".join(str(b.point) for b in self.balls)
        params = []
        seen = set()
        for b in self.balls:
            if b.point in seen:
                continue
            seen.add(b.point)
            params.append(f"{GRID_LR[b.column - 1]},{b.height},{b.speed},{b.interval},"
                          f"{50 + 5 * b.spin},{b.point}")
        name = re.sub(r"[\t\r\n]+", " ", f"AI: {self.name}").strip()[:60]
        return f"{name}\t{points}\t{PAUSES[self.pause]}\t{';'.join(params)}"

    def to_json(self) -> dict:
        return {
            "format": self.fmt,
            "side": self.side,
            "name": self.name,
            "summary": self.summary,
            "why_hard": self.why_hard,
            "players": self.players,
            "pause": self.pause,
            "balls": [{**b.__dict__, "row": b.row, "column": b.column,
                       "seconds": b.interval / 10, "kmh": b.speed - 30} for b in self.balls],
            "structure": self.structure,
            "coaching_points": self.coaching_points,
            "safety": self.safety,
            "notes": self.notes,
            "route": self.route_line(),
            "loop_seconds": round(sum(b.interval for b in self.balls) / 10
                                  + (2 if self.pause == "short" else 0), 1),
        }


def _snap(v, lo: int, hi: int, step: int) -> int:
    try:
        v = int(round(float(v) / step) * step)
    except (TypeError, ValueError):
        raise DrillError(f"not a number: {v!r}") from None
    return max(lo, min(hi, v))


def _text(v, limit: int, default: str = "") -> str:
    s = str(v if v is not None else default).strip()
    return re.sub(r"\s+", " ", s)[:limit]


def extract_json(text: str) -> dict:
    """The first JSON object in the model's reply (tolerates code fences or prose)."""
    start = text.find("{")
    if start < 0:
        raise DrillError("no JSON object in the reply")
    try:
        obj, _ = json.JSONDecoder().raw_decode(text[start:])
    except json.JSONDecodeError as e:
        raise DrillError(f"invalid JSON: {e}") from None
    if not isinstance(obj, dict):
        raise DrillError("reply is not a JSON object")
    return obj


def validate(raw: dict, fmt: str, side: str) -> Drill:
    """Turn the model's JSON into a playable drill, or raise DrillError."""
    notes: list[str] = []
    n_players = 1 if fmt == "singles" else 2
    per_player = BALLS_PER_PLAYER[fmt]
    want = per_player * n_players
    balls_raw = raw.get("balls")
    if not isinstance(balls_raw, list) or len(balls_raw) != want:
        got = len(balls_raw) if isinstance(balls_raw, list) else 0
        raise DrillError(f"need exactly {want} balls per loop, got {got}")

    balls: list[Ball] = []
    per_point: dict[int, tuple] = {}
    for i, b in enumerate(balls_raw, 1):
        if not isinstance(b, dict):
            raise DrillError(f"ball {i} is not an object")
        try:
            point = int(b.get("point"))
        except (TypeError, ValueError):
            raise DrillError(f"ball {i}: bad point {b.get('point')!r}") from None
        if not 1 <= point <= 28:
            raise DrillError(f"ball {i}: point {point} is off the grid")
        stroke = _text(b.get("stroke"), 40).lower()
        if stroke not in STROKES:
            raise DrillError(f"ball {i}: unknown stroke {stroke!r}")
        player = b.get("player", 1)
        player = player if player in (1, 2) else 1
        if player > n_players:
            raise DrillError(f"ball {i}: player {player} in a singles drill")
        settings = (
            _snap(b.get("height", 2000), *UD_RANGE, 100),
            _snap(b.get("speed", 125), SPEED_FLOOR, SPEED_RANGE[1], 5),
            _snap(b.get("interval", 30), INTERVAL_RANGE[0], INTERVAL_CEIL, 5),
            _snap(b.get("spin", 0), *SPIN_RANGE, 1),
        )
        if point in per_point and per_point[point] != settings:
            # The machine can't play the same point two ways; the first one wins.
            notes.append(f"Ball {i} reuses point {point}, so it plays like ball "
                         f"{next(j for j, x in enumerate(balls, 1) if x.point == point)}.")
            settings = per_point[point]
        per_point.setdefault(point, settings)
        height, speed, interval, spin = settings
        balls.append(Ball(point, height, speed, interval, spin, player, stroke,
                          _text(b.get("target"), 80), _text(b.get("cue"), 100)))

    strokes = [b.stroke for b in balls]
    fh = sum(s in FOREHAND_STROKES for s in strokes)
    bh = sum(s in BACKHAND_STROKES for s in strokes)
    if side == "forehand" and fh != len(balls):
        raise DrillError("forehand drill contains non-forehand strokes")
    if side == "backhand" and bh != len(balls):
        raise DrillError("backhand drill contains non-backhand strokes")
    if side == "both" and (fh < len(balls) / 4 or bh < len(balls) / 4):
        raise DrillError(f"'both' drill is lopsided ({fh} forehands, {bh} backhands)")
    if fmt == "doubles":
        counts = [sum(b.player == p for b in balls) for p in (1, 2)]
        if counts != [per_player, per_player]:
            raise DrillError(f"doubles needs {per_player} balls for each player, "
                             f"got {counts[0]} and {counts[1]}")

    players = raw.get("players") if isinstance(raw.get("players"), list) else []
    players = [{"label": _text(p.get("label"), 30, f"Player {k}"),
                "start": _text(p.get("start"), 120)}
               for k, p in enumerate(players[:n_players], 1) if isinstance(p, dict)]
    while len(players) < n_players:
        players.append({"label": f"Player {len(players) + 1}", "start": ""})

    st = raw.get("structure") if isinstance(raw.get("structure"), dict) else {}
    structure = {
        "sets": _snap(st.get("sets", 4), 1, 10, 1),
        "loops_per_set": _snap(st.get("loops_per_set", 3), 1, 10, 1),
        "rest_seconds": _snap(st.get("rest_seconds", 60), 15, 300, 5),
    }
    cps = raw.get("coaching_points") if isinstance(raw.get("coaching_points"), list) else []
    pause = raw.get("pause") if raw.get("pause") in ("none", "short") else "short"

    return Drill(
        fmt=fmt, side=side,
        name=_text(raw.get("name"), 40, "AI drill") or "AI drill",
        summary=_text(raw.get("summary"), 700),
        why_hard=_text(raw.get("why_hard"), 500),
        players=players, pause=pause, balls=balls, structure=structure,
        coaching_points=[_text(c, 160) for c in cps[:6] if c],
        safety=_text(raw.get("safety"), 200),
        notes=notes,
    )
