from typing import Any, Callable, List, Optional, Tuple, Union
from grammar import Str, Regex, OneOf, Seq, Opt, Many, Map, Parser, token


def get_patterns(ws_parser):
    # Helper for tokens
    def t(p):
        return token(p, ws_parser)

    # --- 1. Numbers & Counters ---
    digit = Regex(r"[0-9０-９]+")
    def normalize_number(n_str):
        return int(n_str.translate(str.maketrans("０１２３４５６７８９", "0123456789")))

    number = Map(digit, normalize_number)

    count_patterns = OneOf(
        Map(Seq(number, Str("枚")), lambda x: {"type": "count", "value": x[0], "unit": "枚"}),
        Map(Seq(number, Str("人")), lambda x: {"type": "count", "value": x[0], "unit": "人"}),
        Map(Seq(number, Str("つ")), lambda x: {"type": "count", "value": x[0], "unit": "つ"}),
        Map(Seq(number, Str("個")), lambda x: {"type": "count", "value": x[0], "unit": "個"}),
        Map(Seq(number, Str("回")), lambda x: {"type": "count", "value": x[0], "unit": "回"}),
        Map(Str("全"), lambda x: {"type": "count", "value": -1, "unit": "all"}),
        Map(Str("すべて"), lambda x: {"type": "count", "value": -1, "unit": "all"}),
        Map(Str("好きな枚数"), lambda x: {"type": "count", "value": -1, "unit": "any"}),
    )

    # --- 2. Icons ---
    trigger_icons = OneOf(
        Regex(r"{{toujyou\.png\|登場}}"),
        Regex(r"{{kidou\.png\|起動}}"),
        Regex(r"{{jidou\.png\|自動}}"),
        Regex(r"{{jyouji\.png\|常時}}"),
        Regex(r"{{live_start\.png\|ライブ開始時}}"),
        Regex(r"{{live_success\.png\|ライブ成功時}}"),
        Regex(r"{{turn(\d+)\.png\|ターン\d+回}}"),
    )
    energy_icon = Regex(r"{{icon_energy\.png\|E}}")
    heart_icon = Regex(r"{{heart_(\d+)\.png\|heart\d+}}")
    blade_icon = Regex(r"{{icon_blade\.png\|ブレード}}")
    any_icon = Regex(r"{{[^|]+\|([^}]+)}}")

    # --- 3. Lexicon ---
    major_units = OneOf(
        Str("μ's"), Str("Aqours"), Str("Liella!"), Str("Printemps"), 
        Str("lilywhite"), Str("BiBi"), Str("CYaRon！"), Str("AZALEA"), 
        Str("GuiltyKiss"), Str("A・ZU・NA"), Str("QU4RTZ"), Str("DiverDiva"),
        Str("R3BIRTH"), Str("DOLLCHESTRA"), Str("スリーズブーケ"), Str("みらくらぱーく！")
    )

    zones = OneOf(
        Str("成功ライブカード置き場"), Str("ライブ成功カード置き場"),
        Str("ライブカード置き場"), Str("ライブ置き場"),
        Str("エネルギーデッキ"), Str("エネルギー置き場"), Str("エネルギーゾーン"),
        Str("控え室"), Str("手札"), Str("ステージ"), Str("デッキ"), Str("山札"),
        Str("ウェイト"), Str("アクティブ"),
        Str("ライブ中のカード"),
    )

    sources = OneOf(
        Str("デッキの一番下から"), Str("デッキの上から"), Str("デッキから"),
        Str("山札から"), Str("エネルギーデッキから"), Str("エネルギー置き場から"),
        Str("控え室から"), Str("手札から"), Str("ステージから"),
        Str("ライブカード置き場から"), Str("成功ライブカード置き場から"),
    )

    destinations = OneOf(
        Str("デッキの一番上から4枚目に"),
        Str("デッキの一番上に"), Str("デッキの上に"),
        Str("デッキの一番下に"), Str("デッキの下に"),
        Str("デッキに"), Str("山札に"),
        Str("手札に"), Str("ステージに"),
        Str("エネルギー置き場に"), Str("エネルギー・デッキに"),
        Str("ライブカード置き場に"), Str("成功ライブカード置き場に"),
        Str("控え室に"),
    )

    card_types = OneOf(
        Str("メンバーカード"), Str("メンバー"),
        Str("ライブカード"), Str("ライブ"),
        Str("エネルギーカード"), Str("エネルギー"),
        Str("カード"),
    )

    # --- 4. Verb Stems & Suffixes ---
    base_verbs = OneOf(
        Str("加え"), Str("加える"), Str("加えられ"),
        Str("置い"), Str("置き"), Str("置く"), Str("置か"),
        Str("登場"), Str("出し"), Str("出す"),
        Str("シャッフル"), Str("入れ替え"), Str("入れ替える"),
        Str("無効"), Str("引き"), Str("引く"),
        Str("見せ"), Str("見"), Str("見る"),
        Str("公開"), Str("得"), Str("得る"), Str("する"),
        Str("選び"), Str("選ぶ"), Str("選ば"),
        Str("ウェイト"), Str("アクティブ"), Str("指名"), Str("指定")
    )
    verb_suffixes = OneOf(
        Str("したとき"), Str("した時"), Str("した時、"),
        Str("してもよい"), Str("しても良い"), Str("してもよい。"),
        Str("して"), Str("した"), Str("し"), Str("する"), Str("さ"),
        Str("れる"), Str("られる"), Str("せる"), Str("させる"),
        Str("る"), Str("ます"), Str("。"), Str("、")
    )
    flexible_verb = Seq(base_verbs, Many(verb_suffixes))

    # --- 5. Structural ---
    particles = OneOf(
        Str("を"), Str("に"), Str("が"), Str("は"), Str("から"), Str("の"), Str("で"),
        Str("のうち"), Str("か"), Str("、"), Str("。"), Str("："),
        Str("かつ"), Str("あり、"), Str("あるか、"),
        Str("その後、"), Str("そうした場合"),
        Str("につき"), Str("たび"), Str("以外"), Str("の中に"),
        Str("または"), Str("かつ")
    )

    operators = OneOf(
        Str("以上"), Str("以下"), Str("より少ない"), Str("より多い"),
        Str("未満"), Str("超"),
        Str("高い"), Str("低い"), Str("少ない"), Str("多い"),
        Str("="), Str(">"), Str("<"), Str(">="), Str("<="),
    )

    return {
        "number": number,
        "count": count_patterns,
        "trigger_icon": trigger_icons,
        "energy_icon": energy_icon,
        "heart_icon": heart_icon,
        "blade_icon": blade_icon,
        "any_icon": any_icon,
        "group": OneOf(major_units, Regex(r"『(.+?)』")),
        "name": Regex(r"「(.+?)」"),
        "location": zones,
        "source": sources,
        "destination": destinations,
        "card_type": card_types,
        "verb": flexible_verb,
        "particle": particles,
        "operator": operators,
        "subject": OneOf(Str("このメンバー"), Str("自分"), Str("相手"), Str("自分と相手")),
        "temporal": OneOf(Str("ライブ終了時まで"), Str("ライブ終了まで"), Str("このターンの間"), Str("ターン終了時まで"), Str("このターン")),
        "cost_limit": Seq(Str("コスト"), number, operators),
        "t": t,
    }
