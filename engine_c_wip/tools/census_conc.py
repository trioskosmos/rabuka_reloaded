#!/usr/bin/env python3
"""Corpus reachability census for the two concentrations.

Measures the sites named in the brief BEFORE anything is ported, so a
zero-reachability item is recorded as a FLOOR (VERIFIED-NOOP: 0) rather than
as "no work needed", and a non-zero item is recorded with its true count.

The three tallies the brief warns about were all wrong in the direction that
makes work look unnecessary, so nothing here trusts a top-level count: every
number is a per-NODE walk over unique_abilities, split cost vs effect, and
nested keys (sequential / compound / choice branches) are walked too.
"""
import collections
import io
import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CORPUS = os.path.join(os.path.dirname(ROOT), "cards", "abilities.json")

FILTER_KEYS = ("heart_total_limit", "heart_total_operator", "cost_limit_min",
               "cost_limit_max", "exclude_group_names", "group_names",
               "exclude_names", "exclude_cards", "distinct", "card_property",
               "cost_limit", "cost_limit_operator", "name_constraint")


def walk(node, keys, scalars, actions):
    if isinstance(node, dict):
        for k, v in node.items():
            if k in FILTER_KEYS:
                keys[k] += 1
            if k == "state_change":
                scalars["state_change=" + repr(v)] += 1
            if k == "card_property":
                scalars["card_property=" + repr(v)] += 1
            if k == "distinct":
                scalars["distinct=" + repr(v)] += 1
            if k == "heart_total_operator":
                scalars["heart_total_operator=" + repr(v)] += 1
            if k == "action" and isinstance(v, str):
                actions[v] += 1
            walk(v, keys, scalars, actions)
    elif isinstance(node, list):
        for v in node:
            walk(v, keys, scalars, actions)


def main():
    data = json.load(io.open(CORPUS, encoding="utf-8"))
    ab = data["unique_abilities"]
    ck, cs, ca = collections.Counter(), collections.Counter(), collections.Counter()
    ek, es, ea = collections.Counter(), collections.Counter(), collections.Counter()
    for a in ab:
        walk(a.get("cost"), ck, cs, ca)
        walk(a.get("effect"), ek, es, ea)

    print("corpus: %d unique abilities" % len(ab))
    print("\n=== COST nodes (nested keys included)")
    for k in FILTER_KEYS:
        if ck[k]:
            print("  %-24s %d" % (k, ck[k]))
    print("  -- scalars")
    for k in sorted(cs):
        print("     %-34s %d" % (k, cs[k]))
    print("  -- actions")
    for k, v in sorted(ca.items(), key=lambda kv: -kv[1]):
        print("     %-34s %d" % (k, v))

    print("\n=== EFFECT nodes (nested keys included)")
    for k in FILTER_KEYS:
        if ek[k]:
            print("  %-24s %d" % (k, ek[k]))
    print("  -- scalars")
    for k in sorted(es):
        print("     %-34s %d" % (k, es[k]))

    print("\n=== the six RbCardFilter carriers, per carrier")
    # exclude_cards / exclude_names are quoted `None` by Rust at that site
    # (util.rs:1926-1927), so they are not producer sites at all.
    for k in ("exclude_group_names", "cost_limit_min", "cost_limit_max",
              "heart_total_limit", "group_names", "exclude_cards",
              "exclude_names"):
        print("  %-22s cost=%d effect=%d total=%d"
              % (k, ck[k], ek[k], ck[k] + ek[k]))

    print("\n=== all-hand-discard cost nodes (pay_cost_all:discard_all)")
    hits = []
    for i, a in enumerate(ab):
        c = a.get("cost") or {}
        if "手札をすべて" in json.dumps(c, ensure_ascii=False):
            hits.append((i, a["cards"][:2], c))
    print("  count = %d" % len(hits))
    for i, cards, c in hits:
        print("   #%d %s" % (i, cards))
        print("      %s" % json.dumps(c, ensure_ascii=False)[:260])

    print("\n=== 'rest'/'rested' state_change anywhere")
    n = 0
    for a in ab:
        blob = json.dumps(a, ensure_ascii=False)
        if '"rest"' in blob or '"rested"' in blob:
            n += 1
    print("  nodes mentioning rest/rested = %d (0 => the C's rest arms are "
          "unreachable on this corpus)" % n)


if __name__ == "__main__":
    main()