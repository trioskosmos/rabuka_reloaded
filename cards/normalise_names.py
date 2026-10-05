"""Backfill `name_key` on every card in cards.json.

`cards/scrape_expansion.py` now writes `name_key` for every card it scrapes
(beside the existing `product_name` normalisation, which already solved the same
class of problem for product strings). Cards scraped before that change do not
have one, so this fills them in.

The key is the canonical form used for EQUALITY ONLY. `name` stays the display
string with its spacing intact — 'trial vol.1' must keep its space. See
`name_key` in scrape_expansion.py for why.

Idempotent: re-running recomputes every key and reports 0 changes once the data
is already canonical. Safe to run after any scrape.

Usage:
    python cards/normalise_names.py            # rewrite cards.json
    python cards/normalise_names.py --check    # report only, change nothing
"""

import json
import os
import sys
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from scrape_expansion import name_key  # single source of truth

CARDS = Path(__file__).resolve().parent / "cards.json"


def main() -> int:
    # Same reason as _runner.py: a report is the whole point of this script, and
    # cp932/cp936 consoles raise UnicodeEncodeError on an em-dash or a Japanese
    # card name, turning a useful report into a traceback.
    for stream in (sys.stdout, sys.stderr):
        reconfigure = getattr(stream, "reconfigure", None)
        if reconfigure is not None:
            try:
                reconfigure(errors="replace")
            except (ValueError, OSError):
                pass

    check_only = "--check" in sys.argv[1:]

    cards = json.loads(CARDS.read_text(encoding="utf-8"))
    changed = []
    for card_no, entry in cards.items():
        want = name_key(entry.get("name", ""))
        if entry.get("name_key") != want:
            changed.append(card_no)
            if not check_only:
                entry["name_key"] = want

    print(f"cards: {len(cards)}")
    print(f"name_key missing or stale: {len(changed)}")

    # Report what the normalisation actually does, so the change is visible
    # rather than assumed. Counted against the DISPLAY name, not against a
    # possibly-absent name_key, so the number is true before the first run.
    display = [e.get("name", "") for e in cards.values()]
    whitespace = sum(1 for n in display if any(c.isspace() for c in n))
    fullwidth = sum(1 for n in display if "＆" in n)
    differs = sum(
        1 for e in cards.values() if name_key(e.get("name", "")) != e.get("name", "")
    )
    print(f"names containing whitespace:  {whitespace}")
    print(f"names containing full-width ＆: {fullwidth}")
    print(f"names whose key differs from the display name: {differs}")

    if check_only:
        if changed:
            print("\nSTALE - run: python cards/normalise_names.py")
            return 1
        print("\nall name_key values are canonical")
        return 0

    if changed:
        # Byte-for-byte the shape scrape_expansion.py writes (sorted keys,
        # ensure_ascii=False, indent=2, no trailing newline), so the diff from
        # this script is exactly the added keys and nothing else.
        CARDS.write_text(
            json.dumps(dict(sorted(cards.items())), ensure_ascii=False, indent=2),
            encoding="utf-8",
        )
        print(f"\nwrote {CARDS} ({len(changed)} keys filled)")
    else:
        print("\nnothing to do")
    return 0


if __name__ == "__main__":
    sys.exit(main())