"""Time the card fetch so throughput is a measured number, not a guess."""

import importlib.util
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("sl", HERE / "scrape_decklog.py")
sl = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sl)

codes = [l.strip() for l in (HERE / "codes.txt").read_text(encoding="utf-8").splitlines() if l.strip()]
have = {p.stem for p in (HERE / "raw").glob("*.json")}
missing = [c for c in codes if c not in have]
print(f"codes total      : {len(codes):,}")
print(f"already fetched  : {len(codes) - len(missing):,}")
print(f"still to fetch   : {len(missing):,}")
print()

n = int(sys.argv[1]) if len(sys.argv) > 1 else 200
workers = int(sys.argv[2]) if len(sys.argv) > 2 else 16
if len(missing) >= n:
    sample = missing[:n]
else:
    # not enough outstanding work to time, so re-fetch known decks purely to measure
    sample = (missing + codes[: n - len(missing)])[:n]
print(f"timing {len(sample)} decks at {workers} workers...")


def pull(code):
    t0 = time.time()
    try:
        d = sl.fetch_deck(code)
        return code, len(d.get("list") or []), time.time() - t0, None
    except Exception as exc:
        return code, 0, time.time() - t0, exc


t0 = time.time()
ok = fail = 0
lat = []
with ThreadPoolExecutor(max_workers=workers) as pool:
    for code, ncards, dt, exc in pool.map(pull, sample):
        if exc:
            fail += 1
        else:
            ok += 1
            lat.append(dt)
elapsed = time.time() - t0

rate = ok / elapsed if elapsed else 0
print(f"\nfetched {ok} ok, {fail} failed in {elapsed:.1f}s")
print(f"throughput       : {rate:.1f} decks/sec  ({workers} workers)")
if lat:
    lat.sort()
    print(f"per-request      : median {lat[len(lat) // 2] * 1000:.0f}ms  "
          f"p90 {lat[int(len(lat) * 0.9)] * 1000:.0f}ms  max {lat[-1] * 1000:.0f}ms")
if missing and rate:
    print(f"\nremaining {len(missing):,} decks at that rate: "
          f"{len(missing) / rate / 60:.1f} min")
if rate:
    print(f"full corpus {len(codes):,} decks from scratch: {len(codes) / rate / 60:.1f} min")