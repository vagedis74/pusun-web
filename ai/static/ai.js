// AI drill page: pick options, start a job, poll until Claude's drill is ready, show
// it, and optionally save it into the controller's Saved routes (same origin).
// Everything from the server is inserted with textContent, never as HTML.

const ROUTES_KEY = "pusun-web.routes";
const ROWS = ["short", "mid-short", "mid-deep", "deep"];
const COLS = ["far left", "left", "left of centre", "centre", "right of centre", "right", "far right"];
const $ = (id) => document.getElementById(id);
let current = null;

function el(tag, props = {}, ...kids) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (k === "class") e.className = v;
    else e.setAttribute(k, v);
  }
  for (const k of kids) e.append(k);
  return e;
}

function choice(id) {
  const group = $(id);
  group.addEventListener("click", (ev) => {
    const b = ev.target.closest("button");
    if (!b) return;
    for (const x of group.querySelectorAll("button")) {
      x.classList.toggle("selected", x === b);
      x.setAttribute("aria-pressed", String(x === b));
    }
  });
  return () => group.querySelector("button.selected").dataset.value;
}
const format = choice("format");
const side = choice("side");

function show(id, text) {
  const e = $(id);
  e.textContent = text ?? "";
  e.hidden = !text;
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function generate() {
  const btn = $("generate");
  btn.disabled = true;
  $("again").disabled = true;
  show("error");
  show("saved");
  $("result").hidden = true;
  show("status", "Asking Claude Opus for a drill…");
  try {
    const r = await fetch("api/drills", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ format: format(), side: side() }),
    });
    const j = await r.json().catch(() => ({}));
    if (!r.ok) throw new Error(j.error || `Request failed (${r.status}).`);
    const started = Date.now();
    for (;;) {
      await sleep(2000);
      const p = await fetch(`api/drills/${encodeURIComponent(j.id)}`);
      if (!p.ok) throw new Error(`Lost track of the drill (${p.status}).`);
      const s = await p.json();
      const secs = Math.round((Date.now() - started) / 1000);
      if (s.state === "done") { render(s.drill); show("status"); break; }
      if (s.state === "error") throw new Error(s.error);
      show("status", s.state === "queued"
        ? `Waiting for another drill to finish… ${secs} s`
        : `Claude Opus is designing your drill… ${secs} s`);
      if (secs > 420) throw new Error("This is taking too long; try again.");
    }
  } catch (e) {
    show("status");
    show("error", e.message || String(e));
  } finally {
    btn.disabled = false;
    $("again").disabled = false;
  }
}

function where(b) {
  return `${ROWS[b.row - 1]} ${COLS[b.column - 1]}`;
}

function spinText(s) {
  if (s === 0) return "flat";
  return s > 0 ? `topspin +${s}` : `slice ${s}`;
}

function render(d) {
  current = d;
  $("d-name").textContent = d.name;
  $("d-tags").textContent = `${d.format} · ${d.side === "both" ? "forehand + backhand" : d.side}`;
  $("d-summary").textContent = d.summary;
  $("d-why").textContent = d.why_hard;

  const speeds = d.balls.map((b) => b.kmh);
  const stats = [
    ["Balls per loop", String(d.balls.length)],
    ["Loop", `${d.loop_seconds} s`],
    ["Pace", `${Math.min(...speeds)}–${Math.max(...speeds)}`],
    ["Fastest gap", `${Math.min(...d.balls.map((b) => b.seconds))} s`],
    ["Sets", `${d.structure.sets} × ${d.structure.loops_per_set} loops`],
    ["Rest", `${d.structure.rest_seconds} s`],
  ];
  $("d-stats").replaceChildren(...stats.map(([k, v]) => el("div", {}, el("dt", {}, k), el("dd", {}, v))));

  // Court: row 1 (short) at the top, next to the net, as the player sees it.
  const cells = [];
  for (let p = 1; p <= 28; p++) {
    const c = el("div", { class: "cell" });
    d.balls.forEach((b, i) => {
      if (b.point === p) c.append(el("span", { class: `mark p${b.player}`, title: `Ball ${i + 1}` }, String(i + 1)));
    });
    cells.push(c);
  }
  $("d-court").replaceChildren(...cells);

  $("d-players").replaceChildren(...d.players.map((p, i) =>
    el("li", {}, el("span", { class: `mark p${i + 1}` }, String(i + 1)), `${p.label}: ${p.start}`)));
  $("d-players").hidden = d.format === "singles" && !d.players[0]?.start;

  $("d-balls").replaceChildren(...d.balls.map((b, i) => el("li", {},
    el("span", { class: `mark p${b.player}` }, String(i + 1)),
    el("span", { class: "stroke" }, d.format === "doubles" ? `${b.stroke} · player ${b.player}` : b.stroke),
    el("span", { class: "meta" },
      `${where(b)} (point ${b.point}) · ${b.kmh} pace · ${spinText(b.spin)} · height ${b.height} · next in ${b.seconds} s`),
    el("span", { class: "cue" }, [b.target, b.cue].filter(Boolean).join(" — ")),
  )));

  $("d-coaching").replaceChildren(...d.coaching_points.map((c) => el("li", {}, c)));
  show("d-safety", d.safety ? `Safety: ${d.safety}` : "");
  $("d-notes").replaceChildren(...d.notes.map((n) => el("li", {}, n)));
  $("result").hidden = false;
  $("save").disabled = false;
  $("d-name").scrollIntoView({ behavior: "smooth", block: "start" });
}

function save() {
  if (!current) return;
  const line = current.route;
  const name = line.split("\t", 1)[0];
  let lines = [];
  try {
    lines = (localStorage.getItem(ROUTES_KEY) || "").split("\n").filter(Boolean);
  } catch {
    show("saved", "This browser won't let the page save routes.");
    return;
  }
  lines = lines.filter((l) => l.split("\t", 1)[0] !== name);
  lines.push(line);
  localStorage.setItem(ROUTES_KEY, lines.join("\n"));
  $("save").disabled = true;
  const msg = $("saved");
  msg.replaceChildren(
    `Saved as “${name}”. `,
    el("a", { href: "/" }, "Open the controller"),
    " → Vary Speed → Saved routes, load it and press Start. Difficulty doesn't apply to saved routes; it plays exactly as shown.",
  );
  msg.hidden = false;
}

$("generate").addEventListener("click", generate);
$("again").addEventListener("click", generate);
$("save").addEventListener("click", save);
