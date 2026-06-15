# Performance Modal Redesign & 余剰ハート Fix

## A. Performance Modal — Layout Changes

### 1. Single column per player
- `perf-panel-body-grid`: change from `grid-template-columns: 1.1fr 0.9fr` to `1fr`
- Merge left/right column content into one vertical flow

### 2. Remove `perf-story-grid`
- Delete `renderStoryCards()` call from `renderPlayerPanel()`
- Remove `.perf-story-grid`, `.perf-story-card`, `.perf-story-card h4`, `.perf-story-card p` CSS

### 3. Vertical order (top → bottom)
```
┌─ Summary (score hero — keep as-is) ────────────┐
├─ Stage Contributors (restructured)              │
│  ┌─ per-slot breakdown ────────────────┐        │
│  │  total hearts → base → ability → blades     │
│  └────────────────────────────────────┘        │
│  ┌─ Global/Unlinked bonuses (4th slot) ─┐     │
│  └───────────────────────────────────────┘     │
├─ Yell & Source Pool (restructured)             │
│  per-card vector + total + source breakdown    │
├─ Combined Total (new)                          │
│  stage + yell = grand total                    │
├─ Live Cards (restructured)                     │
│  required / filled / surplus / adjustments     │
├─ Effects and Score Lines (keep as-is)          │
└─────────────────────────────────────────────────┘
```

## B. Stage Section (replaces `renderContributionSection`)

Each stage slot card:
```
┌─ [CardName] — Left/Center/Right ────────────────┐
│  Total hearts:  ♥[3,0,2,0,0,0,0] = 5            │
│    Base hearts:      ♥[3,0,2,0,0,0,0]            │
│    Ability additions: ♥[0,0,0,0,0,0,0]           │
│      └─ per-ability breakdown from ability_heart_bonuses │
│    Blades: ★3 (+1 from abilities)                │
└─────────────────────────────────────────────────┘
```

4th slot "Global bonuses": shows `triggered_abilities` not tied to single member.

## C. Yell Section (replaces `renderYellSection`)

```
┌─ Yell Cards (N cards) ──────────────────────────┐
│  Total yell hearts: ♥[0,0,0,6,0,0,0]             │
│                                                   │
│  Per-card:                                        │
│  ┌── CardA ──┐ ┌── CardB ──┐                     │
│  │ ♥[0,0,0,1]│ │ ♥[0,0,0,1]│                     │
│  │ ♪1 ⎋0    │ │ ♪1 ⎋0    │                     │
│  └──────────┘ └──────────┘                        │
│                                                   │
│  Per-color source:                                │
│    Heart03: 6 (from 6 yell cards)                 │
└─────────────────────────────────────────────────┘
```

## D. Combined Total (new section)

```
┌─ Total Hearts Available ────────────────────────┐
│  Stage:   ♥[3,0,2,0,0,0,0] = 5                  │
│  Yell:    ♥[0,0,0,6,0,0,0] = 6                  │
│  ──────────────────────────────────              │
│  Total:   ♥[3,0,2,6,0,0,0] = 11                 │
└─────────────────────────────────────────────────┘
```

## E. Live Cards (restructured)

```
┌─ Live: CardName ────────────────────────────────┐
│  Required:     ♥[6,2,0,5,0,0,2]  = 15           │
│  Filled:       ♥[6,2,0,5,0,0,2]  = 15           │
│  Surplus:      ♥[6,0,0,0,0,0,0]  = 6            │
│  Adjustments:  -1 Heart03, +1 Heart01            │
│  Score: +3  →  PASS                              │
└─────────────────────────────────────────────────┘
```

### Heart allocation math
1. Stage members contribute `base_hearts + bonus_hearts` per slot
2. Yell cards contribute `blade_hearts` per card
3. Total available = stage + yell sum (per color)
4. For each live card: fill `required[color]` from available `hearts[color]`
5. Remaining colored deficits filled by Heart00 (wildcard)
6. If any deficit remains → live FAILS
7. Remaining hearts after all lives filled = 余剰ハート (surplus)

## F. Engine Fixes (余剰ハート)

### F1. Fix `spare` field in snapshot
- `types.rs:LiveCardResult.spare`: currently stores TOTAL stage hearts (not surplus)
- Fix: compute actual remaining after allocating to all live cards
- **New**: add `surplus_hearts: [u32; 7]` to `PerformanceSnapshot` for the calculated per-color excess

### F2. Fix condition evaluation for color-specific surplus
- `condition/card.rs:1555` (`resource_type == "surplus_heart"`):
  - Currently uses `total_hearts()` (sum of all colors)
  - Fix: when `condition.heart_colors` is set, calculate per-color surplus:
    `stage_hearts[color] - allocated_to_lives[color]`
  - Compare against `condition.count` and `operator`

### F3. Fix `no_excess_heart` flags
- `live.rs:170`: `set_opponent_live_success(true)` — hardcoded `true`
  - Fix: calculate actual surplus after allocation
- `self_no_excess_heart_this_turn`: never set to `true`
  - Fix: set after player's own live phase

### F4. Constant ability logging (postponed)
- `modifiers.rs`: add `active_constant_abilities: HashSet<String>` to `GameState`
- Track false→true transitions for constant ability conditions
- Push `"[Activated (constant)] CardName: ability_text"` log entry
- **Deferred**: not implementing in this pass

## G. Rule Log — Card thumbnail + ability text

- `LogRenderer.js.createStandaloneLogEntry()`:
  - Add `[Activated]` / `[Activated (constant)]` to entry-type classifier
  - `enrichLogEntryWithCard()` already runs for all entries — verifies `[Activated] CardName:` format
  - `appendFullAbility()` already runs — enriches text with `{{icon}}`→`<img>` conversion
- New CSS class: `.activated` / `.triggered` for visual distinction
- **Constant abilities**: not shown in log (per user, 常時 stays hidden for now)

## Files to modify

| File | What |
|------|------|
| `web_ui/js/components/PerformanceRenderer.js` | Restructure stage/yell/live/combined sections, remove story grid, single column |
| `web_ui/css/performance.css` | Remove story grid CSS, single column panel, new breakdown classes |
| `web_ui/js/components/LogRenderer.js` | Add `Activated` classification, ensure thumbnails + ability text (minor) |
| `engine/src/core/types.rs` | Add `surplus_hearts` to `PerformanceSnapshot` |
| `engine/src/turn/live.rs` | Fix `spare` calculation, fix `no_excess_heart` flags, compute `surplus_hearts` |
| `engine/src/ability/condition/card.rs` | Support color-specific surplus evaluation |

## Order of implementation

1. ✅ Plan document (this file)
2. **Performance Modal frontend** (PerformanceRenderer.js + performance.css)
3. Engine 余剰ハート fixes (types.rs + live.rs + card.rs)
4. Rule Log activation display (LogRenderer.js — minor)
