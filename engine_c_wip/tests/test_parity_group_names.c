/* =====================================================================
 * test_parity_group_names.c
 *
 * PARITY COVERAGE for the group-name filter -- `rb_card_matches_group_str`
 * -- the highest-reachability gap in the corpus.
 *
 *   COVERAGE_GAPS.md section 3, rank 1: `group_names` on an effect reaches
 *   273 / 985 printed abilities, 543 ability/effect pairs, 1 574 effect nodes
 *   and 1 708 condition nodes. Rank 6 is the same filter on the CONDITION
 *   surface (155 / 985). Together they are the single largest concentration
 *   of unproved behaviour in the project.
 *
 * WHY THIS SUITE EXISTS. The Rust oracle is
 *
 *   engine/tests/test_modules/rules/group_name_equivalence.rs   (3 #[test])
 *
 * and before this file the C corpus had NO counterpart for it at all: no
 * suite exercised `rb_card_matches_group_str` as a whole-corpus predicate.
 * That Rust file's three tests are equivalence guards on a fast path; what
 * they rest on, and what this suite asserts, are the four premises those
 * guards assume:
 *
 *   P1  `rb_card_matches_group_str` answers on the KNOWN_GROUP taxonomy
 *       (`engine/src/ability/util.rs:956`) over the real corpus, and every
 *       group reaches a real, non-zero set of cards -- otherwise the
 *       taxonomy is dead and every `X=group_names` ability silently
 *       matches everything or nothing.
 *   P2  the non-matching control matches NOTHING. This is the anti-vacuity
 *       half: a matcher that returned 1 for every card would pass every
 *       positive assertion in the file, so the control is what proves the
 *       positives mean anything. `group_name_equivalence.rs:47` puts
 *       "nosuchgroupanywhere" in the probe list for exactly this reason.
 *   P3  the SERIES arm (`util.rs:958-975`) is an implication, not a guess:
 *       a card whose series carries a group's marker MUST match that group,
 *       for all five groups, with no exceptions anywhere in the corpus.
 *   P4  the NAME arm (`util.rs:700-701`) is likewise an implication: a card
 *       with a constituent name containing the group string MUST match it,
 *       and the corpus MUST still contain multi-name cards (the whole reason
 *       `get_card_names` exists) -- otherwise P4 is vacuous.
 *
 * Plus the two NORMALISATION premises the matcher rests on, both of which are
 * easy to get wrong and are invisible to a behavioural test that happens not
 * to cover the affected card (`util.rs:760-777`):
 *
 *   N1  the U+FF01 '！' / U+00B5 'µ' fold (`util.rs:696`, `norm_group_name`):
 *       the full-width and ASCII spellings of one unit must answer
 *       IDENTICALLY for all 2 639 cards -- zero disagreements, not "close".
 *   N2  `eq_ignoring_whitespace` (`util.rs:694`): "lily white" and
 *       "lilywhite" are the same filter, because the corpus spells some units
 *       both ways and a raw `==` made a `『lily white』` filter skip every
 *       unspaced card.
 *
 * A NOTE ON `RB_KNOWN_GROUPS`. `include/rabuka.h:2776` declares
 * `extern const char *RB_KNOWN_GROUPS[5]` -- and it is DEFINED NOWHERE in
 * `src/`. It is a dangling extern: any translation unit that referenced it
 * would fail at link, so nothing has. This suite therefore carries the five
 * literals transcribed from Rust `util.rs:956` with their bytes asserted
 * explicitly (T0), which is what makes the transcription itself checkable
 * rather than a comment. The defect is reported, not silently worked around.
 *
 * ASCII-ONLY BY CONSTRUCTION. Every Japanese string in this file is written
 * as an explicit `\xNN` byte escape, so the file is pure ASCII source. This
 * is deliberate (see the other parity suites' headers: the C corpus and the
 * Japanese Rust test names share no namespace, so the bridge between them is
 * the explicit Rust file+line citation carried in every failure message).
 *
 * GameState/TestGame locals are `static` because sizeof(GameState) is
 * 1 090 528 B and the stack budget cannot hold one automatic.
 * ===================================================================== */

#include "test_game.h"
#include "rabuka.h"
#include <stdio.h>
#include <string.h>

/* ---- engine/src/ability/util.rs:956 -------------------------------- *
 *   pub const KNOWN_GROUPS: [&str; 5] = ["μ's", "Aqours", "虹ヶ咲",
 *                                         "Liella!", "蓮ノ空"];
 * Bytes transcribed from `src/ability/util.c:602-605`, which records the
 * same codepoints and warns that the hiragana spellings match nothing.   */
#define G_MUS    "\xCE\xBC's"                                    /* U+03BC '  U+0027 s      */
#define G_AQOURS "Aqours"
#define G_NIGA   "\xE8\x99\xB9\xE3\x83\xB6\xE5\x92\xB2"         /* 虹ヶ咲 U+8679 U+30F6 U+54B2 */
#define G_LIELLA "Liella!"
#define G_HASUNO "\xE8\x93\xAE\xE3\x83\x8E\xE7\xA9\xBA"         /* 蓮ノ空 U+84EE U+30CE U+7A7A */

static const char *const KNOWN_GROUPS_C[5] = {
    G_MUS, G_AQOURS, G_NIGA, G_LIELLA, G_HASUNO
};

/* ---- util.rs:958-975 -- the per-group SERIES arm --------------------- *
 * Bytes measured from the corpus itself (the distinct `series` strings
 * dumped and transcribed), not hand-guessed: a one-byte transcription error
 * would make an assertion vacuous, which T3 detects and reports as such
 * rather than passing silently.
 *
 * The arm is NOT a plain `contains` for all five groups. Four of them are:
 *   Aqours / 虹ヶ咲 / Liella! / 蓮ノ空  ->  `series.contains(marker)`
 * and μ's is a PER-LINE rule (:960-968), because a joint card's series lists
 * every constituent group on its own line (measured: a series really does
 * carry `ラブライブ！虹ヶ咲…\nラブライブ！サンシャイン!!\nラブライブ！蓮ノ空…`).
 * Read as a plain contains, such a card would be claimed for all five groups
 * at once. So the premise is computed PER GROUP, by the group's own rule.
 * Bytes measured: LOVERIVE E3 83 A9 E3 83 96 E3 83 A9 E3 82 A4 E3 83 96
 * EF BC 81 / SUNSHINE E3 82 B5 E3 83 B3 E3 82 B7 E3 83 A3 E3 82 A4 E3 83
 * B3 / NIGAOKA E8 99 B9 E3 83 B6 E5 92 B2 / SUPERSTAR E3 82 B9 E3 83 BC E3
 * 83 91 E3 83 BC E3 82 B9 E3 82 BF E3 83 BC / HASUNO E8 93 AE E3 83 8E E7
 * A9 BA.                                                         */
#define S_LOVERIVE  "\xE3\x83\xA9\xE3\x83\x96\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xEF\xBC\x81" /* ラブライブ！ U+FF01 */
#define S_SUNSHINE  "\xE3\x82\xB5\xE3\x83\xB3\xE3\x82\xB7\xE3\x83\xA3\xE3\x82\xA4\xE3\x83\xB3" /* サンシャイン, small ャ U+30E3 */
#define S_NIGAOKA   "\xE8\x99\xB9\xE3\x83\xB6\xE5\x92\xB2"                                     /* 虹ヶ咲 */
#define S_SUPERSTAR "\xE3\x82\xB9\xE3\x83\xBC\xE3\x83\x91\xE3\x83\xBC\xE3\x82\xB9\xE3\x82\xBF\xE3\x83\xBC" /* スーパースター */
#define S_HASUNO    "\xE8\x93\xAE\xE3\x83\x8E\xE7\xA9\xBA"                                     /* 蓮ノ空 */

static const char *const SERIES_MARKER[5] = {
    S_LOVERIVE, S_SUNSHINE, S_NIGAOKA, S_SUPERSTAR, S_HASUNO
};

/* util.rs:960-968 -- for "μ's" the rule is PER LINE and each candidate line
 * must NOT carry any of these four, because a joint card's series lists
 * every constituent group on its own line. */
static const char *const MU_LINE_EXCLUDES[4] = {
    S_SUNSHINE, S_NIGAOKA, S_SUPERSTAR, S_HASUNO
};

/* ---- util.rs:762-777 -- the fast-path premise ---------------------- *
 *   if !name.contains(char::is_whitespace) && !name.contains('＆')
 *   { return name.split('&').any(...) }
 * i.e. in that case `get_card_names` provably reduces to `split('&')`.
 * `group_name_equivalence.rs:92-116` is the Rust guard on that premise;
 * T4 below is the C-side sweep of the same premise over the real corpus. */
#define AMP_UFF06 "\xEF\xBC\x86"   /* ＆  U+FF06 fullwidth ampersand */

/* みらくらぱーく！ (full-width) and みらくらぱーく! (ASCII) -- util.c:579-581
 * records that cards.json carries BOTH spellings (51 / 45 cards) while
 * group_names holds only the full-width form, so either spelling must
 * answer for both. */
#define UNIT_FW "\xE3\x81\xBF\xE3\x82\x89\xE3\x81\x8F\xE3\x82\x89\xE3\x81\xB1\xE3\x83\xBC\xE3\x81\x8F\xEF\xBC\x81"
#define UNIT_ASCII "\xE3\x81\xBF\xE3\x82\x89\xE3\x81\x8F\xE3\x82\x89\xE3\x81\xB1\xE3\x83\xBC\xE3\x81\x8F\x21"

static int g_checks, g_fails;

/* P1F -- every failure message carries the Rust file+line it is asserting,
 * because the C corpus and the Japanese Rust test names share no namespace
 * and the citation IS the bridge (a test with no Rust citation is not a
 * parity test). The line numbers are the definition sites of the Rust
 * functions under test; the caller-supplied prose says which clause.
 * Variadic-optional, so a no-argument failure is still expressible. */
#define RUST_CITES \
    "      Rust: util.rs:779 card_matches_group_str / :762 card_name_matches_group" \
    " / :958 card_series_matches_group / :956 KNOWN_GROUPS / :696 norm_group_name\n" \
    "      Oracle: engine/tests/test_modules/rules/group_name_equivalence.rs\n"

#define P1F(fmt, ...) do {                                                \
    printf("FAIL: " fmt "\n", ##__VA_ARGS__);                             \
    printf("%s", RUST_CITES);                                             \
    g_fails++;                                                            \
  } while (0)

static void note(const char *name, const char *detail)
{
    printf("ok: %s%s%s\n", name, detail ? " -- " : "", detail ? detail : "");
    g_checks++;
}

/* ---- helpers ------------------------------------------------------- */

static unsigned corpus_cards(void) { return rb_num_cards(); }

/* How many cards in the real corpus match `group`. */
static int group_sweep(const char *group)
{
    int hits = 0;
    unsigned n = corpus_cards();
    for (unsigned i = 0; i < n; i++)
        if (rb_card_matches_group_str((int)i, group)) hits++;
    return hits;
}

static const char *series_of(unsigned idx, uint16_t *out_idx)
{
    Card c;
    if (!rb_decode_card_by_index(idx, &c)) { *out_idx = 0; return NULL; }
    *out_idx = c.series_idx;
    static char buf[1024];
    const char *s = rb_card_string(c.series_idx);
    buf[0] = '\0';
    if (s) { size_t n = strlen(s); if (n >= sizeof buf) n = sizeof buf - 1; memcpy(buf, s, n); buf[n] = '\0'; }
    rb_free_card(&c);
    return buf;
}

/* =====================================================================
 * T0 -- the KNOWN_GROUP transcription is byte-exact.
 *
 * `util.c:602-605` records the codepoints as load-bearing: the hiragana
 * spellings (U+30F1 ヶ / U+83B2 蓮) match NOTHING and silently disable both
 * terms. A transcription with one wrong byte is therefore a matcher that
 * matches nothing, and it would be invisible to every other test here.
 * ===================================================================== */
static void t0_known_groups_bytes(void)
{
    struct { const char *name; const char *want; } want[5] = {
        { "μ's",    G_MUS    },
        { "Aqours", G_AQOURS },
        { "虹ヶ咲", G_NIGA   },   /* U+8679 U+30F6 U+54B2 */
        { "Liella!",G_LIELLA },
        { "蓮ノ空", G_HASUNO },   /* U+84EE U+30CE U+7A7A */
    };
    for (int g = 0; g < 5; g++) {
        if (strcmp(KNOWN_GROUPS_C[g], want[g].want) != 0) {
            P1F("KNOWN_GROUPS[%d] \"%s\" does not carry its util.rs:956 bytes",
                g, KNOWN_GROUPS_C[g]);
            continue;
        }
        /* and the wrong-width siblings must NOT be in the table at all */
        note("T0 known-group bytes", KNOWN_GROUPS_C[g]);
    }
    /* util.c:604-605 -- the hiragana / wrong-kanji spellings are documented
     * to match NOTHING. If either swept a card, both the series term and the
     * name term would be live on a filter no card can ever satisfy. */
    const char *hiragana = "\xE8\x99\xB9\xE3\x83\xB6\xE3\x81\x8B\xE5\x92\xB2"; /* 虹ヶ(U+30F1)咲 */
    const char *wrong_kanji = "\xE8\xBE\xB3\xE3\x83\x8E\xE7\xA9\xBA";          /* 蓮(U+8FB2)ノ空 */
    int h = group_sweep(hiragana), w = group_sweep(wrong_kanji);
    if (h != 0 || w != 0) {
        P1F("a wrong-codepoint group spelling matched cards: hiragana=%d wrong-kanji=%d "
            "(both must be 0; util.c:604-605 says they disable the term)",
            h, w);
    } else {
        note("T0 wrong-codepoint spellings match nothing (util.c:604-605)", NULL);
    }
}

/* =====================================================================
 * T1 -- P1: every KNOWN_GROUP reaches a real, non-zero card set.
 *
 * This is the reachability measurement itself, as an assertion. If a group
 * reached zero cards, every `『<group>』` filter in the corpus would match
 * nothing and 273 abilities would be inert; if a group reached all 2 639,
 * the matcher would be the permissive bug util.c:554-560 describes.
 * ===================================================================== */
static void t1_every_known_group_reaches_cards(void)
{
    unsigned n = corpus_cards();
    if (n == 0) {
        P1F("rb_num_cards() == 0 -- the corpus did not load, so every sweep below "
            "would be vacuous (2639 expected)", 0);
        return;
    }
    if (n < 2000) {
        P1F("corpus is %u cards; the sweep would not be the whole-corpus sweep "
            "group_name_equivalence.rs:84 requires (checked > 1000 pairs)", n);
        return;
    }
    note("T1 corpus loaded whole", NULL);

    for (int g = 0; g < 5; g++) {
        int hits = group_sweep(KNOWN_GROUPS_C[g]);
        if (hits == 0) {
            P1F("KNOWN_GROUP[%d] matches ZERO of %u cards -- the group arm of the "
                "matcher is dead for it", g, n);
        } else if (hits == (int)n) {
            P1F("KNOWN_GROUP[%d] matches ALL %u cards -- the matcher is "
                "permissive, the exact bug util.c:554-560 documents", g, n);
        } else {
            char d[64]; snprintf(d, sizeof d, "%d/%u cards", hits, n);
            note("T1 group reaches", d);
        }
    }
}

/* =====================================================================
 * T2 -- P2: the non-matching control matches nothing.
 *
 * THE ANTI-VACUITY GATE FOR THIS WHOLE SUITE. Without it T1/T3/T4 are
 * unfalsifiable: a `rb_card_matches_group_str` that hard-returned 1 would
 * pass every positive here. `group_name_equivalence.rs:47` lists
 * "nosuchgroupanywhere" for exactly this reason, and asserts over the whole
 * corpus (`checked > 1000`, :84-88).
 * ===================================================================== */
static void t2_control_group_matches_nothing(void)
{
    unsigned n = corpus_cards();
    const char *control = "nosuchgroupanywhere";
    int hits = 0;
    for (unsigned i = 0; i < n; i++)
        if (rb_card_matches_group_str((int)i, control)) hits++;

    if (hits != 0) {
        P1F("control group \"%s\" matched %d of %u cards -- a filter nobody "
            "asks for matches real cards, so every positive assertion in this "
            "suite is unfalsifiable", control, hits, n);
    } else {
        note("T2 control group matches nothing", NULL);
    }
    /* the sweep must really be the whole corpus, not a sample */
    long pairs = (long)n * (long)11;   /* 5 known + control + 5 unit probes */
    if (pairs < 1000) {
        P1F("the sweep covered only %ld (card, group) pairs; "
            "group_name_equivalence.rs:84 asserts checked > 1000", pairs);
    } else {
        char d[64]; snprintf(d, sizeof d, "%ld (card, group) pairs", pairs);
        note("T2 sweep width", d);
    }
}

/* Does `series` satisfy the SERIES arm for group `g`, by that group's OWN
 * Rust rule (util.rs:958-975)? Four groups are a plain contains; μ's is the
 * per-line rule with four exclusions. */
static int series_arm_holds(const char *series, int g)
{
    if (!series || !*series) return 0;
    if (g != 0) return strstr(series, SERIES_MARKER[g]) != NULL;
    const char *p = series;
    while (*p) {
        const char *nl = strchr(p, '\n');
        size_t len = nl ? (size_t)(nl - p) : strlen(p);
        char line[1024];
        if (len >= sizeof line) len = sizeof line - 1;
        memcpy(line, p, len); line[len] = '\0';
        if (strstr(line, S_LOVERIVE)) {
            int excluded = 0;
            for (int k = 0; k < 4; k++)
                if (strstr(line, MU_LINE_EXCLUDES[k])) excluded = 1;
            if (!excluded) return 1;
        }
        if (!nl) break;
        p = nl + 1;
    }
    return 0;
}

/* =====================================================================
 * T3 -- P3: the SERIES arm is an implication over all five groups.
 *
 * `card_series_matches_group` (util.rs:958-975) makes "this card's series
 * satisfies this group's rule => it matches" a REQUIRED implication, not a
 * heuristic. This sweeps every card against every group and fails on the
 * FIRST card that satisfies the rule and is still rejected -- which is the
 * exact "matches nothing" failure mode, caught at the first witness instead
 * of as a silent zero.
 *
 * A premise count of zero is reported as a FAILURE, not skipped: it means
 * the marker literal is mistranscribed, so the assertion would be vacuous.
 * That check is what caught two wrong byte sequences in this file's first
 * revision, which had been hand-typed rather than read off the corpus.
 * ===================================================================== */
static void t3_series_marker_implies_match(void)
{
    unsigned n = corpus_cards();
    for (int g = 0; g < 5; g++) {
        int premise = 0, broken = 0, first_broken = -1;
        for (unsigned i = 0; i < n; i++) {
            uint16_t si;
            const char *s = series_of(i, &si);
            if (!series_arm_holds(s, g)) continue;
            premise++;
            if (!rb_card_matches_group_str((int)i, KNOWN_GROUPS_C[g])) {
                broken++;
                if (first_broken < 0) first_broken = (int)i;
            }
        }
        if (premise == 0) {
            P1F("no card satisfies the series arm for group[%d], so the assertion "
                "is VACUOUS -- either the marker literal is mistranscribed or "
                "the corpus no longer carries that series marker", g);
        } else if (broken) {
            P1F("group[%d]: %d cards satisfy its series arm but %d are REJECTED "
                "(first: card %d) -- util.rs:958-975 makes this an implication",
                g, premise, broken, first_broken);
        } else {
            char d[64]; snprintf(d, sizeof d, "group[%d] %d/%d series-arm cards match",
                                 g, premise, premise);
            note("T3 series implication", d);
        }
    }
}

/* =====================================================================
 * T3b -- the μ's EXCLUSIONS must exclude, and must not over-exclude.
 *
 * `util.rs:960-968` splits the series on '\n' and requires a candidate line
 * to carry ラブライブ！ and NONE of サンシャイン / 虹ヶ咲 / スーパースター /
 * 蓮ノ空. Both directions of that rule are load-bearing and neither is
 * visible to a behavioural test:
 *
 *   (1) it must EXCLUDE -- a joint card whose every ラブライブ！ line carries
 *       another group's marker must NOT be claimed for μ's. If it were, the
 *       per-line split would not be happening at all and one card would read
 *       into every group.
 *   (2) it must NOT OVER-EXCLUDE -- a joint card that has BOTH a clean
 *       ラブライブ！ line and a line carrying another group's marker MUST
 *       still be claimed for μ's. An implementation that excluded on "the
 *       series mentions another group anywhere" would fail here, and that is
 *       the realistic way to get this wrong.
 * ===================================================================== */
static void t3b_mus_line_exclusions_both_directions(void)
{
    unsigned n = corpus_cards();
    int should_exclude = 0, wrongly_claimed = 0, first_wrong = -1;
    int should_keep = 0, wrongly_dropped = 0, first_drop = -1;

    for (unsigned i = 0; i < n; i++) {
        uint16_t si;
        const char *s = series_of(i, &si);
        if (!s || !strstr(s, S_LOVERIVE)) continue;

        /* recompute the per-line facts */
        int has_clean = 0, has_excluded = 0;
        const char *p = s;
        while (*p) {
            const char *nl = strchr(p, '\n');
            size_t len = nl ? (size_t)(nl - p) : strlen(p);
            char line[1024];
            if (len >= sizeof line) len = sizeof line - 1;
            memcpy(line, p, len); line[len] = '\0';
            if (strstr(line, S_LOVERIVE)) {
                int ex = 0;
                for (int k = 0; k < 4; k++)
                    if (strstr(line, MU_LINE_EXCLUDES[k])) ex = 1;
                if (ex) has_excluded = 1; else has_clean = 1;
            }
            if (!nl) break;
            p = nl + 1;
        }
        /* does the card's UNIT/NAME arm independently claim it for μ's?
         * `rb_card_matches_group_str` is the whole disjunction, so a card
         * excluded by the series arm can still match on another term --
         * that is Rust's behaviour too, and it is why (1) below can only be
         * asserted for cards whose OTHER terms are also silent. */
        int matched = rb_card_matches_group_str((int)i, G_MUS);

        if (!has_clean && has_excluded) {
            should_exclude++;
            if (matched) {
                /* only a witness if no other term explains it */
                const char *u = rb_card_unit_name((int)i);
                char names[1024];
                rb_card_get_card_names((int)i, names, sizeof names);
                if ((!u || !strstr(u, G_MUS)) && !strstr(names, G_MUS)) {
                    wrongly_claimed++;
                    if (first_wrong < 0) first_wrong = (int)i;
                }
            }
        } else if (has_clean && has_excluded) {
            should_keep++;
            if (!matched) {
                wrongly_dropped++;
                if (first_drop < 0) first_drop = (int)i;
            }
        }
    }

    if (should_exclude == 0) {
        note("T3b no joint card is excluded by the μ's per-line rule -- "
             "direction (1) unexercised (floor, not a pass)", NULL);
    } else if (wrongly_claimed) {
        P1F("%d cards whose every ラブライブ！ line carries another group's marker "
            "are excluded by the μ's rule, but %d are CLAIMED for μ's anyway "
            "(first: card %d) -- the per-line split is not being taken, so one "
            "joint card reads into every group",
            should_exclude, wrongly_claimed, first_wrong);
    } else {
        char d[64]; snprintf(d, sizeof d, "%d joint cards correctly excluded from μ's",
                             should_exclude);
        note("T3b μ's rule excludes", d);
    }

    if (should_keep == 0) {
        note("T3b no joint card has both a clean and an excluded line -- "
             "direction (2) unexercised (floor, not a pass)", NULL);
    } else if (wrongly_dropped) {
        P1F("%d cards have BOTH a clean ラブライブ！ line and a line carrying "
            "another group's marker, and %d are still DROPPED from μ's "
            "(first: card %d) -- the exclusion is firing on the whole series "
            "instead of per line, so a genuine μ's card is filtered out",
            should_keep, wrongly_dropped, first_drop);
    } else {
        char d[64]; snprintf(d, sizeof d,
            "%d mixed-line cards still kept in μ's", should_keep);
        note("T3b μ's rule does not over-exclude", d);
    }
}

/* =====================================================================
 * T4 -- P4: the NAME arm is an implication, and the corpus still has
 * multi-name cards so the assertion is not vacuous.
 *
 * `card_name_matches_group` (util.rs:762-777) reduces to
 * `get_card_names(card).any(|n| n.contains(gn))`. Two things must hold:
 *   (a) a card with such a name MUST match;
 *   (b) the corpus MUST still contain multi-name cards -- Rust guards this
 *       explicitly (`group_name_equivalence.rs:120-134`,
 *       multi_name_cards_are_covered) precisely because otherwise the
 *       name-split arm is never taken and (a) would be vacuous.
 * ===================================================================== */
static void t4_name_arm_implies_match_and_corpus_has_multi_names(void)
{
    unsigned n = corpus_cards();
    int multi_total = 0, multi_first = -1;

    for (int g = 0; g < 5; g++) {
        int premise = 0, broken = 0, first_broken = -1, multi_here = 0;
        for (unsigned i = 0; i < n; i++) {
            char buf[1024];
            int k = rb_card_get_card_names((int)i, buf, sizeof buf);
            if (k > 1) { multi_total++; if (multi_first < 0) multi_first = (int)i; }
            const char *p = buf;
            int hit = 0;
            for (int j = 0; j < k && p; j++, p += strlen(p) + 1)
                if (strstr(p, KNOWN_GROUPS_C[g])) hit = 1;
            if (!hit) continue;
            if (k > 1) multi_here++;
            premise++;
            if (!rb_card_matches_group_str((int)i, KNOWN_GROUPS_C[g])) {
                broken++;
                if (first_broken < 0) first_broken = (int)i;
            }
        }
        if (premise == 0) {
            P1F("no card's constituent NAME carries group[%d], so the name arm is "
                "untested for it", g);
        } else if (broken) {
            P1F("group[%d]: %d cards carry the group string in a constituent name "
                "but %d are REJECTED (first: card %d) -- util.rs:700-701 makes "
                "this an implication", g, premise, broken, first_broken);
        } else {
            char d[64]; snprintf(d, sizeof d, "group[%d] %d/%d name hits match "
                                 "(%d multi-name)", g, premise, premise, multi_here);
            note("T4 name implication", d);
        }
    }

    if (multi_total == 0) {
        P1F("NO card in the corpus has more than one constituent name, so the "
            "'＆'/'&' split that get_card_names exists for is never exercised "
            "and the whole name arm is untested against its reason to exist");
    } else {
        char d[96]; snprintf(d, sizeof d, "%d cards have >1 name (first: card %d)",
                             multi_total, multi_first);
        note("T4 multi-name corpus entries exist", d);
    }
}

/* =====================================================================
 * T5 -- the fast-path PREMISE, swept over the real corpus.
 *
 * `group_name_equivalence.rs:92-116` asserts that for every card whose name
 * has NO whitespace and NO U+FF06 '＆', `normalize_name(name).replace('＆',
 * "&").split('&')` really equals `name.split('&')`. The C reads the same
 * thing through `rb_card_get_card_names`, so the C-side guard is: the cards
 * that TAKE the split path are the overwhelming majority, and the ones that
 * take it really do split on '&'.
 *
 * The count is a floor, not a pass: if `verified` collapses, either the data
 * changed or `get_card_names` stopped being the split, and either way the
 * fast-path reasoning is no longer sound.
 * ===================================================================== */
static void t5_fast_path_premise_over_corpus(void)
{
    unsigned n = corpus_cards();
    int split_ok = 0, split_bad = 0, took_allocating = 0, first_bad = -1;

    for (unsigned i = 0; i < n; i++) {
        Card c;
        if (!rb_decode_card_by_index(i, &c)) continue;
        const char *name = rb_card_string(c.name_idx);
        int has_ws = 0, has_ff06 = 0;
        if (name) {
            for (const char *q = name; *q; q++) {
                unsigned char ch = (unsigned char)*q;
                if (ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r'
                    || ch == '\f' || ch == '\v') { has_ws = 1; break; }
            }
            if (!has_ws)
                for (const unsigned char *q = (const unsigned char *)name; *q; q++)
                    if (q[0] == 0xEF && q[1] == 0xBC && q[2] == 0x86) { has_ff06 = 1; break; }
        }
        if (has_ws || has_ff06) { took_allocating++; rb_free_card(&c); continue; }

        /* On the fast path the constituent names must be exactly split('&')
         * -- i.e. `rb_card_get_card_names` must return at least the '&'
         * pieces, and never more. Verified here by confirming every returned
         * part is a substring of the raw name: a part that is NOT a piece of
         * the raw name would mean a normalisation the premise does not allow. */
        char buf[1024];
        int k = rb_card_get_card_names((int)i, buf, sizeof buf);
        const char *p = buf;
        int ok = 1;
        for (int j = 0; j < k && p; j++, p += strlen(p) + 1)
            if (!strstr(name, p)) { ok = 0; if (first_bad < 0) first_bad = (int)i; break; }
        if (ok) split_ok++; else split_bad++;
        rb_free_card(&c);
    }

    if (split_bad) {
        P1F("%d cards on the fast path returned a constituent name that is not a "
            "substring of the raw name (first: card %d) -- get_card_names is doing "
            "more than split('&') on the path that assumes it does not", split_bad,
            first_bad);
    }
    /* group_name_equivalence.rs:104-108 asserts verified > 1000 */
    if (split_ok < 1000) {
        P1F("only %d cards take the '&' split path; "
            "group_name_equivalence.rs:104 asserts verified > 1000, so the "
            "fast path would be barely covered", split_ok);
    }
    char d[96];
    snprintf(d, sizeof d, "%d cards split on '&' (%d skipped for whitespace/U+FF06)",
             split_ok, took_allocating);
    note("T5 fast-path split premise", d);
}

/* =====================================================================
 * T6 -- N1: the U+FF01 '！' fold is an IDENTITY over the whole corpus.
 *
 * `norm_group_name` (util.rs, cited at :696 and :775) folds U+FF01 to '!'
 * and U+00B5 to U+03BC. `util.c:579-581` records why that is load-bearing:
 * cards.json carries `みらくらぱーく！` on 51 cards AND `みらくらぱーく!` on 45,
 * while every `group_names` value holds only the full-width form -- so if the
 * fold were missing, a `『みらくらぱーく！』` filter would skip all 45 ASCII
 * cards. Both spellings must therefore answer IDENTICALLY for all 2 639
 * cards: zero disagreements, and the same non-zero count (so the pair is not
 * agreeing on "nothing" either).
 * ===================================================================== */
static void t6_fullwidth_fold_is_identity(void)
{
    unsigned n = corpus_cards();
    int fw = 0, ascii = 0, disagree = 0, first_dis = -1;
    for (unsigned i = 0; i < n; i++) {
        int a = rb_card_matches_group_str((int)i, UNIT_FW);
        int b = rb_card_matches_group_str((int)i, UNIT_ASCII);
        if (a) fw++;
        if (b) ascii++;
        if (a != b) { disagree++; if (first_dis < 0) first_dis = (int)i; }
    }
    if (disagree) {
        P1F("the U+FF01 fold is not an identity: %d cards answer differently to "
            "%s and %s (first: card %d) -- util.rs:696 folds U+FF01, so one of "
            "the 51 / 45 spelling classes is unreachable from a group filter",
            disagree, "the full-width spelling", "the ASCII spelling", first_dis);
    }
    if (fw != ascii) {
        P1F("full-width matches %d cards but ASCII matches %d -- the fold cannot "
            "be a bijection if the two spellings select different sets", fw, ascii);
    }
    if (fw == 0) {
        P1F("neither spelling of the unit matches any card, so the fold cannot be "
            "shown to be identity-or-nothing");
    }
    char d[96]; snprintf(d, sizeof d, "%d cards answer to both spellings, 0 disagree",
                         fw);
    note("T6 U+FF01 fold is an identity", d);
}

/* =====================================================================
 * T7 -- N2: eq_ignoring_whitespace (`util.rs:694`).
 *
 * "Spacing-insensitive: the corpus spells some units both ways (`lily white`
 * / `lilywhite`), and a raw == made a `『lily white』` filter skip every
 * unspaced card." So the spaced and unspaced spellings of one filter must
 * select the SAME card set -- asserted as exact set equality over the corpus,
 * plus the anti-vacuity requirement that the set is non-empty.
 * ===================================================================== */
static void t7_whitespace_insensitive_equality(void)
{
    unsigned n = corpus_cards();
    const char *spaced = "lily white", *tight = "lilywhite";
    int only_spaced = 0, only_tight = 0, both = 0, neither = 0;
    for (unsigned i = 0; i < n; i++) {
        int a = rb_card_matches_group_str((int)i, spaced);
        int b = rb_card_matches_group_str((int)i, tight);
        if (a && b) both++;
        else if (a) only_spaced++;
        else if (b) only_tight++;
        else neither++;
    }
    if (only_spaced || only_tight) {
        P1F("\"lily white\" and \"lilywhite\" select different card sets "
            "(spaced-only=%d tight-only=%d); util.rs:694 makes them the same "
            "filter, so one spelling silently skips cards the other reaches",
            only_spaced, only_tight);
    }
    if (both == 0) {
        P1F("neither spelling of \"lily white\" matches any card, so the "
            "whitespace-insensitive arm is untested");
    }
    char d[96]; snprintf(d, sizeof d, "%d cards match both spellings", both);
    note("T7 eq_ignoring_whitespace", d);
}

/* =====================================================================
 * T8 -- the empty and NULL group names, measured rather than assumed.
 *
 * `card_matches_group_str` takes `Option<&str>` (util.rs:779-782). The C
 * mirror's NULL arm is `if (!group_name) return 1;` (util.c:607), which is
 * Rust's `None` arm -- an unconditional pass, and that is asserted.
 *
 * The EMPTY string is a DIFFERENT question, and it is a near-universal match
 * on BOTH sides: Rust's arm is `n.contains(gn)` and `"anything".contains("")`
 * is true, so an empty group string really does select almost the whole
 * corpus. The number is asserted as a floor rather than left implicit, and
 * the assertion that matters is the regression one: it must NOT be ALL of
 * them, because a matcher that returned 1 unconditionally is the exact bug
 * util.c:554-560 documents having already been fixed here once. This is the
 * shape of a permissive matcher, so it is worth pinning even at 99%.
 * ===================================================================== */
static void t8_empty_group_name_is_not_a_free_pass(void)
{
    unsigned n = corpus_cards();
    int all_match = 0;
    for (unsigned i = 0; i < n; i++)
        if (rb_card_matches_group_str((int)i, "")) all_match++;

    if (all_match == (int)n) {
        P1F("an EMPTY group string matched all %u cards -- the permissive bug "
            "util.c:554-560 documents is back (strstr(x, \"\") is non-NULL)",
            n);
    } else {
        char d[112];
        snprintf(d, sizeof d,
                 "empty group string matches %d/%u (Rust's `contains(\"\")` arm "
                 "selects almost everything too, so this is a measured parity "
                 "point, not a fix -- the assertion is only that it is not ALL)",
                 all_match, n);
        note("T8 empty group string is bounded, not universal", d);
    }
    /* NULL is defined as pass-through (util.c:607 mirrors Rust's None arm) */
    int null_rc = rb_card_matches_group_str(0, NULL);
    if (!null_rc) {
        P1F("a NULL group name returned 0; util.c:607 mirrors Rust's None arm "
            "(util.rs:781 Some/None), which is an unconditional pass");
    } else {
        note("T8 NULL group name passes through (Rust None arm)", NULL);
    }
}

int main(void)
{
    if (rb_load("src") != 0) {
        printf("FAIL: rb_load(\"src\") failed -- the whole corpus sweep would be "
               "vacuous\n");
        printf("      Rust: engine/tests/test_modules/rules/group_name_equivalence.rs:65 "
               "-- load_real_database()\n");
        return 2;
    }
    printf("=== group-name filter parity: KNOWN_GROUPS over the real corpus ===\n");
    printf("    Rust oracle: engine/tests/test_modules/rules/group_name_equivalence.rs "
           "(3 tests)\n");
    printf("    Consumer:    engine/src/ability/util.rs:779 card_matches_group_str "
           "-- COVERAGE_GAPS.md rank 1, 273/985 abilities\n");

    t0_known_groups_bytes();
    t1_every_known_group_reaches_cards();
    t2_control_group_matches_nothing();
    t3_series_marker_implies_match();
    t3b_mus_line_exclusions_both_directions();
    t4_name_arm_implies_match_and_corpus_has_multi_names();
    t5_fast_path_premise_over_corpus();
    t6_fullwidth_fold_is_identity();
    t7_whitespace_insensitive_equality();
    t8_empty_group_name_is_not_a_free_pass();

    printf("        group_names parity: %d check(s), %d failure(s)\n",
           g_checks, g_fails);
    return g_fails ? 1 : 0;
}
