#!/usr/bin/env python3
"""
analyze_phrases.py — corpus-driven phrase→JSON clustering.

Reads the parsed ability corpus (cards/abilities.json) and reports where the
parser can be compressed: which Japanese ability texts collapse to the same
output JSON shape (one rule with match_any=[...] could cover a cluster), and
where the gaps are (custom/fallthrough, rare shapes).

Usage:
    python analyze_phrases.py                 # full report to stdout
    python analyze_phrases.py --top 25        # top-N clusters per section
    python analyze_phrases.py --dump out.json # also write raw clusters
"""

import json
import sys
from collections import Counter, defaultdict
from pathlib import Path

# Which fields identify a leaf action's output shape. Actions missing here are
# identified by name alone. These mirror the engine's consumed fields.
LEAF_FIELDS = {
    "gain_resource": ["resource", "count", "duration"],
    "move_cards": ["source", "destination", "card_type"],
    "change_state": ["state_change", "card_type"],
    "modify_score": ["operation", "value"],
    "draw_card": ["count", "optional"],
    "select": ["count", "destination"],
    "select_cards": ["count", "destination"],
    "look_at": ["source", "count"],
    "modify_cost": ["operation"],
    "place_energy_under_member": ["energy_count"],
    "restriction": ["restriction_type"],
    "invalidate_ability": ["all"],
    "modify_required_hearts": [],
}

# Wrapper nodes that don't represent an engine action themselves.
WRAPPER_ACTIONS = {
    "sequential",
    "choice",
    "look_and_select",
    "conditional_on_result",
    "conditional_alternative",
    "conditional_on_optional",
}


def leaf_key(node: dict) -> str:
    """A string identifying a leaf action's output shape, e.g. 'move_cards:hand:discard:card'."""
    a = node.get("action", "?")
    key = a
    for f in LEAF_FIELDS.get(a, []):
        v = node.get(f)
        key += ":" + (str(v) if v is not None else "?")
    return key


def walk_actions(node, out):
    """Collect all leaf action nodes in an effect tree."""
    if isinstance(node, dict):
        if node.get("action"):
            out.append(node)
        for v in node.values():
            walk_actions(v, out)
    elif isinstance(node, list):
        for item in node:
            walk_actions(item, out)


def text_sig(text: str) -> str:
    """Normalized phrase for dedup: strip icons to their label text."""
    import re

    t = re.sub(r"\{\{[^|]+\|([^}]+)\}\}", r"\1", text)
    t = re.sub(r"\s+", "", t)
    return t


def main():
    args = sys.argv[1:]
    top_n = 25
    dump_path = None
    for a in args:
        if a == "--top":
            continue
        elif a.isdigit():
            top_n = int(a)
        elif a.startswith("--dump"):
            dump_path = a.split("=", 1)[1] if "=" in a else "clusters.json"

    here = Path(__file__).parent
    src = here.parent / "abilities.json"
    data = json.load(open(src, encoding="utf-8"))
    abilities = data["unique_abilities"]

    # 1) Compose per-ability signature from the whole effect tree.
    sig_to_texts = defaultdict(list)  # full leaf signature -> phrases
    action_counts = Counter()  # every leaf action name
    shape_to_texts = defaultdict(list)  # leaf (action:fields) -> phrases
    gaps = []  # custom / empty / odd
    cond_counts = Counter()

    for a in abilities:
        eff = a.get("effect") or {}
        leaves = []
        walk_actions(eff, leaves)

        # Count condition types (top-level + nested on wrapper nodes).
        conds = []

        def walk_conds(node):
            if isinstance(node, dict):
                c = node.get("condition")
                if isinstance(c, dict) and c.get("type"):
                    conds.append(c.get("type"))
                for v in node.values():
                    walk_conds(v)
            elif isinstance(node, list):
                for item in node:
                    walk_conds(item)

        walk_conds(eff)

        text = text_sig(a.get("triggerless_text", ""))

        if not leaves:
            gaps.append(("empty", a.get("triggerless_text", ""), eff))
            continue
        for n in leaves:
            action_counts[n["action"]] += 1
            shape_to_texts[leaf_key(n)].append(text)
        sig = "|".join(sorted({leaf_key(n) for n in leaves}))
        sig_to_texts[sig].append(text)

        if any(n["action"] == "custom" for n in leaves):
            gaps.append(("custom", a.get("triggerless_text", ""), eff))

        for ct in conds:
            cond_counts[ct] += 1

    n_abilities = len(abilities)
    n_distinct_texts = sum(len(v) for v in sig_to_texts.values())
    n_signatures = len(sig_to_texts)

    sep = "=" * 88
    print(sep)
    print("ABILITY CORPUS — phrase → JSON compression map")
    print(f"source: {src.name}   unique abilities: {n_abilities}")
    print(sep)

    print(f"\nLeaf-signature compression:")
    print(
        f"  {n_distinct_texts} normalized phrases  →  {n_signatures} distinct output signatures"
    )
    ratio = n_distinct_texts / max(1, n_signatures)
    print(
        f"  avg {ratio:.1f} phrasings per signature "
        f"({max(1, round(ratio * 100 / max(1, n_signatures)))}% redundant if rules keyed by signature)\n"
    )

    print("=" * 88)
    print("TOP LEAF SIGNATURES  (structural composition: one rule could cover each)")
    print("=" * 88)
    for sig, texts in sorted(sig_to_texts.items(), key=lambda kv: -len(kv[1]))[:top_n]:
        print(f"\n  [{len(texts):3} abilities] {sig}")
        for t in texts[:4]:
            print(f"      {t[:100]}")

    print("\n" + "=" * 88)
    print("TOP LEAF ACTIONS  (every action node anywhere in any tree)")
    print("=" * 88)
    for name, cnt in action_counts.most_common(35):
        print(f"  {cnt:4}  {name}")

    print("\n" + "=" * 88)
    print("COMPRESSION CLUSTERS  (phrasings → same leaf shape; biggest first)")
    print("=" * 88)
    for shape, texts in sorted(shape_to_texts.items(), key=lambda kv: -len(kv[1]))[
        :top_n
    ]:
        distinct = sorted(set(texts))
        print(f"\n  [{len(texts):3} nodes] {shape}")
        for t in distinct[:5]:
            print(f"      {t[:110]}")

    print("\n" + "=" * 88)
    print("CONDITION TYPES  (every condition node anywhere in any tree)")
    print("=" * 88)
    for name, cnt in cond_counts.most_common(25):
        print(f"  {cnt:4}  {name}")

    print("\n" + "=" * 88)
    print("GAPS  (custom / empty — rules that need attention)")
    print("=" * 88)
    for kind, text, eff in gaps:
        print(f"\n  [{kind}] {text[:120]}")

    if dump_path:
        payload = {
            "n_abilities": n_abilities,
            "n_signatures": n_signatures,
            "signatures": {
                k: v
                for k, v in sorted(sig_to_texts.items(), key=lambda kv: -len(kv[1]))
            },
            "clusters": {
                k: sorted(set(v))
                for k, v in sorted(shape_to_texts.items(), key=lambda kv: -len(kv[1]))
            },
            "gaps": [t for _, t, _ in gaps],
        }
        json.dump(
            payload,
            open(dump_path, "w", encoding="utf-8"),
            ensure_ascii=False,
            indent=2,
        )
        print(f"\n[dumped clusters -> {dump_path}]")


if __name__ == "__main__":
    main()
