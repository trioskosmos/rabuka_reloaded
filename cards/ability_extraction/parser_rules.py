"""
Action dispatch rules and their field setters.

`_ACTION_RULES` is the action-layer dispatch table: first match
wins, so LIST ORDER is the priority. Each row names an action, a
guard (condition / match / match_any / match_all / exclude*), and
the setter that fills the fields. The setters are pure field
fills - they read text and write the action dict. They call back
into parser.py for the shared text/field helpers; that circular
import is resolved by parser.py importing this module at its
bottom, after those helpers are defined.
"""

import re
from typing import List

from parser import (
    COST_CHANGE_VERBS,
    _blade_icon_is_target_filter,
    _extract_place_restriction_destination,
    _handle_cost_modification,
    _handle_dynamic_count,
    _handle_position_change_fields,
    _ic,
    _quoted_group_names,
    _strip_icon_annotations,
    extract_heart_colors_from_text,
    extract_heart_types,
)
from parser_utils import (
    ALL_HEART_ICON,
    ActionRule,
    BLADE_ICON,
    BLADE_ICON_RE,
    ENERGY_ICON,
    HEART_ICON_ID,
    HEART_LABEL,
    as_int,
    extract_cost_limit_with_operator,
    extract_count,
    extract_picker,
    extract_target,
    heart_id,
)
from parser_fields import _has_shuffle, extract_optional
from parser_vocab import _heart_count, _heart_label_counts, _uniform_count

def _set_score_op(t, a):
    """Set operation and value for modify_score from text patterns."""
    sm = re.search(r"([+\-])(\d+)", t)
    if sm:
        a["value"] = as_int(sm.group(2))
        a["operation"] = "remove" if sm.group(1) == "-" else "add"
        return
    cnt = extract_count(t)
    if cnt:
        a["value"] = cnt
        if "マイナス" in t or "減らす" in t or "減る" in t:
            a["operation"] = "remove"
        else:
            a["operation"] = "add"
        return
    if "プラス" in t or "増やす" in t or "増える" in t:
        a["operation"] = "add"
    elif "マイナス" in t or "減らす" in t or "減る" in t:
        a["operation"] = "remove"


def _handle_required_hearts(t, a):
    raw = re.findall(HEART_LABEL, t)
    # Every colour is normally written the same number of times; a clause that
    # disagrees falls back to the smallest count so it cannot over-grant.
    per_color = _uniform_count(_heart_label_counts(t)) or 1
    colors = list(dict.fromkeys(raw))
    a.update(
        {
            "operation": "set",
            "heart_colors": colors,
            "count": per_color,
            "value": per_color,
            "text": t,
        }
    )
    if "このカード" in t:
        a["self_target"] = True
    # Add replace_all flag to signal that unspecified colors should be cleared
    # (semantic: "cost becomes exactly these values")
    a["replace_all"] = True


def _set_state_change_action(text, action, state_change=None):
    """A state change: ウェイト / レスト / アクティブ, and what it applies to.

    The card type is the interesting part, because the clause decides it in a
    fixed order and two of the branches overlap:
      - 「このメンバー」: this member, and unless the clause says 以外, exactly
        this one (target self, count 1) rather than "any member"
      - 「エネルギー」 with no メンバー: an energy card
      - 「メンバー」 plus a state word: any member in that state
    """
    if state_change is not None:
        action["state_change"] = state_change
    target = extract_target(text)
    if target:
        action["target"] = target
    if "このメンバー" in text:
        action["card_type"] = "member_card"
        if "このメンバー以外" not in text:
            action.update({"target": "self", "self_target": True, "count": 1})
    elif "エネルギー" in text and "メンバー" not in text:
        action["card_type"] = "energy_card"
    elif "メンバー" in text and any(
        state in text for state in ("ウェイト", "レスト", "アクティブ")
    ):
        action["card_type"] = "member_card"
    if "してもよい" in text:
        action["optional"] = True
    return action

def _nearest_invalidation_trigger(text):
    ability_pos = text.find('能力')
    if ability_pos < 0:
        return None
    prefix = text[:ability_pos]
    triggers = ('登場', 'ライブ開始時', 'ライブ成功時', '起動', '常時')
    for icon in reversed(re.findall('\\{\\{[^|{}]+\\|([^}]+)\\}\\}', prefix)):
        if icon in triggers:
            return icon
    positions = [(prefix.rfind(trigger), trigger) for trigger in triggers]
    positions = [(position, trigger) for position, trigger in positions if position >= 0]
    return max(positions)[1] if positions else None

# The colour words a 「［緑ハート］」 clause can use, in the order the printed
# text uses them. heart00 is the "no colour named" fallback.
HEART_COLOR_BY_JP = {
    "緑": "heart01",
    "赤": "heart02",
    "青": "heart03",
    "黄": "heart04",
    "紫": "heart05",
    "白": "heart06",
}

# 「［緑ハート］」 — the bracketed colour name of a heart-selection clause.
_BRACKET_HEART_RE = re.compile(r"［([^］]+)ハート］")

# 「N枚になるまで」 — a draw-until clause.
_DRAW_UNTIL_RE = re.compile(r"(\d+)枚になるまで")


def _set_heart_selection_resource(t, a):
    """「［緑ハート］」 — gain hearts of one named colour, chosen by the player.

    An unrecognised colour word falls back to heart00 rather than failing, so a
    clause the parser has not seen still produces a usable action.
    """
    m = _BRACKET_HEART_RE.search(t)
    selected = m.group(1) if m is not None else ""
    a["resource"] = "heart"
    a["heart_selection"] = True
    a["heart_colors"] = [HEART_COLOR_BY_JP.get(selected, "heart00")]
    return a


def _set_action_000(t, a):
    """Target a deck: the main deck, else the energy deck."""
    a["target"] = "deck" if "デッキ" in t else "energy_deck"
    return a


def _set_action_002(t, a):
    """An optional, multi-target action."""
    a["optional"] = extract_optional(t)
    a["multiple_targets"] = True
    return a


def _set_action_003(t, a):
    """Pay energy, optionally — the count is the number of energy icons."""
    a["energy"] = t.count(ENERGY_ICON)
    a["optional"] = "もよい" in t or "してもよい" in t
    return a


def _set_action_004(t, a):
    """The energy count, defaulting to the action's own count."""
    a["energy_count"] = a.get("count") or 1
    return a


def _set_action_005(t, a):
    """The target is whichever member this ability is on."""
    a["target_member"] = "this_member"
    return a


def _set_action_006(t, a):
    """Self-targeted; member_card when the clause says so, else any card."""
    a["self_target"] = True
    a["card_type"] = "member_card" if "メンバー" in t else "card"
    return a


def _set_action_007(t, a):
    """Target whatever is under the activating member."""
    a["under_self"] = True
    return a


def _set_action_008(t, a):
    """Draw from the deck into hand, until the hand holds N cards."""
    a["source"] = "deck"
    a["destination"] = "hand"
    a["target_count"] = as_int(_DRAW_UNTIL_RE.search(t).group(1))
    return a


def _set_action_009(t, a):
    """Draw until the hand holds N cards."""
    a["target_count"] = as_int(_DRAW_UNTIL_RE.search(t).group(1))
    return a


def _set_action_010(t, a):
    """Optionally draw one card from the deck into hand."""
    a["count"] = 1
    a["optional"] = True
    a["source"] = "deck"
    a["destination"] = "hand"
    return a


def _set_action_011(t, a):
    """Discard one card from hand."""
    a["source"] = "hand"
    a["destination"] = "discard"
    return a


def _set_action_012(t, a):
    """Look at / take from the deck into hand."""
    a["source"] = "deck"
    a["destination"] = "hand"
    return a


def _restriction_target(t):
    """Which player a restriction clause names.

    「自分と相手」 and 「お互い」 both contain 自分 AND 相手, so the both-case has to
    be tested first. Three rules resolve this independently and two of them had
    it last, so a 「自分と相手が〜できない」 clause resolved to a single player.
    """
    t = t.replace("相手の効果によっては", "").replace("自分の効果によっては", "").replace("効果によっては", "")
    if "自分と相手" in t or "お互い" in t:
        return "both"
    if "相手" in t:
        return "opponent"
    if "自分" in t:
        return "self"
    return None


def _set_action_013(t, a):
    """Optionally take a card from the deck into hand."""
    a["source"] = "deck"
    a["destination"] = "hand"
    a["optional"] = True
    return a


def _set_action_014(t, a):
    """Cost modification — see _handle_cost_modification for the field rules."""
    return _handle_cost_modification(t, a)


def _set_action_017(t, a):
    """Send this member to the active state."""
    return _set_state_change_action(t, a, "active")


def _set_action_019(t, a):
    """Pay to activate, rather than activating for free."""
    a["activation_type"] = "pay_to_activate"
    return a

def _set_action_020(t, a):
    """Restriction: cannot become a live card."""
    a["restriction_type"] = "cannot_live"
    a["target"] = _restriction_target(t)
    return a


def _set_action_021(t, a):
    """Restriction: cannot activate, optionally in one phase."""
    a["restriction_type"] = "cannot_activate"
    a["target"] = _restriction_target(t)
    a["phase"] = "active_phase" if "アクティブフェイズ" in t else None
    # 「次のターンのアクティブフェイズにアクティブにならない」 applies ONCE, at
    # the NEXT active phase — that is the `delayed` per-card flag in
    # effects/restriction.rs, keyed on the members this ability just waited
    # (HS-PR-038 ab#1). A bare 「アクティブにならない」 stays a standing
    # restriction, which the constants path (modifiers.rs) owns.
    if "次のターン" in t:
        a["delayed"] = True
    return a

def _set_action_023(t, a):
    """Restriction: cannot be sent to the wait state.

    Two clauses collapse into one restriction here, which is why this is not a
    dictionary lookup:
      - 「効果によっては」: conditional, `cannot_wait_by_effect`
      - otherwise: unconditional `cannot_wait`

    Target is read from the clause: both players, else the opponent, else self —
    that order matters, because 「自分と相手の」 also contains 「自分」.
    """
    a["restriction_type"] = (
        "cannot_wait_by_effect" if "効果によっては" in t else "cannot_wait"
    )
    # 「相手の効果によっては」 is a verb modifier ("because of the opponent's
    # effects"), not the restriction's target — strip it before target parsing,
    # otherwise a self-targeted clause reads as the opponent's.
    target_text = t.replace("相手の効果によっては", "").replace("効果によっては", "")
    # This rule tests 「相手の」, not 「相手」 — a bare 相手 is the OPPONENT'S,
    # whereas 「相手のカード」 names the card the restriction is written on. The
    # other two restriction rules do use the bare form; the difference is real
    # and they share only the both/self fallback.
    if "自分と相手" in target_text or "お互い" in target_text:
        a["target"] = "both"
    elif "相手の" in target_text:
        a["target"] = "opponent"
    elif "自分" in target_text:
        a["target"] = "self"
    else:
        a["target"] = None
    a["card_type"] = "member_card" if "メンバー" in t else None
    a["duration"] = "live_end" if "ライブ終了時まで" in t else None
    return a


def _set_action_025(t, a):
    """Restriction: cannot be placed at the stated destination."""
    a["restriction_type"] = "cannot_place"
    a["destination"] = _extract_place_restriction_destination(t)
    return a


def _set_action_026(t, a):
    """Restriction: cannot be placed at the stated destination."""
    a["restriction_type"] = "cannot_place"
    a["destination"] = _extract_place_restriction_destination(t)
    return a


def _set_action_030(t, a):
    """Position change: move the member to the front.

    Reads the target from the clause, applies the position-change fields, and
    marks a member-selection target when the clause names one. All four steps
    write into `a` in place; the caller ignores the return.
    """
    a["target"] = extract_target(t)
    _handle_position_change_fields(t, a)
    if "正面" in t:
        a["destination"] = "front"
    if "メンバー" in t and ("1人" in t or "N人" in t):
        a["target_member"] = "select"
    return a

def _set_action_033(t, a):
    """Blade gain counted per blade icon, defaulting to 1, this turn only."""
    a["resource"] = "blade"
    a["count"] = t.count(BLADE_ICON) or 1
    a["timing_condition"] = "moved_this_turn"
    return a


def _set_action_036(t, a):
    """Blade gain counted per blade icon, defaulting to 1."""
    a["resource"] = "blade"
    a["count"] = _ic(t, BLADE_ICON) or 1
    return a


def _set_action_037(t, a):
    """Blade gain counted per blade icon, unstated when there are none."""
    a["resource"] = "blade"
    a["count"] = t.count(BLADE_ICON) or None
    return a


def _set_action_038(t, a):
    """Heart gain counted per all-heart icon, unstated when there are none."""
    a["count"] = t.count(ALL_HEART_ICON) or None
    return a


def _set_action_039(t, a):
    """Plain heart gain, with no count of its own."""
    a["resource"] = "heart"
    return a

def _set_action_040(t, a):
    """「余剰ハートBake失去」 — lose a resource, as a negative gain.

    Which resource comes from the clause, and the count from the icons: blades
    first, then heart icons (which also record their colours), and only if
    neither is present does a bare 「N枚/個/つ」 supply the number.
    """
    if "余剰ハート" in t or "余分ハート" in t or "それら" in t:
        a["resource"] = "surplus_heart"
    elif "ブレード" in t:
        a["resource"] = "blade"
    else:
        a["resource"] = "heart"
    blade_count = len(re.findall(BLADE_ICON_RE, t))
    heart_count = _heart_count(t)
    if blade_count:
        a["count"] = blade_count
    elif heart_count:
        a["count"] = heart_count
        a["heart_colors"] = extract_heart_types(t)
    elif "count" not in a:
        count_match = re.search(r"(\d+)(?:つ|個|枚)", t)
        if count_match:
            a["count"] = as_int(count_match.group(1))
    if "すべて" in t or "全て" in t:
        a["all"] = True
    if "メンバーカード" in t or "ステージのメンバー" in t:
        a["card_type"] = "member_card"
    if "ライブ終了時まで" in t:
        a["duration"] = "live_end"
    cost_limit = extract_cost_limit_with_operator(t)
    if cost_limit:
        a["cost_limit"], a["cost_limit_operator"] = cost_limit
    a["sign"] = "negative"
    return a

def _set_action_041(t, a):
    """「ブレードハートを失い」 — lose the blade heart, then re-yell once.

    Three shapes of the same printed clause:
      - 「…できない」: cannot, so this rule does not apply at all
      - 「ブレードハートを失い」: the re-yell is one compound step, not two
      - otherwise: the plain blade-heart loss
    """
    if "できない" in t:
        return None
    if "ブレードハートを失い" in t:
        # The loss and the re-yell are one printed clause; the engine gets
        # them as a sequential so the re-yell happens after the loss.
        a.pop("lose_blade_hearts", None)
        a.pop("location", None)
        a["action"] = "sequential"
        a["actions"] = [
            {
                "text": "ブレードハートを失い",
                "action": "re_yell",
                "lose_blade_hearts": True,
                "target": "self",
            },
            {
                "text": "もう一度エールを行う",
                "action": "perform_yell",
                "count": 1,
                "target": "self",
            },
        ]
        return a
    a["lose_blade_hearts"] = True
    return a

def _set_action_042(t, a):
    """Look at the top of the deck.

    「デッキの上」 fixes the source; the look itself may still be a dynamic
    count, which is applied before the action is named.
    """
    if "デッキの上" in t:
        a["source"] = "deck_top"
    _handle_dynamic_count(t, a)
    a["action"] = "look_at"
    return a


def _set_action_043(t, a):
    """Reveal a card from hand, or from an opponent's hand.

    The source defaults to hand, and the two optional clauses are independent:
    「見ないで」 is a blind reveal (no picker) and 「～に選んでもらう」 names one.
    """
    a["source"] = a.get("source") or "hand"
    if "見ないで" in t:
        a["blind"] = True
    picker = extract_picker(t)
    if picker:
        a["picker"] = picker
    return a


def _set_action_046(t, a):
    """Heart colours, but only when nothing else already says what is targeted.

    A clause that names a source or a card type is about THAT, so a heart icon
    in it belongs to the other clause and must not become a filter here.
    """
    if a.get("source") or a.get("card_type") or "{{heart_" not in t:
        return None
    a["heart_colors"] = extract_heart_colors_from_text(t)
    return a


def _set_action_050(t, a):
    """Invalidate the nearest trigger mentioned in the text."""
    a["target_trigger"] = _nearest_invalidation_trigger(t)
    return a


def _set_action_051(t, a):
    """Suppress the ability whose trigger icon precedes 「能力」 in the text.

    The icon is whatever appears first before that word — reading the slice
    rather than the whole clause is what keeps a later icon from being picked up.
    """
    cut = t.find("能力")
    m = re.search(r"\{\{(\w+)\.png\|", t[:cut]) if cut >= 0 else None
    a["suppressed_trigger"] = m.group(1) if m else None
    return a

def _set_heart_type(t, a, allow_selected=False):
    """The heart type a 「ハート○に変える」 clause names.

    Shared by the two rules that read it. They differ only in whether
    「選んだハート」 is a valid answer, so that is the one parameter — the other
    three fields are the same clause and used to be written out twice.
    """
    m = re.search(HEART_ICON_ID, t)
    if m:
        a["heart_type"] = heart_id(m.group(1))
    elif allow_selected and "選んだハート" in t:
        a["heart_type"] = "selected"
    else:
        a["heart_type"] = None
    a["original_value"] = "元々" in t
    a["self_target"] = "このメンバー" in t or "このカード" in t
    a["card_type"] = "member_card" if "メンバー" in t else None
    return a


def _set_action_057(t, a):
    """Set a named heart type, from a printed icon only."""
    return _set_heart_type(t, a)

def _set_action_058(t, a):
    """Offer a choice, aimed at self."""
    a["choice"] = True
    a["target"] = "self"
    return a


def _set_action_059(t, a):
    """Gain a heart of the chosen colour."""
    a["resource"] = "heart"
    a["heart_selection"] = True
    return a


def _set_action_060(t, a):
    """Set a heart type: a printed icon, or the chosen heart."""
    return _set_heart_type(t, a, allow_selected=True)

def _set_action_062(t, a):
    """Cost modification — see _handle_cost_modification for the field rules."""
    return _handle_cost_modification(t, a)


def _set_action_063(t, a):
    """「N回」 — a use limit of N per turn."""
    m = re.search(r"(\d+)回", t)
    if m is not None:
        a["max_repeats"] = as_int(m.group(1))
    return a


def _set_action_069(t, a):
    """「Nになる/なった」 — set a value to N."""
    a["operation"] = "set"
    m = re.search(r"(\d+).*(になる|なった|なっている)", t)
    if m:
        a["value"] = as_int(m.group(1))
    return a


def _set_action_070(t, a):
    """Score operation — see _set_score_op for the field rules."""
    return _set_score_op(t, a)


def _set_action_071(t, a):
    """Place on top of the deck; 「好きな順番で」 also makes the order free."""
    a["destination"] = "deck_top"
    if "好きな順番で" in t:
        a["placement_order"] = "any_order"
    return a


# 「{{icon.png|label}}能力」 — any {{...}} icon markup.
_ICON_MARKUP_RE = re.compile(r"\{\{([^}]+)\}\}")


def _trigger_filter_labels(t):
    """The text label of the icon naming which ability triggers, or [].

    The label is the LAST `|`-separated field, because the clause is written
    with the icon file first and the printed text second
    (`{{icon.png|このカード}}能力`). The icon named is the one between 「持つ」
    and 「能力」.
    """
    if "持つ" not in t:
        return []
    between = t.split("持つ", 1)[1].split("能力", 1)[0]
    m = _ICON_MARKUP_RE.search(between)
    return [m.group(1).split("|")[-1]] if m else []


def _set_action_073(t, a):
    """Trigger on the ability of whatever sits under the member."""
    a["source_location"] = "under_member"
    a["trigger_filter"] = _trigger_filter_labels(t)
    a["all"] = True
    return a


def _set_action_074(t, a):
    """Which ability is gained, read from the clause, unless already known."""
    if a.get("ability_gain") is None:
        a["ability_gain"] = (
            _strip_icon_annotations(t)
            .replace("を失う", "")
            .replace("を得る", "")
            .replace("をえる", "")
        )
    return a


def _set_action_079(t, a):
    """Named identities, matched against any group, from any region."""
    a["identities"] = _quoted_group_names(t) or None
    a["all_regions"] = True
    return a


def _set_action_080(t, a):
    """A count, defaulting to 1 when the clause does not state one."""
    a["count"] = extract_count(t) or 1
    return a


def _set_action_081(t, a):
    """Keep the whole clause as the condition text."""
    a["condition_text"] = t
    return a


# 「10から自分の手札の枚数を引いた数」 — 引く as SUBTRACT (a look COUNT), not a draw.
# The phrase used to make action_012_draw_card win (it runs 30 rules ahead of
# action_042_look_at, and rules are first-match-wins), so a printed LOOK parsed as
# draw_card→hand and the cards went straight into hand instead of being looked at
# and selected from.
_ARITHMETIC_SUBTRACT_RE = re.compile(r"[^、。]*を引いた数")


def _draw_verb_in(t):
    """True when the clause really asks to draw, arithmetic 引いた数 aside."""
    stripped = _ARITHMETIC_SUBTRACT_RE.sub("", t)
    return "引く" in stripped or "引き" in stripped or "引い" in stripped


_ACTION_RULES: List[ActionRule] = [
    ActionRule(name='action_000_shuffle', condition=lambda t: _has_shuffle(t), action='shuffle', setter=_set_action_000),
    ActionRule(name='action_001_position_change', condition=lambda t: '入れ替える' in t or '入れ替えて' in t, action='position_change', setter=None),
    ActionRule(name='action_002_position_change', condition=lambda t: 'フォーメーションチェンジ' in t, action='position_change', setter=_set_action_002),
    ActionRule(name='action_003_pay_energy', condition=lambda t: '{{icon_energy.png|E}}' in t and ('支払う' in t or '支払って' in t) and ('選び' not in t), action='pay_energy', setter=_set_action_003),
    ActionRule(name='action_004_place_energy_under_member', condition=lambda t, a: a.get('destination') == 'under_member' and ('エネルギー' in t or 'energy_card' in t), action='place_energy_under_member', setter=_set_action_004),
    ActionRule(name='action_005_place_energy_under_member', condition=lambda t, a: a.get('source') == 'energy_deck' and 'このメンバーの下にある' in t, action='place_energy_under_member', setter=_set_action_005),
    ActionRule(name='action_006_move_cards', condition=lambda t, a: a.get('destination') == 'under_member' and 'source' not in a and ('エネルギー' not in t) and ('置く' in t or '置いて' in t), action='move_cards', setter=_set_action_006),
    ActionRule(name='action_007_move_cards', condition=lambda t, a: a.get('destination') == 'under_member' and 'このメンバーの下に' in t, action='move_cards', setter=_set_action_007),
    ActionRule(name='action_008_draw_until_count', condition=lambda t: '枚になるまで' in t and '引く' in t, action='draw_until_count', setter=_set_action_008),
    ActionRule(name='action_009_discard_until_count', condition=lambda t: '枚になるまで' in t and ('控え室に置く' in t or '控え室に置き' in t), action='discard_until_count', setter=_set_action_009),
    ActionRule(name='action_010_draw_card', match='カードを1枚引いてもよい', action='draw_card', setter=_set_action_010),
    ActionRule(name='action_011_move_cards', condition=lambda t: '引いた枚数' in t and '手札から' in t and ('控え室に置く' in t), action='move_cards', setter=_set_action_011),
    ActionRule(name='action_012_draw_card', condition=lambda t: _draw_verb_in(t) and '手札から控え室に置く' not in t, action='draw_card', setter=_set_action_012),
    ActionRule(name='action_013_draw_card', condition=lambda t: '引いてもよい' in t, action='draw_card', setter=_set_action_013),
    ActionRule(name='action_014_modify_cost', condition=lambda t: re.search(rf"コスト[はが](\d+){COST_CHANGE_VERBS}", t) or re.search('ためのコストは(\\d+)減る', t), action='modify_cost', setter=_set_action_014),
    ActionRule(name='action_015_move_cards', condition=lambda t, a: 'source' in a and a.get('source') and ('destination' in a) and a.get('destination') and ('選ぶ' not in t) and ('選び' not in t) and (not ('手札に加える' in t and extract_count(t) is not None)), action='move_cards', setter=None),
    ActionRule(name='action_016_change_state', condition=lambda t, a: a.get('state_change') and a.get('state_change') != '', action='change_state', setter=_set_state_change_action),
    ActionRule(name='action_017_change_state', condition=lambda t: 'アクティブにしてもよい' in t or 'アクティブにする' in t or ('アクティブにし' in t and 'しない' not in t), action='change_state', setter=_set_action_017),
    ActionRule(name='action_018_activation_restriction', match_any=['のみ起動できる', 'のみ発動する'], action='activation_restriction', defaults={'restriction_type': 'only'}),
    ActionRule(name='action_019_activate_ability', match='支払って発動させる', action='activate_ability', setter=_set_action_019),
    ActionRule(name='action_020_restriction', match='ライブできない', action='restriction', setter=_set_action_020),
    ActionRule(name='action_021_restriction', match='アクティブにしない', action='restriction', setter=_set_action_021),
    ActionRule(name='action_022_restriction', condition=lambda t: 'アクティブしない' in t and 'アクティブにしない' not in t, action='restriction', defaults={'restriction_type': 'cannot_activate', 'delayed': True}),
    ActionRule(name='action_023_restriction', match='ウェイトしない', action='restriction', setter=_set_action_023),
    ActionRule(name='action_024_restriction', match='バトンタッチで控え室に置けない', action='restriction', defaults={'restriction_type': 'cannot_baton_touch'}),
    ActionRule(name='action_025_restriction', match='置くことができない', action='restriction', setter=_set_action_025),
    ActionRule(name='action_026_restriction', match='置けない', action='restriction', setter=_set_action_026),
    ActionRule(name='action_027_restriction', match='登場できない', action='restriction', defaults={'restriction_type': 'cannot_appear'}),
    ActionRule(name='action_028_restriction', match='移動できない', action='restriction', defaults={'restriction_type': 'cannot_move'}),
    ActionRule(name='action_029_move_cards', match_any=['加える', '加え'], exclude_any=['選ぶ', '選び'], action='move_cards', defaults={'destination': 'hand'}),
    ActionRule(name='action_030_position_change', match='ポジションチェンジ', action='position_change', setter=_set_action_030),
    ActionRule(name='action_031_position_change', match_all=['移動させ', 'エリア'], action='position_change'),
    ActionRule(name='action_032_move_cards', match='移動させ', exclude='エリア', action='move_cards'),
    ActionRule(name='action_033_gain_resource', match_all=['エリアを移動', 'ブレード'], action='gain_resource', setter=_set_action_033),
    ActionRule(name='action_034_position_change', match_any=['移動する', '移動し'], action='position_change'),
    ActionRule(name='action_035_move_cards', condition=lambda t: (('置く' in t or '置いて' in t) or ('置き' in t and '置き場' not in t)) and '選ぶ' not in t and ('選び' not in t), action='move_cards'),
    ActionRule(name='action_036_gain_resource', condition=lambda t: 'ブレードを得る' in t or '選んだブレード' in t, action='gain_resource', setter=_set_action_036),
    ActionRule(name='action_037_gain_resource', condition=lambda t: BLADE_ICON in t and '得る' in t and (not _blade_icon_is_target_filter(t)), action='gain_resource', setter=_set_action_037),
    ActionRule(name='action_038_gain_resource', match='得る', condition=lambda t: '{{icon_all.png' in t, action='gain_resource', defaults={'resource': 'heart', 'heart_type': 'all'}, setter=_set_action_038),
    ActionRule(name='action_039_gain_resource', condition=lambda t: '{{heart' in t and '得る' in t or bool(re.search('ハート.*得る', t)) or ('選んだハート' in t and 'になる' not in t), action='gain_resource', setter=_set_action_039),
    ActionRule(name='action_040_gain_resource', condition=lambda t: re.search(r'(ブレード|ハート|余剰ハート|それら).*?失[うい]', t) is not None and 'もう一度エール' not in t and 'もう1度エール' not in t, action='gain_resource', setter=_set_action_040),
    ActionRule(name='action_041_re_yell', condition=lambda t: 'もう一度エール' in t or 'もう1度エール' in t, action='re_yell', setter=_set_action_041),
    ActionRule(name='action_042_look_at', condition=lambda t: '見る' in t or '見て' in t or t.endswith('見'), action='look_at', setter=_set_action_042),
    ActionRule(name='action_043_reveal', condition=lambda t: '公開する' in t or '公開して' in t, action='reveal', setter=_set_action_043),
    ActionRule(name='action_044_choice', condition=lambda t: '1つを選ぶ' in t and ('以下から' in t or 'のうち' in t) and ('{{heart_' not in t) and ('ハート' not in t), action='choice'),
    ActionRule(name='action_045_select_number', condition=lambda t: bool(re.search('数\\d*つを選ぶ', t)), action='select_number'),
    ActionRule(name='action_046_select', condition=lambda t: '選ぶ' in t or '選び' in t or bool(re.search('選ん(?!だ)', t)), action='select', setter=_set_action_046),
    ActionRule(name='action_048_move_cards', match='登場させ', action='move_cards', defaults={'destination': 'stage'}),
    ActionRule(name='action_049_activate_ability', match_any=['起動でき', '起動して'], action='activate_ability'),
    ActionRule(name='action_050_invalidate_ability', match='無効に', exclude='無効にできない', action='invalidate_ability', setter=_set_action_050),
    ActionRule(name='action_051_suppress_ability_trigger', match='能力は発動しない', action='suppress_ability_trigger', setter=_set_action_051),
    ActionRule(name='action_052_modify_required_hearts', match_any=['必要ハート', 'ハートを増やす', 'ハートを減らす', '増やす', '減らす', '減らし'], action='modify_required_hearts'),
    # 「追加」 must be about スコア. Matching the bare word fired on any sentence
    # containing it, including the connective 「この能力を起動するための追加コストとして」
    # on PL!-pb2-012-P＋'s 起動, which produced an operandless `modify_score` (the
    # `modify_score` missing-`value` validation warning) in the middle of a cost.
    ActionRule(name='action_053_modify_score', match_all=['スコア', '追加'], exclude='エール', action='modify_score', defaults={'operation': 'add'}),
    ActionRule(name='action_054_modify_score', match_any=['スコアを1プラス', 'スコアをプラス'], action='modify_score', defaults={'operation': 'add', 'value': 1}),
    ActionRule(name='action_055_modify_score', match='スコアを1マイナス', action='modify_score', defaults={'operation': 'remove', 'value': 1}),
    ActionRule(name='action_056_set_blade_type', match='ブレードの色を', action='set_blade_type'),
    ActionRule(name='action_057_set_heart_type', condition=lambda t: 'ハートを' in t and 'すべて' in t and ('{{heart_' in t) and ('にする' in t) and (not 'ハートの色を' in t), action='set_heart_type', setter=_set_action_057),
    ActionRule(name='action_058_specify_heart_color', condition=lambda t: 'ハートの色を' in t, action='specify_heart_color', setter=_set_action_058),
    ActionRule(name='action_059_gain_resource', condition=lambda t: 'ハートを' in t and 'にする' in t and (not ('すべて' in t and '{{heart_' in t)) and (not 'ハートの色を' in t), action='gain_resource', setter=_set_action_059),
    ActionRule(name='action_060_set_heart_type', condition=lambda t: ('ハートは' in t or 'ハートが' in t) and 'になる' in t and ('ハートを' not in t) and ('すべて' in t and '{{heart_' in t or '選んだハート' in t), action='set_heart_type', setter=_set_action_060),
    ActionRule(name='action_061_modify_required_hearts', condition=lambda t: ('コストを' in t or 'コストが' in t or 'コストは' in t) and '{{heart_' in t, action='modify_required_hearts', setter=_handle_required_hearts),
    ActionRule(name='action_062_modify_cost', condition=lambda t: 'コストを' in t or 'コストが' in t or 'コストは' in t, action='modify_cost', setter=_set_action_062),
    ActionRule(name='action_063_repeat_procedure', match='繰り返してもよい', action='repeat_procedure', setter=_set_action_063),
    ActionRule(name='action_064_do_nothing', match='何もしない', action='do_nothing'),
    ActionRule(name='action_065_do_nothing', condition=lambda t: t.strip() == '', action='do_nothing'),
    ActionRule(name='action_066_pay_energy', match_all=['{{icon_energy.png|E}}', 'エネルギー'], action='pay_energy'),
    ActionRule(name='action_067_play_baton_touch', match_any=['バトンタッチ', 'baton touch'], action='play_baton_touch'),
    ActionRule(name='action_068_invalidate_ability', match='無効にできない', action='invalidate_ability', defaults={'optional': True}),
    ActionRule(name='action_069_modify_score', condition=lambda t: ('スコアは' in t or 'スコアが' in t) and ('になる' in t or 'なった' in t or 'なっている' in t), action='modify_score', setter=_set_action_069),
    ActionRule(name='action_070_modify_score', match='スコアを', action='modify_score', setter=_set_action_070),
    ActionRule(name='action_071_move_cards', match_any=['デッキの上に置き', 'デッキの上に置く'], action='move_cards', setter=_set_action_071),
    ActionRule(name='action_072_modify_yell_count', match_all=['エール'], match_any=['枚数', '数'], action='modify_yell_count'),
    ActionRule(name='action_073_gain_ability_from_source', condition=lambda t: '持つ' in t and '能力' in t and ('得る' in t) and ('すべて' in t), action='gain_ability_from_source', setter=_set_action_073),
    ActionRule(name='action_074_gain_ability', condition=lambda t: '得る' in t and any((kw in t for kw in ('能力', '常時', 'ライブ成功時', 'ライブ開始時', '登場', '起動'))), action='gain_ability', setter=_set_action_074),
    ActionRule(name='action_075_reduce_live_card_set_limit', match_all=['ライブカードセットフェイズ', '減る'], match_any=['上限', '枚数'], action='reduce_live_card_set_limit'),
    ActionRule(name='action_076_set_card_identity', match_any=['セット', '設定'], exclude='コスト', action='set_card_identity'),
    ActionRule(name='action_077_choose_required_hearts', match='必要ハートを選ぶ', action='choose_required_hearts'),
    ActionRule(name='action_078_all_blade_timing', match_all=['必要ハートを確認する時', 'ALLブレード', '任意の色のハートとして扱う'], action='all_blade_timing', defaults={'timing': 'check_required_hearts', 'treat_as': 'any_heart_color'}),
    ActionRule(name='action_079_set_card_identity', match_all=['すべての領域にあるこのカードは', 'として扱う'], action='set_card_identity', setter=_set_action_079),
    ActionRule(name='action_080_perform_yell', condition=lambda t: bool(re.search('追加で.*エール.*行', t)), action='perform_yell', setter=_set_action_080),
    ActionRule(name='action_081_conditional_alternative', match_all=['代わりに', '置く', '場合'], action='conditional_alternative', setter=_set_action_081),
    ActionRule(name='action_082_gain_resource', condition=lambda t: bool(re.search('［[^］]+ハート］', t)), action='gain_resource', setter=_set_heart_selection_resource),
]
