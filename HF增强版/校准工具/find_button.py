"""Analyse a captured AkagiMS frame: locate the golden action buttons and
derive the game canvas rect, so a normalised (16:9) coordinate can be derived
from a measured pixel position.

The Majsoul UI draws its primary action button as a rounded golden plate
(high R, mid G, low B). Finding that plate's centre gives us the button's
pixel position; the canvas rect then turns it into the 16:9 normalised
coordinate the click path actually uses.
"""

import sys

from PIL import Image

path = sys.argv[1]
img = Image.open(path).convert("RGB")
W, H = img.size
px = img.load()
print(f"image {path}  {W}x{H}")

# --- golden button pixels -------------------------------------------------
pts = []
for y in range(0, H, 2):
    for x in range(0, W, 2):
        r, g, b = px[x, y]
        # gold: bright red, clearly less green, clearly least blue
        if r > 185 and 110 < g < 215 and b < 120 and (r - b) > 90:
            pts.append((x, y))
print("golden pixels:", len(pts))

if not pts:
    raise SystemExit("no golden button found")

# Cluster the golden pixels into connected regions (grid buckets).
CELL = 24
buckets = {}
for x, y in pts:
    buckets.setdefault((x // CELL, y // CELL), []).append((x, y))

regions = []
used = set()
for key in sorted(buckets, key=lambda k: -len(buckets[k])):
    if key in used:
        continue
    # flood fill over neighbouring buckets
    stack = [key]
    cluster = []
    used.add(key)
    while stack:
        k = stack.pop()
        cluster.extend(buckets[k])
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                nk = (k[0] + dx, k[1] + dy)
                if nk in buckets and nk not in used:
                    used.add(nk)
                    stack.append(nk)
    if len(cluster) >= 60:
        xs = [p[0] for p in cluster]
        ys = [p[1] for p in cluster]
        regions.append(
            {
                "n": len(cluster),
                "x0": min(xs), "x1": max(xs),
                "y0": min(ys), "y1": max(ys),
                "cx": (min(xs) + max(xs)) / 2,
                "cy": (min(ys) + max(ys)) / 2,
            }
        )

regions.sort(key=lambda r: -r["n"])
print("\ngolden regions:")
for r in regions[:8]:
    print(
        "  n=%-7d bbox=(%d,%d)-(%d,%d)  size=%dx%d  centre=(%.1f, %.1f)"
        % (
            r["n"], r["x0"], r["y0"], r["x1"], r["y1"],
            r["x1"] - r["x0"], r["y1"] - r["y0"], r["cx"], r["cy"],
        )
    )

# --- the game canvas: the dark WebGL area, left of the control panel ------
# Find the vertical separator: the game view is bounded by a dark region and
# the panel starts with a light background.
col_dark = []
for x in range(0, W, 2):
    dark = 0
    for y in range(40, H - 60, 6):
        r, g, b = px[x, y]
        if r + g + b < 210:
            dark += 1
    col_dark.append((x, dark))

# The canvas is the widest contiguous run of columns that look like game
# pixels (varied colours) rather than the flat panel background.
varied = []
for x in range(0, W, 2):
    vals = [px[x, y] for y in range(120, min(H - 120, 900), 24)]
    if len(vals) < 4:
        continue
    uniq = len({v for v in vals})
    varied.append((x, uniq))

runs = []
cur = None
for x, u in varied:
    if u >= 6:
        if cur is None:
            cur = [x, x]
        else:
            cur[1] = x
    else:
        if cur and cur[1] - cur[0] > 200:
            runs.append(tuple(cur))
        cur = None
if cur and cur[1] - cur[0] > 200:
    runs.append(tuple(cur))

print("\nwide 'varied content' column runs (likely the game canvas):")
for a, b in runs:
    print(f"  x {a}..{b}  width={b - a}")
