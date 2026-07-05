import os
from PIL import Image
from pathlib import Path

src_dir = Path("source_images")
dst_dir = Path("web_ui/img/cards_webp")

dst_dir.mkdir(parents=True, exist_ok=True)

for file_path in src_dir.iterdir():
    if file_path.is_file():
        try:
            # Open image
            with Image.open(file_path) as img:
                # Define output path with .webp extension
                output_path = dst_dir / (file_path.stem + ".webp")
                # Save as webp
                img.save(output_path, "WEBP")
            print(f"Converted {file_path.name} -> {output_path.name}")
        except Exception as e:
            print(f"Error converting {file_path.name}: {e}")
