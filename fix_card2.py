with open('engine_c/src/core/card.c', 'r') as f:
    lines = f.readlines()

# Find the line with "CardDatabase methods"
start_idx = None
for i, line in enumerate(lines):
    if 'CardDatabase methods' in line:
        start_idx = i
        break

if start_idx:
    # Keep everything from CardDatabase methods comment onwards
    new_lines = lines[start_idx:]
    with open('engine_c/src/core/card.c', 'w') as f:
        f.writelines(new_lines)
    print(f'Kept from line {start_idx}')
else:
    print('Not found')