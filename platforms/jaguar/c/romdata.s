/* ROM-embedded data tables, linked into the cart image (not RAM).
 * All blobs are read byte-wise by the engine (le16/le32 readers in
 * core/data.c, rb_le16_is/rb_le32_is in gen_data_rom.c), so the
 * big-endian m68k needs no swap: the bytes are the bytes, wherever
 * they live.
 *
 * The *_len symbols are real 4-byte objects (not `.set` absolutes):
 * C's `extern const unsigned long x` loads from &x, so an absolute
 * symbol would load from the wrong address.
 */
.section .romdata, "a"

.global cards_bin
.global cards_bin_len
.global abstr_bin
.global abstr_bin_len
.global gen_data_jag_bin
.global gen_data_jag_len

cards_bin:
.incbin "cards_jag.bin"
cards_bin_end:

abstr_bin:
.incbin "abstr_jag.bin"
abstr_bin_end:

gen_data_jag_bin:
.incbin "gen_data_jag.bin"
gen_data_jag_end:

cards_bin_len:
.long cards_bin_end - cards_bin

abstr_bin_len:
.long abstr_bin_end - abstr_bin

gen_data_jag_len:
.long gen_data_jag_end - gen_data_jag_bin

/* ROM-resident pointer tables for the string databases (RB_ROM_STRINGS).
 * The shell's g_card_strings_rom / g_strings_rom pointer variables
 * point HERE; the engine indexes them as the live string tables. */
.include "card_ptrs.inc"
.include "abstr_ptrs.inc"
