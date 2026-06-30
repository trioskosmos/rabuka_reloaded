import json
import os
import glob

CARDS_JSON = "cards/cards.json"
MAPPING_JSON = "web_ui/js/card_image_mapping.json"
IMAGES_DIR = "web_ui/img/cards_webp"


def consolidate():
    with open(CARDS_JSON, "r", encoding="utf-8") as f:
        cards = json.load(f)

    referenced_images = set()

    # 1. Consolidate cards.json
    for card in cards:
        # Energy cards use a single representative image
        if card.get("type") == "エネルギー":
            card["image"] = "LL-E-001-SD.webp"
        # Cards with multiple rarities use the first one in rare_list
        elif "rare_list" in card and len(card["rare_list"]) > 1:
            # Use the first rarity's image if it exists, otherwise keep current
            first_rare = card["rare_list"][0]
            if "image" in first_rare:
                card["image"] = first_rare["image"]

        if card.get("image"):
            referenced_images.add(card["image"])

    with open(CARDS_JSON, "w", encoding="utf-8") as f:
        json.dump(cards, f, ensure_ascii=False, indent=2)

    # 2. Update mapping.json to synchronize with cards.json
    # Assuming mapping is { "card_id": "image.webp" } or similar
    with open(MAPPING_JSON, "r", encoding="utf-8") as f:
        mapping = json.load(f)

    # Create a lookup for the updated images
    id_to_image = {
        str(card["id"]): card["image"]
        for card in cards
        if "id" in card and "image" in card
    }

    updated_mapping = {}
    for cid, img in mapping.items():
        if cid in id_to_image:
            updated_mapping[cid] = id_to_image[cid]
        else:
            updated_mapping[cid] = img

    with open(MAPPING_JSON, "w", encoding="utf-8") as f:
        json.dump(updated_mapping, f, ensure_ascii=False, indent=2)

    # 3. Cleanup physical files
    all_files = glob.glob(os.path.join(IMAGES_DIR, "*.webp"))
    for file_path in all_files:
        file_name = os.path.basename(file_path)
        if file_name not in referenced_images:
            try:
                os.remove(file_path)
            except OSError as e:
                print(f"Error deleting {file_name}: {e}")

    print(f"Consolidation complete. Referenced images: {len(referenced_images)}")


if __name__ == "__main__":
    consolidate()
