with open('engine_c/src/core/card.c', 'r') as f:
    content = f.read()

# Find the CardDatabase methods start
idx = content.find('int rb_card_get_card_id')
if idx >= 0:
    # Keep only from CardDatabase methods onwards
    new_content = content[idx:]
    with open('engine_c/src/core/card.c', 'w') as f:
        f.write(new_content)
    print('Done')
else:
    print('Not found')