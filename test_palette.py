import time
from pathlib import Path
import sys

REPO = Path(__file__).resolve().parent
sys.path.insert(0, str(REPO / "cards"))

from tools.bake_card_art import all_card_nos, load_image, make_thumb, build_palette, CACHE, FRONT_W, FRONT_H, FRONT_GRID, STAGE_W, STAGE_H, STAGE_GRID, LIVE_W, LIVE_H, LIVE_GRID, WAIT_W, WAIT_H, WAIT_GRID, TILE, BACK_PNG

used = all_card_nos()
print(f'{len(used)} cards')

sample = list(used)[:200]
thumbs = []
for card_no in sample:
    result = load_image(card_no)
    if result[1] is None:
        continue
    img = result[1]
    for w, h, grid in [
        (FRONT_W, FRONT_H, FRONT_GRID),
        (STAGE_W, STAGE_H, STAGE_GRID),
        (LIVE_W, LIVE_H, LIVE_GRID),
        (WAIT_W, WAIT_H, WAIT_GRID),
    ]:
        thumb = make_thumb(img, w, h).resize((grid[0]*TILE, grid[1]*TILE), Image.LANCZOS)
        thumbs.append(thumb)
if BACK_PNG.exists():
    thumbs.append(
        Image.open(BACK_PNG).convert("RGB").resize((LIVE_W, LIVE_H), Image.LANCZOS)
    )
print(f'{len(thumbs)} thumbs')

start = time.time()
q, pal = build_palette(thumbs, colors=240)
print(f'Palette build: {time.time() - start:.2f}s')