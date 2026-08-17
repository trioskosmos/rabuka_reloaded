#!/usr/bin/env python3
"""Generate engine/tests/TEST_COVERAGE.md from abilities.json + the Rust test suite.

Two coverage levels:
  * CARD level  : is THIS card's card_no referenced in any test .rs file?
  * MECHANIC level : is ANY test exercising a given trigger / effect action /
                     condition type? (even if a specific card is untested, a
                     shared action+condition pair still covers the engine path)

Rerun after changing the parser, card data, or tests:
    python cards/coverage_report.py
"""
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CARDS_JSON = ROOT / "cards" / "abilities.json"
TESTS_DIR = ROOT / "engine" / "tests"
OUT = ROOT / "engine" / "tests" / "TEST_COVERAGE.md"

RARITY_ACTION_LABEL = {
    "move_cards": "move card between zones",
    "draw_card": "draw",
    "gain_resource": "gain heart/blade/score resource",
    "modify_score": "modify live score",
    "change_state": "change active/wait state",
    "position_change": "position change / area move",
    "add_to_energy": "add/place energy",
    "shuffle": "shuffle deck",
    "sequential": "sequential compound effect",
    "reveal": "reveal cards",
    "look_and_select": "look at / select from deck",
    "baton_touch": "baton touch (play onto member)",
    "compare_score": "score comparison",
    "convert": "convert resource",
    "refresh": "refresh / re-deck",
}

TRIGGER_ORDER = ["登場", "自動", "起動", "常時", "ライブ開始時", "ライブ成功時"]


def load_abilities():
    with open(CARDS_JSON, encoding="utf-8") as f:
        return json.load(f)["unique_abilities"]


def collect_test_sources():
    """Return the concatenated text of every test .rs file under engine/tests."""
    texts = []
    for p in TESTS_DIR.rglob("*.rs"):
        try:
            texts.append(p.read_text(encoding="utf-8"))
        except Exception:
            pass
    return "\n".join(texts)


def card_base(card_no):
    """Strip the rarity suffix -> card identity (e.g. PL!N-bp7-011-R+ -> PL!N-bp7-011)."""
    m = re.match(r"^(.*?-\d+)", card_no)
    return m.group(1) if m else card_no


def card_set(card_no):
    """Return the set token, e.g. bp7 / sd2 / pb1 / cl1 / joint."""
    m = re.search(r"-(bp\d+|sd\d+|pb\d+|cl\d+|PR)-\d+", card_no)
    if m:
        return m.group(1)
    return "other"


def effect_action(eff):
    a = (eff or {}).get("action")
    if isinstance(a, str):
        return a
    # compound/sequential actions
    if isinstance(a, dict):
        return a.get("type") or "compound"
    return "unknown"


def condition_type(eff):
    c = (eff or {}).get("condition") or {}
    t = c.get("type")
    if not t:
        te = c.get("trigger_event") or {}
        if te.get("type"):
            return f"trigger:{te['type']}"
        return "none"
    if t == "movement_condition":
        te = c.get("trigger_event") or {}
        return f"movement:{te.get('type','?')}"
    return t


def human_action(a):
    return RARITY_ACTION_LABEL.get(a, a)


def main():
    abilities = load_abilities()
    src = collect_test_sources()
    n_files = sum(1 for _ in TESTS_DIR.rglob("*.rs"))

    # ---- per-ability coverage ----
    by_card = defaultdict(list)       # base -> [ability]
    untested = []                     # (base, card_no sample, trigger, action, cond, text)
    card_covered = {}                 # base -> bool
    trigger_counts = defaultdict(lambda: [0, 0])   # trigger -> [covered, total]
    action_counts = defaultdict(lambda: [0, 0])
    cond_counts = defaultdict(lambda: [0, 0])
    set_counts = defaultdict(lambda: [0, 0])

    for u in abilities:
        cards = [c.split(" | ")[0] for c in u.get("cards", [])]
        base = card_base(cards[0]) if cards else "?"
        trig = u.get("triggers") or ""
        eff = u.get("effect") or {}
        act = effect_action(eff)
        cond = condition_type(eff)
        sset = card_set(cards[0]) if cards else "?"

        covered = any(c in src for c in cards) or (base in src)
        card_covered[base] = card_covered.get(base, False) or covered

        for t in trig.split(","):
            t = t.strip()
            if t:
                trigger_counts[t][0] += 1 if covered else 0
                trigger_counts[t][1] += 1
        action_counts[act][0] += 1 if covered else 0
        action_counts[act][1] += 1
        cond_counts[cond][0] += 1 if covered else 0
        cond_counts[cond][1] += 1
        set_counts[sset][0] += 1 if covered else 0
        set_counts[sset][1] += 1

        if not covered:
            name = (cards[0] + " | " + (u.get("cards", [""])[0].split(" | ")[-1])) if cards else "?"
            untested.append((base, cards[0], trig, act, cond, (u.get("full_text") or "")[:110]))

    # ---- aggregate card stats ----
    total_cards = len(card_covered)
    covered_cards = sum(card_covered.values())
    abilities_on_covered_card = 0
    for u in abilities:
        if any(card_covered.get(card_base(c.split(" | ")[0]), False) for c in u.get("cards", [])):
            abilities_on_covered_card += 1

    # ---- mechanic-level: which actions/conditions/triggers have ANY test? ----
    # (already encoded in *_counts; a row is 'mechanically exercised' if covered>0)

    def pct(c, t):
        return f"{c}/{t}" if t else "0"

    lines = []
    w = lines.append
    w("# Test Coverage Report")
    w("")
    w(f"_Auto-generated by `cards/coverage_report.py` — do not edit by hand. Rerun it after changing the parser, card data, or tests._")
    w("")
    w("Covers the **real card database** (`cards/abilities.json`, "
      f"{len(abilities)} unique abilities) against the Rust suite "
      f"(`engine/tests`, {n_files} test files).")
    w("")
    w("## How to read this")
    w("")
    w("Two coverage levels — they answer different questions:")
    w("")
    w("| Level | Question | Meaning |")
    w("|---|---|---|")
    w("| **Card** | Is this card's `card_no` referenced in any test? | A test `game.id(\"PL!N-bp7-011-R+\")` shows up in the suite. Does NOT prove every ability on that card is exercised. |")
    w("| **Mechanic** | Is any test exercising this trigger / action / condition? | Even if a specific card is untested, another card sharing the same `action`+`condition` still covers that engine path. |")
    w("")
    w("A card with multiple abilities counts as **covered** if *any* of them is touched; check the per-ability gap list for finer detail.")
    w("")
    w(f"## Overall")
    w("")
    w(f"- **Unique abilities:** {len(abilities)}")
    w(f"- **Distinct cards (base identity):** {total_cards}")
    w(f"- **Cards referenced in tests:** {covered_cards} / {total_cards}  ({pct(covered_cards, total_cards)})")
    w(f"- **Abilities on a referenced card:** {abilities_on_covered_card} / {len(abilities)}")
    w("")

    def table(title, counts, order=None, fmt=lambda k: k):
        rows = sorted(counts.items(), key=lambda kv: (order.index(kv[0]) if order and kv[0] in order else 999, kv[0]))
        w(f"## {title}")
        w("")
        w("| Category | Covered | Total | % |")
        w("|---|---|---|---|")
        for k, (c, t) in rows:
            w(f"| {fmt(k)} | {c} | {t} | {pct(c,t)} |")
        w("")

    table("By trigger type", trigger_counts, TRIGGER_ORDER, lambda k: {"登場":"登場 (Debut)","自動":"自動 (Auto)","起動":"起動 (Activation)","常時":"常時 (Constant)","ライブ開始時":"ライブ開始時 (LiveStart)","ライブ成功時":"ライブ成功時 (LiveSuccess)"}.get(k,k))
    table("By effect action", action_counts, fmt=human_action)
    table("By condition type", cond_counts)
    table("By card set", set_counts)

    # ---- untested gap ----
    untested.sort(key=lambda r: (r[0], r[2]))
    w("## Untested abilities (gap)")
    w("")
    w("These abilities' cards are **not referenced by any test**. They are the "
      "highest-value targets for new tests. Grouped by trigger type.")
    by_trig = defaultdict(list)
    for r in untested:
        by_trig[r[2]].append(r)
    for t in sorted(by_trig.keys()):
        grp = by_trig[t]
        w(f"### {t}  ({len(grp)})")
        w("")
        w("| Card | Set | Action | Condition | Text |")
        w("|---|---|---|---|---|")
        for base, card, _tr, act, cond, text in sorted(grp, key=lambda r: r[0]):
            safe = text.replace("|", "/").replace("\n", " ")
            w(f"| `{card}` | {card_set(card)} | {act} | {cond} | {safe} |")
        w("")

    out_text = "\n".join(lines) + "\n"
    OUT.write_text(out_text, encoding="utf-8")
    print(f"Wrote {OUT.relative_to(ROOT)}")
    print(f"  abilities={len(abilities)} cards={total_cards} covered_cards={covered_cards} ({pct(covered_cards,total_cards)})")
    print(f"  untested abilities (referenced-card absent) = {len(untested)}")


if __name__ == "__main__":
    sys.exit(main())
