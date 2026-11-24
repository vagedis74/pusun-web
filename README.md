# pusun-web

Browser controller for the Pusun **PT-9001** tennis ball machine (BLE names `PT7…`/`PT8…`),
written in Rust with Leptos and compiled to WebAssembly. It talks to the machine directly
over Web Bluetooth; there is no server.

## Run

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --version '~0.21' --locked   # 0.22 needs rustc >= 1.96
trunk serve            # http://localhost:8080
trunk build --release  # static files in dist/
cargo test             # protocol byte-for-byte tests (host)
```

The Courses section loads its list from Pusun's API, which sends no CORS headers. `trunk serve`
proxies `/pusun-api/*` to it (see `Trunk.toml`); a static host needs the same proxy rule,
otherwise the section explains why the list cannot load. Videos and images load directly.

Web Bluetooth only works in **Chrome or Edge** (desktop or Android) on `https://` or
`http://localhost`. Safari and all iOS browsers do not support it. To use a phone, host
`dist/` on any HTTPS static host.

## Features

- Connect, battery level, fault alerts (wheel, ball gate, out of balls)
- Speed (13 levels), feed frequency (10), spin (−6…+6), aim left/right and height
- Court height: low / mid / high tables, plus fine height 4–16 on mid
- All patterns from the app's main screen: Fixed point 1–3, Random, Primary, Smash,
  Moon, Volley 1–3, five horizontal and three vertical sweeps, six crosscourt patterns.
  Selecting one applies the app's preset aim, speed and spin.
- Per-pattern defaults: "Save default" stores aim, speed, frequency, spin and fine height
  for the selected pattern; they are applied whenever that pattern is selected
- Program panel: up to 28 of the 41 court points in your own order; save and load
  named programs (points plus frequency, speed, spin and height)
- Custom points: edit six points (aim, height, speed, interval), saved in the browser;
  save and load named point sets (all six points plus the play order)
- Vary Speed drills: up to 15 shots on a 7×4 target grid, each shot tunable (aim, height,
  speed, interval, spin; changes are sent live), optional pause before repeating
- Saved routes (name, order, pause, shot settings) stored in the browser
- AI Coach: random drills from first-ball side, level and ball count (2–9)
- Courses: Pusun's online training courses with their videos; "Practice in Vary Speed"
  loads the course's drill, like the app's Start button (works without a machine connected)
- BLE log of every frame sent and received

## Protocol

Recovered by static analysis of the official Android app v2.3 (`MainActivity7New`);
see `src/protocol.rs`. GATT: write `FFF0/FFF2`, notify `FFF0/FFF1` (PT0/PT1 names use
`FFE5/FFE9` and `FFE0/FFE4`). Frames are `AA cmd d0..d5 chk A5`, `chk` = XOR of bytes 1–7,
except several frames the app sends with fixed trailers. Those are reproduced exactly.

| cmd | meaning |
|---|---|
| `01`–`29` | court points 1–41 (7×5 grid + 6 custom), uploaded after connect |
| `24`+n | Vary Speed shot n: aim, spin, speed, interval (+pause on the last shot) |
| `61` / `62` / `63` | frequency / spin / speed |
| `64` / `65` | left-right / height limits (sent on connect) |
| `66` | apply (after every change and at start) |
| `67` | battery query |
| `6A` | pattern: `01` fixed, `02` horizontal, `03` vertical, `04` random, `05` sequence, `06` custom sequence, `08` drill |
| `6B` | stop |
| `6C` | aim position |
| `6D` | sequence pages (point ids in play order) |

Notifications: `03` battery, `5E` fault, `30` status (unused).

## Court points

Points 1–35 form a 7×5 grid: columns run left to right (left/right value 2070 = left,
210 = right), rows run from short (1–7) to deep (29–35). This follows from the app's
crosscourt labels, e.g. "Left deep & middle light" plays points 22 and 4.
Points 36–41 are the custom points.

## Not verified on hardware

Everything here comes from reading the app's code, not from a captured session.
Check these with a real machine:

- Vary Speed grid depth: all 28 targets share one height; rows differ only in speed
  (98–122), so the UI assumes the fastest row lands deepest and draws it on top.
- Whether the firmware checks the checksum on the quirk frames.
- Spin range: the app's slider limit was read as ±6 from constants, not from the layout.
- Timing: start/stop sequences follow the app's delays, but some gaps are approximated.

## Not ported

- Commands `70` and `71`: hidden factory buttons in the app ("change name", "read value").
