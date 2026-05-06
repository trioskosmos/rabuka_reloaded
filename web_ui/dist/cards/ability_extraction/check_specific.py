"""Check specific problematic cases."""
import json, os

abilities_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'abilities.json')
with open(abilities_path, 'r', encoding='utf-8') as f:
    data = json.load(f)
abilities = data['unique_abilities']

# Case 1: "のみ発動する" without activation_condition
print("=== のみ発動する CASE (no activation_condition) ===")
for a in abilities:
    ft = a.get('full_text', '')
    if 'のみ発動する' in ft:
        eff = a.get('effect') or {}
        print(f"card_count={a['card_count']}")
        print(f"full_text: {ft}")
        print(f"effect keys: {list(eff.keys())}")
        print(f"activation_condition in effect: {'activation_condition' in eff}")
        print(f"parenthetical in effect: {'parenthetical' in eff}")
        print()

# Case 2: "この能力は" without comma (no comma after は)
print("=== この能力は without comma ===")
for a in abilities:
    ft = a.get('full_text', '')
    if 'この能力はセンターエリア' in ft:
        eff = a.get('effect') or {}
        print(f"card_count={a['card_count']}")
        print(f"full_text: {ft[:200]}")
        print(f"activation_condition: {eff.get('activation_condition', 'MISSING')}")
        print(f"activation_condition_parsed: {eff.get('activation_condition_parsed', 'MISSING')}")
        print()

# Case 3: "この能力は左サイド" / "この能力は右サイド" (side area conditions)
print("=== SIDE AREA activation conditions ===")
for a in abilities:
    ft = a.get('full_text', '')
    if 'この能力は左サイド' in ft or 'この能力は右サイド' in ft:
        eff = a.get('effect') or {}
        print(f"card_count={a['card_count']}")
        print(f"full_text: {ft[:200]}")
        cond = eff.get('activation_condition', 'MISSING')
        acp = eff.get('activation_condition_parsed', 'MISSING')
        print(f"activation_condition: {cond}")
        if acp != 'MISSING':
            print(f"activation_condition_parsed type: {acp.get('type', '?')}")
        else:
            print(f"activation_condition_parsed: MISSING")
        print()

# Case 4: Parenthetical activation conditions for side areas
print("=== PARENTHETICAL activation conditions ===")
for a in abilities:
    ft = a.get('full_text', '')
    if '（この能力は' in ft:
        eff = a.get('effect') or {}
        cond = eff.get('activation_condition', 'MISSING')
        if cond != 'MISSING':
            print(f"card_count={a['card_count']}")
            print(f"ft: {ft[:150]}")
            print(f"activation_condition: {cond}")
            print(f"parenthetical: {eff.get('parenthetical', 'N/A')}")
            print()

# Case 5: Position change with target: None
print("=== POSITION CHANGE with target: None ===")
for a in abilities:
    ft = a.get('full_text', '')
    if 'ポジションチェンジ' in ft:
        eff = a.get('effect') or {}
        if eff.get('target') is None:
            print(f"card_count={a['card_count']} target=None")
            print(f"  ft: {ft[:120]}")
            print()

# Case 6: abilities with garbled text
print("=== CHECKING FOR GARBLED TEXT ===")
import re
garbled_count = 0
for a in abilities:
    ft = a.get('full_text', '')
    eff_text = (a.get('effect') or {}).get('text', '')
    # Check for common garbled patterns
    if '刁' in ft or '𤳈' in ft or 'E�E' in ft:
        garbled_count += 1
        if garbled_count <= 3:
            print(f"card_count={a['card_count']}")
            print(f"  {ft[:150]}")
print(f"Total garbled: {garbled_count}")

print("\nDONE")
