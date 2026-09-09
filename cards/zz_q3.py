import json

d = json.load(open("engine/tests/TEST_INVENTORY.json", encoding="utf-8"))
q = d["quality"]
for smell in ["pendency_only", "no_drive"]:
    print("=" * 100)
    print(smell, q[smell]["count"])
    for r in q[smell]["rows"]:
        print("%s :: %s (L%s)" % (r["file"].split("/")[-1], r["test"], r["line"]))
