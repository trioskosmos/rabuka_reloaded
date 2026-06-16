"""Per-type normalization and validation."""

import re
from typing import Dict, Any, Optional

POSITION_KEYWORDS = [
    ("センターエリア", "center"),
    ("左サイドエリア", "left_side"),
    ("右サイドエリア", "right_side"),
    ("センター", "center"),
    ("左サイド", "left_side"),
    ("右サイド", "right_side"),
]


def normalize_tree(effect: Dict[str, Any], original_text: str = None) -> Dict[str, Any]:
    if not effect or not isinstance(effect, dict):
        return effect

    _full_text = effect.get("text") or original_text or ""

    def _walk(d: Dict[str, Any], ctx_text: str = None) -> Dict[str, Any]:
        if not isinstance(d, dict):
            return d

        d_ctx = d.get("text") or ctx_text or _full_text

        # Propagate group_names from context
        if "group_names" not in d:
            gms = re.findall(r"『([^』]+)』", d_ctx)
            from_parent = not gms and ctx_text
            if from_parent:
                gms = re.findall(r"『([^』]+)』", ctx_text or "")
            if gms:
                gms = list(dict.fromkeys(gms))
                own_has_group = any(g in (d.get("text", "") or "") for g in gms)
                # Propagate if group is in own text, or from parent, or this is a condition node
                if own_has_group or from_parent or d.get("type"):
                    if d.get("action") != "gain_resource" and not (
                        d.get("action") == "change_state"
                        and d.get("card_type") == "energy_card"
                    ):
                        d["group_names"] = gms

        # Propagate exclude_self
        if "exclude_self" not in d and d_ctx:
            if "このメンバー以外" in d_ctx or "ほかの" in d_ctx or "他の" in d_ctx:
                d["exclude_self"] = True

        # Propagate all
        if "all" not in d and d_ctx:
            if re.search(
                r"すべての|全ての|全部の|全て|全員|全体|カードをすべて", d_ctx
            ):
                d["all"] = True

        # Propagate shuffle
        if "shuffle" not in d and d_ctx and "シャッフル" in d_ctx:
            d["shuffle"] = True

        # Propagate position (check parent context too)
        if (
            "position" not in d
            and "source_position" not in d
            and "exclude_position" not in d
        ):
            search_texts = [d_ctx]
            if ctx_text:
                search_texts.append(ctx_text)
            for st in search_texts:
                for kw, pos in POSITION_KEYWORDS:
                    if kw in st:
                        d["position"] = pos
                        break
                if d.get("position"):
                    break

        # Propagate original_value
        if (
            "original_value" not in d
            and d_ctx
            and ("元々持つ" in d_ctx or "元々" in d_ctx)
        ):
            d["original_value"] = True

        # Propagate distinct
        if (
            "distinct" not in d
            and d_ctx
            and ("名前の異なる" in d_ctx or "異なる名前" in d_ctx)
        ):
            d["distinct"] = "card_name"

        # Clean gain_resource
        if "heart_colors" not in d and d.get("action") in (
            "gain_resource",
            "modify_required_hearts",
            "move_cards",
        ):
            if d.get("action") != "gain_resource" or d.get("resource") not in (
                "blade",
                "ブレード",
            ):
                search_text = d.get("text", "") or d_ctx
                hc = list(
                    dict.fromkeys(
                        f"heart{m.zfill(2)}"
                        for m in re.findall(r"heart_(\d+)", search_text)
                    )
                )
                if hc:
                    d["heart_colors"] = hc

        if d.get("action") == "gain_resource":
            if d.get("resource") in ("blade", "ブレード"):
                d.pop("heart_colors", None)
            d.pop("source", None)

        if d.get("action") == "sequential" and "actions" in d:
            d["actions"] = [a for a in d["actions"] if a.get("action") != "do_nothing"]
        if d.get("action") == "sequential" and not d.get("actions"):
            d.pop("action", None)

        if d.get("type") == "location_condition" and "target" not in d:
            d["target"] = "self"

        # Heart_colors for conditions
        if "heart_colors" in d and "condition" in d:
            cond = d["condition"]
            if isinstance(cond, dict) and "heart_colors" not in cond:
                cond_type = cond.get("type", "")
                loc = cond.get("location", "")
                if cond_type in ("location_condition",) and loc in (
                    "stage",
                    "hand",
                    "live_card_zone",
                    "",
                ):
                    cond["heart_colors"] = d["heart_colors"]

        # Infer operator for comparison conditions
        ct = d.get("type")
        if (
            ct in ("comparison_condition", "card_count_condition")
            and "operator" not in d
        ):
            if d.get("comparison_target") and not d.get("operator"):
                text = d.get("text", "")
                if "高い" in text or "多い" in text or "大きい" in text:
                    d["operator"] = ">"
                elif "低い" in text or "少ない" in text or "小さい" in text:
                    d["operator"] = "<"
                elif "同じ" in text:
                    d["operator"] = "="
            elif (
                d.get("count")
                and not d.get("operator")
                and not d.get("comparison_target")
            ):
                d["operator"] = "="

        # Per-unit default count
        if d.get("per_unit") and "per_unit_count" not in d:
            d["per_unit_count"] = 1

        # Collapse single-action sequential wrappers
        if (
            d.get("action") == "sequential"
            and d.get("actions")
            and len(d["actions"]) == 1
        ):
            inner = d["actions"][0]
            if not d.get("condition") and not d.get("conditional"):
                outer_fields = {}
                for k in ("condition", "trigger_type", "text"):
                    if k in d:
                        outer_fields[k] = d[k]
                d.clear()
                d.update(inner)
                for k, v in outer_fields.items():
                    if k not in d:
                        d[k] = v

        # Recurse into sub-actions
        for sub_key in (
            "actions",
            "options",
            "primary_effect",
            "alternative_effect",
            "select_action",
            "look_action",
            "opponent_action",
            "followup_action",
            "optional_action",
            "conditional_action",
        ):
            sub = d.get(sub_key)
            if isinstance(sub, list):
                for item in sub:
                    if isinstance(item, dict):
                        if (
                            "activation_position" not in item
                            and "activation_position" in d
                        ):
                            item["activation_position"] = d["activation_position"]
                        if "position" not in item and "position" in d:
                            item["position"] = d["position"]
                        _walk(item, d_ctx)
            elif isinstance(sub, dict):
                if "activation_position" not in sub and "activation_position" in d:
                    sub["activation_position"] = d["activation_position"]
                if "position" not in sub and "position" in d:
                    sub["position"] = d["position"]
                _walk(sub, d_ctx)

        return d

    return _walk(effect, original_text)


def clean(obj):
    if isinstance(obj, dict):
        return {
            k: clean(v)
            for k, v in obj.items()
            if v is not None and v is not False and v != [] and v != {} and v != ""
        }
    if isinstance(obj, list):
        cleaned = [clean(item) for item in obj]
        return [x for x in cleaned if x is not None and x != {}]
    return obj


def process_abilities(data: dict) -> dict:
    """Post-processing pass applied after all abilities are parsed."""
    for ability in data["unique_abilities"]:
        if isinstance(ability.get("effect"), dict):
            continue
        t = ability.get("triggerless_text", "")
        if not t or ability.get("is_null", False):
            continue

        cost_text = None
        effect_text = t
        if "：" in effect_text:
            parts = effect_text.split("：", 1)
            cost_text = parts[0].strip()
            effect_text = parts[1].strip()

        from test_parser.cost import parse_cost
        from test_parser.effect import parse_effect

        cost = parse_cost(cost_text) if cost_text else None
        effect = parse_effect(effect_text)

        effect = normalize_tree(effect, t)
        effect = clean(effect)
        if isinstance(effect.get("actions"), list) and not effect["actions"]:
            effect.pop("actions", None)

        ability["cost"] = cost if cost else None
        if isinstance(effect, dict) and "cost" in effect:
            ability["cost"] = effect.pop("cost")
        ability["effect"] = effect

    # Pass 2a: activation_position from full triggerless_text (icons are in cost part)
    for ability in data["unique_abilities"]:
        eff = ability.get("effect")
        if not isinstance(eff, dict):
            continue
        if "activation_position" not in eff:
            full = ability.get("full_text", "") or ability.get("triggerless_text", "")
            if "{{center.png|センター}}" in full:
                eff["activation_position"] = "center"
            elif "{{left.png|左サイド}}" in full:
                eff["activation_position"] = "left_side"
            elif "{{right.png|右サイド}}" in full:
                eff["activation_position"] = "right_side"

    # Pass 2b: temporal_condition augmentation
    for ability in data["unique_abilities"]:
        eff = ability.get("effect")
        if not isinstance(eff, dict):
            continue
        cond = eff.get("condition")
        if not isinstance(cond, dict):
            continue
        if "heart_colors" in cond:
            continue
        if cond.get("type") != "temporal_condition":
            continue
        hm = re.findall(r"{{heart_(\d+)\.png\|heart\d+}}", cond.get("text", ""))
        if hm:
            cond["heart_colors"] = sorted(set(f"heart{m.zfill(2)}" for m in hm))

    # Pass 3: infer action for effects with missing action
    for ability in data["unique_abilities"]:
        eff = ability.get("effect")
        if not isinstance(eff, dict):
            continue
        if eff.get("action"):
            continue
        if eff.get("source") and eff.get("destination"):
            eff["action"] = "move_cards"
        elif eff.get("actions"):
            eff["action"] = "sequential"
        elif eff.get("opponent_action"):
            eff["action"] = "opponent_action"
        if eff.get("per_unit") and eff.get("action") in ("draw", "draw_card"):
            if eff.get("count") is None:
                eff["count"] = 1
        if eff.get("action") == "sequential":
            parent_ct = eff.get("card_type")
            for sub in eff.get("actions", []):
                if isinstance(sub, dict):
                    if not sub.get("card_type") and parent_ct:
                        sub["card_type"] = parent_ct
                    if not sub.get("action"):
                        if sub.get("source") and sub.get("destination"):
                            sub["action"] = "move_cards"
                        elif sub.get("actions"):
                            sub["action"] = "sequential"

    # Pass 4: clean gain_resource
    def _clean_gain(node):
        if isinstance(node, dict):
            if node.get("action") == "gain_resource":
                if node.get("resource") in ("blade", "ブレード"):
                    node.pop("heart_colors", None)
                node.pop("source", None)
            for v in node.values():
                _clean_gain(v)
        elif isinstance(node, list):
            for item in node:
                _clean_gain(item)

    for a in data["unique_abilities"]:
        for key in ("effect", "cost"):
            val = a.get(key)
            if isinstance(val, dict):
                _clean_gain(val)

    # Pass 5: opponent_action flattening
    for ability in data["unique_abilities"]:
        eff = ability.get("effect")
        if not isinstance(eff, dict):
            continue
        if eff.get("action") == "opponent_action" and isinstance(
            eff.get("opponent_action"), dict
        ):
            oa = eff.pop("opponent_action")
            for k, v in oa.items():
                if k not in eff:
                    eff[k] = v
            inner_action = oa.get("action")
            if inner_action:
                eff["action"] = inner_action
            eff.setdefault("target", "opponent")
            eff.setdefault("action_by", "opponent")

    # Pass 6: conditional_on_optional migration
    for ability in data["unique_abilities"]:
        eff = ability.get("effect")
        if not isinstance(eff, dict):
            continue
        if eff.get("action") != "conditional_on_optional":
            continue
        if "positive_action" in eff and "conditional_action" not in eff:
            eff["conditional_action"] = eff.pop("positive_action")
        if "negative_action" in eff:
            eff.pop("negative_action")

    # Pass 7: each_time source fixup
    for ability in data["unique_abilities"]:
        eff = ability.get("effect")
        if not isinstance(eff, dict):
            continue
        if eff.get("trigger_type") != "each_time":
            continue
        tc = eff.get("trigger_condition")
        if not isinstance(tc, dict):
            continue
        if tc.get("location") == "discard" and "source" not in tc:
            tc["source"] = "preceding_moved"

    return data
