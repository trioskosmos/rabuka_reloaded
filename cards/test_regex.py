import re

# Test single-quote regex
text = "自分のステージにいる'μ's'のメンバー1人は"
print('text:', repr(text))
for m in re.finditer(r"'([^']+)'", text):
    print('match:', repr(m.group(0)), 'name:', repr(m.group(1)))
    print('name len:', len(m.group(1)))

# Now test extract_group from parser
import sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import extract_group
result = extract_group(text)
print('extract_group result:', result)
