import re

with open('test_qa_data.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Fix all embedded newlines in string literals
# This pattern matches strings with embedded newlines and removes them
content = re.sub(r'"([^\n"]*)\n\s*([^\n"]*)"', r'"\1\2"', content)

# Fix specific known issues
content = content.replace('"Auto ability\n should have', '"Auto ability should have')
content = content.replace('"Auto ability trigger count should be 0 after\n clearing', '"Auto ability trigger count should be 0 after clearing')
content = content.replace('"Hand should be\n empty', '"Hand should be empty')
content = content.replace('"Deck should be\n empty initially', '"Deck should be empty initially')
content = content.replace('"Energy zone should be\n empty initially', '"Energy zone should be empty initially')
content = content.replace('"Waitroom should be\n empty initially', '"Waitroom should be empty initially')
content = content.replace('"Center should be\n empty', '"Center should be empty')
content = content.replace('"LeftSide should be\n empty', '"LeftSide should be empty')
content = content.replace('"RightSide should be\n empty', '"RightSide should be empty')
content = content.replace('"\nShould have member cards', '"Should have member cards')
content = content.replace('"\nShould have energy cards', '"Should have energy cards')

with open('test_qa_data.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Fixed all embedded newlines in string literals')
