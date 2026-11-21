"""Extract the route-change timeline from AkagiMS all.log.

Prints every `.lq.Route.requestRouteChange` REQUEST with its timestamp and
route_id, plus the UI phase transitions that bracket it, so we can tell
whether `route_id == "route-4"` really fires when the client returns to the
lobby after a game.
"""

import io
import re
import sys

LOG = (
    r"C:\Users\13728\Desktop\akagims-1.3.2-windows-x64\logs"
    r"\20261120-133120\all.log"
)

TS = re.compile(r"^(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3})")
ROUTE = re.compile(r"requestRouteChange (\{.*?\})")
PHASE = re.compile(r"UI phase phase=(\w+)")

interesting = {"route-2", "route-3", "route-4", "route-5", "route-6"}

with io.open(LOG, encoding="utf-8", errors="replace") as fh:
    for line in fh:
        m = TS.match(line)
        if not m:
            continue
        ts = m.group(1)
        body = line[m.end():].rstrip()

        rm = ROUTE.search(body)
        if rm:
            print(f"{ts}  ROUTE  {rm.group(1)}")
            continue

        pm = PHASE.search(body)
        if pm:
            print(f"{ts}  PHASE  {pm.group(1)}")
            continue

        for key in (
            "startUnifiedMatch",
            "NotifyMatchGameStart",
            "NotifyGameEndResult",
            "confirmNewRound",
            "prepareLogin",
            "fetchAccountInfo",
            "game ended",
        ):
            if key in body:
                print(f"{ts}  EVENT  {key}")
                break
