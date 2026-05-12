import json

with open('../cards/abilities.json', encoding='utf-8') as f:
    abilities = json.load(f)

print('All keys:', list(abilities.keys()))

if 'unique_abilities' in abilities:
    ab_list = abilities['unique_abilities']
    print('Abilities type:', type(ab_list))
    print('Abilities count:', len(ab_list))
    
    debut_cards = []
    for i, ab in enumerate(ab_list):
        if isinstance(ab, dict) and ab.get('triggers') == '登場' and not ab.get('is_null', True):
            effect = ab.get('effect', {})
            if isinstance(effect, dict) and effect.get('action') == 'move_cards':
                continue
            cards = ab.get('cards', [])
            for c in cards:
                if isinstance(c, str) and '|' in c:
                    card_no = c.split('|')[0].strip()
                    debut_cards.append((card_no, ab.get('full_text', '')[:80]))
    
    print(f'Found {len(debut_cards)} debut ability cards')
    for card_no, text in debut_cards[:20]:
        print(f'  {card_no}: {text}')
