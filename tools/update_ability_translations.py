import json
import os

ABILITIES_FILE = "cards/abilities.json"
TRANSLATIONS_FILE = "web_ui/js/i18n/ability_translations.json"

def main():
    # Load current abilities
    if not os.path.exists(ABILITIES_FILE):
        print(f"Error: {ABILITIES_FILE} not found")
        return

    with open(ABILITIES_FILE, "r", encoding="utf-8") as f:
        abilities_data = json.load(f)

    unique_abilities = [ab["full_text"] for ab in abilities_data.get("unique_abilities", [])]

    # Load current translations
    translations = {}
    if os.path.exists(TRANSLATIONS_FILE):
        with open(TRANSLATIONS_FILE, "r", encoding="utf-8") as f:
            try:
                translations = json.load(f)
            except json.JSONDecodeError:
                translations = {}

    # Find new abilities and add them with placeholder
    added_count = 0
    for text in unique_abilities:
        if text not in translations:
            # Use the Japanese text as a placeholder, or a marker
            translations[text] = text
            added_count += 1

    # Write back to translation file
    # Sort keys to keep it organized
    sorted_translations = {k: translations[k] for k in sorted(translations.keys())}
    
    with open(TRANSLATIONS_FILE, "w", encoding="utf-8") as f:
        json.dump(sorted_translations, f, ensure_ascii=False, indent=2)

    print(f"Updated {TRANSLATIONS_FILE}:")
    print(f"  Total abilities: {len(unique_abilities)}")
    print(f"  Added {added_count} new entries.")
    print(f"  Total translation entries: {len(sorted_translations)}")

if __name__ == "__main__":
    main()
