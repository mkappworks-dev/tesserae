"""Serve only this prototype on loopback without external dependencies."""

import argparse
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("--port", type=int, default=8133)
args = parser.parse_args()
class PrototypeHandler(SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

    def do_GET(self):
        if "If-Modified-Since" in self.headers:
            del self.headers["If-Modified-Since"]
        route = self.path.split("?", 1)[0].rstrip("/")
        if route in ("", "/worlds", "/mods", "/menu", "/play", "/settings", "/inventory", "/catalogue", "/layout", "/death", "/opening") or route.startswith(
            ("/worlds/", "/settings/")
        ):
            self.path = "/index.html"
        super().do_GET()


handler = partial(PrototypeHandler, directory=str(Path(__file__).parent))
server = ThreadingHTTPServer(("127.0.0.1", args.port), handler)
print(f"Tesserae prototype: http://127.0.0.1:{args.port}", flush=True)
server.serve_forever()
