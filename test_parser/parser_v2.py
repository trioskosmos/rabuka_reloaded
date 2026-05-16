from typing import Any, Dict, List, Optional, Tuple, Union
from grammar import Str, Regex, OneOf, Seq, Opt, Many, Map, Capture, Parser, token, ParseError
from models import Ability, Action, Cost, Condition, UnknownAction
from patterns import get_patterns


class AbilityParser:
    def __init__(self):
        ws_parser = Regex(r"\s*")
        self.p = get_patterns(ws_parser)
        self.t = lambda p: token(p, ws_parser)

        # --- Atoms ---
        self.period = self.t(Str("。"))
        self.comma = self.t(Str("、"))
        self.colon = self.t(Str("："))
        
        # --- Patterns ---
        self.any_icon = self.t(self.p["any_icon"])
        self.trigger_icon = self.t(self.p["trigger_icon"])
        self.energy_icon = self.t(self.p["energy_icon"])
        self.verb = self.t(self.p["verb"])
        self.subject = self.t(self.p["subject"])
        self.source = self.t(self.p["source"])
        self.destination = self.t(self.p["destination"])
        self.location = self.t(self.p["location"])
        self.count = self.t(self.p["count"])
        self.card_type = self.t(self.p["card_type"])
        self.group = self.t(self.p["group"])
        self.name = self.t(self.p["name"])
        self.temporal = self.t(self.p["temporal"])
        self.operator = self.t(self.p["operator"])
        self.particle = self.t(self.p["particle"])
        self.number = self.t(self.p["number"])
        self.heart_icon = self.t(self.p["heart_icon"])
        self.cost_limit = self.t(self.p["cost_limit"])

        # --- Bag of Parts ---
        self.generic_part = OneOf(
            self.subject, self.source, self.destination, self.location,
            self.count, self.card_type, self.group, self.name,
            self.temporal, self.operator, self.particle, self.any_icon,
            self.heart_icon, self.trigger_icon, self.cost_limit,
            self.t(Str("の")), self.t(Str("ある")), self.t(Str("いる")), 
            self.t(Str("した")), self.t(Str("された")), self.t(Str("その")),
            self.t(Str("これにより")), self.t(Str("につき")), self.t(Str("ため")),
            self.t(Str("残りを")), self.t(Str("好きな")), self.t(Str("好きな枚数を")),
            self.t(Str("コスト")), self.number,
            Map(Regex(r"「([^」]+)」"), lambda x: f"nested:{x[1]}")
        )

        # --- Assembly Helpers ---

        def get_full_verb(v_tuple):
            base, suffixes = v_tuple
            return base + "".join(suffixes)

        def enrich_fields(data, parts):
            for p in parts:
                if isinstance(p, dict):
                    if p.get("type") == "count":
                        data["count"] = p["value"]
                        if p.get("unit"): data["unit"] = p["unit"]
                elif isinstance(p, tuple) and len(p) == 3: # Cost limit (cost, num, op)
                    if p[0] == "コスト":
                        data["card_property"] = f"cost_{p[2]}_{p[1]}"
                elif isinstance(p, str):
                    if p.startswith("nested:"):
                        data["nested_text"] = p[7:]
                    elif p.endswith("から") or p.endswith("にある"):
                        data["source"] = p
                    elif p.endswith("に"):
                        data["destination"] = p
                    elif p in ["自分", "相手", "このメンバー"]:
                        data["target"] = p
                    elif "カード" in p or "メンバー" in p or "ライブ" in p:
                        data["card_type"] = p
                    elif p in ["以上", "以下", "より多い", "より少ない", "超", "未満"]:
                        data["comparison_operator"] = p
                    elif p.startswith("『"):
                        if "group_names" not in data: data["group_names"] = []
                        data["group_names"].append(p.strip("『』"))
                    elif p in ["ライブ終了時まで", "ライブ終了まで", "このターンの間", "ターン終了時まで"]:
                        data["duration"] = p
                    elif "以外" in p:
                        data["exclude_self"] = True

        def assemble_action(parts, verb_tuple, raw_text):
            full_verb = get_full_verb(verb_tuple)
            data = {"text": raw_text.strip()}
            if "もよい" in full_verb: data["optional"] = True
            
            enrich_fields(data, parts)
            base = verb_tuple[0]
            if any(x in base for x in ["加え", "置", "登場", "出し"]):
                return Action(type="move_cards", **{k:v for k,v in data.items() if k in Action.model_fields})
            elif "引" in base:
                return Action(type="draw_card", count=data.get("count", 1), text=data["text"])
            elif any(x in base for x in ["得", "する"]):
                return Action(type="gain_resource", text=data["text"])
            elif any(x in base for x in ["見", "公開"]):
                return Action(type="look_at", count=data.get("count", 1), text=data["text"])
            elif "選" in base:
                return Action(type="select_cards", count=data.get("count", 1), text=data["text"])
            else:
                return Action(type="simple_action", text=data["text"])

        def assemble_condition(parts, terminator, raw_text):
            data = {"text": raw_text.strip(), "type": "comparison"}
            enrich_fields(data, parts)
            return Condition(**{k:v for k,v in data.items() if k in Condition.model_fields})

        # --- Sentences ---

        self.energy_cost = Map(
            Capture(Seq(self.energy_icon, Many(self.energy_icon))),
            lambda x: Cost(type="pay_energy", value=1 + len(x[0][1]), text=x[1].strip())
        )

        self.condition_sentence = Map(
            Capture(Seq(Many(self.generic_part), self.t(OneOf(Str("場合"), Str("とき"), Str("なら"))))),
            lambda x: assemble_condition(x[0][0], x[0][1], x[1])
        )

        self.action_sentence = Map(
            Capture(Seq(Many(self.generic_part), self.verb)),
            lambda x: assemble_action(x[0][0], x[0][1], x[1])
        )

        self.action_cost_sentence = Map(
            Capture(Seq(self.action_sentence, self.colon)),
            lambda x: x[0][0] # Return the Action from the sequence, but keep it as cost
        )

        # --- The Noise-Resilient Bag ---
        self.trash = Map(Regex(r"."), lambda x: None)

        self.ability_block = OneOf(
            self.trigger_icon,
            self.energy_cost,
            self.action_cost_sentence,
            self.condition_sentence,
            self.action_sentence,
            self.colon, self.comma, self.period,
            self.trash
        )

        self.ability_rule = Many(self.ability_block)

    def parse_ability(self, text: str, debug: bool = False) -> Ability:
        original_text = text
        try:
            clean_text = text.strip()
            res, _ = self.ability_rule.parse(clean_text, debug=debug)
            
            triggers = []
            cost = None
            condition = None
            effects = []

            for item in res:
                if item is None: continue
                
                if isinstance(item, str):
                    if item.startswith("{{") and "png" in item:
                        trigger_text = item.split("|")[-1].strip("}")
                        if trigger_text not in triggers:
                            triggers.append(trigger_text)
                elif isinstance(item, Cost):
                    cost = item
                elif isinstance(item, Condition):
                    condition = item
                elif isinstance(item, Action):
                    effects.append(item)

            return Ability(
                cost=cost,
                condition=condition,
                effects=effects,
                raw_text=original_text
            )
        except Exception as e:
            if debug: print(f"Parse failed: {e}")
            return Ability(
                effects=[UnknownAction(raw_text=text, text=f"error_{str(e)[:40]}")],
                raw_text=original_text
            )


if __name__ == "__main__":
    parser = AbilityParser()
    test_texts = [
        "{{toujyou.png|登場}}自分のステージにコスト13以上のメンバーがいる場合、カードを1枚引く。",
    ]
    for text in test_texts:
        print(f"\nParsing: {text}")
        result = parser.parse_ability(text, debug=False)
        print(result.model_dump_json(indent=2, ensure_ascii=False))
