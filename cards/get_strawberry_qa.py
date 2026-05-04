"""Get full QA entries for Strawberry Trapper."""
import json
qa = json.load(open('cards/qa_data.json', encoding='utf-8'))
for q in qa:
    for rc in q.get('related_cards', []):
        if 'PL!S-pb1-021' in rc.get('card_no', ''):
            print(f"ID: {q['id']}")
            print(f"Date: {q.get('date','')}")
            print(f"Q: {q['question']}")
            print(f"A: {q['answer']}")
            print()
