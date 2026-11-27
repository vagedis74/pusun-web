"""AI drill generator for pusun-web: a small Flask app that asks Claude Opus (through
claude-gateway) for a very intense drill and hands it back as a playable Vary Speed
route.

Served on 127.0.0.1 only; server/server.py proxies /ai/ to it, so the page shares the
controller's origin (and its Cloudflare Access login) and can save into its routes.

Generation takes 30-90 s, longer than Cloudflare waits for a response, so it runs as a
background job that the page polls.
"""
from __future__ import annotations

import json
import logging
import os
import pathlib
import threading
import time
import urllib.error
import urllib.request
import uuid
from collections import deque

from flask import Flask, abort, jsonify, render_template, request

import drill

GATEWAY_URL = os.environ.get("DRILL_GATEWAY_URL", "http://127.0.0.1:8787").rstrip("/")
TOKEN_FILE = pathlib.Path(os.environ.get(
    "DRILL_GATEWAY_TOKEN_FILE", "~/.config/claude-gateway.token")).expanduser()
MODEL = os.environ.get("DRILL_MODEL", "claude-opus-5-5")
TIMEOUT_S = int(os.environ.get("DRILL_TIMEOUT_S", "300"))
MAX_PER_HOUR = int(os.environ.get("DRILL_MAX_PER_HOUR", "20"))
JOB_TTL_S = 3600

log = logging.getLogger("pusun-ai")
app = Flask(__name__, static_url_path="/ai/static")
app.config["MAX_CONTENT_LENGTH"] = 4096

_jobs: dict[str, dict] = {}
_jobs_lock = threading.Lock()
_one_at_a_time = threading.Semaphore(1)  # don't stack Opus calls on the subscription
_recent: deque[float] = deque()


# --- gateway -----------------------------------------------------------------------

def _ask_claude(messages: list[dict]) -> str:
    body = json.dumps({
        "model": MODEL,
        "max_tokens": 4096,
        "system": drill.SYSTEM_PROMPT,
        "messages": messages,
    }).encode()
    req = urllib.request.Request(
        f"{GATEWAY_URL}/v1/messages", data=body, method="POST",
        headers={"content-type": "application/json",
                 "anthropic-version": "2023-06-01",
                 "x-api-key": TOKEN_FILE.read_text().strip()})
    try:
        with urllib.request.urlopen(req, timeout=TIMEOUT_S) as r:
            data = json.load(r)
    except urllib.error.HTTPError as e:
        try:
            msg = json.load(e).get("error", {}).get("message", "")
        except Exception:
            msg = ""
        if e.code == 429:
            raise RuntimeError("Claude is busy or the usage limit is reached; try again later.") from None
        raise RuntimeError(f"Gateway error {e.code}. {msg[:200]}") from None
    except (urllib.error.URLError, TimeoutError):
        raise RuntimeError("The drill generator can't reach Claude right now.") from None
    return "".join(b.get("text", "") for b in data.get("content", []) if b.get("type") == "text")


def _generate(fmt: str, side: str) -> dict:
    messages = [{"role": "user", "content": drill.user_prompt(fmt, side)}]
    last_error = None
    for attempt in range(2):  # one repair round if the first answer doesn't validate
        reply = _ask_claude(messages)
        try:
            return drill.validate(drill.extract_json(reply), fmt, side).to_json()
        except drill.DrillError as e:
            last_error = e
            log.warning("attempt %d rejected: %s", attempt + 1, e)
            messages += [
                {"role": "assistant", "content": reply},
                {"role": "user", "content": f"That drill is invalid: {e}. Reply with the "
                                            "corrected JSON object only."},
            ]
    raise RuntimeError(f"Claude's drill didn't pass the checks ({last_error}). Try again.")


def _run_job(job_id: str, fmt: str, side: str) -> None:
    with _one_at_a_time:
        _set(job_id, state="running", started=time.time())
        try:
            result = _generate(fmt, side)
            _set(job_id, state="done", drill=result)
        except Exception as e:  # report to the page, never crash the worker
            log.exception("job %s failed", job_id)
            _set(job_id, state="error", error=str(e) if isinstance(e, RuntimeError)
                 else "Something went wrong generating the drill.")


def _set(job_id: str, **kw) -> None:
    with _jobs_lock:
        _jobs[job_id].update(kw)


def _prune() -> None:
    now = time.time()
    with _jobs_lock:
        for k in [k for k, j in _jobs.items() if now - j["created"] > JOB_TTL_S]:
            del _jobs[k]
    while _recent and now - _recent[0] > 3600:
        _recent.popleft()


# --- routes ------------------------------------------------------------------------

@app.after_request
def _headers(resp):
    resp.headers["Content-Security-Policy"] = (
        "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; "
        "connect-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'; "
        "frame-ancestors 'none'")
    resp.headers["X-Content-Type-Options"] = "nosniff"
    resp.headers["Referrer-Policy"] = "same-origin"
    resp.headers["Cache-Control"] = "no-store"
    return resp


@app.get("/ai/")
def index():
    return render_template("index.html")


@app.post("/ai/api/drills")
def create_job():
    # JSON body only: a cross-site form can't send it, so no one can burn the quota
    # through a logged-in browser. Also check Origin when the browser sends it.
    origin = request.headers.get("Origin")
    if origin and origin.split("://", 1)[-1] != request.host:
        abort(403)
    body = request.get_json(silent=True, force=False)
    if not isinstance(body, dict):
        abort(415)
    fmt, side = body.get("format"), body.get("side")
    if fmt not in drill.FORMATS or side not in drill.SIDES:
        return jsonify(error="Choose singles or doubles, and forehand, backhand or both."), 400
    _prune()
    with _jobs_lock:
        if len(_recent) >= MAX_PER_HOUR:
            return jsonify(error=f"Limit of {MAX_PER_HOUR} drills per hour reached."), 429
        if sum(j["state"] in ("queued", "running") for j in _jobs.values()) >= 3:
            return jsonify(error="Already generating drills; wait for those first."), 429
        _recent.append(time.time())
        job_id = uuid.uuid4().hex
        _jobs[job_id] = {"state": "queued", "created": time.time(), "format": fmt, "side": side}
    threading.Thread(target=_run_job, args=(job_id, fmt, side), daemon=True).start()
    return jsonify(id=job_id), 202


@app.get("/ai/api/drills/<job_id>")
def get_job(job_id: str):
    with _jobs_lock:
        job = dict(_jobs.get(job_id) or {})
    if not job:
        abort(404)
    out = {"state": job["state"]}
    if job.get("started"):
        out["elapsed"] = round(time.time() - job["started"])
    if job["state"] == "done":
        out["drill"] = job["drill"]
    elif job["state"] == "error":
        out["error"] = job["error"]
    return jsonify(out)


@app.get("/ai/healthz")
def health():
    return jsonify(ok=True)


logging.basicConfig(level=logging.INFO, format="%(levelname)s %(name)s: %(message)s")
