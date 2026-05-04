"""Extract character→group mappings from rules.txt."""
import re
text = open('engine/rules/rules.txt', encoding='utf-8').read()

# Find the section that maps characters to groups
# It looks like: "高海千歌 Aqours" or similar patterns
lines = text.split('\n')
for i, line in enumerate(lines):
    # Look for lines with a Japanese name followed by a group name
    # Pattern: Japanese characters, space, group name
    m = re.match(r'^(\S+)\s+(.+)$', line.strip())
    if m:
        name, group = m.group(1), m.group(2)
        # Filter to only real mappings (not section headers or other text)
        if group in ['Aqours', "μ's", '虹ヶ咲', 'Liella!', '蓮ノ空', 'SaintSnow', 
                      'CYaRon!', 'AZALEA', 'GuiltyKiss',
                      'BiBi', 'Printemps', 'lilywhite',
                      'CatChu!', '5yncri5e!', 'KALEIDOSCORE',
                      'Qu4rtz', 'DiverDiva', 'A・ZU・NA', 'R3BIRTH',
                      'DOLLCHESTRA', 'みらくらぱーく！', 'スリーズブーケ', 'EdelNote',
                      'SunnyPassion', 'A-RISE']:
            print(f"'{name}' => '{group}'")
