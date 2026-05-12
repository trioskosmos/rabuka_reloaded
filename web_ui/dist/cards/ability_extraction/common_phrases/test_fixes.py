import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_effect, parse_ability

ok = True

# Test: modify_score with -N
t = 'ライブの合計スコアを-1する'
r = parse_effect(t)
print(f'Test modify_score -1: action={r.get("action")} op={r.get("operation")} val={r.get("value")}')
if r.get('operation') != 'remove' or r.get('value') != 1:
    print('FAIL: modify_score -1')
    ok = False

# Test: modify_score with +1
t = 'ライブの合計スコアを+1する'
r = parse_effect(t)
print(f'Test modify_score +1: action={r.get("action")} op={r.get("operation")} val={r.get("value")}')
if r.get('operation') != 'add' or r.get('value') != 1:
    print('FAIL: modify_score +1')
    ok = False

if ok:
    print('All modify_score tests passed')
else:
    print('SOME TESTS FAILED')

