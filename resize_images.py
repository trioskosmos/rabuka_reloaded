import os
from PIL import Image

source_dir = "source_images"
target_dir = r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\web_ui\img\cards_webp"
target_size = (179, 249)

if not os.path.exists(target_dir):
    os.makedirs(target_dir)

for filename in os.listdir(source_dir):
    if filename.endswith('.webp'):
        path = os.path.join(source_dir, filename)
        with Image.open(path) as img:
            img.thumbnail(target_size)
            img.save(os.path.join(target_dir, filename), "WEBP")

print("Resizing complete.")
