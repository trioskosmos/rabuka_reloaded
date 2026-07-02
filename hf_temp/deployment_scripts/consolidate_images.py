import json
import os
import glob

CARDS_JSON = "cards/cards.json"
MAPPING_JSON = "web_ui/js/card_image_mapping.json"
IMAGES_DIR = "web_ui/img/cards_webp"
# The path prefix as seen by the web browser/server
IMAGE_PATH_PREFIX = "img/cards_webp/"


def consolidate():
    print("Loading files...")
    with open(CARDS_JSON, "r", encoding="utf-8") as f:
        cards = json.load(f)

    new_mapping = {}
    referenced_images = set()

    print("Calculating consolidation...")
    for card_id, card in cards.items():
        # 1. Energy cards -> single representative image
        if card.get("type") == "エネルギー":
            filename = "LL-E-001-SD.webp"
        # 2. Rarity Reduction -> first rarity in rare_list
        elif (
            "rare_list" in card
            and isinstance(card["rare_list"], list)
            and len(card["rare_list"]) > 1
        ):
            first_rare = card["rare_list"][0]
            filename = first_rare.get("card_no", card.get("card_no", card_id)) + ".webp"
        else:
            # 3. Default -> card's own card_no or id
            filename = card.get("card_no", card_id) + ".webp"

        # Store the full path in the mapping for the frontend
        new_mapping[card_id] = IMAGE_PATH_PREFIX + filename
        # Store only the filename for physical file cleanup
        referenced_images.add(filename)

    # Update mapping.json
    print(f"Updating {MAPPING_JSON}...")
    with open(MAPPING_JSON, "w", encoding="utf-8") as f:
        json.dump(new_mapping, f, ensure_ascii=False, indent=2)

    # Physical Cleanup
    print(f"Cleaning up {IMAGES_DIR}...")
    all_files = glob.glob(os.path.join(IMAGES_DIR, "*.webp"))
    deleted_count = 0
    for file_path in all_files:
        file_name = os.path.basename(file_path)
        if file_name not in referenced_images:
            try:
                os.remove(file_path)
                deleted_count += 1
            except OSError as e:
                print(f"Error deleting {file_name}: {e}")

    print(f"--- Results ---")
    print(f"Unique images kept: {len(referenced_images)}")
    print(f"Files deleted: {deleted_count}")
    print(f"Total cards processed: {len(cards)}")


if __name__ == "__main__":
    consolidate()
