"""Static server for the Pusun Web site behind the Cloudflare tunnel.

Like `python3 -m http.server`, but tells browsers to revalidate HTML on every load,
so a phone picks up a new build at once. Trunk's hashed .js/.wasm/.css names change
with each build, so those may be cached for a long time.

It also sends a Content-Security-Policy with every HTML page and does not list
directories. Trunk writes one inline module script into index.html whose text changes
with each build, so the policy allows it by its SHA-256 hash, read from the file
being served.
"""
import base64
import functools
import hashlib
import http
import http.server
import os
import pathlib
import re
import sys

HASHED = re.compile(r"-[0-9a-f]{16}(_bg)?\.(js|wasm|css)$")
INLINE_SCRIPT = re.compile(rb"<script(?![^>]*\bsrc=)[^>]*>(.*?)</script>", re.S | re.I)

# Course images and videos come from Pusun's servers and their CDN, which may change,
# so any HTTPS host is allowed for media. Scripts, styles and fetches stay on this origin.
# 'wasm-unsafe-eval' lets the page compile its WebAssembly; Leptos sets style attributes.
CSP = (
    "default-src 'self'; script-src 'self' 'wasm-unsafe-eval' {hashes}; "
    "style-src 'self' 'unsafe-inline'; img-src 'self' https: data:; media-src 'self' https:; "
    "connect-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'; "
    "frame-ancestors 'none'"
)


def script_hashes(html: bytes) -> str:
    return " ".join(
        "'sha256-" + base64.b64encode(hashlib.sha256(m.group(1)).digest()).decode() + "'"
        for m in INLINE_SCRIPT.finditer(html)
    )


class Handler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        path = self.path.split("?", 1)[0].split("#", 1)[0]
        if HASHED.search(path):
            self.send_header("Cache-Control", "public, max-age=31536000, immutable")
        else:
            self.send_header("Cache-Control", "no-cache")
        self.send_header("X-Content-Type-Options", "nosniff")
        html = self.html_file()
        if html is not None:
            try:
                hashes = script_hashes(html.read_bytes())
            except OSError:
                hashes = ""
            self.send_header("Content-Security-Policy", CSP.format(hashes=hashes))
        super().end_headers()

    def html_file(self):
        """The HTML file this request serves, if any."""
        local = pathlib.Path(self.translate_path(self.path))
        if local.is_dir():
            local = local / "index.html"
        return local if local.suffix in (".html", ".htm") and local.is_file() else None

    def list_directory(self, path):
        self.send_error(http.HTTPStatus.NOT_FOUND, "File not found")
        return None


def main():
    port = int(sys.argv[1])
    root = pathlib.Path(sys.argv[2])
    handler = functools.partial(Handler, directory=os.fspath(root))
    http.server.ThreadingHTTPServer(("127.0.0.1", port), handler).serve_forever()


if __name__ == "__main__":
    main()
