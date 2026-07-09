from PIL import Image
import os, glob

base = os.path.dirname(os.path.abspath(__file__))

webp_dir = os.path.join(base, "web_ui", "img", "cards_webp")
webp_files = sorted(glob.glob(os.path.join(webp_dir, "*.webp")))

# Find unique aspect ratios and orientations
aspects = set()
for f in webp_files:
    img = Image.open(f)
    name = os.path.basename(f)
    aspect = round(img.width / img.height, 4)
    aspects.add((img.width, img.height, aspect))

print("=== UNIQUE SOURCE DIMENSIONS ===")
for w, h, a in sorted(aspects):
    orient = "landscape" if w > h else "portrait"
    print(f"  {w}x{h} (ratio {a}) {orient}")

# Count orientations
landscape = sum(1 for f in webp_files if Image.open(f).width > Image.open(f).height)
print(
    f"\nTotal webp: {len(webp_files)}, landscape: {landscape}, portrait: {len(webp_files) - landscape}"
)

# Check a couple PNGs more closely
png_dir = os.path.join(base, "engine_3ds", "romfs", "cards")
png_files = sorted(glob.glob(os.path.join(png_dir, "*.png")))
png_aspects = set()
for f in png_files:
    img = Image.open(f)
    png_aspects.add((img.width, img.height))
print("\n=== UNIQUE PNG OUTPUT DIMENSIONS ===")
for w, h in sorted(png_aspects):
    orient = "landscape" if w > h else "portrait"
    print(f"  {w}x{h} {orient}")
