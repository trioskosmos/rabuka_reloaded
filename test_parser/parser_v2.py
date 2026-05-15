import json
from typing import List, Optional
from .grammar import Seq, Str, Re, OneOf, Opt, Many, Map, Ref, keyword, Rule
from .models import (
    Ability, MoveCardsCost, DrawCardAction, MoveCardsAction, LookAndSelectAction, 
    LookAtAction, SelectAction, PayEnergyCost, GainResourceAction, ChangeStateAction,
    SequentialAction, AnyCost, AnyAction, ModifyScoreAction, AppearAction, RevealAction,
    UnknownAction, UnknownCost, ChangeStateCost
)

# ==========================================
# 1. GRAMMAR DEFINITIONS (Using grammar.py)
# ==========================================

def make_grammar():
    # --- Basic Tokens ---
    number = Map(Re(r'\d+'), int)
    
    # Optional punctuation
    comma = OneOf(Str("、"), Str("，"))
    colon = OneOf(Str("："), Str(":"))
    period = OneOf(Str("。"), Str("．"))
    
    # Card Types
    ct_live = Map(Str("ライブカード"), lambda _: "live_card")
    ct_member = Map(Str("メンバーカード"), lambda _: "member_card")
    ct_energy = Map(Str("エネルギーカード"), lambda _: "energy_card")
    ct_card = Map(Str("カード"), lambda _: "card")
    card_type = OneOf(ct_live, ct_member, ct_energy, ct_card)
    
    # Sources
    src_deck_top = Map(Str("デッキの上から"), lambda _: "deck_top")
    src_deck = Map(Str("デッキから"), lambda _: "deck")
    src_discard = Map(Str("控え室から"), lambda _: "discard")
    src_stage = Map(Str("ステージから"), lambda _: "stage")
    src_energy_deck = Map(Str("エネルギーデッキから"), lambda _: "energy_deck")
    source = OneOf(src_deck_top, src_deck, src_discard, src_stage, src_energy_deck)
    
    # Counts
    count_cards = Map(Seq(number, Str("枚")), lambda x: x[0])
    count_people = Map(Seq(number, Str("人")), lambda x: x[0])
    
    # --- Cost Rules ---
    
    # 1. Discard hand: "手札を1枚控え室に置く" or "手札を1枚控え室に置いてもよい"
    discard_hand_cost = Seq(Opt(Str("手札を")), count_cards, Str("控え室に置"), OneOf(Str("く"), Str("いてもよい")))
    def map_discard_hand_cost(res):
        _hand, count, _put, suffix = res
        return MoveCardsCost(
            type="move_cards",
            text="".join(str(r) for r in res if r),
            source="hand",
            destination="discard",
            count=count,
            optional=(suffix == "いてもよい") or None
        )
        
    # 2. Stage to discard: "このメンバーをステージから控え室に置く"
    stage_to_discard_cost = Seq(Str("このメンバーを"), src_stage, Str("控え室に置く"))
    def map_stage_to_discard(res):
        return MoveCardsCost(
            type="move_cards",
            text="".join(str(r) for r in res if r),
            source="stage",
            destination="discard",
            count=1,
            card_type="member_card",
            self_cost=True
        )

    # 3. Pay energy: "{{icon_energy.png|E}}支払ってもよい" or "{{icon_energy.png|E}}{{icon_energy.png|E}}"
    energy_icon = Str("{{icon_energy.png|E}}")
    energy_pay_cost = Seq(Many(energy_icon), Opt(Str("支払ってもよい")))
    def map_energy_pay_cost(res):
        icons, optional = res
        if not icons: return None
        return PayEnergyCost(
            type="pay_energy",
            text="".join(icons) + (optional or ""),
            energy=len(icons),
            count=len(icons),
            optional=bool(optional) or None
        )
        
    # 4. Change State Cost: "このメンバーをウェイトにしてもよい"
    wait_cost = Seq(Str("このメンバーをウェイトにしてもよい"))
    def map_wait_cost(res):
        return ChangeStateCost(
            type="change_state",
            text=res[0],
            state_change="wait",
            card_type="member_card",
            optional=True,
            self_cost=True  # Usually implicit self cost
        )

    # 5. Unknown Cost Fallback
    unknown_cost = Re(r'[^：:]+')
    def map_unknown_cost(res):
        return UnknownCost(
            type="unknown_cost",
            text=res
        )

    cost_rule = OneOf(
        Map(discard_hand_cost, map_discard_hand_cost),
        Map(stage_to_discard_cost, map_stage_to_discard),
        Map(energy_pay_cost, map_energy_pay_cost),
        Map(wait_cost, map_wait_cost),
        Map(unknown_cost, map_unknown_cost)
    )
    
    # --- Effect Rules ---
    
    # 1. Draw: "カードをX枚引く" or "カードをX枚引き"
    draw_effect = Seq(Str("カードを"), count_cards, OneOf(Str("引く"), Str("引き")))
    def map_draw_effect(res):
        _, count, _ = res
        return DrawCardAction(
            action="draw_card",
            text=f"カードを{count}枚引く",
            count=count
        )
        
    # 2. Recover from discard: "自分の控え室から[type]をX枚手札に加える"
    recover_effect = Seq(Opt(Str("自分の")), src_discard, Opt(card_type), Opt(Str("を")), count_cards, Str("手札に加える"))
    def map_recover_effect(res):
        _my, _src, c_type, _wo, count, _add = res
        return MoveCardsAction(
            action="move_cards",
            text="".join(str(r) for r in res if r),
            source="discard",
            destination="hand",
            count=count,
            card_type=c_type or "card",
            target="self"
        )
        
    # 3. Discard from hand action: "手札をX枚控え室に置く"
    discard_action = Seq(Opt(Str("手札を")), count_cards, Str("控え室に置く"))
    def map_discard_action(res):
        _hand, count, _ = res
        return MoveCardsAction(
            action="move_cards",
            text="".join(str(r) for r in res if r),
            source="hand",
            destination="discard",
            count=count,
            card_type="card"
        )
        
    # 4. Gain Resource (Blades/Hearts): "ライブ終了時まで、{{icon_blade.png|ブレード}}...を得る"
    blade_icon = Map(Str("{{icon_blade.png|ブレード}}"), lambda _: "blade")
    heart_icon = Map(Re(r'\{\{heart_\d+\.png\|heart\d+\}\}|\{\{icon_all\.png\|ハート\}\}'), lambda _: "heart")
    resource_icon = OneOf(blade_icon, heart_icon)
    gain_resource_effect = Seq(Opt(Str("ライブ終了時まで、")), Many(resource_icon), Str("を得る"))
    def map_gain_resource(res):
        dur, icons, _ = res
        if not icons: return None
        # Group by resource type
        blades = icons.count("blade")
        hearts = icons.count("heart")
        if blades and not hearts:
            return GainResourceAction(action="gain_resource", text="".join(str(r) for r in res if r), resource="blade", count=blades, duration="live_end" if dur else None)
        elif hearts and not blades:
            return GainResourceAction(action="gain_resource", text="".join(str(r) for r in res if r), resource="heart", count=hearts, duration="live_end" if dur else None)
        else:
            # If both, we return a SequentialAction (or could make a complex GainResource)
            return SequentialAction(action="sequential", text="".join(str(r) for r in res if r), actions=[
                GainResourceAction(action="gain_resource", text="", resource="blade", count=blades, duration="live_end" if dur else None),
                GainResourceAction(action="gain_resource", text="", resource="heart", count=hearts, duration="live_end" if dur else None)
            ])
        
    # 5. Look and Select: 
    # Type A: その中からX枚を手札に加え、残りを控え室に置く
    # Type B: その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く
    look_select_pat = r'自分のデッキの上からカードを(\d+)枚見る。その中から((\d+)枚を手札に加え|好きな枚数を好きな順番でデッキの上に置き)、残りを控え室に置く'
    look_select_str = Re(look_select_pat)
    import re
    def map_look_select(res):
        m = re.match(look_select_pat, res)
        text = res
        look_cnt = int(m.group(1))
        look_act = LookAtAction(action="look_at", text=f"自分のデッキの上からカードを{look_cnt}枚見る", source="deck_top", count=look_cnt)
        
        if "手札に加え" in text:
            sel_cnt = int(m.group(3))
            sel_act = SelectAction(action="select_cards", text=f"その中から{sel_cnt}枚を手札に加え、残りを控え室に置く", destination="hand", count=sel_cnt, discard_remaining=True)
        else:
            sel_act = MoveCardsAction(action="move_cards", text="その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く", source="deck_top", destination="deck_top", any_number=True, placement_order="any_order")
            
        return LookAndSelectAction(
            action="look_and_select",
            text=text,
            look_action=look_act,
            select_action=sel_act
        )
        
    # 6. Change State Effect (Wait): "相手のステージにいるコスト4以下のメンバー1人をウェイトにする"
    wait_effect_pat = r'(相手のステージにいるコスト(\d+)以下のメンバー)(\d+)人をウェイトにする'
    wait_effect_str = Re(wait_effect_pat)
    def map_wait_effect(res):
        m = re.match(wait_effect_pat, res)
        text = res
        cost_limit = int(m.group(2))
        count = int(m.group(3))
        return ChangeStateAction(
            action="change_state",
            text=text,
            state_change="wait",
            card_type="member_card",
            target="opponent",
            count=count
        )

    # 7. Change State Effect (Active): "エネルギーを2枚アクティブにする"
    active_effect = Seq(Str("エネルギーを"), count_cards, Str("アクティブにする"))
    def map_active_effect(res):
        _, cnt, _ = res
        return ChangeStateAction(
            action="change_state",
            text="".join(str(r) for r in res if r),
            state_change="active",
            card_type="energy_card",
            count=cnt
        )
        
    # 8. Score Effect: "ライブの合計スコアを＋１する"
    score_effect = Seq(Opt(Str("ライブの合計")), Str("スコアを"), OneOf(Str("＋"), Str("+")), number, Str("する"))
    def map_score_effect(res):
        _p1, _p2, _op, val, _p3 = res
        return ModifyScoreAction(
            action="modify_score",
            text="".join(str(r) for r in res if r),
            operation="add",
            value=val
        )
        
    # 9. Appear: "このカードを控え室からステージに登場させる"
    appear_effect = Seq(Str("このカードを控え室からステージに登場させる"))
    def map_appear_effect(res):
        return AppearAction(
            action="appear",
            text=res[0],
            source="discard",
            destination="stage"
        )
        
    # 10. Reveal: "手札のライブカードを1枚公開し"
    reveal_effect = Seq(Str("手札の"), card_type, Str("を"), count_cards, OneOf(Str("公開し"), Str("公開する")))
    def map_reveal_effect(res):
        _hand, c_type, _o, cnt, _v = res
        return RevealAction(
            action="reveal",
            text="".join(str(r) for r in res if r),
            source="hand",
            count=cnt
        )

    # 11. Select Heart Color: "{{heart_01.png|heart01}}か...のうち、1つを選ぶ" or "好きなハートの色を1つ指定する"
    select_heart_pat = r'((?:\{\{heart_\d+\.png\|heart\d+\}\}か)+.*?のうち、1つを選ぶ|好きなハートの色を1つ指定する)'
    select_heart_effect = Re(select_heart_pat)
    def map_select_heart(res):
        return SelectAction(
            action="select",
            text=res
        )
        
    # 12. Gain Selected Heart Per Unit: "ライブ終了時まで、自分の成功ライブカード置き場にあるカード1枚につき、選んだハートを1つ得る"
    # Or "ライブ終了時まで、そのハートを1つ得る"
    gain_selected_heart_pat = r'(ライブ終了時まで、)?(自分の成功ライブカード置き場にあるカード1枚につき、)?(選んだハート|そのハート)を1つ得る'
    gain_selected_heart_effect = Re(gain_selected_heart_pat)
    def map_gain_selected_heart(res):
        m = re.match(gain_selected_heart_pat, res)
        text = res
        return GainResourceAction(
            action="gain_resource",
            text=text,
            resource="heart",
            count=1,
            duration="live_end" if "ライブ終了時まで" in text else None,
            per_unit=bool(m.group(2)),
            location="success_live_zone" if "成功ライブカード" in text else None,
            per_unit_count=1 if m.group(2) else None,
            per_unit_type="card" if m.group(2) else None
        )

    # 13. Unknown Effect Fallback
    unknown_effect = Re(r'[^、。]+')
    def map_unknown_effect(res):
        return UnknownAction(
            action="unknown",
            text=res
        )

    single_effect = OneOf(
        Map(draw_effect, map_draw_effect),
        Map(recover_effect, map_recover_effect),
        Map(discard_action, map_discard_action),
        Map(gain_resource_effect, map_gain_resource),
        Map(look_select_str, map_look_select),
        Map(wait_effect_str, map_wait_effect),
        Map(active_effect, map_active_effect),
        Map(score_effect, map_score_effect),
        Map(appear_effect, map_appear_effect),
        Map(reveal_effect, map_reveal_effect),
        Map(select_heart_effect, map_select_heart),
        Map(gain_selected_heart_effect, map_gain_selected_heart),
        Map(unknown_effect, map_unknown_effect)
    )
    
    # Handle sequential effects (e.g. Draw 1, discard 1) separated by "、"
    def map_seq_effects(res):
        first_eff, rest = res
        actions = [first_eff]
        for _, eff in rest:
            actions.append(eff)
        if len(actions) == 1:
            return actions[0]
        return SequentialAction(
            action="sequential",
            text="、".join(a.text for a in actions if a.text),
            actions=actions
        )
    sequential_effects = Map(Seq(single_effect, Many(Seq(comma, single_effect))), map_seq_effects)

    # Condition Rules
    # E.g. "登場している場合、"
    condition_rule = Re(r'([^、]+(場合|とき))')
    # For now, we will just parse the text out. A proper schema would map this to a Condition object.

    # Complete ability
    parenthetical = Re(r'（[^）]+）')
    ability_rule = Seq(Opt(Seq(cost_rule, colon)), Opt(Seq(condition_rule, comma)), sequential_effects, Opt(period), Opt(parenthetical))
    return ability_rule

# ==========================================
# 2. ORCHESTRATION & VALIDATION
# ==========================================

def parse_ability_v2(text: str) -> Ability:
    grammar = make_grammar()
    match = grammar.match(text)
    
    if not match:
        raise ValueError(f"Could not parse ability: {text}")
        
    end_pos, result = match
    # Require full consumption
    if end_pos < len(text):
        raise ValueError(f"Partial match only. Leftover: {text[end_pos:]}")
        
    cost_part, cond_part, effect_part, _period, _parenthetical = result
    
    cost = None
    if cost_part: 
        cost = cost_part[0]
        
    # We could attach condition to effect_part if it supports it, but for now we just parse it
    
    return Ability(
        triggerless_text=text,
        cost=cost,
        effect=effect_part
    )

if __name__ == "__main__":
    import os
    
    input_path = os.path.join(os.path.dirname(__file__), "..", "cards", "abilities.json")
    output_path = os.path.join(os.path.dirname(__file__), "test_abilities.json")
    
    with open(input_path, encoding="utf-8") as f:
        data = json.load(f)
        
    abilities = data.get("unique_abilities", [])
    results = []
    
    print(f"Testing parser_v2 on {len(abilities)} unique abilities...")
    success = 0
    for a in abilities:
        text = a["triggerless_text"]
        if not text:
            continue
        try:
            parsed = parse_ability_v2(text)
            res = json.loads(parsed.model_dump_json(exclude_none=True))
            res["status"] = "success"
            results.append(res)
            success += 1
        except Exception as e:
            results.append({
                "triggerless_text": text,
                "status": "failed",
                "error": str(e)
            })
            
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(results, f, ensure_ascii=False, indent=2)
        
    print(f"Successfully parsed {success} / {len(abilities)} abilities.")
    print(f"Wrote results to {output_path}")
