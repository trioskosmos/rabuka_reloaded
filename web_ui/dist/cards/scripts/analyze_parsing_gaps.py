"""
Analyze parsing gaps between card ability text and the parser's dispatch table.

Extracts all dispatch patterns from parser.py, then finds stuff in card text
that doesn't match any pattern. Reports the "signal" — overlooked game mechanics.
"""

import re, json, sys, os
from pathlib import Path

# Load abilities
abilities_path = Path(__file__).parent.parent / "abilities.json"
with open(abilities_path, "r", encoding="utf-8") as f:
    data = json.load(f)
abilities = data["unique_abilities"]

# ----- 1. Extract ALL dispatch patterns from parser.py -----
parser_path = Path(__file__).parent.parent / "ability_extraction" / "parser.py"
with open(parser_path, "r", encoding="utf-8") as f:
    parser_source = f.read()

# Find the normalize_action function that contains R() calls
# Extract all string literals used as conditions for R() calls
# Also extract all lambda condition bodies

# Find R() calls: R("string", "action", ...) or R(lambda t: ..., "action", ...)
dispatch_entries = []

# Match R() calls - both string and lambda conditions
r_calls = re.finditer(
    r"R\(\s*("
    r'"[^"]*"|'  # string literal condition
    r"lambda\s+t[^,]+"  # lambda condition
    r')\s*,\s*"([^"]+)"',  # action name
    parser_source,
)

for m in r_calls:
    cond = m.group(1).strip()
    action = m.group(2).strip()
    dispatch_entries.append((cond, action))

print(f"Dispatch entries found: {len(dispatch_entries)}")
print(f"Unique actions: {len(set(a for _, a in dispatch_entries))}")
print(f"Actions: {sorted(set(a for _, a in dispatch_entries))}")

# ----- 2. Extract common phrases/patterns from triggerless_text -----
texts = [a.get("triggerless_text", "") for a in abilities]

# Extract ALL quoted text patterns
quoted_patterns = re.compile(r"「([^」]+)」")
all_quoted = []
for t in texts:
    all_quoted.extend(quoted_patterns.findall(t))

# Count unique quoted phrases
from collections import Counter

quoted_counts = Counter(all_quoted)
print(f"\nUnique quoted phrases: {len(quoted_counts)}")

# Also extract all icon patterns: {{...}}
icon_patterns = re.compile(r"\{\{([^}]+)\}\}")
all_icons = []
for t in texts:
    all_icons.extend(icon_patterns.findall(t))

icon_counts = Counter(all_icons)
print(f"Unique icon patterns: {len(icon_counts)}")

# ----- 3. Extract unique term/phrase substrings -----
# Split texts into meaningful segments: between punctuation
min_freq = 2
segments = []
for t in texts:
    # Split on Japanese punctuation
    parts = re.split(r"[、。：\n]", t)
    for p in parts:
        p = p.strip()
        if len(p) >= 6:  # At least 3 chars of meaningful text
            # Remove icons for pattern matching
            clean = re.sub(r"\{\{[^}]+\}\}", "", p).strip()
            if len(clean) >= 6:
                segments.append(clean)

segment_counts = Counter(segments)
print(
    f"\nUnique text segments (freq>={min_freq}): {sum(1 for _, c in segment_counts.items() if c >= min_freq)}"
)

# ----- 4. Find terms that look like game mechanics but aren't in dispatch -----
# Common game-mechanic particles/patterns in Japanese ability text
mechanic_patterns = [
    (r"を[失得]う", "を失う/を得る"),
    (r"[をが]選ぶ", "を選ぶ"),
    (r"[をが]選んで", "を選んで"),
    (r"公開する", "公開する"),
    (r"[をに]移動", "を/に移動"),
    (r"[にへ]置く", "に置く"),
    (r"[にへ]置いて", "に置いて"),
    (r"引く", "引く"),
    (r"引いて", "引いて"),
    (r"[をが]見る", "を見る"),
    (r"[をが]見て", "を見て"),
    (r"変更する", "変更する"),
    (r"支払う", "支払う"),
    (r"増やす|増える|増加", "増やす"),
    (r"減らす|減る|減少", "減らす"),
    (r"戻す|戻る", "戻す"),
    (r"出す", "出す"),
    (r"入れる", "入れる"),
    (r"合計", "合計"),
    (r"すべての|全ての", "すべての"),
    (r"任意", "任意"),
    (r"好きな", "好きな"),
    (r"可能|できない", "可能"),
    (r"ターン|フェイズ|ライブ", "ターン/フェイズ/ライブ"),
    (r"コスト", "コスト"),
    (r"ハート|heart", "ハート"),
    (r"ブレード|blade", "ブレード"),
    (r"スコア|score", "スコア"),
    (r"デッキ", "デッキ"),
    (r"手札", "手札"),
    (r"控え室|waitroom", "控え室"),
    (r"エネルギー|energy", "エネルギー"),
    (r"ステージ|stage", "ステージ"),
    (r"エリア|エール", "エリア/エール"),
]

# For each mechanic, count how many abilities contain it
mechanic_counts = []
for pattern, name in mechanic_patterns:
    count = sum(1 for t in texts if re.search(pattern, t))
    mechanic_counts.append((name, count, pattern))

mechanic_counts.sort(key=lambda x: -x[1])
print("\n=== Mechanic patterns by frequency ===")
for name, count, pattern in mechanic_counts:
    print(f"  {count:4d}x  {name:25s}")

# ----- 5. Find quoted text that doesn't look like a character name -----
# Character names are typically short (<=5 chars in Japanese)
# Longer quoted text likely contains ability or resource descriptions
suspicious_quotes = []
for q, count in quoted_counts.most_common(200):
    clean_icon = re.sub(r"\{\{[^}]+\}\}", "", q).strip()
    if len(clean_icon) >= 6 and count >= 2:
        # Not just a character name - could be an ability description
        suspicious_quotes.append((count, q, clean_icon))

print(f"\n=== Quoted text that might be abilities (freq>=2, len>=6) ===")
print(f"  Count: {len(suspicious_quotes)}")
for count, q, clean in suspicious_quotes[:30]:
    print(f"  {count:3d}x  {clean[:60]}")

# ----- 6. Find abilities that have no well-known action -----
# Check if there are many "custom" actions that shouldn't be
action_counts = Counter(a.get("effect", {}).get("action", "none") for a in abilities)
print(f"\n=== Action distribution ===")
for action, count in action_counts.most_common(20):
    print(f"  {count:4d}x  {action}")

# Show the one "custom" action
for a in abilities:
    eff = a.get("effect") or {}
    if eff.get("action") == "custom":
        text = a.get("triggerless_text", "")
        print(f"\nCUSTOM: {text[:120]}")

# ----- 7. Find repeated sentence-ending patterns -----
# In Japanese ability text, the verb/concept at the end often determines
# the action type. Extract sentence endings.
endings = []
for t in texts:
    # Get the last verb-like segment
    sentences = re.split(r"[。]", t)
    for s in sentences:
        s = s.strip()
        if s and len(s) >= 4:
            clean = re.sub(r"\{\{[^}]+\}\}", "", s).strip()
            if clean:
                endings.append(clean[-30:])  # Last ~30 chars

ending_counts = Counter(endings)
print(f"\n=== Unique sentence endings (showing freq>=3) ===")
for ending, count in ending_counts.most_common(50):
    if count >= 3:
        print(f"  {count:3d}x  {ending}")
