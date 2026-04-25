import json
from collections import defaultdict

def extract_all_fields(obj, path="", fields=None):
    if fields is None:
        fields = defaultdict(set)
    
    if isinstance(obj, dict):
        for key, value in obj.items():
            current_path = f"{path}.{key}" if path else key
            fields[current_path].add(type(value).__name__)
            extract_all_fields(value, current_path, fields)
    elif isinstance(obj, list):
        for i, item in enumerate(obj):
            current_path = f"{path}[{i}]"
            extract_all_fields(item, current_path, fields)
    
    return fields

with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

fields = extract_all_fields(data)

print("All unique field paths and their types:")
for path, types in sorted(fields.items()):
    print(f"{path}: {', '.join(sorted(types))}")
