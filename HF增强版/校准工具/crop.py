"""Crop a region of a capture and upscale it, so a control can be located
by eye at full resolution."""

import sys
from PIL import Image

src = sys.argv[1]
dst = sys.argv[2]
x0, y0, x1, y1 = (int(v) for v in sys.argv[3:7])
scale = int(sys.argv[7]) if len(sys.argv) > 7 else 3

im = Image.open(src).convert("RGB").crop((x0, y0, x1, y1))
im = im.resize((im.width * scale, im.height * scale), Image.NEAREST)
im.save(dst)
print("wrote", dst, im.size, "from", (x0, y0, x1, y1))
