import re, difflib, sys
base_path = r'C:\Users\trios\AppData\Local\Temp\kilo\base_norm.txt'
with open(base_path, encoding='utf-8') as f:
    base = f.read().splitlines()
with open(r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json', encoding='utf-8') as f:
    t = f.read()
t = re.sub(r'"generated_at":[^\n]*\n', '', t)
t = re.sub(r'"engine_commit":[^\n]*\n', '', t)
cur = t.splitlines()
d = list(difflib.unified_diff(base, cur, lineterm=''))
print('parser diff lines:', len(d))
for line in d[:40]:
    print(line)
