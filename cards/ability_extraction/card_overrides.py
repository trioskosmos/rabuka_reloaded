#!/usr/bin/env python3
"""Data-driven per-card overrides.

Every card-ID-specific patch lives here instead of being scattered through
parser.py. Each override is an entry in OVERRIDES with:

  name       unique id for logging
  cards      substring match against the ability's `cards` entries
  ab_index   optional "(ab#N)" filter applied together with `cards`
  text_any   all of these substrings must appear in triggerless_text
  pred       optional extra predicate(ability) -> bool
  apply      fn(ability, ctx) -> bool  (ctx has triggerless_text/fix_stats)

apply_card_overrides(data, fix_stats) runs every override over
data["unique_abilities"] in one pass. Add new card-specific fixes here,
never in parser.py.
"""

import re
from typing import Any, Dict

from parser import (
    categorize_quoted_text,
    extract_all_quoted_names,
    parse_action,
    parse_condition,
    parse_effect,
)


def _cards_match(ability, substrings, ab_index=None):
    cards = ability.get("cards", [])
    for c in cards:
        if all(s in c for s in substrings) and (
            ab_index is None or f"(ab#{ab_index})" in c
        ):
            return True
    return False


def _text_match(ability, needles):
    tt = ability.get("triggerless_text", "")
    return all(n in tt for n in needles)


# ------------------------------------------------------------------
# LL-bp7-001 ab#0 play-cost template (LOAD-BEARING, not cosmetic).
#
# "このカードのプレイに際し、手札から「A」「B」「C」のメンバーカードを
# それぞれ1枚ずつ控え室に置いてもよい。そうしたとき、このカードのコストは
# 10になる。" is a PLAY-TIME cost modifier, not a triggered ability. The
# generic parser reads it as conditional_on_optional and loses the contract
# the engine expects: modify_cost(set) + location hand + characters +
# optional, which modifiers.rs/phases.rs route as a play-time cost hook.
# A general handler for 「プレイに際し…コストはNになる」 should eventually
# replace this override.
# ------------------------------------------------------------------
_LL_BP7_001_EFFECT = {
    "text": None,  # filled from triggerless_text at apply time
    "action": "modify_cost",
    "operation": "set",
    "value": 10,
    "source": "hand",
    "location": "hand",
    "card_type": "member_card",
    "characters": ["国木田花丸", "優木せつ菜", "嵐千砂都"],
    "count": 3,
    "optional": True,
}
_LL_BP7_001_COST = {
    "text": "手札から「国木田花丸」と「優木せつ菜」と「嵐千砂都」のメンバーカードをそれぞれ1枚ずつ控え室に置く",
    "type": "move_cards",
    "source": "hand",
    "zone": "hand",
    "destination": "discard",
    "card_type": "member_card",
    "characters": ["国木田花丸", "優木せつ菜", "嵐千砂都"],
    "count": 1,
    "per_character": True,
    "optional": True,
}


def _apply_ll_bp7_001(ability, ctx):
    import copy

    eff = copy.deepcopy(_LL_BP7_001_EFFECT)
    eff["text"] = ctx["triggerless_text"]
    ability["effect"] = eff
    ability["cost"] = copy.deepcopy(_LL_BP7_001_COST)
    return True


# ------------------------------------------------------------------
# PL!N-bp7-029-L Burn!!: parser mis-labels the under_member→energy_zone move
# as place_energy_under_member (which is for placing *under*). Correct to
# move_cards.
# ------------------------------------------------------------------
def _apply_burn_under_move(ability, ctx):
    eff = ability.get("effect")
    if not isinstance(eff, dict) or eff.get("action") != "conditional_on_result":
        return False
    prim = eff.get("primary_effect")
    if not (isinstance(prim, dict) and prim.get("source") == "under_member"):
        return False
    if prim.get("action") == "place_energy_under_member":
        prim["action"] = "move_cards"
    prim["source"] = "under_member"
    prim["destination"] = "energy_zone"
    # energy cards, all, wait state should remain
    prim.setdefault("card_type", "energy_card")
    prim.setdefault("all", True)
    return True


# ------------------------------------------------------------------
# PL!S-bp2-008 (小原鞠莉) ab#1 — constant conditional_alternative is
# actually a gain_ability. _try_sequential splices the text before
# parse_action's gain_ability fallback runs, so we fix it post-hoc.
# ------------------------------------------------------------------
def _apply_mari_gain_ability(ability, ctx):
    eff = ability.get("effect")
    if not isinstance(eff, dict):
        return False
    if eff.get("action") not in ("sequential", "conditional_alternative"):
        return False
    tt = ctx["triggerless_text"]
    if "得る" not in tt:
        return False
    q = extract_all_quoted_names(tt)
    if not q:
        return False
    cat = categorize_quoted_text(q)
    if not cat["abilities"]:
        return False
    fixed = parse_action(tt)
    if fixed.get("action") != "gain_ability":
        return False
    # parse_action's gain_ability early return skips condition extraction.
    # Re-extract condition from the triggerless text (scan for 場合、 etc.)
    if not fixed.get("condition"):
        for sep in ["とき、", "場合、", "たび、", "なら、"]:
            idx = tt.find(sep)
            if idx >= 0:
                ct = tt[: idx + 2]
                tc = parse_condition(ct)
                if tc and tc.get("type") not in (None, "custom"):
                    fixed["condition"] = tc
                    break
    ability["effect"] = fixed
    ctx["fix_stats"]["leak"] = ctx["fix_stats"].get("leak", 0) + 1
    # Re-run enrichment: gained_effect from ability_gain text
    if "gained_effect" not in fixed and fixed.get("ability_gain"):
        clean_gain = re.sub(r"【[^】]+】", "", fixed["ability_gain"]).strip()
        gained = parse_effect(clean_gain)
        if gained and gained.get("action") and gained.get("action") != "custom":
            fixed["gained_effect"] = gained
    return True


OVERRIDES: list = [
    {
        "name": "mari_gain_ability",
        "cards": ["S-bp2-008"],
        "ab_index": 1,
        "pred": _apply_mari_gain_ability,
    },
    {
        "name": "ll_bp7_001_play_cost",
        "cards": ["LL-bp7-001"],
        "ab_index": 0,
        "text_any": ["プレイに際し", "コストは10になる"],
        "pred": _apply_ll_bp7_001,
    },
    {
        "name": "burn_under_move",
        "cards": ["N-bp7-029-L"],
        "pred": _apply_burn_under_move,
    },
]


def apply_card_overrides(data: Dict[str, Any], fix_stats: Dict[str, int]) -> None:
    """Run every card-specific override over all unique abilities."""
    for ability in data.get("unique_abilities", []):
        ctx = {
            "triggerless_text": ability.get("triggerless_text", ""),
            "fix_stats": fix_stats,
        }
        for ov in OVERRIDES:
            if not _cards_match(ability, ov["cards"], ov.get("ab_index")):
                continue
            if ov.get("text_any") and not _text_match(ability, ov["text_any"]):
                continue
            if ov.get("pred")(ability, ctx):
                fix_stats.setdefault("overrides", {})
                fix_stats["overrides"][ov["name"]] = (
                    fix_stats["overrides"].get(ov["name"], 0) + 1
                )
