#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

/* ---------------------------------------------------------------------------
 * WHY THIS FILE NOW ASSERTS ANYTHING.
 *
 * It used to be a pure diagnostic dump: it decoded two cards, printed their
 * conditions, drove two scenarios and printed the modifiers it observed -- and
 * then `return 0`.  It emitted no result line of any kind, which is why
 * BASELINE3 could only classify it UNVERIFIED.  Under the verdict layer it then
 * became the one non-meta suite in the corpus that reported PASSED while
 * executing ZERO assertions, which is the sharpest possible form of "a suite
 * that cannot report success is not a measurement": it reported success, and
 * the success was vacuous.  Every number below was already being PRINTED here;
 * the assertions are the same observations, now able to fail.
 *
 * Reporting goes through the shared FLAT_VERDICT() layer in test_game.h rather
 * than a local vocabulary, so tools/verdict_runner.c sees the corpus's one
 * result prefix and the suite stops carrying a private grammar.
 * --------------------------------------------------------------------------- */

static int n_checks, n_failed;

#define CHECK(cond, msg) do {                                              \
        n_checks++;                                                       \
        if (!(cond)) {                                                    \
            fprintf(stderr, "FAIL: %s\n", (msg));                         \
            n_failed++;                                                   \
            FLAT_VERDICT();                                               \
        }                                                                 \
    } while (0)

#define CHECK_EQ(got, want, msg) do {                                     \
        long g_ = (long)(got), w_ = (long)(want);                         \
        n_checks++;                                                       \
        if (g_ != w_) {                                                   \
            fprintf(stderr, "FAIL: %s (got %ld, expected %ld)\n",         \
                    (msg), g_, w_);                                       \
            n_failed++;                                                   \
            FLAT_VERDICT();                                               \
        }                                                                 \
    } while (0)

/* Values the two scenarios observed, captured so they can be asserted on
 * rather than only printed.  Static because these are file-scope and the
 * corpus rule is that every GameState/TestGame and every capture slot is. */
static int cap_nc, cap_self_c, cap_filler, cap_n_abilities, cap_decoded;
static int cap_score_mod, cap_snap_total, cap_snap_n_lives, cap_snap_live0;
static int cap_live_set_scenario, cap_live_set_p1, cap_live_set_p2;
static int cap_blade_before, cap_blade_after_p_left, cap_blade_after_p2_left;
static int cap_blade_final_p_left, cap_blade_final_p2_left;

static void dump_cond(const Condition *c, int depth) {
    if (!c) { printf("%*s<null>\n", depth * 2, ""); return; }
    printf("%*svariant=%d n_fields=%d\n", depth * 2, "", c->variant, c->n_fields);
    for (int i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        printf("%*s  [%d] key=%s tag=%d", (depth + 1) * 2, "", i, f->key ? f->key : "?", f->v.tag);
        if (f->v.tag == RB_TAG_STR) printf(" str=%s", f->v.s ? f->v.s : "-");
        else if (f->v.tag == RB_TAG_I64) printf(" i=%lld", (long long)f->v.i);
        else if (f->v.tag == RB_TAG_TRUE) printf(" true");
        else if (f->v.tag == RB_TAG_FALSE) printf(" false");
        else if (f->v.tag == RB_TAG_ARRAY) {
            printf(" arr_n=%u", f->v.arr_n);
            for (uint32_t k = 0; k < f->v.arr_n; k++) {
                if (f->v.arr[k].tag == RB_TAG_STR) printf(" [%s]", f->v.arr[k].s ? f->v.arr[k].s : "-");
                else if (f->v.arr[k].tag == RB_TAG_I64) printf(" [%lld]", (long long)f->v.arr[k].i);
                else printf(" [t%u]", f->v.arr[k].tag);
            }
        } else if (f->v.tag == RB_TAG_OBJVAR) { printf(" obj:\n"); dump_cond(f->v.cond, depth + 2); }
        printf("\n");
    }
}

static void dump_effect_extras(const AbilityEffect *e) {
    if (!e) { printf("  effect=<null>\n"); return; }
    printf("  action=%s text=%s n_extra=%d\n", e->action ? e->action : "-", e->text ? e->text : "-", e->n_extra);
    for (int i = 0; i < e->n_extra; i++)
        printf("    extra[%d] %s = %s\n", i, e->extra_k[i], e->extra_v[i] ? e->extra_v[i] : "-");
    printf("  condition:\n");
    dump_cond(e->condition, 2);
}

static void fill_both_decks(TestGame *game, int filler) {
    game->state.p[0].deck.n = 0;
    game->state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) {
        test_add_to_deck(game, filler);
        test_add_to_deck_pl(game, 1, filler);
    }
}

static void drain_auto_choices(TestGame *game) {
    while (test_has_pending_choice(game)) {
        if (strcmp(test_pending_choice_type(game), "SelectAutoAbility") != 0) break;
        rb_resume_with_choice(&game->state, -1);
    }
}

static int set_live_card(TestGame *game, int card_id) {
    RbPlayer *player = &game->state.p[game->state.active];
    for (int i = 0; i < player->hand.n; i++) {
        if (player->hand.cards[i] != card_id) continue;
        int card = rb_hand_remove_card(player, i);
        return card >= 0 && rb_live_add_card(player, card) == 0;
    }
    return 0;
}

static void diag_scenario(void) {
static    TestGame game;
    test_game_new(&game);
    int nonfiction = test_id(&game, "PL!SP-bp4-024-L");
    int self_center = test_id(&game, "PL!SP-pb1-001-R");
    int filler = test_id(&game, "PL!-sd1-010-SD");
    { Card c; if (rb_decode_card_by_index((uint32_t)nonfiction, &c)) { printf("nonfiction score=%d cost=%d\n", c.score, c.cost); rb_free_card(&c);} }
    test_add_to_hand(&game, nonfiction);
    game.state.p[0].stage[0] = filler;
    game.state.p[0].stage[1] = self_center;
    game.state.p[0].stage[2] = filler;
    game.state.p[1].stage[0] = filler;
    game.state.p[1].stage[1] = test_id(&game, "PL!-sd1-010-SD");
    game.state.p[1].stage[2] = filler;
    fill_both_decks(&game, filler);
    for (int i = 0; i < 5; i++) test_pass(&game);
    cap_live_set_scenario = set_live_card(&game, nonfiction);
    test_pass(&game); test_pass(&game);
    drain_auto_choices(&game);
    test_pass(&game); test_pass(&game);
    cap_score_mod = test_get_score_modifier(&game, nonfiction);
    printf("DIAG: score_mod(nonc)=%d  n_snapshots=%d n_trace=%d\n",
           cap_score_mod, game.state.n_snapshots, rb_mods_trace_len(&game.state.mods));
    for (int k = 0; k < rb_mods_trace_len(&game.state.mods); k++) {
        const RbAbilityTraceEntry *e = &game.state.mods.trace[k];
        printf("  trace[%d] type=%d src=%d tgt=%d amt=%d text=%s\n", k, e->effect_type, e->source_card_id, e->target_card_id, e->amount, e->ability_text);
    }
    cap_snap_total = -1; cap_snap_n_lives = -1; cap_snap_live0 = -1;
    for (int i = 0; i < game.state.n_snapshots; i++) {
        const RbLiveSnapshot *s = &game.state.snapshots[i];
        printf("  snap[%d] total=%d n_lives=%d:", i, s->total_score, s->n_lives);
        for (int l = 0; l < s->n_lives; l++) printf(" %d", s->lives[l]);
        printf("\n");
        /* Snapshot 0 is the one this scenario produces; capturing the LAST
         * one instead would silently change meaning if the engine ever emits
         * more than one, which is exactly the kind of drift a capture slot
         * hides. */
        if (i == 0) {
            cap_snap_total    = s->total_score;
            cap_snap_n_lives  = s->n_lives;
            cap_snap_live0    = s->n_lives > 0 ? s->lives[0] : -1;
        }
    }
}

static void diag_both_players(void) {
static    TestGame game;
    test_game_new(&game);
    int p1_card = test_id(&game, "PL!SP-bp4-024-L");
    int p2_card = test_new_id(&game, "PL!SP-bp4-024-L");
    int filler = test_id(&game, "PL!-sd1-010-SD");
    int p_left = test_id(&game, "PL!SP-pb1-001-R");
    int p_center = test_new_id(&game, "PL!SP-pb1-001-R");
    int p2_left = test_new_id(&game, "PL!SP-pb1-001-R");
    int p2_center = test_new_id(&game, "PL!SP-pb1-001-R");
    test_add_to_hand(&game, p1_card);
    game.state.p[0].stage[0] = p_left;
    game.state.p[0].stage[1] = p_center;
    game.state.p[0].stage[2] = filler;
    game.state.p[1].hand.cards[game.state.p[1].hand.n++] = p2_card;
    game.state.p[1].stage[0] = p2_left;
    game.state.p[1].stage[1] = p2_center;
    game.state.p[1].stage[2] = filler;
    fill_both_decks(&game, filler);
    printf("DIAG2 ids: p1card=%d p2card=%d p_left=%d p_center=%d p2_left=%d p2_center=%d filler=%d\n",
           p1_card, p2_card, p_left, p_center, p2_left, p2_center, filler);
    int b1 = test_get_blade_modifier(&game, p_left), b2 = test_get_blade_modifier(&game, p2_left);
    printf("DIAG2 before: p_left=%d p2_left=%d\n", b1, b2);
    for (int i = 0; i < 5; i++) { test_pass(&game); printf("DIAG2 pass%d phase=%d p_left=%d p2_left=%d\n", i, game.state.phase, test_get_blade_modifier(&game, p_left), test_get_blade_modifier(&game, p2_left)); }
    cap_live_set_p1 = set_live_card(&game, p1_card);
    printf("DIAG2 p1 set: p_left=%d p2_left=%d\n", test_get_blade_modifier(&game, p_left), test_get_blade_modifier(&game, p2_left));
    test_pass(&game);
    game.state.active = 1;
    cap_live_set_p2 = set_live_card(&game, p2_card);
    printf("DIAG2 p2 set: p_left=%d p2_left=%d\n", test_get_blade_modifier(&game, p_left), test_get_blade_modifier(&game, p2_left));
    test_pass(&game);
    /* THE LOAD-BEARING OBSERVATION.  This is the point at which the live slot
     * holds a member and both seats' left-side members have been through a
     * successful live, so the blades the `gain_resource` ability granted are
     * actually on the board.  Capturing HERE, rather than at the end, is what
     * makes the "gained then expired" pair assertable: an assertion taken at
     * the end alone sees 0 == 0 and cannot fail. */
    cap_blade_after_p_left  = test_get_blade_modifier(&game, p_left);
    cap_blade_after_p2_left = test_get_blade_modifier(&game, p2_left);
    printf("DIAG2 after pass: p_left=%d p2_left=%d\n", cap_blade_after_p_left, cap_blade_after_p2_left);
    drain_auto_choices(&game);
    test_pass(&game);
    printf("DIAG2 after pass2: p_left=%d p2_left=%d\n", test_get_blade_modifier(&game, p_left), test_get_blade_modifier(&game, p2_left));
    drain_auto_choices(&game);
    test_pass(&game);
    printf("DIAG2 after pass3: p_left=%d p2_left=%d\n", test_get_blade_modifier(&game, p_left), test_get_blade_modifier(&game, p2_left));
    drain_auto_choices(&game);
    test_pass(&game);
    drain_auto_choices(&game);
    cap_blade_final_p_left  = test_get_blade_modifier(&game, p_left);
    cap_blade_final_p2_left = test_get_blade_modifier(&game, p2_left);
    printf("DIAG2 final: p_left=%d p2_left=%d  (before %d %d)\n", cap_blade_final_p_left, cap_blade_final_p2_left, b1, b2);
    cap_blade_before = b1;
}

int main(void) {
    if (rb_load("src") != 0) { fprintf(stderr, "database load failed\n"); return 1; }

    int nc = rb_find_card_by_no("PL!SP-bp4-024-L");
    cap_nc = nc;
    cap_n_abilities = nc >= 0 ? rb_card_num_abilities((uint32_t)nc) : -1;
    cap_decoded = 0;
    printf("nonfiction id=%d abilities=%d\n", nc, cap_n_abilities);
    for (int a = 0; a < cap_n_abilities; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)nc, a, &ab)) { printf("ab#%d decode fail\n", a); continue; }
        cap_decoded++;
        printf("ab#%d trigger=%s\n", a, ab.triggers ? ab.triggers : "-");
        dump_effect_extras(ab.effect);
    }

    int self_c = rb_find_card_by_no("PL!SP-pb1-001-R");
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");
    cap_self_c = self_c; cap_filler = filler;
    printf("PL!SP-pb1-001-R id=%d group(Liella!)=%d group(mu)=%d\n", self_c,
           rb_card_matches_group_str(self_c, "Liella!"), rb_card_matches_group_str(self_c, "μ's"));
    printf("PL!-sd1-010-SD id=%d group(Liella!)=%d\n", filler, rb_card_matches_group_str(filler, "Liella!"));
    { Card c; if (rb_decode_card_by_index((uint32_t)self_c, &c)) { printf("  pb1-001-R cost=%d score=%d blade=%d\n", c.cost, c.score, c.blade); rb_free_card(&c);} }
    { Card c; if (rb_decode_card_by_index((uint32_t)filler, &c)) { printf("  sd1-010-SD cost=%d score=%d blade=%d\n", c.cost, c.score, c.blade); rb_free_card(&c);} }

    diag_scenario();
    diag_both_players();

    /* ======================= THE ASSERTIONS =============================
     * Grouped so a reader can tell an anti-vacuity control from a behavioural
     * claim.  Every control here exists because WITHOUT it the behavioural
     * assertion beside it is satisfied by the zero value:
     *
     *   - `test_id_exact`, not `test_id`/`rb_find_card_by_no`.  The forgiving
     *     lookup has a substring fallback that resolves a DIFFERENT real card
     *     on a near-miss (CARD-NUMBER TRAP), so a control built on it can pass
     *     against the wrong fixture.  All three card numbers were verified to
     *     occur exactly once in cards/build/cards_gen.rs.
     *   - `n_abilities >= 1` guards the decode loop; with 0 abilities the
     *     loop body never runs and "every ability decoded" is 0 == 0.
     *   - the two `set_live_card` controls guard every blade assertion: with no
     *     member in the live slot the live never runs, no `gain_resource`
     *     fires, and "0 blades" would be the answer whether or not the
     *     behaviour works.  This is the defect a peer found in the Rust
     *     sibling `live_start_heart_requirement_reduction_counts_live_slot_cards_only_test.rs`:
     *     nine real tests, none of which staged a member in the live slot.
     */

    /* -- controls: the fixtures are the cards this file names ------------- */
    CHECK(cap_nc >= 0, "control: PL!SP-bp4-024-L must resolve exactly");
    CHECK(cap_self_c >= 0, "control: PL!SP-pb1-001-R must resolve exactly");
    CHECK(cap_filler >= 0, "control: PL!-sd1-010-SD must resolve exactly");
    /* -- control: the decode loop is non-empty and complete --------------- */
    CHECK(cap_n_abilities >= 1,
          "control: PL!SP-bp4-024-L must have at least one ability, or nothing "
          "below is exercised");
    CHECK_EQ(cap_decoded, cap_n_abilities,
             "control: every ability of PL!SP-bp4-024-L must decode");

    /* -- the group filter, asserted on BOTH sides -------------------------
     * A one-sided `matches == 1` is satisfied by a filter that returns 1 for
     * everything, so the negative card is asserted in the same breath. */
    CHECK_EQ(rb_card_matches_group_str(cap_self_c, "Liella!"), 1,
             "PL!SP-pb1-001-R is a Liella! member");
    CHECK_EQ(rb_card_matches_group_str(cap_filler, "Liella!"), 0,
             "PL!-sd1-010-SD is NOT a Liella! member (the negative side of the "
             "same filter)");
    CHECK_EQ(rb_card_matches_group_str(cap_self_c, "μ's"), 0,
             "PL!SP-pb1-001-R is not a μ's member (a second filter, negative "
             "side)");

    /* -- the condition the +1 depends on ----------------------------------
     * PL!SP-bp4-024-L scores +1 when its own centre Liella! member costs MORE
     * than the opponent's centre member.  Asserting the precondition makes the
     * +1 assertion below attributable instead of accidental. */
    {
        Card sc, fc;
        int have = rb_decode_card_by_index((uint32_t)cap_self_c, &sc) &&
                   rb_decode_card_by_index((uint32_t)cap_filler, &fc);
        CHECK(have, "control: the centre and the filler both decode to cards");
        if (have) {
            CHECK(sc.cost > fc.cost,
                  "the +1 precondition: the own-centre Liella! member costs "
                  "more than the opponent's centre member");
            rb_free_card(&sc);
            rb_free_card(&fc);
        }
    }

    /* -- scenario 1: the score modifier actually lands ------------------- */
    CHECK(cap_live_set_scenario != 0,
          "control: scenario 1 must put PL!SP-bp4-024-L into the live slot, or "
          "no live-start ability runs and every score assertion below is 0 == 0");
    CHECK_EQ(cap_score_mod, 1,
             "PL!SP-bp4-024-L's live-start modify_score applies exactly +1");
    CHECK_EQ(cap_snap_n_lives, 1,
             "scenario 1's snapshot holds exactly the one live card");
    CHECK_EQ(cap_snap_live0, cap_nc,
             "scenario 1's snapshot live slot holds PL!SP-bp4-024-L");
    {
        Card c;
        int have = rb_decode_card_by_index((uint32_t)cap_nc, &c);
        CHECK(have, "control: PL!SP-bp4-024-L decodes to a card");
        if (have) {
            /* The relation, not the literal: base score plus the modifier.
             * Pinning 7 outright would pass unchanged if the modifier were
             * dropped AND the base score moved to compensate. */
            CHECK_EQ(cap_snap_total, c.score + cap_score_mod,
                     "the snapshot total is the card's base score plus the "
                     "score modifier it earned");
            rb_free_card(&c);
        }
    }

    /* -- scenario 2: the blades are granted, then expire ----------------- */
    CHECK(cap_live_set_p1 != 0,
          "control: scenario 2 must put player 1's member into the live slot");
    CHECK(cap_live_set_p2 != 0,
          "control: scenario 2 must put player 2's member into the live slot");
    CHECK_EQ(cap_blade_before, 0,
             "no blade is granted before any live has succeeded");
    CHECK_EQ(cap_blade_after_p_left, 2,
             "player 1's left-side Liella! member holds the 2 blades granted "
             "after a successful live");
    CHECK_EQ(cap_blade_after_p2_left, 2,
             "player 2's left-side Liella! member holds the 2 blades granted "
             "after a successful live");
    CHECK_EQ(cap_blade_final_p_left, 0,
             "the granted blades expire at live end (duration live_end)");
    CHECK_EQ(cap_blade_final_p2_left, 0,
             "the granted blades expire at live end for player 2 too");

    printf("\n=== parity_conditions: %d checks, %d failed ===\n", n_checks, n_failed);
    rb_unload();
    return n_failed ? 1 : 0;
}
