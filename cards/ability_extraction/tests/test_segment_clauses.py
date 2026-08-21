"""
tests/test_segment_clauses.py

Stage A IR tests: segment_clauses() must recover sentence boundaries, cost/effect
split, leading condition gates, inter-clause links, and choice bullets from raw
ability text — purely structurally, no semantics.

Run: python cards/ability_extraction/tests/test_segment_clauses.py
"""
import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))
from parser import segment_clauses


def kinds(node):
    return [c["kind"] for c in node.get("children", [])]


def test_single_action():
    r = segment_clauses("カードを1枚引く。")
    assert kinds(r) == ["sentence"]
    c = r["children"][0]
    assert c["body"] == "カードを1枚引く"
    assert "condition" not in c
    assert "cost_text" not in r


def test_cost_split():
    r = segment_clauses("{{icon_energy.png|E}}：カードを1枚引く。")
    assert r.get("cost_text") == "{{icon_energy.png|E}}"
    assert len(r["children"]) == 1


def test_condition_gate():
    r = segment_clauses("自分のステージに『Aqours』のメンバーが2人以上いる場合、カードを1枚引く。")
    c = r["children"][0]
    assert c["condition"]["marker"] == "場合"
    assert c["condition"]["text"] == "自分のステージに『Aqours』のメンバーが2人以上いる場合、"
    assert c["body"] == "カードを1枚引く"


def test_condition_marker_not_inside_brackets():
    # 場合、 inside 「」 quotes must not be treated as a gate
    r = segment_clauses("「場合、」と言う。カードを1枚引く。")
    assert len(r["children"]) == 2
    assert all("condition" not in c for c in r["children"])


def test_then_link():
    r = segment_clauses("デッキの上からカードを3枚見る。その後、それらをデッキの上に置く。")
    a, b = r["children"]
    assert "link" not in a or a.get("link") is None
    assert b["link"] == "then"
    assert b["body"].startswith("それらを")


def test_on_accept_link():
    r = segment_clauses("手札を1枚控え室に置いてもよい。そうしたとき、カードを2枚引く。")
    b = r["children"][1]
    assert b["link"] == "on_accept"
    assert b["body"] == "カードを2枚引く"


def test_furthermore_link():
    r = segment_clauses("スコアを+1する。さらに、カードを1枚引く。")
    assert r["children"][1]["link"] == "furthermore"


def test_choice_bullets():
    text = "{{icon_energy.png|E}}支払ってもよい：以下から1つを選ぶ。\n・相手のメンバー1人をウェイトにする。\n・カードを1枚引く。"
    r = segment_clauses(text)
    assert r.get("cost_text") == "{{icon_energy.png|E}}支払ってもよい"
    assert kinds(r) == ["choice"]
    ch = r["children"][0]
    assert len(ch["options"]) == 2
    assert ch["options"][0] == "相手のメンバー1人をウェイトにする"
    assert ch["options"][1] == "カードを1枚引く"


def test_sentence_with_period_inside_quotes_stays_whole():
    r = segment_clauses("「ライブの合計スコアを+1する。」を得る。")
    assert len(r["children"]) == 1
    assert r["children"][0]["body"] == "「ライブの合計スコアを+1する。」を得る"


def test_kagiri_gate():
    r = segment_clauses("このメンバーの下にメンバーカードが3枚以上置かれているかぎり、スコアを＋１する。")
    c = r["children"][0]
    assert c["condition"]["marker"] == "かぎり"
    assert "スコア" in c["body"]


def test_multi_sentence_mixed():
    text = "このターン、自分のデッキがリフレッシュしていた場合、このカードのスコアを＋２する。そうしなかった場合、何もしない。"
    r = segment_clauses(text)
    assert len(r["children"]) == 2
    first = r["children"][0]
    assert first["condition"]["marker"] == "場合"
    assert "スコア" in first["body"]


def test_never_crashes_on_odd_input():
    for t in ("", "。", "：", "{{broken", "場合、だけ"):
        r = segment_clauses(t)
        assert isinstance(r, dict) and r.get("kind") == "ability"


if __name__ == '__main__':
    import traceback
    tests = [(k, v) for k, v in sorted(globals().items()) if k.startswith('test_')]
    passed, failed = 0, 0
    for name, t in tests:
        try:
            t()
            print(f'  PASS  {name}')
            passed += 1
        except AssertionError as e:
            print(f'  FAIL  {name}')
            for line in str(e).splitlines():
                print(f'        {line}')
            failed += 1
        except Exception as e:
            print(f'  ERROR {name}: {e}')
            traceback.print_exc()
            failed += 1
    print(f'\n{passed} passed, {failed} failed')
    sys.exit(0 if failed == 0 else 1)
