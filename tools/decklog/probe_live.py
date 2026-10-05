import json
import sys
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from scrape_decklog import headers, BASE, VIEW_API, fetch_deck

CODE = "1TG900"
url = VIEW_API.format(code=CODE)
print("GET", url)
req = urllib.request.Request(url, headers=headers(f"{BASE}/view/{CODE}"))
try:
    with urllib.request.urlopen(req, timeout=30) as r:
        status = r.status
        ctype = r.headers.get("Content-Type")
        body = r.read().decode("utf-8", "replace")
    print(f"status={status} content-type={ctype} len={len(body)}")
    print("body head:", body[:400])
except Exception as exc:
    print("ERROR:", type(exc).__name__, exc)
    if hasattr(exc, "read"):
        print("error body:", exc.read()[:400])

print()
print("via fetch_deck():")
try:
    d = fetch_deck(CODE)
    print("  keys:", ", ".join(list(d)[:12]))
    print("  list entries:", len(d.get("list") or []))
    print("  sub_list entries:", len(d.get("sub_list") or []))
except Exception as exc:
    print("  raised:", type(exc).__name__, exc)