#include "deck_parser.h"
#include "rabuka.h"
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

/* ── rb_normalize_card_no vs DeckParser::normalize_card_no (deck_parser.rs:200-208)
 *
 * Rust REWRITES every ASCII '+' and '!' to fullwidth (U+FF0B / U+FF01) and only
 * then strips a TRAILING one. The C strips trailing ASCII first and trailing
 * fullwidth second, leaving a non-trailing ASCII '+' or '!' byte-for-byte.
 * The two therefore disagree on exactly one class of input: a '+' or '!' that
 * is present but NOT trailing.
 *
 * That disagreement is INERT for lookup, and this test is the proof rather
 * than the assertion: card_no_codepoint (src/core/card.c:316-326) folds
 * U+FF0B -> '+' and U+FF01 -> '!' , and rb_card_normalize_no applies it to
 * every lookup key, so both spellings normalise to the same key before
 * rb_card_get_card_id ever sees them.
 */
static void test_normalize_card_no(void)
{
    char out[128];

    CHECK_EQ(rb_normalize_card_no("PL!-bp3-004-R", out, sizeof(out)), 0,
             "plain card number normalises");
    CHECK(strcmp(out, "PL!-bp3-004-R") == 0, "plain card number is unchanged");

    CHECK_EQ(rb_normalize_card_no("PL!-bp3-004-R+", out, sizeof(out)), 0,
             "trailing ASCII plus normalises");
    CHECK(strcmp(out, "PL!-bp3-004-R") == 0, "trailing ASCII plus is stripped");

    CHECK_EQ(rb_normalize_card_no("PL!-bp3-004-R!", out, sizeof(out)), 0,
             "trailing ASCII bang normalises");
    CHECK(strcmp(out, "PL!-bp3-004-R") == 0, "trailing ASCII bang is stripped");

    CHECK_EQ(rb_normalize_card_no("PL!-bp3-004-R\xef\xbc\x8b", out, sizeof(out)), 0,
             "trailing fullwidth plus (U+FF0B) normalises");
    CHECK(strcmp(out, "PL!-bp3-004-R") == 0, "trailing fullwidth plus is stripped");

    CHECK_EQ(rb_normalize_card_no("PL!-bp3-004-R\xef\xbc\x81", out, sizeof(out)), 0,
             "trailing fullwidth bang (U+FF01) normalises");
    CHECK(strcmp(out, "PL!-bp3-004-R") == 0, "trailing fullwidth bang is stripped");

    /* Interior punctuation: the C leaves ASCII alone, Rust would widen it. */
    CHECK_EQ(rb_normalize_card_no("PL!-bp3-004-R+S", out, sizeof(out)), 0,
             "interior ASCII plus still normalises to a non-empty string");
    CHECK(strcmp(out, "PL!-bp3-004-R+S") == 0,
          "interior ASCII plus is left verbatim (Rust would widen it to U+FF0B)");

    /* Whitespace-trimming first: rust trims nothing here (parse_line already
     * trimmed the whole line), so a raw leading space is part of the token and
     * the leading whitespace is dropped by the C. Both end at the same card. */
    CHECK_EQ(rb_normalize_card_no("  PL!-bp3-004-R  ", out, sizeof(out)), 0,
             "surrounding whitespace is trimmed");
    CHECK(strcmp(out, "PL!-bp3-004-R") == 0, "trimming leaves the card number");

    CHECK(rb_normalize_card_no(NULL, out, sizeof(out)) < 0, "NULL input is rejected");
    CHECK(rb_normalize_card_no("   ", out, sizeof(out)) < 0, "blank input is rejected");
    CHECK(rb_normalize_card_no("+", out, sizeof(out)) < 0,
          "a token that is only a plus normalises to empty and is rejected");
    CHECK(rb_normalize_card_no("PL!-bp3-004-R", out, 4) < 0,
          "a buffer too small for the token is rejected, not truncated");

    /* Inertness, proved rather than asserted: build the string the way RUST's
     * normalize_card_no would have (interior '+' widened to U+FF0B), run BOTH
     * through rb_card_normalize_no, and require the same lookup key. */
    {
        char manual[128];
        char canon[128];
        const char *ascii_form = "PL!-bp3-004-R+S";
        memcpy(manual, "PL!-bp3-004-R", 13);
        manual[13] = (char)0xef; manual[14] = (char)0xbc; manual[15] = (char)0x8b;
        manual[16] = 'S';
        manual[17] = 0;
        rb_card_normalize_no(ascii_form, out, sizeof out);
        rb_card_normalize_no(manual, canon, sizeof canon);
        if (strcmp(out, canon) != 0) {
            char h1[64], h2[64];
            size_t a, b;
            for (a = 0; a < sizeof h1 - 1 && out[a]; a++)
                h1[a] = out[a] < 0x20 || out[a] > 0x7e ? '?' : out[a];
            h1[a] = 0;
            for (b = 0; b < sizeof h2 - 1 && canon[b]; b++)
                h2[b] = canon[b] < 0x20 || canon[b] > 0x7e ? '?' : canon[b];
            h2[b] = 0;
            fprintf(stderr, "FAIL: ASCII '+' and U+FF0B normalise to the SAME lookup key "
                            "(card.c:316-326)\n  ascii     -> \"%s\" (%u bytes)\n"
                            "  fullwidth -> \"%s\" (%u bytes)\n",
                    h1, (unsigned)strlen(out), h2, (unsigned)strlen(canon));
            failures++;
        }
    }
}

/* ── parse_line (deck_parser.rs:212-235) divergences, pinned as divergences.
 *
 * Rust splits on the EXACT substring " x " (lowercase x, one space each side)
 * and skips only lines that start with "//". The C scans for any 'x', 'X' or
 * U+00D7 at any offset and strips a "//" or "#" comment from anywhere in the
 * line. Three consequences, each asserted below so nobody later reads the C as
 * a faithful port of parse_line:
 *   1. "x" written as X, as ×, or without spaces is accepted by the C and
 *      rejected by Rust (Rust then requires no space in the line, fails, and
 *      returns None).
 *   2. An inline "#" comment is stripped by the C; Rust keeps it, which makes
 *      the line contain a space and so fail the bare-card_no test.
 *   3. A quantity of 0 parses as u8 in Rust and yields Some((card_no, 0)) — an
 *      entry with zero copies. The C rejects the line outright, so the entry is
 *      absent. Downstream this is invisible (zero copies expand to nothing), but
 *      DeckList.entries differs, so rb_deck_list_to_card_numbers input differs.
 */
static void test_parse_line_divergences_from_rust(void)
{
    char card[128];
    uint8_t qty = 0;

    /* (1) U+00D7 multiplier: C accepts. */
    CHECK_EQ(rb_parse_deck_line("3 \xc3\x97 PL!HS-bp1-019-L", card, sizeof card, &qty), 1,
             "U+00D7 multiplier is accepted (Rust's parse_line would return None)");
    CHECK_EQ(qty, 3, "U+00D7 multiplier quantity is 3");
    CHECK(strcmp(card, "PL!HS-bp1-019-L") == 0, "U+00D7 multiplier yields the card number");

    /* (1) uppercase X with no spaces: C accepts. */
    CHECK_EQ(rb_parse_deck_line("PL!-sd1-010-SDX2", card, sizeof card, &qty), 1,
             "'X' with no surrounding spaces is accepted (Rust would return None)");
    CHECK_EQ(qty, 2, "'X' no-space quantity is 2");
    CHECK(strcmp(card, "PL!-sd1-010-SD") == 0, "'X' no-space splits card from quantity");

    /* (2) inline '#' comment: C strips it. */
    CHECK_EQ(rb_parse_deck_line("PL!-sd1-010-SD # fourth copy", card, sizeof card, &qty), 1,
             "an inline '#' comment is stripped (Rust keeps it and rejects the line)");
    CHECK_EQ(qty, 1, "inline '#' comment leaves quantity 1");
    CHECK(strcmp(card, "PL!-sd1-010-SD") == 0, "inline '#' comment leaves the card number");

    /* (3) quantity 0: C rejects the line, Rust returns Some((card_no, 0)). */
    CHECK_EQ(rb_parse_deck_line("0 x PL!-sd1-010-SD", card, sizeof card, &qty), 0,
             "quantity 0 is rejected (Rust returns Some((card_no, 0)) instead)");
    /* 256 overflows u8 in Rust's parse::<u8>() too, so both reject. */
    CHECK_EQ(rb_parse_deck_line("256 x PL!-sd1-010-SD", card, sizeof card, &qty), 0,
             "quantity 256 is rejected by both sides");
    /* Rust's bare-card_no form requires no space at all; a lone card works. */
    CHECK_EQ(rb_parse_deck_line("PL!-sd1-010-SD", card, sizeof card, &qty), 1,
             "a bare card number parses with quantity 1");
    CHECK_EQ(qty, 1, "bare card number quantity is 1");
    /* A token with no '-' is never a card number on either side. */
    CHECK_EQ(rb_parse_deck_line("NOTACARD", card, sizeof card, &qty), 0,
             "a token without '-' is not a card number");
    CHECK_EQ(rb_parse_deck_line("", card, sizeof card, &qty), 0, "an empty line is skipped");
    CHECK_EQ(rb_parse_deck_line("// comment", card, sizeof card, &qty), 0,
             "a '//' comment line is skipped");
}

int main(void)
{
    test_line_formats();
    test_content_and_html();
    test_file_status_and_expansion();
    test_normalize_card_no();
    test_parse_line_divergences_from_rust();
    if (failures) return 1;
    printf("ALL DECK PARSER CHECKS PASSED\n");
    return 0;
}
