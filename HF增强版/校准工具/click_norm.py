"""Click a 16:9-normalised coordinate inside the AkagiMS game canvas.

The canvas rect in the AkagiMS window was solved once from the live
client (2026-11-20): the canvas is 1663.5 x 936.3 physical pixels at
device scale 1.5, anchored at (4.4, 216.5) in the window. The window
itself sits at (0, 0) on this machine.

The window size is checked on every call: if it differs from the size the
anchor was measured at, the click is refused rather than aimed at the
wrong pixel.

Usage:
    python click_norm.py <x_norm> <y_norm> [--label NAME]
"""

import ctypes
import ctypes.wintypes as wt
import sys
import time

user32 = ctypes.windll.user32
user32.SetProcessDPIAware()
WNDENUMPROC = ctypes.WINFUNCTYPE(wt.BOOL, wt.HWND, wt.LPARAM)

# Measured anchor (window 2266 x 1383).
ANCHOR_WINDOW = (2266, 1383)
CANVAS_ORIGIN = (4.4, 216.5)
CANVAS_SIZE = (1663.5, 936.3)


def find_window_exact(title):
    hits = []

    def cb(hwnd, _):
        if not user32.IsWindowVisible(hwnd):
            return True
        n = user32.GetWindowTextLengthW(hwnd)
        if n:
            buf = ctypes.create_unicode_buffer(n + 1)
            user32.GetWindowTextW(hwnd, buf, n + 1)
            if buf.value == title:
                hits.append(hwnd)
        return True

    user32.EnumWindows(WNDENUMPROC(cb), 0)
    return hits


def main():
    x_norm = float(sys.argv[1])
    y_norm = float(sys.argv[2])
    label = ""
    if "--label" in sys.argv:
        label = sys.argv[sys.argv.index("--label") + 1]

    hits = find_window_exact("AkagiMS")
    if not hits:
        print("AkagiMS window not found")
        sys.exit(2)
    hwnd = hits[0]
    r = wt.RECT()
    user32.GetWindowRect(hwnd, ctypes.byref(r))
    w, h = r.right - r.left, r.bottom - r.top
    print("window: %dx%d at (%d,%d)" % (w, h, r.left, r.top))
    if (w, h) != ANCHOR_WINDOW:
        print("!! window size differs from the measured anchor %dx%d — refusing"
              % ANCHOR_WINDOW)
        sys.exit(3)

    px = CANVAS_ORIGIN[0] + x_norm / 16.0 * CANVAS_SIZE[0]
    py = CANVAS_ORIGIN[1] + y_norm / 9.0 * CANVAS_SIZE[1]
    sx, sy = int(r.left + px), int(r.top + py)
    print("norm (%.2f, %.2f) -> window (%.0f, %.0f) -> screen (%d, %d) %s"
          % (x_norm, y_norm, px, py, sx, sy, label))

    user32.SetForegroundWindow(hwnd)
    time.sleep(0.4)
    user32.SetCursorPos(sx, sy)
    time.sleep(0.35)
    user32.mouse_event(0x0002, 0, 0, 0, 0)   # LEFTDOWN
    time.sleep(0.12)
    user32.mouse_event(0x0004, 0, 0, 0, 0)   # LEFTUP
    time.sleep(0.35)
    print("clicked")


if __name__ == "__main__":
    main()
