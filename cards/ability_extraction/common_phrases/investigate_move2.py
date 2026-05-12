import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_action, extract_destination

# Test #605: text with デッキの上に置く
t = '自分の控え室にある『Aqours』と『SaintSnow』のライブカードを4枚まで好きな順番でデッキの上に置いてもよい'
print('extract_destination:', extract_destination(t))
r = parse_action(t)
print('parse_action:')
print('  action:', r.get('action'))
print('  source:', r.get('source'))
print('  dest:', r.get('destination'))
print()

# Test #628: text with "デッキの一番上か一番下に置き"
t2 = 'これにより公開したカードをデッキの一番上か一番下に置き'
print('extract_destination:', extract_destination(t2))
r2 = parse_action(t2)
print('parse_action:')
print('  action:', r2.get('action'))
print('  source:', r2.get('source'))
print('  dest:', r2.get('destination'))
print('  position:', r2.get('position'))
print()

# Test #628 explicitly
t3 = 'デッキの一番上か一番下に置く'
print('extract_destination:', extract_destination(t3))
print('extract_deck_position:', __import__('importlib').import_module('parser').extract_deck_position_for_action(t3) if False else 'skip')
