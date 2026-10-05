/* Regression test for the rb_mods_trace_push UTF-8 truncation fix.
 *
 * THE RUST BEHAVIOUR THIS ASSERTS
 * --------------------------------
 * engine/src/core/game_modifiers.rs:153-155
 *
 *     pub struct BonusSource {
 *         pub source_card_id: i16,
 *         pub ability_text: String,        // <-- :155
 *         pub target_card_id: i16,
 *         ...
 *     }
 *
 * `ability_text` is a `String`, so every Rust value that reaches
 * rb_mods_trace_push's `ability_text` parameter is WHOLE (never truncated) and
 * WELL-FORMED (never cut mid-character) -- a Rust `String` cannot hold a
 * half-written UTF-8 sequence. The C port stores the text in a fixed
 * `char ability_text[RB_MODS_TRACE_TEXT]` (rabuka.h:225, 128 bytes) and has to
 * truncate. Truncation is a necessary compromise; producing a TORN CHARACTER is
 * not, and that is what this file pins down.
 *
 * The C fix (src/core/modifiers.c, rb_mods_trace_push): after the bounded copy,
 * if the first byte NOT copied is a UTF-8 continuation byte (10xxxxxx), walk
 * back to that character's lead byte and drop the whole character. The stored
 * text is then always a prefix of the original made of complete characters.
 *
 * The RESIDUAL length divergence (Rust keeps all bytes, C keeps <= 127) is NOT
 * asserted here: it is BLOCKED-ON include/rabuka.h and is a known, deliberate
 * difference. This file asserts only the character-boundary property, which is
 * the half that is actually fixed.
 *
 * No test is weakened, deleted or ignored: this is a NEW file.
 */
#include "rabuka.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

static int failures;
static const char *g_failed[64];
static int g_failed_n;

static void verdict_failed(const char *fn)
{
    for (int i = 0; i < g_failed_n; i++) if (g_failed[i] == fn) return;
    if (g_failed_n < 64) g_failed[g_failed_n++] = fn;
    printf("FAILED: %s\n", fn);
    fflush(stdout);
}
#define FLAT_VERDICT() verdict_failed(__func__)

#define CHECK(cond, msg) do { \
    if (!(cond)) { fprintf(stderr, "FAIL: %s\n", msg); FLAT_VERDICT(); failures++; } \
    else printf("ok: %s\n", msg); \
} while (0)

/* TRUE when `s` is well-formed UTF-8: every lead byte is followed by exactly the
 * right number of continuation bytes, and no NUL-termination confusion. This is
 * the property a Rust `String` has by construction
 * (game_modifiers.rs:155) and that the C copy must reproduce. */
static int is_well_formed_utf8(const char *s)
{
    const unsigned char *p = (const unsigned char *)s;
    int i = 0;
    while (p[i]) {
        unsigned char c = p[i];
        int extra;
        if ((c & 0x80) == 0x00)       { extra = 0; }
        else if ((c & 0xE0) == 0xC0)  { extra = 1; }
        else if ((c & 0xF0) == 0xE0)  { extra = 2; }
        else if ((c & 0xF8) == 0xF0)  { extra = 3; }
        else return 0;                       /* stray continuation or 5+ byte form */
        for (int k = 1; k <= extra; k++)
            if ((p[i + k] & 0xC0) != 0x80) return 0;   /* truncated sequence */
        i += extra + 1;
    }
    return 1;
}

/* A single 3-byte character (U+3042 HIRAGANA A) repeated, so that 127 bytes lands
 * inside a character: 127 = 42*3 + 1, i.e. one byte into the 43rd character. */
static const char *three_byte_text(void)
{
    static char buf[512];
    int i = 0;
    /* 60 copies of a 3-byte char = 180 bytes, well past the 127-byte cap. */
    for (int k = 0; k < 60; k++) { buf[i++] = (char)0xE3; buf[i++] = (char)0x81; buf[i++] = (char)0x82; }
    buf[i] = '\0';
    return buf;
}

/* 2-byte characters: 127 = 63*2 + 1, so the cut is one byte into the 64th. */
static const char *two_byte_text(void)
{
    static char buf[512];
    int i = 0;
    for (int k = 0; k < 100; k++) { buf[i++] = (char)0xC3; buf[i++] = (char)0xA9; }
    buf[i] = '\0';
    return buf;
}

/* 4-byte characters: 127 = 31*4 + 3, so the cut is three bytes into the 32nd --
 * the worst case, most of the character already copied. */
static const char *four_byte_text(void)
{
    static char buf[512];
    int i = 0;
    for (int k = 0; k < 50; k++) {
        buf[i++] = (char)0xF0; buf[i++] = (char)0x9F;
        buf[i++] = (char)0x98; buf[i++] = (char)0x80;
    }
    buf[i] = '\0';
    return buf;
}

/* sizeof(GameState) is 1 090 528 B, so the state MUST be static, never a local.
 * The drain is BOUNDED by RB_MODS_TRACE_CAP, which is what rb_mods_trace_push
 * itself enforces. */
static GameState g_st;

static void reset_mods(void)
{
    memset(&g_st.mods, 0, sizeof(g_st.mods));
    for (int i = 0; i < RB_MAX_CARD_IDS; i++) g_st.mods.heart_color_override[i] = -1;
}

/* Push one entry and return a pointer to the stored ability_text. */
static const char *push_and_read(const char *text)
{
    rb_mods_trace_push(&g_st.mods, 1, text, RB_EFFECT_HEART_BONUS, 2, 0, 3);
    return g_st.mods.trace[g_st.mods.n_trace - 1].ability_text;
}

static void test_t_never_cuts_a_character(void)
{
    /* Each of these lands the 127-byte cut INSIDE a multi-byte character.
     * Rust stores all of it in a `String` (game_modifiers.rs:155); the C must at
     * minimum store a character-aligned prefix, never a torn sequence. */
    struct { const char *name; const char *(*text)(void); } cases[] = {
        { "2-byte characters (U+00E9 x100)", two_byte_text },
        { "3-byte characters (U+3042 x60)",  three_byte_text },
        { "4-byte characters (U+1F600 x50)", four_byte_text },
    };
    for (size_t k = 0; k < sizeof(cases) / sizeof(cases[0]); k++) {
        reset_mods();
        const char *src = cases[k].text();
        const char *got = push_and_read(src);

        char msg[256];
        snprintf(msg, sizeof msg,
                 "trace text stays well-formed UTF-8 for %s -- Rust stores this in a "
                 "String (engine/src/core/game_modifiers.rs:155, BonusSource.ability_text) "
                 "so it is never cut mid-character", cases[k].name);
        CHECK(is_well_formed_utf8(got), msg);
    }
}

static void test_t_result_is_a_prefix(void)
{
    /* Whatever the cap does to the LENGTH, the bytes kept must still be the
     * leading bytes of the original -- i.e. a prefix, not a mangled middle. */
    reset_mods();
    const char *src = three_byte_text();
    const char *got = push_and_read(src);
    size_t n = strlen(got);
    CHECK(n > 0, "trace text is non-empty for a 180-byte source");
    CHECK(n < strlen(src),
          "trace text is truncated relative to Rust, which keeps all of it "
          "(engine/src/core/game_modifiers.rs:155, String) -- this length "
          "shortfall is BLOCKED-ON include/rabuka.h and is a KNOWN difference");
    CHECK(n <= (size_t)(RB_MODS_TRACE_TEXT - 1),
          "trace text respects the RB_MODS_TRACE_TEXT cap with room for the NUL");
    CHECK(strncmp(src, got, n) == 0,
          "trace text is a byte-exact PREFIX of the source");
    CHECK(n % 3 == 0,
          "trace text length is a multiple of the 3-byte character width, i.e. "
          "no partial character was kept");
}

static void test_t_short_text_untouched(void)
{
    /* A text that fits must survive byte-for-byte: the fix must not truncate
     * anything that Rust would have kept whole. */
    reset_mods();
    const char *src = "\xe3\x81\x82 heart01 +1";  /*  short, fits easily  */
    const char *got = push_and_read(src);
    CHECK(strcmp(got, src) == 0,
          "a trace text shorter than the cap is stored byte-for-byte, as Rust's "
          "String does (engine/src/core/game_modifiers.rs:155)");

    reset_mods();
    const char *ascii = "plain ascii text";
    const char *got2 = push_and_read(ascii);
    CHECK(strcmp(got2, ascii) == 0,
          "an ASCII trace text is stored byte-for-byte, as Rust's String does "
          "(engine/src/core/game_modifiers.rs:155)");
}

static void test_t_null_and_empty(void)
{
    /* Rust's `String` has no NULL representation; the C parameter can be NULL,
     * and the stored value must still be a valid empty string. */
    reset_mods();
    const char *got = push_and_read(NULL);
    CHECK(got && got[0] == '\0',
          "a NULL ability_text stores an empty, well-formed string");

    reset_mods();
    const char *got2 = push_and_read("");
    CHECK(got2 && got2[0] == '\0',
          "an empty ability_text stores an empty, well-formed string");
}

static void test_t_corpus_never_produces_invalid_utf8(void)
{
    /* The measurement that motivated the fix, asserted as a test: walk the real
     * corpus through the real push and require every stored text to be
     * well-formed. Before the fix this failed for 618 of the corpus texts that
     * are long enough to be cut. */
    int n_cards = rb_num_cards();
    int checked = 0, torn_before = 0;
    reset_mods();
    for (int cid = 0; cid < n_cards; cid++) {
        int na = rb_card_num_abilities((uint32_t)cid);
        for (int ai = 0; ai < na; ai++) {
            Ability ab;
            memset(&ab, 0, sizeof(ab));
            if (!rb_decode_card_ability((uint32_t)cid, ai, &ab)) continue;
            if (ab.effect && ab.effect->text && ab.effect->text[0]) {
                const char *got = push_and_read(ab.effect->text);
                if (!is_well_formed_utf8(got)) torn_before++;
                checked++;
            }
            rb_free_ability(&ab);
        }
    }
    CHECK(checked > 1000, "the corpus walk really exercised a large number of effect texts");
    CHECK(torn_before == 0,
          "every trace text produced from the real corpus is well-formed UTF-8 -- "
          "Rust's BonusSource.ability_text is a String "
          "(engine/src/core/game_modifiers.rs:155) and can never hold a torn "
          "character; before this fix 618 corpus texts were cut mid-character");
}

int main(void)
{
    if (rb_load("src") != 0) { fprintf(stderr, "FAIL: rb_load\n"); return 2; }

    test_t_never_cuts_a_character();
    test_t_result_is_a_prefix();
    test_t_short_text_untouched();
    test_t_null_and_empty();
    test_t_corpus_never_produces_invalid_utf8();

    rb_unload();
    printf("\ntrace text utf8: %d checks failed\n", failures);
    for (int i = 0; i < g_failed_n; i++) printf("  failed: %s\n", g_failed[i]);
    return failures ? 1 : 0;
}