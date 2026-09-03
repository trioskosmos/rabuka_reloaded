# GBA Display Refactor: Sprite-Based Card Rendering

## Problem Summary

The current implementation uses 7 `RegularBackground` layers (4bpp + 8bpp pairs) for card rendering:
- `board_art_bg`/`board_ui_bg` - board view (hand, stage, live)
- `menu_bg`/`menu_art_bg` - choice menus
- `detail_art_bg`/`detail_text_bg` - card detail view
- `action_bg` - actions screen

**Failures:**
1. **VRAM leak**: 4200 `set_tile` calls/frame just to clear → agb allocator never frees tiles
2. **Ghost cards**: Clearing with `TileSetting::new(0, ...)` draws card-back tile 0 instead of transparency
3. **Transition OOM**: Detail (216 tiles) + Board (150 tiles) + Menu (150 tiles) coexist briefly → "ran out of video RAM"
4. **Wrong abstraction**: Backgrounds are for static tilemaps, not dynamic card sprites

## Solution: Sprite-Based Architecture

### Hardware Reality (GBA)
- **128 sprites max**, each 8×8 to 64×64 pixels
- **OBJ VRAM**: 16 KB (separate from BG VRAM)
- **OAM**: 128 entries, updated per frame
- **No manual clearing needed** - sprites not shown = not rendered
- **agb manages VRAM** via `SpriteVram` + reference counting

### New Architecture

| Layer | Type | Purpose | Management |
|-------|------|---------|------------|
| Text/UI | `RegularBackground` (4bpp) | Headers, action bar, zone fills, badges, cursors | Static, cleared per frame |
| Card Art | `Object` (sprites, 8bpp) | All card fronts/backs | Dynamic, created on demand, auto-freed |

### Sprite Mapping

| Card Type | Pixel Size | Sprite Size | Sprites Needed |
|-----------|------------|-------------|----------------|
| Stage (40×48) | 34×48 card + padding | 32×32 + 8×16 | 2 |
| Hand (24×32) | 24×32 | 32×32 | 1 |
| Live (24×16) | 24×16 | 32×16 | 1 |
| Detail (96×144) | 96×144 | 64×64 + 32×64 + 64×32 + 32×32 | 4 |
| Menu choice (24×32) | 24×32 | 32×32 | 1 |

**Max concurrent**: 10 hand + 6 stage + 4 live + 1 detail = 21 sprites << 128 limit

### Palette Strategy

- **Single `PaletteVramMulti`** (256 colors) for ALL card sprites
- Load `MASTER_PAL` once at startup → `PaletteMulti` → `PaletteVramMulti::new()`
- All `Object::set_palette(&palette_vram_multi)` share it
- Bank 15 reserved for 4bpp text UI (`TEXT_PALETTE`)

### Sprite Lifecycle

```rust
// On-demand creation, cached by card_no
struct CardSpriteCache {
    sprites: HashMap<String, Vec<SpriteVram>>,  // key: "card_no:size"
    palette: PaletteVramMulti,
}

// Frame rendering
fn render_board(&mut self, frame: &BoardFrame) {
    // 1. Clear text BG (4bpp) - cheap, 1 tile repeated
    // 2. Create/update Objects for visible cards
    // 3. Show objects in priority order (hand above stage, cursor top)
    // 4. Frame commit - agb handles OAM upload
}

// No manual clearing! Objects not shown = not in OAM = invisible
```

### Data Structures

```rust
struct CardSprite {
    objects: Vec<Object>,           // 1-4 sprites per card
    size: SpriteSize,               // Stage/Hand/Live/Detail
    position: (i32, i32),           // screen pixels
    flipped: bool,                  // opponent cards
    priority: Priority,             // P0/P1 layering
}

struct Display {
    gfx: Graphics,
    text_bg: RegularBackground,     // single 4bpp BG for all text/UI
    card_sprites: HashMap<String, CardSprite>,  // active this frame
    sprite_cache: CardSpriteCache,  // reusable SpriteVram
    master_palette: PaletteVramMulti,
    // ... text rendering state
}
```

### Rendering Flow

**Board Frame:**
1. Clear text BG (UI_EMPTY tile)
2. Render text (header, action bar, cursors) to text BG
3. For each card slot:
   - Get/create `CardSprite` from cache
   - Set position, flip, priority
   - `object.show(&mut frame)`
4. `text_bg.show(&mut frame)` (under cards) or over (badges)
5. `frame.commit()`

**Menu/Detail:** Same pattern, different layout coordinates.

### VRAM Budget

| Resource | Size |
|----------|------|
| Text BG (4bpp, 32×32) | ~2 KB tiles + 2 KB map |
| Master palette (256 color) | 512 bytes |
| Card sprites (max 21 × 32×32 8bpp) | ~21 KB |
| **Total** | **~25 KB** << 96 KB VRAM |

### Migration Plan

1. **Add sprite imports** - `agb::display::object::{Object, DynamicSprite256, PaletteVramMulti, Size, Priority}`
2. **Create `CardSpriteCache`** with `MASTER_PAL` → `PaletteVramMulti`
3. **Replace 7 backgrounds** with 1 text BG + sprite cache
4. **Rewrite `draw_slot`** to create/position `Object`s instead of `set_tile`
5. **Rewrite `render_card_detail`** to use 4 sprites for 96×144 portrait
6. **Rewrite `swap_buffers`** menu cards as sprites
7. **Remove all clear loops** - sprites auto-hidden when not shown
8. **Remove `reset_vram`** - no longer needed

### Risk Mitigation

- **Sprite limit**: 21 max vs 128 available - safe
- **agb allocator**: `DynamicSprite256::new(Size::S32x32)` allocates from IWRAM, `.to_vram(palette)` uploads to VRAM
- **Priority**: `Object::set_priority(Priority::P1)` for cards, `P0` for text BG = correct layering
- **Flipping**: `Object::set_hflip(true)` + `set_vflip(true)` for 180° rotation (opponent)
- **Transparency**: Sprite pixel 0 = transparent by default in 8bpp mode

## Success Criteria

- [ ] Zero `set_tile` calls for card art
- [ ] Zero manual clear loops for 8bpp layers
- [ ] Zero VRAM OOM on any screen transition
- [ ] < 50 sprites/frame typical
- [ ] Build passes, ROM runs