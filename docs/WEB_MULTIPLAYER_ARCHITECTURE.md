# Web Multiplayer Architecture & Bandwidth Optimization

## Current Architecture (v2 - Delta + Optimistic + Minimal) — **DEPLOYED**

```
┌─────────────────────────────────────────────────────────────────┐
│  GitHub Pages (Free CDN) - STATIC ASSETS ONLY                    │
│  ├── index.html, JS, CSS  (~200 KB gzipped)                      │
│  ├── cards/cards.json      (2.5 MB)                              │
│  ├── cards/abilities.json  (1.4 MB)                              │
│  ├── engine/card_id_mapping.json (~500 KB, optional)             │
│  └── img/cards_webp/       (3,232 × 80 KB = 260 MB)             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Render (Rust Server) - DYNAMIC API ONLY                         │
│  ├── POST /api/execute-action  →  ActionResult {success, frame} │
│  ├── GET  /api/game-state/delta?since=X  →  Delta (action only) │
│  ├── GET  /api/actions/legal   →  Action types only             │
│  ├── GET  /api/events (SSE)    →  "update <frame_id>"           │
│  ├── GET  /api/rooms/spectate  →  Read-only game state          │
│  ├── POST /api/export_game     →  Full state snapshot           │
│  ├── POST /api/import_game     →  Replay import (stub)          │
│  ├── GET  /metrics             →  Prometheus format             │
│  └── GET  /health              →  {"status":"ok"}               │
└─────────────────────────────────────────────────────────────────┘
```

**Per-turn bandwidth (PVP)**: **~232 bytes** (99.4% reduction from v1)
- `POST /api/execute-action` → 80 B (`{"success":true,"frame_id":42}`)
- `GET /api/game-state/delta?since=42` → 130 B (`{"frame_id":43,"executed_action":{...}}`)
- SSE push → 22 B (`data: update 43\n\n`)

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

**Per-turn bandwidth**: ~50 bytes (4.6× less than current web)
**Model**: Deterministic - both clients run identical engine, only actions sync

---

## Target Web Architecture (v3 - WASM P2P) — **THEORETICAL**

```
┌─────────────────────────────────────────────────────────────────┐
│  GitHub Pages (Free CDN) - STATIC + WASM                         │
│  ├── rabuka_engine.wasm  (~3-5 MB gzipped)                       │
│  ├── All static assets (unchanged)                               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Cloudflare Workers / Fly.io (Relay Only)                        │
│  ├── WebSocket / WebRTC relay (~10 MB, no game logic)           │
│  └── Rate limiting, DDoS protection                              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Browser A (WASM Engine)          Browser B (WASM Engine)       │
│  ├── Full deterministic engine    ├── Full deterministic engine │
│  ├── Local replay/analysis      ├── Local replay/analysis       │
│  └── Offline play               └── Offline play                │
└─────────────────────────────────────────────────────────────────┘
```

**Per-turn bandwidth**: ~50 B (ActionSync + RNG seeds)
**Latency**: Local execution + relay (~10-50 ms)
**Server cost**: $0 (Cloudflare Workers free tier)

---

## Implemented Endpoints (v2)

| Endpoint | Method | Response | Size | Purpose |
|----------|--------|----------|------|---------|
| `/api/execute-action` | POST | `ActionResult {success, frame_id}` | ~80 B | Execute action, get frame |
| `/api/game-state/delta?since=X` | GET | `GameStateDelta {frame_id, executed_action}` | ~130 B | Incremental sync |
| `/api/actions/legal` | GET | `{actions: [{action_type, description}]}` | ~200 B | Legal moves only |
| `/api/events` (SSE) | GET | `data: update <frame_id>\n\n` | ~22 B | Real-time push |
| `/api/rooms/spectate?room_id=X` | GET | Full `GameStateDisplay` (read-only) | ~30 KB | Spectators |
| `/api/export_game` | GET | Full state + action history | ~30 KB | Replay export |
| `/api/import_game` | POST | Stub (returns success) | — | Replay import |
| `/metrics` | GET | Prometheus text format | ~200 B | Observability |
| `/health` | GET | `{"status":"ok"}` | — | Health check |

---

## Data Structures (Implemented)

### ActionResult
```rust
pub struct ActionResult {
    pub success: bool,
    pub error: Option<String>,
    pub frame_id: u64,
    pub state_delta: Option<GameStateDelta>,
}
```

### GameStateDelta (Minimal - Action-Only)
```rust
pub struct GameStateDelta {
    pub frame_id: u64,
    pub phase: Option<Phase>,
    pub active_player: Option<u8>,
    pub rps_winner: Option<u8>,
    pub player1_rps_choice: Option<u8>,
    pub player2_rps_choice: Option<u8>,
    pub pending_choice: Option<serde_json::Value>,
    pub legal_actions: Option<Vec<ActionIndex>>,
    pub executed_action: Option<FrameAction>,  // The action to replay
    pub rng_results: Option<Vec<RngResult>>,   // Future: shuffle seeds
}
```

### FrameAction (Stored per frame for delta sync)
```rust
pub struct FrameAction {
    pub action_type: String,
    pub player_id: u8,
    pub card_id: Option<i16>,
    pub card_indices: Option<Vec<usize>>,
    pub stage_area: Option<String>,
    pub use_baton_touch: bool,
}
```

---

## Client-Side Improvements (Implemented)

| Feature | File | Description |
|---------|------|-------------|
| **Optimistic Updates** | `GameService.js` | Apply action locally before server confirms |
| **Delta Polling** | `GameService.js` | Poll `/delta?since=X` after SSE push |
| **SSE Reconnection** | `SSEClient.js` | Exponential backoff (1s→30s cap) |
| **SSE Frame ID** | `SSEClient.js` | Server sends `update <frame_id>` |
| **Static Assets from Pages** | `state.js` | `cards.json`, `card_id_mapping.json` from GitHub Pages |
| **Cross-Origin API** | `network.js` | `apiFetch` → Render, relative fetch → Pages |

---

## Bandwidth Comparison

| Metric | v1 (Full State) | v2 (Delta/Action) | v3 (WASM P2P) |
|--------|-----------------|-------------------|---------------|
| **Per-turn (PVP)** | ~40 KB | **~232 B** | ~50 B |
| **Initial load** | 30 KB | 30 KB | +5 MB WASM |
| **Monthly Render (1k games)** | 675 MB | **~2.3 MB** | ~0.5 MB (relay) |
| **Latency/turn** | 1 RTT (~100ms) | 1 RTT | Local (~0ms) + relay |
| **Offline/Replay** | ❌ | ❌ (server needed) | ✅ Full local |
| **Cheat prevention** | Server validates | Server validates | Commit-reveal + replay |

---

## Offloading to GitHub Pages (✅ Done)

| Asset | Size | Host |
|-------|------|------|
| `cards.json` | 2.5 MB | GitHub Pages |
| `abilities.json` | 1.4 MB | GitHub Pages |
| `card_id_mapping.json` | ~500 KB | GitHub Pages (optional, generated at deploy) |
| 3,232 card `.webp` | 260 MB | GitHub Pages |
| `texticon/` UI icons | ~2 MB | GitHub Pages |
| `tutorial/` images | ~5 MB | GitHub Pages |
| All JS/CSS/HTML | ~500 KB | GitHub Pages |

**Server serves**: Only dynamic game logic (actions, state deltas, SSE)
**Server stores**: Room metadata, game state per active match (~few KB each)

---

## Recent Fixes (2026-09-07)

### CORS Configuration Fixed
- **Issue**: Preflight requests blocked - `Access-Control-Allow-Origin` missing
- **Fix**: Updated CORS middleware in `web_server.rs:3178-3182` to include:
  - `allowed_origin("https://trioskosmos.github.io")` + trailing slash variant
  - `allowed_methods: GET, POST, OPTIONS, HEAD`
  - `allowed_headers: Content-Type, Authorization, X-Session-Token, X-Room-Id, Accept, Origin`
  - `supports_credentials()` + `expose_headers`
  - Removed manual CORS headers from SSE endpoint (middleware handles it)

### Missing `card_id_mapping.json` on GitHub Pages
- **Issue**: `GET https://trioskosmos.github.io/rabuka_reloaded/engine/card_id_mapping.json 404`
- **Fix**: Added fallback in deploy workflow (`.github/workflows/deploy-pages.yml:26-37`) to create empty `{}` if file doesn't exist
- **Client handling**: Already graceful in `state.js:363-367` - optional, doesn't block loading

### DOM Elements Missing
- **Issue**: `Element not found: room-code-header`, `room-display`, `system-status-badge`
- **Fix**: Added elements to `index.html:52-57` inside header info-bar

### Permissions-Policy Warning
- **Issue**: `[Compat] Permissions-Policy header: Unrecognized feature: 'attribution-reporting'`
- **Note**: This is a **GitHub Pages** added header, not server-controlled. Browser warning only, no functional impact.

---

## Future: WASM P2P (v3) — Effort Estimate

| Component | Status | Effort |
|-----------|--------|--------|
| Core engine → WASM | ✅ Compiles (`wasm32-unknown-unknown`) | 0 |
| Card DB embedding | ⚠️ 2.5 MB `include_bytes!` or async fetch | 2 days |
| Determinism audit | ❌ `HashMap` iteration, `thread_rng()`, time | 1 week |
| `wasm-bindgen` bindings | ❌ Need JS glue | 2 days |
| Web Worker off-main-thread | ❌ `postMessage` + `SharedArrayBuffer` | 2 days |
| Commit-reveal anti-cheat | ❌ Ed25519, replay verify | 3-5 days |
| Cloudflare Workers relay | ❌ 10 lines Worker script | 1 day |
| **Total** | | **~3 weeks** |

**Verdict**: Stay with v2. It works, costs $0, scales. Port to WASM only when you need true P2P, offline replay, or zero-server-cost at massive scale.