import json

bak = r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json.bak"
new = r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json"
o = json.load(open(bak, encoding="utf-8"))
n = json.load(open(new, encoding="utf-8"))

# Show a few concrete examples side by side
examples = [0, 2, 10, 50, 100, 200]
for idx in examples:
    oa = o["unique_abilities"][idx]
    na = n["unique_abilities"][idx]
    print(f"=== ABILITY #{idx} ===")
    print(f"FULL: {oa['full_text'][:100]}")
    print()
    
    # Compare cost
    oc = oa.get("cost") or {}
    nc = na.get("cost") or {}
    if oc != nc:
        print("  COST diff:")
        print(f"    OLD: {json.dumps(oc, ensure_ascii=False)}")
        print(f"    NEW: {json.dumps(nc, ensure_ascii=False)}")
    else:
        print("  COST: same")
    
    # Compare effect
    oe = oa.get("effect") or {}
    ne = na.get("effect") or {}
    if oe != ne:
        print("  EFFECT diff:")
        o_str = json.dumps(oe, ensure_ascii=False, indent=4)
        n_str = json.dumps(ne, ensure_ascii=False, indent=4)
        # Only show first 500 chars of each
        if len(o_str) > 500 or len(n_str) > 500:
            o_lines = o_str.split('\n')
            n_lines = n_str.split('\n')
            for line in range(max(len(o_lines), len(n_lines))):
                if line < len(o_lines) and line < len(n_lines):
                    if o_lines[line] != n_lines[line]:
                        print(f"    -{o_lines[line]}")
                        print(f"    +{n_lines[line]}")
                elif line < len(o_lines):
                    print(f"    -{o_lines[line]}")
                else:
                    print(f"    +{n_lines[line]}")
        else:
            print(f"    OLD: {o_str}")
            print(f"    NEW: {n_str}")
    else:
        print("  EFFECT: same")
    
    print()

# Summary of all field diffs
print("=== FIELD-BY-FIELD SUMMARY ===")
o_fields = set()
n_fields = set()
for a in o["unique_abilities"]:
    oe = a.get("effect") or {}
    o_fields.update(oe.keys())
for a in n["unique_abilities"]:
    ne = a.get("effect") or {}
    n_fields.update(ne.keys())
    
added = n_fields - o_fields
removed = o_fields - n_fields
print(f"Fields added to effect: {sorted(added)}")
print(f"Fields removed from effect: {sorted(removed)}")
print(f"Fields in both: {sorted(o_fields & n_fields)}")

# Count how many abilities differ
diff_count = 0
for oa, na in zip(o["unique_abilities"], n["unique_abilities"]):
    if oa.get("effect") != na.get("effect") or oa.get("cost") != na.get("cost"):
        diff_count += 1
print(f"\nTotal abilities with any difference: {diff_count}/{len(o['unique_abilities'])}")
