# Adding a new expansion pack

End-to-end procedure for importing a new Love Live OCG expansion into the engine.
Written while importing **PBLL02 プレミアムブースター ラブライブ！DUO** (113 cards,
release 2026.10.03) — every number, path and command below is from that run.

---

## 0. What you are actually touching

```
cards/cards.json                      master card database (source of truth)
cards/abilities.json                  GENERATED - never hand-edit
cards/build/cards.bin{,.z}            GENERATED
cards/build/generation_manifest.json  GENERATED
engine/src/core/cards_gen.rs          GENERATED (gitignored, see .gitignore:192)
engine/src/ability/abilities_gen.rs  GENERATED, checked in (bytecode the engine loads)
cards/images/<EXP>/<card_no>.png     source card art
web_ui/img/cards_webp/<card_no>.webp  art the web UI loads
```

The engine never parses `cards.json` at runtime. It loads a compact binary blob and
the ability bytecode, so **a new pack is not "done" until the artifacts are
regenerated** or the cards simply will not exist at runtime.

---

## 1. Find the expansion code

The browser URL carries it: `?expansion=<CODE>`.

`PBLL02` = プレミアムブースター ラブライブ！DUO.

Codes in use: `BP01`–`BP07` (booster packs), `PBLL`, `PBLS`, `PBSP`, `PBSP02`,
`PBN`, `PBHS` (premium boosters), `NSD01/02`, `SPSD01/02`, `SSD01`, `HSSD01`,
`PLSD01` (starter decks), `PR`, `CLHS01`.

The **card-number prefix is a different thing** from the expansion code and is not
guessable from the product name:

| Expansion | Product | Card numbers |
|---|---|---|
| `PBLL` | プレミアムブースターラブライブ！ | `PL!-pb1-*` |
| `PBLL02` | プレミアムブースターラブライブ！DUO | `PL!-pb2-*` **and** `PL!-bp3/bp4/bp5-*`, `PL!-sd1-*` |
| `PBSP02` | プレミアムブースターラブライブ！スーパースター!!DUO | `PL!SP-pb2-*` |

`PBLL02` mixes four numbering schemes (79 `pb2`, 9 `bp3`, 8 `bp4`, 5 `bp5`, 5 `sd1`) —
new cards plus reprints numbered inside their original sets. **Never infer the set
from the card number.**

---

## 2. Scrape

```bash
python cards/scrape_expansion.py PBLL02            # write cards.json + images
python cards/scrape_expansion.py PBLL02 --dry-run  # report only, change nothing
python cards/scrape_expansion.py PBLL02 --no-images
```

`cards/scrape_expansion.py` is the tool written for this. It was built from
`llocg_db/database.py` (the community DB scraper) after that script stopped working —
**read section 3 before assuming either one still functions.**

Output:

```
loaded 2526 existing cards
PBLL02: 113 cards listed
product name: 'プレミアムブースターラブライブ！DUO'
new: 113 | already present & identical: 0 | changed: 0
wrote .../cards/cards.json (2639 cards)
downloading images: {'ok': 113}
```

The scraper writes PNGs to `cards/images/<EXP>/` and WebP to
`web_ui/img/cards_webp/` (Pillow required for the WebP step; it degrades to
PNG-only with a warning if Pillow is missing).

---

## 3. Why the old scrapers stopped working

This is the part that will bite you next time. Two independent scrapers broke, for
the same reason: **the site was rebuilt onto a JSON API.**

The old `llocg_db/database.py` still runs — it just returns nothing:

```
$ python database.py
0 products fetched
```

| Its call | Result |
|---|---|
| `GET /cardlist/cardsearch_ex?expansion=..&limit=100` | HTTP 200, but **zero** `card="..."` matches — the regex target no longer exists in the markup |
| `POST /cardlist/detail/` with `cardno=` | **HTTP 404 for every card**, including ones already in our DB |
| scrape of `<a class="productsList-Item">` on `/cardlist/` | 0 matches |

Verified against **BP07, a set we already had**, not a guess.

The working endpoints are declared by the `PWOCG_CFG` object inlined in any cardlist
page. Read it first when the site changes again:

```powershell
$r = Invoke-WebRequest "https://llofficial-cardgame.com/cardlist/searchresults/?expansion=PBLL02" -UseBasicParsing
$c = $r.Content; $i = $c.IndexOf("var PWOCG_CFG"); $c.Substring($i, 1200)
```

`apiBase` = `https://llofficial-cardgame.com/manage`, endpoints:

| Endpoint | Use |
|---|---|
| `GET /manage/card-list-user/list?title=<CODE>&page=N&per_page=100&sort=no` | card list; `total` is authoritative |
| `GET /manage/card-list-user/detail?id=<numeric id>` | full card. **Keyed by numeric `id`, not card number** |
| `GET /manage/card-list-user/products` | product list |
| `GET /manage/card-list-user/createSearchForm` | search-form schema |

There is no "give me all cards" call — page through `list` until `len == total`.

---

## 4. Field mapping (the part that will silently corrupt data if guessed)

Derived by diffing **all 2372 cards we already owned** against this API. Base
hearts, member cost, member blade and live score matched with **0 mismatches**; the
rest is below.

| Local field | API source | Notes |
|---|---|---|
| `card_no` | `card_number` | keeps fullwidth `＋` |
| `img` / `_img` | `picture` | `_img` = `img/cards/<picture>`; the filename uses `2` for `＋` |
| `name` | `card_name` | |
| `product` | `expansion_name` | **strip `-` and whitespace**: ブースターパック MELLOW MOMENT → ブースターパックMELLOWMOMENT |
| `type` | `card_kind` | メンバー / ライブ / エネルギー |
| `series` | derived from card number | order matters, see below |
| `unit` | `unit_name` | omit when `-` |
| `rare` | `rare` | halfwidth `+` (the API mixes this with fullwidth `＋` between sets) |
| `ability` | `text` | icon tokens → `{{file\|label}}` |
| **メンバー** `cost` | `cost` | |
| **メンバー** `base_heart` | `heart01`..`heart06` | zero values omitted; `heart0` (無) is **not** included |
| **メンバー** `blade` | `attack` | omit when `-` |
| **メンバー** `blade_heart` | `blade_heart` | |
| **ライブ** `score` | **`blade_heart`** | the fields are **inverted** for live cards |
| **ライブ** `need_heart` | `heart0`..`heart06` | **includes** `heart0` (無) |
| **ライブ** `blade_heart` | **`attack`** | |
| `special_heart: {}` | — | emitted when live `cost` is スコア or ドロー |
| `rare_list` | — | groups prints sharing a **card number**, not a name |

### 4.1 The two traps

**Blade-heart palette ≠ base-heart palette.** Measured over ~800 sampled cards:

| Colour | base (`base_heart`/`need_heart`) | blade (`blade_heart`) |
|---|---|---|
| 桃 | `heart01` | `b_heart01` |
| 赤 | `heart04` | `b_heart02` |
| 黄 | `heart05` | `b_heart03` |
| 緑 | `heart02` | `b_heart04` |
| 青 | `heart03` | `b_heart05` |
| 紫 | `heart06` | `b_heart06` |
| 無 | `heart0` | `b_heart07` |

Deriving blade hearts from the base table produces a card that is quietly wrong —
it compiles, it loads, it just grants the wrong colour. Keep the two tables separate
(`BASE_HEART_IDX` / `BLADE_HEART_IDX` in `cards/scrape_expansion.py`).

**Live cards invert two fields.** For a live card the API's `blade_heart` holds the
*score* and `attack` holds the *blade hearts*. Read the tables again before touching
that code.

### 4.2 Series derivation order

```python
SERIES_PREFIXES = [
    ("PL!N",  "ラブライブ！虹ヶ咲学園スクールアイドル同好会"),
    ("PL!HS", "ラブライブ！蓮ノ空女学院スクールアイドルクラブ"),
    ("PL!SP", "ラブライブ！スーパースター!!"),   # <-- must precede PL!S
    ("PL!S",  "ラブライブ！サンシャイン!!"),
]
```

`PL!SP` **starts with** `PL!S`. Checking `PL!S` first mislabels all 551 Superstar
cards as Sunshine. The scraper got this wrong on the first pass and the diff caught
it immediately — which is the argument for diffing instead of eyeballing.

### 4.3 Icon vocabulary

`text` arrives as plain Japanese with `[...]`/`【...】` tokens. They become
`{{file.png|label}}`. Labels are **not** the file stem: `heart_00.png` is labelled
`heart0`, and `icon_all.png` is labelled `ハート`. Both forms are already in the DB
and the parser matches on the whole string, so a "tidier" label breaks extraction.

Tokens that stay **literal** (do not icon-ise): `[桃ブレード]`, `[赤ブレード]`,
`[黄ブレード]`, `[緑ブレード]`, `[青ブレード]`, `[紫ブレード]` — 3 existing cards keep
them verbatim.

---

## 5. Regenerate everything

Order matters. Each step consumes the previous one's output.

```bash
# 1. card blob + cards_gen.rs
python cards/compile_cards.py

# 2. abilities.json + bytecode + BOTH generated Rust decoders (one command)
python cards/ability_extraction/extract_card_abilities.py

# 3. coverage inventory (docs/ABILITY_MATRIX.md, engine/tests/TEST_*.{md,json})
python cards/test_inventory.py

# 4. engine suite
cd engine && cargo test --test run_all
```

Step 2 is the single regeneration entry point. It extracts, compiles the bytecode,
rewrites `engine/src/ability/abilities_gen.rs`, regenerates
`effect_decoder_gen.rs` / `condition_decoder_gen.rs` from `core/card.rs`, and
validates. Do **not** hand-edit `condition_decoder_gen.rs` / `effect_decoder_gen.rs`
— they are generated; edit `core/card.rs` or the generator.

PBLL02 result:

```
Cards: 2639   Blob: 631712 bytes   compressed: 84180 bytes
Unique abilities: 985   (was 936)
Bytecode: 132622 -> 98045 bytes (26.1% smaller)
compressed: 30295 bytes
card->ability pairs: 2116
```

### Verifying a parser change was output-neutral

```bash
python cards/ability_extraction/extract_card_abilities.py
git diff --numstat cards/abilities.json   # must show only generated_at / engine_commit
```

This is the only reliable check that a refactor changed nothing. No test caught the
`action_047` incident recorded in `cards/ability_extraction/PARSER_NOTES.md` — the
byte-diff did.

---

## 6. Read the two reports, do not skip them

`extract_card_abilities.py` prints a **MISSING MECHANICS ANALYSIS** and a
**validation** block. For PBLL02 it found 4 rule mismatches + 1 validation issue on
first run. These are the actual work items:

```
Total abilities checked: 985
Total mismatches found: 4
>> distinct_name   PL!-pb2-018-P＋   distinct card names required but no distinct field
>> card_identity   PL!-pb2-041-L   card treated as N but no set_card_identity
>> all_blade       PL!-pb2-001-PP   ALL blade / any-colour handling but no all_blade_timing
>> or_location     PL!-pb2-038-L   OR location pattern but fewer than 2 locations
  GROUP FILTER ISSUES: PL!-pb2-042-L  missing group_names ('西木野真姫')
```

Plus a parser `WARNING: Action 'modify_score' missing required field 'value'` on
`PL!-pb2-012-P＋` / `-R`.

Do not convert these to `#[ignore]`, delete them, or weaken assertions. Per
`AGENTS.md`: a gap is **fixed end-to-end** — add the parser pattern, regenerate, add
the engine branch, make the test assert the card's full printed behaviour.

---

## 7. CI

`.github/workflows/engine-tests.yml` runs `cargo test --test run_all --locked` and
triggers on any change to `cards/cards.json`, `cards/abilities.json`, the
`ability_extraction/` tree, the compilers, or `generation_manifest.json`.
`cards/test_inventory.py --check` is a separate inventory gate.

A pack import that leaves the suite red will fail CI on push.

---

## 8. Commit contents

Commit these together — the generated artifacts and the source data must land in the
same commit or the tree is broken for everyone else:

```
cards/cards.json
cards/abilities.json
cards/build/cards.bin.z
cards/build/abilities.bin.z
cards/build/generation_manifest.json
engine/src/ability/abilities_gen.rs
engine/tests/TEST_COVERAGE.md
engine/tests/TEST_QUALITY.md
engine/tests/TEST_INVENTORY.{md,json}
docs/ABILITY_MATRIX.md
cards/images/<EXP>/*.png
web_ui/img/cards_webp/*.webp
cards/scrape_expansion.py          (first time only)
```

Not committed: `engine/src/core/cards_gen.rs` and `cards/build/cards_gen.rs` are
gitignored (`.gitignore:192`, `.gitignore:113`) and rebuilt by
`compile_cards.py`.

Reference commit for the previous pack: `d84f06fb` (BP07 + NSD02, 233 cards).

---

## 9. Quick checklist

- [ ] Expansion code confirmed from the URL; card-number prefix **not** assumed
- [ ] `PWOCG_CFG` re-read; the two old endpoints confirmed dead against a *known* set
- [ ] `scrape_expansion.py <CODE> --dry-run` — counts and product name sane
- [ ] `cards.json` written; total = old + new; UTF-8 intact
- [ ] Images: PNG count == WebP count == new card count
- [ ] `compile_cards.py` → `extract_card_abilities.py` → `test_inventory.py`
- [ ] MISSING MECHANICS report read; every entry assigned work
- [ ] `cargo test --test run_all` green, or each failure traced to a named mechanic
- [ ] Parser changes proved output-neutral by `git diff --numstat cards/abilities.json`
- [ ] Generated artifacts committed alongside `cards.json`
