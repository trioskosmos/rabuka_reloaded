import json

def find_custom_fields(obj, path=""):
    """Recursively find all fields with type: custom"""
    customs = []
    if isinstance(obj, dict):
        if obj.get('type') == 'custom':
            customs.append((path, obj))
        for k, v in obj.items():
            customs.extend(find_custom_fields(v, f"{path}.{k}" if path else k))
    elif isinstance(obj, list):
        for i, item in enumerate(obj):
            customs.extend(find_custom_fields(item, f"{path}[{i}]"))
    return customs

with open('abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

all_customs = []
for item in data['unique_abilities']:
    customs = find_custom_fields(item)
    if customs:
        all_customs.append({
            'triggerless': item.get('triggerless_text', ''),
            'full_text': item.get('full_text', ''),
            'customs': customs
        })

print(f"Found {len(all_customs)} entries with custom types\n")

with open('custom_types_analysis.txt', 'w', encoding='utf-8') as f:
    for i, entry in enumerate(all_customs):
        f.write(f"--- Entry {i+1} ---\n")
        f.write(f"Text: {entry['triggerless'][:200]}...\n")
        for path, custom in entry['customs']:
            f.write(f"  Path: {path}\n")
            f.write(f"  Fields: {list(custom.keys())}\n")
            f.write(f"  Full custom: {custom}\n")
        f.write("\n")

print("Saved to custom_types_analysis.txt")
