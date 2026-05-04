"""Map series values to groups in cards.json."""
import json
cards = json.load(open('cards/cards.json', encoding='utf-8'))

series_to_group = {}
for k, v in cards.items():
    series = v.get('series', '')
    unit = v.get('unit', '')
    if series not in series_to_group:
        # Try to determine the group from series or unit
        group = 'unknown'
        if 'サンシャイン' in series:
            group = 'Aqours'
        elif 'スーパースター' in series or 'Liella' in series:
            group = 'Liella!'
        elif '虹ヶ咲' in series:
            group = '虹ヶ咲'
        elif '蓮ノ空' in series or 'Hasunosora' in series:
            group = '蓮ノ空'
        elif 'μ' in series or 'ラブライブ！' == series.strip():
            group = "μ's"
        series_to_group[series] = {
            'group': group,
            'unit': unit,
            'example_card': k
        }

# Show only unique series
for series, info in sorted(series_to_group.items()):
    print(f"{repr(series[:40]):<45} → {info['group']:<10} (e.g. {info['example_card']})")
