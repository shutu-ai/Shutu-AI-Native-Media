#!/usr/bin/env python3
"""serve.py — M0-T3 getUserMedia 测试页控制服务（stdlib-only）。

职责：
- 静态服务同目录 index.html（http://127.0.0.1:<port>/ 视为安全上下文，getUserMedia 可用）
- GET /cmd      → 返回命令文件当前内容（外部经文件通道下发 stop/start/report/enumerate）
- POST /report  → 页面上报 JSON 追加写入报告文件（jsonl）

用法：
  python -I serve.py --port 8093 --cmdfile <run>/staging/chrome-cmd.txt --reportfile <run>/logs/chrome-reports.jsonl
"""
from __future__ import annotations

import argparse
import json
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

LOCK = threading.Lock()


def build_handler(fixture_dir: Path, cmd_file: Path, report_file: Path) -> type:
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, fmt: str, *args) -> None:  # 静默默认 stderr 噪声
            pass

        def _send(self, code: int, body: bytes, ctype: str = "text/plain; charset=utf-8") -> None:
            self.send_response(code)
            self.send_header("Content-Type", ctype)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self) -> None:
            path = self.path.split("?")[0]
            if path in ("/", "/index.html"):
                data = (fixture_dir / "index.html").read_bytes()
                self._send(200, data, "text/html; charset=utf-8")
            elif path == "/cmd":
                try:
                    cmd = cmd_file.read_text(encoding="utf-8").strip()
                except OSError:
                    cmd = ""
                self._send(200, json.dumps({"cmd": cmd}).encode("utf-8"), "application/json")
            elif path == "/healthz":
                self._send(200, b"ok")
            else:
                self._send(404, b"not found")

        def do_POST(self) -> None:
            if self.path.split("?")[0] != "/report":
                self._send(404, b"not found")
                return
            try:
                length = int(self.headers.get("Content-Length", "0"))
                if length > 262144:  # 256 KiB 上限，防异常载荷
                    self._send(413, b"too large")
                    return
                payload = json.loads(self.rfile.read(length).decode("utf-8"))
                if not isinstance(payload, dict):
                    raise ValueError("not an object")
                with LOCK:
                    with report_file.open("a", encoding="utf-8") as fh:
                        fh.write(json.dumps(payload, ensure_ascii=False) + "\n")
                self._send(200, b"ok")
            except (ValueError, OSError) as exc:
                self._send(400, f"bad request: {exc}".encode("utf-8"))

    return Handler


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=8093)
    ap.add_argument("--dir", default=str(Path(__file__).resolve().parent))
    ap.add_argument("--cmdfile", required=True)
    ap.add_argument("--reportfile", required=True)
    args = ap.parse_args()

    fixture_dir = Path(args.dir).resolve()
    cmd_file = Path(args.cmdfile).resolve()
    report_file = Path(args.reportfile).resolve()
    report_file.parent.mkdir(parents=True, exist_ok=True)
    cmd_file.parent.mkdir(parents=True, exist_ok=True)
    if not cmd_file.exists():
        cmd_file.write_text("none", encoding="utf-8")

    server = ThreadingHTTPServer(("127.0.0.1", args.port), build_handler(fixture_dir, cmd_file, report_file))
    print(f"serving {fixture_dir} on http://127.0.0.1:{args.port}/", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
