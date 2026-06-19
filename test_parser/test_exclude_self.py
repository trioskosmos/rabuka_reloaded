import sys, os

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from test_parser.fields import ExtractedFields
from test_parser.effect import parse_effect
from test_parser.condition import parse_condition

ok = True


def check(label, got, expected):
    global ok
    if got != expected:
        print(f"FAIL: {label}: got {got!r}, expected {expected!r}")
        ok = False
    else:
        print(f"  OK: {label}")


# Test 1: exclude_self with group name between ほかの and メンバー (the fix)
print("[1] ExtractedFields: ほかの『虹ヶ咲』のメンバー")
t = "自分のステージにいるほかの『虹ヶ咲』のメンバーは{{icon_blade.png|ブレード}}を得る"
f = ExtractedFields(t)
check("target", f.target, "self")
check("exclude_self", f.exclude_self, True)
check("group_names", f.group_names, ["虹ヶ咲"])
check("card_type", f.card_type, "member_card")

# Test 2: exclude_self with contiguous ほかのメンバー (regression)
print("[2] ExtractedFields: contiguous ほかのメンバー")
t = "自分のステージにほかのメンバーがいる場合"
f = ExtractedFields(t)
check("exclude_self for contiguous ほかのメンバー", f.exclude_self, True)

# Test 3: exclude_self with このメンバー以外 (baseline)
print("[3] ExtractedFields: このメンバー以外")
t = "自分のステージにこのメンバー以外のコスト11のメンバー"
f = ExtractedFields(t)
check("exclude_self for このメンバー以外", f.exclude_self, True)

# Test 4: exclude_self with kanji 他の separated by group name (baseline)
print("[4] ExtractedFields: 他の『みらくらぱーく！』のメンバー")
t = "自分のステージにいる他の『みらくらぱーく！』のメンバー1人につき"
f = ExtractedFields(t)
check("exclude_self for 他の『みらくらぱーく！』のメンバー", f.exclude_self, True)

# Test 5: effect parsing - ほかの『虹ヶ咲』のメンバー → exclude_self
print("[5] parse_effect: ほかの『虹ヶ咲』のメンバー → exclude_self")
t = "ライブ終了時まで、自分のステージにいるほかの『虹ヶ咲』のメンバーは{{icon_blade.png|ブレード}}を得る"
r = parse_effect(t)
check("effect exclude_self", r.get("exclude_self"), True)
check("effect action", r.get("action"), "gain_resource")
check("effect resource", r.get("resource"), "blade")
check("effect count", r.get("count"), 1)
check("effect group_names", r.get("group_names"), ["虹ヶ咲"])
check("effect target", r.get("target"), "self")
check("effect duration", r.get("duration"), "live_end")

# Test 6: condition parsing - このメンバーか、ほかのメンバーが (contiguous with comma)
print("[6] parse_condition: このメンバーか、ほかのメンバーが")
t = "自分のステージに、このメンバーか、ほかのメンバーがバトンタッチして登場したとき"
r = parse_condition(t)
# This is an OR condition, exclude_self may or may not apply depending on parser logic
# Just verify the parser handles it without error
check("condition parsed without error", isinstance(r, dict), True)

# Test 7: condition parsing - ほかの『スリーズブーケ』のメンバーが登場
print("[7] parse_condition: ほかの『スリーズブーケ』のメンバーが登場")
t = "自分のステージにほかの『スリーズブーケ』のメンバーが登場するたび"
r = parse_condition(t)
check(
    "condition exclude_self for ほかの『スリーズブーケ』のメンバー",
    r.get("exclude_self"),
    True,
)

# Test 8: condition parsing - このメンバー以外のコスト11のメンバー
print("[8] parse_condition: このメンバー以外")
t = "自分のステージにこのメンバー以外のコスト11のメンバーが登場したとき"
r = parse_condition(t)
check("condition exclude_self for このメンバー以外", r.get("exclude_self"), True)
check("condition card_type", r.get("card_type"), "member_card")

print()
if ok:
    print("ALL TESTS PASSED")
else:
    print("SOME TESTS FAILED")
    sys.exit(1)
