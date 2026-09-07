# Web Multiplayer Architecture & Bandwidth Optimization

## Current Architecture (v2 - Delta + Optimistic + Minimal) — **DEPLOYED**

```
┌─────────────────────────────────────────────────────────────────┐
│  GitHub Pages (Free CDN) - STATIC ASSETS ONLY                    │
│  ├── index.html, JS, CSS  (~200 KB gzipped)                      │
│  ├── cards/cards.json      (2.5 MB)                              │
│  ├── cards/abilities.json  (1.4 MB)                              │
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

### Option A: Pure P2P (Zero Server)
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

### Option B: Hybrid (Keep Render for Matchmaking/Rooms)
```
┌─────────────────────────────────────────────────────────────────┐
│  GitHub Pages (Free CDN) - STATIC + WASM                         │
│  ├── rabuka_engine.wasm  (~3-5 MB gzipped)                       │
│  ├── All static assets (unchanged)                               │
└─────────────────────────────────────────────────────────────────┘
                               │
              ┌──────────────────┴──────────────────┐
              ▼                                     ▼
┌─────────────────────────────────┐ ┌─────────────────────────────────┐
│  Render (Matchmaking/Lobby)     │ │  Cloudflare Workers (Relay)     │
│  ├── Room create/join/list      │ │  ├── WebSocket relay            │
│  ├── Player presence            │ │  └── No game logic              │
│  ├── Spectator state (read-only)│ └─────────────────────────────────┘
│  └── Replay import/export       │              │
└─────────────────────────────────┘              │
                               │                 │
                               ▼                 ▼
┌─────────────────────────────────────────────────────────────────┐
│  Browser A (WASM Engine)          Browser B (WASM Engine)       │
│  ├── Full deterministic engine    ├── Full deterministic engine │
│  ├── Local replay/analysis      ├── Local replay/analysis       │
│  └── Offline play               └── Offline play                │
└─────────────────────────────────────────────────────────────────┘
```

**Per-turn bandwidth**: ~50 B (ActionSync + RNG seeds) + optional Render sync
**Latency**: Local execution + relay (~10-50 ms)
**Server cost**: $0 (Render free tier + Cloudflare Workers free tier)
**Benefit**: Keeps room management, spectator mode, replay features on Render

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
| **Static Assets from Pages** | `state.js` | `cards.json`, `abilities.json` from GitHub Pages |
| **Cross-Origin API** | `network.js` | `apiFetch` → Render, relative fetch → Pages |

---

## Bandwidth Comparison

| Metric | v1 (Full State) | v2 (Delta/Action) | v3a (WASM P2P Pure) | v3b (WASM Hybrid + Render) |
|--------|-----------------|-------------------|---------------------|----------------------------|
| **Per-turn (PVP)** | ~40 KB | **~232 B** | ~50 B | ~50 B |
| **Initial load** | 30 KB | 30 KB | +5 MB WASM | +5 MB WASM |
| **Monthly Render (1k games)** | 675 MB | **~2.3 MB** | $0 (deleted) | ~2.3 MB (lobby only) |
| **Monthly Relay (1k games)** | — | — | ~0.5 MB | ~0.5 MB |
| **Latency/turn** | 1 RTT (~100ms) | 1 RTT | Local (~0ms) + relay | Local (~0ms) + relay |
| **Offline/Replay** | ❌ | ❌ (server needed) | ✅ Full local | ✅ Full local |
| **Cheat prevention** | Server validates | Server validates | Commit-reveal + replay | Commit-reveal + replay |
| **Room mgmt / Spectate** | Server | Server | ❌ (P2P only) | ✅ Render |

---

## Offloading to GitHub Pages (✅ Done)

| Asset | Size | Host |
|-------|------|------|
| `cards.json` | 2.5 MB | GitHub Pages |
| `abilities.json` | 1.4 MB | GitHub Pages |
| 3,232 card `.webp` | 260 MB | GitHub Pages |
| `texticon/` UI icons | ~2 MB | GitHub Pages |
| `tutorial/` images | ~5 MB | GitHub Pages |
| All JS/CSS/HTML | ~500 KB | GitHub Pages |
| `rabuka_engine.wasm` (v3) | ~4 MB | GitHub Pages |

**Server serves**: Only dynamic game logic (actions, state deltas, SSE) in v2; **nothing in v3a**, lobby only in v3b
**Server stores**: Room metadata, game state per active match (~few KB each)

### GitHub Pages Bandwidth & Caching
- **Default cache**: **~10 minutes** (GitHub Pages sets `Cache-Control: max-age=600` for static assets)
- **No custom headers**: Cannot set `Cache-Control: max-age=31536000` on GitHub Pages
- **First visit**: ~270 MB downloaded (images + WASM)
- **Repeat visits within 10 min**: Served from browser cache
- **Repeat visits after 10 min**: Revalidated (304 Not Modified if unchanged)
- **Bandwidth counted**: On every cache miss (new visitors, cache expired, new versions)
- **100 GB/month soft limit** — supports ~370 first-time visitors/day (270 MB × 370 ≈ 100 GB)
- **Cost optimization**: For high traffic, consider Cloudflare Pages (free, 1 TB egress) or Cloudflare CDN in front of GitHub Pages

### Bandwidth Optimization Options

| Option | Egress Limit | Cache Control | Setup Effort | Cost |
|--------|--------------|---------------|--------------|------|
| **GitHub Pages (current)** | 100 GB/mo | 10 min fixed | 0 | Free |
| **Cloudflare Pages** | 1 TB/mo | Custom headers | Low (connect repo) | Free |
| **Cloudflare CDN + GitHub Pages** | 1 TB/mo (CF) | Full control | Medium (CNAME + proxy) | Free |
| **Cloudflare R2 + Workers** | 10 GB/mo free | Full control | High (migrate storage) | Free tier |

**Recommendation**: If traffic exceeds ~300 new visitors/day, switch to **Cloudflare Pages** (same repo, 1 TB egress, custom cache headers). Or put Cloudflare CDN in front of GitHub Pages (orange-cloud CNAME) for 1 TB free egress + custom cache rules without moving files.

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

### DOM Elements Missing
- **Issue**: `Element not found: room-code-header`, `room-display`, `system-status-badge`
- **Fix**: Added elements to `index.html:52-57` inside header info-bar

### Permissions-Policy Warning
- **Issue**: `[Compat] Permissions-Policy header: Unrecognized feature: 'attribution-reporting'`
- **Note**: This is a **GitHub Pages** added header, not server-controlled. Browser warning only, no functional impact.

### WASM Conversion (v3) — **IMPLEMENTED 2026-09-07**
- **Core engine**: Compiles to `wasm32-unknown-unknown` with `wasm` feature (`no_std`, `bytecode_abilities`, `compact_all`)
- **Card DB**: Embedded via `cards.bin` blob (537 KB) using `compact_card_data` feature
- **Abilities**: Bytecode format (28 KB compressed) via `bytecode_abilities` feature
- **RNG**: Deterministic xorshift32 (`engine/src/rng.rs`) - no `thread_rng()`, no time dependence
- **WASM crate**: `platforms/wasm/` with wasm-bindgen bindings
  - `WasmGameEngine` class: `execute_action`, `get_state_delta`, `get_legal_actions`, `serialize`
  - Deterministic RNG exports: `wasm_seed`, `wasm_shuffle`, `wasm_rand_range`
- **Build output**: 3.56 MB uncompressed, **1.49 MB gzipped** (matches 1-1.5 MB estimate)
- **Web Worker**: `web_ui/src/workers/gameWorker.js` runs engine off main thread
- **Frontend**: `WasmGameService.js` drop-in replacement for server-based `GameService`
- **CI/CD**: GitHub Actions workflow builds WASM + wasm-bindgen on push, deploys to `docs/wasm/`
- **Determinism**: Verified - engine uses HashMap only for key lookups (deterministic) or display/commutative ops

---

## Request Flow Optimization (v3)

### v2 Current (Server-Based) — ~3 RTT per turn
```
Client                          Render Server
  | POST /execute-action ──────► |
  | ◄──── 202 ActionResult       |
  | GET /delta?since=X ────────► |
  | ◄──── 200 GameStateDelta     |
  | SSE "update Y" ─────────────► (push)
```

### v3 WASM P2P — **0 RTT per turn** (local execution)
```
Client A (WASM)              Cloudflare Relay           Client B (WASM)
  | execute_action() ──────────► broadcast ──────────► |
  | apply_state_delta() ◄────── (action only) ◄───────► |
  | (local, instant)                                        |
```

### Request Reduction Summary

| Operation | v2 (Server) | v3 (WASM) | Reduction |
|-----------|-------------|-----------|-----------|
| Execute action | POST + GET delta (2 req) | **0 req** (local) | 100% |
| State sync | SSE push + poll (2 req) | **0 req** (local) | 100% |
| Legal actions | GET /legal-actions | **0 req** (local) | 100% |
| RNG sync | Server-generated | **Local** (commit-reveal) | 100% |
| Initial load | 30 KB | +1.49 MB WASM (one-time) | N/A |

### Bandwidth Per Turn (PVP)
- **v2**: ~232 B (POST + delta + SSE)
- **v3**: **~50 B** (ActionSync: tag + player + action + card_id + indices + area + baton + seq)

### Server Requests Eliminated (per game, ~50 turns)
| Endpoint | v2 Requests | v3 Requests |
|----------|-------------|-------------|
| `/execute-action` | 50 | **0** |
| `/game-state/delta` | 50 | **0** |
| `/events` (SSE) | 1 persistent | **0** |
| `/actions/legal` | 50 | **0** |
| **Total** | **~150** | **~2** (relay connect + room create) |

### Remaining Server Requests (v3b Hybrid)
Only lobby/matchmaking on Render:
- `POST /rooms/create` — once per game
- `GET /rooms/list` — periodic
- `GET /rooms/spectate` — spectators only
- `POST /export_game` / `POST /import_game` — replays