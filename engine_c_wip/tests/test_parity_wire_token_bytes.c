/* test_parity_wire_token_bytes.c — pins the WIRE-TOKEN BYTES that twelve suites
 * had misspelled, so the next agent cannot silently re-break them.
 *
 * WHY THIS SUITE EXISTS
 * ---------------------
 * The Japanese wire tokens (trigger names, group names, series names) are
 * compared with `strstr`/`strcmp`, which are BYTEWISE. Several glyphs involved
 * have more than one Unicode codepoint that renders almost identically --
 * 開 (U+958B vs U+8177), セ (U+30BB) vs ホ (U+30DB), 虹 (U+8679 vs U+8671 vs
 * U+8666), 蓮 (U+84EE vs U+8236), 咲 (U+54B2) vs サ (U+30B5), 場 (U+5834) vs
 * 獲 (U+7372). A test that types the wrong one does not fail loudly: the
 * comparison simply never matches, and the assertion it guards passes or
 * fails for a reason that has nothing to do with the engine.
 *
 * Every assertion below is made against the ENGINE'S OWN DATA, never against
 * the suite-local macro, so this file cannot drift with the macros it guards.
 * The Rust line that fixes each token's spelling is cited per case.
 *
 * MEASURED DEFECT CLASS. Method: every \xNN / \NNN escape literal in the
 * test sources was decoded and counted against cards/cards.json plus all files
 * under engine/src. Anything with a ZERO corpus count is a candidate; each
 * candidate below was then confirmed by reading its call site.
 *
 *   tests/test_parity_stats_pipeline.c:71        ライブ<開>始時 U+8177 -> U+958B
 *   tests/test_parity_zones_position.c:70        <ホ>ンター     U+30DB -> U+30BB
 *   tests/test_parity_zone_source.c:58           <虦><ケ>咲  -> U+8679 U+30F6
 *   tests/test_parity_cce_gaps.c:2951            虹ヶ<サ>       U+30B5 -> U+54B2
 *   tests/test_parity_jidou_watch_clusters.c:3x  <近>ノ空       U+8FD1 -> U+84EE
 *   tests/test_parity_ability_watch.c:141        登<獲>         U+7372 -> U+5834
 *   tests/test_parity_trigger_paths.c:187        <癛>場         U+767D -> U+767B
 *   tests/test_trigger_paths.c:61                <立><破>       -> U+767B U+5834
 *   tests/test_trigger_paths.c:62                ライブ<悄><启>時 -> U+958B
 *   tests/test_parity_draw_delta.c:484           虹ヶ<サ>       U+30B5 -> U+54B2
 *   tests/test_parity_yell.c:229                 <莲>ノ空       U+82B2 -> U+84EE
 *   tests/test_parity_score_per_card2.c:219      虹<ケ><サ><ク> -> U+8679 U+30F6 U+54B2
 *   tests/test_parity_gain_delta.c:286           <虛><ノ>咲学園  -> 虹ヶ咲学園
 *
 * DELIBERATE near-miss literals that a sweep must NOT "fix" are asserted here
 * too, so the next sweep leaves them alone:
 *   tests/test_audit_zones.c:938         U+8671 虹ヶ咲 (wrong-spelling control)
 *   tests/test_parity_blade_under.c:198  時時       (TRIG_CONSTANT_WRONG control)
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

/* zones.c owns this and rabuka.h does not declare it yet; declared locally the
 * same way tests/test_parity_zones_position.c and src/turn/triggers.c do. */
int rb_check_trigger_position(const char *triggers, int card_position);

static int failures;

/* ── the canonical tokens, one definition each ─────────────────────────── */

/* Rust engine/src/triggers.rs and turn/trigger_scan.rs live-start token. Every
 * occurrence in cards/cards.json, src/core/generated/bytecode_blob.c and every
 * file under engine/src is U+958B. */
#define TOK_LIVE_START "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE9\x96\x8B\xE5\xA7\x8B\xE6\x99\x82"

/* Rust engine/src/core/zones.rs:169 `trig.contains("センター")`; the C twin is
 * src/core/zones.c:157 inside rb_check_trigger_position. */
#define TOK_CENTER      "\xE3\x82\xBB\xE3\x83\xB3\xE3\x82\xBF\xE3\x83\xBC"
#define TOK_LEFT        "\xE5\xB7\xA6\xE3\x82\xB5\xE3\x82\xA4\xE3\x83\x89"
#define TOK_RIGHT       "\xE5\x8F\xB3\xE3\x82\xB5\xE3\x82\xA4\xE3\x83\x89"

/* Rust engine/src/core/card.rs map_series_to_group returns these two; the C
 * twin is src/core/card.c:367-376 rb_card_series_to_group. */
#define GROUP_NIJI      "\xE8\x99\xB9\xE3\x83\xB6\xE5\x92\xB2"   /* U+8679 U+30F6 U+54B2 */
#define GROUP_HASU      "\xE8\x93\xAE\xE3\x83\x8E\xE7\xA9\xBA"   /* U+84EE U+30CE U+7A7A */

/* Rust engine/src/triggers.rs:22-23 and turn/trigger_scan.rs:12 -- 登場. */
#define TOK_DEBUT       "\xE7\x99\xBB\xE5\xA0\xB4"               /* U+767B U+5834 */

/* The Nijigasaki series prefix exactly as src/core/card.c:371 spells it.
 *
 * NOTE the first six bytes: E3 83 A9 E3 83 96 E3 83 A9 E3 82 A4 E3 83 96 is
 * ラ ブ ラ イ ブ, NOT the linguistically expected ラブライブ (which would
 * start E3 83 A9 E3 82 A4 E3 83 96). That transposition is in the upstream card
 * data, so it appears in cards/cards.json, in src/core/card.c:371 AND in
 * engine/src/core/card.rs map_series_to_group. Both engines agree on it, so it
 * is DATA, not a defect -- but a test literal that "corrects" it matches
 * nothing and the strstr premise goes vacuously false. The same suffix is what
 * tests/test_parity_cce_gaps.c and tests/test_parity_gain_delta.c depend on. */
#define SERIES_NIJI     "\xE3\x83\xA9\xE3\x83\x96\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96" \
                        "\xEF\xBC\x81\xE8\x99\xB9\xE3\x83\xB6\xE5\x92\xB2"

/* Wrong spellings this suite asserts are NOT the canonical bytes. */
#define BAD_NIJI_FULLKE "\xE8\x99\xB1\xE3\x83\xB6\xE5\x92\xB2"   /* U+8671 */
#define BAD_CENTER_HO   "\xE3\x83\x9B\xE3\x83\xB3\xE3\x82\xBF\xE3\x83\xBC"
#define BAD_HASU_SIMPL  "\xE8\x8E\xB2\xE3\x83\x8E\xE7\xA9\xBA"   /* U+82B2 */

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
        FLAT_VERDICT(); \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* ── 1. the CENTER arm of check_trigger_position ─────────────────────────
 * Rust engine/src/core/zones.rs:163-171. The C engine (zones.c:153) was
 * already correct; the SUITE's TRIG_CENTER was not. This asserts the engine's
 * own behaviour, so the token stays pinned even if a suite macro is edited. */
static void test_center_token_gate(void)
{
    CHECK(rb_check_trigger_position(TOK_CENTER, 1) == 1,
          "zones.rs:169 the C center arm admits a center member");
    CHECK(rb_check_trigger_position(TOK_CENTER, 0) == 0,
          "zones.rs:169 the C center arm refuses a left-side member");
    CHECK(rb_check_trigger_position(TOK_CENTER, 2) == 0,
          "zones.rs:169 the C center arm refuses a right-side member");
    CHECK(rb_check_trigger_position(TOK_LEFT, 0) == 1,
          "zones.rs:163 the left arm admits a left-side member");
    CHECK(rb_check_trigger_position(TOK_RIGHT, 2) == 1,
          "zones.rs:166 the right arm admits a right-side member");
    /* The guard against re-introducing the U+30DB typo. */
    CHECK(rb_check_trigger_position(BAD_CENTER_HO, 0) == 1,
          "the U+30DB misspelling does not gate and must never be a suite literal");
}

/* Count the abilities of `cid` whose decoded trigger string contains `tok`. */
static int count_trigger_hits(int cid, const char *tok)
{
    int hits = 0;
    int nab = rb_card_num_abilities((uint32_t)cid);
    for (int a = 0; a < nab; a++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        if (ab.triggers && strstr(ab.triggers, tok)) hits++;
        rb_free_ability(&ab);
    }
    return hits;
}

/* ── 2. the live-start trigger token against real decoded data ────────────
 * Rust engine/tests/helpers/mod.rs:114-141 fire_trigger matches
 * `a.triggers.as_deref() == Some(trig)`; the C predicate is
 * src/turn/triggers.c:76 rb_trigger_is, a plain strstr. Every card below
 * really does print this token in the shipped bytecode blob. */
static void test_live_start_token_against_data(void)
{
    static const char *cards[] = {
        "PL!S-pb1-020-L",   /* heart04 total >= 10 -> +2 score      */
        "PL!SP-bp5-026-L",  /* Liella! hearts >= 11 -> +1          */
        "PL!N-bp4-028-L",   /* distinct-name waitroom lives        */
        "PL!N-sd1-028-SD",  /* blade total >= 10 -> +1             */
        "PL!N-bp1-027-L",   /* one score per unique heart colour   */
        "PL!SP-pb2-045-L"   /* only the >=4-heart member counts    */
    };
    for (size_t i = 0; i < sizeof cards / sizeof cards[0]; i++) {
        int cid = rb_find_card_by_no(cards[i]);
        CHECK(cid >= 0, "the live-start fixture card exists in the blob");
        if (cid < 0) continue;
        CHECK(count_trigger_hits(cid, TOK_LIVE_START) > 0,
              "the decoded ability really prints the canonical live-start token");
    }
}

/* ── 3. the DEBUT trigger token against real decoded data ──────────────── */
static void test_debut_token_against_data(void)
{
    int cid = rb_find_card_by_no("PL!HS-bp5-011-N");
    CHECK(cid >= 0, "the debut fixture card exists in the blob");
    if (cid < 0) return;
    CHECK(count_trigger_hits(cid, TOK_DEBUT) > 0,
          "the decoded ability really prints the canonical 登場 token");
}

/* ── 4. the derived GROUP names ──────────────────────────────────────────
 * Rust engine/src/core/card.rs map_series_to_group. The C twin is
 * src/core/card.c:367-376 rb_card_series_to_group.
 *
 * Driven by REAL card data on purpose. An earlier revision of this file
 * hand-typed the full series strings, which is exactly the transcription
 * hazard this suite exists to catch -- a re-typed series can differ from the
 * corpus by one byte and then assert nothing. Reading the series back off a
 * card removes the copy step entirely. */
static void assert_derived_group(const char *card_no, const char *want,
                                 const char *label)
{
    int cid = rb_find_card_by_no(card_no);
    CHECK(cid >= 0, "the group probe card exists");
    if (cid < 0) return;
    Card c;
    memset(&c, 0, sizeof c);
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) {
        CHECK(0, label);
        return;
    }
    const char *s = (c.series_idx != UINT16_MAX) ? rb_card_string(c.series_idx) : NULL;
    CHECK(s && strcmp(rb_card_series_to_group(s), want) == 0, label);
    rb_free_card(&c);
}

static void test_group_names_are_canonical(void)
{
    assert_derived_group("PL!N-bp1-025-L", GROUP_NIJI,
        "card.rs map_series_to_group derives 虹ヶ咲 with U+8679 U+30F6 U+54B2");
    assert_derived_group("PL!HS-bp1-001-L", GROUP_HASU,
        "card.rs map_series_to_group derives 蓮ノ空 with U+84EE U+30CE U+7A7A");
    /* The deliberate wrong-spelling control used by test_audit_zones.c:938. */
    CHECK(strcmp(GROUP_NIJI, BAD_NIJI_FULLKE) != 0,
          "the U+8671 full-size-ke spelling is a DIFFERENT string and stays that way");
    CHECK(strcmp(GROUP_HASU, BAD_HASU_SIMPL) != 0,
          "the U+82B2 simplified 莲 spelling is a DIFFERENT string and stays that way");
}

/* ── 5. the group predicate actually matches a real card ───────────────────
 * This is the assertion the U+8666/U+30B1 literal silently destroyed. */
static void test_group_predicate_matches_real_card(void)
{
    int niji = rb_find_card_by_no("PL!N-bp1-025-L");
    CHECK(niji >= 0, "a real Nijigasaki live card exists");
    if (niji >= 0) {
        CHECK(rb_card_matches_group_str(niji, GROUP_NIJI),
              "the canonical 虹ヶ咲 literal matches a real Nijigasaki card");
        CHECK(!rb_card_matches_group_str(niji, BAD_NIJI_FULLKE),
              "the U+8671 misspelling does NOT match and is not a usable literal");
    }
    int hasu = rb_find_card_by_no("PL!HS-bp1-001-L");
    if (hasu >= 0) {
        CHECK(rb_card_matches_group_str(hasu, GROUP_HASU),
              "the canonical 蓮ノ空 literal matches a real Hasunosora card");
        CHECK(!rb_card_matches_group_str(hasu, BAD_HASU_SIMPL),
              "the U+82B2 misspelling does NOT match and is not a usable literal");
    }
}

/* ── 6. the SERIES substring premise (cce_gaps / gain_delta) ─────────────── */
static void test_niji_series_substring(void)
{
    int cid = rb_find_card_by_no("PL!N-bp1-025-L");
    CHECK(cid >= 0, "the Nijigasaki series probe card exists");
    if (cid < 0) return;
    Card c;
    memset(&c, 0, sizeof c);
    int ok = rb_decode_card_by_index((uint32_t)cid, &c);
    CHECK(ok, "the series probe card decodes");
    if (!ok) return;
    const char *s = (c.series_idx != UINT16_MAX) ? rb_card_string(c.series_idx) : NULL;
    CHECK(s && strstr(s, SERIES_NIJI) != NULL,
          "card.rs the canonical series prefix really is a substring of a real series");
    CHECK(s && strcmp(rb_card_series_to_group(s), GROUP_NIJI) == 0,
          "card.rs the probe card's series derives the canonical 虹ヶ咲 group");
    rb_free_card(&c);
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    test_center_token_gate();
    test_live_start_token_against_data();
    test_debut_token_against_data();
    test_group_names_are_canonical();
    test_group_predicate_matches_real_card();
    test_niji_series_substring();
    rb_unload();
    if (failures) {
        fprintf(stderr, "WIRE TOKEN BYTES: %d failure(s)\n", failures);
        return 1;
    }
    printf("ALL WIRE TOKEN BYTE CHECKS PASSED\n");
    return 0;
}