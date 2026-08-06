"""Convert WebP card images to 3DS per-set texture atlases.

Pipeline: web_ui/img/cards_webp/*.webp
  -> .card_png_cache/*.png               (resized, orientation-detected, cached)
   -> engine_3ds/romfs/cards/*.t3x        (per-set tex3ds atlases)
   -> engine_3ds/romfs/cards_manifest.json (card->atlas+index map)

Resized PNGs are cached in .card_png_cache (OUTSIDE romfs, so they do not bloat
the RomFS). The cache persists between builds, so a PNG is only regenerated when
its source WebP is newer or missing. tex3ds only runs for "dirty" sets (atlas
missing or a card in that set not yet recorded in the manifest).

Orientation: detected per-image. If width > height, card is landscape
and resized to 90xN. Portrait becomes Nx90.

Pass --force to rebuild every atlas from scratch.

Usage:
  python scripts/convert_cards.py              # full pipeline (incremental)
  python scripts/convert_cards.py --force      # rebuild all atlases
  python scripts/convert_cards.py --png-only   # skip tex3ds step
"""

import json
import os
import sys
import subprocess
import argparse
from collections import OrderedDict
from PIL import Image

SRC_REL = "../../../web_ui/img/cards_webp"
CACHE_REL = "../.card_png_cache"
ATLAS_REL = "../romfs/cards"
MANIFEST_REL = "../romfs/cards_manifest.json"
# Long-edge target for resized card PNGs. Higher = sharper when cards are
# blown up in detail view, but larger atlases. Override with RABUKA_CARD_RES.
TARGET_LONG = int(os.environ.get("RABUKA_CARD_RES", "160"))


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


def load_manifest(manifest_path: str) -> dict:
    """Load the existing manifest (card_no -> {atlas, index}), or {} if absent."""
    if not os.path.exists(manifest_path):
        return {}
    try:
        with open(manifest_path, "r", encoding="utf-8") as f:
            data = json.load(f)
        return data if isinstance(data, dict) else {}
    except (json.JSONDecodeError, OSError) as e:
        print(f"[WARN] Could not read existing manifest ({e}); rebuilding all atlases",
              file=sys.stderr)
        return {}


def convert_webp_to_png(src_dir: str, cache_dir: str) -> dict[str, list[str]]:
    """Convert .webp -> .png into the persistent cache, grouped by set prefix.

    A PNG is only regenerated when its source WebP is newer (or the PNG is
    missing), so incremental builds skip unchanged cards entirely.
    """
    os.makedirs(cache_dir, exist_ok=True)
    files = sorted(f for f in os.listdir(src_dir) if f.lower().endswith(".webp"))
    if not files:
        print(f"No .webp files found in {src_dir}")
        return {}

    by_set: dict[str, list[str]] = OrderedDict()
    converted = 0
    for fname in files:
        src = os.path.join(src_dir, fname)
        dst_name = fname.removesuffix(".webp") + ".png"
        dst = os.path.join(cache_dir, dst_name)
        prefix = get_set_prefix(dst_name)
        by_set.setdefault(prefix, []).append(dst_name)

        # Skip if an up-to-date PNG is already cached (incremental)
        if os.path.exists(dst) and os.path.getmtime(dst) >= os.path.getmtime(src):
            continue

        img = Image.open(src).convert("RGBA")
        w, h = img.size

        if w > h:
            new_w, new_h = TARGET_LONG, int(h * TARGET_LONG / w)
        else:
            new_h, new_w = TARGET_LONG, int(w * TARGET_LONG / h)

        img = img.resize((new_w, new_h), Image.LANCZOS)
        img.save(dst, "PNG")
        converted += 1

    print(f"Converted {converted} webp -> PNG into {cache_dir} "
          f"({len(files)} total, {len(by_set)} sets)")
    return by_set


def set_is_dirty(atlas_path: str, manifest: dict, atlas_name: str,
                 card_nos: list[str]) -> bool:
    """True if this set's atlas must be rebuilt (missing or has new cards)."""
    if not os.path.exists(atlas_path):
        return True
    for cn in card_nos:
        entry = manifest.get(cn)
        if not isinstance(entry, dict) or entry.get("atlas") != atlas_name:
            return True
    return False


def generate_atlases(cache_dir: str, atlas_dir: str, by_set: dict[str, list[str]],
                     manifest: dict, force: bool) -> dict:
    """Run tex3ds per dirty set, writing atlases to atlas_dir.

    Reads resized PNGs from the cache, outputs .t3x into romfs/cards, and
    returns the merged card->atlas manifest.
    """
    tex3ds = os.path.join(
        os.environ.get("DEVKITPRO", "C:/devkitPro"), "tools", "bin", "tex3ds.exe"
    )
    if not os.path.exists(tex3ds):
        print(f"tex3ds not found at {tex3ds}, skipping atlas step", file=sys.stderr)
        return manifest

    os.makedirs(atlas_dir, exist_ok=True)
    manifest = dict(manifest)
    built = 0
    skipped = 0
    for set_prefix, pngs in by_set.items():
        atlas_name = f"cards_{set_prefix}.t3x"
        atlas_path = os.path.join(atlas_dir, atlas_name)

        sorted_pngs = sorted(pngs)
        card_nos = [p.removesuffix(".png") for p in sorted_pngs]

        if not force and not set_is_dirty(atlas_path, manifest, atlas_name, card_nos):
            skipped += 1
            continue

        args = [tex3ds, "--atlas", "-o", atlas_path, "-f", "rgba5551", "-z", "auto"]
        args.extend(os.path.join(cache_dir, p) for p in sorted_pngs)

        print(f"  {atlas_name} ({len(sorted_pngs)} cards)...")
        result = subprocess.run(args, capture_output=True, text=True)
        if result.returncode != 0:
            print(f"    FAILED:\n{result.stderr}", file=sys.stderr)
            continue

        # Rebuild manifest entries for this whole set (indices = input order,
        # matching how tex3ds preserves subtexture ordering).
        for idx, png_name in enumerate(sorted_pngs):
            card_no = png_name.removesuffix(".png")
            manifest[card_no] = {"atlas": atlas_name, "index": idx}
        built += 1

    print(f"Created/updated {built} atlas files ({skipped} unchanged)")
    return manifest


def main():
    parser = argparse.ArgumentParser(description="Convert card images for 3DS")
    parser.add_argument(
        "--png-only", action="store_true", help="Skip tex3ds atlas step"
    )
    parser.add_argument(
        "--force", action="store_true",
        help="Rebuild every atlas from scratch (ignores the existing manifest)"
    )
    args = parser.parse_args()

    src_dir = resolve(SRC_REL)
    cache_dir = resolve(CACHE_REL)
    atlas_dir = resolve(ATLAS_REL)
    manifest_path = resolve(MANIFEST_REL)

    by_set = convert_webp_to_png(src_dir, cache_dir)

    if by_set and not args.png_only:
        existing = load_manifest(manifest_path)
        manifest = generate_atlases(cache_dir, atlas_dir, by_set, existing, args.force)

        if manifest:
            with open(manifest_path, "w", encoding="utf-8") as f:
                json.dump(manifest, f, ensure_ascii=False, indent=2)
            print(f"Wrote manifest: {manifest_path} ({len(manifest)} entries)")

    if by_set:
        total = sum(len(v) for v in by_set.values())
        print(f"OK {total} cards, {len(by_set)} sets")


if __name__ == "__main__":
    main()
