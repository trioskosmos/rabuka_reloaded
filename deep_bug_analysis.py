# -*- coding: utf-8 -*-
import json, re

def load_json(path):
    with open(path, 'r', encoding='utf-8') as f:
        return json.load(f)

cards = load_json('cards/cards.json')
ad = load_json('cards/abilities.json')
uabs = ad['unique_abilities']
qa = load_json('cards/qa_data.json')
qa_by_id = {e['id']: e for e in qa}

# Build mapping
card_ab_to_uab = {}
for uab_idx, entry in enumerate(uabs):
    for card_ref in entry.get('cards', []):
        m = re.match(r'^(.+?)\s*\|.*?\(ab#(\d+)\)', card_ref)
        if m:
            cno = m.group(1).strip()
            ab_slot = int(m.group(2))
            card_ab_to_uab.setdefault(cno, {})[ab_slot] = uab_idx

def get_uab(cid, slot):
    m = card_ab_to_uab.get(cid, {})
    idx = m.get(slot)
    if idx is not None:
        return uabs[idx]
    return None

lines = []
def L(s=""):
    lines.append(s)

L("=" * 100)
L("COMPREHENSIVE BUG/ISSUE REPORT: RAW ABILITY vs PARSED JSON")
L("=" * 100)
L("")

# ===================== Card 1: PL!-pb1-001-R =====================
cid = "PL!-pb1-001-R"
cname = "高坂穂乃果"
cdata = cards[cid]
L("=" * 100)
L(f"CARD 1: {cid} ({cname})")
L("=" * 100)
L("")
L("--- RAW ABILITY TEXT ---")
L(cdata['ability'])
L("")
L("--- Q CODES ---")
for fe in cdata.get('faq',[]):
    L(f"  {fe['title']}")
    entry = qa_by_id.get(re.match(r'(Q\d+)', fe['title']).group(1))
    if entry:
        L(f"    Q: {entry['question'][:150]}...")
        L(f"    A: {entry['answer'][:150]}...")
L("")

entry = get_uab(cid, 0)
L("--- PARSED (uab#261) ---")
L(f"  Type: Activated (起動), Use limit: 1")
L(f"  Cost: sequential_cost [change_state(wait), move_cards(hand->discard x1)]")
L(f"  Effect: sequential")
effect = entry['effect']
for i, act in enumerate(effect.get('actions', [])):
    L(f"    [{i}] {act.get('action')}: {act.get('text','')[:100]}")
L("")
L(">> ISSUE 1: SELECT action in effect[0] specifies card_type='member_card' and cost_limit=10")
L("   but raw text says 'ライブカードかコスト10以上のメンバーカードのどちらか1つ'")
L("   (either a LIVE card OR a member card with cost 10+).")
L("   The 'live_card' option is NOT represented in the parsed output.")
L("   This means the engine would only search for member_card with cost_limit=10,")
L("   missing the option to pick a live_card of any cost.")
L("")
L(">> ISSUE 2: The effect captures 'select->reveal->move' but the reveal action")
L("   '選んだカードが公開されるまで' (until selected card is revealed) is parsed")
L("   as a nested 'sequential' with inner 'select' which seems semantically odd.")
L("   The 'reveal until found' pattern should be a 'reveal_until' type, not nested sequential.")
L("")

# ===================== Card 2: PL!HS-bp1-022-L =====================
cid2 = "PL!HS-bp1-022-L"
cname2 = "AWOKE"
cdata2 = cards[cid2]
L("=" * 100)
L(f"CARD 2: {cid2} ({cname2})")
L("=" * 100)
L("")
L("--- RAW ABILITY TEXT ---")
L(cdata2['ability'])
L("")
entry2 = get_uab(cid2, 0)
L("--- PARSED (uab#380) ---")
L(f"  Trigger: ライブ成功時")
L(f"  Condition: card_count_condition(location=revealed_cards, count=10, operator=>=)")
L(f"  Action: modify_score(+1, self)")
L(f"  Parenthetical: captured")
L("")
L(">> ISSUE: Q107 is assigned to this card but Q107's QUESTION references TWO abilities:")
L("   1) 自動 ability (from another card) that re-yells")
L("   2) THIS card's live_success ability")
L("   The Q107 QA clarifies: '2つ目の能力を使用する時点で公開されている、2回目のエールにより公開された自分のカードのみ参照します'")
L("   This means the condition must check cards revealed by the CURRENT yell (second yell),")
L("   not ALL yell-revealed cards this turn. The parsed condition uses 'revealed_cards' which")
L("   is ambiguous - does it scope to the current yell's revealed cards? If not, it's wrong.")
L("")
L(">> ISSUE: No cost is expected (triggered ability), but 'Missing cost field entirely'")
L("   warns incorrectly. This is actually correct for triggered abilities.")
L("")

# ===================== Card 3: PL!N-bp5-030-L =====================
cid3 = "PL!N-bp5-030-L"
cname3 = "繚乱！ビクトリーロード"
cdata3 = cards[cid3]
L("=" * 100)
L(f"CARD 3: {cid3} ({cname3})")
L("=" * 100)
L("")
L("--- RAW ABILITY TEXT ---")
L(cdata3['ability'])
L("")
entry3a = get_uab(cid3, 0)
entry3b = get_uab(cid3, 1)
L("--- PARSED ab#0 (uab#542) ---")
L(f"  Trigger: 自動, trigger_type: each_time")
L(f"  Condition: location_condition(stage, member, negation=true, heart_type=all)")
L(f"  Action: gain_ability(count=1, duration=live_end)")
L("")
L(">> ISSUE 1: The raw text says '自分のステージにいるメンバーのライブ開始時能力が解決するたび'")
L("   (each time a member's live_start ability resolves)")
L("   This is a 'watching' trigger - it triggers when OTHER cards' abilities resolve.")
L("   The parsed entry has trigger_type=each_time but does NOT capture the sub-trigger")
L("   condition that it only fires when 'live_start' abilities resolve. This means the")
L("   engine might trigger on ANY auto ability resolving, not just live_start abilities.")
L("")
L(">> ISSUE 2: QA Q217 and Q227 clarify that this ability ONLY triggers if cost WAS PAID")
L("   for the triggering live_start ability. This 'cost_paid' dependency is NOT captured")
L("   anywhere in the parsed condition/effect.")
L("   Q227: 'コストの支払いが必要な能力に対してコストを支払いませんでした。このとき、このカードの能力は発動しますか？ → いいえ'")
L("")

entry3b = get_uab(cid3, 1)
L("--- PARSED ab#1 (uab#543) ---")
L(f"  Action: sequential [do_nothing, draw_card]")
L("")
L(">> CRITICAL BUG: Effect[0] is 'do_nothing' with empty text!")
L("   This is a parser artifact. The raw text says simply 'カードを1枚引く' (draw 1 card).")
L("   There is NO reason for a do_nothing action before draw_card.")
L("   The parsed output should just be {action: draw_card, count: 1, ...}")
L("   This 'do_nothing' wasteland will cause the engine to waste an action step doing nothing.")
L("")

# ===================== Card 4: PL!S-pb1-002-R =====================
cid4 = "PL!S-pb1-002-R"
cname4 = "桜内梨子"
cdata4 = cards[cid4]
L("=" * 100)
L(f"CARD 4: {cid4} ({cname4})")
L("=" * 100)
L("")
L("--- RAW ABILITY TEXT ---")
L(cdata4['ability'])
L("")
entry4 = get_uab(cid4, 0)
L("--- PARSED (uab#230) ---")
L(f"  Effect: sequential")
for i, act in enumerate(entry4['effect'].get('actions', [])):
    L(f"    [{i}] {act.get('action')}: {act.get('text','')[:120]}")
L("")
L(">> ISSUE 1: The conditional logic 'そうしなかった場合' (if opponent did NOT discard)")
L("   is NOT captured as a conditional branch in the sequential effect.")
L("   The three actions run sequentially regardless. The engine needs to know that")
L("   actions[1] and [2] should ONLY execute if the opponent chose NOT to discard.")
L("   Currently parsed as unconditional sequential, which is WRONG.")
L("")
L(">> ISSUE 2: Actions[1] and [2] split 'ライブの合計スコアを+1するを得る' into two:")
L("   [1] modify_score (score+1) and [2] gain_ability")
L("   These should be a SINGLE action: 'gain an ability that does modify_score'.")
L("   As parsed, the engine would add +1 to score immediately AND gain a separate")
L("   ability that does... nothing (since the score mod was already applied).")
L("")

# ===================== Card 5: PL!SP-bp4-023-L =====================
cid5 = "PL!SP-bp4-023-L"
cname5 = "Dazzling Game"
cdata5 = cards[cid5]
L("=" * 100)
L(f"CARD 5: {cid5} ({cname5})")
L("=" * 100)
L("")
L("--- RAW ABILITY TEXT ---")
for line in cdata5['ability'].split('\n'):
    if line.strip():
        L(f"  {line.strip()}")
L("")
entry5a = get_uab(cid5, 0)
L("--- PARSED ab#0 (uab#501) ---")
for i, act in enumerate(entry5a['effect'].get('actions', [])):
    L(f"  action[{i}]: {act.get('action')} | {act.get('text','')[:120]}")
L("")
L(">> CRITICAL BUG: FIRST SELECTION IS MISSING!")
L("   Raw text: '「澁谷かのん」「ウィーン・マルガレーテ」「鬼塚冬毬」のうちのメンバー1人と、")
L("            これにより選んだメンバー以外の『Liella!』のメンバー1人は、ブレードを得る。'")
L("   This requires TWO selections:")
L("     1) Select 1 member FROM {Kanon, Margarete, Fuyuka}")
L("     2) Select 1 Liella! member OTHER than the one chosen in step 1")
L("   Parsed output only has ONE select action (step 2) and skips step 1 entirely!")
L("   The 'group': {'name': 'Liella!'} doesn't reflect the 3 named characters.")
L("   QA Q187 confirms: '選んだメンバー以外のメンバーを選ぶ必要がありますか？ はい'")
L("   (Is it necessary to select a member other than the chosen one? YES)")
L("   This means the engine would select just 1 Liella! member and give blade,")
L("   missing the first named-character target entirely.")
L("")

# ===================== Card 6: PL!S-bp3-019-L =====================
cid6 = "PL!S-bp3-019-L"
cname6 = "MIRACLE WAVE"
cdata6 = cards[cid6]
L("=" * 100)
L(f"CARD 6: {cid6} ({cname6})")
L("=" * 100)
L("")
L("--- RAW ABILITY TEXT ---")
L(cdata6['ability'])
L("")
entry6 = get_uab(cid6, 0)
L("--- PARSED (uab#447) ---")
L(f"  Condition: {json.dumps(entry6['effect'].get('condition'), ensure_ascii=False)}")
L(f"  Action: set_score(value=4, self_target=true)")
L("")
L(">> CRITICAL BUG: ONLY ONE OF TWO CONDITIONS IS PARSED!")
L("   Raw text says: 'エールにより公開された自分のカードの中にブレードハートを持たないカードが0枚の場合")
L("                  か、または自分が余剰ハートを2つ以上持っている場合'")
L("   Condition A: 0 cards WITHOUT blade heart among yell-revealed cards")
L("   Condition B: 2+ surplus hearts")
L("   These are OR conditions (どちらか = either one qualifies).")
L("   The parsed output ONLY shows condition B (surplus_heart >= 2)!")
L("   Condition A about blade-heart-less cards is COMPLETELY MISSING.")
L("")
L("   QA Q182 confirms Condition A works independently:")
L("   'ウェイト状態などによってエールで公開したカードが0枚の場合、このライブカードのスコアはいくつになりますか？")
L("    → エールにより公開された自分のカードの中にブレードハートを持たないカードが0枚の場合 を満たすため、4'")
L("   This is a MAJOR bug - the parser dropped the first condition entirely.")
L("")

# ===================== Card 7: PL!SP-bp5-005-R+ =====================
cid7 = "PL!SP-bp5-005-R＋"  # actual key with ＋
cname7 = "葉月 恋"
cdata7 = cards.get(cid7, cards.get('PL!SP-bp5-005-R+', {}))
L("=" * 100)
L(f"CARD 7: {cid7} ({cname7})")
L("=" * 100)
L("")
L("--- RAW ABILITY TEXT ---")
L(cdata7.get('ability', 'NOT FOUND'))
L("")
entry7a = get_uab(cid7, 0)
L("--- PARSED ab#0 (uab#106) - Activated ability ---")
L(f"  Cost: move_cards(deck_top->discard, 3)")
L(f"  Effect: gain_resource(blade, per_unit=true, group=Liella!, duration=live_end)")
L("")
L(">> Looks correct. The 'per_unit' mechanism captures '1枚につき' (per card).")
L("")

entry7b = get_uab(cid7, 1)
L("--- PARSED ab#1 (uab#107) - Auto ability ---")
L(f"  Effect actions:")
for i, act in enumerate(entry7b['effect'].get('actions', [])):
    L(f"    [{i}] {act.get('action')}: {act.get('text','')[:120]}")
L(f"  Conditional: {entry7b['effect'].get('conditional')}")
L("")
L(">> ISSUE 1: The optional E payment (actions[0]) has action='custom' which is vague.")
L("   The raw text says 'Eを支払ってもよい' (may pay E). This should be an optional cost")
L("   payment, not a 'custom' action.")
L("")
L(">> ISSUE 2: The card recovery (actions[1]) uses source='revealed_cards' and")
L("   dynamic_count with reference 'previous_reveal'. But the actual source is 'discard' -")
L("   the cards that were just placed in the discard triggered this ability.")
L("   QA Q221 confirms: 'それらのカードの中 = 能力の誘発条件として控え室に置いたカードの中から選ぶ'")
L("   (pick from among the cards placed in discard that triggered this ability)")
L("   Using 'revealed_cards' as the source is INCORRECT - it should be 'discard' with")
L("   a reference to the cards that triggered this ability instance.")
L("")

# ===================== Card 8: PL!-pb1-009-R =====================
cid8 = "PL!-pb1-009-R"
cname8 = "矢澤にこ"
cdata8 = cards[cid8]
L("=" * 100)
L(f"CARD 8: {cid8} ({cname8})")
L("=" * 100)
L("")
L("--- RAW ABILITY TEXT ---")
for line in cdata8['ability'].split('\n'):
    if line.strip():
        L(f"  {line.strip()}")
L("")
entry8a = get_uab(cid8, 0)
L("--- PARSED ab#0 (uab#270) - Wait ability ---")
L(f"  Action: change_state(wait)")
L(f"  Target: opponent")
L(f"  Card type: member_card")
L(f"  Count: 1")
L("")
L(">> ISSUE: Condition '元々持つブレードの数が1つ以下' (originally has <=1 blade)")
L("   is NOT captured as a target filter condition.")
L("   The target selection should only include opponent members whose base blade count")
L("   is 1 or less. Without this condition, the engine could target ANY opponent member.")
L("")

entry8b = get_uab(cid8, 1)
L("--- PARSED ab#1 (uab#271) - Restriction ability ---")
L(f"  Action: restriction(cannot_activate_by_effect)")
L(f"  Target: both")
L(f"  Duration: this_turn")
L("")
L(">> Looks correct. The restriction type captures the semantics well.")
L("")

# Final summary
L("")
L("=" * 100)
L("SUMMARY OF CRITICAL PARSER BUGS FOUND")
L("=" * 100)
L("")
L("CRITICAL (affects gameplay):")
L("  1. Dazzling Game ab#0: FIRST MEMBER SELECTION ENTIRELY MISSING")
L("     The parser skipped selecting from {かのん, マルガレーテ, 冬毬} entirely.")
L("  2. MIRACLE WAVE: FIRST OR-CONDITION DROPPED")
L("     The '0 cards without blade heart' condition is not parsed.")
L("  3. 繚乱！ビクトリーロード ab#1: SPURIOUS do_nothing ACTION")
L("     Empty 'do_nothing' action before draw_card wastes an action step.")
L("  4. 桜内梨子: CONDITIONAL BRANCH NOT CAPTURED")
L("     'そうしなかった場合' (if [opponent] did not discard) logic is missing.")
L("  5. 高坂穂乃果: SELECT ACTION MISSES live_card OPTION")
L("     Only member_card (cost_limit=10) is parsed, live_card option dropped.")
L("")
L("MODERATE (affects correctness):")
L("  6. 葉月恋 ab#1: WRONG SOURCE in move_cards")
L("     Should be 'discard' not 'revealed_cards' for the card recovery.")
L("  7. 桜内梨子: SPLIT modify_score + gain_ability")
L("     Should be one 'gain ability' action, not separate score mod + ability gain.")
L("  8. 矢澤にこ ab#0: MISSING blade count filter")
L("     '元々持つブレードが1つ以下' condition not in target filter.")
L("  9. 繚乱！ビクトリーロード ab#0: MISSING sub-trigger scope")
L("     Should only fire when 'live_start' abilities resolve, not all auto abilities.")
L("  10. AWOKE: Condition scope ambiguity - current yell vs all revealed cards.")
L("")
L("LOW (edge cases / parser quality):")
L("  11. 高坂穂乃果: 'reveal until found' pattern parsed as nested sequential+select")
L("     instead of a dedicated 'reveal_until' action type.")
L("  12. 葉月恋 ab#1: Optional E payment as 'custom' instead of proper optional cost.")
L("")

with open('bugs_report.txt', 'w', encoding='utf-8') as f:
    f.write('\n'.join(lines))

print("Bug report written to bugs_report.txt")
