"""Re-check expansions using scrape_expansion's own parser.

The earlier probe parsed the list response with hand-rolled key handling and silently
skipped expansions it could not read. Importing the scraper guarantees the same parsing
the working PR/BP07 runs used.
"""

import importlib.util
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
CARDSDIR = HERE.parent.parent / "cards"

spec = importlib.util.spec_from_file_location("sx", CARDSDIR / "scrape_expansion.py")
sx = importlib.util.module_from_spec(spec)
sys.path.insert(0, str(CARDSDIR))
spec.loader.exec_module(sx)

WANT = "bp4-999"
exps = ["BP04", "PB04", "PBLL02", "BP07", "PBLL", "PR"]
exps += [f"BP{i:02d}" for i in range(1, 13)]
exps += [f"PBLL{i:02d}" for i in range(1, 6)]
exps += [f"PB{i:02d}" for i in range(1, 8)]

seen = set()
for exp in exps:
    if exp in seen:
        continue
    seen.add(exp)
    try:
        listing = sx.fetch_card_numbers(exp)
    except Exception as exc:
        print(f"  {exp:8} ERR {str(exc)[:60]}")
        continue
    if not listing:
        continue
    hits = [c for c in listing if WANT in c]
    print(f"  {exp:8} {len(listing):4} cards   999-series: "
          f"{[c for c in listing if '999' in c] or '-'}")
    for h in hits:
        print(f"      FOUND {h}")
    if hits:
        sys.exit(0)

print("\nno expansion listing contains it")