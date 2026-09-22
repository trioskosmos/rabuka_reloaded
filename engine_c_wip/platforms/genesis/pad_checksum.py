#!/usr/bin/env python3
# engine_c/platforms/genesis/pad_checksum.py
# Pad a Genesis ROM binary to a power-of-two size and write the Sega checksum.
# Checksum = sum of all 16-bit words from 0x200 to end-of-ROM, mod 0x10000,
# written big-endian at offset 0x1CE.
import sys

def main():
    if len(sys.argv) < 3:
        print("usage: pad_checksum.py <bin> <size>")
        sys.exit(2)
    path = sys.argv[1]
    size = int(sys.argv[2], 0)
    with open(path, "rb") as f:
        data = bytearray(f.read())
    if len(data) > size:
        print("warning: image %d > %d, not padding" % (len(data), size))
    else:
        data.extend(b"\x00" * (size - len(data)))
    # checksum over [0x200, len)
    s = 0
    for i in range(0x200, len(data) - 1, 2):
        s += (data[i] << 8) | data[i + 1]
    s &= 0xFFFF
    data[0x1CE] = (s >> 8) & 0xFF
    data[0x1CF] = s & 0xFF
    with open(path, "wb") as f:
        f.write(data)
    print("padded to %d bytes, checksum=0x%04X" % (len(data), s))

if __name__ == "__main__":
    main()
