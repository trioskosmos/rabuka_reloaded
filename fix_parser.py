import re

with open(
    r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\ability_extraction\parser.py",
    encoding="utf-8",
) as f:
    content = f.read()

# Fix 1: cost characters, first instance
old1 = '    # Card names from \u300c\u300d\n    name_matches = re.findall(r"\u300c([^\u300d]+)\u300d", text)\n    if name_matches:\n        cost["characters"] = name_matches\n\n'

new1 = """    # Card names from \u300c\u300d \u2014 detect exclusion patterns (\u300cname\u300d\u4ee5\u5916)
    name_matches = re.findall(r"\u300c([^\u300d]+)\u300d", text)
    include_chars = []
    exclude_chars = []
    for name in name_matches:
        idx = text.find(f"\u300c{name}\u300d")
        if idx >= 0:
            after = text[idx + len(f"\u300c{name}\u300d"):idx + len(f"\u300c{name}\u300d") + 3]
            if after.startswith("\u4ee5\u5916"):
                exclude_chars.append(name)
            else:
                include_chars.append(name)
    if include_chars:
        cost["characters"] = include_chars
    if exclude_chars:
        cost["exclude_characters"] = exclude_chars

"""

content = content.replace(old1, new1, 1)

# Fix 2: cost characters in parse_cost
old2 = '    names = re.findall(r"\u300c([^\u300d]+)\u300d", text)\n    if names:\n        cost["characters"] = names\n    if "てもよい" in text or "てもいい" in text or "もよい" in text:\n        cost["optional"] = True'

new2 = """    names = re.findall(r"\u300c([^\u300d]+)\u300d", text)
    include_chars = []
    exclude_chars = []
    for name in names:
        idx = text.find(f"\u300c{name}\u300d")
        if idx >= 0:
            after = text[idx + len(f"\u300c{name}\u300d"):idx + len(f"\u300c{name}\u300d") + 3]
            if after.startswith("\u4ee5\u5916"):
                exclude_chars.append(name)
            else:
                include_chars.append(name)
    if include_chars:
        cost["characters"] = include_chars
    if exclude_chars:
        cost["exclude_characters"] = exclude_chars
    if "てもよい" in text or "てもいい" in text or "もよい" in text:
        cost["optional"] = True"""

content = content.replace(old2, new2, 1)
assert old1 not in content, "fix 1 not applied"
assert old2 not in content, "fix 2 not applied"

with open(
    r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\ability_extraction\parser.py",
    "w",
    encoding="utf-8",
) as f:
    f.write(content)

print("Parser fixes applied")
