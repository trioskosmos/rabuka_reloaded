# GBA Deck Builder / Save System Plan

## Overview
Implement a deck builder for GBA that allows players to create custom decks saved to SRAM (`.sav` file), since GBA lacks the 3DS's QR code reader.

## Key Constraints
- **SAV format** (`engine/src/game/sav.rs`): 8 decks max, 72 cards/deck, 24-char card numbers, 32-char names
- **GBA hardware**: No QR reader, 240×160 screen (30×20 tiles), 4bpp text BG + OBJ sprites
- **Current flow**: `DeckSelectP1` → `DeckSelectP2` → `Match` (baked decks only)
- **Card DB**: ~3000+ cards in `cards.json`, baked per-deck blobs in `engine/baked/decks/`

## Input Optimization Strategy

### 4-Field Navigation (Left/Right = field, Up/Down = value)

| Field | Values | Notes |
|-------|--------|-------|
| 1. Series | 7 groups (μ's, Aqours, 虹ヶ咲, Liella!, 蓮ノ空, etc.) | Maps to `Card.group` |
| 2. Rarity | N, N＋, R, R＋, SR, SR＋, SEC, P, P＋, PR, PR＋, L, etc. | From card_no suffix |
| 3. Card Index | 1..N within (series, rarity) | Filtered list |
| 4. Quantity | 1..4 (deck limit) | Per-card copy count |
| 5. Deck Name | Text input (32 chars max) | For SAV entry name |

**Navigation flow:**
```
[Deck Name] ←→ [Series] ←→ [Rarity] ←→ [Card] ←→ [Qty]
     ↑                                                ↓
     ←──────── A: confirm / Done ────────────────────→
```

### Screen Layout (30 cols × 20 rows)

```
┌────────────────────────────────┐
│ DECK BUILDER  [0/72]  A:Done   │  ← Header: deck name, count, hint
├────────────────────────────────┤
│ Name:    > My Deck       <     │  ← Field 1: Deck name (editable)
│ Series:    μ's                │  ← Field 2: Series filter
│ Rarity:    R＋                │  ← Field 3: Rarity filter
│ Card:      PL!-BP1-001-R      │  ← Field 4: Card (shows name)
│ Qty:       [3]                │  ← Field 5: Quantity
├────────────────────────────────┤
│ Last picked:                   │  ← Recent picks (fits ~6 cards)
│  1. PL!-BP1-001-R x3  高坂穂乃果│
│  2. PL!-BP1-002-R x2  絢瀬絵里 │
│  3. LL-BP2-001-R＋  南ことり  │
├────────────────────────────────┤
│ L:Detail  R:CardArt  Sel:Zone  │  ← Hint bar
└────────────────────────────────┘
```

## Key Features

1. **Pre-baked non-energy cards**: Filter `CardType != Energy` at build time (like `sav.rs:34-35`)
2. **Zone viewer pattern**: Reuse `overlay.rs:83-202` `show_zone_grid` for card grid (5×1 stage-size cards, pagination)
3. **Detail on A**: Press A on field 4 (Card) → shows `show_card_detail` (art + stats + ability), press A again to confirm
4. **Recent picks**: Show last 6-8 cards added with quantities (scrollable if more)
5. **Start button**: Opens StartMenu (Game Log, ZoneGrid, Close) - already implemented
6. **Select button**: Toggle to ZoneGrid view for browsing by zone
7. **L/R**: Detail viewer / Card art preview (existing pattern)

## Minimizing Inputs

| Technique | Savings |
|-----------|---------|
| Series → Rarity → Card filtering | Avoids scrolling 3000+ cards |
| Quantities default to 1, max 4 | 1-2 presses per card |
| Recent picks list | Re-add common cards in 2 presses |
| L/R detail preview | Verify before committing |
| ZoneGrid (Select) | Browse by waitroom/stage/success |
| A on field = confirm, A on card = detail | Single-button dual-purpose |

## Deck Name Input

Since GBA has no keyboard, use **character picker**:
- Left/Right: Move cursor in name field
- Up/Down: Cycle character (A-Z, 0-9, symbols, kana)
- A: Confirm character, move to next position
- B: Delete/backspace
- Max 31 chars + NUL (per `SAV_NAME_LEN = 32`)

Character set: `ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-+!_ あいうえおかきくけこ...`

## Implementation Files

### New Files
1. `platforms/gba/src/deck_builder.rs` - Core state machine
2. `platforms/gba/src/deck_builder_ui.rs` - Rendering (reuses `Display`, `Board`, `menu`)

### Modified Files
1. `platforms/gba/src/screens.rs` - Add `DeckBuilder` screen enum
2. `platforms/gba/src/bin/rabuka_gba.rs` - Insert builder between `ModeSelect` and `DeckSelect`
3. `tools/bake_deck_cards.py` - Add "custom" slot for SRAM decks
4. `engine/src/game/sav.rs` - Already has `encode_sav`/`decode_sav` for SRAM

## SRAM Integration

- On "Done" (A at 72 cards or early with ≥1 card): `encode_sav` → write to SRAM via GBA flash
- On boot: `decode_sav` → populate custom deck slots in `DECKS` array
- Fallback: If SRAM empty/corrupt, only baked decks available
- Max 8 custom decks in SRAM (matches `MAX_SAV_DECKS = 8`)

## Boot Flow Update

```
ModeSelect
    ↓ (A/Start on "Deck Builder")
DeckBuilder → builds deck → saves to SRAM → returns to ModeSelect
    ↓ (A/Start on "VS AI" / "2 Player" / etc.)
DeckSelectP1 (now includes SRAM decks)
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

### DeckBuilderState
```rust
struct DeckBuilderState {
    deck_name: String,           // 0-31 chars
    cards: Vec<(String, u8)>,    // (card_no, quantity)
    field: Field,                // Current field being edited
    series_idx: usize,           // Series filter index
    rarity_idx: usize,           // Rarity filter index
    card_idx: usize,             // Card index within filtered list
    quantity: u8,                // 1-4
    name_cursor: usize,          // Position in deck_name
    name_char_idx: usize,        // Character index for current position
    recent_picks: Vec<(String, u8)>, // Last 8 cards added
    filtered_cards: Vec<String>, // Card numbers matching filters
}
```

### Field Enum
```rust
enum Field {
    DeckName,   // Character picker mode
    Series,
    Rarity,
    Card,
    Quantity,
}
```

## Filtering Logic

1. **Series filter**: Group by `Card.group` (μ's, Aqours, 虹ヶ咲, Liella!, 蓮ノ空, PR, etc.)
2. **Rarity filter**: Parse from card_no suffix (R, R＋, SR, SEC, P, P＋, PR, L, etc.)
3. **Card list**: All non-energy cards matching both filters, sorted by card_no
4. **Dynamic update**: Changing series/rarity resets card_idx to 0, rebuilds filtered list

## Character Picker for Deck Name

```
Position: 0  1  2  3  4  5  ...
Name:     [M][y][ ][D][e][c]...
Cursor:            ^
Char:      'e' (index 4 in charset)
```

Charset: `ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-+!_ あいうえおかきくけこさしすせそたちつてとなんいはひふへほまみむめもやゆよらりるれろわをんアイウエオカキクケコサシスセソタチツテトナニハヒフヘホマミムメモヤユヨラリルレロワヲン`

Total: ~80 characters, cycle with Up/Down.

## Testing Checklist

- [ ] Deck builder opens from ModeSelect
- [ ] All 4+1 fields navigable with L/R
- [ ] Series/Rarity filters work correctly
- [ ] Card list shows correct filtered cards
- [ ] Quantity adjustment works (1-4)
- [ ] Recent picks show last 8 additions
- [ ] Deck name character picker works
- [ ] L/R show detail/art preview
- [ ] Select opens ZoneGrid
- [ ] Start opens StartMenu
- [ ] Done (A at 72 cards or ≥1 card) saves to SRAM
- [ ] SRAM decks appear in DeckSelect
- [ ] SRAM persists across reboots (emulator)
- [ ] Empty/invalid SRAM falls back gracefully
- [ ] Legality checker validates deck on save
- [ ] Illegal decks show specific violation messages
- [ ] Legal indicator shown in deck list (✓/✗)

## Legality Checkers

### Deck Validation Rules

The deck builder must enforce these rules before allowing save (matching official tournament rules):

| Rule | Description | Check Timing |
|------|-------------|--------------|
| **Min cards** | Deck must have ≥40 main-deck cards (member + live) | On save |
| **Max cards** | Deck must have ≤72 main-deck cards (SAV limit) | On save |
| **Copy limit** | Max 4 copies of same card_no (by card number) | On add |
| **Energy cards** | Energy cards auto-added by engine, not in deck | N/A (excluded) |
| **Live card limit** | Max 4 live cards in live card zone | On save |
| **Series legality** | All cards must be from legal series (format-dependent) | On save |
| **Banned cards** | No banned/restricted cards (if banlist exists) | On save |

### Implementation

```rust
enum LegalityError {
    TooFewCards { current: usize, minimum: usize },
    TooManyCards { current: usize, maximum: usize },
    TooManyCopies { card_no: String, count: u8 },
    TooManyLiveCards { count: u8 },
    IllegalSeries { card_no: String, series: String },
    BannedCard { card_no: String },
    RestrictedCard { card_no: String, max_allowed: u8 },
}

fn validate_deck(cards: &[(String, u8)], all_cards: &[CardEntry]) -> Result<(), LegalityError> {
    let total: usize = cards.iter().map(|(_, q)| *q as usize).sum();
    
    // Min/max cards
    if total < 40 { return Err(LegalityError::TooFewCards { current: total, minimum: 40 }); }
    if total > 72 { return Err(LegalityError::TooManyCards { current: total, maximum: 72 }); }
    
    // Copy limit
    for (no, qty) in cards {
        if *qty > 4 { return Err(LegalityError::TooManyCopies { card_no: no.clone(), count: *qty }); }
    }
    
    // Live card count
    let live_count: u8 = cards.iter()
        .filter_map(|(no, q)| all_cards.iter().find(|c| c.card_no == *no))
        .filter(|c| c.card_type == CardType::Live)
        .map(|c| c.1)
        .sum();
    if live_count > 4 { return Err(LegalityError::TooManyLiveCards { count: live_count }); }
    
    // Series legality (format: standard = last 2 years, expanded = all)
    // For now: all series legal
    // Future: check against format banlist
    
    Ok(())
}
```

### UI Integration

1. **Real-time indicator**: Show deck legality status in header
   - `DECK BUILDER [42/72] ✓ Legal` or `✗ Illegal: Too few cards (40 min)`
   
2. **Save blocked**: A on Quantity field with illegal deck shows error detail screen instead of saving

3. **Detail on violation**: Press L on error message to see full explanation

4. **Auto-fix hints**: Show which cards need to be added/removed

### Banlist Support (Future)

```rust
// Loaded from external file or baked into ROM
static BANLIST: &[(&str, BanType)] = &[
    ("PL!-BP1-001-R", BanType::Banned),
    ("LL-BP2-001-R＋", BanType::Restricted(1)),
];

enum BanType {
    Banned,
    Restricted(u8), // max copies allowed
}
```

### GBA-Specific Considerations

- **ROM space**: Banlist baked into ROM as static array (minimal space)
- **No network**: Can't download updated banlist; requires ROM update
- **Format selection**: Could add "Standard/Expanded" toggle in ModeSelect
- **SRAM decks**: Validate on load too (in case of corruption or rule changes)