import json
ABILITIES_FILE = 'cards/abilities.json'
with open(ABILITIES_FILE, encoding='utf-8') as f:
    data = json.load(f)
abilities = data['unique_abilities']

# Entry index -> description mapping from our analysis
entries = {
    145: '1_choudo_blade_missing_operator_bladelimit',
    199: '2_kagiri_unless_not_as_long_as',
    216: '3_goukei_cost_missing_aggregate',
    557: '4_goukei_heart_missing_aggregate',
    558: '5_goukei_heart2_missing_aggregate',
    234: '6_sorezore_condition_no_multiple_targets',
    188: '7_center_condition_no_activation_position',
    500: '8_center_targetting_no_activation_position',
    536: '9_center_set_value_no_activation_position',
    400: '10_original_value_heart_compare_missing',
}

for idx, desc in entries.items():
    a = abilities[idx]
    t = a.get('triggerless_text', '') or a.get('full_text', '')
    eff = a.get('effect', {})
    cost = a.get('cost', {})
    print('=' * 60)
    print('IDX=' + str(idx) + ' | ' + desc)
    print('TEXT: ' + t)
    print()
    if isinstance(eff, dict) and eff:
        print('EFFECT:')
        print(json.dumps(eff, ensure_ascii=False, indent=2))
    if isinstance(cost, dict) and cost:
        print('COST:')
        print(json.dumps(cost, ensure_ascii=False, indent=2))
    print()
