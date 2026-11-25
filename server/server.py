"""Static server for the Pusun Web site behind the Cloudflare tunnel.

Like `python3 -m http.server`, but tells browsers to revalidate HTML on every load,
so a phone picks up a new build at once. Trunk's hashed .js/.wasm/.css names change
with each build, so those may be cached for a long time.
"""
import functools
import http.server
import pathlib
import re
import sys

HASHED = re.compile(r"-[0-9a-f]{16}(_bg)?\.(js|wasm|css)$")


class Handler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        path = self.path.split("?", 1)[0]
        if HASHED.search(path):
            self.send_header("Cache-Control", "public, max-age=31536000, immutable")
        else:
            self.send_header("Cache-Control", "no-cache")
        super().end_headers()


def main():
    port = int(sys.argv[1])
    root = pathlib.Path(sys.argv[2])
    handler = functools.partial(Handler, directory=str(root))
    http.server.ThreadingHTTPServer(("127.0.0.1", port), handler).serve_forever()


if __name__ == "__main__":
    main()
