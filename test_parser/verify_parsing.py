import json
import sys
import os

# Add the root directory to sys.path
ROOT_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.append(ROOT_DIR)

from test_parser.parser_v2 import AbilityParser


def main():
    real_abilities_path = os.path.join(os.path.dirname(__file__), "real_abilities.json")

    if not os.path.exists(real_abilities_path):
        print(f"Error: {real_abilities_path} not found.")
        return

    with open(real_abilities_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    abilities = data.get("abilities", [])
    parser = AbilityParser()

    print(f"{'RAW TEXT':<50} | {'PARSED (count, type, card, loc)':<50}")
    print("-" * 110)

    for entry in abilities[:20]:  # Check first 20 for brevity
        text = entry.get("triggerless_text", "")
        if not text:
            continue

        result = parser.parse_ability(text)

        parsed_str = "FAILED"
        if result.effects:
            eff = result.effects[0]
            # Construct a summary string
            parts = []
            if eff.count is not None:
                parts.append(f"n={eff.count}")
            if eff.card_type:
                parts.append(f"ct={eff.card_type}")
            if eff.location:
                parts.append(f"loc={eff.location}")
            if parts:
                parsed_str = ", ".join(parts)
            else:
                parsed_str = "No structured data"
        else:
            parsed_str = "No effects"

        # Truncate raw text for display
        display_text = (text[:47] + "...") if len(text) > 47 else text
        print(f"{display_text:<50} | {parsed_str:<50}")


if __name__ == "__main__":
    main()
