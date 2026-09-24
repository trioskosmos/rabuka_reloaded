#ifndef RABUKA_DECK_PARSER_H
#define RABUKA_DECK_PARSER_H

#include <stddef.h>
#include <stdint.h>

typedef struct {
    char *card_no;
    uint8_t quantity;
} RbDeckEntry;

typedef struct {
    char *name;
    RbDeckEntry *entries;
    size_t count;
} RbDeckList;

int rb_normalize_card_no(const char *raw, char *out, size_t out_size);
int rb_parse_deck_line(const char *line, char *card_no, size_t card_no_size, uint8_t *quantity);
int rb_parse_deck_content(const char *content, char ***card_numbers, size_t *card_count);
int rb_parse_deck_file(const char *path, RbDeckList *out);
void rb_deck_list_free(RbDeckList *deck);

#endif
