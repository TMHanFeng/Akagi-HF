"""Locate a muted light-grey control (the panels' close cross) in a capture.

Unlike find_close.py this uses a luminance threshold with a configurable
floor and reports the brightest pixels, so a grey cross on a navy panel can
be found without also catching bright background signage.

Usage:
    python find_close2.py <image> <x0> <x1> <y0> <y1> [min_lum]
"""

import sys
from PIL import Image

path = sys.argv[1]
x0, x1, y0, y1 = (int(v) for v in sys.argv[2:6])
min_lum = int(sys.argv[6]) if len(sys.argv) > 6 else 90

im = Image.open(path).convert("RGB")
px = im.load()

hits = []
best = 0
for y in range(y0, y1):
    for x in range(x0, x1):
        r, g, b = px[x, y]
        lum = 0.299 * r + 0.587 * g + 0.114 * b
        if lum > best:
            best = lum
        if lum >= min_lum:
            hits.append((x, y, lum))

if not hits:
    print("no pixels above luminance %d (max in region was %.0f)" % (min_lum, best))
    raise SystemExit(1)

xs = [p[0] for p in hits]
ys = [p[1] for p in hits]
cx = (min(xs) + max(xs)) / 2
cy = (min(ys) + max(ys)) / 2
print("pixels: %d  bbox (%d,%d)-(%d,%d)  max_lum %.0f"
      % (len(hits), min(xs), min(ys), max(xs), max(ys), best))
print("centre px (%.1f, %.1f)" % (cx, cy))
print("norm (%.2f, %.2f)" % ((cx - 4.4) / 1663.5 * 16, (cy - 216.5) / 936.3 * 9))
