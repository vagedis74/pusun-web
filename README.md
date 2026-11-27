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
proxies `/pusun-api/*` to it (see `Trunk.toml`). Without a proxy, the page loads `courses.json`
from next to itself instead (a saved copy of the API response);
with neither, the section explains why the list cannot load. Videos and images load directly.

Live build: https://pusun.kingconfig.com, served from this machine through a Cloudflare
Tunnel and protected by Cloudflare Access (Zero Trust application `pusun`): only the allowed
email addresses can log in, with a one-time code or another configured login method.
A Bypass policy lets requests from the IP of `telefoon.kingconfig.com` (62.45.87.204,
the home connection) in without a login. Access matches IPs, not hostnames, so update
that policy if the IP changes.
`server/install.sh` installs two user services: `pusun-web-origin` runs `server/server.py`
on `127.0.0.1:8917` (it marks HTML `no-cache` so phones pick up new builds, sends a
Content-Security-Policy that allows Trunk's inline loader script by its hash, and does not list
directories), and
`pusun-web-tunnel` runs `cloudflared` with the token in `~/.config/pusun-web/tunnel.token`
(not in the repo). To publish a build: `trunk build --release && ./deploy.sh`. `deploy.sh`
keeps the site folder's `courses.json`, a saved copy of the course list.

The GitHub Pages deploy (`.github/workflows/pages.yml`) is disabled and the Pages site is
unpublished, because Pages cannot require a login. The workflow file is kept as a reference
for building `courses.json`; it only runs when started by hand, its actions are pinned to
commit SHAs, and only the deploy job gets the Pages write and OIDC token permissions.

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
- Aim display: in the custom point and Vary Speed editors, left/right aim is shown as a percentage
  from the centre column, "L 25%" or "R 25%", where 100% is the machine's limit on that side
  (custom points: 1140 centre, 2070 / 210 limits; Vary Speed: 1200 centre, 2370 / 90 limits, so
  its outer grid columns show L 79% and R 84%). The machine's own values are unitless positions;
  see `drill_lr_percent` and `custom_lr_percent` in `src/protocol.rs`. Height is shown as in the
  app: the raw value for custom points, the value / 100 in Vary Speed.
- Saved routes (name, order, pause, shot settings) stored in the browser
- Drills (tennis): a built-in library (`src/drills.rs`), each drill with a level and description.
  - Twelve singles drills: warm-up rally, deep consistency, corner to corner, run-around forehand,
    approach and volley, first strike, Janny drill, lob and smash, heavy topspin defence, low slice
    handling, figure of eight and pressure feed.
  - Ten doubles drills for a pair: doubles warm-up, down the middle, crosscourt pair, one up one back,
    poach and cover, both at the net, turbo doubles, storm the net, the Cannon of Steenwijk, and
    lob and switch.
  - Start plays a drill at once, Stop stops the machine, Edit opens it in Vary Speed to tune each
    ball first. The playing drill's card is outlined.
  - A 1–5 difficulty setting (remembered in the browser) applies to all drills: 3 plays them as
    written, and each step up feeds the next ball 0.5 s sooner and 3 speed units faster.
  - Setup popup: hovering over (or tapping) a drill's name shows the court from above with the
    machine (centre, about 1 m behind the far baseline, which the drills' aim assumes) and where
    each player starts. A looping animation then plays the drill: a yellow ball flies to where
    each ball lands and leaves a numbered mark (ringed in the colour of the player it is for),
    the players run to where they hit each ball, and each hit shows the stroke (forehand,
    backhand, forehand/backhand volley, swing volley, smash) and a dashed line to where to play
    it. The animation runs at about 0.6× the drill's real timing and starts again from the first
    ball every time the popup opens (hover, focus, or another tap on the name).
  - The animation works out who takes each ball, the stroke and where it is hit from the balls'
    height, speed and landing spot, assuming right-handers and crosscourt replies. A drill can
    fix these per ball (`receivers`, `strokes`, `hit_depths`), as poach and cover, run-around
    forehand, lob and switch and turbo doubles do.
- Help: every **?** button opens a short explanation that closes by itself after a reading time
  (4–12 s, shown by a shrinking bar), or at once on any tap
- AI Coach: random drills from first-ball side, level and ball count (2–9)
- AI drill: a link under Saved routes in Vary Speed opens `/ai/`, where Claude Opus designs a new, very intense drill (see [AI drill generator](#ai-drill-generator-ai)); saved drills appear in Saved routes
- Courses: Pusun's online training courses with their videos, with names, descriptions and
  video titles translated from Chinese (by pattern, in `src/courses.rs`); "Practice in Vary Speed"
  loads the course's drill, like the app's Start button (works without a machine connected)
- Tennis / Padel switch (the app's "Paddle Tennis" screen, `MainActivityPad2`), remembered
  in the browser. Padel uses its own speed table, mid court table (aims 600–700 higher) and
  custom-point defaults, fine height 4–30, padel aim presets, and three extra shots: Bandeja,
  Bajada and Serve return. Vary Speed, AI Coach and Courses exist only on the tennis screen.
  Pattern defaults and the current custom points are stored per sport (the app shares one
  defaults table between both screens); saved programs and point sets are shared, as in the app
- BLE log of every frame sent and received
- Advanced (off by default, "Show advanced" at the bottom): the app's hidden factory
  controls. "Read value" sends `71` and shows the two values of the `30` reply; the point
  editor writes any court point 1–41 with unchecked values, then sends apply

## AI drill generator (`ai/`)

`https://pusun.kingconfig.com/ai/` (behind the same Cloudflare Access login): pick
**Singles** or **Doubles** and **Forehand**, **Backhand** or **Both**, and Claude Opus 5.5
designs a new, very intense drill for the PT-9001. A loop is **8 balls** in singles and
**4 balls per player** (8 in all) in doubles, then it repeats.

- Flask app `ai/app.py` on `127.0.0.1:8918` (user service `pusun-web-ai`); `server/server.py`
  forwards `/ai/` to it, so it shares the controller's origin.
- It calls [claude-gateway](https://github.com/vagedis74/claude-gateway) (`127.0.0.1:8787`,
  token `~/.config/claude-gateway.token`) with model `claude-opus-5-5`. A drill takes about
  30-40 s. It runs as a background job that the page polls, because Cloudflare cuts off
  responses that take longer than 100 s.
- `ai/drill.py` holds the prompt, which describes the 28-point Vary Speed grid and its
  ranges, and the checks. Every value is snapped and clamped to the machine's ranges
  (speed 110-140, gaps 2.5-4.0 s), there must be exactly 8 balls (4 per player in doubles),
  and strokes must match the chosen side. A point used twice must play the same way. One
  repair round is tried if Claude's answer fails the checks.
- **Save to Saved routes** writes the drill to the controller's `pusun-web.routes` (same
  format as `save_routes` in `src/app.rs`). Open the controller → Vary Speed → Saved routes
  → load → Start. The difficulty setting doesn't apply; it plays as shown.
- Limits: one generation at a time, at most 3 queued, 20 per hour (`DRILL_MAX_PER_HOUR`).
  POSTs need a JSON body and a same-origin `Origin`, so other sites can't trigger
  generations through a logged-in browser.
- Tests: `cd ai && .venv/bin/python -m pytest -q tests`.
- The gateway runs on the owner's Claude subscription, which is for personal use. If other
  people on the Access allow list use this, point `DRILL_GATEWAY_URL` / the token at a
  Console API key instead (same Messages API).

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

Notifications: `03` battery, `5E` fault, `30` two values (shown by Advanced → Read value).

## Court points

Points 1–35 form a 7×5 grid: columns run left to right (left/right value 2070 = left,
210 = right), rows run from short (1–7) to deep (29–35). This follows from the app's
crosscourt labels, e.g. "Left deep & middle light" plays points 22 and 4.
Points 36–41 are the custom points.

## Padel mode

Same frames as tennis, except:

- Speed levels 1–13 map to 66, 76, 80, 83, 87, 91, 95, 100, 105, 115, 125, 135, 145.
- The mid table is `BasicData21.b3`; low and high tables are the same as tennis.
- Selection presets (aim, speed): fixed point height 2000 (pusun-web uses 1000 on both screens), volley 1800, smash (1140, 3600) speed 4,
  primary (1140, 2300), moon (1140, 3000), bandeja (1140, 4200) speed 3, bajada (1140, 3800)
  speed 4, serve return (1140, 2000) speed 2. Horizontal and vertical aim at (1320, 2000)
  (tennis: (1320, 900)). Bandeja, bajada and serve return start like fixed point (`6A 01`).
- Reselecting fixed point or volley while running sends `66` with trailer `66` instead of `67`.
- In the official app the padel screen only writes to machines named `PT2…`/`PT8…`;
  for `PT7…` machines it sends nothing. pusun-web sends anyway and shows a warning.

## Keeping the Bluetooth link up

Web Bluetooth gives a page no background service, so the app keeps the link up itself
(`src/app.rs`, `src/ble.rs`):

- **Reconnects by itself.** When the link drops (machine switched off, out of range, phone
  asleep), the app connects to the same machine again without the device chooser, with pauses
  of 0.5 s growing to 8 s (30 s while the page is hidden), until it is back or Disconnect or
  Cancel is pressed. The court table and settings are sent again after every connect.
- **Commands wait for the link.** A command given while the link is down, or one that fails,
  is queued and sent once the link is back, if that is within 15 s. Older commands are dropped,
  so a drill never starts long after it was asked for. Stop clears the queue. A drill that
  was playing when the link dropped is not restarted: press Start again.
- **Restarts a stuck link.** A failed write restarts the link and sends the rest after it,
  up to 3 times in a row; then the error is shown.
- **Keepalive and watchdog.** Every 5 s the link is checked; after 15 s without a command a
  battery query goes out. Commands are written without response, so a write "succeeds" even
  when the machine hears nothing: the machine's answer to the battery query is what proves the
  link works. Once it has answered on a link, 3 unanswered queries in a row (probed every 5 s)
  restart the link, so a silently dead link is found within about 15–20 s. A link that died
  without a disconnect event is restarted too.
- **Stop is sent twice**, 60 ms apart: a dropped write without response gives no error, and a
  lost Stop would leave the machine firing. Writes are at least 10 ms apart, so two commands
  at once cannot overrun the phone's write queue.
- **Follows the phone's Bluetooth switch.** With Bluetooth off the app says so and waits; when
  it comes back on, or the page is shown again, a waiting reconnect tries at once.
- **Log lines carry the time** (HH:MM:SS), so a drop can be traced afterwards.
- **Screen stays on** while connected (Screen Wake Lock), so Android does not put the tab
  and its Bluetooth link to sleep. Coming back to the page re-checks the link and the lock.
- **Reconnects after a reload** without the chooser where Chrome has
  `navigator.bluetooth.getDevices()`. On Chrome for Android this may need
  `chrome://flags/#enable-web-bluetooth-new-permissions-backend` turned on; without it, a
  reload needs one tap on Connect machine.

## Verified on hardware

Tested with a `PT8…` machine (date code 260830, so Vary Speed uses mode `08`) from Chrome
on Android:

- Connecting: the machine is found, GATT connects on `FFF0/FFF2` and `FFF0/FFF1`, and
  notifications start.
- Commands must be written **without response**. With response, Android never got a
  confirmation, so the first write stayed pending and every later command waited behind it.
  `src/ble.rs` uses write without response whenever the characteristic supports it and gives
  up on any write after 3 s.
- Starting a Vary Speed drill from the Drills library: the shot slots (`24`+n), sequence pages
  (`6D`), mode (`6A 08`, trailer `6F`) and apply (`66`, trailer `67`) are accepted, so this
  firmware does not reject those quirk frames for their non-checksum trailers.

Link tests on 2026-11-27 with the same machine (`PT8260830001`), from a Linux PC with
Python `bleak`, sending only the battery query (`67`):

- GATT has service `FFF0` with `FFF1` (notify) and `FFF2` (write and write without response).
- Connecting took 2.0–3.3 s, and 3 disconnects and reconnects in a row all worked.
- The battery query was answered every time (15 of 15) with `BB 03 <level> …`, in
  77–379 ms. The watchdog's 3 s limit for an answer is therefore ample.
- The machine has no idle timeout of 10 minutes or less: with no traffic at all, the link
  stayed up for 10 minutes, and the battery query was still answered in 0.2 s at the end.
  Drops on the phone therefore come from the phone (screen off, tab asleep, range), not from
  the machine. Longer idle times were not tested.

## Not verified on hardware

The rest comes from reading the app's code, not from a captured session.
Check these with a real machine:

- Vary Speed grid depth: all 28 targets share one height; rows differ only in speed
  (98–122), so the UI assumes the fastest row lands deepest and draws it on top.
- Where the library drills' balls land, at each difficulty level (lob and volley heights and
  speeds are estimates). The setup popup's landing spots and hitting positions are drawn from
  the grid, not measured.
- Whether the quirk frames other than those above are accepted, and whether firmware before
  date code 250101 (Vary Speed mode `06`) behaves the same.
- Spin range: the app's slider limit was read as ±6 from constants, not from the layout.
- Timing: start/stop sequences follow the app's delays, but some gaps are approximated.
- Padel mode on any machine, and in particular on `PT7…` machines (see above).
- Switching sport: the app reopens the other screen and reconnects; pusun-web instead stops,
  re-sends the court table, speed and apply on the existing connection.

## Not ported

- Command `70`: hidden factory "change name" button. It sends a fixed payload
  (`03 16 05 0C 01 00`) whose effect is unknown and may rewrite the machine's name/date code.
