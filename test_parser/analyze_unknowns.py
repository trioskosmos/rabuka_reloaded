#!/usr/bin/env python3
"""Analyze unknown patterns and group them by type."""

import re
from collections import Counter, defaultdict

unknown_text = """   2x: その中から{{heart_02.png|heart02}}か{{heart_04.png|heart04}}か{{heart_05.png|heart05}}を持つメンバーカードを3枚まで公開して手札に加えてもよい
   2x: 自分のステージにいるのメンバー1人のすべての{{live_start.png|ライブ開始時}}能力を、ライブ終了時まで、無効にしてもよい。これにより無効にした場合、自分の控え室からのカードを1枚手札に加える。
   2x: これにより控え室に置いたメンバーカードの{{toujyou.png|登場}}能力1つを発動させる
   2x: 自分のステージから控え室に置く
   2x: これによりアクティブにしたメンバーと
   2x: 2人のメンバーとバトンタッチしてもよい
   2x: ライブの合計スコアが相手より高い場合、自分のエネルギーデッキから、このメンバーの下にあるエネルギーカードの枚数に1を足した枚数のエネルギーカードをウェイト状態で置く。
   2x: このメンバーがステージから控え室に置かれたとき、メンバー1人をポジションチェンジさせてもよい。
   2x: として扱う
   2x: 手札を2枚控え室に置く：自分の控え室から必要ハートに{{heart_06.png|heart06}}を3以上含むライブカードを1枚手札に加える。
   2x: その中からハートに{{heart_04.png|heart04}}を2つ以上持つメンバーカードを1枚公開して手札に加えてもよい"""

# Parse and categorize
patterns = defaultdict(list)
count_by_text = Counter()

for line in unknown_text.split('\n'):
    if not line.strip():
        continue
    match = re.match(r'\s*(\d+)x:\s*(.+)', line)
    if not match:
        continue
    count, text = int(match.group(1)), match.group(2)
    count_by_text[text] += count
    
    # Categorize by pattern
    if '{{' in text and 'として扱う' not in text:
        patterns['icon_selection'].append(text)
    elif 'として扱う' in text:
        patterns['treat_as'].append(text)
    elif '無効にする' in text or '無効に' in text:
        patterns['disable_ability'].append(text)
    elif 'バトンタッチ' in text:
        patterns['position_change'].append(text)
    elif '公開して手札に加える' in text or '公開して手札に加えてもよい' in text:
        patterns['look_and_select'].append(text)
    elif '何もしない' in text:
        patterns['do_nothing'].append(text)
    else:
        patterns['other'].append(text)

print("=== Unknown Pattern Summary ===\n")
for category, items in patterns.items():
    print(f"\n{category.upper()} ({len(items)} unique, {sum(count_by_text[t] for t in items)} total):")
    # Show top 3 by frequency
    sorted_items = sorted(items, key=lambda x: count_by_text[x], reverse=True)
    for item in sorted_items[:3]:
        print(f"  {count_by_text[item]:2d}x: {item[:80]}...")
