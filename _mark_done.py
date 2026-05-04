with open('cards/qa_card_list.md', 'r', encoding='utf-8') as f:
    content = f.read()
content = content.replace('PL!-pb1-018-R         矢澤にこ (BiBi)', 'PL!-pb1-018-R         矢澤にこ (BiBi)  ✓ DONE')
with open('cards/qa_card_list.md', 'w', encoding='utf-8') as f:
    f.write(content)
print('Marked done')
