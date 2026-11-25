#!/usr/bin/env bash
# Installs the user services that serve the site at https://pusun.kingconfig.com:
#   pusun-web-origin  static server (server.py) for ~/.local/share/pusun-web/site
#   pusun-web-tunnel  Cloudflare Tunnel to it; reads ~/.config/pusun-web/tunnel.token
# The tunnel token is not in the repo. Re-run after changing a .service file;
# a change to server.py only needs `systemctl --user restart pusun-web-origin`.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(dirname "$HERE")"
UNITS="$HOME/.config/systemd/user"

if [[ ! -f "$HOME/.config/pusun-web/tunnel.token" ]]; then
  echo "warning: ~/.config/pusun-web/tunnel.token is missing; the tunnel will not start" >&2
fi

mkdir -p "$UNITS"
sed "s|@REPO@|$REPO|g" "$HERE/pusun-web-origin.service" > "$UNITS/pusun-web-origin.service"
cp "$HERE/pusun-web-tunnel.service" "$UNITS/pusun-web-tunnel.service"

systemctl --user daemon-reload
systemctl --user enable pusun-web-origin pusun-web-tunnel
systemctl --user restart pusun-web-origin pusun-web-tunnel
systemctl --user --no-pager is-active pusun-web-origin pusun-web-tunnel
