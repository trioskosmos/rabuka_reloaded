"""
invert_abilities.py

Modes:
  python invert_abilities.py                        -- generate 3 report files (default)
  python invert_abilities.py --query '<json>'        -- search abilities containing a JSON fragment
  python invert_abilities.py --card <card_no>        -- show a card's full parsed JSON tree
  python invert_abilities.py --diff <card1> <card2>  -- compare two cards (structural diff)
  python invert_abilities.py --collisions            -- fingerprints with multiple distinct texts
  python invert_abilities.py --orphans               -- sub-objects with no 'text' field
"""

import json, os, re, sys
from collections import defaultdict

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ABILITIES_JSON = os.path.join(BASE, "abilities.json")
DIR = os.path.join(BASE, "ability_docs_scripts")

with open(ABILITIES_JSON, encoding="utf-8") as f:
    DATA = json.load(f)
UNIQUE = DATA["unique_abilities"]

SKIP_KEYS = {"text", "full_text", "triggerless_text"}
HEART_RE = re.compile(r"heart_\d{2}|icon_all")
POSITIONS = {"center", "left_side", "right_side"}
UNITS = {"人", "枚", "つ", "色", "種類", "個", "回"}

# ---- Helpers ----


def clean(t):
    return t.replace("{{", "{").replace("}}", "}") if t else ""


def short_cards(entry, max_show=4):
    cards = [c.split(" |")[0] for c in entry.get("cards", [])]
    shown = cards[:max_show]
    rest = len(cards) - max_show
    s = ", ".join(shown)
    if rest > 0:
        s += f" (... +{rest} more)"
    return s


def fmt_json(obj):
    return json.dumps(obj, indent=2, ensure_ascii=False)


def canonical_structure(obj):
    if isinstance(obj, dict):
        items = {}
        for k, v in sorted(obj.items()):
            if k in SKIP_KEYS or v is None:
                continue
            items[k] = canonical_structure(v)
        return items if items else None
    elif isinstance(obj, list):
        items = [canonical_structure(v) for v in obj]
        items = [v for v in items if v is not None]
        return items if items else None
    return obj


def canonical_json(obj):
    cs = canonical_structure(obj)
    if cs is None:
        return None
    return json.dumps(cs, sort_keys=True, ensure_ascii=False)


def abstract_value(k, v):
    if isinstance(v, bool):
        return (v, None)
    if isinstance(v, int):
        return ("<N>", v)
    if isinstance(v, str) and HEART_RE.fullmatch(v):
        return ("<HEART>", v)
    if isinstance(v, str) and v in POSITIONS:
        return ("<POS>", v)
    if isinstance(v, str) and v in UNITS:
        return ("<UNIT>", v)
    return (v, None)


def abstract_structure(obj, track=None):
    if track is None:
        track = []
    if isinstance(obj, dict):
        items = {}
        for k, v in sorted(obj.items()):
            if k in SKIP_KEYS or v is None:
                continue
            if k == "group_names" and isinstance(v, list):
                sg = sorted(str(x) for x in v if x)
                items[k] = ["<GROUP>"] * len(sg) if sg else None
                if sg:
                    for g in sg:
                        track.append(("group", g))
            elif k == "characters" and isinstance(v, list):
                names = [str(x) for x in v if x]
                items[k] = ["<CHAR>"] * len(names) if names else None
                if names:
                    for n in names:
                        track.append(("char", n))
            elif k in (
                "count",
                "energy",
                "value",
                "calculation_value",
                "cost_limit",
                "cost_limit_value",
            ):
                if isinstance(v, int):
                    items[k] = "<N>"
                    track.append(("N", v))
                else:
                    items[k] = abstract_structure(v, track)
            elif k in ("operator",):
                if isinstance(v, str):
                    items[k] = "<OP>"
                    track.append(("op", v))
                else:
                    items[k] = abstract_structure(v, track)
            elif k in ("target",):
                if isinstance(v, str):
                    items[k] = "<TARGET>"
                    track.append(("target", v))
                else:
                    items[k] = abstract_structure(v, track)
            elif k in ("location",):
                if isinstance(v, str):
                    items[k] = "<LOC>"
                    track.append(("loc", v))
                else:
                    items[k] = abstract_structure(v, track)
            elif k in ("card_type",):
                if isinstance(v, str):
                    items[k] = "<CT>"
                    track.append(("ct", v))
                else:
                    items[k] = abstract_structure(v, track)
            elif k in ("comparison_type",):
                if isinstance(v, str):
                    items[k] = "<COMP>"
                    track.append(("comp", v))
                else:
                    items[k] = abstract_structure(v, track)
            elif k in ("operation",):
                if isinstance(v, str):
                    items[k] = "<OPN>"
                    track.append(("opn", v))
                else:
                    items[k] = abstract_structure(v, track)
            elif k in ("state_change",):
                if isinstance(v, str):
                    items[k] = "<STATE>"
                    track.append(("state", v))
                else:
                    items[k] = abstract_structure(v, track)
            elif k == "resource" and isinstance(v, str) and "heart" in v.lower():
                items[k] = v
            else:
                items[k] = abstract_structure(v, track)
        return items if items else None
    elif isinstance(obj, list):
        items = [abstract_structure(v, track) for v in obj]
        items = [v for v in items if v is not None]
        return items if items else None
    else:
        av, concrete = abstract_value("", obj)
        if concrete is not None:
            track.append(("value", concrete))
        return av


def abstract_json(obj):
    track = []
    st = abstract_structure(obj, track)
    if st is None:
        return None, []
    return json.dumps(st, sort_keys=True, ensure_ascii=False), track


def extract_all_sub_objects(obj):
    results = []
    if isinstance(obj, dict):
        non_text_keys = set(obj.keys()) - SKIP_KEYS
        if non_text_keys:
            cj = canonical_json(obj)
            if cj:
                results.append((cj, obj))
        for k, v in obj.items():
            if k in SKIP_KEYS:
                continue
            results.extend(extract_all_sub_objects(v))
    elif isinstance(obj, list):
        for v in obj:
            results.extend(extract_all_sub_objects(v))
    return results


def get_text(obj):
    if isinstance(obj, dict):
        return obj.get("text", "")
    return ""


def find_json_fragment(obj, fragment):
    if isinstance(obj, dict):
        if all(obj.get(k) == v for k, v in fragment.items()):
            return True
        return any(find_json_fragment(v, fragment) for v in obj.values())
    elif isinstance(obj, list):
        return any(find_json_fragment(v, fragment) for v in obj)
    return False


def print_tree(obj, indent=0, label=""):
    lines = []
    prefix = "  " * indent
    if isinstance(obj, dict):
        action = obj.get("action", obj.get("type", ""))
        text = obj.get("text", "")
        parts = [prefix + label + (" " if label else "") + "{"]
        if action:
            parts.append(action)
        extra = []
        for k in (
            "count",
            "energy",
            "value",
            "resource",
            "destination",
            "source",
            "position",
            "activation_position",
            "card_type",
            "location",
            "target",
            "duration",
            "state_change",
            "operator",
            "comparison_type",
        ):
            if k in obj and obj[k] is not None:
                extra.append(f"{k}={obj[k]}")
        if extra:
            parts.append("[" + ", ".join(extra) + "]")
        parts.append("}")
        lines.append(" ".join(parts))
        if text:
            lines.append(prefix + "  -> " + clean(text[:120]))
        for k, v in obj.items():
            if k in SKIP_KEYS or v is None:
                continue
            if isinstance(v, (dict, list)):
                sub = print_tree(
                    v, indent + 1, f".{k}" if not isinstance(v, list) else f"[{v}]"
                )
                lines.extend(sub)
    elif isinstance(obj, list):
        for i, v in enumerate(obj):
            sub = print_tree(
                v,
                indent,
                (label + "[" + str(i) + "]") if label else ("[" + str(i) + "]"),
            )
            lines.extend(sub)
    return lines


# ==============
# MODE: QUERY
# ==============


def run_query(fragment_str):
    try:
        fragment = json.loads(fragment_str)
    except json.JSONDecodeError as e:
        print(f"Invalid JSON: {e}")
        sys.exit(1)
    print(f"Query: {json.dumps(fragment, ensure_ascii=False)}")
    matches = []
    for uid, entry in enumerate(UNIQUE):
        if entry.get("is_null"):
            continue
        if find_json_fragment(entry, fragment):
            matches.append((uid, entry))
    print(f"Found: {len(matches)} unique abilities\n")
    for uid, entry in matches[:20]:
        cards = short_cards(entry)
        print(f"--- {cards} (uid={uid}) ---")
        cost = entry.get("cost")
        eff = entry.get("effect", {})
        if cost:
            print("Cost:")
            print(fmt_json(cost)[:300])
        print("Effect:")
        print(fmt_json(eff)[:300])
        print()
    if len(matches) > 20:
        print(f"... +{len(matches) - 20} more")


# ==============
# MODE: CARD
# ==============


def run_card(card_no):
    found = False
    for uid, entry in enumerate(UNIQUE):
        if entry.get("is_null"):
            continue
        cards = [c.split(" |")[0] for c in entry.get("cards", [])]
        if any(card_no in c for c in cards):
            found = True
            print(f"=== {', '.join(cards)} (uid={uid}) ===")
            print(f"Trigger: {entry.get('triggers', '?')}")
            print(f"Full: {entry.get('full_text', '')[:250]}")
            print()
            cost = entry.get("cost")
            if cost:
                print("+ COST")
                for line in print_tree(cost):
                    print(line)
                print()
            eff = entry.get("effect", {})
            if eff:
                print("+ EFFECT")
                for line in print_tree(eff):
                    print(line)
                print()
    if not found:
        print(f"Card '{card_no}' not found")


# ==============
# MODE: DIFF
# ==============


def run_diff(card_a, card_b):
    entries = {}
    for uid, entry in enumerate(UNIQUE):
        if entry.get("is_null"):
            continue
        cards = [c.split(" |")[0] for c in entry.get("cards", [])]
        for c in cards:
            if card_a in c:
                entries["A"] = (uid, entry)
            if card_b in c:
                entries["B"] = (uid, entry)

    if "A" not in entries:
        print(f"Card '{card_a}' not found")
        return
    if "B" not in entries:
        print(f"Card '{card_b}' not found")
        return

    ua, ea = entries["A"]
    ub, eb = entries["B"]

    ca = short_cards(ea)
    cb = short_cards(eb)

    print(f"=== Diff: {card_a} (uid={ua}) vs {card_b} (uid={ub}) ===")
    print()

    # Compare triggers
    ta = ea.get("triggers", "")
    tb = eb.get("triggers", "")
    if ta != tb:
        print(f"[-TRIGGER] {ta}")
        print(f"[+TRIGGER] {tb}")
    else:
        print(f"[=TRIGGER] {ta}")
    print()

    # Compare full text
    fa = clean(ea.get("full_text", ""))
    fb = clean(eb.get("full_text", ""))
    print(f"[A] {fa[:200]}")
    print(f"[B] {fb[:200]}")
    print()

    # Compare cost
    costa = ea.get("cost") or {}
    costb = eb.get("cost") or {}
    ca_str = fmt_json(costa)
    cb_str = fmt_json(costb)
    if ca_str != cb_str:
        print("[-COST] " + ca_str[:300])
        print("[+COST] " + cb_str[:300])
    else:
        print("[=COST] (same)")
    print()

    # Compare effect
    effa = fmt_json(ea.get("effect", {}))
    effb = fmt_json(eb.get("effect", {}))
    if effa != effb:
        print("[-EFFECT]")
        for line in print_tree(ea["effect"]):
            print("  " + line)
        print("[+EFFECT]")
        for line in print_tree(eb["effect"]):
            print("  " + line)
    else:
        print("[=EFFECT] (same)")


# ==============
# MODE: COLLISIONS
# ==============


def run_collisions():
    fp_groups = defaultdict(list)
    for uid, entry in enumerate(UNIQUE):
        if entry.get("is_null"):
            continue
        card_short = short_cards(entry)
        full_text = entry.get("full_text", "")
        for cj, obj in extract_all_sub_objects(entry):
            source_text = get_text(obj)
            fp_groups[cj].append((uid, card_short, full_text, source_text))

    # Filter: only groups with multiple distinct source_texts
    collisions = []
    for fp, entries in fp_groups.items():
        unique_texts = set()
        seen = set()
        for uid, cs, ft, st in entries:
            if st and len(st) > 3:
                unique_texts.add(st)
                seen.add(uid)
        if len(unique_texts) >= 2:
            collisions.append((len(unique_texts), len(seen), fp, entries))

    collisions.sort(key=lambda x: -x[0])

    print(f"# Collisions Report\n")
    print(f"JSON fingerprints shared by multiple distinct text strings.\n")
    print(f"These are places where the parser collapses different source texts")
    print(f"into the same JSON structure -- potentially losing information.\n")
    print(f"Total: {len(collisions)} collision groups\n")

    for n_texts, n_abilities, fp, entries in collisions[:50]:
        print(f"## {n_texts} distinct texts across {n_abilities} abilities\n")
        print("```json")
        print(fp)
        print("```\n")
        seen_texts = set()
        for uid, cs, ft, st in entries:
            st_clean = clean(st.strip())
            if st_clean and st_clean not in seen_texts:
                seen_texts.add(st_clean)
                print(f"- Cards: {cs}")
                print(f"  Text: {st_clean[:120]}")
                print()
        print("---\n")

    if len(collisions) > 50:
        print(f"... +{len(collisions) - 50} more collision groups\n")

    # Summary table
    print("| Texts | Abilities | JSON Fingerprint |")
    print("|-------|-----------|-----------------|")
    for n_texts, n_abilities, fp, _ in collisions[:30]:
        fpp = fp[:80]
        print(f"| {n_texts} | {n_abilities} | {fpp} |")


# ==============
# MODE: ORPHANS
# ==============


def run_orphans():
    print(f"# Orphan Report\n")
    print(f"Sub-objects in the ability JSON that have NO 'text' field.\n")
    print(f"These may be parser artifacts -- structure that was created from")
    print(f"text processing but whose parsed components lost their source text.\n")

    count = 0
    for uid, entry in enumerate(UNIQUE):
        if entry.get("is_null"):
            continue

        def find_orphans(obj, path=""):
            orphans = []
            if isinstance(obj, dict):
                non_text = set(obj.keys()) - SKIP_KEYS
                if non_text and "text" not in obj:
                    orphans.append((path, obj))
                for k, v in obj.items():
                    if k in SKIP_KEYS:
                        continue
                    orphans.extend(find_orphans(v, f"{path}.{k}"))
            elif isinstance(obj, list):
                for i, v in enumerate(obj):
                    orphans.extend(find_orphans(v, f"{path}[{i}]"))
            return orphans

        orphans = find_orphans(entry)
        for path, obj in orphans:
            count += 1
            if count > 40:
                break

            cards = short_cards(entry)
            action = obj.get("action", obj.get("type", "?"))
            extra = {
                k: v
                for k, v in obj.items()
                if k not in SKIP_KEYS and k not in ("action", "type") and v is not None
            }
            extra_str = ", ".join(f"{k}={v}" for k, v in list(extra.items())[:5])
            print(f"- **{cards}**")
            print(f"  path: {path}")
            print(f"  type: {action}")
            if extra_str:
                print(f"  keys: {extra_str}")
            print()

        if count > 40:
            break

    print(f"Total orphans found: check the first {min(count, 40)} above")
    if count > 40:
        print(f"(showing first 40 of {count})")


# ==============
# MODE: REPORTS
# ==============


def run_reports():
    concrete_groups = defaultdict(list)
    abstract_groups = defaultdict(list)

    for uid, entry in enumerate(UNIQUE):
        if entry.get("is_null"):
            continue
        card_short = short_cards(entry)
        full_text = entry.get("full_text", "")
        for cj, obj in extract_all_sub_objects(entry):
            source_text = get_text(obj)
            concrete_groups[cj].append((card_short, full_text, source_text))
            afp, track = abstract_json(obj)
            if afp:
                abstract_groups[afp].append((card_short, full_text, source_text, track))

    sorted_concrete = sorted(concrete_groups.items(), key=lambda x: -len(x[1]))
    sorted_abstract = sorted(abstract_groups.items(), key=lambda x: -len(x[1]))

    # Verbose
    lines_v = [
        "# Inverted Abilities Index\n",
        f"Source: abilities.json ({len(UNIQUE)} unique abilities)\n",
        "For every unique JSON sub-structure at any depth, this lists ALL raw",
        "Japanese texts that produce it. When the same JSON is produced by texts",
        "with different meanings, the parser is losing information.\n",
        f"**Total unique JSON fingerprints: {len(sorted_concrete)}**\n",
        "---\n",
    ]
    for fp, entries in sorted_concrete:
        lines_v.append(f"## {len(entries)} occurrence(s)\n")
        lines_v.append("```json")
        lines_v.append(fp)
        lines_v.append("```\n")
        unique_texts = {}
        for card_short, full_text, source_text in entries:
            if full_text not in unique_texts:
                unique_texts[full_text] = {"cards": card_short, "source_texts": []}
            unique_texts[full_text]["source_texts"].append(source_text)
        for ft, info in unique_texts.items():
            lines_v.append(f"- **Cards**: {info['cards']}")
            for st in info["source_texts"]:
                if st and len(st) > 3:
                    lines_v.append(f"  - **Mapped text**: {clean(st[:150])}")
            lines_v.append(f"  **Full**: {clean(ft[:280])}")
            lines_v.append("")

    with open(os.path.join(DIR, "INVERTED_ABILITIES.md"), "w", encoding="utf-8") as f:
        f.write("\n".join(lines_v))
    print(
        f"Verbose: {os.path.join(DIR, 'INVERTED_ABILITIES.md')}  ({len(lines_v)} lines)"
    )

    # Condensed
    lines_c = [
        "# Inverted Abilities Index (Condensed)\n",
        f"Source: abilities.json ({len(UNIQUE)} unique abilities)\n",
        f"**Total unique JSON fingerprints: {len(sorted_concrete)}**\n",
        "---\n",
    ]
    for fp, entries in sorted_concrete:
        lines_c.append("```json")
        lines_c.append(fp)
        lines_c.append("```\n")
        text_counts = {}
        for card_short, full_text, source_text in entries:
            st_clean = clean(source_text.strip())
            if st_clean:
                text_counts[st_clean] = text_counts.get(st_clean, 0) + 1
        for st, cnt in sorted(text_counts.items(), key=lambda x: (-x[1], x[0])):
            lines_c.append(f"- {st} (x{cnt})")
        lines_c.append("")

    with open(
        os.path.join(DIR, "INVERTED_ABILITIES_CONDENSED.md"), "w", encoding="utf-8"
    ) as f:
        f.write("\n".join(lines_c))
    print(
        f"Condensed: {os.path.join(DIR, 'INVERTED_ABILITIES_CONDENSED.md')}  ({len(lines_c)} lines)"
    )

    # Abstract
    lines_a = [
        "# Inverted Abilities Index (Abstract)\n",
        f"Source: abilities.json ({len(UNIQUE)} unique abilities)\n",
        "Values replaced with placeholders so structurally-similar JSON entries",
        "group together. Each group shows the variable breakdown per text.\n",
        f"**Total abstract fingerprints: {len(sorted_abstract)}**\n",
        f"(Concrete fingerprints: {len(sorted_concrete)})\n",
        "---\n",
    ]
    for afp, entries in sorted_abstract:
        lines_a.append(f"## {len(entries)} occurrence(s)\n")
        lines_a.append("```json")
        lines_a.append(afp)
        lines_a.append("```\n")
        tvc = defaultdict(lambda: {"count": 0, "vars": defaultdict(int)})
        for card_short, full_text, source_text, track in entries:
            st_clean = clean(source_text.strip())
            if st_clean:
                tvc[st_clean]["count"] += 1
                for vtype, vval in track:
                    tvc[st_clean]["vars"][(vtype, str(vval))] += 1
        all_vars = defaultdict(set)
        for st_clean, info in tvc.items():
            for (vtype, vval), _ in info["vars"].items():
                all_vars[vtype].add(vval)
        if all_vars:
            lines_a.append("| Variable | Values |")
            lines_a.append("|----------|--------|")
            for vtype in sorted(all_vars.keys()):
                vals = sorted(str(v) for v in all_vars[vtype])
                lines_a.append(f"| {vtype} | {', '.join(vals)} |")
            lines_a.append("")
        for st_clean, info in sorted(tvc.items(), key=lambda x: -x[1]["count"]):
            cnt = info["count"]
            var_parts = [
                f"{vt}={vv}"
                for (vt, vv), _ in sorted(info["vars"].items(), key=lambda x: -x[1])
            ]
            var_str = ", ".join(var_parts)
            lines_a.append(
                f"- {st_clean} (x{cnt}, {var_str})"
                if var_str
                else f"- {st_clean} (x{cnt})"
            )
        lines_a.append("")

    with open(
        os.path.join(DIR, "INVERTED_ABILITIES_ABSTRACT.md"), "w", encoding="utf-8"
    ) as f:
        f.write("\n".join(lines_a))
    print(
        f"Abstract: {os.path.join(DIR, 'INVERTED_ABILITIES_ABSTRACT.md')}  ({len(lines_a)} lines)"
    )
    print(f"\nConcrete fingerprints: {len(sorted_concrete)}")
    print(f"Abstract fingerprints: {len(sorted_abstract)}")


# ==============
# MAIN
# ==============

if __name__ == "__main__":
    if len(sys.argv) > 1:
        mode = sys.argv[1]
        if mode == "--query" and len(sys.argv) > 2:
            run_query(sys.argv[2])
        elif mode == "--card" and len(sys.argv) > 2:
            run_card(sys.argv[2])
        elif mode == "--diff" and len(sys.argv) > 3:
            run_diff(sys.argv[2], sys.argv[3])
        elif mode == "--collisions":
            run_collisions()
        elif mode == "--orphans":
            run_orphans()
        elif mode == "--report":
            run_reports()
        else:
            print("Usage:")
            print("  python invert_abilities.py                         -- 3 reports")
            print(
                "  python invert_abilities.py --query '<json>'        -- search JSON fragment"
            )
            print(
                "  python invert_abilities.py --card <card_no>        -- show parsed tree"
            )
            print(
                "  python invert_abilities.py --diff <a> <b>          -- compare two cards"
            )
            print(
                "  python invert_abilities.py --collisions            -- same JSON, different texts"
            )
            print(
                "  python invert_abilities.py --orphans               -- sub-objects missing text"
            )
    else:
        run_reports()
