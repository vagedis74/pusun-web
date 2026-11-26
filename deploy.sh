#!/usr/bin/env bash
# Publish the current build to the local site folder served at
# https://pusun.kingconfig.com (pusun-web-origin + pusun-web-tunnel user services).
# Run `trunk build --release` first; this script only copies dist/.
set -euo pipefail

SRC="$(cd "$(dirname "$0")" && pwd)/dist/"
DEST="${PUSUN_SITE_DIR:-$HOME/.local/share/pusun-web/site}/"

if [[ ! -f "$SRC/index.html" ]]; then
  echo "error: $SRC has no index.html; run 'trunk build --release' first" >&2
  exit 1
fi

mkdir -p "$DEST"
# Copy the new hashed assets first and index.html last, so a page load during the
# deploy never gets an index.html whose .js/.wasm/.css are not there yet.
rsync -a --exclude=/index.html "$SRC" "$DEST"
# --delete drops stale hashed .js/.wasm/.css files from earlier builds.
# courses.json is not part of the build (it is a saved copy of Pusun's course
# list that the Courses section falls back to), so keep it.
rsync -a --delete --exclude=/courses.json "$SRC" "$DEST"

echo "Deployed $(find "$DEST" -type f | wc -l) files to $DEST"
