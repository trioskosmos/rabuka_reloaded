#!/usr/bin/env python3
"""Describe-fidelity report (audit F3).

Compares engine-rendered descriptions (test_output/describe_dump.json, from
`cargo run --bin describe_dump`) against the original Japanese ability text.
Ranks abilities whose parsed structure least matches their text, so parser
gaps surface as report entries instead of silent divergence.

Non-gating diagnostic. Run:
  1. cd engine && cargo run --bin describe_dump
  2. python cards/describe_fidelity_report.py [--top N]
"""

import argparse
import json
import re
import sys
from pathlib import Path

ICON_RE = re.compile(r"\{\{[^|]+\|([^}]+)\}\}")
TRIGGER_PREFIX_RE = re.compile(r"^[^：]{0,40}?[時配]}}")


def normalize(text: str) -> str:
    """Strip icons to their labels, drop trigger prefixes/parens, canonicalize."""
    if not text:
        return ""
    t = ICON_RE.sub(r"\1", text)
    # Drop parenthetical rule reminders
    t = re.sub(r"（[^）]*）", "", t)
    t = re.sub(r"\([^)]*\)", "", t)
    # Canonicalize digits and punctuation for loose comparison
    t = re.sub(r"[、。\s]", "", t)
    return t


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--dump", default="test_output/describe_dump.json")
    ap.add_argument("--out", default="test_output/describe_fidelity_report.md")
    ap.add_argument("--top", type=int, default=40, help="report the N worst matches")
    args = ap.parse_args()

    dump_path = Path(args.dump)
    if not dump_path.exists():
        print(f"ERROR: {dump_path} missing — run `cargo run --bin describe_dump` first.")
        return 1
    entries = json.loads(dump_path.read_text(encoding="utf-8"))

    rows = []
    for e in entries:
        src = normalize(e["full_text"])
        ja = normalize(e["describe_ja"])
        en = e["describe_en"]
        # Coverage heuristics:
        # - ja_overlap: how much of the source's distinctive characters appear
        #   in the JA description (crude but monotone in fidelity).
        # - identical: description fell back to raw text (parity test should
        #   catch these, but keep the signal here too).
        identical = ja == src or en == src
        src_chars = set(src) - set("！?？+－-")
        ja_chars = set(ja)
        overlap = len(src_chars & ja_chars) / max(1, len(src_chars))
        rows.append(
            {
                "index": e["index"],
                "full_text": e["full_text"],
                "describe_en": en,
                "describe_ja": ja,
                "identical": identical,
                "overlap": round(overlap, 3),
            }
        )

    rows.sort(key=lambda r: r["overlap"])
    worst = [r for r in rows if not r["identical"]][: args.top]

    lines = [
        "# Describe fidelity report",
        "",
        f"Abilities dumped: {len(rows)}; raw-text fallbacks: {sum(r['identical'] for r in rows)}",
        "",
        f"## {len(worst)} lowest JA-overlap abilities (structure may under-express text)",
        "",
    ]
    for r in worst:
        lines.append(f"- **#{r['index']}** overlap={r['overlap']}")
        lines.append(f"  - text: {r['full_text'][:120]}")
        lines.append(f"  - ja: {r['describe_ja'][:100]}")
        lines.append(f"  - en: {r['describe_en'][:100]}")

    Path(args.out).write_text("\n".join(lines), encoding="utf-8")
    print(
        f"Report written to {args.out}: {len(rows)} abilities, "
        f"{sum(r['identical'] for r in rows)} fallbacks, {len(worst)} lowest-overlap listed."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
