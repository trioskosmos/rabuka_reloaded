"""Convert WebP card images to 3DS per-set texture atlases.

Pipeline: web_ui/img/cards_webp/*.webp
  -> engine_3ds/romfs/cards/*.png        (resized, orientation-detected)
  -> engine_3ds/romfs/cards/*.t3x        (per-set tex3ds atlases)
  -> engine_3ds/romfs/cards_manifest.json (card->atlas+index map)

Orientation: detected per-image. If width > height, card is landscape
and resized to 90xN. Portrait becomes Nx90.

Usage:
  python scripts/convert_cards.py              # full pipeline
  python scripts/convert_cards.py --png-only   # skip tex3ds step
"""

import json
import os
import sys
import subprocess
import argparse
from collections import OrderedDict
from PIL import Image

SRC_REL = "../../web_ui/img/cards_webp"
DST_REL = "../romfs/cards"
TARGET_LONG = 90


def resolve(path: str) -> str:
    return os.path.normpath(os.path.join(os.path.dirname(__file__), path))


def get_set_prefix(filename: str) -> str:
    """Extract set prefix from card_no-based filename.

    LL-bp1-001-R2.png  ->  LL-bp1
    PL!-bp3-020-L.png  ->  PL!-bp3
    LL-PR-004-PR.png   ->  LL-PR
    PL!N-PR-038-PR.png ->  PL!N-PR
    """
    parts = filename.rsplit("-", 2)  # remove rarity + number
    return "-".join(parts[0].split("-")[:2])  # first 2 segments


def convert_webp_to_png(src_dir: str, dst_dir: str) -> dict[str, list[str]]:
    """Convert all .webp to .png, grouped by set prefix. Returns {set: [png_names]}."""
    os.makedirs(dst_dir, exist_ok=True)
    files = sorted(f for f in os.listdir(src_dir) if f.lower().endswith(".webp"))
    if not files:
        print(f"No .webp files found in {src_dir}")
        return {}

    by_set: dict[str, list[str]] = OrderedDict()
    for fname in files:
        src = os.path.join(src_dir, fname)
        dst_name = fname.removesuffix(".webp") + ".png"
        dst = os.path.join(dst_dir, dst_name)

        img = Image.open(src).convert("RGBA")
        w, h = img.size

        if w > h:
            new_w, new_h = TARGET_LONG, int(h * TARGET_LONG / w)
        else:
            new_h, new_w = TARGET_LONG, int(w * TARGET_LONG / h)

        img = img.resize((new_w, new_h), Image.LANCZOS)
        img.save(dst, "PNG")

        prefix = get_set_prefix(dst_name)
        by_set.setdefault(prefix, []).append(dst_name)

    print(f"Converted {len(files)} webp -> PNG in {dst_dir} ({len(by_set)} sets)")
    return by_set


def generate_atlases(png_dir: str, by_set: dict[str, list[str]]) -> dict:
    """Run tex3ds per set, return card->atlas manifest."""
    tex3ds = os.path.join(
        os.environ.get("DEVKITPRO", "C:/devkitPro"), "tools", "bin", "tex3ds.exe"
    )
    if not os.path.exists(tex3ds):
        print(f"tex3ds not found at {tex3ds}, skipping atlas step", file=sys.stderr)
        return {}

    manifest = {}
    for set_prefix, pngs in by_set.items():
        atlas_name = f"cards_{set_prefix}.t3x"
        atlas_path = os.path.join(png_dir, atlas_name)

        args = [tex3ds, "--atlas", "-o", atlas_path, "-f", "rgba5551", "-z", "auto"]
        args.extend(os.path.join(png_dir, p) for p in pngs)

        print(f"  {atlas_name} ({len(pngs)} cards)...")
        result = subprocess.run(args, capture_output=True, text=True)
        if result.returncode != 0:
            print(f"    FAILED:\n{result.stderr}", file=sys.stderr)
            continue

        # Build manifest: card_no (without .png) -> atlas + index
        for idx, png_name in enumerate(pngs):
            card_no = png_name.removesuffix(".png")
            manifest[card_no] = {"atlas": atlas_name, "index": idx}

    print(f"Created {len(by_set)} atlas files")
    return manifest


def main():
    parser = argparse.ArgumentParser(description="Convert card images for 3DS")
    parser.add_argument(
        "--png-only", action="store_true", help="Skip tex3ds atlas step"
    )
    args = parser.parse_args()

    src_dir = resolve(SRC_REL)
    dst_dir = resolve(DST_REL)

    by_set = convert_webp_to_png(src_dir, dst_dir)

    manifest = {}
    if by_set and not args.png_only:
        manifest = generate_atlases(dst_dir, by_set)

    # Write manifest
    if manifest:
        manifest_path = resolve("../romfs/cards_manifest.json")
        with open(manifest_path, "w", encoding="utf-8") as f:
            json.dump(manifest, f, ensure_ascii=False, indent=2)
        print(f"Wrote manifest: {manifest_path} ({len(manifest)} entries)")

    if by_set:
        total = sum(len(v) for v in by_set.values())
        print(f"OK {total} cards, {len(by_set)} sets")


if __name__ == "__main__":
    main()
