import re
from typing import Dict, List, Tuple

SOURCE_PATTERNS: List[Tuple[str, str]] = [
    ("デッキの一番上からカードを", "deck_top"),
    ("デッキの一番上のカードを", "deck_top"),
    ("これにより公開されたほかのすべてのカードを", "revealed_remaining"),
    ("これにより公開したカードを", "revealed_cards"),
    ("公開したカードをすべて", "revealed_cards"),
    ("それらのカードの中から", "those_cards"),
    ("このカードを手札に加えてもよい", "revealed_card"),
    ("自分の成功ライブカード置き場にある", "success_live_zone"),
    ("エールにより公開された", "revealed_cards"),
    ("メンバーの下にある", "under_member"),
    ("メンバー1人の下にある", "under_member"),
    ("自分の控え室にある", "discard"),
    ("控え室からライブカード", "discard"),
    ("控え室を", "discard"),
    ("手札を", "hand"),
    ("手札の", "hand"),
    ("手札から", "hand"),
    ("デッキの一番下から", "deck_bottom"),
    ("デッキの上から", "deck_top"),
    ("エネルギーデッキから", "energy_deck"),
    ("デッキから", "deck"),
    ("山札から", "deck"),
    ("エネルギー置き場から", "energy_zone"),
    ("控え室にある", "discard"),
    ("控え室から", "discard"),
    ("相手の控え室にある", "discard"),
    ("相手の控え室から", "discard"),
    ("からライブカード", "discard"),
    ("ステージから", "stage"),
    ("ライブカード置き場から", "live_card_zone"),
    ("成功ライブカード置き場から", "success_live_zone"),
]

DESTINATION_PATTERNS: List[Tuple[str, str]] = [
    ("デッキの一番上に置いてもよい", "deck_top"),
    ("そのメンバーの下に置く", "under_member"),
    ("デッキの一番上か一番下に置く", "deck_top_or_bottom"),
    ("デッキの一番上か一番下に置き", "deck_top_or_bottom"),
    ("デッキの一番上か一番下に置いて", "deck_top_or_bottom"),
    ("山札の上に置く", "deck_top"),
    ("山札の下に置く", "deck_bottom"),
    ("ライブカード置き場に置いてもよい", "live_card_zone"),
    ("表向きでライブカード置き場に置く", "live_card_zone"),
    ("いたエリアに", "same_area"),
    ("置かれていたエリアに", "same_area"),
    ("控え室に送る", "discard"),
    ("デッキに戻す", "deck"),
    ("デッキの一番上から4枚目に置く", "deck_position_4"),
    ("デッキの一番上から4枚目に置き", "deck_position_4"),
    ("デッキの一番上に置く", "deck_top"),
    ("デッキの一番上に置き", "deck_top"),
    ("デッキの一番上に置いて", "deck_top"),
    ("デッキの上に置く", "deck_top"),
    ("デッキの上に置き", "deck_top"),
    ("デッキの上に置いて", "deck_top"),
    ("デッキの一番下に置く", "deck_bottom"),
    ("デッキの一番下に置いて", "deck_bottom"),
    ("デッキの一番下に置き", "deck_bottom"),
    ("デッキの下に置く", "deck_bottom"),
    ("デッキの下に置き", "deck_bottom"),
    ("デッキの下に置いて", "deck_bottom"),
    ("デッキに置く", "deck"),
    ("控え室に置く", "discard"),
    ("控え室に置いて", "discard"),
    ("控え室に置き", "discard"),
    ("枚控え室に置く", "discard"),
    ("枚控え室に置いて", "discard"),
    ("手札に加える", "hand"),
    ("手札に加えて", "hand"),
    ("手札に置く", "hand"),
    ("ステージに置く", "stage"),
    ("ステージに登場させる", "stage"),
    ("エネルギー置き場に置く", "energy_zone"),
    ("エネルギーゾーンに置く", "energy_zone"),
    ("エネルギー・デッキに置く", "energy_deck"),
    ("エネルギー・デッキに置いてもよい", "energy_deck"),
    ("成功ライブカード置き場に置く", "success_live_zone"),
    ("ライブカード置き場に置く", "live_card_zone"),
    ("メンバーのいないエリア", "empty_area"),
    ("そのメンバーがいたエリア", "same_area"),
    ("このメンバーの下に置く", "under_member"),
    ("このメンバーの下に置いて", "under_member"),
    ("このメンバーの下に置き", "under_member"),
]

STATE_CHANGE_PATTERNS: List[Tuple[str, str]] = [
    ("ウェイトにする", "wait"),
    ("ウェイトにしてもよい", "wait"),
    ("ウェイトにし", "wait"),
    ("ウェイト状態で置く", "wait"),
    ("ウェイト状態で登場させる", "wait"),
    ("アクティブにする", "active"),
]

LOCATION_PATTERNS: List[Tuple[str, str]] = [
    ("成功ライブカード置き場", "success_live_card_zone"),
    ("ライブカード置き場", "live_card_zone"),
    ("控え室", "discard"),
    ("手札", "hand"),
    ("ステージ", "stage"),
    ("デッキ", "deck"),
    ("エネルギーデッキ", "energy_deck"),
    ("エネルギー置き場", "energy_zone"),
]

CARD_TYPE_PATTERNS: List[Tuple[str, str]] = [
    ("メンバーカード", "member_card"),
    ("メンバー", "member_card"),
    ("ライブカード", "live_card"),
    ("エネルギーカード", "energy_card"),
]

OPERATOR_PATTERNS: List[Tuple[str, str]] = [
    ("以上", ">="),
    ("以下", "<="),
    ("より少ない", "<"),
    ("より多い", ">"),
    ("未満", "<"),
    ("超", ">"),
]

POSITION_KEYWORDS: Dict[str, str] = {
    "センターエリア": "center",
    "左サイドエリア": "left_side",
    "右サイドエリア": "right_side",
    "センター": "center",
    "左サイド": "left_side",
    "右サイド": "right_side",
    "正面": "front",
}

COMPARISON_TARGETS: Dict[str, str] = {
    "相手より": "opponent",
    "自分より": "self",
    "このメンバーより": "self",
}

COMPARISON_OPERATORS: Dict[str, str] = {
    "高い": ">",
    "低い": "<",
    "少ない": "<",
    "多い": ">",
    "大きい": ">",
    "小さい": "<",
}

COMPARISON_TYPES: Dict[str, str] = {
    "スコア": "score",
    "コスト": "cost",
}

DURATION_PREFIX_MAP: Dict[str, str] = {
    "ライブ終了時まで": "live_end",
    "ライブ終了まで": "live_end",
    "このターンの間": "this_turn",
    "このライブの間": "this_live",
    "ターン終了時まで": "turn_end",
    "そのターンの間": "turn_end",
}
