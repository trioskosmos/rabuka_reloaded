# DLP Services - 3dbrew

Download Play services are not used by the Download Play child application at all (the title which runs on the Download Play clients). This is instead: APT:GetWirelessRebootInfo.

There can only be one session open for each individual DLP service at a time.

## Download Play Client Service "dlp:CLNT"

| Command | Description |
|---------|-------------|
| 0x000100C3 | Initialize |
| 0x00020000 | Finalize |
| 0x00030000 | GetEventDesc. Writes 0x18-bytes of output. |
| 0x00040000 | GetChannel. Writes an output u16. |
| 0x00050180 | StartScan |
| 0x00060000 | StopScan |
| 0x00070080 | GetServerInfo |
| 0x00080100 | GetTitleInfo |
| 0x00090040 | GetTitleInfoInOrder |
| 0x000A0080 | DeleteScanInfo |
| 0x000B0100 | PrepareForSystemDownload |
| 0x000C0000 | StartSystemDownload |
| 0x000D0100 | StartTitleDownload |
| 0x000E0000 | GetMyStatus |
| 0x000F0040 | GetConnectingNodes |
| 0x00100040 | GetNodeInfo |
| 0x00110000 | GetWirelessRebootPassphrase |
| 0x00120000 | StopSession |
| 0x00130100 | GetCupVersion |
| 0x00140100 | GetDupAvailability |

## Download Play Server Service "dlp:SRVR"

| Command | Description |
|---------|-------------|
| 0x00010183 | Initialize |
| 0x00020000 | Finalize |
| 0x00030000 | GetServerState |
| 0x00040000 | GetEventDescription |
| 0x00050080 | StartAccepting |
| 0x00060000 | EndAccepting |
| 0x00070000 | StartDistribution |
| 0x000800C0 | SendWirelessRebootPassphrase |
| 0x00090040 | AcceptClient |
| 0x000A0040 | DisconnectClient |
| 0x000B0042 | GetConnectingClients |
| 0x000C0040 | GetClientInfo |
| 0x000D0040 | GetClientState |
| 0x000E0040 | IsChild |
| 0x000F0303 | InitializeWithName |
| 0x00100000 | GetDupNoticeNeed |

## Download Play Fake Client Service "dlp:FKCL"

Similar to dlp:CLNT but with additional commands for fake sessions.

## WirelessRebootPassphrase

This 9-byte UDS passphrase is sent by the DLP host application to the DLP clients via DLPSRVR:SendWirelessRebootPassphrase. The dlplay client system-application then loads it via DLP:GetWirelessRebootPassphrase for setting the NS WirelessRebootInfo.

Normally this is a randomly-generated ASCII hex string, however it can be arbitrary. These strings are less than 9-bytes for some titles, the unused bytes are cleared to zero. This is the passphrase for the new UDS network which will be used by the clients and host for communicating, once the DLP child titles on those clients launch.
