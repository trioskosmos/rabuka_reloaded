# Web Multiplayer Architecture & Bandwidth Optimization

## Current Architecture (v1 - Full State Push)

```
┌─────────────────────────────────────────────────────────────────┐
│  GitHub Pages (Free CDN)                                         │
│  ├── index.html, JS, CSS  (~200 KB gzipped)                      │
│  ├── cards/cards.json      (2.5 MB)                              │
│  ├── cards/abilities.json  (1.4 MB)                              │
│  └── img/cards_webp/       (3,232 × 80 KB = 260 MB)             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Render/Fly.io (Rust Server)                                     │
│  ├── /api/game-state      →  Full GameStateResponse (10-50 KB)  │
│  ├── /api/execute-action  →  Full GameStateResponse (10-50 KB)  │
│  ├── /api/events (SSE)    →  "update" ping only (0 bytes data)  │
│  └── /api/health          →  {"status":"ok"}                     │
└─────────────────────────────────────────────────────────────────┘
```

**Per-turn bandwidth**: ~20-50 KB (JSON state round-trip)
**Monthly on Render free tier**: ~750 hrs × 30 turns/hr × 30 KB ≈ 675 MB (well within limits but wasteful)

---

## 3DS Architecture (Reference - Deterministic Sync)

```
┌─────────────────────────────────────────────────────────────────┐
│  3DS Cartridge (Local)                                           │
│  ├── Full engine + card database                                 │
│  ├── All card images (local)                                     │
│  └── Deterministic RNG seed                                      │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  UDS/UDP Local Wireless                                          │
│  ├── DeckSync (once):  seed(8) + 4×len(2) + N×u16(2) ≈ 150 B   │
│  ├── ActionSync (per turn): tag(1) + player(1) + action_tag(2)  │
│  │   + card_id?(2) + idx_len(1) + N×idx(2) + area(1)            │
│  │   + baton(1) + ability_idx?(2) + seq(4) ≈ 30-50 B           │
│  └── ACK: 5 bytes                                                │
└─────────────────────────────────────────────────────────────────┘
```

**Per-turn bandwidth**: ~50 bytes (1000× less than web)
**Model**: Deterministic - both clients run identical engine, only actions sync

---

## Target Web Architecture (v2 - Delta + Optimistic)

```
┌─────────────────────────────────────────────────────────────────┐
│  GitHub Pages (Free CDN) - UNCHANGED                             │
│  All static assets: HTML, JS, CSS, cards.json, abilities.json,  │
│  3,232 card images (260 MB)                                      │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Render/Fly.io (Rust Server) - OPTIMIZED                         │
│  ├── /api/game-state              →  Full state (initial load)  │
│  ├── /api/game-state/delta?since  →  Delta (changed fields)     │
│  ├── /api/execute-action          →  ActionResult (minimal)     │
│  ├── /api/actions/legal           →  Legal actions only         │
│  ├── /api/events (SSE)            →  "update" + frame_id        │
│  └── /api/health                  →  {"status":"ok"}            │
└─────────────────────────────────────────────────────────────────┘
```

**Per-turn bandwidth**: ~500 B - 2 KB (10-50× reduction)
**Key changes**:
1. `execute-action` returns `ActionResult` not full state
2. Delta endpoint for incremental updates
3. Legal actions separate endpoint
4. SSE includes frame_id for delta requests

---

## Implementation Plan

### Phase 1: Server (Rust)

1. **Add `ActionResult` struct** - minimal response for execute-action
2. **Add `/api/game-state/delta?since=<frame>`** - returns only changed fields
3. **Add `/api/actions/legal`** - returns legal actions only
4. **Modify SSE** to include `frame_id` in update messages
5. **Track frame counter** in Room state

### Phase 2: Client (JS)

1. **Update `sendAction`** to expect `ActionResult`
2. **Add delta polling** alongside SSE (fallback)
3. **Lazy-load legal actions** via separate endpoint
4. **Optimistic UI** - apply action locally, reconcile with delta

---

## Data Structures

### ActionResult (new)
```rust
pub struct ActionResult {
    pub success: bool,
    pub error: Option<String>,
    pub frame_id: u64,
    pub state_delta: Option<GameStateDelta>,  // Only if error or complex
}
```

### GameStateDelta (new)
```rust
pub struct GameStateDelta {
    pub frame_id: u64,
    pub phase: Option<Phase>,
    pub active_player: Option<u8>,
    pub rps_winner: Option<u8>,
    pub player1_rps_choice: Option<u8>,
    pub player2_rps_choice: Option<u8>,
    pub zone_changes: Vec<ZoneChange>,  // card moved from A to B
    pub log_entries: Vec<LogEntry>,
    pub pending_choice: Option<Choice>,
    pub legal_actions: Option<Vec<ActionIndex>>,
    // ... only fields that actually changed
}
```

### ZoneChange
```rust
pub struct ZoneChange {
    pub card_id: i16,
    pub from_zone: Zone,
    pub to_zone: Zone,
    pub from_index: Option<usize>,
    pub to_index: Option<usize>,
}
```

---

## Migration Strategy

1. **Keep backward compatibility** - old endpoints work for initial load
2. **Feature flag** - client detects delta support via `api/game-state/version`
3. **Progressive enhancement** - if delta fails, fall back to full state
4. **Version bump** - increment frame counter on every state mutation

---

## Expected Bandwidth Savings

| Endpoint | Before | After | Reduction |
|----------|--------|-------|-----------|
| `/api/game-state` (initial) | 30 KB | 30 KB | 0% (needed) |
| `/api/execute-action` | 30 KB | 500 B | 98% |
| `/api/game-state` (polling) | 30 KB | 1-2 KB (delta) | 93-97% |
| SSE | ping only | ping + frame_id | same |
| **Per turn (PVP)** | ~40 KB | ~1.5 KB | **96%** |

**Monthly Render bandwidth** (estimated): 675 MB → **27 MB** (25× less)

---

## Offloading to GitHub Pages (Already Done ✅)

| Asset | Size | Host |
|-------|------|------|
| `cards.json` | 2.5 MB | GitHub Pages |
| `abilities.json` | 1.4 MB | GitHub Pages |
| `card_id_mapping.json` | ~500 KB | GitHub Pages |
| 3,232 card `.webp` | 260 MB | GitHub Pages |
| `texticon/` UI icons | ~2 MB | GitHub Pages |
| `tutorial/` images | ~5 MB | GitHub Pages |
| All JS/CSS/HTML | ~500 KB | GitHub Pages |

**Server serves**: Only dynamic game logic (actions, state deltas, SSE)
**Server stores**: Room metadata, game state per active match (~few KB each)

---

## Next Steps

1. Create `GameStateDelta` and `ActionResult` types in `engine/src/game/display.rs`
2. Implement `generate_delta(since_frame)` in `game_state`
3. Add delta endpoint in `web_server.rs`
4. Add legal actions endpoint
5. Update JS `network.js` and `GameService.js`
6. Test with 2-browser PVP session