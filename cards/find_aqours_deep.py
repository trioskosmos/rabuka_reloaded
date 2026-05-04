"""Find ALL cards related to Aqours across every field."""
import json
cards = json.load(open('cards/cards.json', encoding='utf-8'))

# Check all unique field names
all_fields = set()
for v in cards.values():
    all_fields.update(v.keys())
print("All fields in cards.json:", sorted(all_fields))
print()

# Find Aqours in every field
for k, v in cards.items():
    for field_name, field_val in v.items():
        if isinstance(field_val, str) and 'Aqours' in field_val:
            print(f"  {k} .{field_name} = {repr(field_val)}")
        elif isinstance(field_val, list):
            for item in field_val:
                if isinstance(item, str) and 'Aqours' in item:
                    print(f"  {k} .{field_name}[0] = {repr(item)}")
