import sys, json
sys.path.insert(0, 'cards/ability_extraction')
import parser

# Test the parsing of the triggerless text
test_text = 'ライブ終了時まで、自分のステージにいる「澁谷かのん」1人は{{heart_05.png|heart05}}{{icon_blade.png|ブレード}}を、「唐可可」1人は{{heart_01.png|heart01}}{{icon_blade.png|ブレード}}を得る。'

# Simulate parse_effect pipeline
text = parser.normalize_fullwidth_digits(test_text).strip()
text = parser.strip_parenthetical(text)
text = parser.strip_suffix_period(text)
print('Cleaned text:', text[:120])

# Try each handler
for handler in parser._EFFECT_HANDLERS:
    result = handler(text)
    if result is not None:
        print(f'Handler {handler.__name__} matched!')
        actions = result.get('actions', [])
        for i, a in enumerate(actions):
            has_tc = 'target_count' in a
            print(f'  Action {i}: target_count={a.get("target_count")}, has_target_count={has_tc}')
            print(f'    Keys: {list(a.keys())[:12]}')
        break
else:
    print('No handler matched')

# Also test with the full text including trigger
full_text = '{{live_start.png|ライブ開始時}}ライブ終了時まで、自分のステージにいる「澁谷かのん」1人は{{heart_05.png|heart05}}{{icon_blade.png|ブレード}}を、「唐可可」1人は{{heart_01.png|heart01}}{{icon_blade.png|ブレード}}を得る。'
print('\n--- Testing with full text (trigger prefix) ---')
# The trigger parsing happens before effect parsing
# Check if the effect parsing handles this correctly
text2 = parser.normalize_fullwidth_digits(full_text).strip()
# strip parenthetical
text2 = parser.strip_parenthetical(text2)
text2 = parser.strip_suffix_period(text2)
print('Text after cleanup:', text2[:120])

# Remove trigger prefix like the main code does
import re
trigger_match = re.match(r'\{\{[^}]+\}\}(.*)', text2)
if trigger_match:
    text2 = trigger_match.group(1).strip()
    print('Text after trigger removal:', text2[:120])

for handler in parser._EFFECT_HANDLERS:
    result = handler(text2)
    if result is not None:
        print(f'Handler {handler.__name__} matched!')
        actions = result.get('actions', [])
        for i, a in enumerate(actions):
            has_tc = 'target_count' in a
            print(f'  Action {i}: target_count={a.get("target_count")}, has_target_count={has_tc}')
        break
