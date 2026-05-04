"""
validate_consistency.py — Search-only diagnostic for abilities.json.

Reads abilities.json, scans for patterns that indicate real parser bugs
or data issues. Reports findings clearly. Never modifies anything.

Modes (run with no args for all):
  --missing-fields   : text evidence exists but JSON field absent
  --action-check     : action type doesn't match text verbs
  --trigger-check    : trigger icons not matching triggers field
  --condition-check  : condition markers without condition in JSON
  --source-check     : source/destination anomalies
"""

import json
import re
import sys
from pathlib import Path
from collections import defaultdict


def load(path=None):
    p = Path(path) if path else Path(__file__).parent.parent / "abilities.json"
    with open(p, encoding="utf-8") as f:
        return json.load(f)["unique_abilities"]


def _get_cost_effect_texts(ab):
    """Split triggerless_text into cost_text and effect_text."""
    text = ab.get("triggerless_text", "") or ab.get("full_text", "")
    if "：" in text:
        parts = text.split("：", 1)
        return parts[0].strip(), parts[1].strip()
    return "", text


# ── Check 1: Missing condition fields ────────────────────────────────────

def check_missing_conditions(abilities):
    """Find abilities with condition markers in effect text but no condition field."""
    findings = []
    for idx, ab in enumerate(abilities):
        _, effect_text = _get_cost_effect_texts(ab)
        eff = ab.get("effect") or {}
        has_cond_marker = any(m in effect_text for m in ("場合、", "とき、", "なら、"))
        has_cond_field = "condition" in eff
        if has_cond_marker and not has_cond_field:
            # Skip if condition exists in a nested structure
            nested = _has_nested_condition(eff)
            if not nested:
                # Extract which condition marker
                marker = next(m for m in ("場合、", "とき、", "なら、") if m in effect_text)
                cond_text = effect_text[:effect_text.find(marker) + len(marker)]
                findings.append({
                    "idx": idx,
                    "text": ab.get("full_text", "")[:120],
                    "condition_text": cond_text,
                    "marker": marker.strip("、"),
                    "action": eff.get("action", "?"),
                })
    return findings


def _has_nested_condition(obj, depth=0):
    if depth > 10 or not isinstance(obj, dict):
        return False
    if obj.get("condition"):
        return True
    for key in ("actions", "options"):
        for item in obj.get(key, []):
            if _has_nested_condition(item, depth + 1):
                return True
    for key in ("primary_effect", "alternative_effect"):
        sub = obj.get(key)
        if isinstance(sub, dict) and _has_nested_condition(sub, depth + 1):
            return True
    return False


# ── Check 2: Action type mismatches ──────────────────────────────────────

_ACTION_VERBS = {
    "move_cards": ["置く", "加える", "送る", "戻す", "移す"],
    "draw_card": ["引く", "引き"],
    "gain_resource": ["得る", "得て"],
    "gain_ability": ["能力を得る", "能力を得て"],
    "change_state": ["ウェイトにする", "アクティブにする"],
    "look_at": ["見る", "見て"],
    "reveal": ["公開する", "公開し"],
    "shuffle": ["シャッフル"],
    "modify_score": ["スコア", "プラス", "マイナス"],
    "modify_cost": ["コストが減る", "コストは減る"],
    "position_change": ["ポジションチェンジ", "入れ替える"],
    "restriction": ["できない", "なならない"],
    "invalidate_ability": ["無効にする"],
}


def check_action_mismatches(abilities):
    """Find actions that don't align with text verbs (likely bugs)."""
    findings = []
    for idx, ab in enumerate(abilities):
        _, effect_text = _get_cost_effect_texts(ab)
        eff = ab.get("effect") or {}
        action = eff.get("action", "")

        if not action or action in ("custom", "null", "do_nothing"):
            continue

        # For each verb expected by this action, check if any appear in text
        expected_verbs = _ACTION_VERBS.get(action, [])
        if not expected_verbs:
            continue

        has_expected_verb = any(v in effect_text for v in expected_verbs)

        # If action is a concrete type but no expected verb found, flag it
        if not has_expected_verb and action not in ("sequential", "look_and_select", "choice", "conditional_alternative"):
            # Check if there's a verb from another action type instead
            for other_action, verbs in _ACTION_VERBS.items():
                if other_action == action:
                    continue
                for v in verbs:
                    if v in effect_text:
                        findings.append({
                            "idx": idx,
                            "text": ab.get("full_text", "")[:120],
                            "json_action": action,
                            "text_verb": v,
                            "suggested": other_action,
                        })
                        break
                else:
                    continue
                break

    return findings


# ── Check 3: Custom action detection ────────────────────────────────────

def _find_custom_actions(obj, path=""):
    """Recursively find action: 'custom' in nested effect structures."""
    findings = []
    if isinstance(obj, dict):
        if obj.get("action") == "custom" and obj.get("text", "").strip():
            findings.append((path, obj.get("text", "")[:80]))
        for key in ("actions", "options"):
            for i, item in enumerate(obj.get(key, [])):
                findings += _find_custom_actions(item, f"{path}.{key}[{i}]")
        for key in ("look_action", "select_action", "primary_effect", "alternative_effect"):
            sub = obj.get(key)
            if isinstance(sub, dict):
                findings += _find_custom_actions(sub, f"{path}.{key}")
    return findings


def check_custom_actions(abilities):
    """Find abilities with unresolved 'custom' actions."""
    findings = []
    for idx, ab in enumerate(abilities):
        eff = ab.get("effect") or {}
        customs = _find_custom_actions(eff)
        if customs:
            findings.append({
                "idx": idx,
                "count": len(customs),
                "locations": customs,
                "text": ab.get("full_text", "")[:120],
            })
    return findings


# ── Check 4: Trigger inconsistencies ─────────────────────────────────────

_NON_TRIGGER_ICONS = ("icon_", "heart_", "turn", "center")

def check_triggers(abilities):
    """Find trigger icons in full_text that don't appear in triggers field."""
    findings = []
    for idx, ab in enumerate(abilities):
        full = ab.get("full_text", "")
        triggers_str = ab.get("triggers") or ""
        actual_triggers = set(t.strip() for t in triggers_str.split(",") if t.strip())

        expected = set()
        for m in re.finditer(r"\{\{([^|}]+)\|([^}]+)\}\}", full):
            icon = m.group(1)
            label = m.group(2).split("/")[0].strip()
            if any(icon.startswith(p) for p in _NON_TRIGGER_ICONS):
                continue
            expected.add(label)

        missing = expected - actual_triggers
        if missing:
            # Check if icon is inside a gained ability text (「...」)
            for m in list(missing):
                # If the trigger label appears inside quoted text, it's a
                # gained ability's trigger, not this ability's trigger
                quoted = re.findall(r"「([^」]+)」", full)
                for q in quoted:
                    if m in q or any(f"{{{{{icon}" in q for icon, _ in re.findall(r"\{\{([^|}]+)\|([^}]+)\}\}", full)):
                        missing.discard(m)
                        break
            if missing:
                findings.append({
                    "idx": idx,
                    "text": ab.get("full_text", "")[:120],
                    "missing_triggers": sorted(missing),
                    "actual_triggers": sorted(actual_triggers) if actual_triggers else [],
                })
    return findings


# ── Check 4: Missing fields that should be present ───────────────────────

def check_missing_source_dest(abilities):
    """Find move_cards actions missing source or destination."""
    findings = []
    for idx, ab in enumerate(abilities):
        eff = ab.get("effect") or {}
        if eff.get("action") not in ("move_cards",):
            continue
        # Only flag if text evidence exists for the missing field
        text = eff.get("text", "")
        src = eff.get("source")
        dst = eff.get("destination")
        if not src and ("から" in text or "にある" in text):
            findings.append({
                "idx": idx,
                "field": "source",
                "text": text[:80],
            })
        if not dst and ("に置く" in text or "に加える" in text or "に送る" in text or "に戻す" in text):
            findings.append({
                "idx": idx,
                "field": "destination",
                "text": text[:80],
            })
    return findings


def check_missing_optional(abilities):
    """Find effects with optional markers but no optional flag."""
    findings = []
    for idx, ab in enumerate(abilities):
        _, effect_text = _get_cost_effect_texts(ab)
        eff = ab.get("effect") or {}
        if "もよい" in effect_text or "てもよい" in effect_text:
            if not eff.get("optional"):
                findings.append({
                    "idx": idx,
                    "text": effect_text[:100],
                    "action": eff.get("action", "?"),
                })
    return findings


def check_missing_max(abilities):
    """Find effects with まで limit but no max flag."""
    findings = []
    for idx, ab in enumerate(abilities):
        _, effect_text = _get_cost_effect_texts(ab)
        eff = ab.get("effect") or {}
        if "人まで" in effect_text or "枚まで" in effect_text:
            if not eff.get("max"):
                findings.append({
                    "idx": idx,
                    "text": effect_text[:100],
                    "action": eff.get("action", "?"),
                })
    return findings


def check_missing_per_unit(abilities):
    """Find effects with につき marker but no per_unit flag."""
    findings = []
    for idx, ab in enumerate(abilities):
        _, effect_text = _get_cost_effect_texts(ab)
        eff = ab.get("effect") or {}
        if "につき" in effect_text or "ごとに" in effect_text:
            if not eff.get("per_unit"):
                # Skip cost modification patterns (handled differently)
                if "コスト" in effect_text and "減る" in effect_text:
                    continue
                findings.append({
                    "idx": idx,
                    "text": effect_text[:100],
                    "action": eff.get("action", "?"),
                })
    return findings


# ── Main ─────────────────────────────────────────────────────────────────

def main():
    args = set(sys.argv[1:])
    run_all = not any(a.startswith("--") for a in sys.argv[1:])

    abilities = load()

    checks = []

    if run_all or "--condition-check" in args:
        checks.append(("Missing condition fields", check_missing_conditions(abilities)))
    if run_all or "--action-check" in args:
        checks.append(("Action type mismatches", check_action_mismatches(abilities)))
    if run_all or "--custom-check" in args:
        checks.append(("Unresolved 'custom' actions", check_custom_actions(abilities)))
    if run_all or "--trigger-check" in args:
        checks.append(("Trigger inconsistencies", check_triggers(abilities)))
    if run_all or "--source-check" in args:
        checks.append(("Missing source/destination", check_missing_source_dest(abilities)))
    if run_all or "--missing-fields" in args:
        checks.append(("Missing optional flag", check_missing_optional(abilities)))
        checks.append(("Missing max flag", check_missing_max(abilities)))
        checks.append(("Missing per_unit flag", check_missing_per_unit(abilities)))

    total = 0
    for title, findings in checks:
        total += len(findings)
        print(f"\n  {'='*60}")
        print(f"  {title} ({len(findings)})")
        print(f"  {'='*60}")
        for f in findings[:15]:
            print(f"  #{f['idx']}: ", end="")
            if "json_action" in f:
                print(f"action={f['json_action']}, text has '{f['text_verb']}'")
                print(f"    Text: {f['text'][:100]}")
            elif "missing_triggers" in f:
                print(f"triggers field: {','.join(f['actual_triggers'])}")
                print(f"    missing from full_text: {','.join(f['missing_triggers'])}")
            elif "field" in f:
                print(f"missing {f['field']}")
                print(f"    Text: {f['text'][:100]}")
            elif "condition_text" in f:
                print(f"action={f['action']}, marker='{f['marker']}'")
                print(f"    condition: {f['condition_text'][:80]}")
                print(f"    Text: {f['text'][:100]}")
            elif "action" in f:
                print(f"action={f['action']}")
                print(f"    Text: {f['text'][:100]}")
            elif "count" in f:
                print(f"custom action found in {f['count']} location(s)")
                for loc, txt in f["locations"][:3]:
                    print(f"    {loc}: \"{txt}...\"")
                print(f"    Text: {f['text'][:100]}")
        if len(findings) > 15:
            print(f"  ... and {len(findings)-15} more")

    print(f"\n  {'='*60}")
    print(f"  Total: {total} findings across {len(checks)} check(s)")
    print(f"  {'='*60}\n")
    return 1 if total else 0


if __name__ == "__main__":
    sys.exit(main())
