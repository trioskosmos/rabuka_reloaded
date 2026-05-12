import re

with open('src/qa_test_suite.rs', 'rb') as f:
    data = f.read()

# Decode with replace
text = data.decode('utf-8', errors='replace')
print('File length:', len(text))

# Check for smart quotes that might cause issues
for char_name, char in [("LEFT SINGLE QUOTATION MARK", "\u2018"), ("RIGHT SINGLE QUOTATION MARK", "\u2019"),
                         ("LEFT DOUBLE QUOTATION MARK", "\u201C"), ("RIGHT DOUBLE QUOTATION MARK", "\u201D")]:
    count = text.count(char)
    if count > 0:
        print(f"Found {count} of {char_name} (U+{ord(char):04X})")

# The issue might be with \n at start of strings - that's valid Rust
# Let me check lines around errors
lines = text.split('\n')
problem_lines = [1248, 1265, 1272, 1276, 1285, 1286, 1287, 1315, 1319, 1332, 1338, 1344, 1361, 1364, 1366, 1370, 1414, 1415, 1428, 1429, 1431, 1435]

for ln in problem_lines:
    if ln < len(lines):
        line_text = lines[ln-1]  # 0-indexed
        print(f"Line {ln}: {repr(line_text[:120])}")
