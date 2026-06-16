"""
Golden file test for abilities.json.
Regenerate:  python cards/ability_extraction/extract_card_abilities.py
Check:       python test_against_golden.py

Locks baseline at current diff count.  Every code change must move
the diff count down, never up.
"""

import json, sys, os

GOLDEN = r"C:\Users\trios\Downloads\rabuka_reloaded-master (2)\rabuka_reloaded-master\cards\abilities.json"
GENERATED = os.path.join(os.path.dirname(__file__), "cards", "abilities.json")
BASELINE = 26  # current diff count — decrease this as fixes land


def compare():
    ref = json.load(open(GOLDEN, encoding="utf-8"))
    gen = json.load(open(GENERATED, encoding="utf-8"))

    ra = sorted(ref["unique_abilities"], key=lambda x: x.get("full_text", ""))
    ga = sorted(gen["unique_abilities"], key=lambda x: x.get("full_text", ""))

    ref_keys = {a["full_text"] for a in ra}
    gen_keys = {a["full_text"] for a in ga}

    shared_keys = ref_keys & gen_keys
    missing = ref_keys - gen_keys
    extra = gen_keys - ref_keys

    # Build lookup
    ref_by_key = {a["full_text"]: a for a in ra}
    gen_by_key = {a["full_text"]: a for a in ga}

    exact = 0
    diffs = 0
    diff_details = []

    for k in sorted(shared_keys):
        r = ref_by_key[k]
        g = gen_by_key[k]
        rd = {
            k: v
            for k, v in r.items()
            if k not in ("cards", "card_count", "generated_at")
        }
        gd = {
            k: v
            for k, v in g.items()
            if k not in ("cards", "card_count", "generated_at")
        }
        if rd == gd:
            exact += 1
        else:
            diffs += 1
            if len(diff_details) < 5:
                eff_r = r.get("effect", {}) or {}
                eff_g = g.get("effect", {}) or {}
                diff_fields = [
                    k
                    for k in set(list(eff_r.keys()) + list(eff_g.keys()))
                    if eff_r.get(k) != eff_g.get(k)
                ]
                diff_details.append((k[:80], diff_fields))

    print(f"Shared abilities:    {len(shared_keys)}")
    print(f"  Exact match:       {exact}")
    print(f"  Differ:            {diffs}")
    print(f"Missing (ref only):  {len(missing)}")
    print(f"Extra (gen only):    {len(extra)}")
    print()

    if diff_details:
        print("First 5 diffs (full_text, fields differing):")
        for ft, fields in diff_details:
            print(f"  {ft}")
            print(f"    Fields: {fields}")

    return diffs


if __name__ == "__main__":
    d = compare()
    print()
    if d <= BASELINE:
        print(f"PASS: {d} diffs <= baseline {BASELINE}")
        sys.exit(0)
    else:
        print(f"FAIL: {d} diffs > baseline {BASELINE}")
        sys.exit(1)
