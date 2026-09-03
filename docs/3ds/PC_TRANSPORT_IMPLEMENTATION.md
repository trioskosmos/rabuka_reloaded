# PC Transport Implementation for 3DS Crossplay

## Overview
Added a transport-agnostic multiplayer architecture to enable 3DS ↔ PC crossplay over Direct LAN (same Wi-Fi network), alongside existing UDS (3DS↔3DS ad-hoc) multiplayer.

## Files Created/Modified

### New Files
- `platforms/3ds/src/transport.rs` — `Transport` trait + `UdsTransport` + `PcTransport` implementations
- `platforms/3ds/src/pc_transport.rs` — High-level `PcMultiplayer` wrapper for setup phases
- `pc_relay/` — Minimal PC relay server (UDP + WebSocket)

### Modified Files
- `platforms/3ds/src/lib.rs` — Added `transport` and `pc_transport` modules
- `platforms/3ds/src/ctru_shim.c` — Added BSD socket FFI functions (`_3ds_pc_socket`, `_3ds_pc_connect`, `_3ds_pc_send`, `_3ds_pc_recv`, `_3ds_pc_close`, `_3ds_pc_is_connected`, `_3ds_get_local_ip`)
- `platforms/3ds/src/ffi.rs` — Added FFI declarations for PC transport functions
- `platforms/3ds/src/steps.rs` — Added PC multiplayer `SetupPhase` variants
- `platforms/3ds/src/setup.rs` — Added 5 new PC multiplayer setup phase handlers

## Architecture

```
Transport Trait (transport-agnostic)
├── UdsTransport → wraps existing uds.rs (3DS ↔ 3DS ad-hoc)
└── PcTransport → BSD sockets via libctru (3DS ↔ PC Direct LAN)
```

### Protocol
- **Magic bytes**: `0x52424B50` ("RBKP") — 4-byte prefix on every packet
- **Payload**: Same `ActionSync` / `DeckSync` structs as UDS (~20 bytes/action)
- **Port**: 7341 (same as SysMon for consistency)
- **Reliability**: Sequence numbers + ACK + retry in Rust layer (same as UDS)

## All Multiplayer Setup Phases

### UDS (3DS ↔ 3DS Local Wireless)

| Phase | Purpose |
|-------|---------|
| `MultiplayerDeck(usize)` | Select deck for local MP |
| `MultiplayerPickRole(usize, usize)` | Choose Host (create) / Client (join) |
| `MultiplayerHostWait(usize)` | Host: create UDS network, wait for client |
| `MultiplayerClientScan(usize, u32)` | Client: scan for host networks (auto-rescan) |
| `MultiplayerClientHostSelect(usize, Vec<u16>, usize)` | Client: pick host from scanned list |
| `MultiplayerSyncDeck(usize, usize, bool)` | Exchange deck template IDs + seed |
| `MultiplayerLoading(usize, usize, bool, Option<Vec<u8>>, u64)` | Build identical GameState from sync |

### PC Direct LAN (3DS ↔ PC via UDP)

| Phase | Purpose |
|-------|---------|
| `MultiplayerPcPickMode(usize)` | Cursor: 0=Host (3DS hosts), 1=Client (3DS connects to PC) |
| `MultiplayerPcHostWait(usize)` | Host: bind UDP 7341, show local IP, wait for PC relay |
| `MultiplayerPcClientConnect(usize)` | Client: edit PC IP (D-pad L/R=cursor, U/D=digit), A=connect |
| `MultiplayerPcSyncDeck(usize, usize, bool)` | Exchange deck template IDs + seed over UDP |
| `MultiplayerPcLoading(usize, usize, bool, Option<Vec<u8>>, u64)` | Build identical GameState from sync |

## Usage Flow

### 3DS as Host (PC connects to 3DS)
1. 3DS: Local MP → PC Multiplayer → **Host**
2. 3DS shows: "Your IP: 192.168.1.50" + "Port: 7341"
3. PC: `cargo run -- -u 0.0.0.0:7341 -w 0.0.0.0:7342`
4. PC relay connects to `192.168.1.50:7341`
5. Deck sync → identical GameState → play

### 3DS as Client (3DS connects to PC)
1. PC: `cargo run -- -u 0.0.0.0:7341 -w 0.0.0.0:7342` (shows PC IP)
2. 3DS: Local MP → PC Multiplayer → **Client**
3. 3DS: Edit PC IP (L/R=cursor, U/D=digit), press **A** to connect
4. Deck sync → identical GameState → play

### Web Browser Player
1. Open `ws://<PC_IP>:7342` in browser
2. Join same session ID
3. Actions forwarded bidirectionally via relay

## PC Relay Server (`pc_relay/`)

```bash
cd pc_relay
cargo run -- --cards-path ../cards/cards.json --decks-path ../cards/decks/
# UDP  : 0.0.0.0:7341  (3DS ↔ Relay)
# WS   : 0.0.0.0:7342  (Browser ↔ Relay)
```

**Architecture:**
```
3DS (UDP) ──► [PC Relay: UDP listener + GameState] ──► WebSocket clients (browser)
                 │
                 ▼
         Deterministic engine
         (same seed → same state)
         Only ActionSync (~20 bytes) forwarded
```

**Features:**
- Runs same `rabuka_engine` for deterministic lockstep
- Forwards `ActionSync` between 3DS and WebSocket clients
- Session management with timeout cleanup
- Deck sync via `DeckSync` (template IDs + seed)

## Integration with Existing Code

Your existing UDS protocol in `uds.rs` / `net.rs` is **reused unchanged**:
- `ActionSync` struct (action_tag, card_id, card_indices, stage_area, use_baton_touch, ability_index, action_seq, player_id)
- `DeckSync` struct (seed, p1/p2 main/energy templates)
- `net.rs` routing functions (`route_authoritative_action`, `execute_received_action`)
- Only transport layer swapped: `UdsTransport` ↔ `PcTransport`

## References
- PGGKEC 3DS example: `research/PGGKEC/examples/3ds/source/main.c`
- libctru BSD sockets: `socket`, `connect`, `send`, `recv`, `fcntl`, `setsockopt`
- SysMon port 7341 convention
- Your existing UDS implementation in `uds.rs` / `net.rs`