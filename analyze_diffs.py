import json


def strip_text(obj):
    if isinstance(obj, dict):
        return {k: strip_text(v) for k, v in obj.items() if k != "text"}
    elif isinstance(obj, list):
        return [strip_text(v) for v in obj]
    return obj


with open("cards/abilities.json", encoding="utf-8") as f:
    gen = json.load(f)
with open(
    "C:/Users/trios/Downloads/rabuka_reloaded-master (2)/rabuka_reloaded-master/cards/abilities.json",
    encoding="utf-8",
) as f:
    ref = json.load(f)

gen_lookup = {a.get("triggerless_text", ""): a for a in gen["unique_abilities"]}
ref_lookup = {a.get("triggerless_text", ""): a for a in ref["unique_abilities"]}

n = 0
for t in sorted(set(gen_lookup.keys()) & set(ref_lookup.keys())):
    ge = strip_text(gen_lookup[t].get("effect", {}))
    re = strip_text(ref_lookup[t].get("effect", {}))
    if ge != re:
        n += 1
        keys = sorted(set(list(ge.keys()) + list(re.keys())))
        print(f"=== DIFF #{n}: {'|'.join(keys)} ===")
        print(f"TEXT: {t}")

        # Show key structural diffs
        def show_diffs(a, b, path=""):
            if not isinstance(a, dict) or not isinstance(b, dict):
                if a != b:
                    print(
                        f"  {path}: gen={json.dumps(str(a)[:80], ensure_ascii=False)} ref={json.dumps(str(b)[:80], ensure_ascii=False)}"
                    )
                return
            for k in sorted(set(list(a.keys()) + list(b.keys()))):
                if k == "text":
                    continue
                av = a.get(k, "_MISSING_")
                bv = b.get(k, "_MISSING_")
                if isinstance(av, list) and isinstance(bv, list):
                    for i, (av_i, bv_i) in enumerate(zip(av, bv)):
                        if av_i != bv_i:
                            show_diffs(av_i, bv_i, f"{path}.{k}[{i}]")
                elif isinstance(av, dict) and isinstance(bv, dict):
                    if av != bv:
                        show_diffs(av, bv, f"{path}.{k}")
                elif av != bv:
                    print(
                        f"  {path}.{k}: gen={json.dumps(str(av)[:80], ensure_ascii=False)} ref={json.dumps(str(bv)[:80], ensure_ascii=False)}"
                    )

        show_diffs(ge, re)
        print()
        if n >= 13:
            break
