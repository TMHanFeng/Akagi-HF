"""Measure the on-screen position of a UI button in AkagiMS's window and
convert it to the 16:9 normalised coordinate the click path uses.

The conversion chain (all of it measured, nothing assumed):

  * AkagiMS's own log records every autoplay press as
    `x_norm=... css_x=... dpr=...`. Two presses with different x_norm give
    the canvas' CSS scale and offset:  css = scale * x_norm + offset.
    The same for y. `scale * 16` is the canvas' CSS width.
  * dpr comes from the same log line.
  * The captured frame is in physical pixels, so
    window_px = canvas_origin_px + dpr * (css - canvas_css_offset).
    The canvas origin is solved from the one button whose normalised
    position is already known for certain (the result screen's confirm
    control, at (14.59, 8.37)).

Usage:
    python measure_btn.py <capture.png> [--anchor-x 14.59] [--anchor-y 8.37]
"""

import ctypes
import os
import re
import sys

from PIL import Image

LOG = r"C:\Users\13728\Desktop\akagims-1.3.2-windows-x64\logs\20261120-133120\all.log"

PRES_RE = re.compile(
    r"cdp press x_norm=([-\d.]+) y_norm=([-\d.]+) css_x=([-\d.]+) css_y=([-\d.]+) dpr=Some\(([\d.]+)\)"
)


def load_scale():
    """canvas css scale/offset for x and y, plus the dpr."""
    pts_x = []
    pts_y = []
    dpr = None
    with open(LOG, "r", encoding="utf-8", errors="replace") as fh:
        for line in fh:
            m = PRES_RE.search(line)
            if not m:
                continue
            xn, yn, cx, cy, dp = (float(m.group(i)) for i in range(1, 6))
            pts_x.append((xn, cx))
            pts_y.append((yn, cy))
            dpr = dp
    if len(pts_x) < 2:
        raise SystemExit("not enough presses in the log")
    (x1, c1), (x2, c2) = pts_x[0], pts_x[-1]
    sx = (c2 - c1) / (x2 - x1)
    ox = c1 - sx * x1
    (y1, d1), (y2, d2) = pts_y[0], pts_y[-1]
    sy = (d2 - d1) / (y2 - y1)
    oy = d1 - sy * y1
    return sx, ox, sy, oy, dpr, len(pts_x)


def fit(ax, ay, bx, by, sx, ox, sy, oy):
    """Least-squares line fit over every logged press."""
    xs = []
    for line in open(LOG, "r", encoding="utf-8", errors="replace"):
        m = PRES_RE.search(line)
        if m:
            xs.append((float(m.group(1)), float(m.group(2)), float(m.group(3)), float(m.group(4))))
    n = len(xs)
    mx = sum(p[0] for p in xs) / n
    my = sum(p[1] for p in xs) / n
    mcx = sum(p[2] for p in xs) / n
    mcy = sum(p[3] for p in xs) / n
    sxx = sum((p[0] - mx) ** 2 for p in xs)
    syy = sum((p[1] - my) ** 2 for p in xs)
    scx = sum((p[0] - mx) * (p[2] - mcx) for p in xs)
    scy = sum((p[1] - my) * (p[3] - mcy) for p in xs)
    sx = scx / sxx
    ox = mcx - sx * mx
    sy = scy / syy
    oy = mcy - sy * my
    return sx, ox, sy, oy


def main():
    path = sys.argv[1]
    ax, ay = 14.59, 8.37
    args = sys.argv[2:]
    while args:
        if args[0] == "--anchor-x":
            ax = float(args[1]); args = args[2:]
        elif args[0] == "--anchor-y":
            ay = float(args[1]); args = args[2:]
        else:
            args = args[1:]

    sx, ox, sy, oy = fit(ax, ay, 0, 0, 0, 0, 0, 0)
    _, _, _, _, dpr, n = load_scale()
    w_css = sx * 16
    h_css = sy * 9
    print("presses in log: %d   dpr=%s" % (n, dpr))
    print("canvas css: x = %.6f * x_norm + %.4f  -> width %.3f css" % (sx, ox, w_css))
    print("canvas css: y = %.6f * y_norm + %.4f  -> height %.3f css" % (sy, oy, h_css))

    img = Image.open(path).convert("RGB")
    W, H = img.size
    px = img.load()

    # --- anchor: the golden confirm plate ---------------------------------
    def golden(x, y):
        r, g, b = px[x, y]
        return r > 185 and 110 < g < 215 and b < 120 and (r - b) > 90

    pts = [(x, y) for y in range(0, H, 2) for x in range(0, W, 2) if golden(x, y)]
    CELL = 24
    buckets = {}
    for x, y in pts:
        buckets.setdefault((x // CELL, y // CELL), []).append((x, y))
    used = set()
    regions = []
    for key in sorted(buckets, key=lambda k: -len(buckets[k])):
        if key in used:
            continue
        stack = [key]; cluster = []; used.add(key)
        while stack:
            k = stack.pop()
            cluster.extend(buckets[k])
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    nk = (k[0] + dx, k[1] + dy)
                    if nk in buckets and nk not in used:
                        used.add(nk); stack.append(nk)
        if len(cluster) >= 60:
            xs = [p[0] for p in cluster]; ys = [p[1] for p in cluster]
            w = max(xs) - min(xs); h = max(ys) - min(ys)
            if w * h >= 0.5 * len(cluster) * 4:  # fill-ratio filter
                regions.append((min(xs), min(ys), max(xs), max(ys), len(cluster)))
    regions.sort(key=lambda r: -r[4])
    print("\ngolden regions (fill-filtered):")
    for r in regions[:6]:
        print("   bbox=(%d,%d)-(%d,%d) n=%d  centre=(%.1f, %.1f)"
              % (r[0], r[1], r[2], r[3], r[4], (r[0] + r[2]) / 2, (r[1] + r[3]) / 2))
    if not regions:
        raise SystemExit("no golden plate")
    ab = regions[0]
    acx = (ab[0] + ab[2]) / 2
    acy = (ab[1] + ab[3]) / 2

    # anchor css coords
    acss_x = ax * sx + ox
    acss_y = ay * sy + oy
    # physical canvas origin
    o_px = acx - dpr * acss_x
    o_py = acy - dpr * acss_y
    print("\nanchor plate centre px = (%.1f, %.1f)  -> canvas origin px = (%.1f, %.1f)"
          % (acx, acy, o_px, o_py))
    print("canvas physical size  = %.1f x %.1f" % (dpr * w_css, dpr * h_css))

    def to_norm(px_, py_):
        cssx = (px_ - o_px) / dpr
        cssy = (py_ - o_py) / dpr
        return (cssx - ox) / sx, (cssy - oy) / sy

    print("anchor round-trip norm = (%.3f, %.3f)  (expected %.2f, %.2f)"
          % (*to_norm(acx, acy), ax, ay))

    # --- blue plates: the "play again" button ------------------------------
    blues = []
    for y in range(0, H, 2):
        for x in range(0, W, 2):
            r, g, b = px[x, y]
            if b > 110 and b > r + 45 and b > g + 25 and r < 140:
                blues.append((x, y))
    buckets = {}
    for x, y in blues:
        buckets.setdefault((x // CELL, y // CELL), []).append((x, y))
    used = set()
    regions = []
    for key in sorted(buckets, key=lambda k: -len(buckets[k])):
        if key in used:
            continue
        stack = [key]; cluster = []; used.add(key)
        while stack:
            k = stack.pop()
            cluster.extend(buckets[k])
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    nk = (k[0] + dx, k[1] + dy)
                    if nk in buckets and nk not in used:
                        used.add(nk); stack.append(nk)
        if len(cluster) >= 120:
            xs = [p[0] for p in cluster]; ys = [p[1] for p in cluster]
            w = max(xs) - min(xs); h = max(ys) - min(ys)
            fill = len(cluster) * 4 / float(max(1, w * h))
            regions.append((min(xs), min(ys), max(xs), max(ys), len(cluster), fill))
    regions.sort(key=lambda r: -r[4])
    print("\nblue regions (fill ratio shown):")
    for r in regions[:10]:
        cx = (r[0] + r[2]) / 2
        cy = (r[1] + r[3]) / 2
        nx, ny = to_norm(cx, cy)
        print("   bbox=(%d,%d)-(%d,%d) size=%dx%d n=%d fill=%.2f centre=(%.1f,%.1f) norm=(%.3f, %.3f)"
              % (r[0], r[1], r[2], r[3], r[2] - r[0], r[3] - r[1], r[4], r[5], cx, cy, nx, ny))


if __name__ == "__main__":
    main()
