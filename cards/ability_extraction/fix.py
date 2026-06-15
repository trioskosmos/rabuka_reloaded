import os, sys
os.chdir(os.path.dirname(os.path.abspath(__file__)))

with open('parser.py', 'r', encoding='utf-8') as f:
    c = f.read()

original = c

# Change 1: Add heart_colors extraction in _try_live_mid
old1 = '    loc = extract_location(text)\n    if loc and "location" not in result:\n        result["location"] = loc\n    return result\n\n\ndef _extract_generic_fields'
new1 = '    loc = extract_location(text)\n    if loc and "location" not in result:\n        result["location"] = loc\n    hm = re.findall(r"{{heart_(\\d+)\\.png\\|heart\\d+}}", text)\n    if hm:\n        result["heart_colors"] = sorted(set(f"heart{m.zfill(2)}" for m in hm))\n    cm = re.search(r"(\\d+)以上", text)\n    if cm and "count" not in result:\n        result["count"] = int(cm.group(1))\n    return result\n\n\ndef _extract_generic_fields'

if old1 in c:
    c = c.replace(old1, new1)
    print("Change 1 OK")
else:
    print("Change 1 FAILED")

# Change 2: Add temporal_condition to propagation
old2 = '                    elif cond_type in ("location_condition",):\n                        cond["heart_colors"] = d["heart_colors"]'
new2 = '                    elif cond_type == "temporal_condition":\n                        cond["heart_colors"] = d["heart_colors"]\n                    elif cond_type in ("location_condition",):\n                        cond["heart_colors"] = d["heart_colors"]'

if old2 in c:
    c = c.replace(old2, new2)
    print("Change 2 OK")
else:
    print("Change 2 FAILED")

if c != original:
    with open('parser.py', 'w', encoding='utf-8') as f:
        f.write(c)
    print("Written to parser.py")
else:
    print("No changes made")
