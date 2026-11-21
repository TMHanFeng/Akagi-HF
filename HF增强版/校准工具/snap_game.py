"""Capture the live Majsoul screen from the running AkagiMS window.

Read-only: it only brings the window to the front (PrintWindow returns a
black frame for background windows) and grabs its pixels. Nothing inside the
app is clicked, typed or otherwise modified.
"""

import ctypes
import ctypes.wintypes as wt
import os
import sys
import time

from PIL import Image

OUT = sys.argv[1] if len(sys.argv) > 1 else (
    r"C:\Users\13728\.minimax\sessions\mvs_d88f515d4b7d4ebcb612d3cdc6c1fec9"
    r"\workspace\captures"
)
os.makedirs(OUT, exist_ok=True)

user32 = ctypes.windll.user32
gdi32 = ctypes.windll.gdi32
user32.SetProcessDPIAware()
WNDENUMPROC = ctypes.WINFUNCTYPE(wt.BOOL, wt.HWND, wt.LPARAM)


def find_window(substr):
    hits = []

    def cb(hwnd, _):
        if not user32.IsWindowVisible(hwnd):
            return True
        n = user32.GetWindowTextLengthW(hwnd)
        if n:
            buf = ctypes.create_unicode_buffer(n + 1)
            user32.GetWindowTextW(hwnd, buf, n + 1)
            if substr.lower() in buf.value.lower():
                hits.append((hwnd, buf.value))
        return True

    user32.EnumWindows(WNDENUMPROC(cb), 0)
    return hits


class BITMAPINFOHEADER(ctypes.Structure):
    _fields_ = [
        ("biSize", wt.DWORD), ("biWidth", wt.LONG), ("biHeight", wt.LONG),
        ("biPlanes", wt.WORD), ("biBitCount", wt.WORD), ("biCompression", wt.DWORD),
        ("biSizeImage", wt.DWORD), ("biXPelsPerMeter", wt.LONG),
        ("biYPelsPerMeter", wt.LONG), ("biClrUsed", wt.DWORD), ("biClrImportant", wt.DWORD),
    ]


def capture(hwnd, path):
    rect = wt.RECT()
    user32.GetWindowRect(hwnd, ctypes.byref(rect))
    w, h = rect.right - rect.left, rect.bottom - rect.top
    print("  window rect:", w, h, flush=True)
    if w <= 0 or h <= 0:
        return None
    hdc = user32.GetWindowDC(hwnd)
    memdc = gdi32.CreateCompatibleDC(hdc)
    bmp = gdi32.CreateCompatibleBitmap(hdc, w, h)
    gdi32.SelectObject(memdc, bmp)
    ok = user32.PrintWindow(hwnd, memdc, 2)
    print("  PrintWindow:", ok, flush=True)
    bi = BITMAPINFOHEADER()
    bi.biSize = ctypes.sizeof(BITMAPINFOHEADER)
    bi.biWidth = w
    bi.biHeight = -h
    bi.biPlanes = 1
    bi.biBitCount = 32
    bi.biCompression = 0
    buf = ctypes.create_string_buffer(w * h * 4)
    got = gdi32.GetDIBits(memdc, bmp, 0, h, buf, ctypes.byref(bi), 0)
    gdi32.DeleteObject(bmp)
    gdi32.DeleteDC(memdc)
    user32.ReleaseDC(hwnd, hdc)
    if not got:
        return None
    img = Image.frombuffer("RGBA", (w, h), buf, "raw", "BGRA", 0, 1).convert("RGB")
    img.save(path)
    # A black frame means the window was not in the foreground.
    ex = img.convert("L").getextrema()
    print("  luminance range:", ex, flush=True)
    return (w, h, path, ex)


target = sys.argv[2] if len(sys.argv) > 2 else "AkagiMS"
targets = find_window(target)
print("windows found:", targets, flush=True)

for hwnd, title in targets:
    # PrintWindow yields black for a background window — bring it forward.
    user32.ShowWindow(hwnd, 9)  # SW_RESTORE
    user32.SetForegroundWindow(hwnd)
    user32.BringWindowToTop(hwnd)
    time.sleep(1.5)

    p = os.path.join(OUT, "shot_%d_%s.png" % (int(time.time()), title.replace(" ", "_")[:20]))
    try:
        res = capture(hwnd, p)
    except Exception as e:
        print("  error:", e, flush=True)
        continue
    print("  ->", res, flush=True)
