#!/usr/bin/env python3
"""Add self_cost and exclude_self fields to abilities.json based on cost text."""
import json

with open('../abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

for ability in data['unique_abilities']:
    cost = ability.get('cost')
    if cost and cost.get('text'):
        cost_text = cost['text']
        
        # Check for self_cost: "このメンバーを" but not "このメンバー以外" or "ほかのメンバー"
        if 'このメンバーを' in cost_text and 'このメンバー以外' not in cost_text and 'ほかのメンバー' not in cost_text:
            cost['self_cost'] = True
        
        # Check for exclude_self: "このメンバー以外" or "ほかのメンバー"
        if 'このメンバー以外' in cost_text or 'ほかのメンバー' in cost_text:
            cost['exclude_self'] = True

with open('../abilities.json', 'w', encoding='utf-8') as f:
    json.dump(data, f, ensure_ascii=False, indent=2)

print("Added self_cost and exclude_self fields to abilities.json")
