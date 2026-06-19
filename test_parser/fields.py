import re
from typing import Dict, Any, Optional, List

from test_parser.schema import (
    SOURCE_PATTERNS,
    DESTINATION_PATTERNS,
    STATE_CHANGE_PATTERNS,
    LOCATION_PATTERNS,
    CARD_TYPE_PATTERNS,
    OPERATOR_PATTERNS,
    POSITION_KEYWORDS,
    COMPARISON_TARGETS,
    COMPARISON_OPERATORS,
    COMPARISON_TYPES,
    DURATION_PREFIX_MAP,
)

REGEX_COUNT = re.compile(r"(\d+)(枚|人|つ|回|個|種類)")
REGEX_DECK_POS = re.compile(r"(?:一番上から|上から)(\d+)枚目")
REGEX_GROUP_NAME = re.compile(r"『([^』]+)』")
REGEX_QUOTED_TEXT = re.compile(r"「([^」]+)」")


def normalize_fullwidth_digits(text: str) -> str:
    fullwidth = "０１２３４５６７８９＋−－"
    halfwidth = "0123456789+--"
    return text.translate(str.maketrans(fullwidth, halfwidth))


def strip_parenthetical(text: str) -> str:
    text = re.sub(r"（[^）]*）", "", text)
    text = re.sub(r"\([^)]*\)", "", text)
    return text.strip()


def extract_by_pattern(text: str, patterns: list) -> Optional[str]:
    for pattern_str, code in patterns:
        if pattern_str in text:
            return code
    return None


class ExtractedFields:
    __slots__ = (
        "source",
        "destination",
        "state_change",
        "location",
        "locations",
        "card_type",
        "operator",
        "target",
        "count",
        "unit",
        "optional",
        "max",
        "any_number",
        "shuffle",
        "multiple_targets",
        "exclude_self",
        "self_cost",
        "self_target",
        "all",
        "distinct",
        "negation",
        "group_names",
        "quoted_text",
        "characters",
        "cost_limit",
        "cost_limit_operator",
        "blade_limit",
        "blade_limit_operator",
        "deck_position",
        "heart_colors",
        "energy_count",
        "blade_count",
        "comparison_target",
        "comparison_operator",
        "comparison_type",
        "aggregate_total",
        "non_stackable",
        "placement_order",
        "source_position",
        "exclude_position",
        "position",
        "original_value",
        "cost_total",
        "cost_total_operator",
        "has_condition_marker",
        "has_sequential",
        "has_per_unit",
        "has_choice",
        "has_duration",
        "has_alternative",
        "has_revealed_context",
        "same_unit_name",
        "duration_code",
        "activated_icons",
        "cond_markers",
        "seq_markers",
        "per_unit",
        "choice",
        "duration",
        "alternative",
        "baton_touch",
        "each_time",
        "furthermore",
        "unless_pay",
        "opponent_choice",
        "kore_niyori",
        "sou_shinai",
        "sou_shita",
        "activation_suffix",
    )

    def __init__(self, text: str):
        for s in self.__slots__:
            setattr(self, s, None)

        raw = text

        # Source
        self.source = extract_by_pattern(raw, SOURCE_PATTERNS)

        # Destination
        self.destination = extract_by_pattern(raw, DESTINATION_PATTERNS)
        if not self.destination:
            if re.search(r"デッキの一番上から(\d+)枚目に置", raw):
                self.destination = "deck"
            if "エネルギーカードを1枚ウェイト状態で置いてもよい" in raw:
                self.destination = "energy_zone"
            if (
                "メンバーのいないエリアに登場させる" in raw
                or "メンバーのいないエリアにウェイト状態で登場させる" in raw
            ):
                self.destination = "empty_area"
            elif "ウェイト状態で置く" in raw or (
                "エネルギーカードを" in raw and "置く" in raw
            ):
                self.destination = "energy_zone"
            elif "登場させる" in raw:
                self.destination = "stage"

        # State change (wait/active)
        self.state_change = extract_by_pattern(raw, STATE_CHANGE_PATTERNS)

        # Location (general)
        self.location = extract_by_pattern(raw, LOCATION_PATTERNS)

        # Multiple locations (と conjunction)
        locs = []
        for pat_str, loc_name in LOCATION_PATTERNS:
            if pat_str in raw:
                locs.append(loc_name)
        if "success_live_card_zone" in locs and "live_card_zone" in locs:
            locs = [l for l in locs if l != "live_card_zone"]
        if len(locs) >= 2:
            self.locations = locs

        # Override location for revealed cards context
        if "公開した" in raw or "公開された" in raw or "公開する" in raw:
            self.location = "revealed_cards"
        if "エールにより公開された" in raw:
            self.location = "revealed_cards"

        # Card type
        self.card_type = extract_by_pattern(raw, CARD_TYPE_PATTERNS)

        # Operator
        self.operator = extract_by_pattern(raw, OPERATOR_PATTERNS)

        # Target
        if (
            "自分と相手の" in raw
            or "自分と相手は" in raw
            or "自分と対戦相手の" in raw
            or "自分と対戦相手は" in raw
            or "自分と対戦相手" in raw
        ):
            self.target = "both"
        elif "自分か相手の" in raw:
            self.target = "either"
        elif "相手の" in raw:
            self.target = "opponent"
        elif "自分の" in raw:
            self.target = "self"

        # Count
        cm = re.search(REGEX_COUNT, raw)
        if cm:
            self.count = int(cm.group(1))
            self.unit = cm.group(2)

        # Cost limit
        clm = re.search(r"元々のコスト[がは](\d+)(?:以上|以下|未満|超)", raw)
        if not clm:
            clm = re.search(r"(\d+)コスト(?:以上|以下|未満|超)", raw)
        if not clm:
            clm = re.search(r"コスト(\d+)(?:以上|以下|未満|超|の)", raw)
        if clm:
            self.cost_limit = int(clm.group(1))
            for kw, op in [("以下", "<="), ("以上", ">="), ("未満", "<"), ("超", ">")]:
                if kw in raw:
                    self.cost_limit_operator = op
                    break
            if not self.cost_limit_operator:
                self.cost_limit_operator = "="

        # Optional
        self.optional = "もよい" in raw or "てもよい" in raw

        # Max
        self.max = "枚まで" in raw or "人まで" in raw

        # Any number
        self.any_number = (
            "好きな枚数" in raw
            or "好きな枚数まで" in raw
            or "任意の枚数" in raw
            or "好きな数" in raw
        )

        # Shuffle
        self.shuffle = "シャッフル" in raw

        # Multiple targets
        self.multiple_targets = "ずつ" in raw or "それぞれ" in raw

        # Exclude self
        self.exclude_self = (
            "このメンバー以外" in raw
            or bool(re.search(r"ほかの.*メンバー", raw))
            or bool(re.search(r"他の.*メンバー", raw))
        )

        # Self cost / self target
        self.self_cost = (
            "このメンバー" in raw
            and "このメンバー以外" not in raw
            and not bool(re.search(r"ほかの.*メンバー", raw))
            and bool(re.search(r"このメンバー[をが]", raw))
        )
        self.self_target = "このカード" in raw or (
            "このメンバー" in raw and "このメンバー以外" not in raw
        )

        # All
        if re.search(r"すべての|全ての|全部の|全て|全員|全体|カードをすべて", raw):
            self.all = True

        # Distinct names
        if "コストがそれぞれ異なる" in raw:
            self.distinct = "cost"
        elif any(
            kw in raw for kw in ["名前が異なる", "名前の異なる", "カード名が異なる"]
        ):
            self.distinct = "card_name"
        elif "グループ名が異なる" in raw:
            self.distinct = "group_name"

        # Negation
        self.negation = (
            bool(re.search(r"がない", raw))
            or bool(re.search(r"が\d*ない", raw))
            or "いない" in raw
            or "を持たない" in raw
        )

        # Group names
        self.group_names = re.findall(r"『([^』]+)』", raw)
        if not self.group_names:
            self.group_names = []

        # Quoted text and characters
        all_quoted = re.findall(r"「([^」]+)」", raw)
        self.quoted_text = all_quoted
        self.characters = [q for q in all_quoted if "{{" not in q and len(q) <= 10]

        # Deck position
        dm = re.search(REGEX_DECK_POS, raw)
        if dm:
            self.deck_position = int(dm.group(1))

        # Heart colors from icons
        heart_matches = re.findall(r"heart_(\d+)", raw)
        if heart_matches:
            self.heart_colors = sorted(set(f"heart{m.zfill(2)}" for m in heart_matches))
        else:
            self.heart_colors = []

        # Energy count
        ec = raw.count("{{icon_energy.png|E}}")
        if ec:
            self.energy_count = ec

        # Blade count
        bc = raw.count("{{icon_blade.png|ブレード}}")
        if bc:
            self.blade_count = bc

        # Comparison target/operator/type
        for txt, tgt in COMPARISON_TARGETS.items():
            if txt in raw:
                self.comparison_target = tgt
                break
        for txt, op in COMPARISON_OPERATORS.items():
            if txt in raw:
                self.comparison_operator = op
                break
        for txt, ct in COMPARISON_TYPES.items():
            if txt in raw:
                self.comparison_type = ct
                break

        # Aggregate total
        self.aggregate_total = "合計" in raw or None

        # Non-stackable
        self.non_stackable = "この効果は重複しない" in raw or None

        # Placement order
        self.placement_order = "any_order" if "好きな順番で" in raw else None

        # Position keywords
        matched_positions = set()
        for kw, pos in POSITION_KEYWORDS.items():
            if kw in raw:
                matched_positions.add(pos)
        if len(matched_positions) == 1:
            self.position = matched_positions.pop()
        elif len(matched_positions) > 1:
            positions_list = sorted(matched_positions)
            self.position = positions_list[0]
            self.exclude_position = positions_list[1]
        if "センターエリア以外" in raw or "センター以外" in raw:
            self.exclude_position = "center"
            self.position = None

        # Source position (center → wait, etc.)
        if "センターにいる" in raw:
            self.source_position = "center"

        # Original value
        self.original_value = "元々持つ" in raw or "元々" in raw or None

        # Same unit name
        self.same_unit_name = "同じユニット名" in raw or None

        # Structural markers
        self.per_unit = "につき" in raw or "ごとに" in raw
        self.choice = "以下から1つを選ぶ" in raw
        self.baton_touch = "バトンタッチ" in raw
        self.each_time = "たび" in raw
        self.furthermore = "さらに" in raw
        self.unless_pay = "しないかぎり" in raw or "ないかぎり" in raw
        self.opponent_choice = (
            "相手は" in raw and "てもよい" in raw and "そうしなかった" in raw
        )
        self.kore_niyori = "これにより" in raw
        self.sou_shinai = "そうしなかった場合" in raw
        self.sou_shita = "そうした場合" in raw
        self.alternative = "代わりに" in raw
        self.activation_suffix = "この能力は" in raw and (
            "場合のみ" in raw or "起動できる" in raw or "発動する" in raw
        )

        # Duration
        for pat_str, code in DURATION_PREFIX_MAP.items():
            if raw.startswith(pat_str) or pat_str in raw:
                self.duration_code = code
                self.duration = True
                break

        # Revealed context
        self.has_revealed_context = (
            "エールにより公開された" in raw
            or "これにより公開した" in raw
            or "これにより公開された" in raw
        )

        # Condition markers
        self.cond_markers = []
        if "場合" in raw:
            self.cond_markers.append("場合")
        if "とき" in raw:
            self.cond_markers.append("とき")
        if "なら" in raw:
            self.cond_markers.append("なら")
        if "時、" in raw:
            self.cond_markers.append("時")

        # Sequential markers
        self.seq_markers = []
        if "その後、" in raw:
            self.seq_markers.append("その後")
        if "、" in raw:
            self.seq_markers.append("、")
        if "。" in raw:
            self.seq_markers.append("。")

        # Position icons from activation area
        icons = []
        if "{{center.png|センター}}" in raw:
            icons.append("center")
        if "{{left.png|左サイド}}" in raw:
            icons.append("left_side")
        if "{{right.png|右サイド}}" in raw:
            icons.append("right_side")
        self.activated_icons = icons

        # Blade limit from text
        clean_blade = re.sub(r"\{\{icon_blade\.png\|ブレード\}\}", "ブレード", raw)
        bm = re.search(
            r"ブレード[の]数[がは](\d+)[つ個](以下|以上|未満|超)", clean_blade
        )
        if bm:
            self.blade_limit = int(bm.group(1))
            op_map = {"以下": "<=", "以上": ">=", "未満": "<", "超": ">"}
            self.blade_limit_operator = op_map.get(bm.group(2), "==")
        else:
            bm2 = re.search(r"ブレード[の]数[がは]ちょうど(\d+)[つ個]", clean_blade)
            if bm2:
                self.blade_limit = int(bm2.group(1))
                self.blade_limit_operator = "=="

    def to_dict(self) -> Dict[str, Any]:
        result = {}
        for s in self.__slots__:
            v = getattr(self, s)
            if v is not None and v != [] and v != "" and v != {}:
                result[s] = v
        return result
