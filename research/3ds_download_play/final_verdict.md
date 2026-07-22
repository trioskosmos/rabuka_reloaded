# DLP Server: Final Verdict

## Can homebrew use dlp:SRVR?

**Technically yes** - `dlp:SRVR` is listed in the ServiceAccessControl of multiple RSF templates:
- lpp-3ds template
- BootNTR template
- Various other homebrew templates

This means homebrew CAN request a handle to the service via `srvGetServiceHandle("dlp:SRVR")`.

## Has anyone done it?

**No.** After extensive searching:
- No homebrew source code exists that uses `dlp:SRVR`
- The only implementation is in Azahar emulator (Citra fork) for HLE
- The libctru DLP client PR (#502) has no server example
- The maintainer specifically asked "do you have a server example?" and got no answer

## Why hasn't anyone done it?

Looking at the Azahar implementation, DLP server is complex:
1. Initialize with shared memory
2. Create UDS network with specific parameters (wlan_comm_id=0x2710, id8=0x55)
3. Broadcast title info (icon, name, description) via spectator data
4. Handle client authentication packets
5. Send CIA fragments with checksums
6. Handle wireless reboot passphrase
7. Manage client state machine

## What would it take?

To implement DLP server in homebrew, you'd need to:
1. Open `dlp:SRVR` service
2. Call Initialize with shared memory
3. Call StartAccepting to create the UDS network
4. Implement the broadcast/title info system
5. Handle client connections and authentication
6. Implement CIA fragment sending (if distributing content)
7. All with raw IPC calls (no libctru wrapper exists)

## Conclusion

**UDS is still the right choice** for Rabuka Reloaded:
- Working examples exist (HexIsoPath)
- Simpler API
- Both players have the game
- No need to transfer a CIA

DLP server is a "first person to do it" project - possible but lots of work.
