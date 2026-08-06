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
from concurrent.futures import ThreadPoolExecutor, as_completed
from PIL import Image

SRC_REL = "../../../web_ui/img/cards_webp"
CACHE_REL = "../.card_png_cache"
ATLAS_REL = "../romfs/cards"
MANIFEST_REL = "../romfs/cards_manifest.json"
# Long-edge target for resized card PNGs (1:1 with the ~188px detail display).
# Higher = sharper but larger atlases. Override with RABUKA_CARD_RES.
TARGET_LONG = int(os.environ.get("RABUKA_CARD_RES", "192"))
# Texture format for the .t3x atlases. ETC1/ETC1A4 are 4-bit-per-pixel GPU
# formats the 3DS decodes natively (~4x smaller than rgba5551). Use
# rgba5551 for maximum quality at the cost of size. Forcing etc1 flattens
# card alpha (fills transparency) for the smallest atlases.
TEX_FORMAT = os.environ.get("RABUKA_TEX_FMT", "etc1")
# ETC1 works on 4x4 blocks, so card sizes must be multiples of 4.
BLOCK = 4
# How many tex3ds jobs to run at once. tex3ds is single-threaded per file, so
# running several in parallel speeds up the atlas build dramatically.
PARALLEL = max(1, min(8, int(os.environ.get("RABUKA_PARALLEL", "8"))))
CACHE_RES_MARKER = ".res"


def align4(v: int) -> int:
    return max(BLOCK, (v // BLOCK) * BLOCK)


def load_card_image(path: str) -> Image.Image:
    """Open a source card image, and flatten alpha when the target format is
    ETC1 (which has no alpha) so transparent regions don't encode as garbage."""
    img = Image.open(path).convert("RGBA")
    if TEX_FORMAT == "etc1":
        bg = Image.new("RGBA", img.size, (0, 0, 0, 255))
        img = Image.alpha_composite(bg, img).convert("RGB")
    return img


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
    missing). If the resolution marker in the cache differs from the current
    TARGET_LONG, the whole cache is cleared and re-encoded so the resize
    actually takes effect.
    """
    os.makedirs(cache_dir, exist_ok=True)
    marker = os.path.join(cache_dir, CACHE_RES_MARKER)
    cur_marker = ""
    if os.path.exists(marker):
        try:
            cur_marker = open(marker, "r", encoding="ascii").read().strip()
        except OSError:
            cur_marker = ""
    # Resolution OR format change invalidates the cache (ETC1 needs 4-aligned
    # dims, so the resized PNGs must be re-encoded when the format changes).
    want_marker = f"{TARGET_LONG}|{TEX_FORMAT}"
    if cur_marker != want_marker:
        for f in os.listdir(cache_dir):
            if f.lower().endswith(".png"):
                os.remove(os.path.join(cache_dir, f))
        print(f"Card cache stale (res/format change) - re-encoding")
        with open(marker, "w", encoding="ascii") as f:
            f.write(want_marker)

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

        img = load_card_image(src)
        w, h = img.size

        if w > h:
            new_w = align4(TARGET_LONG)
            new_h = align4(int(h * TARGET_LONG / w))
        else:
            new_h = align4(TARGET_LONG)
            new_w = align4(int(w * TARGET_LONG / h))

        img = img.resize((new_w, new_h), Image.LANCZOS)
        img.save(dst, "PNG")
        converted += 1

    print(f"Converted {converted} webp -> PNG into {cache_dir} "
          f"({len(files)} total, {len(by_set)} sets)")
    return by_set


def set_is_dirty(atlas_dir: str, manifest: dict, atlas_names: list[str],
                 card_nos: list[str]) -> bool:
    """True if this set's atlases must be rebuilt (missing or has new cards)."""
    for an in atlas_names:
        if not os.path.exists(os.path.join(atlas_dir, an)):
            return True
    for cn in card_nos:
        entry = manifest.get(cn)
        if not isinstance(entry, dict):
            return True
        if not any(entry.get("atlas") == an for an in atlas_names):
            return True
    return False


def chunk_by_area(cache_dir: str, sorted_pngs: list[str], max_area: int) -> list[list[str]]:
    """Split a set's PNGs into chunks whose total pixel area fits one texture.

    tex3ds packs a chunk into a single texture; oversized sets would fail with
    "No atlas solution found", so we chunk so each atlas stays within limits.
    """
    chunks: list[list[str]] = []
    cur: list[str] = []
    cur_area = 0
    for p in sorted_pngs:
        with Image.open(os.path.join(cache_dir, p)) as im:
            a = im.size[0] * im.size[1]
        if cur and cur_area + a > max_area:
            chunks.append(cur)
            cur = []
            cur_area = 0
        cur.append(p)
        cur_area += a
    if cur:
        chunks.append(cur)
    return chunks


def _build_one(tex3ds: str, cache_dir: str, atlas_dir: str,
               atlas_name: str, chunk: list[str], tex_format: str):
    """Run tex3ds for a single atlas chunk. Returns (atlas_name, chunk, result)."""
    atlas_path = os.path.join(atlas_dir, atlas_name)
    args = [tex3ds, "--atlas", "-o", atlas_path, "-f", tex_format, "-z", "auto"]
    args.extend(os.path.join(cache_dir, p) for p in chunk)
    result = subprocess.run(args, capture_output=True, text=True)
    return atlas_name, chunk, result


def generate_atlases(cache_dir: str, atlas_dir: str, by_set: dict[str, list[str]],
                     manifest: dict, force: bool) -> dict:
    """Run tex3ds per dirty set, chunking oversized sets into multiple atlases.

    Reads resized PNGs from the cache, outputs .t3x files into romfs/cards, and
    returns the merged card->atlas manifest. Each card maps to (atlas, index);
    a set may span several atlases (cards_<set>_0.t3x, _1.t3x, ...).

    tex3ds is single-threaded per file, so chunks are built concurrently to
    make the atlas build much faster.
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
    # If the atlas resolution/format marker differs from the current settings,
    # every set is stale and must be rebuilt.
    res_changed = manifest.get("_res") is None or str(manifest.get("_res")) != str(TARGET_LONG)
    fmt_changed = manifest.get("_fmt") is None or str(manifest.get("_fmt")) != TEX_FORMAT
    max_area = 850 * 850  # conservative pixel budget per atlas texture (fits 1024^2)

    tasks = []
    for set_prefix, pngs in by_set.items():
        sorted_pngs = sorted(pngs)
        chunks = chunk_by_area(cache_dir, sorted_pngs, max_area)
        atlas_names = [f"cards_{set_prefix}_{i}.t3x" for i in range(len(chunks))]
        card_nos = [p.removesuffix(".png") for p in sorted_pngs]

        if not force and not res_changed and not fmt_changed and not set_is_dirty(atlas_dir, manifest, atlas_names, card_nos):
            skipped += 1
            continue
        for atlas_name, chunk in zip(atlas_names, chunks):
            tasks.append((atlas_name, chunk))

    if tasks:
        print(f"Building {len(tasks)} atlas files ({skipped} sets unchanged) "
              f"at res={TARGET_LONG} fmt={TEX_FORMAT} ({PARALLEL} parallel)...")
        with ThreadPoolExecutor(max_workers=PARALLEL) as ex:
            futs = {ex.submit(_build_one, tex3ds, cache_dir, atlas_dir, an, ch, TEX_FORMAT): (an, ch)
                    for an, ch in tasks}
            for fut in as_completed(futs):
                atlas_name, chunk, result = fut.result()
                if result.returncode != 0:
                    print(f"    FAILED {atlas_name}:\n{result.stderr}", file=sys.stderr)
                    continue
                for idx, png_name in enumerate(chunk):
                    card_no = png_name.removesuffix(".png")
                    manifest[card_no] = {"atlas": atlas_name, "index": idx}
                built += 1

    manifest["_res"] = str(TARGET_LONG)
    manifest["_fmt"] = TEX_FORMAT
    print(f"Built {built} atlas files ({skipped} sets unchanged) "
          f"at res={TARGET_LONG} fmt={TEX_FORMAT}")
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
            # Persist the resolution/format markers so the build script can
            # detect a change and trigger an atlas rebuild.
            res_marker = os.path.join(atlas_dir, "..", "cards_res.txt")
            with open(res_marker, "w", encoding="ascii") as f:
                f.write(str(TARGET_LONG))
            fmt_marker = os.path.join(atlas_dir, "..", "cards_fmt.txt")
            with open(fmt_marker, "w", encoding="ascii") as f:
                f.write(TEX_FORMAT)

    if by_set:
        total = sum(len(v) for v in by_set.values())
        print(f"OK {total} cards, {len(by_set)} sets")


if __name__ == "__main__":
    main()
