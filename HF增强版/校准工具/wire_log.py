import json
import sys
import datetime

PATH = r"C:\Users\13728\Desktop\akagims-1.3.2-windows-x64\logs\20261120-133120\inspector.jsonl"

lo = sys.argv[1] if len(sys.argv) > 1 else "2026-11-20T14:40:00"
hi = sys.argv[2] if len(sys.argv) > 2 else "2026-11-20T14:48:00"
lo_ts = int(datetime.datetime.fromisoformat(lo).timestamp() * 1000)
hi_ts = int(datetime.datetime.fromisoformat(hi).timestamp() * 1000)

rows = []
with open(PATH, "r", encoding="utf-8", errors="replace") as fh:
    for line in fh:
        if "heartbeat" in line and "confirmNewRound" not in line and "startUnifiedMatch" not in line:
            continue
        try:
            o = json.loads(line)
        except Exception:
            continue
        ts = o.get("ts_ms", 0)
        if not (lo_ts <= ts <= hi_ts):
            continue
        p = o.get("parsed") or {}
        m = p.get("method") or (p.get("args") or {}).get("method") or ""
        if not m:
            continue
        rows.append((ts, o.get("direction"), m, o.get("size"), o.get("flow_id")))

rows.sort()
for ts, d, m, size, flow in rows:
    t = datetime.datetime.fromtimestamp(ts / 1000).strftime("%H:%M:%S.%f")[:-3]
    print("%s  %-5s %-46s size=%-5s %s" % (t, d, m, size, flow))
print("\nframes shown:", len(rows))
