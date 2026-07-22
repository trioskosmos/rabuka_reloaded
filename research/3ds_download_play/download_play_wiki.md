# Download Play - 3dbrew

## Overview

The 3DS dlplay title has two dlplay modes: 3DS and DS. DS dlplay is just regular dsmode dlplay, same interface and protocol as before. Like DS gamecards, holding down start+select while starting the dsmode dlplay client will disable stretching the screens.

## Download Play Protocol

The Download Play protocol for 3DS is completely different from the DS Wireless Multiboot (WMB) protocol. While the DS WMB protocol used to send program code in plaintext over wireless, the 3DS Download Play protocol uses UDS which uses CCMP encryption etc.

### Download Play UDS Protocol

This section describes the data transferred using the UDS service. All data is stored as big-endian.

UDS data_channel 0x1 is used for spectator data, while all non-spectator data uses data_channel 0x2. The spectator data is received by connecting to the network as a spectator then receiving data-frames, this is handled when scanning for DLP networks.

The spectator data includes the 48x48 icon, this has the same format as SMDH.

#### Data Frame Structure

This is the data starting at offset 0x0 of UDS PullPacket/SendTo:

| Offset | Size | Description |
|--------|------|-------------|
| 0x0 | 0x1 | Must be 0x1 for spectator data. |
| 0x3 | 0x3 | ? |
| 0x4 | 0x2 | Size of the entire frame. The actual_size from PullPacket is the same size as this value. |
| 0x6 | 0x2 | u16, unknown. For spectator data this is only used for the metadata frame. |
| 0x8 | 0x4 | Checksum |
| 0xC | 0x1 | Spectator data: frameid, must be less than total_frames. Normal data: unknown. |
| 0xD | 0x1 | Spectator data: total_frames. Normal data: unknown. |
| 0xE | 0x1 | Unknown. This must match a state value. When this frame value is non-zero, 0x1 is used for the frame value when doing the compare instead. |
| 0xF | 0x1 | ? |
| 0x10 | - | The frame-specific payload starts here. |

Total_frames is at least 0x4 normally. When a sysupdate is included, total_frames is 0x4+<total frames required for the titlelist(normally 0x1)>. Total_frames should be <=0x20, but the code doesn't check for this specific value.

#### Checksum

The checksum seed is the 4-byte output from encrypting zeros with AES-CTR using keytype5. The CTR is the output from this:

```c
for(i=0; i<0x10; i++) ctr[i] = ctrseed[i] ^ host_macaddress[i % 6];
```

This ctrseed is a fixed 0x10-byte random-data block stored in DLP-sysmodule .rodata. This seed is initialized after connecting to/creating the DLP network.

The checksum stored in the above data frame header is then calculated using this checksum seed.

First, the calc_checksum is initialized to 0. Then calc_checksum is added with all words in the data frame loaded as big-endian, with the data-frame checksum cleared to zero here. If the frame_size isn't word-aligned, the remaining <4-bytes are loaded as big-endian for adding as well.

Then this is run:

```c
//During init before the above adding, shift and count are initialized:
//shift = (((u8*)&checksum_seed)[2] & 0xf) + 0x4;
//count = (((u8*)&checksum_seed)[3] & 0x7) + 0x2;
for(pos=0; pos<count; pos++) checksum = (checksum<<shift | checksum>>shift) ^ checksum_seed;
//The u32 checksum_seed is byte-swapped on 3DS for this.
```

Lastly the calculated checksum is written to output as big-endian (hence on 3DS it's byte-swapped before writing to output).

#### Frames

| Frameid | frame_size | Description |
|---------|------------|-------------|
| 0x0 | 0x300 | Metadata + start of the icon gfx, see below. |
| 0x1-0x3 | 0x5A8 | The remaining icon gfx. |
| 0x4-<total frames-1> | 0x5B8 | Sysupdate titlelist, if any. |

##### Metadata Frame

| Offset | Size | Description |
|--------|------|-------------|
| 0x0 | 0x8 | u64 DLP-child titleID. Must be a CTR TID-high, with the TID-high set for DLP-child. The low 4-bits of the TID-low must be clear. |
| 0x8 | 0x2 | u16, probably the DLP-child title-version. |
| 0xA | 0x1 | u8, unknown. |
| 0xB | 0x1 | u8, unknown. |
| 0xC | 0x4 | u32, chunk_size. This is the chunk_size used for transferring the CIA. This is the exact size used for the FS .cia file-reads on the host, and the exact size used for AM CIA file-writes on the client(s). Normally this is 0x0003FFC0. |
| 0x10 | 0x2 | Unused by the DLP-client (sysmodule). |
| 0x12 | 0x2 | u16, unknown. |
| 0x14 | 0x4 | Unused by the DLP-client (sysmodule). |
| 0x18 | 0x10 | Unknown 0x10-byte structure. |
| 0x28 | 0x4 | u32, unknown. |
| 0x2C | 0x4 | u32, unknown. Must be <=0x02000000. |
| 0x30 | 0x80 | UTF-16 Application name string. The last u16 is ignored, value 0x0 is always written to output for it instead. |
| 0xB0 | 0x100 | UTF-16 Description string. The last u16 is ignored, value 0x0 is always written to output for it instead. |
| 0x1B0 | 0x138 | Array of 0x9C u16s for the start of the icon gfx. |

## Broadcasted Application Data

The Download Play protocol broadcasts 3DS application data in the CIA format. The title is installed to NAND, and is kept there until new CIA data from a different game is received through the Download Play protocol.

## Remote Distribution of System-Updates

As part of the child distribution process, a 3DS acting as the server in a local Download Play session, can send system updates to another 3DS unit acting as the client, through first sending the system update package then instructing the client to install reboot and reinstantiate a connection (which it caches information about temporarily) remotely, if it finds system updates are necessary before distributing the child-application.
