#!/usr/bin/env python3
"""Gap report: find abilities whose parse looks thin relative to their text.

Two views:
  1. under-parsed  — long triggerless_text but few structured fields
  2. confusion     — near-duplicate texts (normalized) that parse to
                     different top-level actions

Usage: python gap_report.py [top_n]
"""

import json
import re
import sys
import io
from pathlib import Path
from difflib import SequenceMatcher

try:
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
except (AttributeError, io.UnsupportedOperation):
    pass

ROOT = Path(__file__).parent.parent
TOP_N = int(sys.argv[1]) if len(sys.argv) > 1 else 12


def strip_noise(t):
    t = re.sub(r"\{\{[^}]+\}\}", "", t)
    t = re.sub(r"「[^」]*」", "Q", t)
    t = re.sub(r"【[^】]*】", "", t)
    return t


def count_fields(node):
    if isinstance(node, dict):
        n = sum(
            1
            for k, v in node.items()
            if k not in ("text", "type", "action") and v not in (None, [], {}, False)
        )
        return n + sum(count_fields(v) for v in node.values())
    if isinstance(node, list):
        return sum(count_fields(i) for i in node)
    return 0


def top_action(eff):
    a = eff.get("action")
    if not a and eff.get("actions"):
        first = eff["actions"][0]
        if isinstance(first, dict):
            a = first.get("action")
    return a or "?"


def norm_text(t):
    t = re.sub(r"\{\{[^}]+\}\}", "I", t)
    t = re.sub(r"\d+", "N", t)
    t = re.sub(r"[「『][^」』]*[」』]", "Q", t)
    return re.sub(r"\s+", "", t)


def main():
    data = json.load(open(ROOT / "abilities.json", encoding="utf-8"))
    ua = [e for e in data["unique_abilities"] if not e.get("is_null")]

    print("=== 1. MOST UNDER-PARSED (fields per char, text >= 40 chars) ===")
    scored = []
    for e in ua:
        t = strip_noise(e.get("triggerless_text", "") or e.get("full_text", ""))
        f = count_fields(e.get("effect") or {})
        L = len(t)
        if L >= 40:
            scored.append((f / max(L, 1), L, f, e))
    scored.sort(key=lambda x: x[0])
    for r, L, f, e in scored[:TOP_N]:
        txt = re.sub(r"\s+", "", e["triggerless_text"])[:70]
        card = e["cards"][0][:42] if e["cards"] else "?"
        print(f"{r:.3f} len={L:3} fields={f:2} | {card} | {txt}")

    print()
    print("=== 2. CONFUSION PAIRS (normalized text sim >= 0.85, different action) ===")
    items = []
    for i, e in enumerate(ua):
        nt = norm_text(e.get("triggerless_text", "") or "")
        act = top_action(e.get("effect") or {})
        items.append((i, nt, act, e))

    seen = set()
    pairs = 0
    for a in range(len(items)):
        ia, ta, aa, ea = items[a]
        for b in range(a + 1, len(items)):
            ib, tb, ab, eb = items[b]
            key = (min(ia, ib), max(ia, ib))
            if key in seen:
                continue
            if abs(len(ta) - len(tb)) > 12:
                continue
            sim = SequenceMatcher(None, ta, tb).ratio()
            if sim >= 0.85 and aa != ab:
                seen.add(key)
                pairs += 1
                if pairs <= TOP_N:
                    ca = ea["cards"][0][:30] if ea["cards"] else "?"
                    cb = eb["cards"][0][:30] if eb["cards"] else "?"
                    print(f"\n[{aa} vs {ab}] sim={sim:.2f}")
                    print(f"  A: {ca} | {re.sub(chr(10),' ',ea['triggerless_text'])[:80]}")
                    print(f"  B: {cb} | {re.sub(chr(10),' ',eb['triggerless_text'])[:80]}")
    print(f"\ntotal confusion pairs: {pairs}")


if __name__ == "__main__":
    main()
