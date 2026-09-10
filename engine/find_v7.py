with open('src/bot/strategy_v7.rs', 'r') as f:
    content = f.read()

# Find the exact text
idx = content.find('Doctrine 1: passable')
idx2 = content.find('// Doctrine 2', idx)
if idx2 >= 0:
    print('EXACT CONTENT:')
    print(repr(content[idx:idx2+100]))
else:
    print('Doctrine 2 not found')