/* ROM-embedded data tables, linked into the cart image (not RAM). */
.section .romdata, "a"

.global cards_bin
.global cards_bin_len
.global abstr_bin
.global abstr_bin_len

cards_bin:
.incbin "cards_gen.bin"
cards_bin_end:

abstr_bin:
.incbin "abstr_gen.bin"
abstr_bin_end:

.set cards_bin_len, cards_bin_end - cards_bin
.set abstr_bin_len, abstr_bin_end - abstr_bin

/* ROM-resident pointer tables for the string databases (RB_ROM_STRINGS). */
.include "card_ptrs.inc"
.include "abstr_ptrs.inc"
