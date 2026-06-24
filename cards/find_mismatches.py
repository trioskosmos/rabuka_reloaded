import json, re
from collections import defaultdict

with open("cards/abilities.json", "r", encoding="utf-8") as f:
    data = json.load(f)
abilities = data["unique_abilities"]


def get_actions(obj):
    acts = set()
    if isinstance(obj, dict):
        if obj.get("action"):
            acts.add(obj["action"])
        for v in obj.values():
            if isinstance(v, (dict, list)):
                acts.update(get_actions(v))
    elif isinstance(obj, list):
        for item in obj:
            acts.update(get_actions(item))
    return acts


def json_contains(obj, pred):
    if isinstance(obj, dict):
        if pred(obj):
            return True
        for v in obj.values():
            if isinstance(v, (dict, list)):
                if json_contains(v, pred):
                    return True
    elif isinstance(obj, list):
        for item in obj:
            if json_contains(item, pred):
                return True
    return False


SUSPECT_PATTERNS = [
    (
        r"なかった場合|なければ|ない場合|なけれ|なかったとき",
        lambda j: json_contains(
            j,
            lambda d: d.get("action")
            in (
                "conditional_alternative",
                "conditional_on_result",
                "conditional_on_optional",
            ),
        ),
    ),
    (
        r"好きな順番|任意の順番",
        lambda j: json_contains(j, lambda d: "placement_order" in d),
    ),
    (
        r"まで公開|現れるまで|出るまで",
        lambda j: json_contains(
            j, lambda d: d.get("action", "").startswith("reveal_until")
        ),
    ),
    (
        r"繰り返す|まで繰り返|もう一度.*行う|さらに.*回",
        lambda j: json_contains(
            j,
            lambda d: d.get("action") == "repeat_procedure"
            or d.get("repeat_limit") is not None,
        ),
    ),
    (
        r"相手.*選ぶ|相手.*選ん",
        lambda j: json_contains(j, lambda d: d.get("action_by") == "opponent"),
    ),
    (
        r"異なるカード名|カード名の異なる|名前の異なる|名前が異なる",
        lambda j: json_contains(j, lambda d: d.get("distinct") == "card_name"),
    ),
    (
        r"このカード以外|このメンバー以外|自身以外",
        lambda j: json_contains(j, lambda d: d.get("exclude_self") == True),
    ),
    (
        r"お互い|両プレイヤー|自分と相手|相手と自分",
        lambda j: json_contains(
            j, lambda d: d.get("target") == "both" or d.get("self_target") == True
        ),
    ),
    (
        r"次の.*から.*1つ.*選ぶ|以下から.*選ぶ|どちらか.*選ぶ",
        lambda j: json_contains(j, lambda d: d.get("action") == "choice"),
    ),
]

results = []
for idx, entry in enumerate(abilities):
    text = entry.get("triggerless_text") or entry.get("full_text") or ""
    effect = entry.get("effect")
    if not effect or not isinstance(effect, dict):
        continue
    if not text:
        continue

    for pattern, check in SUSPECT_PATTERNS:
        m = re.search(pattern, text)
        if not m:
            continue
        if check(effect):
            continue
        snippet = text[max(0, m.start() - 20) : m.end() + 40]
        results.append(
            {
                "pattern": pattern,
                "cards": entry.get("cards", []),
                "trigger": entry.get("triggers", ""),
                "snippet": snippet,
                "actions": sorted(get_actions(effect)),
                "json": json.dumps(effect, ensure_ascii=False)[:300],
            }
        )

by_pat = defaultdict(list)
for r in results:
    by_pat[r["pattern"]].append(r)

print(f"Total abilities: {len(abilities)}")
print(f"Mismatches flagged: {len(results)}")
print()

for pat, entries in sorted(by_pat.items()):
    r0 = entries[0]
    print("=" * 70)
    print(f"PATTERN: {pat}  ({len(entries)} abilities)")
    print("=" * 70)
    for r in entries[:10]:
        card = r["cards"][0] if r["cards"] else "(none)"
        acts = ", ".join(r["actions"][:6])
        print(f"\n  CARD: {card}")
        print(f"  TRIGGER: {r['trigger']}")
        print(f"  TEXT: ...{r['snippet']}...")
        print(f"  ACTIONS: [{acts}]")
    if len(entries) > 10:
        print(f"\n  ... and {len(entries) - 10} more")
    print()

### Second pass: action-level suspicion ###
print("=" * 70)
print("ACTION-SUSPICIOUS ABILITIES")
print("(text says one mechanic but actions suggest something else)")
print("=" * 70)
print()

for idx, entry in enumerate(abilities):
    text = entry.get("triggerless_text") or entry.get("full_text") or ""
    effect = entry.get("effect")
    if not effect or not isinstance(effect, dict):
        continue

    acts = get_actions(effect)
    flags = []
    jstr = json.dumps(effect, ensure_ascii=False)

    if re.search(r"ブレード.*得|blade", text) and "gain_resource" not in acts:
        if not json_contains(effect, lambda d: "blade" in str(d.get("resource", ""))):
            flags.append("'blade gain' text but no gain_resource with blade")

    has_heart_color = json_contains(
        effect,
        lambda d: d.get("heart_colors")
        or d.get("heart_type")
        or d.get("action") in ("specify_heart_color", "set_heart_type"),
    )
    if (
        re.search(
            r"ハートの.*(色|種類)|heart.*(color|type)|好きな.*ハート|任意の.*ハート",
            text,
        )
        and not has_heart_color
    ):
        flags.append(
            "heart color/type mentioned but no heart_color/set_heart_type/specify_heart_color in JSON"
        )

    has_temporal_flag = [False]

    def check_tf(obj):
        if isinstance(obj, dict):
            if obj.get("temporal") == "this_turn":
                has_temporal_flag[0] = True
            for v in obj.values():
                if isinstance(v, (dict, list)):
                    check_tf(v)
        elif isinstance(obj, list):
            for i in obj:
                check_tf(i)

    if re.search(r"この[ターン]", text):
        check_tf(effect)
        if not has_temporal_flag[0]:
            flags.append("'this turn' in text but no temporal='this_turn' in JSON")

    has_next_turn = [False]

    def check_nt(obj):
        if isinstance(obj, dict):
            if obj.get("temporal") == "next_turn":
                has_next_turn[0] = True
            for v in obj.values():
                if isinstance(v, (dict, list)):
                    check_nt(v)
        elif isinstance(obj, list):
            for i in obj:
                check_nt(i)

    if re.search(r"次の[ターン]", text):
        check_nt(effect)
        if not has_next_turn[0]:
            flags.append("'next turn' in text but no temporal='next_turn' in JSON")

    has_count_condition = json_contains(
        effect,
        lambda d: isinstance(d.get("count"), int)
        and d.get("type") == "card_count_condition",
    )
    if re.search(r"(\d+)枚以上|\d+枚以下", text):
        m2 = re.search(r"(\d+)枚以上|\d+枚以下", text)
        num = m2.group(1) if m2 else "?"
        # Check if this specific count appears in JSON
        if num not in jstr:
            flags.append(
                f"count '{num}' in text but may not be reflected in JSON conditions"
            )

    if flags:
        cards = entry.get("cards", [])
        print(f"CARD: {cards[0] if cards else '(none)'}")
        print(f"TRIGGER: {entry.get('triggers', '')}")
        print(f"ACTIONS: {sorted(acts)}")
        for f in flags:
            print(f"  !! {f}")
        snippet = text[:200]
        print(f"  TEXT: {snippet}")
        print()
