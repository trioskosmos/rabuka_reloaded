import json

with open("final_11_diffs.json", encoding="utf-8") as f:
    diffs = json.load(f)

for d in diffs:
    print(f"=== Diff #{d['num']}: {d['text']} ===")
    for df in d["diffs"]:
        path, gen, ref = df
        print(f"  Path: {path}")
        print(f"    Gen: {gen}")
        print(f"    Ref: {ref}")
    print()
