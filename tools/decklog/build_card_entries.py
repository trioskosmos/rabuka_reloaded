"""Build cards.json-shaped entries for the cards missing from the database.

Ability text was transcribed from the official card images (cards/images/), which is the
authoritative source - the decklog API exposes no ability field, and the deck API's
p_param is not cost (it reads 10 where the card prints 13).

Fields that could not be read off the card with confidence are left out and listed in
`needs_manual`, rather than guessed. Nothing here mutates cards.json.
"""

import json
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
MISSING = REPO / "tools/decklog/missing_cards.json"

E = "{{icon_energy.png|E}}"
TURN1 = "{{turn1.png|ターン1回}}"
TURN3 = "{{turn2.png|ターン2回}}{{turn1.png|ターン1回}}"
HEART = "{{icon_all.png|ハート}}"

MILL = ("自分のデッキの上から、10から自分の手札の枚数を引いた数に等しい枚数のカードを見る。"
        "その中から2枚まで手札に加える。残りを控え室に置く。")
BATTLE_START_LIVE3 = ("自分の控え室にあるカードが9枚以下の場合、"
                      "自分の控え室にあるライブカードを3枚まで選び、"
                      "自分のデッキの上に好きな順番で置く。")

# card_no -> (name, type, cost, blade, ability)
MEMBERS = {
    "PL!HS-PR-038-PR": (
        "セラス 柳田 リリエンフェルト", "メンバー", 4, 3,
        "{{toujyou.png|登場}}手札以外から登場したとき、ライブ終了時まで、" + HEART + "を得る。\n"
        "{{live_success.png|ライブ成功時}}相手のステージにいるコスト4以下のメンバー1人をウェイトにする。"
        "それは次のターンのアクティブフェイズにアクティブにならない。"),
    "PL!-PR-022-PR": (
        "西木野真姫", "メンバー", 13, 3,
        "{{kidou.png|起動}}" + TURN1 +
        "このメンバーをウェイトにする：自分のステージに「星空 凛」か「小泉花陽」がいる場合、"
        "カードを2枚引き、手札を1枚控え室に置く。2人といる場合、さらにこのメンバーをアクティブにする。"),
    "PL!-PR-023-PR": (
        "絢瀬絵里", "メンバー", 11, 3,
        "{{jidou.png|自動}}" + TURN3 +
        "自分が相手のステージにいるメンバーがウェイト状態になるたび、ライブ終了時まで、" + HEART + "を得る。\n"
        "{{kidou.png|起動}}" + TURN1 +
        "メンバー1人をウェイトにし、手札を1枚控え室に置く：カードを1枚引く。"),
    "PL!HS-PR-039-PR": ("百生吟子", "メンバー", 15, 5,
                        "{{toujyou.png|登場}}" + MILL),
    "PL!SP-PR-028-PR": ("米女メイ", "メンバー", 15, 6,
                        "{{toujyou.png|登場}}" + MILL),
    "PL!S-PR-048-PR": ("黒澤ルビィ", "メンバー", 13, 6,
                       "{{live_start.png|ライブ開始時}}" + BATTLE_START_LIVE3),
    "PL!SP-PR-027-PR": ("平安名すみれ", "メンバー", 13, 5,
                        "{{live_start.png|ライブ開始時}}" + BATTLE_START_LIVE3),
    "PL!S-PR-046-PR": ("渡辺 曜", "メンバー", 5, 1,
                       "{{live_success.png|ライブ成功時}}"
                       "エールにより公開された相手のカードの中にライブカードがある場合、"
                       "カードを1枚引く。"),
    # identical on-card text to PL!HS-PR-040-PR (安養寺姫芽) - same card, different year
    "PL!S-PR-047-PR": ("桜内梨子", "メンバー", 4, 1, None),
    "PL!HS-PR-040-PR": ("安養寺姫芽", "メンバー", 4, 1, None),
}

# energy cards carry no ability text at all
ENERGY = {
    "PL!N-PR-035-PR": "上原歩夢",
    "PL!N-PR-035-PR＋": "上原歩夢",
    "PL!S-PR-049-PR": "高海千歌",
    "PL!S-PR-049-PR＋": "高海千歌",
    "PL!SP-PR-029-PR": "澁谷かのん",
    "PL!SP-PR-030-PR": "ゆいがおー",
    "PL!SP-bp7-999-SEC2": "米女メイ",
    "PL!S-bp7-999-SEC＋": "国木田花丸",
    "PL!SP-bp4-999-SEC＋": "嵐 千砂都",
}

# read but the heart symbols in the effect could not be identified with confidence
NEEDS_MANUAL = {
    "PL!S-PR-047-PR": "自動 ... バトンタッチで登場したメンバーは [heart][heart] を得る - "
                      "heart icon identity unconfirmed (same text as PL!HS-PR-040-PR)",
    "PL!HS-PR-040-PR": "自動 ... バトンタッチで登場したメンバーは [heart][heart] を得る - "
                       "heart icon identity unconfirmed",
    "PL!HS-PR-042-PR＋": "not yet transcribed from the card image",
}


def main():
    missing = {m["card_number"]: m for m in json.loads(MISSING.read_text(encoding="utf-8"))}
    entries, manual = [], []

    for card_no, info in sorted(missing.items()):
        base = {
            "card_no": card_no,
            "name": info.get("name"),
            "img": info.get("img_url") or "",
            "_img": info.get("_img") or "",
            "rare": info.get("rare"),
        }
        if card_no in ENERGY:
            base["type"] = "エネルギー"
            base["blade"] = 0
            base["ability"] = ""
            entries.append(base)
            continue
        if card_no in MEMBERS:
            name, ctype, cost, blade, ability = MEMBERS[card_no]
            base.update({"name": name, "type": ctype, "cost": cost,
                         "blade": blade, "ability": ability})
            entries.append(base)
            if card_no in NEEDS_MANUAL:
                manual.append({"card_no": card_no, "issue": NEEDS_MANUAL[card_no]})
            continue
        base["type"] = "メンバー" if (info.get("card_kind") == "M") else "エネルギー"
        base["ability"] = None
        entries.append(base)
        manual.append({"card_no": card_no,
                       "issue": NEEDS_MANUAL.get(card_no, "not transcribed yet")})

    out = REPO / "tools/decklog/cards_additions.json"
    out.write_text(json.dumps(entries, ensure_ascii=False, indent=1), encoding="utf-8")
    print(f"entries written : {len(entries)} -> {out}")

    done = sum(1 for e in entries if e.get("ability"))
    energy = sum(1 for e in entries if e.get("type") == "エネルギー")
    print(f"  energy (no ability needed): {energy}")
    print(f"  members with ability text  : {done - energy}")
    print(f"  needing manual work        : {len(manual)}")
    for m in manual:
        print(f"    {m['card_no']:22} {m['issue']}")
    print("\ncards.json was NOT modified. merge entries from cards_additions.json when ready.")


if __name__ == "__main__":
    main()