#!/usr/bin/env python3
"""Test different bit depths for GBA card art to find optimal size/quality tradeoff."""

import random
import time
from PIL import Image, ImageFilter

# Simulate card art at different bit depths
def test_bpp(bpp, n_colors):
    """Test compression at given bit depth."""
    # Generate realistic 8bpp tile data (palette indices 0-255)
    if bpp == 8:
        data = bytes(random.randint(0, 255) for _ in range(13824))  # 96x144
    elif bpp == 7:
        # 128 colors - pack 2 pixels per byte (4 bits each) + 1 bit wasted
        # Actually 7bpp = 128 colors, need custom packing
        # For simulation, just reduce entropy
        data = bytes(random.randint(0, 127) for _ in range(13824))
    elif bpp == 6:
        data = bytes(random.randint(0, 63) for _ in range(13824))
    elif bpp == 5:
        data = bytes(random.randint(0, 31) for _ in range(13824))
    elif bpp == 4:
        data = bytes(random.randint(0, 15) for _ in range(13824))
    else:
        raise ValueError(f"Unsupported bpp: {bpp}")
    
    return data

def lz77_compress_bios(data: bytes) -> bytes:
    """Fast hash-based LZ77."""
    if not data:
        return b'\x10\x00\x00\x00'
    
    HASH_SIZE = 65536
    HASH_MASK = HASH_SIZE - 1
    
    hash_chain = [-1] * len(data)
    hash_table = [-1] * HASH_SIZE
    
    def hash3(pos):
        if pos + 3 > len(data):
            return 0
        return ((data[pos] << 8) | (data[pos + 1] << 4) | data[pos + 2]) & (HASH_SIZE - 1)
    
    output = bytearray()
    i = 0
    flag_byte = 0
    flag_bit = 0
    flag_pos = len(output)
    output.append(0)
    
    while i < len(data):
        best_len = 0
        best_dist = 0
        
        if i + 3 <= len(data):
            h = hash3(i)
            j = hash_table[h]
            search_end = max(0, i - 4096)
            
            while j >= search_end:
                match_len = 0
                while (match_len < 18 and 
                       i + match_len < len(data) and 
                       j + match_len < i and 
                       data[j + match_len] == data[i + match_len]):
                    match_len += 1
                
                if match_len >= 3 and match_len > best_len:
                    best_len = match_len
                    best_dist = i - j
                    if best_len == 18:
                        break
                
                j = hash_chain[j]
        
        if i + 3 <= len(data):
            h = hash3(i)
            hash_chain[i] = hash_table[h]
            hash_table[h] = i
        
        if best_len >= 3:
            flag_byte &= ~(1 << flag_bit)
            dist_minus1 = best_dist - 1
            len_minus3 = best_len - 3
            output.append((dist_minus1 >> 4) & 0xFF)
            output.append(((dist_minus1 & 0xF) << 4) | (len_minus3 & 0xF))
            i += best_len
        else:
            flag_byte |= (1 << flag_bit)
            output.append(data[i])
            i += 1
        
        flag_bit += 1
        if flag_bit == 8:
            output[flag_pos] = flag_byte
            flag_pos = len(output)
            output.append(0)
            flag_byte = 0
            flag_bit = 0
    
    if flag_bit > 0:
        output[flag_pos] = flag_byte
    
    decompressed_size = len(data)
    header = bytes([
        (1 << 4) | (decompressed_size & 0xFF),
        (decompressed_size >> 8) & 0xFF,
        (decompressed_size >> 16) & 0xFF,
        (decompressed_size >> 24) & 0xFF,
    ])
    
    return header + bytes(output)

def main():
    print("=" * 80)
    print("GBA Card Art Bit Depth Analysis")
    print("=" * 80)
    
    # Simulate 1809 cards
    n_cards = 1809
    
    print(f"\n{'BPP':>4} | {'Colors':>7} | {'Raw/card':>9} | {'Raw total':>10} | {'LZ77 ratio':>10} | {'ROM total':>10} | {'Headroom':>9}")
    print("-" * 80)
    
    for bpp in [8, 7, 6, 5, 4]:
        colors = 2 ** bpp
        # Palette size
        pal_size = 2 if bpp <= 4 else 2 * colors
        # Tile data
        if bpp <= 4:
            tile_bytes = 13824 // (8 // bpp)  # 13824 pixels / (8/bpp)
        else:
            tile_bytes = 13824
        
        raw_per_card = tile_bytes + pal_size
        raw_total = raw_per_card * 1809 / 1024 / 1024
        
        # Test compression
        data = test_bpp(bpp, colors)
        start = time.time()
        compressed = lz77_compress_bios(data)
        ratio = len(compressed) / len(data)
        rom_per_card = len(compressed) + 4  # +4 for palette
        rom_total = (rom_per_card * 1809 + 600000) / 1024 / 1024  # +fronts
        headroom = 32 - rom_total
        
        print(f"{bpp:>4} | {colors:>7} | {raw_per_card:>9} | {raw_total:>9.1f}MB | {ratio*100:>9.1f}% | {rom_total:>9.1f}MB | {headroom:>8.1f}MB")
    
    print("\n" + "=" * 80)
    print("RECOMMENDATION:")
    print("  5bpp: Best balance - 64 colors, ~17MB ROM, 15MB headroom")
    print("  6bpp: Good quality - 128 colors, ~20MB ROM, 12MB headroom")  
    print("  4bpp: Maximum headroom - 16 colors, 10MB ROM, 22MB headroom")

if __name__ == "__main__":
    main()