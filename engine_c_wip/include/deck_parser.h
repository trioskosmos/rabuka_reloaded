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

typedef enum {
    RB_DECK_OK = 0,
    RB_DECK_EINVAL,
    RB_DECK_EIO,
    RB_DECK_EFORMAT,
    RB_DECK_EOVERFLOW,
    RB_DECK_ENOMEM
} RbDeckStatus;

int rb_normalize_card_no(const char *raw, char *out, size_t out_size);
int rb_parse_deck_line(const char *line, char *card_no, size_t card_no_size, uint8_t *quantity);
int rb_parse_deck_content(const char *content, char ***card_numbers, size_t *card_count);
int rb_parse_deck_file(const char *path, RbDeckList *out);
RbDeckStatus rb_parse_deck_file_ex(const char *path, RbDeckList *out,
                                   char *error, size_t error_size);
const char *rb_deck_status_str(RbDeckStatus status);
int rb_deck_list_to_card_numbers(const RbDeckList *deck, char ***out, size_t *out_count);
void rb_deck_card_numbers_free(char **cards, size_t count);
void rb_deck_list_free(RbDeckList *deck);

#endif
