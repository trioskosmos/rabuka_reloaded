"""Condition parsing using a flat pattern registry."""

import re
from typing import Dict, Any, Optional

from test_parser.fields import (
    ExtractedFields,
    normalize_fullwidth_digits,
    strip_parenthetical,
)
from test_parser.schema import (
    POSITION_KEYWORDS,
    COMPARISON_TARGETS,
    COMPARISON_OPERATORS,
    COMPARISON_TYPES,
    LOCATION_PATTERNS,
    CARD_TYPE_PATTERNS,
    OPERATOR_PATTERNS,
)


def parse_condition(text: str) -> Optional[Dict[str, Any]]:
    """Parse a condition text. Returns parsed dict or None if no match."""
    text = normalize_fullwidth_digits(strip_parenthetical(text))
    f = ExtractedFields(text)

    for _, handler in _CONDITION_HANDLERS:
        result = handler(text, f)
        if result is not None:
            return result

    # Fallback: generic field extraction
    condition = {"text": text}
    _generic_fields(condition, text, f)
    return _infer_type(condition, text, f)


def _generic_fields(d: Dict[str, Any], text: str, f: ExtractedFields):
    d["text"] = text

    # Target
    tgt = f.target
    if tgt:
        d["target"] = tgt

    # Location
    loc = f.location
    if loc:
        d["location"] = loc

    # If revealed context, prefer revealed_cards
    if f.has_revealed_context:
        d["location"] = "revealed_cards"

    # Multiple locations via 'と'
    if f.locations:
        d["locations"] = f.locations

    # Card type
    ct = f.card_type
    if ct:
        d["card_type"] = ct

    # Count & operator
    cnt = f.count
    if cnt is not None:
        d["count"] = cnt
    op = f.operator
    if op:
        d["operator"] = op

    # Comparison
    if f.comparison_target:
        d["comparison_target"] = f.comparison_target
    if f.comparison_operator:
        d["operator"] = f.comparison_operator
    if f.comparison_type:
        d["comparison_type"] = f.comparison_type

    # Aggregate
    if f.aggregate_total:
        d["aggregate"] = "total"

    # Negation
    if f.negation:
        d["negation"] = True

    # Distinct
    if f.distinct:
        d["distinct"] = f.distinct

    # Exclude self
    if f.exclude_self:
        d["exclude_self"] = True

    # Group names
    if f.group_names:
        d["group_names"] = f.group_names

    # Heart colors
    if f.heart_colors:
        d["heart_colors"] = f.heart_colors

    # Cost limit
    if f.cost_limit is not None:
        d["cost_limit"] = f.cost_limit
        if f.cost_limit_operator:
            d["cost_limit_operator"] = f.cost_limit_operator

    # Position
    if f.position:
        d["position"] = f.position
    if f.source_position:
        d["source_position"] = f.source_position
    if f.exclude_position:
        d["exclude_position"] = f.exclude_position

    # Original value
    if f.original_value:
        d["original_value"] = True

    # All areas
    if "エリアすべて" in text:
        d["all_areas"] = True

    # Movement
    if "移動した" in text:
        d["movement"] = "moved"
    elif "移動する" in text:
        d["movement"] = "moves"
    if "移動している" in text:
        d["movement_state"] = "has_moved"

    # Temporal
    if "このターン" in text:
        d["temporal"] = "this_turn"
    elif "このライブ" in text:
        d["temporal"] = "this_live"

    # Resource type / energy context
    if "エネルギー" in text:
        d["resource_type"] = "energy"
    if "余剰ハート" in text:
        d["resource_type"] = "surplus_heart"

    # Same/equality/ちょうど
    if "ちょうど" in text or "同じ" in text:
        if "同じ" in text and d.get("comparison_type") != "score":
            d["comparison_type"] = "equality"
        d["operator"] = "="

    # Characters (「」 quoted names with group pattern)
    cm = re.search(r"((?:「[^」]+」か? ?)+)がいる", text)
    if cm:
        names = re.findall(r"「([^」]+)」", cm.group(1))
        if names:
            d["characters"] = names

    # Resource type from surplus heart
    if "余剰ハート" in text:
        d["resource_type"] = "surplus_heart"

    # Blade limit
    if f.blade_limit is not None:
        d["blade_limit"] = f.blade_limit
        if f.blade_limit_operator:
            d["blade_limit_operator"] = f.blade_limit_operator

    # Values (いずれか)
    if "いずれか" in text:
        vm = re.findall(r"\d+", text)
        if vm:
            d["values"] = [int(v) for v in vm]


def _infer_type(
    d: Dict[str, Any], text: str, f: ExtractedFields
) -> Optional[Dict[str, Any]]:
    if not text.strip():
        return None

    # comparison_target takes priority
    if d.get("comparison_target"):
        d["type"] = "comparison_condition"
        return d
    if d.get("comparison_type") and d.get("comparison_type") != "equality":
        d["type"] = "comparison_condition"
        return d
    if d.get("resource_type"):
        d["type"] = "comparison_condition"
        return d
    if d.get("locations") and d.get("card_type"):
        d.setdefault("count", 1)
        d.setdefault("operator", ">=")
        d.setdefault("target", "self")
        d["type"] = "card_count_condition"
        return d
    if d.get("group_names"):
        d["type"] = "group_condition"
        return d
    if d.get("location") and d.get("card_type"):
        d["type"] = "location_condition"
        return d
    if d.get("location") and d.get("position"):
        d["type"] = "position_condition"
        return d
    if d.get("operator") and d.get("target"):
        d["type"] = "comparison_condition"
        return d
    if d.get("aggregate") == "total":
        d["type"] = "comparison_condition"
        return d
    if d.get("location") and d.get("target"):
        d["type"] = "location_condition"
        return d
    if d.get("location") and d.get("operator"):
        d.setdefault("target", "self")
        d["type"] = "location_condition"
        return d
    if d.get("card_type"):
        d.setdefault("count", 1)
        d.setdefault("operator", ">=")
        d["type"] = "card_count_condition"
        return d
    if any(
        k in d
        for k in (
            "operator",
            "count",
            "location",
            "card_type",
            "target",
            "comparison_type",
            "group_names",
        )
    ):
        d.setdefault("count", 1)
        d.setdefault("operator", ">=")
        d.setdefault("target", "self")
        d["type"] = "comparison_condition"
        return d
    if text.strip():
        d["type"] = "custom"
        return d
    return None


# ===================== CONDITION HANDLER REGISTRY =====================

_CONDITION_HANDLERS = []


def register(priority: int = 0):
    """Decorator to register a condition handler with explicit priority (higher = first)."""

    def wrapper(func):
        _CONDITION_HANDLERS.append((priority, func))
        _CONDITION_HANDLERS.sort(key=lambda x: -x[0])
        return func

    return wrapper


@register(100)
def _try_complex(text, f):
    """これにより cause-effect relationships."""
    markers = ["これにより", "その結果"]
    for marker in markers:
        if marker in text:
            parts = text.split(marker, 1)
            if len(parts) == 2 and parts[0].strip() and not parts[1].startswith("場合"):
                cause = parse_condition(parts[0].strip())
                effect = parse_condition(parts[1].strip())
                if cause and effect:
                    return {
                        "type": "complex_condition",
                        "cause": cause,
                        "effect": effect,
                        "text": text,
                    }
    return None


@register(95)
def _try_compound(text, f):
    """かつ / あり、— compound AND conditions."""
    op = "かつ" if "かつ" in text else None
    if not op and "あり、" in text:
        op = "あり、"
    if not op:
        return None
    parts = [p.strip() for p in text.split(op) if p.strip()]
    if len(parts) < 2:
        return None
    parsed = [parse_condition(p) for p in parts]
    parsed = [p for p in parsed if p]
    if len(parsed) < 2:
        return None
    result = {"type": "compound", "operator": "and", "conditions": parsed, "text": text}
    if f.target:
        result["target"] = f.target
    if f.location:
        result["location"] = f.location
    if f.card_type:
        result["card_type"] = f.card_type
    if any("このカード" in c.get("text", "") for c in parsed):
        result["check_self"] = True
    # Propagate distinct from sub-conditions
    for sub in parsed:
        if sub.get("distinct"):
            result["distinct"] = sub["distinct"]
            break
    # Check for cost distinct
    if "コストがそれぞれ異なる" in text:
        result["distinct"] = "cost"
    elif any(kw in text for kw in ["名前が異なる", "名前の異なる", "カード名が異なる"]):
        result["distinct"] = "card_name"
    elif "グループ名が異なる" in text:
        result["distinct"] = "group_name"
    return result


@register(90)
def _try_distinct(text, f):
    """名前が異なる — distinct name condition."""
    if not any(
        kw in text
        for kw in [
            "名前が異なる",
            "名前の異なる",
            "ユニット名がそれぞれ異なる",
            "グループ名がそれぞれ異なる",
        ]
    ):
        return None
    dist_val = "card_name"
    if "ユニット名" in text or "グループ名" in text:
        dist_val = "group_name"
    result = {
        "type": "location_condition",
        "target": "self",
        "distinct": dist_val,
        "text": text,
    }
    if f.location:
        result["location"] = f.location
    else:
        result["location"] = "stage"
    if f.locations:
        result["locations"] = f.locations
    if "エリアすべて" in text:
        result["all_areas"] = True
    m = re.search(r"(\d+)(人|枚|つ)以上いる", text)
    if m:
        result["count"] = int(m.group(1))
        result["operator"] = ">="
        result["unit"] = m.group(2)
    if f.group_names:
        result["group_names"] = f.group_names
    return result


@register(87)
def _try_state_change(text, f):
    """アクティブ状態からウェイト状態になった — state change condition."""
    if "アクティブ状態からウェイト状態になった" not in text and not (
        "アクティブ状態" in text and "ウェイト状態" in text
    ):
        return None
    result = {"type": "state_change_condition", "text": text}
    if "ウェイト状態になった" in text or "アクティブ状態から" in text:
        result["from_state"] = "active"
        result["to_state"] = "wait"
    else:
        result["from_state"] = "wait"
        result["to_state"] = "active"
    if "メインフェイズの間" in text:
        result["phase"] = "main"
    if f.target:
        result["target"] = f.target
    if f.count is not None:
        result["count"] = f.count
        result["operator"] = f.operator or "="
        if f.unit:
            result["unit"] = f.unit
    return result


@register(85)
def _try_or(text, f):
    """あるか、— OR condition."""
    if "あるか、" not in text:
        return None
    parts = [p.strip() for p in text.split("あるか、") if p.strip()]
    if len(parts) < 2:
        return None
    parts = [p + "ある" if i < len(parts) - 1 else p for i, p in enumerate(parts)]
    parsed = [parse_condition(p) for p in parts]
    parsed = [p for p in parsed if p]
    if len(parsed) < 2:
        return None
    return {"type": "or_condition", "conditions": parsed, "text": text}


@register(83)
def _try_blade_count(text, f):
    """ブレードの数がNつ以上 — blade count condition."""
    clean = re.sub(r"\{\{.*?\|([^}]+)\}\}", r"\1", text)
    for pat, op in [
        (r"ブレードが(\d+)つ以上", ">="),
        (r"ブレードの数が(\d+)以上", ">="),
        (r"ブレードの数が(\d+)つ以上", ">="),
    ]:
        m = re.search(pat, clean)
        if m:
            return {
                "type": "card_blade_condition",
                "count": int(m.group(1)),
                "operator": op,
                "text": text,
                "source": "selected_cards",
            }
    return None


@register(80)
def _try_card_count(text, f):
    """N枚/人/つ以上 — card count condition."""
    matched = None
    for pat, op in [
        (r"(\d+)枚以上ある", ">="),
        (r"(\d+)つ以上ある", ">="),
        (r"(\d+)種類以上ある", ">="),
        (r"(\d+)枚ある", "="),
        (r"(\d+)枚以上", ">="),
        (r"(\d+)人以上いる", ">="),
        (r"(\d+)人以上", ">="),
    ]:
        m = re.search(pat, text)
        if m:
            matched = (int(m.group(1)), op)
            break
    if not matched and re.search(r"(\d+)(人|枚|つ)以上いる", text):
        m = re.search(r"(\d+)(人|枚|つ)以上いる", text)
        if m:
            matched = (int(m.group(1)), ">=")
    if not matched:
        return None

    count_val, operator_val = matched
    result = {
        "type": "card_count_condition",
        "count": count_val,
        "operator": operator_val,
        "text": text,
    }

    # Unit
    unit_match = re.search(r"(\d+)(人|枚|つ|種類)", text)
    if unit_match:
        result["unit"] = unit_match.group(2)
        if unit_match.group(2) == "人":
            result["card_type"] = "member_card"

    # Negation
    if "ない" in text or "いない" in text:
        result["negation"] = True

    # Distinct
    if "コストがそれぞれ異なる" in text:
        result["distinct"] = "cost"
    if any(kw in text for kw in ["名前が異なる", "カード名が異なる"]):
        result["distinct"] = "card_name"

    # Exclude self
    if (
        "このメンバー以外" in text
        or "このカード以外" in text
        or bool(re.search(r"ほかの.*メンバー", text))
    ):
        result["exclude_self"] = True
        result["card_type"] = "member_card"

    # Energy context
    if "エネルギー" in text:
        result["location"] = "energy_zone"

    # Revealed cards context
    if "エールにより公開された" in text or "これにより公開された" in text:
        result["location"] = "revealed_cards"

    # Live card zone
    if "ライブ中のカード" in text:
        result["location"] = "live_card_zone"

    # Surplus heart
    if "余剰ハート" in text:
        result["type"] = "comparison_condition"
        result["resource_type"] = "surplus_heart"
        if "相手" in text:
            result["target"] = "opponent"

    # Extract location/target/card_type
    if not result.get("location"):
        loc = f.location
        if loc:
            zone_keywords = ["置き場", "ゾーン"]
            if not any(kw in text for kw in zone_keywords):
                result["location"] = loc

    if f.target and "target" not in result:
        result["target"] = f.target

    if f.card_type and "card_type" not in result:
        zone_keywords = ["置き場", "ゾーン"]
        if not any(kw in text for kw in zone_keywords):
            result["card_type"] = f.card_type

    # Comparison target
    if f.comparison_target:
        result["comparison_target"] = f.comparison_target

    # Group names
    if f.group_names:
        result["group_names"] = f.group_names

    # Heart colors
    if f.heart_colors:
        result["heart_colors"] = f.heart_colors

    return result


@register(78)
def _try_both(text, f):
    """それらが両方ある — both condition."""
    if "それらが両方ある" not in text:
        return None
    return {"type": "both_condition", "text": text}


@register(75)
def _try_temporal_this_turn(text, f):
    """このターン — temporal condition."""
    if "このターン" not in text:
        return None
    temporal_patterns = {
        "移動していない": "not_moved",
        "移動している": "has_moved",
        "ライブを成功させていた": "opponent_live_success",
        "余剰ハートを持たない": "no_excess_heart",
    }
    for pat, cond_type in temporal_patterns.items():
        if pat in text:
            result = {
                "type": "temporal_condition",
                "temporal": "this_turn",
                "condition": {"type": cond_type},
                "text": text,
            }
            if f.card_type:
                result["card_type"] = f.card_type
            return result
    return None


@register(70)
def _try_baton_touch(text, f):
    """バトンタッチして登場した/しており/控え室に置か — baton touch condition."""
    if not any(
        kw in text
        for kw in [
            "バトンタッチして登場した",
            "バトンタッチして登場しており",
            "バトンタッチして控え室に置か",
        ]
    ):
        return None
    is_to_stage = "バトンタッチして登場した" in text
    result = {
        "type": "movement_condition",
        "movement": "baton_touch",
        "target": "self",
        "baton_touch_trigger": True,
        "text": text,
    }
    result["location"] = "stage" if is_to_stage else "discard"

    m = re.search(r"「([^」]+)」からバトンタッチ", text)
    if m:
        result["baton_touch_source"] = m.group(1)
    m = re.search(r"『([^』]+)』からバトンタッチ", text)
    if m:
        result["baton_touch_group"] = m.group(1)
    count_m = re.search(r"(\d+)人からバトンタッチ", text)
    if count_m:
        result["min_baton_touch_count"] = int(count_m.group(1))

    if f.cost_limit is not None:
        result["cost_limit"] = f.cost_limit
        if f.cost_limit_operator:
            result["cost_limit_operator"] = f.cost_limit_operator

    if f.group_names:
        result["group_names"] = f.group_names
    if f.exclude_self:
        result["exclude_self"] = True
    if "能力を持たない" in text or "能力も持たない" in text:
        result["ability_filter"] = "no_ability"
    if "コスト" in text and ("低い" in text or "高い" in text):
        result["comparison_type"] = "cost"
        result["operator"] = "<" if "低い" in text else ">"
    return result


@register(65)
def _try_temporal_count(text, f):
    """このターン、N回登場 — temporal with count."""
    if "このターン" not in text and "ターン目" not in text:
        return None
    if "回" not in text and "登場" not in text:
        return None
    result = {"type": "temporal_condition", "temporal": "this_turn", "text": text}
    m = re.search(r"(\d+)回", text)
    if m:
        result["count"] = int(m.group(1))
    elif "登場" in text and "回" not in text:
        result["count"] = 1
    if "ライブフェイズ" in text:
        result["phase"] = "live_phase"
    elif "メインフェイズ" in text:
        result["phase"] = "main_phase"
    if f.location:
        result["location"] = f.location
    if f.card_type:
        result["card_type"] = f.card_type
    if f.target:
        result["target"] = f.target
    if "エリアすべて" in text:
        result["all_areas"] = True
    if "移動している" in text:
        result["movement_state"] = "has_moved"
    return result


@register(60)
def _try_either_target(text, f):
    """自分か相手の — either target condition."""
    if "自分か相手の" not in text:
        return None
    m = re.search(r"自分か相手の(.+?)(?:に|が|にある)", text)
    if not m:
        return None
    loc_text = m.group(1).strip()
    loc_map = {
        "成功ライブカード置き場": "success_live_zone",
        "ライブカード置き場": "live_card_zone",
        "控え室": "discard",
        "手札": "hand",
        "ステージ": "stage",
        "エネルギー置き場": "energy_zone",
    }
    for kw, code in loc_map.items():
        if kw in loc_text:
            result = {
                "type": "location_condition",
                "location": code,
                "target": "either",
                "text": text,
            }
            if f.cost_limit is not None:
                result["cost_limit"] = f.cost_limit
            if f.operator:
                result["operator"] = f.operator
            return result
    return None


@register(55)
def _try_movement(text, f):
    """移動した/移動している/移動する — movement condition."""
    if "移動した" not in text and "移動している" not in text and "移動する" not in text:
        return None
    result = {"type": "movement_condition", "text": text}
    if "移動する" in text and "移動した" not in text and "移動している" not in text:
        result["movement"] = "moves"
    else:
        result["movement"] = "moved"
        result["movement_state"] = "has_moved"
    if "移動していない" in text:
        result["negation"] = True
    if "自分のカードの効果" in text:
        result["self_effect_only"] = True
    if "エネルギーが置かれ" in text:
        result["energy_placed"] = True
    return result


@register(50)
def _try_appearance(text, f):
    """登場 — appearance condition."""
    if "登場" not in text:
        return None
    result = {
        "type": "appearance_condition",
        "appearance": True,
        "text": text,
        "location": "stage",
    }
    m = re.search(r"「([^」]+)」[がを]登場", text)
    if m:
        result["characters"] = [m.group(1)]
    elif re.findall(r"「([^」]+)」", text[: text.find("登場")]):
        quoted = re.findall(r"「([^」]+)」", text[: text.find("登場")])
        if quoted:
            result["characters"] = [quoted[-1]]
    if "エリアすべて" in text:
        result["all_areas"] = True
    if "バトンタッチ" in text:
        result["baton_touch_trigger"] = True
        if f.group_names:
            result["group_names"] = f.group_names
        m = re.search(r"「([^」]+)」からバトンタッチ", text)
        if m:
            result["baton_touch_source"] = m.group(1)
    if f.target:
        result["target"] = f.target
    if f.position:
        result["position"] = f.position
    return result


@register(48)
def _try_state(text, f):
    """ウェイト状態/アクティブ状態 — state condition."""
    for patterns, state in [
        (["ウェイト状態である", "ウェイト状態にある", "ウェイト状態の"], "wait"),
        (
            ["アクティブ状態である", "アクティブ状態にある", "アクティブ状態の"],
            "active",
        ),
    ]:
        if any(p in text for p in patterns):
            result = {"type": "state_condition", "state": state, "text": text}
            if state == "active" and "エネルギー" in text:
                result["resource_type"] = "energy"
            if "すべて" in text:
                result["all"] = True
            return result
    return None


@register(45)
def _try_revealed(text, f):
    """エールにより公開された自分のカードの中に — revealed cards condition."""
    if "エールにより公開された自分のカードの中に" not in text:
        return None
    result = {
        "type": "location_condition",
        "location": "revealed_cards",
        "target": "self",
        "text": text,
    }
    if "持たない" in text or "ない" in text:
        result["negation"] = True
    if "ブレードハートを持つ" in text or "ブレードハートを持たない" in text:
        result["card_property"] = "has_blade_heart"
    if "0枚" in text:
        result["count"] = 1
        result["operator"] = ">="
    return result


@register(40)
def _try_position_change(text, f):
    """ポジションチェンジ — position change condition."""
    if not any(
        kw in text
        for kw in [
            "ポジションチェンジしてもよい",
            "ポジションチェンジさせてもよい",
            "ポジションチェンジする",
            "フォーメーションチェンジ",
        ]
    ):
        return None
    result = {
        "type": "position_change_condition",
        "action": "position_change",
        "optional": "してもよい" in text,
        "text": text,
    }
    if "自分と相手" in text:
        result["target"] = "both"
    if "センターエリア以外" in text:
        result["exclude_position"] = "center"
    if "センターにいる" in text:
        result["source_position"] = "center"
    return result


@register(35)
def _try_energy_state(text, f):
    """エネルギーがある/ない — energy state condition."""
    has_pos = "エネルギーがある" in text
    has_neg = "エネルギーがない" in text
    if not has_pos and not has_neg:
        return None
    result = {"type": "energy_state_condition", "text": text}
    if has_neg:
        result["negation"] = True
    if "アクティブ状態" in text:
        result["state"] = "active"
    return result


@register(30)
def _try_otherwise(text, f):
    """それ以外の場合 — otherwise condition."""
    if "それ以外の場合" not in text:
        return None
    return {"type": "otherwise_condition", "text": text}


@register(25)
def _try_ability_filter(text, f):
    """能力を持たない/能力も持たない — ability filter condition."""
    if "能力も持たない" not in text and "能力を持たない" not in text:
        return None
    result = {
        "type": "ability_filter_condition",
        "text": text,
        "ability_filter": "no_ability",
    }
    if "能力も" in text:
        result["ability_filter"] = "no_ability_type"
        triggers = re.findall(r"\{\{(\w+)\.png\|[^}]+\}\}能力も", text)
        if triggers:
            result["ability_filter_triggers"] = triggers
    return result


@register(20)
def _try_heart_possession(text, f):
    """ハートを持たない/を持つ — heart possession condition."""
    if "余剰ハート" in text:
        return None
    if not re.search(
        r"{{icon_([^}]+)\.png\|[^}}]+}}(?:[^持]*)(持たない|を持つ)", text
    ) and not ("ハート" in text and ("持たない" in text or "を持つ" in text)):
        return None
    result = {
        "type": "location_condition",
        "card_type": "member_card",
        "location": "stage",
        "text": text,
    }
    if "持たない" in text:
        result["negation"] = True
    if "icon_all" in text:
        result["heart_type"] = "all"
    if "元々" in text and "ハート" in text and "より多い" in text:
        result["original_value"] = True
        result["operator"] = ">"
        result["count"] = 1
    return result


@register(15)
def _try_live_mid(text, f):
    """ライブ中 — during live condition."""
    if "ライブ中" not in text:
        return None
    result = {"text": text}
    count_match = re.search(r"(\d+)枚以上", text)
    if count_match:
        result["type"] = "card_count_condition"
        result["count"] = int(count_match.group(1))
        result["operator"] = ">="
        result["card_type"] = "live_card"
        result["target"] = "self"
        result["temporal"] = "during_live"
    else:
        result["type"] = "temporal_condition"
        result["temporal"] = "during_live"
    if "手札にある" in text:
        result["location"] = "hand"
    elif "ステージにいる" in text:
        result["location"] = "stage"
    if f.target:
        result["target"] = f.target
    loc = f.location
    if loc and "location" not in result:
        result["location"] = loc
    return result
