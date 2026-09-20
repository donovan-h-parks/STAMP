#!/usr/bin/env python3
"""
STAMP-web launcher — start STAMP's web UI like a desktop app.

Runs the FastAPI server (which serves the built React frontend + the API) on a local port
that only 127.0.0.1 can reach, then opens it. Everything stays on this machine — no accounts,
no hosting, no data leaving the desktop. This is the "browser instead of the Qt GUI" entry
point.

Usage:
    python stamp_web.py              # start server, open your default browser
    python stamp_web.py --window     # open in its own app window (needs: pip install pywebview)
    python stamp_web.py --no-browser # just run the server (print the URL)
    python stamp_web.py --port 8000  # pin a port instead of auto-picking a free one

Requires STAMP's .venv313 (fastapi + uvicorn) and a built frontend (webpoc/frontend/dist).
If the frontend isn't built yet, this script builds it once via `npm run build` when npm is
available, otherwise it tells you how.
"""
import argparse
import os
import socket
import subprocess
import sys
import threading
import time
import urllib.request
import webbrowser

HERE = os.path.dirname(os.path.abspath(__file__))
BACKEND = os.path.join(HERE, "backend")
DIST = os.path.join(HERE, "frontend", "dist")


def find_free_port(preferred=None):
    """Return `preferred` if it's free, otherwise an OS-assigned free port."""
    if preferred:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            if s.connect_ex(("127.0.0.1", preferred)) != 0:
                return preferred
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def ensure_frontend_built():
    if os.path.isfile(os.path.join(DIST, "index.html")):
        return True
    frontend = os.path.join(HERE, "frontend")
    from shutil import which
    if which("npm") is None:
        sys.exit("Frontend not built and npm not found.\n"
                 f"Build it once:  cd {frontend} && npm install && npm run build")
    print("Frontend not built yet — building it once (npm run build)…")
    subprocess.check_call(["npm", "install", "--no-audit", "--no-fund"], cwd=frontend)
    subprocess.check_call(["npm", "run", "build"], cwd=frontend)
    return True


def wait_until_up(url, timeout=20.0):
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            urllib.request.urlopen(url, timeout=1)
            return True
        except Exception:
            time.sleep(0.15)
    return False


def main():
    ap = argparse.ArgumentParser(description="Launch the STAMP web UI locally.")
    ap.add_argument("--port", type=int, default=None, help="port (default: auto-pick a free one)")
    ap.add_argument("--window", action="store_true", help="open in a native app window (pywebview)")
    ap.add_argument("--no-browser", action="store_true", help="just run the server, don't open anything")
    args = ap.parse_args()

    ensure_frontend_built()

    # import the FastAPI app from backend/
    sys.path.insert(0, BACKEND)
    import uvicorn
    from app import app  # noqa: E402

    port = find_free_port(args.port or 8000)
    url = f"http://127.0.0.1:{port}"

    server = uvicorn.Server(uvicorn.Config(app, host="127.0.0.1", port=port, log_level="warning"))

    if args.window:
        # pywebview must own the main thread → run uvicorn in a background thread.
        try:
            import webview
        except ImportError:
            sys.exit("--window needs pywebview:  .venv313/bin/python -m pip install pywebview")
        threading.Thread(target=server.run, daemon=True).start()
        if not wait_until_up(url):
            sys.exit("Server did not start in time.")
        print(f"STAMP-web running at {url} (app window)")
        webview.create_window("STAMP", url, width=1280, height=860)
        webview.start()
        server.should_exit = True
        return

    if not args.no_browser:
        threading.Thread(
            target=lambda: (wait_until_up(url) and webbrowser.open(url)), daemon=True
        ).start()

    print(f"STAMP-web running at {url}   (Ctrl+C to stop)")
    server.run()


if __name__ == "__main__":
    main()
