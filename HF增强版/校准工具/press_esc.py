"""Send a real ESC key to the AkagiMS game window.

Some of the client's panels close on ESC, which is a cleaner way out than
hitting a thin cross-stroke with the mouse.
"""

import ctypes
import time

user32 = ctypes.windll.user32
user32.SetProcessDPIAware()
P = ctypes.WINFUNCTYPE(ctypes.c_bool, ctypes.c_void_p, ctypes.c_long)

hits = []


def cb(h, _):
    n = user32.GetWindowTextLengthW(h)
    if n:
        b = ctypes.create_unicode_buffer(n + 1)
        user32.GetWindowTextW(h, b, n + 1)
        if b.value == "AkagiMS":
            hits.append(h)
    return True


user32.EnumWindows(P(cb), 0)
if not hits:
    raise SystemExit("AkagiMS window not found")

h = hits[0]
user32.SetForegroundWindow(h)
time.sleep(0.4)
user32.keybd_event(0x1B, 0, 0, 0)   # ESC down
time.sleep(0.08)
user32.keybd_event(0x1B, 0, 2, 0)   # ESC up
time.sleep(0.4)
print("ESC sent")
