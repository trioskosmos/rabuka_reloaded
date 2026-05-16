import json
import sys
import os
from pathlib import Path

# Add the root directory to sys.path
ROOT_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.append(ROOT_DIR)

from test_parser.parser_v2 import AbilityParser

def extract_triggers_v2(text: str):
    import re
    trigger_pattern = re.compile(r'\{\{([^|]+)\|([^}]+)\}\}')
    matches = trigger_pattern.findall(text)
    
    triggers = []
    effect_text = text
    for match in matches:
        icon_file = match[0]
        icon_text = match[1]
        if any(x in icon_file for x in ['heart', 'blade', 'energy', 'score']):
            continue
        triggers.append(icon_text)
        effect_text = effect_text.replace(f"{{{{{icon_file}|{icon_text}}}}}", "").strip()
    
    if effect_text.startswith("："):
        effect_text = effect_text[1:].strip()
        
    return triggers, effect_text

def main():
    abilities_json = Path(ROOT_DIR) / "cards" / "abilities.json"
    output_file = Path(__file__).parent / "real_abilities.json"

    if not abilities_json.exists():
        print(f"Error: {abilities_json} not found.")
        return

    with open(abilities_json, 'rb') as f:
        content = f.read().decode('utf-8', errors='ignore')
        data = json.loads(content)
        unique_abilities = data.get("unique_abilities", [])

    parser = AbilityParser()
    abilities = []

    print(f"Parsing {len(unique_abilities)} unique abilities from abilities.json...")

    for item in unique_abilities:
        line = item.get("full_text", "").strip()
        if not line:
            continue
            
        triggers, triggerless = extract_triggers_v2(line)
        parsed = parser.parse_ability(line)
        
        abilities.append({
            "full_text": line,
            "triggerless_text": triggerless,
            "triggers": triggers,
            "parsed": parsed.model_dump()
        })

    output_data = {
        "total_unique_found": len(abilities),
        "abilities": abilities,
    }

    with open(output_file, "w", encoding="utf-8") as f:
        json.dump(output_data, f, ensure_ascii=False, indent=2)

    print(f"Successfully wrote {len(abilities)} abilities to {output_file}")

if __name__ == "__main__":
    main()
