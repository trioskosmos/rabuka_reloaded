"""
Parser v3: A declarative, data-driven replacement for the monolithic parser.py.
Focuses on patterns and rules to provide comparable depth with 97% fewer lines of code.
"""

import re
import json
from typing import Dict, Any, List, Optional, Tuple

class AbilityParserV3:
    def __init__(self):
        # 1. Lexicon - Japanese terms to Engine Codes
        self.LOCATIONS = {
            "成功ライブカード置き場": "success_live_zone", "ライブカード置き場": "live_card_zone",
            "控え室": "discard", "手札": "hand", "ステージ": "stage", "デッキ": "deck",
            "エネルギーデッキ": "energy_deck", "エネルギー置き場": "energy_zone"
        }
        self.CARD_TYPES = {"メンバーカード": "member_card", "メンバー": "member_card", "ライブカード": "live_card", "エネルギーカード": "energy_card"}
        self.TARGETS = {"自分の": "self", "相手の": "opponent", "自分と相手の": "both", "自分か相手の": "either"}
        self.DURATIONS = {"ライブ終了時まで": "live_end", "ライブ終了まで": "live_end", "このターンの間": "this_turn", "ターン終了時まで": "turn_end", "このライブの間": "this_live"}
        self.OPERATORS = {"以上": ">=", "以下": "<=", "未満": "<", "超": ">", "より多い": ">", "より少ない": "<"}

        # 2. Declarative Extraction Rules
        self.RULES = [
            (r"(\d+)枚", "count", lambda m: int(m.group(1))),
            (r"(\d+)人", "count", lambda m: int(m.group(1))),
            (r"(\d+)つ", "count", lambda m: int(m.group(1))),
            (r"『(.+?)』", "group_names", lambda m: [m.group(1)]),
            (r"「(.+?)」", "characters", lambda m: [m.group(1)]),
            (r"コスト(\d+)", "cost_limit", lambda m: int(m.group(1))),
            (r"(以上|以下|未満|超|より多い|より少ない)", "operator", lambda m: self.OPERATORS.get(m.group(1), m.group(1))),
            (r"スコアを[+＋](\d+)", "score_value", lambda m: int(m.group(1))),
        ]

        self.STEMS = [
            ("引く", "draw_card"), ("引き", "draw_card"), ("引い", "draw_card"),
            ("加える", "move_cards"), ("加え", "move_cards"),
            ("置く", "move_cards"), ("置き", "move_cards"), ("置いて", "move_cards"), ("送る", "move_cards"),
            ("登場", "move_cards"), ("出す", "move_cards"), ("出し", "move_cards"),
            ("得る", "gain_resource"), ("得て", "gain_resource"), ("得られる", "gain_resource"),
            ("にする", "change_state"), ("にし", "change_state"), ("アクティブ", "change_state"),
            ("見る", "look_at"), ("見て", "look_at"),
            ("選ぶ", "select"), ("選び", "select"), ("選ん", "select"),
            ("公開", "reveal"), ("シャッフル", "shuffle"), ("入れ替える", "swap"),
            ("プラス", "modify_score"), ("マイナス", "modify_score"), ("増やす", "modify_limit"), ("減らす", "modify_limit"),
            ("無効", "invalidate_ability"), ("ポジションチェンジ", "position_change"), ("エール", "yell")
        ]

    def _normalize(self, text: str) -> str:
        trans = str.maketrans("０１２３４５６７８９＋：", "0123456789+:")
        return text.translate(trans).strip()

    def _extract_fields(self, text: str, data: Dict):
        for pattern, field, transform in self.RULES:
            for match in re.finditer(pattern, text):
                val = transform(match)
                if field in ["group_names", "characters"]:
                    data.setdefault(field, [])
                    if val[0] not in data[field]: data[field].extend(val)
                elif field == "score_value":
                    data["action"] = "modify_score"
                    data["value"] = val
                    data["operation"] = "add"
                elif field == "operator":
                    if "operator" not in data: data["operator"] = val
                else: data[field] = val

        if "E" in text:
            data["energy"] = text.count("E")
            data["action"] = "pay_energy"
            data["type"] = "pay_energy"
            data["zone"] = "energy_zone"

        for kw, code in self.LOCATIONS.items():
            if kw + "から" in text or kw + "にある" in text:
                source = code
                if kw == "デッキ" and "の上" in text: source = "deck_top"
                if kw == "デッキ" and "の下" in text: source = "deck_bottom"
                data["source"] = source
            if any(s in text for s in [kw + "に置", kw + "に加", kw + "に登", kw + "に送"]):
                dest = code
                if kw == "デッキ" and "の上" in text: dest = "deck_top"
                if kw == "デッキ" and "の下" in text: dest = "deck_bottom"
                data["destination"] = dest

        for kw, code in self.CARD_TYPES.items():
            if kw in text: data.setdefault("card_type", code)
        for kw, code in self.TARGETS.items():
            if kw in text: data["target"] = code
        for kw, code in self.DURATIONS.items():
            if kw in text: data["duration"] = code

        if "ブレード" in text:
            data.setdefault("resource", "blade")
            bc = text.count("ブレード")
            if bc > 0: data["count"] = bc
        if "ハート" in text:
            data.setdefault("resource", "heart")
        if "もよい" in text: data["optional"] = True
        if "好きな枚数" in text: data["any_number"] = True
        if any(x in text for x in ["このメンバー", "このカード"]):
            data["self_cost"] = True
            data["self_target"] = True

        if "を得る" in text and "「" in text:
            data["action"] = "gain_ability"
            m = re.search(r"「(.+?)」", text)
            if m: data["ability_gain"] = m.group(1)

    def parse_fragment(self, text: str) -> Dict:
        data = {"text": text.strip(), "action": "custom"}
        self._extract_fields(text, data)
        if data["action"] in ["custom", "pay_energy"]:
            for stem, action in self.STEMS:
                if stem in text:
                    data["action"] = action
                    break
        if "その中から" in text:
            parts = text.split("その中から", 1)
            data.update({"action": "look_and_select", "look_action": self.parse_fragment(parts[0]), "select_action": self.parse_fragment(parts[1])})
            if "残りを控え室に置く" in text:
                data["select_action"]["discard_remaining"] = True
        return {k: v for k, v in data.items() if v is not None}

    def parse_condition(self, text: str) -> Dict:
        if "あり" in text or "かつ" in text:
            parts = re.split(r"あり[、，]|かつ", text)
            conds = []
            for p in parts:
                c = {"text": p.strip(), "type": "condition"}
                self._extract_fields(p, c)
                conds.append(c)
            return {"text": text, "type": "compound", "operator": "and", "conditions": conds}
        else:
            c = {"text": text, "type": "condition"}
            self._extract_fields(text, c)
            return c

    def parse_ability(self, text: str) -> Dict:
        normalized = self._normalize(text)
        ability = {"full_text": text, "triggers": [], "cost": None, "effect": None}

        icons = re.findall(r"{{.+?\|(.+?)}}", normalized)
        ability["triggers"] = [t for t in icons if t not in ["E", "ブレード"]]
        clean_text = re.sub(r"{{.+?\|(.+?)}}", r"\1", normalized)

        parts = clean_text.split(":", 1)
        if len(parts) > 1:
            ability["cost"] = self.parse_fragment(parts[0])
            effect_part = parts[1]
        else: effect_part = parts[0]

        if "以下から1つを選ぶ" in effect_part:
            main, options_raw = effect_part.split("以下から1つを選ぶ", 1)
            options = [self.parse_fragment(o) for o in re.split(r"[・\n]", options_raw) if o.strip()]
            ability["effect"] = {"action": "choice", "text": "以下から1つを選ぶ", "options": options}
            return ability

        actions = []
        for s in [s.strip() for s in re.split(r"[。]", effect_part) if s.strip()]:
            markers = ["場合", "とき", "なら"]
            split_idx = -1
            for m in markers:
                idx = s.rfind(m + "、")
                if idx == -1: idx = s.rfind(m + "，")
                if idx > split_idx: split_idx = idx + len(m) + 1

            if split_idx > 0:
                cond_text, action_text = s[:split_idx-1], s[split_idx:].strip()
                act = self.parse_fragment(action_text)
                act["condition"] = self.parse_condition(cond_text)
                actions.append(act)
                continue

            frags = [f.strip() for f in re.split(r"[、，]", s) if f.strip()]
            if len(frags) > 1 and any(f.endswith(("し", "て", "引き", "置き")) for f in frags[:-1]):
                actions.extend([self.parse_fragment(f) for f in frags])
            else:
                actions.append(self.parse_fragment(s))

        if len(actions) > 1:
            merged = []
            i = 0
            while i < len(actions):
                if i < len(actions) - 1 and actions[i].get("action") == "look_at" and actions[i+1].get("action") == "look_and_select":
                    if not actions[i+1].get("look_action") or not actions[i+1]["look_action"].get("source"):
                        combined = actions[i+1].copy()
                        combined["look_action"] = actions[i]
                        combined["text"] = actions[i]["text"] + "。" + actions[i+1]["text"]
                        merged.append(combined)
                        i += 2
                        continue
                merged.append(actions[i])
                i += 1
            actions = merged

        ability["effect"] = actions[0] if len(actions) == 1 else {"action": "sequential", "actions": actions}
        return ability

if __name__ == "__main__":
    p = AbilityParserV3()
    tests = [
        "このメンバーをステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える。",
        "{{toujyou.png|登場}}カードを1枚引き、手札を1枚控え室に置く。",
        "手札を1枚控え室に置いてもよい：自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く。"
    ]
    for t in tests:
        print(json.dumps(p.parse_ability(t), ensure_ascii=False, indent=2))
