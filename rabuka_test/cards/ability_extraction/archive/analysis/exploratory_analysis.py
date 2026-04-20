"""Exploratory analysis to find patterns not predefined - data-driven approach."""
import json
import re
from collections import Counter, defaultdict

# Load abilities
with open('../abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']
texts = [a.get('triggerless_text', '') for a in abilities if a.get('triggerless_text')]

print(f"Exploratory analysis of {len(texts)} abilities...\n")

# Find all unique words/tokens
all_tokens = []
for text in texts:
    # Split by common delimiters
    tokens = re.split(r'[：。、（）\n\s「」『』]', text)
    tokens = [t for t in tokens if t.strip()]
    all_tokens.extend(tokens)

token_counts = Counter(all_tokens)
print("=== TOP 50 TOKENS ===")
for token, count in token_counts.most_common(50):
    print(f"{token}: {count}")

# Find all unique bigrams (2-word sequences)
bigrams = []
for text in texts:
    tokens = re.split(r'[：。、（）\n\s「」『』]', text)
    tokens = [t for t in tokens if t.strip()]
    for i in range(len(tokens) - 1):
        bigrams.append((tokens[i], tokens[i+1]))

bigram_counts = Counter(bigrams)
print("\n=== TOP 30 BIGRAMS ===")
for bigram, count in bigram_counts.most_common(30):
    print(f"{bigram[0]} + {bigram[1]}: {count}")

# Find abilities with unusual characters
unusual_chars = set()
for text in texts:
    for char in text:
        if char not in '：。、（）\n\s「」『』0123456789アイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワヲンガギグゲゴザジズゼゾダヂヅデドバビブベボパピプペポゃゅょっゃゅょ':
            unusual_chars.add(char)

print(f"\n=== UNUSUAL CHARACTERS FOUND ===")
for char in sorted(unusual_chars):
    count = sum(1 for text in texts if char in text)
    print(f"'{char}': {count} occurrences")

# Find abilities by length distribution
lengths = [len(text) for text in texts]
print(f"\n=== LENGTH DISTRIBUTION ===")
print(f"Min: {min(lengths)}")
print(f"Max: {max(lengths)}")
print(f"Mean: {sum(lengths)/len(lengths):.1f}")
print(f"Median: {sorted(lengths)[len(lengths)//2]}")
print(f"75th percentile: {sorted(lengths)[int(len(lengths)*0.75)]}")
print(f"90th percentile: {sorted(lengths)[int(len(lengths)*0.9)]}")

# Find outlier abilities (very long or very short)
outliers = []
for i, text in enumerate(texts):
    if len(text) > 200 or (len(text) > 0 and len(text) < 20):
        outliers.append((i, len(text), text[:100]))

print(f"\n=== LENGTH OUTLIERS ===")
for i, length, text in outliers[:10]:
    print(f"Length {length}: {text}...")

# Find abilities with unique structures not matching common patterns
common_patterns = [
    '：', '場合', 'とき', 'かつ', 'その後', '以下から1つを選ぶ', 'かぎり', 
    'につき', 'もよい', '引く', '置く', '得る', '加える', '登場させる'
]

unmatched = []
for i, text in enumerate(texts):
    if not any(pattern in text for pattern in common_patterns):
        unmatched.append((i, text[:100]))

print(f"\n=== ABILITIES NOT MATCHING COMMON PATTERNS ({len(unmatched)}) ===")
for i, text in unmatched[:20]:
    print(f"{i}: {text}...")

# Find all unique verbs (action words)
verbs = []
for text in texts:
    # Look for words ending in common verb endings
    verb_matches = re.findall(r'([^\s]+)(?:する|る|える|う|く|ぐ|ぶ|む|ぬ)', text)
    verbs.extend(verb_matches)

verb_counts = Counter(verbs)
print(f"\n=== TOP 30 VERBS ===")
for verb, count in verb_counts.most_common(30):
    print(f"{verb}: {count}")

# Find all unique noun-like patterns
nouns = []
for text in texts:
    # Look for words with common noun endings
    noun_matches = re.findall(r'([^\s]+)(?:カード|メンバー|エネルギー|ライブ|ステージ|控え室|手札|デッキ|ハート|ブレード|スコア|コスト|能力|効果)', text)
    nouns.extend(noun_matches)

noun_counts = Counter(nouns)
print(f"\n=== TOP 30 NOUN PATTERNS ===")
for noun, count in noun_counts.most_common(30):
    print(f"{noun}: {count}")

# Find abilities with special icons (patterns like {{...}})
icon_patterns = []
for text in texts:
    icon_matches = re.findall(r'\{\{[^}]+\}\}', text)
    if icon_matches:
        icon_patterns.extend(icon_matches)

icon_counts = Counter(icon_patterns)
print(f"\n=== TOP 20 ICON PATTERNS ===")
for icon, count in icon_counts.most_common(20):
    print(f"{icon}: {count}")

# Find abilities with quoted text
quoted_patterns = []
for text in texts:
    quote_matches = re.findall(r'「([^」]+)」', text)
    if quote_matches:
        quoted_patterns.extend(quote_matches)

quote_counts = Counter(quoted_patterns)
print(f"\n=== TOP 20 QUOTED PATTERNS ===")
for quote, count in quote_counts.most_common(20):
    print(f"{quote}: {count}")

# Find abilities with parenthetical content
paren_patterns = []
for text in texts:
    paren_matches = re.findall(r'（([^）]+)）', text)
    if paren_matches:
        paren_patterns.extend(paren_matches)

paren_counts = Counter(paren_patterns)
print(f"\n=== TOP 20 PARENTHETICAL PATTERNS ===")
for paren, count in paren_counts.most_common(20):
    print(f"{paren}: {count}")

# Cluster abilities by first word/phrase
first_words = []
for text in texts:
    first_word = text.split()[0] if text.split() else ''
    first_words.append(first_word)

first_word_counts = Counter(first_words)
print(f"\n=== TOP 30 FIRST WORDS ===")
for word, count in first_word_counts.most_common(30):
    print(f"{word}: {count}")

# Find abilities with numbers and their surrounding context
number_contexts = defaultdict(list)
for i, text in enumerate(texts):
    matches = re.finditer(r'([^\s]{0,10})(\d+)([^\s]{0,10})', text)
    for m in matches:
        context = f"{m.group(1)}{m.group(2)}{m.group(3)}"
        number_contexts[m.group(2)].append((i, context))

print(f"\n=== NUMBER CONTEXTS BY NUMBER ===")
for num in sorted(number_contexts.keys())[:10]:
    contexts = number_contexts[num]
    print(f"\nNumber {num} ({len(contexts)} occurrences):")
    for idx, ctx in contexts[:5]:
        print(f"  {ctx}")
