#!/usr/bin/env python
"""CDP probe for the game browser that Akagi v3 launches.

Akagi picks a **random** remote-debugging port on every start
(`TcpListener::bind(("127.0.0.1", 0))` in `capture/chromium/launch.rs`), so
this script finds it: it looks for the running Chrome whose
`--user-data-dir` is the `chrome-profile` next to `akagi.exe` and reads its
`--remote-debugging-port`.

Usage:
  python cdp.py rect                     # canvas geometry + viewport
  python cdp.py eval "<js expression>"   # evaluate JS in the page
  python cdp.py shot "<abs output.png>"  # screenshot the page
  python cdp.py click <norm_x> <norm_y> [--shot out.png] [--no-click]
                                         # click a 16:9-normalised point on
                                         # the game canvas — the same maths
                                         # and the same CDP events the
                                         # autoplay module uses

The click path is deliberately identical to the module's: hover, then
press, then release, all through `Input.dispatchMouseEvent`. The
coordinates are normalised against the canvas' own rect, so they do not
depend on the window size.

Needs `websocket-client` (`pip install websocket-client`).
"""
import base64
import json
import os
import re
import subprocess
import sys
import time
import urllib.request


def find_port():
    """The remote-debugging port of the running v3 game browser."""
    if len(sys.argv) > 1 and sys.argv[1].startswith("--port="):
        return int(sys.argv[1].split("=", 1)[1])

    # 1) the process list — the authoritative source.
    #    The command prints *only* the port number: the command line holds
    #    non-ASCII paths, and the console's codepage is not UTF-8.
    try:
        ps = subprocess.run(
            [
                "powershell",
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_Process -Filter \"Name='chrome.exe'\" | "
                "Where-Object { $_.CommandLine -like '*chrome-profile*' } | "
                "ForEach-Object { if ($_.CommandLine -match "
                "'--remote-debugging-port=(\\d+)') { $matches[1] } }",
            ],
            capture_output=True,
            text=True,
            errors="replace",
            timeout=30,
        )
        for line in (ps.stdout or "").splitlines():
            line = line.strip()
            if line.isdigit():
                return int(line)
    except Exception as exc:  # noqa: BLE001
        print("note: process lookup failed: %s" % exc, file=sys.stderr)

    # 2) Chrome's own port file (written when Chrome picks the port)
    for base in candidate_profiles():
        path = os.path.join(base, "DevToolsActivePort")
        if os.path.exists(path):
            with open(path, encoding="utf-8") as f:
                first = f.readline().strip()
            if first.isdigit():
                return int(first)
    raise SystemExit(
        "could not find the CDP port — is Akagi v3 running with its game "
        "browser open? (pass --port=N to override)"
    )


def candidate_profiles():
    out = []
    here = os.path.dirname(os.path.abspath(__file__))
    for base in (here, os.path.dirname(here), os.getcwd()):
        out.append(os.path.join(base, "chrome-profile"))
    return out


def http_json(url):
    with urllib.request.urlopen(url, timeout=10) as r:
        return json.load(r)


class CDP:
    def __init__(self, port):
        import websocket  # websocket-client

        self.port = port
        self.target = None
        for t in http_json(f"http://127.0.0.1:{port}/json/list"):
            if t.get("type") == "page" and "maj-soul" in t.get("url", ""):
                self.target = t
                break
        if self.target is None:
            for t in http_json(f"http://127.0.0.1:{port}/json/list"):
                if t.get("type") == "page":
                    self.target = t
                    break
        if self.target is None:
            raise SystemExit("no page target on port %d" % port)
        self.ws = websocket.create_connection(
            self.target["webSocketDebuggerUrl"], timeout=30, suppress_origin=True
        )
        self.id = 0

    def send(self, method, params=None, timeout=30):
        self.id += 1
        mid = self.id
        self.ws.send(json.dumps({"id": mid, "method": method, "params": params or {}}))
        deadline = time.time() + timeout
        while time.time() < deadline:
            self.ws.settimeout(max(0.5, deadline - time.time()))
            try:
                msg = json.loads(self.ws.recv())
            except Exception as exc:  # noqa: BLE001
                raise SystemExit(f"CDP recv failed: {exc}")
            if msg.get("id") == mid:
                if "error" in msg:
                    raise SystemExit(f"CDP {method} error: {msg['error']}")
                return msg.get("result", {})
        raise SystemExit(f"CDP {method} timeout")

    def evaluate(self, expr):
        r = self.send(
            "Runtime.evaluate",
            {"expression": expr, "returnByValue": True, "awaitPromise": True},
        )
        if r.get("exceptionDetails"):
            return {"__error__": json.dumps(r["exceptionDetails"])[:800]}
        return r.get("result", {}).get("value")

    def screenshot(self, path):
        r = self.send("Page.captureScreenshot", {"format": "png"})
        with open(path, "wb") as f:
            f.write(base64.b64decode(r["data"]))
        return path


RECT_EXPR = """(()=>{
  const cs=Array.from(document.getElementsByTagName('canvas')).map((c,i)=>{
    const r=c.getBoundingClientRect();
    return {i,x:r.x,y:r.y,w:r.width,h:r.height,cw:c.width,ch:c.height,
            attrW:c.getAttribute('width'),attrH:c.getAttribute('height'),
            style:(c.getAttribute('style')||'').slice(0,160)};
  });
  return {url:location.href,title:document.title,
          innerW:innerWidth,innerH:innerHeight,
          dpr:devicePixelRatio,screenW:screen.width,screenH:screen.height,
          bodyW:document.body?document.body.clientWidth:0,
          bodyH:document.body?document.body.clientHeight:0,
          canvases:cs,
          divs:document.getElementsByTagName('div').length};
})()"""


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--port=")]
    mode = args[0] if args else "rect"
    port = find_port()
    c = CDP(port)
    print("CDP port %d  page %s" % (port, c.target.get("url", "")[:70]), file=sys.stderr)

    if mode == "eval":
        print(json.dumps(c.evaluate(args[1]), ensure_ascii=False, indent=2))
    elif mode == "shot":
        print(c.screenshot(args[1]))
    elif mode == "rect":
        print(json.dumps(c.evaluate(RECT_EXPR), ensure_ascii=False, indent=2))
    elif mode == "click":
        nx = float(args[1])
        ny = float(args[2])
        rect = c.evaluate(RECT_EXPR)
        canvas = rect["canvases"][0] if rect.get("canvases") else None
        if not canvas:
            raise SystemExit("no canvas on the page")
        # 16:9 normalised against the canvas box
        px = canvas["x"] + nx / 16.0 * canvas["w"]
        py = canvas["y"] + ny / 9.0 * canvas["h"]
        print(
            json.dumps(
                {
                    "canvas": canvas,
                    "norm": [nx, ny],
                    "page_css": [round(px, 2), round(py, 2)],
                },
                ensure_ascii=False,
            )
        )
        if "--no-click" not in args:
            # hover → press → release, exactly like the module does
            c.send("Input.dispatchMouseEvent",
                   {"type": "mouseMoved", "x": px, "y": py, "button": "none"})
            time.sleep(0.4)
            c.send("Input.dispatchMouseEvent",
                   {"type": "mousePressed", "x": px, "y": py,
                    "button": "left", "clickCount": 1})
            time.sleep(0.3)
            c.send("Input.dispatchMouseEvent",
                   {"type": "mouseReleased", "x": px, "y": py,
                    "button": "left", "clickCount": 1})
            print("clicked")
        if "--shot" in args:
            i = args.index("--shot")
            time.sleep(1.2)
            print(c.screenshot(args[i + 1]))
    else:
        raise SystemExit(f"unknown mode {mode}")


if __name__ == "__main__":
    main()
