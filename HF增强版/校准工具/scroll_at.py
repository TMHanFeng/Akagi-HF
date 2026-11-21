"""Scroll inside the AkagiMS game window with the real mouse wheel.

WebView2 only scrolls on real wheel events, so this moves the cursor to a
point and sends MOUSEEVENTF_WHEEL there — the same thing a hand does.

Usage:
    python scroll_at.py <x_norm> <y_norm> <notches>   # positive = up
"""

import ctypes
import sys
import time

user32 = ctypes.windll.user32
user32.SetProcessDPIAware()
WNDENUMPROC = ctypes.WINFUNCTYPE(ctypes.c_bool, ctypes.c_void_p, ctypes.c_long)

ANCHOR_WINDOW = (2266, 1383)
CANVAS_ORIGIN = (4.4, 216.5)
CANVAS_SIZE = (1663.5, 936.3)

WHEEL_DELTA = 120


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
    notches = int(sys.argv[3])

    hits = find_window_exact("AkagiMS")
    if not hits:
        print("AkagiMS window not found")
        sys.exit(2)
    hwnd = hits[0]
    user32.SetForegroundWindow(hwnd)
    time.sleep(0.3)

    px = CANVAS_ORIGIN[0] + x_norm / 16.0 * CANVAS_SIZE[0]
    py = CANVAS_ORIGIN[1] + y_norm / 9.0 * CANVAS_SIZE[1]
    user32.SetCursorPos(int(px), int(py))
    time.sleep(0.3)
    for _ in range(abs(notches)):
        user32.mouse_event(0x0800, 0, 0, WHEEL_DELTA if notches > 0 else -WHEEL_DELTA, 0)
        time.sleep(0.12)
    time.sleep(0.4)
    print("scrolled %d notch(es) at (%.0f, %.0f)" % (notches, px, py))


if __name__ == "__main__":
    main()
