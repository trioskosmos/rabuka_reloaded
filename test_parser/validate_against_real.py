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
    print(f"Testing {len(abilities)} abilities from real data...")

    parser = AbilityParser()

    success_count = 0
    failure_count = 0
    failures = []

    for ability_data in abilities:
        text = ability_data.get("triggerless_text", "")
        if not text:
            continue

        try:
            # In a real scenario, we might want to parse the whole ability.
            # For now, let's see if it can at least handle the text without crashing.
            result = parser.parse_ability(text)

            # A 'successful' parse in this context means we didn't just return the raw text.
            # Since our parser is very basic, let's check if it actually produced an effect.
            if result.effects and result.effects[0].text != text:
                success_count += 1
            else:
                failure_count += 1
                failures.append(
                    {
                        "input": text,
                        "error": "Failed to parse structured effect (returned raw text)",
                    }
                )
        except Exception as e:
            failure_count += 1
            failures.append({"input": text, "error": str(e)})

    print(f"\nResults:")
    print(f"  Successes (structured): {success_count}")
    print(f"  Failures (raw/error):   {failure_count}")
    print(f"  Total:                 {success_count + failure_count}")

    if failures:
        print("\nFirst 10 failures:")
        for f in failures[:10]:
            print(f"  - Input: {f['input']}")
            print(f"    Error: {f['error']}")

        with open(
            os.path.join(os.path.dirname(__file__), "validation_failures.json"),
            "w",
            encoding="utf-8",
        ) as f:
            json.dump(failures, f, ensure_ascii=False, indent=2)
        print(f"\nFull failure list written to validation_failures.json")


if __name__ == "__main__":
    main()
