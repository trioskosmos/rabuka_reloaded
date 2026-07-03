import os
import math
from PIL import Image
import pillow_avif

source_dir = "source_images"
target_dir = r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\web_ui\img\cards_webp"
target_pixels = 179 * 249

if not os.path.exists(target_dir):
    os.makedirs(target_dir)

# Clear target directory of old .webp files
for f in os.listdir(target_dir):
    if f.endswith('.webp'):
        os.remove(os.path.join(target_dir, f))

for filename in os.listdir(source_dir):
    if filename.endswith('.webp'):
        path = os.path.join(source_dir, filename)
        with Image.open(path) as img:
            w, h = img.size
            aspect_ratio = w / h
            
            # W * H = target_pixels; W = aspect_ratio * H
            # aspect_ratio * H^2 = target_pixels
            new_h = int(math.sqrt(target_pixels / aspect_ratio))
            new_w = int(new_h * aspect_ratio)
            
            img_resized = img.resize((new_w, new_h), Image.Resampling.LANCZOS)
            
            target_filename = os.path.splitext(filename)[0] + ".avif"
            img_resized.save(os.path.join(target_dir, target_filename), "AVIF")

print("Conversion complete.")
