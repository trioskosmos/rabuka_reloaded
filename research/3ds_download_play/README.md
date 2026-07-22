# 3DS Download Play & Local Wireless Research

## Summary

The 3DS has two different local wireless systems:

1. **Download Play (DLP)** - Like Mario Kart DS/7 where players without the game can join
2. **Local Wireless (UDS)** - For players who both have the game installed

For Rabuka Reloaded, **UDS is the way to go** since both players will have the homebrew installed.

## Key Files

| File | Description |
|------|-------------|
| `uds_example.c` | Official libctru UDS demo (host/client with data transfer) |
| `dlp_client_example.c` | DLP client example (downloads game from host) |
| `hexisopath_uds.c` | **Working 2-player board game** using UDS - most relevant! |
| `download_play_wiki.md` | Technical docs on DLP protocol |
| `dlp_services_wiki.md` | DLP service commands reference |
| `dlp_client_pr.md` | Status of DLP client PR to libctru |

## The Reality of Download Play

**DLP works, but with major caveats for homebrew:**

1. **Client-side DLP**: There's an unmerged PR (#502) adding `dlp:CLNT` to libctru. The code works but isn't in mainline.

2. **Server-side DLP**: The `dlp:SRVR` service is a **system service**. For homebrew to act as a DLP server, you'd need to either:
   - Use the system's dlp:SRVR service (may not be accessible)
   - Implement the DLP protocol yourself over UDS (very complex)
   - Just use UDS directly (simpler)

3. **What DLP actually does**: The host sends a CIA to the client, which installs it temporarily. The client then runs a "DLP child" title. For a card game with custom assets, this is finicky.

4. **Real-world issues**: The nds-bootstrap project shows DLP crashes when hosting from homebrew.

## The UDS Alternative (Recommended)

UDS is what games use when both players have the game. It's:
- Simpler (no CIA transfer)
- Already working in libctru
- Has working examples (HexIsoPath)

## How UDS Works (from HexIsoPath)

### Host (Server):
```c
// 1. Generate network struct (max 2 players)
udsGenerateDefaultNetworkStruct(&networkstruct, wlancommID, 0, 2);

// 2. Create network with passphrase
udsCreateNetwork(&networkstruct, passphrase, strlen(passphrase)+1, &bindctx, data_channel, recv_buffer_size);

// 3. Set app data (for identification)
udsSetApplicationData(appdata, sizeof(appdata));

// 4. Optionally reject spectators
udsEjectSpectator();
```

### Client:
```c
// 1. Scan for networks
udsScanBeacons(tmpbuf, tmpbuf_size, &networks, &total_networks, wlancommID, 0, NULL, false);

// 2. Find matching network (check appdata header)
for (int i = 0; i < total_networks; i++) {
    if (memcmp(networks[i].network.appdata, appdata, 4+strlen(APP_DATA_HEADER)) == 0) {
        // 3. Connect
        udsConnectNetwork(&networks[i].network, passphrase, strlen(passphrase)+1, &bindctx, UDS_BROADCAST_NETWORKNODEID, UDSCONTYPE_Client, data_channel, recv_buffer_size);
        break;
    }
}
```

### Sending Data:
```c
udsSendTo(UDS_BROADCAST_NETWORKNODEID, data_channel, UDS_SENDFLAG_Default, data, size);
```

### Receiving Data:
```c
udsPullPacket(&bindctx, buffer, bufsize, &receivedSize, &src_NetworkNodeID);
```

## Key Constants

- `wlancommID`: Unique identifier for your game (use a random value like `0xFF150848`)
- `data_channel`: Usually 1 for game data
- `UDS_DEFAULT_RECVBUFSIZE`: Default receive buffer size
- `UDS_BROADCAST_NETWORKNODEID`: Send to all nodes
- `UDS_DATAFRAME_MAXSIZE`: Maximum data frame size

## Multiplayer Flow for Rabuka

1. **Menu**: Add "Local Multiplayer" option
2. **Lobby**: Player chooses Host or Join
3. **Host**:
   - Creates UDS network
   - Waits for client to connect
   - Runs game logic (TurnEngine)
   - Sends game state to client each turn
4. **Client**:
   - Scans for networks
   - Connects to host
   - Receives game state
   - Sends player actions to host

## Data to Sync

- Current phase (Draw, MainPhase, RPS, etc.)
- Player hands (card IDs)
- Field state
- Turn number
- Action results (what card was played, battle results, etc.)

## Notes

- `uds.h` header is in libctru, no special #include needed beyond `<3ds.h>`
- Both players must be in range (~10-30 feet)
- Connection is fast (milliseconds)
- No internet required, pure local WiFi
