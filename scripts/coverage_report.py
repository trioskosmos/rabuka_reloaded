"""Compare tested abilities (from test_debug.log) against unique_abilities from
abilities.json. Reports coverage rate and lists untested abilities.

Usage:
    cd engine && cargo test --test run_all -- --nocapture 2> ../test_debug.log
    cd .. && python scripts/coverage_report.py
"""

import json, re, sys
from collections import defaultdict
from pathlib import Path


def extract_tested(log_path):
    """Unique full_text strings from [AB]  TEXT lines following [AB]ABILITY."""
    tested = set()
    in_ab = False
    abi = re.compile(r'\[AB\]\s*ABILITY\s+"(.+?)"\s+\(\d+\)')
    txt = re.compile(r"\[AB\]\s+TEXT\s+(.*)")
    with open(log_path, encoding="utf-8", errors="replace") as f:
        for line in f:
            m = abi.search(line)
            if m:
                in_ab = True
                continue
            if not in_ab:
                continue
            m = txt.search(line)
            if m:
                tested.add(m.group(1).strip())
                in_ab = False
    return tested


def load_abilities_json(path):
    """Return (unique_abilities_list, card_to_texts_dict)."""
    with open(path, encoding="utf-8") as f:
        data = json.load(f)
    unique = data.get("unique_abilities", [])
    by_card = defaultdict(list)
    for entry in unique:
        ft = entry.get("full_text", "").strip()
        if not ft:
            continue
        for card_ref in entry.get("cards", []):
            card_no = card_ref.split("|")[0].strip()
            by_card[card_no].append(ft)
    return unique, dict(by_card)


def write_output(text, path):
    """Write text to a file to avoid cp932 console encoding issues on Windows."""
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)
    print(f"Report written to {path}")


def main():
    root = Path(__file__).resolve().parent.parent
    ab_path = root / "cards" / "abilities.json"
    log_path = root / "test_debug.log"

    if not log_path.exists():
        print(
            f"ERROR: {log_path} not found — run:\n  cd engine && cargo test --test run_all -- --nocapture 2> ../test_debug.log",
            file=sys.stderr,
        )
        return 1

    unique_abilities, by_card = load_abilities_json(str(ab_path))
    tested = extract_tested(str(log_path))

    # Count unique abilities
    all_ft = [
        e["full_text"].strip()
        for e in unique_abilities
        if e.get("full_text", "").strip()
    ]
    total = len(all_ft)
    tested_count = sum(1 for ft in all_ft if ft in tested)
    untested_fts = [ft for ft in all_ft if ft not in tested]

    # Build per-card untested list
    untested_by_card = defaultdict(list)
    for card_no, texts in by_card.items():
        for t in texts:
            if t not in tested:
                untested_by_card[card_no].append(t)

    lines = []
    lines.append("=" * 60)
    lines.append("UNIQUE ABILITY COVERAGE REPORT")
    lines.append("=" * 60)
    lines.append(f"Unique abilities in abilities.json:  {total}")
    lines.append(f"Activated during tests:             {tested_count}")
    lines.append(f"Untested:                            {total - tested_count}")
    lines.append(
        f"Coverage rate:                       {(tested_count / total * 100):.1f}%"
    )
    lines.append("")

    if untested_by_card:
        lines.append("=" * 60)
        lines.append(f"UNTESTED — {len(untested_by_card)} cards affected")
        lines.append("=" * 60)
        for card_no in sorted(untested_by_card):
            lines.append(f"  {card_no}")
            for t in untested_by_card[card_no]:
                preview = t.replace("\n", " ")[:100]
                lines.append(f"    {preview}")
            lines.append("")
        lines.append(f"Total cards with untested abilities: {len(untested_by_card)}")
    else:
        lines.append("All abilities are covered!")

    lines.append("=" * 60)
    write_output("\n".join(lines), str(root / "coverage_report.txt"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
