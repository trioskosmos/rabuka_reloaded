#ifndef JAGUAR_ROMDATA_H
#define JAGUAR_ROMDATA_H

/* ROM-resident data blobs (produced by pack.py from engine_c_wip/src). */
extern const unsigned char cards_bin[];
extern const unsigned long cards_bin_len;
extern const unsigned char abstr_bin[];
extern const unsigned long abstr_bin_len;
extern const unsigned char gen_data_jag_bin[];
extern const unsigned long gen_data_jag_len;

/* ROM pointer tables into the two NUL-terminated string blobs.
 * The shell assigns these to the engine's g_card_strings_rom /
 * g_strings_rom pointer variables (data.c's RB_ROM_STRINGS externs). */
extern const char * const card_ptrs_table[];
extern const char * const abstr_ptrs_table[];

/* Embedded by engine_c_wip/src/core/generated/bytecode_blob.c */
extern const unsigned char RBKA_BYTECODE[];
extern const unsigned long RBKA_BYTECODE_LEN;

#endif
