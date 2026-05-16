from typing import Any, Dict, List, Optional, Tuple, Union
from grammar import Str, Regex, OneOf, Seq, Opt, Many, Map, Capture, Parser, token, ParseError
from models import Ability, Action, Cost, Condition, UnknownAction
from patterns import get_patterns

# Mark Action as Cost
class ActionCostWrapper:
    def __init__(self, action):
        self.action = action

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
            self.t(Str("コスト")), self.number, self.t(Str("以外")),
            self.t(Str("合計")), self.t(Str("のみ")), self.t(Str("枚につき")),
            Map(Regex(r"「([^」]+)」"), lambda x: f"bracketed:{x[1]}")
        )

        # --- Assembly Helpers ---

        def get_full_verb(v_tuple):
            base, suffixes = v_tuple
            return base + "".join(suffixes)

        def is_ability_text(text):
            # Expanded ability detection
            keywords = ["。", "、", "：", "する", "を得る", "引く", "置く", "加える", "登場", "移動"]
            return any(x in text for x in keywords)

        def enrich_fields(data, parts):
            for i, p in enumerate(parts):
                if isinstance(p, dict):
                    if p.get("type") == "count":
                        data["count"] = p["value"]
                        if p.get("unit"): data["unit"] = p["unit"]
                elif isinstance(p, tuple) and len(p) == 3: # Cost limit
                    data["cost_limit"] = p[1]
                    data["comparison_operator"] = p[2]
                elif isinstance(p, str):
                    if p.startswith("bracketed:"):
                        content = p[10:]
                        if is_ability_text(content):
                            if "actions" not in data: data["actions"] = []
                            data["actions"].append(Action(type="nested_ability", text=content))
                        else:
                            if "characters" not in data: data["characters"] = []
                            data["characters"].append(content)
                    elif p.endswith("から") or p.endswith("にある"):
                        data["source"] = p
                    elif p.endswith("に") or p.endswith("置き場") or p.endswith("ゾーン"):
                        data["location"] = p if not data.get("location") else data["location"]
                        data["destination"] = p if p.endswith("に") else data.get("destination")
                    elif p in ["自分", "相手", "このメンバー"]:
                        data["target"] = p
                    elif "カード" in p or "メンバー" in p or "ライブ" in p:
                        if not data.get("card_type"): data["card_type"] = p
                    elif p in ["以上", "以下", "より多い", "より少ない", "超", "未満"]:
                        data["comparison_operator"] = p
                    elif p.startswith("『"):
                        if "group_names" not in data: data["group_names"] = []
                        data["group_names"].append(p.strip("『』"))
                    elif p in ["ライブ終了時まで", "ライブ終了まで", "このターンの間", "ターン終了時まで"]:
                        data["duration"] = p
                        data["temporal"] = p
                    elif p == "以外":
                        data["exclude_self"] = True
                    elif p == "合計":
                        data["aggregate"] = "total"
                    elif p == "のみ":
                        data["all_members"] = True
                    elif p == "枚につき":
                        # Per-unit logic: usually preceded by a count or condition
                        data["per_unit"] = True
                        if i > 0 and isinstance(parts[i-1], str) and "カード" in parts[i-1]:
                            data["per_unit_type"] = parts[i-1]

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
            if any(x in raw_text for x in ["いない", "持たない", "なかった"]):
                data["negation"] = True
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
            Seq(self.action_sentence, self.colon),
            lambda x: ActionCostWrapper(x[0])
        )

        # Compound Sentence (XかつY)
        self.compound_condition = Map(
            Capture(Seq(self.condition_sentence, self.t(OneOf(Str("かつ"), Str("または"), Str("、"))), self.condition_sentence)),
            lambda x: Condition(type="compound", text=x[1], conditions=[x[0][0], x[0][2]], logical_operator="and" if "かつ" in x[0][1] else "or")
        )

        # --- The Bag ---
        self.trash = Map(Regex(r"."), lambda x: None)

        self.ability_block = OneOf(
            self.trigger_icon,
            self.energy_cost,
            self.action_cost_sentence,
            self.compound_condition,
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
            costs = []
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
                    costs.append(item)
                elif isinstance(item, ActionCostWrapper):
                    costs.append(item.action)
                elif isinstance(item, Condition):
                    condition = item
                elif isinstance(item, Action):
                    effects.append(item)

            final_cost = costs[0] if len(costs) == 1 else (costs if costs else None)

            return Ability(
                cost=final_cost,
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
