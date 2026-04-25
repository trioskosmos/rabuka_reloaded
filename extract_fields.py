import json
from collections import defaultdict

def extract_effect_fields(obj, fields=None):
    if fields is None:
        fields = defaultdict(set)
    
    if isinstance(obj, dict):
        # Check if this is an effect/cost object (has 'action' or 'type' field)
        if 'action' in obj or 'type' in obj:
            for key, value in obj.items():
                fields[key].add(type(value).__name__)
        
        # Recurse into nested structures
        for value in obj.values():
            extract_effect_fields(value, fields)
    elif isinstance(obj, list):
        for item in obj:
            extract_effect_fields(item, fields)
    
    return fields

with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

fields = extract_effect_fields(data)

print("All unique fields in effect/cost objects:")
for field, types in sorted(fields.items()):
    print(f"  {field}: {', '.join(sorted(types))}")
