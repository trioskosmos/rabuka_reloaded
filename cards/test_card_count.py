import sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import _try_card_count

text = "自分のステージにメンバーが1人以上いる場合"
result = _try_card_count(text)
print('Result:', result)

text2 = "3人以上"
result2 = _try_card_count(text2)
print('Result2:', result2)
