import json
import subprocess
import re
from pathlib import Path


def main():
    # Paths
    root_dir = Path(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded")
    abilities_json_path = root_dir / "cards" / "abilities.json"
    output_json_path = root_dir / "cards" / "untested_abilities.json"
    test_log_path = root_dir / "engine" / "test_output.log"

    print("Running all tests and logging to file... This may take a while.")
    try:
        with open(test_log_path, "w", encoding="utf-8") as log_file:
            # On Windows with shell=True, use a string command
            subprocess.run(
                "cargo test -- --nocapture --test-threads=1",
                stdout=log_file,
                stderr=subprocess.STDOUT,
                cwd=root_dir / "engine",
                shell=True,
                text=True,
            )
    except Exception as e:
        print(f"Error running tests: {e}")
        return

    print("Analyzing test logs...")
    triggered_card_ids = set()
    triggered_ability_texts = set()

    if not test_log_path.exists():
        print(f"Error: Log file {test_log_path} was not created.")
        return

    with open(test_log_path, "r", encoding="utf-8", errors="ignore") as f:
        lines = f.readlines()

        current_ability_text = []
        is_capturing_text = False
        for line in lines:
            # Extract card IDs
            triggered_card_ids.update(re.findall(r"card_no=([^\s\n\r]+)", line))
            triggered_card_ids.update(re.findall(r"ability=([^\s\n\r_]+)", line))
            triggered_card_ids.update(re.findall(r" Ability\[([^\]]+)\]:", line))

            # Ability text capturing
            if line.startswith("[AB]  TEXT "):
                is_capturing_text = True
                current_ability_text = [line[len("[AB]  TEXT ") :].strip()]
            elif is_capturing_text:
                if (
                    line.startswith("[AB]")
                    or line.startswith("DEBUG:")
                    or line.startswith("[TRIGGER]")
                    or line.startswith("[LIVE")
                    or line.startswith("[AUTO")
                    or line.startswith("test ")
                ):
                    triggered_ability_texts.add(" ".join(current_ability_text).strip())
                    is_capturing_text = False
                    if line.startswith("[AB]  TEXT "):
                        is_capturing_text = True
                        current_ability_text = [line[len("[AB]  TEXT ") :].strip()]
                else:
                    current_ability_text.append(line.strip())

        if is_capturing_text:
            triggered_ability_texts.add(" ".join(current_ability_text).strip())

    print(f"Found {len(triggered_card_ids)} triggered card IDs.")
    print(f"Found {len(triggered_ability_texts)} triggered ability texts.")

    # Load abilities.json
    with open(abilities_json_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    unique_abilities = data.get("unique_abilities", [])

    untested_unique = []
    for ab in unique_abilities:
        is_tested = False
        for card_str in ab.get("cards", []):
            card_id = card_str.split(" | ")[0].strip()
            if card_id in triggered_card_ids:
                is_tested = True
                break
        if is_tested:
            continue

        full_text = ab.get("full_text", "")
        normalized_full_text = " ".join(full_text.split())
        for log_text in triggered_ability_texts:
            normalized_log_text = " ".join(log_text.split())
            if normalized_full_text == normalized_log_text:
                is_tested = True
                break
        if not is_tested:
            untested_unique.append(ab)

    print(f"Total unique abilities: {len(unique_abilities)}")
    print(f"Untested unique abilities: {len(untested_unique)}")

    # Sort by complexity: count keys in cost and effect, ignoring metadata like 'cards'
    def get_complexity(ab):
        score = 0
        if ab.get("triggers"):
            score += 1
        cost = ab.get("cost")
        if isinstance(cost, dict):
            score += len(cost.keys())
        effect = ab.get("effect")
        if isinstance(effect, dict):
            score += len(effect.keys())
        if ab.get("use_limit") is not None:
            score += 1
        return score

    untested_unique.sort(key=get_complexity, reverse=True)

    output_data = {
        "schema": data.get("schema", "extracted_abilities.v1"),
        "generated_at": data.get("generated_at", ""),
        "generated_by": "generate_untested_abilities.py",
        "source_file": data.get("source_file", ""),
        "statistics": {
            "total_unique_untested": len(untested_unique),
            "total_untested_cards": sum(
                len(ab.get("cards", [])) for ab in untested_unique
            ),
        },
        "unique_abilities": untested_unique,
    }

    # Write to JSON file
    with open(output_json_path, "w", encoding="utf-8") as f:
        json.dump(output_data, f, indent=2, ensure_ascii=False)

    # Also write to TXT file to match the original requested format
    output_txt_path = root_dir / "cards" / "untested_abilities.txt"
    txt_lines = [
        f"Untested member cards with abilities: {output_data['statistics']['total_untested_cards']}\n"
    ]

    for ab in untested_unique:
        trigger_str = ab.get("triggers", "")
        if trigger_str:
            parts = [p.strip() for p in trigger_str.split(",")]
            trigger_set = "{" + ", ".join(f"'{p}'" for p in parts) + "}"
        else:
            trigger_set = "{}"

        full_text = ab.get("full_text", "")
        for card_str in ab.get("cards", []):
            card_id = card_str.split(" | ")[0].strip()
            txt_lines.append(f"{card_id}: triggers={trigger_set}")
            txt_lines.append(f"  {full_text}\n")

    with open(output_txt_path, "w", encoding="utf-8") as f:
        f.write("\n".join(txt_lines))

    print(
        f"Successfully wrote untested abilities to {output_json_path} and {output_txt_path}"
    )


if __name__ == "__main__":
    main()
