/* ROM-embedded data tables, linked into the cart image (not RAM).
 * All blobs are read byte-wise by the engine (le16/le32 readers in
 * core/data.c), so the big-endian m68k needs no swap: the bytes are
 * the bytes, wherever they live. */
.section .romdata, "a"

.global cards_bin
.global cards_bin_len
.global abstr_bin
.global abstr_bin_len

cards_bin:
.incbin "cards_jag.bin"
cards_bin_end:

abstr_bin:
.incbin "abstr_jag.bin"
abstr_bin_end:

.set cards_bin_len, cards_bin_end - cards_bin
.set abstr_bin_len, abstr_bin_end - abstr_bin

/* ROM-resident pointer tables for the string databases (RB_ROM_STRINGS). */
.include "card_ptrs.inc"
.include "abstr_ptrs.inc"
