#!/usr/bin/env python3
"""
Extract card abilities from cards.json.
Splits abilities by newline and extracts triggers.

This script generates: data/abilities_extracted_from_cards.json
Source: data/cards.json
"""

import json
import logging
import re
import sys
import io
from pathlib import Path
from datetime import datetime

logging.basicConfig(
    level=logging.WARNING,
    format="%(levelname)s: %(message)s",
)
from collections import defaultdict

# Ensure stdout can handle Unicode (cp932 is the default on Windows Japanese)
try:
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
except (AttributeError, io.UnsupportedOperation):
    pass


def _encode_safe(text):
    """Encode text safely for the terminal encoding."""
    enc = sys.stdout.encoding or "utf-8"
    return text.encode(enc, errors="replace").decode(enc, errors="replace")


# Add parent directory to path for imports
sys.path.append(str(Path(__file__).parent.parent))

from parser import (
    parse_cost,
    parse_effect,
    extract_phase_gate,
    split_cost_effect,
    _normalize_effect_tree,
    _enrich_effect_type,
    _validate_semantic,
)

TRIGGER_PATTERN = re.compile(r"\{\{([^|]+)\|([^}]+)\}\}")
# Also match patterns with / prefix like /{{...}}
SLASH_TRIGGER_PATTERN = re.compile(r"/\{\{([^|]+)\|([^}]+)\}\}")


def extract_trigger(text: str) -> tuple[list, int | None, str]:
    """Extract triggers and use limits from ability text and return (triggers, use_limit, effect)."""
    # Cost icon patterns to exclude from triggers.
    # IMPORTANT: Only filter as cost AFTER a trigger has been found.
    # If at the very start of text, treat as position/activation requirement,
    # not as a cost (e.g. {{live_start.png|ライブ開始時}}{{center.png|センター}}).
    cost_icon_patterns = [
        "icon_energy",
        "heart",
        "icon_blade",
        "icon_b_all",
        "icon_score",
    ]

    # Position icons that should NOT prevent trigger extraction.
    # These appear after trigger icons like {{live_start.png|ライブ開始時}}{{center.png|センター}}.
    # They are NOT costs -- they are activation position requirements.
    position_icon_patterns = ["center", "left", "right"]

    # Known trigger icon patterns (for debugging/validation)
    trigger_icon_patterns = [
        "kidou",
        "jidou",
        "toujyou",
        "live_start",
        "live_success",
        "live_end",
        "turn",
        "center",  # center can be both cost, trigger, and position requirement
    ]

    # Use limit patterns (turn restrictions)
    use_limit_patterns = ["turn", "ターン"]

    triggers = []
    use_limit = None
    # Strip leading/trailing quote chars that may be JSON artifacts
    text = text.strip().lstrip('"').lstrip("\u201c").lstrip("\u201d").strip()
    effect = text

    # Handle ［ターン1回］ bracket format (non-icon turn limit notation)
    bracket_turn_match = re.search(r"［ターン1回］", effect)
    if bracket_turn_match:
        use_limit = 1
        effect = effect.replace("［ターン1回］", "", 1).strip()

    # First, remove / prefix trigger patterns
    slash_matches = SLASH_TRIGGER_PATTERN.findall(text)
    for match in slash_matches:
        icon_file = match[0]
        icon_text = match[1]
        slash_pattern = f"/{{{{{icon_file}|{icon_text}}}}}"
        effect = effect.replace(slash_pattern, "", 1)
        triggers.append(icon_text)

    # Find all trigger patterns
    trigger_matches = TRIGGER_PATTERN.findall(text)

    # Only consider triggers at the very start (before any non-trigger, non-whitespace text)
    pos = 0
    for match in trigger_matches:
        icon_file = match[0]
        icon_text = match[1]
        match_start = text.find(f"{{{{{icon_file}|{icon_text}}}}}", pos)

        # Check if there's any non-trigger text before this match
        before = text[pos:match_start]
        if before.strip() and before.strip() != "：":
            # If the only non-trigger text is a slash prefix (from /{{trigger}}), skip it
            if before.strip() == "/":
                pos = match_start + len(f"{{{{{icon_file}|{icon_text}}}}}")
                continue
            # Found non-trigger text, stop here
            break

        # Check if this is a cost icon (not a trigger).
        # BUT: if we already found a trigger, don't skip subsequent position icons
        # like {{center.png|センター}} -- they are activation position requirements,
        # not costs. Only skip actual cost resources (energy, heart, blade, score).
        if any(cost_pattern in icon_file for cost_pattern in cost_icon_patterns):
            pos = match_start + len(f"{{{{{icon_file}|{icon_text}}}}}")
            continue

        # Position icons (center, left, right) at the start should be skipped
        # only if no trigger has been found yet AND no other trigger icon precedes.
        # If they appear after a trigger icon, they're position requirements, not costs.
        if triggers and any(p in icon_file for p in position_icon_patterns):
            pos = match_start + len(f"{{{{{icon_file}|{icon_text}}}}}")
            continue

        # Check if this is a use limit (turn restriction)
        if any(
            use_limit_pattern in icon_text for use_limit_pattern in use_limit_patterns
        ):
            use_limit_text = icon_text
            # Convert Japanese turn limit text to integer
            if use_limit_text == "ターン1回":
                use_limit = 1
            elif use_limit_text == "ターン2回":
                use_limit = 2
            elif use_limit_text == "ターン3回":
                use_limit = 3
            else:
                # Try to extract any number from text like "ターンN回"
                num_match = re.match(r"ターン(\d+)回", use_limit_text)
                use_limit = int(num_match.group(1)) if num_match else use_limit_text
            # Remove use limit from effect
            trigger_pattern = f"{{{{{icon_file}|{icon_text}}}}}"
            effect = effect.replace(trigger_pattern, "", 1)
            pos = match_start + len(trigger_pattern)
            continue

        # Check if we're inside quoted text
        # Count quotes before this position
        quote_count = text[:match_start].count("「") - text[:match_start].count("」")
        if quote_count > 0:
            # We're inside quoted text, skip
            pos = match_start + len(f"{{{{{icon_file}|{icon_text}}}}}")
            continue

        # Skip if this trigger was already extracted (e.g. via slash prefix)
        if icon_text in triggers:
            pos = match_start + len(f"{{{{{icon_file}|{icon_text}}}}}")
            continue
        triggers.append(icon_text)
        # Remove this trigger icon from effect
        trigger_pattern = f"{{{{{icon_file}|{icon_text}}}}}"
        effect = effect.replace(trigger_pattern, "", 1)
        pos = match_start + len(trigger_pattern)

    effect = effect.strip()

    return triggers, use_limit, effect


def extract_abilities_from_card(card_id: str, card: dict) -> list:
    """Extract all abilities from a single card."""
    abilities = []

    ability_text = card.get("ability", "")
    if not ability_text:
        return abilities

    # Split by newline for multiple abilities
    ability_lines = ability_text.split("\n")

    for i, line in enumerate(ability_lines):
        line = line.strip()
        if not line:
            continue

        # Check if this is a continuation line (starts with ・)
        if line.startswith("・"):
            # Append to previous ability
            if abilities:
                abilities[-1]["full_text"] += "\n" + line
                abilities[-1]["triggerless_text"] += "\n" + line
            continue

        # Check if this is a parenthetical note (wrapped in parentheses)
        # Standalone ALL blade substitution is a real 常時 effect and must not be dropped.
        if (line.startswith("(") and line.endswith(")")) or (
            line.startswith("（") and line.endswith("）")
        ):
            inner = line[1:-1].strip()
            if "として扱う" in inner and "ALLブレード" in inner:
                action_text = inner
                if "、" in inner and inner.startswith("必要ハートを確認する時"):
                    action_text = inner.split("、", 1)[1].strip()
                abilities.append(
                    {
                        "card_id": card_id,
                        "full_text": line,
                        "triggerless_text": action_text,
                        "use_limit": None,
                        "triggers": ["常時"],
                        "trigger_count": 1,
                        "ability_index": i,
                        "is_null": False,
                    }
                )
                continue
            if "1つにつき" in inner and "スコアの合計に" in inner and "加算" in inner and "スコア" in inner:
                abilities.append(
                    {
                        "card_id": card_id,
                        "full_text": line,
                        "triggerless_text": inner,
                        "use_limit": None,
                        "triggers": ["常時"],
                        "trigger_count": 1,
                        "ability_index": i,
                        "is_null": False,
                    }
                )
                continue
            if abilities:
                abilities[-1]["full_text"] += "\n" + line
                abilities[-1]["triggerless_text"] += "\n" + line
            else:
                abilities.append(
                    {
                        "card_id": card_id,
                        "full_text": line,
                        "triggerless_text": "",
                        "use_limit": None,
                        "triggers": [],
                        "trigger_count": 0,
                        "ability_index": i,
                        "is_null": True,
                    }
                )
            continue

        # Check if this line starts with a trigger pattern (new ability)
        # If it doesn't have a trigger but the previous ability had one, it might be a continuation
        triggers, use_limit, effect = extract_trigger(line)

        # Check if this is a continuation of a previous ability (no trigger, but previous had trigger)
        # This handles cases like "回答がチョコミントの場合、..." which are conditional outcomes
        if not triggers and abilities and abilities[-1]["trigger_count"] > 0:
            # Check if this looks like a conditional outcome (starts with "回答が" or similar patterns)
            if line.startswith("回答が") or line.startswith("場合") or "の場合" in line:
                # Append to previous ability
                abilities[-1]["full_text"] += "\n" + line
                abilities[-1]["triggerless_text"] += "\n" + line
                continue
            # Check if this is a fragment continuation (starts with "とき、" or similar)
            # This handles cases where abilities are split incorrectly across newlines
            if (
                line.startswith("とき、")
                or line.startswith("なら、")
                or line.startswith("場合、")
            ):
                # Append to previous ability
                abilities[-1]["full_text"] += line
                abilities[-1]["triggerless_text"] += line
                continue

        # If this line has no trigger at all, treat it as a note (is_null: True)
        # Only parse as regular ability if it has trigger brackets
        if not triggers:
            abilities.append(
                {
                    "card_id": card_id,
                    "full_text": line,
                    "triggerless_text": "",
                    "use_limit": None,
                    "triggers": [],
                    "trigger_count": 0,
                    "ability_index": i,
                    "is_null": True,
                }
            )
        else:
            abilities.append(
                {
                    "card_id": card_id,
                    "full_text": line,
                    "triggerless_text": effect,
                    "use_limit": use_limit,
                    "once_per_turn": use_limit == 1 if use_limit else False,
                    "triggers": triggers,
                    "trigger_count": len(triggers),
                    "ability_index": i,
                }
            )

    return abilities


# Japanese color → heartXX mapping (matches BladeColor enum order 0-5 = heart01-06)
_JP_HEART_MAP = {
    "\u6843": "01",  # 桃 → heart01
    "\u8d64": "02",  # 赤 → heart02
    "\u9ec4": "03",  # 黄 → heart03
    "\u7dd1": "04",  # 緑 → heart04
    "\u9752": "05",  # 青 → heart05
    "\u7d2b": "06",  # 紫 → heart06
}
_JP_HEART_PATTERN = (
    r"\uff3b([\u6843\u8d64\u9ec4\u7dd1\u9752\u7d2b])\u30cf\u30fc\u30c8\uff3d"
)
_JP_HEART_RE = re.compile(_JP_HEART_PATTERN)


def _normalize_heart_notation(text: str) -> str:
    """Replace ［色名ハート］ with {{heart_XX.png|heartXX}}."""

    def _repl(m):
        color = m.group(1)
        num = _JP_HEART_MAP.get(color)
        if num:
            return f"{{{{heart_{num}.png|heart{num}}}}}"
        return m.group(0)

    return _JP_HEART_RE.sub(_repl, text)


def extract_all_abilities(cards_file: Path) -> dict:
    """Extract all abilities from cards.json."""
    with open(cards_file, encoding="utf-8") as f:
        cards = json.load(f)

    all_abilities = []
    ability_groups = defaultdict(list)

    # Handle both dict and list formats
    if isinstance(cards, list):
        cards_dict = {card.get("card_no", str(i)): card for i, card in enumerate(cards)}
    else:
        cards_dict = cards

    for card_id, card in cards_dict.items():
        abilities = extract_abilities_from_card(card_id, card)
        for ability in abilities:
            ability["full_text"] = _normalize_heart_notation(ability["full_text"])
            ability["triggerless_text"] = _normalize_heart_notation(
                ability["triggerless_text"]
            )
            all_abilities.append(ability)
            card_example = (
                f"{card_id} | {card.get('name', '')} (ab#{ability['ability_index']})"
            )
            ability_groups[ability["full_text"]].append(card_example)

    # Group abilities by full_text
    unique_abilities = []
    for full_text, card_examples in ability_groups.items():
        sample = next(a for a in all_abilities if a["full_text"] == full_text)

        # Parse semantic effect and cost
        effect_text = sample["triggerless_text"]

        # Extract phase gate as its own condition before splitting
        phase_gate, remaining_text = extract_phase_gate(effect_text)

        # Skip parsing for is_null abilities (notes without triggers)
        if sample.get("is_null", False):
            unique_abilities.append(
                {
                    "full_text": full_text,
                    "triggerless_text": sample["triggerless_text"],
                    "card_count": len(card_examples),
                    "cards": card_examples,
                    "triggers": ", ".join(sample["triggers"])
                    if sample["triggers"]
                    else None,
                    "use_limit": sample["use_limit"],
                    "is_null": True,
                    "cost": None,
                    "effect": None,
                }
            )
            continue

        # Split cost and effect
        cost_text = None
        if "：" in remaining_text:
            c_text, e_text = split_cost_effect(remaining_text)
            if c_text:
                cost_text = c_text
                effect_text = e_text
            else:
                effect_text = remaining_text
        else:
            effect_text = remaining_text

        # Parse cost
        cost = None
        if cost_text:
            try:
                cost = parse_cost(cost_text)
            except Exception as e:
                print(f"WARNING: parse_cost failed for '{cost_text}': {e}")
                cost = None

        # Parse effect
        effect = {}
        try:
            effect = parse_effect(effect_text)
            # Run post-processing normalizer (propagates exclude_self, distinct, position, original_value, etc.)
            effect = _normalize_effect_tree(effect, sample["triggerless_text"])
            # Check if effect has empty actions array
            if (
                isinstance(effect, dict)
                and "actions" in effect
                and not effect.get("actions")
            ):
                print(f"Warning: Effect parsed with empty actions: {effect_text[:100]}")
                print(f"Effect dict: {effect}")
            _enrich_effect_type(effect, triggerless=sample["triggerless_text"])

        except Exception as e:
            print(f"Error parsing effect: {effect_text}")
            print(f"Exception: {e}")
            import traceback

            traceback.print_exc()
            effect = {"text": effect_text, "actions": []}

        # If the effect handler embedded a cost (e.g. "unless pay N energy"),
        # lift it to the ability level (Q92: player chooses whether to pay)
        if isinstance(effect, dict) and "cost" in effect:
            cost = effect.pop("cost")

        ability_entry = {
            "full_text": full_text,
            "triggerless_text": sample["triggerless_text"],
            "card_count": len(card_examples),
            "cards": card_examples,
            "triggers": ", ".join(sample["triggers"]) if sample["triggers"] else None,
            "use_limit": sample["use_limit"],
            "is_null": sample.get("is_null", False),
            "cost": cost,
            "effect": effect,
        }
        # Merge phase gate into effect["condition"] (not ability_entry["condition"])
        # so the Rust Ability struct picks it up via AbilityEffect.condition.
        if phase_gate and isinstance(effect, dict):
            existing_cond = effect.get("condition")
            if existing_cond and isinstance(existing_cond, dict):
                effect["condition"] = {
                    "type": "compound",
                    "operator": "and",
                    "conditions": [phase_gate, existing_cond],
                }
            else:
                effect["condition"] = phase_gate
        unique_abilities.append(ability_entry)

    # Sort by card count
    unique_abilities.sort(key=lambda x: -x["card_count"])

    # Compute repository-relative source path
    try:
        repo_root = Path(__file__).parent.parent.parent
        rel_source = str(cards_file.relative_to(repo_root))
    except ValueError:
        rel_source = str(cards_file)

    # Get git commit hash for reproducibility tracking
    git_hash = "unknown"
    try:
        import subprocess

        result = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            capture_output=True,
            text=True,
            cwd=str(Path(__file__).parent.parent.parent),
            timeout=5,
        )
        if result.returncode == 0:
            git_hash = result.stdout.strip()
    except Exception:
        pass

    return {
        "schema": "extracted_abilities.v1",
        "generated_at": datetime.now().isoformat(),
        "generated_by": "cards/ability_extraction/extract_card_abilities.py",
        "source_file": rel_source,
        "engine_commit": git_hash,
        "parser_version": "1.0",
        "input_hash": None,  # filled by caller if needed
        "statistics": {
            "total_cards": len(cards_dict),
            "cards_with_abilities": len(
                [c for c in cards_dict.values() if c.get("ability")]
            ),
            "total_abilities": len(all_abilities),
            "unique_abilities": len(unique_abilities),
        },
        "unique_abilities": unique_abilities,
    }


def test_parsing():
    test_ability = "{{kidou.png|起動}}このメンバーをステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える。"
    triggers, use_limit, effect = extract_trigger(test_ability)

    print("=== Test Parsing ===")
    print(f"Original: {test_ability}")
    print(f"Triggers: {triggers}")
    print(f"Use Limit: {use_limit}")
    print(f"Effect: {effect}")
    print()


def main():
    test_parsing()

    cards_file = Path(__file__).parent.parent / "cards.json"
    output_file = Path(__file__).parent.parent / "abilities.json"

    print(f"Extracting abilities from {cards_file}...")
    result = extract_all_abilities(cards_file)

    print(
        f"Found {result['statistics']['total_abilities']} abilities across {result['statistics']['cards_with_abilities']} cards"
    )
    print(f"Unique abilities: {result['statistics']['unique_abilities']}")

    # Post-process: infer actions, apply targeted fixes
    from parser import process_abilities

    result = process_abilities(result)

    with open(output_file, "w", encoding="utf-8") as f:
        json.dump(result, f, ensure_ascii=False, indent=2)

    print(f"Output written to {output_file}")

    # Run basic validation: check for known gap patterns
    _validate_semantic(result["unique_abilities"])
    print("Validation complete.")

    # Auto-regenerate bytecode so abilities_gen.rs stays in sync
    import subprocess, sys

    compile_script = Path(__file__).parent.parent / "compile_abilities.py"
    if compile_script.exists():
        bin_file = compile_script.parent / "build" / "abilities.bin"
        if bin_file.exists():
            bin_file.unlink()
        result = subprocess.run(
            [sys.executable, str(compile_script)],
            cwd=compile_script.parent,
        )
        if result.returncode == 0:
            print("Bytecode regenerated.")
            if bin_file.exists():
                bin_file.unlink()
            for line in (result.stdout or "").splitlines()[-5:]:
                print(f"  {line}")

    # Auto-regenerate the Rust decoders so a NEW field added to
    # engine/src/core/card.rs is picked up without a separate manual step.
    # These are generated FROM card.rs, so they only change when card.rs does.
    decoder_scripts = [
        compile_script.parent / "generate_effect_decoder.py",
        compile_script.parent / "generate_condition_decoder.py",
    ]
    for dec in decoder_scripts:
        if dec.exists():
            dr = subprocess.run([sys.executable, str(dec)], cwd=dec.parent)
            if dr.returncode != 0:
                print(f"WARNING: {dec.name} failed ({dr.returncode}); decoders may be stale.")




BRACKET_RE = re.compile(r"[『「]([^』」]+)[』」]")
FILTER_FIELDS = {
    "group_names",
    "exclude_group_names",
    "characters",
    "exclude_characters",
    "card_names",
}


def _walk_filters(obj):
    """Recursively walk JSON and return all (value, field_name) pairs from filter fields.
    Excludes 'text' keys since those are raw descriptions, not structured filters."""
    filters = set()
    if isinstance(obj, dict):
        for key, value in obj.items():
            if key == "text":
                continue
            if key in FILTER_FIELDS and isinstance(value, list):
                for item in value:
                    if isinstance(item, str):
                        filters.add((item, key))
            if isinstance(value, (dict, list)):
                filters.update(_walk_filters(value))
    elif isinstance(obj, list):
        for item in obj:
            if isinstance(item, (dict, list)):
                filters.update(_walk_filters(item))
    return filters


def _validate_group_filters(abilities):
    """Check that every bracketed 『X』 name in ability text has a corresponding
    group_names/characters filter somewhere in the JSON structure."""
    issues = []
    for a in abilities:
        text = a.get("full_text", "") + a.get("triggerless_text", "")
        bracketed = set(BRACKET_RE.findall(text))
        if not bracketed:
            continue

        # Normalize variants
        def variants(name):
            v = {name}
            if "!" in name:
                v.add(name.replace("!", "！"))
            if "！" in name:
                v.add(name.replace("！", "!"))
            if "µ" in name:
                v.add(name.replace("µ", "μ"))
            if "μ" in name:
                v.add(name.replace("μ", "µ"))
            return v

        filter_values = {fv for fv, _ in _walk_filters(a)}

        for name in bracketed:
            if "{{" in name:
                continue  # skip template/ability text, not a card/group name
            # Skip card names introduced with カード名が/カード名は — they're not group/character filters
            # Both 『』 and 「」 bracket styles are used in card texts
            card_name_ctx = re.search(
                rf"カード名(?:が|は)[『「]{re.escape(name)}[』」]", text
            )
            if card_name_ctx:
                continue
            if not (variants(name) & filter_values):
                card_list = a.get("cards", [])
                text_preview = a.get("full_text", "")[:80]
                issues.append(
                    f'  『{name}』 in "{text_preview}…" — {card_list[0] if card_list else "?"}'
                )
                break  # one report per ability

    if issues:
        print(
            f"\n  GROUP FILTER ISSUES ({len(issues)} abilities with missing group_names):"
        )
        for issue in issues:
            print(_encode_safe(issue))
    else:
        print("    All bracketed names have matching filter fields.")


if __name__ == "__main__":
    main()
