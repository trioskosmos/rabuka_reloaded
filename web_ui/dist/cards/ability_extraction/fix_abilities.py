"""
fix_abilities.py — Consolidated fix script for abilities.json.

Replaces: fix_empty_effects.py, fix_missing_action.py,
          fix_ability_issues.py, fix_missing_counts.py

Applies all known fixes in one pass:
  - Infer empty/null effects from text
  - Fix missing action fields
  - Fix missing count/dynamic_count
  - Fix missing source/destination in cost
  - Remap invalid action types
  - Infer missing targets, card_types, etc.

Supports --dry-run to preview and auto-creates .bak backup.
"""

import json
import sys
import shutil
from pathlib import Path
from datetime import datetime
from collections import defaultdict


ABILITIES_PATH = Path(__file__).parent.parent / "abilities.json"

# ── Action remapping ─────────────────────────────────────────────────────────
ACTION_MAPPINGS = {
    "set_heart_type": "gain_resource",
    "set_card_identity_all_regions": "set_card_identity",
    "modify_required_hearts_global": "custom",
    "activation_cost": "custom",
    "modify_required_hearts": "custom",
}

# Actions that should have some kind of count
COUNT_REQUIRED_ACTIONS = {
    "move_cards", "draw_card", "gain_resource", "change_state",
    "place_energy_under_member", "set_card_identity", "reveal",
    "look_and_select", "select", "discard_until_count", "draw_until_count",
}

# ── Text inference helpers ────────────────────────────────────────────────────

def _infer_action_from_text(text):
    text_n = text.strip()
    if "得る" in text_n and ("ハート" in text_n or "ブレード" in text_n):
        return "gain_resource"
    if "得る" in text_n and "能力" in text_n:
        return "gain_ability"
    if "引く" in text_n or "引き" in text_n or "引いてもよい" in text_n:
        return "draw_card"
    if "見る" in text_n or "見て" in text_n:
        return "look_at"
    if "置く" in text_n or "加える" in text_n or "戻す" in text_n or "送る" in text_n:
        return "move_cards"
    if "ウェイトにする" in text_n or "アクティブにする" in text_n:
        return "change_state"
    if "スコア" in text_n and ("+" in text_n or "プラス" in text_n or "-" in text_n or "マイナス" in text_n):
        return "modify_score"
    if "シャッフル" in text_n:
        return "shuffle"
    if "公開する" in text_n:
        return "reveal"
    if "選ぶ" in text_n:
        return "select"
    if "何もしない" in text_n:
        return "do_nothing"
    return "custom"


def _infer_source_from_text(text):
    if "手札" in text:
        return "hand"
    if "控え室" in text:
        return "discard"
    if "デッキ" in text or "山札" in text:
        return "deck"
    if "ステージ" in text:
        return "stage"
    if "エネルギー" in text:
        return "energy_zone"
    if "ライブカード" in text:
        return "live_card_zone"
    return None


def _infer_destination_from_text(text):
    if "手札に加える" in text:
        return "hand"
    if "控え室" in text:
        return "discard"
    if "デッキ" in text:
        return "deck"
    if "ステージ" in text:
        return "stage"
    if "エネルギー" in text:
        return "energy_zone"
    if "ライブカード" in text:
        return "live_card_zone"
    return None


def _infer_count_from_text(text):
    import re
    for pat in [r"(\d+)枚", r"(\d+)人", r"(\d+)つ", r"(\d+)回"]:
        m = re.search(pat, text)
        if m:
            return int(m.group(1))
    return None


def _count_resource_icons(text):
    heart = len(re.findall(r"{{heart_\d+\.png\|heart\d+}}", text))
    blade = text.count("{{icon_blade.png|ブレード}}")
    energy = text.count("{{icon_energy.png|E}}")
    return heart + blade + energy


def _infer_dynamic_count(obj):
    src = obj.get("source", "")
    if src == "looked_at_remaining":
        return {"type": "RemainingLookedAt"}
    if obj.get("any_number"):
        return {"type": "PlayerChoice"}
    if src in ("selected_cards", "revealed_cards"):
        return {"type": "RevealedCards"}
    return None


# ── Fix applicators ──────────────────────────────────────────────────────────

def fix_null_effect(ability, idx):
    """Fix abilities with null/empty effect."""
    eff = ability.get("effect")
    if eff and eff.get("action") not in (None, "", "null"):
        return []
    if ability.get("is_null"):
        ability["effect"] = {"action": "null", "type": "null", "text": ""}
        return [f"#{idx}: Set null effect for is_null ability"]

    text = ability.get("triggerless_text", "") or ability.get("full_text", "")
    action = _infer_action_from_text(text)
    ability["effect"] = {"action": action, "text": text}
    if action == "gain_resource":
        if "{{icon_blade.png|ブレード}}" in text:
            ability["effect"]["resource"] = "blade"
        elif "ハート" in text:
            ability["effect"]["resource"] = "heart"
    return [f"#{idx}: Filled empty effect with action={action}"]


def fix_missing_action_in_effect(eff, ability_idx, path="effect", fixes=None):
    """Recursively fix missing action fields in effect and nested actions."""
    if fixes is None:
        fixes = []
    if not isinstance(eff, dict):
        return fixes

    action = eff.get("action")
    if not action or action == "":
        text = eff.get("text", "")
        inferred = _infer_action_from_text(text)
        eff["action"] = inferred
        fixes.append(f"#{ability_idx} {path}: Added missing action={inferred}")

    # Remap invalid actions
    if eff.get("action") in ACTION_MAPPINGS:
        old = eff["action"]
        eff["action"] = ACTION_MAPPINGS[old]
        fixes.append(f"#{ability_idx} {path}: Remapped action {old} -> {eff['action']}")

    # Fix missing count
    action_now = eff.get("action", "")
    cnt = eff.get("count")
    dyn = eff.get("dynamic_count")
    if cnt is None and dyn is None:
        if action_now in COUNT_REQUIRED_ACTIONS:
            # Try to infer from dynamic source
            dyn_inferred = _infer_dynamic_count(eff)
            if dyn_inferred:
                eff["dynamic_count"] = dyn_inferred
                fixes.append(f"#{ability_idx} {path}: Added dynamic_count ({dyn_inferred['type']})")
            else:
                # Try text inference
                text = eff.get("text", "")
                inferred_cnt = _infer_count_from_text(text)
                if inferred_cnt is not None:
                    eff["count"] = inferred_cnt
                    fixes.append(f"#{ability_idx} {path}: Added count={inferred_cnt} from text")
                elif action_now == "gain_resource":
                    ic = _count_resource_icons(text)
                    if ic:
                        eff["count"] = ic
                        fixes.append(f"#{ability_idx} {path}: Added count={ic} from resource icons")
                    else:
                        eff["count"] = 1
                        fixes.append(f"#{ability_idx} {path}: Added default count=1")
                else:
                    eff["count"] = 1
                    fixes.append(f"#{ability_idx} {path}: Added default count=1")

    # Fix missing source for move_cards
    if action_now == "move_cards" and not eff.get("source"):
        text = eff.get("text", "")
        src = _infer_source_from_text(text)
        if src:
            eff["source"] = src
            fixes.append(f"#{ability_idx} {path}: Added source={src}")

    # Fix missing destination for move_cards
    if action_now == "move_cards" and not eff.get("destination"):
        text = eff.get("text", "")
        dst = _infer_destination_from_text(text)
        if dst:
            eff["destination"] = dst
            fixes.append(f"#{ability_idx} {path}: Added destination={dst}")

    # Recurse into nested structures
    for key in ("actions", "look_action", "select_action"):
        if key in eff:
            if isinstance(eff[key], dict):
                fix_missing_action_in_effect(eff[key], ability_idx, f"{path}.{key}", fixes)
            elif isinstance(eff[key], list):
                for i, sub in enumerate(eff[key]):
                    if isinstance(sub, dict):
                        fix_missing_action_in_effect(sub, ability_idx, f"{path}.{key}[{i}]", fixes)
    return fixes


def fix_cost(cost, ability_idx, fixes=None):
    """Fix missing fields in cost."""
    if fixes is None:
        fixes = []
    if not isinstance(cost, dict):
        return fixes
    if cost.get("type") == "move_cards":
        text = cost.get("text", "")
        if not cost.get("source"):
            src = _infer_source_from_text(text)
            if src:
                cost["source"] = src
                fixes.append(f"#{ability_idx} cost: Added source={src}")
        if not cost.get("destination"):
            dst = _infer_destination_from_text(text)
            if dst:
                cost["destination"] = dst
                fixes.append(f"#{ability_idx} cost: Added destination={dst}")
    return fixes


# ── Main ─────────────────────────────────────────────────────────────────────

def main():
    import argparse
    parser = argparse.ArgumentParser(description="Fix abilities.json issues")
    parser.add_argument("--dry-run", action="store_true", help="Preview changes without saving")
    parser.add_argument("--path", help="Path to abilities.json (default: ../abilities.json)")
    parser.add_argument("--no-backup", action="store_true", help="Skip backup creation")
    args = parser.parse_args()

    path = Path(args.path) if args.path else ABILITIES_PATH

    # Load
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)

    abilities = data["unique_abilities"]
    all_fixes = []

    # Backup
    if not args.dry_run and not args.no_backup:
        backup_path = path.with_suffix(".bak")
        shutil.copy2(path, backup_path)
        print(f"  Backup created: {backup_path}")

    # Fix each ability
    for idx, ability in enumerate(abilities):
        # 1. Empty/null effects
        all_fixes += fix_null_effect(ability, idx)

        # 2. Missing/blank actions in effect
        eff = ability.get("effect")
        if eff:
            all_fixes += fix_missing_action_in_effect(eff, idx)

        # 3. Cost fixes
        cost = ability.get("cost")
        if cost:
            all_fixes += fix_cost(cost, idx)

    # Report
    print(f"\n  {'='*60}")
    print(f"  FIX REPORT")
    print(f"  {'='*60}")
    print(f"  Total abilities: {len(abilities)}")
    print(f"  Total fixes: {len(all_fixes)}")

    if all_fixes:
        print(f"\n  Fixes ({len(all_fixes)}):")
        for fix in all_fixes:
            print(f"    - {fix}")

    # Save
    if not args.dry_run and all_fixes:
        with open(path, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
        print(f"\n  ✓ Saved to {path}")
    elif args.dry_run:
        print(f"\n  (dry-run — no changes saved)")
    else:
        print(f"\n  No changes needed.")

    return 1 if all_fixes else 0


if __name__ == "__main__":
    sys.exit(main())
