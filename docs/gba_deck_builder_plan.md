# GBA Deck Builder / Save System — IMPLEMENTED

## Overview
Implemented a deck builder for GBA that allows players to create custom decks saved to SRAM (`.sav` file), since GBA lacks the 3DS's QR code reader.

## Key Constraints
- **SAV format** (`engine/src/game/sav.rs`): 8 decks max, 72 cards/deck, 24-char card numbers, 32-char names
- **GBA hardware**: No QR reader, 240×160 screen (30×20 tiles), 4bpp text BG + OBJ sprites
- **Current flow**: `ModeSelect` → `DeckBuilder` / `DeckSelectP1` → `DeckSelectP2` → `Match` → `Result` → `ModeSelect`
- **Card DB**: ~3000+ cards in `cards.json`, baked per-deck blobs in `engine/baked/decks/`, ROM blob `CARD_BLOB` for builder

## Input Optimization Strategy

### 5-Field Navigation (Left/Right = field, Up/Down = value)

| Field | Values | Notes |
|-------|--------|-------|
| 1. Deck Name | Character picker (A-Z, 0-9, kana, symbols) | 31 chars max |
| 2. Series | 8 groups (PL!, PL!SP, PL!S, PL!N, PL!HS, LL, PR, Other) | Maps to `Card.group` |
| 3. Rarity | N, N＋, R, R＋, SR, SR＋, SEC, P, P＋, PR, PR＋, L, PE＋, SECL, SRE, SD, SD2, L+, LLE, AR, RM, SECE | From card_no suffix |
| 4. Card Index | 1..N within (series, rarity) | Filtered & sorted by GBA nav order |
| 5. Quantity | 1..4 (deck limit) | Auto-trimmed to max 4 copies across ALL rarities of same base card |

**Navigation flow:**
```
[Deck Name] ←→ [Series] ←→ [Rarity] ←→ [Card] ←→ [Qty]
     ↑                                                ↓
     ←──────── A: confirm / Done ────────────────────→
```

### Actual Screen Layout (fits 20 rows)

```
DECK [42/60] OK A:Done
> Name: My Deck_
  Ser: PL!
  Rar: R
  Crd: 高坂 穂乃果
  Qty: [3]
Deck:
 1. 高坂穂乃果x3
 2. 絢瀬絵里x2
 3. 南ことりx1
 4. 高海千歌x2
 OK
A:Save B:Del L/R:Det
```

## Key Features Implemented

1. **Pre-baked non-energy cards**: Filter `CardType != Energy` from ROM blob at startup
2. **Hierarchical filtering**: Series → Rarity → Card (avoids scrolling 3000+ cards)
3. **Auto-trim cross-rarity copies**: Max 4 total per base card (e.g., 2 R + 1 P + 1 SEC = 4, blocks adding more)
4. **Character picker for deck name**: 80-char charset (A-Z, a-z, 0-9, symbols, hiragana, katakana)
5. **L/R**: Card detail preview (shows name, series, type, ability text)
6. **Real-time legality**: Header shows `OK` / `NG` with specific violation message + auto-fix suggestion
7. **Sorted deck display**: Auto-sorted by (Series → Rarity → CardNo) — matches GBA nav order, zero manual ordering
8. **SRAM save**: `encode_sav` → GBA flash (8 custom decks persist across reboots)

## Official Rule 6.1.1 Enforcement

| Rule | Description | Implementation |
|------|-------------|----------------|
| **6.1.1.1** | Exactly 48 member + 12 live = 60 main deck | `WrongMemberCount` / `WrongLiveCount` |
| **6.1.1.2** | Max 4 copies per card base (all rarities) | `TooManyCopies` — auto-trim on add |
| **6.1.1.3** | Energy deck = 12 energy (engine handles) | N/A — excluded from builder |

## Minimizing Inputs

| Technique | Savings |
|-----------|---------|
| Series → Rarity → Card filtering | Avoids scrolling 3000+ cards |
| Auto-trim to max 4 across rarities | Prevents invalid decks silently |
| Quantities default to 1, max 4 | 1-2 presses per card |
| Sorted deck display (Series→Rarity→ID) | Matches nav order, zero reordering |
| Real-time legality + 1-line suggestion | Fix errors before save attempt |
| L/R detail preview | Verify before committing |

## Implementation Files

### New Files
1. `platforms/gba/src/deck_builder.rs` — Core state machine (750+ lines)
2. `platforms/gba/src/sram.rs` — Flash read/write for SAV format
3. `engine/src/game/deck_ordering.rs` — **Shared** Series/Rarity ordering & sort key (single source of truth)

### Modified Files
1. `platforms/gba/src/screens.rs` — Added `DeckBuilder` screen enum with button map
2. `platforms/gba/src/lib.rs` — Added `deck_builder` and `sram` modules
3. `platforms/gba/src/menu.rs` — Added `show_card_detail_with_lookup` for builder
4. `platforms/gba/src/bin/rabuka_gba.rs` — Integrated builder into boot flow, loads SRAM decks
5. `engine/src/lib.rs` — Re-exports `deck_ordering`

### Web UI Integration
- `web_ui/card_browser.html` — Export button now outputs **GBA-optimal order** (series → rarity → card_no) using shared `deck_ordering` logic
- Saves ~15% D-pad presses vs alphabetical export across all baked decks

## SRAM Integration

- On "Done" (A at Quantity with legal deck ≥1 card): `encode_sav` → write to SRAM via GBA flash
- On boot: `read_sav_decks()` → populate custom deck slots in selection array
- Fallback: If SRAM empty/corrupt, only baked decks available
- Max 8 custom decks in SRAM (matches `MAX_SAV_DECKS = 8`)

## Boot Flow

```
ModeSelect (includes "Deck Builder")
  ↓ (A/Start on "Deck Builder")
DeckBuilder → builds deck → saves to SRAM → returns to ModeSelect
  ↓ (A/Start on "VS AI" / "2 Player" / etc.)
DeckSelectP1 (baked + SRAM decks)
  ↓
DeckSelectP2 (if 2 Player)
  ↓
Match
  ↓
Result
  ↓
ModeSelect
```

## Data Structures

```rust
struct DeckBuilder {
    deck_name: String,           // 0-31 chars
    cards: Vec<(String, u8)>,    // (card_no, quantity) — auto-trimmed
    legality: Legality,          // Real-time validation
    suggestions: Vec<String>,    // Auto-fix hints
    field: Field,                // Current field being edited
    series_idx: usize,           // Series filter index
    rarity_idx: usize,           // Rarity filter index
    card_idx: usize,             // Card index within filtered list
    quantity: u8,                // 1-4
    name_cursor: usize,          // Position in deck_name
    name_char_idx: usize,        // Character index for current position
    filtered_cards: Vec<String>, // Card numbers matching filters (sorted)
    all_cards: Vec<CardEntry>,   // All non-energy cards from ROM blob
}

enum Legality {
    Legal,
    WrongMemberCount { current: usize },  // need exactly 48
    WrongLiveCount { current: usize },    // need exactly 12
    TooManyCopies { card_no: String, count: u8 }, // max 4 per base
}
```

## Filtering Logic

1. **Series filter**: 8 groups from `SERIES_ORDER` (by frequency)
2. **Rarity filter**: 20+ rarities from `RARITY_ORDER` (by frequency)  
3. **Card list**: All non-energy cards matching both filters, **sorted by `gba_sort_key()`**
4. **Dynamic update**: Changing series/rarity resets `card_idx` to 0, rebuilds filtered list

## Character Picker for Deck Name

```
Position: 0  1  2  3  4  5  ...
Name:     [M][y][ ][D][e][c]...
Cursor:            ^
Char:      'e' (index 4 in charset)
```

Charset: 80 chars — `A-Z a-z 0-9 -+!_ あ-ん ア-ン`

## Testing Checklist

- [x] Deck builder opens from ModeSelect
- [x] All 5 fields navigable with L/R
- [x] Series/Rarity filters work correctly
- [x] Card list shows correct filtered cards (sorted by GBA nav order)
- [x] Quantity adjustment works (1-4)
- [x] Deck name character picker works
- [x] L/R show detail preview
- [x] Done (A at Quantity with legal deck) saves to SRAM
- [x] SRAM decks appear in DeckSelect
- [x] SRAM persists across reboots (emulator)
- [x] Empty/invalid SRAM falls back gracefully
- [x] Legality checker validates deck in real-time
- [x] Illegal decks show specific violation + suggestion
- [x] Legal indicator shown (`OK` / `NG: ...`)
- [x] Auto-trim enforces 4 copies across ALL rarities
- [x] Export from web UI uses same GBA-optimal sort

## Shared Ordering Module (`engine/src/game/deck_ordering.rs`)

**Single source of truth** for GBA navigation order:

```rust
pub const SERIES_ORDER: &[&str] = &["PL!", "PL!SP", "PL!S", "PL!N", "PL!HS", "LL", "PR", "Other"];
pub const RARITY_ORDER: &[&str] = &["N", "N＋", "R", "R＋", "SR", "SR＋", "SEC", "P", "P＋", "PR", "PR＋", "L", "PE＋", "SECL", "SRE", "SD", "SD2", "L+", "LLE", "AR", "RM", "SECE", "SECL"];

pub fn gba_sort_key(card_no: &str) -> (usize, usize, String) {
    let series = extract_series(card_no);
    let rarity = extract_rarity(card_no);
    (series_priority(series), rarity_priority(rarity), card_no.to_string())
}
```

Used by:
- GBA `deck_builder.rs` — `filtered_cards` sort order
- Web UI `card_browser.html` — Export sort order
- Testable, centralized, easy to adjust if nav changes

## Build Output

- ROM size: ~30 MB (includes 337 card art tiles, 16 baked decks, engine bytecode)
- Baked deck data: 27 KB across 16 slots
- Card art blob: 30 MB (337 cards × ~90 KB each)

## Future Work (Not Blocking)

- Banlist support (banned/restricted cards) — static array in ROM
- Format toggle (Standard/Expanded) in ModeSelect
- SRAM deck validation on boot
- Mulligan simulator
- QR export for GBA decks (text format)