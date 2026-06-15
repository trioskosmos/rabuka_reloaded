"""
Download missing card images from GitHub, convert to webp, fix mapping.
"""

import json, os, sys, urllib.request, time
from pathlib import Path
from PIL import Image

BASE_DIR = Path(__file__).resolve().parent
CARDS_JSON = BASE_DIR / "cards" / "cards.json"
MAPPING_JSON = BASE_DIR / "web_ui" / "js" / "card_image_mapping.json"
WEBP_DIR = BASE_DIR / "web_ui" / "img" / "cards_webp"
GITHUB_API = "https://api.github.com/repos/wlt233/llocg_db/git/trees/master?recursive=1"
RAW_BASE = "https://raw.githubusercontent.com/wlt233/llocg_db/master"

dry_run = "--download" not in sys.argv


def log(msg):
    print(msg, flush=True)


def get_github_files():
    log("Fetching GitHub tree...")
    req = urllib.request.Request(GITHUB_API, headers={"User-Agent": "Mozilla/5.0"})
    with urllib.request.urlopen(req, timeout=30) as resp:
        data = json.loads(resp.read())
    files = []
    for item in data.get("tree", []):
        p = item["path"]
        if p.startswith("img/cards/") and p.endswith(".png"):
            files.append(p)
    log(f"Found {len(files)} PNG files on GitHub")
    return files


def get_card_nos():
    with open(CARDS_JSON, encoding="utf-8") as f:
        return list(json.load(f).keys())


log(f"Mode: {'DRY RUN' if dry_run else 'DOWNLOAD'}")

# Get GitHub files and create name->path mapping
github_files = get_github_files()
github_map = {os.path.splitext(os.path.basename(p))[0]: p for p in github_files}
log(f"Unique image names on GitHub: {len(github_map)}")

# Get all card IDs
card_nos = get_card_nos()
log(f"Cards in cards.json: {len(card_nos)}")

# Read existing mapping
existing_mapping = {}
if MAPPING_JSON.exists():
    with open(MAPPING_JSON, encoding="utf-8") as f:
        existing_mapping = json.load(f)

# Get existing webp files
existing_webps = (
    {f.stem for f in WEBP_DIR.glob("*.webp")} if WEBP_DIR.exists() else set()
)

log(f"Existing mapping entries: {len(existing_mapping)}")
log(f"Existing webp files: {len(existing_webps)}")

# Categorize cards
have_webp = []
missing_webp_found_on_github = []
missing_webp_not_on_github = []
for c in card_nos:
    if c in existing_webps:
        have_webp.append(c)
    elif c in github_map:
        missing_webp_found_on_github.append(c)
    else:
        missing_webp_not_on_github.append(c)

log(f"\nCards with webp: {len(have_webp)}")
log(f"Cards missing webp (found on GitHub): {len(missing_webp_found_on_github)}")
log(f"Cards missing webp (NOT on GitHub): {len(missing_webp_not_on_github)}")

# Fix mapping entries that point to non-existent webp files
fixes = []
for k, v in list(existing_mapping.items()):
    if k not in existing_webps:
        webp_file = WEBP_DIR / os.path.basename(v)
        if webp_file.exists():
            continue  # mapping is fine, file just wasn't in our set
        # Check if the mapping points to a different filename than card_no
        expected = f"img/cards_webp/{k}.webp"
        if v != expected:
            if (WEBP_DIR / f"{k}.webp").exists():
                existing_mapping[k] = expected
                fixes.append(k)
            elif k in github_map:
                pass  # will be downloaded
            else:
                # Update to standard name anyway
                existing_mapping[k] = expected
                fixes.append(k)
        elif k not in github_map:
            # Card not found on GitHub - keep mapping
            pass

if fixes:
    log(f"Fixed {len(fixes)} mapping entries with wrong paths")
    # Remove corrupt entries (those with 白 characters)
    corrupt = [k for k in existing_mapping if "白" in k]
    for k in corrupt:
        del existing_mapping[k]
    log(f"Removed {len(corrupt)} corrupt mapping entries")

# Download missing images
if not dry_run and missing_webp_found_on_github:
    WEBP_DIR.mkdir(parents=True, exist_ok=True)
    downloaded = 0
    failed = 0
    total = len(missing_webp_found_on_github)

    for idx, card_no in enumerate(missing_webp_found_on_github):
        if idx % 50 == 0 and idx > 0:
            log(f"Progress: {idx}/{total} ({downloaded} ok, {failed} failed)")
            # Save progress periodically
            with open(MAPPING_JSON, "w", encoding="utf-8") as f:
                json.dump(existing_mapping, f, ensure_ascii=False, indent=2)

        github_path = github_map[card_no]
        raw_url = f"{RAW_BASE}/{github_path}"
        webp_path = WEBP_DIR / f"{card_no}.webp"
        png_tmp = WEBP_DIR / f"{card_no}.png_tmp"

        try:
            req = urllib.request.Request(raw_url, headers={"User-Agent": "Mozilla/5.0"})
            with urllib.request.urlopen(req, timeout=30) as resp:
                with open(png_tmp, "wb") as f:
                    f.write(resp.read())
            img = Image.open(png_tmp).convert("RGB")
            img.save(webp_path, "WEBP", quality=85)
            png_tmp.unlink(missing_ok=True)
            existing_mapping[card_no] = f"img/cards_webp/{card_no}.webp"
            downloaded += 1
        except Exception as e:
            log(f"  FAILED {card_no}: {e}")
            if png_tmp.exists():
                png_tmp.unlink(missing_ok=True)
            failed += 1
            if "429" in str(e) or "rate limit" in str(e).lower():
                log("Rate limited - waiting 10s...")
                time.sleep(10)

    log(f"\nDownload complete: {downloaded} ok, {failed} failed out of {total}")

# Add mapping entries for existing webp files that aren't in mapping
added = 0
for c in existing_webps:
    if c not in existing_mapping and c in card_nos:
        existing_mapping[c] = f"img/cards_webp/{c}.webp"
        added += 1
if added:
    log(f"Added {added} missing mapping entries for existing webp files")

# Remove mapping entries for cards not in cards.json
# (but keep them since they might be for card variants)
pruned = len(existing_mapping) - len(card_nos)
log(f"Mapping has {len(existing_mapping)} entries ({len(card_nos)} cards in db)")

log("\nSaving mapping...")
with open(MAPPING_JSON, "w", encoding="utf-8") as f:
    json.dump(existing_mapping, f, ensure_ascii=False, indent=2)

# Copy to dist
dist = BASE_DIR / "web_ui" / "dist" / "js" / "card_image_mapping.json"
with open(dist, "w", encoding="utf-8") as f:
    json.dump(existing_mapping, f, ensure_ascii=False, indent=2)

log("Done!")
