# Rabuka 3DS — Visual Design Plan

## Screen Specs

| Screen | Resolution | Colors | Used for |
|--------|-----------|--------|----------|
| Top (upper) | 400×240 | RGB565 | Game board (stage, live zone, energy, hand) |
| Bottom (lower) | 320×240 | RGB565 | Actions list + rule log |

Rendering: citro3d GPU with citro2d text/font (Noto Sans JP BCFNT).

---

## Layout — Top Screen (400×240)

The top screen shows the game board in 4 fixed zones stacked vertically:

```
┌────────────────────────────────────────────┐
│ HEADER BAR                 ┌─────┐ ┌─────┐ │  h=30
│ Turn 3 · MAIN · P1(AP)    │P1 H│ │P2 H│ │
│                            │E 5│ │E 3│ │
│                            └─────┘ └─────┘ │
├────────────────────────────────────────────┤
│ LIVE ZONE  [card] [card] [card]  (landsc.) │  h=50
├────────────────────────────────────────────┤
│ STAGE          │ UTILITY                   │
│ ┌──┐ ┌──┐ ┌──┐│ Main: 12  E: 5           │  h=55
│ │L │ │C │ │R ││ Wait: 3  Suc: 2          │
│ │  │ │  │ │  ││ M:10 L:6 P:<=9           │
│ └──┘ └──┘ └──┘│                            │
├────────────────────────────────────────────┤
│ ENERGY   [▪][▪][▪] [▫][▫]  3/5 active    │  h=25
├────────────────────────────────────────────┤
│ HAND  [card][card][card][card][card] ...   │  h=80
│       (horizontally scrollable)            │
└────────────────────────────────────────────┘
```

### Zone Details

**Header (30px)**
- Left: Turn number, phase name, active player indicator
- Right: Compact stats — Hand count, Energy count for both players
- Color: Dark nav bar with gold text for key values

**Live Zone (50px)**
- 3 landscape card slots side by side
- Each slot: shows card_no + performance badge if performed
- Background: slightly different shade to distinguish from stage

**Stage + Utility (55px)**
- Stage: 3 portrait card slots (L/C/R)
- Under each: small energy/under-card counter
- Utility column (right ~80px): deck counts, energy deck count, discard count, deck composition (M:L:P)
- Valid-target cards get a gold border indicator

**Energy Bar (25px)**
- Thin strip with energy pips
- Active energy = filled pip (▪), wait energy = unfilled (▫)
- Count text on right

**Hand (80px)**
- Row of card slots horizontally scrollable (D-pad left/right)
- Each slot shows card_no + name
- Selected card highlighted with `>` marker
- Cards with valid play actions shown in gold
- Inert cards (no actions) shown dimmed

### Color Palette (adapted from web UI)

| Role | RGB565 Hex | Usage |
|------|-----------|-------|
| BG primary | #0f141e | Screen background |
| BG zone | #1a2332 | Zone backgrounds |
| Text primary | #ffffff | Main text |
| Text dim | #a0aec0 | Secondary info |
| Gold | #f59e0b | Headers, play actions, highlights |
| Blue | #4a9eff | Stage members, interactive |
| Green | #2ecc71 | Success, pass, valid actions |
| Pink | #ff55aa | Live cards, selection |
| Purple | #9b59b6 | Ability actions |
| Red | #ef4444 | Danger, errors |
| Border | rgba(255,255,255,0.1) | Zone separators |

---

## Layout — Bottom Screen (320×240)

The bottom screen splits into two stacked sections:

```
┌────────────────────────────────┐
│ ACTIONS                   [tab]│  header h=20
├────────────────────────────────┤
│ ┌──────────────────────────┐   │
│ │[▲ 3 more above]          │   │
│ │>[00] Confirm mulligan    │   │
│ │ [01] Select card 1       │   │
│ │ [02] Select card 2       │   │
│ │ [03] Select card 3       │   │
│ │ [04] Select card 4       │   │
│ │[▼ 2 more below]          │   │
│ └──────────────────────────┘   │  actions area h=170
├────────────────────────────────┤
│ LOG line 1                     │
│ LOG line 2                     │  log area h=50
│ LOG line 3 (truncated)         │
└────────────────────────────────┘
```

### Actions Section
- Scrollable list of available actions (D-pad up/down)
- Each item: `>[index] action_description`
- Selected action highlighted with `>` and gold text
- Scroll indicators: `▲ N more above` / `▼ N more below`
- Action types color-coded:
  - Confirm/Pass: Green
  - Play member: Gold
  - Use ability: Purple
  - Select/Deselect: Blue
  - Cancel/Skip: Dim

### Log Section
- Last 2-3 log lines shown at the bottom
- Scrollable with L/R shoulder buttons to expand
- Color-coded by event type (PLAY=green, ACTIVATE=gold, TRIGGER=red, etc.)
- Shows most recent events

---

## 3DS Controls

| Button | Function |
|--------|----------|
| D-Pad Up/Down | Navigate actions list |
| D-Pad Left/Right | Scroll hand / navigate card selection |
| A | Select / Confirm |
| B | Go back / Cancel |
| X | Toggle action list ↔ log view |
| Y | Refresh display |
| L / R | Page up/down in actions |
| START | Exit to homebrew launcher |

---

## Phases & What to Show

### Mulligan Phase (top screen)
- Show hand cards with mulligan selection state
- Selected cards for mulligan highlighted in pink
- Confirm mulligan always first action on bottom screen

### Main Phase (top screen)
- Full board: stage, energy, hand, live zone
- Pass - End Main Phase always first action on bottom screen
- Play member actions grouped by card
- Use ability actions listed after play actions

### Live Card Set Phase (top screen)
- Hand visible, cards selectable for live set
- Live zone shows currently selected cards
- Confirm live card set always first action on bottom screen

### Performance Phase (top screen)
- Live zone cards shown with performance badges
- Auto-advance through performance resolution
- Results shown in log area

---

## Implementation Priority

### Phase 1 — Functional Text UI (DONE)
- Full text rendering with Noto Sans JP
- Scrollable action list with dynamic line count
- Header with game state info
- Basic keyboard navigation

### Phase 2 — Zone-based Board Layout (NEXT)
- Draw zone boxes with citro2d rectangles
- Card slots in each zone with card_no text
- Energy pips with filled/unfilled states
- Stage positions (L/C/R) with labels

### Phase 3 — Enhanced Actions (AFTER PHASE 2)
- Color-coded action types
- Card name preview in action list
- Scroll indicators with exact counts
- Cursor position indicator

### Phase 4 — Log & Feedback (LATER)
- Bottom screen log area
- Turn history accessible
- Ability resolution feedback
- Performance results display

### Phase 5 — Polish (FUTURE)
- Card images rendered from ROMFS (pre-converted webp)
- Animated card movements (slide transitions)
- Touch screen support for bottom screen actions
- Sound effects via DSP

---

## Technical Constraints

- **No per-card images yet**: Render cards as text (card_no + name)
- **No touch input yet**: D-pad + buttons only
- **No smooth scrolling**: Page-based viewport
- **No GPU-accelerated UI widgets**: C2D draws text + filled rectangles
- **Heap: 64MB**: Font takes ~25MB, game data ~15MB, leaves ~24MB for rendering
- **Framerate target**: 30fps minimum (60fps ideal for text-only)
- **Line height measured via C2D_TextGetDimensions**: Dynamic per-font calculation
