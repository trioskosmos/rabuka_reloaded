#include "deck_parser.h"
#include <stdio.h>
#include <string.h>

static int failures;

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

static void test_line_formats(void)
{
    char card[128];
    uint8_t quantity = 0;
    CHECK(rb_parse_deck_line("PL!-sd1-010-SD x 2", card, sizeof(card), &quantity),
          "card-first quantity parses");
    CHECK_EQ(quantity, 2, "card-first quantity is preserved");
    CHECK(!strcmp(card, "PL!-sd1-010-SD"), "card-first identifier is normalized");
    CHECK(rb_parse_deck_line("3 × PL!HS-bp1-019-L", card, sizeof(card), &quantity),
          "unicode multiplication separator parses");
    CHECK_EQ(quantity, 3, "unicode quantity is preserved");
    CHECK(rb_parse_deck_line("PL!N-bp7-030-LX4", card, sizeof(card), &quantity),
          "separator without spaces parses");
    CHECK_EQ(quantity, 4, "separator quantity is preserved");
    CHECK(rb_parse_deck_line("  PL!-sd1-019-SD # comment", card, sizeof(card), &quantity),
          "inline hash comment is ignored");
    CHECK(rb_parse_deck_line("PL!-sd1-019-SD // comment", card, sizeof(card), &quantity),
          "inline slash comment is ignored");
    CHECK(!rb_parse_deck_line("0 x PL!-sd1-019-SD", card, sizeof(card), &quantity),
          "zero quantity is rejected");
    CHECK(!rb_parse_deck_line("PL!-sd1-019-SD x nope", card, sizeof(card), &quantity),
          "invalid quantity is rejected");
}

static void test_content_and_html(void)
{
    const char *content =
        "<table><tr><td>2 x PL!-sd1-010-SD</td></tr>"
        "<tr><td>PL!HS-bp1-019-L</td></tr>"
        "<tr><td>LL-E-001-SD × 2</td></tr></table>";
    char **cards = NULL;
    size_t count = 0;
    CHECK_EQ(rb_parse_deck_content(content, &cards, &count), 0, "HTML deck content parses");
    CHECK_EQ(count, 5, "HTML quantities expand in source order");
    if (count == 5) {
        CHECK(!strcmp(cards[0], "PL!-sd1-010-SD"), "first expanded card is source first");
        CHECK(!strcmp(cards[1], "PL!-sd1-010-SD"), "first quantity expands in place");
        CHECK(!strcmp(cards[2], "PL!HS-bp1-019-L"), "bare card follows quantity");
        CHECK(!strcmp(cards[3], "LL-E-001-SD"), "unicode quantity card follows");
        CHECK(!strcmp(cards[4], "LL-E-001-SD"), "unicode quantity expands in place");
    }
    rb_deck_card_numbers_free(cards, count);
    cards = NULL;
    count = 123;
    CHECK_EQ(rb_parse_deck_content(NULL, &cards, &count), -1, "null content fails");
    CHECK(cards == NULL && count == 0, "failed content parse initializes empty output");
}

static void test_file_status_and_expansion(void)
{
    const char *path = "deck_parser_fixture.txt";
    FILE *file = fopen(path, "wb");
    CHECK(file != NULL, "temporary deck fixture opens");
    if (!file) return;
    fputs("2 x PL!-sd1-010-SD\n", file);
    fputs("PL!HS-bp1-019-L\n", file);
    fputs("# ignored\n", file);
    fputs("LL-E-001-SD x 3\n", file);
    fclose(file);
    RbDeckList deck;
    char error[128];
    CHECK_EQ(rb_parse_deck_file_ex(path, &deck, error, sizeof(error)), RB_DECK_OK,
             "deck file parses with status");
    CHECK(deck.name && !strcmp(deck.name, "deck_parser_fixture"),
          "deck file name uses the filename stem");
    CHECK_EQ(deck.count, 3, "deck file preserves entry lines");
    char **cards = NULL;
    size_t count = 0;
    CHECK_EQ(rb_deck_list_to_card_numbers(&deck, &cards, &count), 0,
             "deck list expands to card numbers");
    CHECK_EQ(count, 6, "deck list expansion preserves quantities");
    rb_deck_card_numbers_free(cards, count);
    rb_deck_list_free(&deck);
    remove(path);
    CHECK_EQ(rb_parse_deck_file_ex("missing-parser-fixture.txt", &deck, error, sizeof(error)),
             RB_DECK_EIO, "missing deck file returns I/O status");
    CHECK(error[0] != '\0', "deck file error text is populated");
    CHECK(strcmp(rb_deck_status_str(RB_DECK_EOVERFLOW), "deck overflow") == 0,
          "deck status string is stable");
}

int main(void)
{
    test_line_formats();
    test_content_and_html();
    test_file_status_and_expansion();
    if (failures) return 1;
    printf("ALL DECK PARSER CHECKS PASSED\n");
    return 0;
}
