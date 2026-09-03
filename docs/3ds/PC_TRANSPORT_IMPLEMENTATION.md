# PC Transport Implementation for 3DS Crossplay

## Overview
Added a transport-agnostic multiplayer architecture to enable 3DS ↔ PC crossplay over Direct LAN (same Wi-Fi network).

## Files Created/Modified

### New Files
- `platforms/3ds/src/transport.rs` — `Transport` trait + `UdsTransport` + `PcTransport` implementations
- `platforms/3ds/src/pc_transport.rs` — High-level `PcMultiplayer` wrapper for setup phases

### Modified Files
- `platforms/3ds/src/lib.rs` — Added `transport` and `pc_transport` modules
- `platforms/3ds/src/ctru_shim.c` — Added BSD socket FFI functions (`_3ds_pc_socket`, `_3ds_pc_connect`, `_3ds_pc_send`, `_3ds_pc_recv`, `_3ds_pc_close`, `_3ds_pc_is_connected`)
- `platforms/3ds/src/ffi.rs` — Added FFI declarations for PC transport functions
- `platforms/3ds/src/steps.rs` — Added PC multiplayer `SetupPhase` variants

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

## PC Multiplayer Setup Phases (added to SetupPhase)

| Phase | Purpose |
|-------|---------|
| `MultiplayerPcPickMode(usize)` | Cursor: 0=Host (create server), 1=Client (connect to IP) |
| `MultiplayerPcHostWait(usize)` | Host: waiting for PC client to connect |
| `MultiplayerPcClientConnect(usize)` | Client: edit IP address, press A to connect |
| `MultiplayerPcSyncDeck(usize, usize, bool)` | Exchange deck template IDs + seed |
| `MultiplayerPcLoading(usize, usize, bool, Option<Vec<u8>>, u64)` | Build identical GameState from sync data |

## Usage Flow

### Host (3DS)
1. Select "Local MP" → "PC Multiplayer" → "Host"
2. 3DS shows "Waiting for PC client..." + local IP
3. PC runs relay server, connects to 3DS IP:7341
4. Deck sync → identical GameState → play

### Client (3DS)
1. Select "Local MP" → "PC Multiplayer" → "Client"
2. Edit PC's IP address (D-pad), press A to connect
3. Deck sync → identical GameState → play

## PC Relay Server (next step)
Create `pc_relay/` crate that:
- Listens on UDP 7341
- Runs same `rabuka_engine` for deterministic lockstep
- Forwards `ActionSync` between 3DS and WebSocket clients (web UI)
- Stateless relay — no game logic, just packet forwarding

## References
- PGGKEC 3DS example: `research/PGGKEC/examples/3ds/source/main.c`
- libctru BSD sockets: `socket`, `connect`, `send`, `recv`, `fcntl`, `setsockopt`
- Your existing UDS protocol in `uds.rs` / `net.rs` reused unchanged