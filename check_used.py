import sys
sys.path.insert(0, 'tools')
from bake_card_art import deck_card_nos
used = deck_card_nos()
print(f'Total used: {len(used)}')
print(f'PL!SP-bp1-011-R in used: {"PL!SP-bp1-011-R" in used}')
print(f'PL!SP-bp1-011-P in used: {"PL!SP-bp1-011-P" in used}')