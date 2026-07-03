import os
import math
from PIL import Image
import pillow_avif
from concurrent.futures import ProcessPoolExecutor

source_dir = "source_images"
target_dir = r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\web_ui\img\cards_webp"
target_pixels = 179 * 249

def process_image(filename):
    try:
        path = os.path.join(source_dir, filename)
        with Image.open(path) as img:
            w, h = img.size
            aspect_ratio = w / h
            new_h = int(math.sqrt(target_pixels / aspect_ratio))
            new_w = int(new_h * aspect_ratio)
            img_resized = img.resize((new_w, new_h), Image.Resampling.LANCZOS)
            target_filename = os.path.splitext(filename)[0] + ".avif"
            img_resized.save(os.path.join(target_dir, target_filename), "AVIF")
        return True
    except Exception as e:
        return f"Error processing {filename}: {e}"

if __name__ == "__main__":
    if not os.path.exists(target_dir):
        os.makedirs(target_dir)

    for f in os.listdir(target_dir):
        if f.endswith('.webp'):
            os.remove(os.path.join(target_dir, f))

    files = [f for f in os.listdir(source_dir) if f.endswith('.webp')]
    with ProcessPoolExecutor() as executor:
        results = list(executor.map(process_image, files))
    
    print(f"Processed {len(results)} images.")
