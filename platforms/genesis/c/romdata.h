#ifndef GENESIS_ROMDATA_H
#define GENESIS_ROMDATA_H

/* ROM-resident data blobs (produced by pack.py from engine_c/src/*.bin). */
extern const unsigned char cards_bin[];
extern const unsigned long cards_bin_len;
extern const unsigned char abstr_bin[];
extern const unsigned long abstr_bin_len;

/* Already embedded by engine_c/src/core/generated/bytecode_blob.c */
extern const unsigned char RBKA_BYTECODE[];
extern const unsigned long RBKA_BYTECODE_LEN;

#endif
