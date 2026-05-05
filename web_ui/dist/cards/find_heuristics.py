"""Find ALL heuristic fallback patterns in parser.py.
A heuristic = keyword presence implies semantic meaning without explicit game logic.
"""
import re

code = open('cards/ability_extraction/parser.py', encoding='utf-8').read()
lines = code.split('\n')

print("=== HEURISTIC MARKERS IN PARSER ===")
for i, line in enumerate(lines, 1):
    s = line.strip()
    if 'infer' in s.lower() and '#' not in s[:5]:
        print(f"  L{i}: [INFER] {s[:100]}")

print()

# Find pattern: `if 'KEYWORD' in text` followed by assignment of a game value
# These are keyword→semantic mappings
heuristics = []
for i, line in enumerate(lines, 1):
    s = line.strip()
    # Direct keyword → result field assignments
    m = re.search(r"if '([^']+)' in text:\s*$", s)
    if m:
        kw = m.group(1)
        # Check next few lines for assignment
        for j in range(i, min(i+5, len(lines))):
            n = lines[j].strip()
            am = re.search(r"result\['(\w+)'\]\s*=\s*'(\w+)'", n)
            if am:
                heuristics.append((i, kw, am.group(1), am.group(2)))
                break
            am2 = re.search(r"return '(\w+)'", n)
            if am2:
                heuristics.append((i, kw, 'return', am2.group(1)))
                break

print(f"Keyword→value heuristics: {len(heuristics)}")
for line_no, kw, field, val in heuristics:
    print(f"  L{line_no}: '{kw}' → {field}='{val}'")

# Now find ALL places where '人' is used to infer 'stage' or 'member_card'
print()
print("=== ALL '人' → semantic inferences ===")
for i, line in enumerate(lines, 1):
    if "'人'" in line or '"人"' in line:
        if any(w in line for w in ['stage', 'member_card', 'location', 'card_type']):
            print(f"  L{i}: {line.strip()[:120]}")
