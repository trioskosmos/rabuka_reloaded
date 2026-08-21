import re, difflib
base = open(r'C:\Users\trios\AppData\Local\Temp\kilo\base_norm.txt', encoding='utf-8').read().splitlines()
t = open(r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json', encoding='utf-8').read()
t = re.sub(r'"generated_at":[^\n]*\n', '', t)
t = re.sub(r'"engine_commit":[^\n]*\n', '', t)
cur = t.splitlines()
d = list(difflib.unified_diff(base, cur, lineterm=''))
with open(r'C:\Users\trios\AppData\Local\Temp\kilo\g7_diff.txt', 'w', encoding='utf-8') as f:
    f.write('\n'.join(d))
print('total diff lines:', len(d))
