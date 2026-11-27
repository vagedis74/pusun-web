#!/usr/bin/env bash
# Installs the user services that serve the site at https://pusun.kingconfig.com:
#   pusun-web-origin  static server (server.py) for ~/.local/share/pusun-web/site
#   pusun-web-tunnel  Cloudflare Tunnel to it; reads ~/.config/pusun-web/tunnel.token
#   pusun-web-ai      AI drill generator (ai/app.py) on 127.0.0.1:8918, proxied at /ai/;
#                     needs claude-gateway and its token in ~/.config/claude-gateway.token
# The tunnel token is not in the repo. Re-run after changing a .service file;
# a change to server.py only needs `systemctl --user restart pusun-web-origin`.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(dirname "$HERE")"
UNITS="$HOME/.config/systemd/user"

if [[ ! -f "$HOME/.config/pusun-web/tunnel.token" ]]; then
  echo "warning: ~/.config/pusun-web/tunnel.token is missing; the tunnel will not start" >&2
fi

if [[ ! -x "$REPO/ai/.venv/bin/gunicorn" ]]; then
  python3 -m venv "$REPO/ai/.venv"
  "$REPO/ai/.venv/bin/pip" install -q -r "$REPO/ai/requirements.txt"
fi

mkdir -p "$UNITS"
sed "s|@REPO@|$REPO|g" "$HERE/pusun-web-origin.service" > "$UNITS/pusun-web-origin.service"
sed "s|@REPO@|$REPO|g" "$HERE/pusun-web-ai.service" > "$UNITS/pusun-web-ai.service"
cp "$HERE/pusun-web-tunnel.service" "$UNITS/pusun-web-tunnel.service"

systemctl --user daemon-reload
systemctl --user enable pusun-web-origin pusun-web-tunnel pusun-web-ai
systemctl --user restart pusun-web-ai pusun-web-origin pusun-web-tunnel
systemctl --user --no-pager is-active pusun-web-ai pusun-web-origin pusun-web-tunnel
