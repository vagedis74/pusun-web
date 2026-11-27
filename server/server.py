"""Static server for the Pusun Web site behind the Cloudflare tunnel.

Like `python3 -m http.server`, but tells browsers to revalidate HTML on every load,
so a phone picks up a new build at once. Trunk's hashed .js/.wasm/.css names change
with each build, so those may be cached for a long time.

It also sends a Content-Security-Policy with every HTML page and does not list
directories. Trunk writes one inline module script into index.html whose text changes
with each build, so the policy allows it by its SHA-256 hash, read from the file
being served.

Requests under /ai/ are passed to the AI drill generator (ai/app.py) on 127.0.0.1:8918,
so it shares this origin and login. The generator sets its own headers.
"""
import base64
import functools
import hashlib
import http
import http.client
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


AI_PORT = int(os.environ.get("PUSUN_AI_PORT", "8918"))
AI_PREFIX = "/ai/"
AI_MAX_BODY = 16 * 1024
HOP_BY_HOP = {"connection", "keep-alive", "proxy-authenticate", "proxy-authorization",
              "te", "trailers", "transfer-encoding", "upgrade", "content-length"}


class Handler(http.server.SimpleHTTPRequestHandler):
    status = None
    proxied = False

    def do_GET(self):
        if self.is_ai():
            return self.proxy_ai()
        if self.path.split("?", 1)[0] == "/ai":
            self.send_response(http.HTTPStatus.MOVED_PERMANENTLY)
            self.send_header("Location", AI_PREFIX)
            self.send_header("Content-Length", "0")
            return self.end_headers()
        return super().do_GET()

    def do_HEAD(self):
        return self.proxy_ai() if self.is_ai() else super().do_HEAD()

    def do_POST(self):
        if self.is_ai():
            return self.proxy_ai()
        self.send_error(http.HTTPStatus.METHOD_NOT_ALLOWED)

    def is_ai(self):
        return self.path.startswith(AI_PREFIX)

    def proxy_ai(self):
        try:
            length = int(self.headers.get("Content-Length") or 0)
        except ValueError:
            return self.send_error(http.HTTPStatus.BAD_REQUEST)
        if length > AI_MAX_BODY:
            return self.send_error(http.HTTPStatus.REQUEST_ENTITY_TOO_LARGE)
        body = self.rfile.read(length) if length else None
        headers = {k: v for k, v in self.headers.items()
                   if k.lower() in ("content-type", "accept", "origin", "host", "user-agent")}
        conn = http.client.HTTPConnection("127.0.0.1", AI_PORT, timeout=30)
        try:
            conn.request(self.command, self.path, body=body, headers=headers)
            resp = conn.getresponse()
            data = resp.read()
        except OSError:
            return self.send_error(http.HTTPStatus.BAD_GATEWAY, "AI drill service unavailable")
        finally:
            conn.close()
        self.proxied = True
        self.send_response(resp.status)
        for k, v in resp.getheaders():
            if k.lower() not in HOP_BY_HOP:
                self.send_header(k, v)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(data)

    def send_response(self, code, message=None):
        self.status = code
        super().send_response(code, message)

    def end_headers(self):
        if self.proxied:  # the AI service sends its own caching and security headers
            return super().end_headers()
        path = self.path.split("?", 1)[0].split("#", 1)[0]
        # Only a file that exists may be cached for good: a 404 for a hashed name that a
        # deploy has not copied yet must not stick in browser or edge caches.
        if self.status in (http.HTTPStatus.OK, http.HTTPStatus.NOT_MODIFIED) and HASHED.search(path):
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
