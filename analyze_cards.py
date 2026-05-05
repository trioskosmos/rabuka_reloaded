# -*- coding: utf-8 -*-
import json, re, sys

def main():
    cards = json.load(open('cards/cards.json', 'r', encoding='utf-8'))
    abilities_data = json.load(open('cards/abilities.json', 'r', encoding='utf-8'))
    unique_abilities = abilities_data.get('unique_abilities', [])

    # Build: (card_no, ab_slot) -> uab_idx
    card_ab_to_uab = {}
    for uab_idx, entry in enumerate(unique_abilities):
        cards_list = entry.get('cards', [])
        for card_ref in cards_list:
            m = re.match(r'^(.+?)\s*\|.*?\(ab#(\d+)\)', card_ref)
            if m:
                cno = m.group(1).strip()
                ab_slot = int(m.group(2))
                if cno not in card_ab_to_uab:
                    card_ab_to_uab[cno] = {}
                card_ab_to_uab[cno][ab_slot] = uab_idx

    # Build Q-code to uab_idx mapping from the 'q_code' field if it exists
    qcode_to_uab = {}
    for uab_idx, entry in enumerate(unique_abilities):
        qc = entry.get('q_code')
        if qc:
            qcode_to_uab[qc] = uab_idx

    # Also try to find Q codes in card references in the cards list
    for uab_idx, entry in enumerate(unique_abilities):
        for card_ref in entry.get('cards', []):
            # Check if card_ref itself contains a Q code reference
            qm = re.search(r'(Q\d+)', card_ref)
            if qm:
                qc = qm.group(1)
                if qc not in qcode_to_uab:
                    qcode_to_uab[qc] = uab_idx

    target_slots = [
        ('PL!-pb1-001-R', [0, 1]),
        ('PL!HS-bp1-022-L', [0, 1]),
        ('PL!N-bp5-030-L', [0, 1]),
        ('PL!S-pb1-002-R', [0, 1]),
        ('PL!SP-bp4-023-L', [0, 1]),
        ('PL!S-bp3-019-L', [0, 1]),
        ('PL!SP-bp5-005-R+', [0, 1]),
        ('PL!-pb1-009-R', [0]),
    ]

    output = []
    output.append("=" * 80)
    output.append("CARD ABILITY ANALYSIS: RAW TEXT vs PARSED JSON")
    output.append("=" * 80)
    output.append("")

    for cid, slots in target_slots:
        cdata = cards.get(cid, {})
        raw_ability = cdata.get('ability', 'NO ABILITY TEXT')
        card_name = cdata.get('name', '???')

        output.append("=" * 70)
        output.append(f"CARD: {cid} ({card_name})")
        output.append("=" * 70)
        output.append("")
        output.append("--- RAW ABILITY TEXT (from cards.json) ---")
        output.append(raw_ability)
        output.append("")

        q_in_raw = re.findall(r'(Q\d+)', raw_ability)
        output.append(f"Q-codes found in raw text: {q_in_raw}")
        output.append("")

        for ab_slot in slots:
            output.append(f"--- PARSED: ability slot ab#{ab_slot} ---")

            uab_idx = None
            if cid in card_ab_to_uab and ab_slot in card_ab_to_uab[cid]:
                uab_idx = card_ab_to_uab[cid][ab_slot]

            if uab_idx is not None and uab_idx < len(unique_abilities):
                entry = unique_abilities[uab_idx]
                output.append(f"  Unique ability index: #{uab_idx}")
                output.append(f"  Q code: {entry.get('q_code', 'N/A')}")
                output.append(f"  Full text: {entry.get('full_text', 'N/A')}")
                output.append(f"  Triggerless text: {entry.get('triggerless_text', 'N/A')}")
                output.append(f"  Triggers: {entry.get('triggers', 'N/A')}")
                output.append(f"  Use limit: {entry.get('use_limit', 'N/A')}")
                output.append(f"  Is null: {entry.get('is_null', 'N/A')}")
                if entry.get('activation_condition'):
                    output.append(f"  Activation condition: {entry['activation_condition']}")
                output.append("")

                cost = entry.get('cost', {})
                output.append("  COST:")
                if isinstance(cost, dict):
                    for k, v in cost.items():
                        output.append(f"    {k}: {v}")
                else:
                    output.append(f"    {cost}")

                output.append("")
                output.append("  EFFECT:")
                effect = entry.get('effect', {})
                if isinstance(effect, dict):
                    for k, v in effect.items():
                        output.append(f"    {k}: {v}")
                elif isinstance(effect, list):
                    for i, e in enumerate(effect):
                        output.append(f"    [{i}]:")
                        if isinstance(e, dict):
                            for k, v in e.items():
                                output.append(f"      {k}: {v}")
                        else:
                            output.append(f"      {e}")
                else:
                    output.append(f"    {effect}")

                output.append("")
                output.append("  ANALYSIS / POTENTIAL ISSUES:")
                issues = analyze_ability(cid, raw_ability, entry)
                if issues:
                    for issue in issues:
                        output.append(f"    [ISSUE] {issue}")
                else:
                    output.append("    No obvious issues detected.")
            else:
                output.append(f"  ** NOT FOUND in unique_abilities for ab#{ab_slot} **")
                available = card_ab_to_uab.get(cid, {})
                output.append(f"  Available slots: {available}")

            output.append("")
            output.append("-" * 50)
            output.append("")

    # Cross-reference: show ALL Q codes in raw text and their mapping
    output.append("=" * 70)
    output.append("CROSS-REFERENCE: Q-codes in raw text -> unique ability index")
    output.append("=" * 70)
    for cid, slots in target_slots:
        cdata = cards.get(cid, {})
        raw_ability = cdata.get('ability', '')
        q_in_raw = re.findall(r'(Q\d+)', raw_ability)
        for qc in q_in_raw:
            uab_idx = qcode_to_uab.get(qc)
            if uab_idx is not None:
                entry = unique_abilities[uab_idx]
                output.append(f"  {cid}: {qc} -> uab#{uab_idx}, trigger={entry.get('triggers')}, null={entry.get('is_null')}")
            else:
                output.append(f"  {cid}: {qc} -> NOT MAPPED")

    return '\n'.join(output)


def analyze_ability(cid, raw_text, parsed_entry):
    issues = []
    cost = parsed_entry.get('cost', {})
    effect = parsed_entry.get('effect', {})
    triggers = parsed_entry.get('triggers', '')
    is_null = parsed_entry.get('is_null', False)

    if is_null:
        issues.append("Entry is marked as is_null=True (parser could not parse this ability)")

    if not cost:
        issues.append("Missing cost field entirely")
    elif isinstance(cost, dict):
        if not cost.get('type') and not cost.get('text'):
            issues.append("Cost has no type or text")
    elif isinstance(cost, list):
        if len(cost) == 0:
            issues.append("Cost is an empty list")
    else:
        issues.append(f"Cost has unexpected type: {type(cost).__name__}")

    if not effect:
        issues.append("Missing effect field entirely")
    elif isinstance(effect, dict):
        if not effect.get('action') and not effect.get('text'):
            issues.append("Effect has no action or text")
    elif isinstance(effect, list):
        if len(effect) == 0:
            issues.append("Effect is an empty list")
        for i, e in enumerate(effect):
            if isinstance(e, dict) and not e.get('action') and not e.get('text'):
                issues.append(f"Effect[{i}] has no action or text")
    else:
        issues.append(f"Effect has unexpected type: {type(effect).__name__}")

    if not triggers:
        issues.append("No trigger specified (triggers field empty/falsy)")

    return issues


if __name__ == '__main__':
    output = main()
    with open('card_analysis_output.txt', 'w', encoding='utf-8') as f:
        f.write(output)
    print("Done. Output written to card_analysis_output.txt")
