#!/usr/bin/env python3
"""Localhost HTTP sidecar for Shadow's `ModelDetector` (stdlib only + torch).

    python serve.py --model model.pt --host 127.0.0.1 --port 8878

GET  /healthz -> {"ok": true, "model": "<path>", "meta": {...}}
POST /detect  {"text": "..."} -> {"entities": [{"start", "end", "label", "score"}]}
Offsets are Unicode codepoint indices into `text`, end exclusive.
"""
from __future__ import annotations

import argparse
import ipaddress
import json
import socket
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent))

MAX_BODY = 2 * 1024 * 1024


def is_loopback(host: str) -> bool:
    try:
        infos = socket.getaddrinfo(host, None)
    except socket.gaierror:
        return False
    return bool(infos) and all(ipaddress.ip_address(i[4][0].split("%")[0]).is_loopback for i in infos)


def make_handler(model: Any, model_name: str, min_score: float) -> type[BaseHTTPRequestHandler]:
    lock = threading.Lock()  # one forward pass at a time; torch threads internally

    class Handler(BaseHTTPRequestHandler):
        server_version = "shadow-ner/1"

        def _send(self, code: int, body: dict[str, Any]) -> None:
            data = json.dumps(body).encode("utf-8")
            self.send_response(code)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

        def do_GET(self) -> None:  # noqa: N802
            if self.path == "/healthz":
                self._send(200, {"ok": True, "model": model_name, "meta": getattr(model, "meta", {})})
            else:
                self._send(404, {"error": "not found"})

        def do_POST(self) -> None:  # noqa: N802
            if self.path != "/detect":
                return self._send(404, {"error": "not found"})
            try:
                length = int(self.headers.get("Content-Length", "0"))
            except ValueError:
                return self._send(400, {"error": "bad Content-Length"})
            if length > MAX_BODY:
                return self._send(413, {"error": f"body exceeds {MAX_BODY} bytes"})
            try:
                req = json.loads(self.rfile.read(length) or b"{}")
                text = req["text"]
                if not isinstance(text, str):
                    raise TypeError
            except (ValueError, KeyError, TypeError):
                return self._send(400, {"error": 'expected JSON {"text": "..."}'})
            with lock:
                entities = model.predict(text, min_score=min_score)
            self._send(200, {"entities": entities})

        def log_message(self, fmt: str, *args: Any) -> None:
            # Never log request bodies; only method/path/status.
            sys.stderr.write(f"[shadow-ner] {self.address_string()} {fmt % args}\n")

    return Handler


def main() -> None:
    ap = argparse.ArgumentParser(description="Shadow NER sidecar")
    ap.add_argument("--model", default="model.pt")
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=8878)
    ap.add_argument("--device", default="cpu")
    ap.add_argument("--min-score", type=float, default=0.0,
                    help="server-side score floor (Shadow also applies its own min_score)")
    ap.add_argument("--allow-remote", action="store_true",
                    help="permit binding a non-loopback address (prompts would leave this machine)")
    args = ap.parse_args()

    if not is_loopback(args.host) and not args.allow_remote:
        sys.exit(f"refusing to bind non-loopback host {args.host!r}: prompt text must stay on this "
                 "machine. Use --host 127.0.0.1, or --allow-remote if you really mean it.")
    if not Path(args.model).is_file():
        sys.exit(f"model file not found: {args.model}\n"
                 "Train one first:  python train.py --out model.pt --epochs 8 --synthetic 20000")
    try:
        from shadow_ner.model import load
    except ImportError:
        sys.exit("PyTorch is required: python -m venv .venv && pip install -r requirements.txt")

    model = load(args.model, device=args.device)
    server = ThreadingHTTPServer((args.host, args.port), make_handler(model, args.model, args.min_score))
    print(f"shadow-ner serving {args.model} on http://{args.host}:{args.port} (POST /detect, GET /healthz)")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
