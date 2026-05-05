# -*- coding: utf-8 -*-
import json, re, sys
from pprint import pformat

def load_json(path):
    with open(path, 'r', encoding='utf-8') as f:
        return json.load(f)

def main():
    cards = load_json('cards/cards.json')
    abilities_data = load_json('cards/abilities.json')
    unique_abilities = abilities_data.get('unique_abilities', [])
    qa_data = load_json('cards/qa_data.json')  # list of {id, date, question, answer, related_cards}
    
    # Build qa lookup
    qa_by_id = {}
    for entry in qa_data:
        qa_by_id[entry['id']] = entry

    # Build mapping: card_no -> {ab_slot -> uab_idx}
    card_ab_to_uab = {}
    for uab_idx, entry in enumerate(unique_abilities):
        for card_ref in entry.get('cards', []):
            m = re.match(r'^(.+?)\s*\|.*?\(ab#(\d+)\)', card_ref)
            if m:
                cno = m.group(1).strip()
                ab_slot = int(m.group(2))
                card_ab_to_uab.setdefault(cno, {})[ab_slot] = uab_idx

    # Also build: uab_idx -> list of (card_no, ab_slot) for reverse lookup
    uab_to_card_slots = {}
    for uab_idx, entry in enumerate(unique_abilities):
        for card_ref in entry.get('cards', []):
            m = re.match(r'^(.+?)\s*\|.*?\(ab#(\d+)\)', card_ref)
            if m:
                cno = m.group(1).strip()
                ab_slot = int(m.group(2))
                uab_to_card_slots.setdefault(uab_idx, []).append((cno, ab_slot))

    # Target cards with their Q codes
    targets = [
        ('PL!-pb1-001-R', '高坂穂乃果', ['Q166', 'Q167']),
        ('PL!HS-bp1-022-L', 'AWOKE', ['Q107', 'Q36']),
        ('PL!N-bp5-030-L', '繚乱！ビクトリーロード', ['Q217', 'Q227']),
        ('PL!S-pb1-002-R', '桜内梨子', ['Q130', 'Q171']),
        ('PL!SP-bp4-023-L', 'Dazzling Game', ['Q187', 'Q192']),
        ('PL!S-bp3-019-L', 'MIRACLE WAVE', ['Q182', 'Q36']),
        ('PL!SP-bp5-005-R+', '葉月恋', ['Q221', 'Q233']),
        ('PL!-pb1-009-R', '矢澤にこ', ['Q180']),
    ]

    output = []
    def out(s=""):
        output.append(s)

    out("=" * 90)
    out("CARD ABILITY ANALYSIS: RAW TEXT vs PARSED JSON vs QA DATA")
    out("Generated: comprehensive cross-reference")
    out("=" * 90)
    out("")

    # First, handle the card that might have a different key
    # Check for PL!SP-bp5-005-R+
    actual_cid_005 = None
    for cid in cards:
        if 'SP-bp5-005' in cid:
            actual_cid_005 = cid
            break

    for cid, cname, qcodes in targets:
        # Resolve actual card ID
        actual_cid = cid
        if cid == 'PL!SP-bp5-005-R+':
            if actual_cid_005:
                actual_cid = actual_cid_005
            else:
                # Try alternate
                for cid2 in cards:
                    if 'SP-bp5-005' in cid2:
                        actual_cid = cid2
                        break

        cdata = cards.get(actual_cid, {})
        raw_ability = cdata.get('ability', 'NO ABILITY TEXT FOUND')
        card_name = cdata.get('name', '???')

        out("▓" * 70)
        out(f"CARD: {actual_cid} ({card_name})")
        out(f"  Requested ID: {cid}")
        out(f"  Name (raw): {repr(card_name)}")
        out("▓" * 70)
        out("")

        # Raw ability text
        out("--- RAW ABILITY TEXT (from cards.json) ---")
        out(raw_ability)
        out("")

        # Parse ability text into lines/paragraphs
        raw_lines = [l for l in raw_ability.split('\n') if l.strip()]
        out(f"  ({len(raw_lines)} ability paragraph(s))")
        out("")

        # Q codes from card's faq field
        faq_entries = cdata.get('faq', [])
        out(f"--- FAQ / Q-codes on this card ---")
        for fe in faq_entries:
            title = fe.get('title', '')
            qm = re.match(r'(Q\d+)', title)
            if qm:
                qc = qm.group(1)
                out(f"  {title}")
                # Find QA data
                qa_entry = qa_by_id.get(qc)
                if qa_entry:
                    qq = qa_entry.get('question', '')
                    qa = qa_entry.get('answer', '')
                    out(f"    Q: {qq[:200]}...")
                    out(f"    A: {qa[:200]}...")
        out("")

        # Show which abilities from abilities.json correspond
        out("--- MAPPING: ability slots -> unique_abilities ---")
        available = card_ab_to_uab.get(actual_cid, {})
        out(f"  Available slots: {available}")
        
        # For each expected Q code, try to map to ability
        for qc in qcodes:
            qa_entry = qa_by_id.get(qc)
            if qa_entry:
                out(f"")
                out(f"  QA #{qc}:")
                out(f"    Question: {qa_entry.get('question','')[:300]}")
                out(f"    Answer: {qa_entry.get('answer','')[:300]}")
            else:
                out(f"  QA #{qc}: NOT FOUND in qa_data.json")
        out("")

        # For each ability slot
        max_slot = max(list(available.keys()) + [0])
        for ab_slot in range(max_slot + 1):
            uab_idx = available.get(ab_slot)
            out("")
            out("-" * 70)
            out(f"--- PARSED: Ability slot ab#{ab_slot} (uab#{uab_idx}) ---")

            if uab_idx is not None and uab_idx < len(unique_abilities):
                entry = unique_abilities[uab_idx]
                out(f"  Card count using this ability: {entry.get('card_count')}")
                out(f"  Full text: {entry.get('full_text', 'N/A')}")
                out(f"  Triggerless text: {entry.get('triggerless_text', 'N/A')}")
                out(f"  Triggers: {entry.get('triggers', 'N/A')}")
                out(f"  Use limit: {entry.get('use_limit', 'N/A')}")
                out(f"  Is null: {entry.get('is_null', 'N/A')}")
                if entry.get('activation_condition'):
                    out(f"  Activation condition: {entry['activation_condition']}")
                out("")

                cost = entry.get('cost')
                out("  COST:")
                if cost is None:
                    out("    (None)")
                elif isinstance(cost, dict):
                    for k, v in cost.items():
                        val = json.dumps(v, ensure_ascii=False)
                        if len(val) > 200:
                            val = val[:200] + "..."
                        out(f"    {k}: {val}")
                elif isinstance(cost, list):
                    for i, c in enumerate(cost):
                        out(f"    [{i}]: {json.dumps(c, ensure_ascii=False)[:200]}")
                else:
                    out(f"    {cost}")

                out("")
                effect = entry.get('effect')
                out("  EFFECT:")
                if effect is None:
                    out("    (None)")
                elif isinstance(effect, dict):
                    for k, v in effect.items():
                        val = json.dumps(v, ensure_ascii=False)
                        if len(val) > 300:
                            val = val[:300] + "..."
                        out(f"    {k}: {val}")
                elif isinstance(effect, list):
                    for i, e in enumerate(effect):
                        out(f"    [{i}]:")
                        if isinstance(e, dict):
                            for k, v in e.items():
                                val = json.dumps(v, ensure_ascii=False)
                                if len(val) > 200:
                                    val = val[:200] + "..."
                                out(f"      {k}: {val}")
                        else:
                            out(f"      {e}")
                else:
                    out(f"    {effect}")

                # Analysis
                out("")
                out("  === ISSUES DETECTED ===")
                issues = analyze_ability(cid, cname, raw_ability, entry, qcodes, qa_by_id)
                if issues:
                    for issue in issues:
                        out(f"    ! {issue}")
                else:
                    out("    No issues detected.")
            else:
                out(f"  ** NOT FOUND in unique_abilities **")
                out(f"  Available: {available}")

        out("")
        out("")

    # Summary table
    out("=" * 90)
    out("SUMMARY OF ALL FINDINGS")
    out("=" * 90)
    out("")

    return '\n'.join(output)


def analyze_ability(cid, cname, raw_text, parsed_entry, expected_qcodes, qa_by_id):
    issues = []
    
    cost = parsed_entry.get('cost')
    effect = parsed_entry.get('effect')
    triggers = parsed_entry.get('triggers', '')
    is_null = parsed_entry.get('is_null', False)
    use_limit = parsed_entry.get('use_limit')

    if is_null:
        issues.append("PARSER FAILED: is_null=True, meaning parser could not parse this ability")

    # Check cost
    if cost is None:
        # Determine if this ability should have a cost
        # Triggered abilities (自動, ライブ成功時, ライブ開始時) typically have no cost
        # Activated abilities (起動) should have a cost
        if triggers in ('起動',):
            issues.append(f"MISSING COST: Trigger '{triggers}' is an activated ability type and should have a cost entry")

    # Check effect
    if effect is None:
        issues.append("MISSING EFFECT: No effect entry at all")
    elif isinstance(effect, dict):
        if not effect.get('action') and not effect.get('text'):
            issues.append("EFFECT INCOMPLETE: No 'action' or 'text' field in effect dict")
        # Check for sequential actions
        if effect.get('action') == 'sequential':
            actions = effect.get('actions', [])
            if not actions:
                issues.append("SEQUENTIAL EFFECT EMPTY: action='sequential' but no actions list")
            for i, act in enumerate(actions):
                if isinstance(act, dict) and not act.get('action') and not act.get('text'):
                    issues.append(f"SEQUENTIAL ACTION[{i}] MISSING: no action or text")
    elif isinstance(effect, list):
        if len(effect) == 0:
            issues.append("EFFECT IS EMPTY LIST")
        for i, e in enumerate(effect):
            if isinstance(e, dict):
                if not e.get('action') and not e.get('text'):
                    issues.append(f"EFFECT[{i}] MISSING: no action or text")

    # Check triggers
    if triggers == '' or triggers is None:
        issues.append("MISSING TRIGGER: No trigger keyword extracted")

    # Check raw vs parsed text alignment
    full_text = parsed_entry.get('full_text', '')
    if full_text and raw_text:
        # The full_text should approximately match some portion of the raw ability
        # Check for trigger icon presence
        pass

    # Cross-reference with QA data
    for qc in expected_qcodes:
        qa_entry = qa_by_id.get(qc)
        if qa_entry:
            qq = qa_entry.get('question', '')
            qa = qa_entry.get('answer', '')
            # Check if the QA question mentions specific behavior that the parsed effect might not capture
            if 'できない' in qa or 'できません' in qa:
                issues.append(f"QA {qc} says something is NOT ALLOWED ('できない') - verify parser handles this restriction")

    return issues


if __name__ == '__main__':
    output = main()
    with open('card_analysis_detailed.txt', 'w', encoding='utf-8') as f:
        f.write(output)
    print("Detailed analysis written to card_analysis_detailed.txt")
    print(f"Total lines: {len(output.split(chr(10)))}")
