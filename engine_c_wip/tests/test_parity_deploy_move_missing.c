/* Parity tests for the deploy / move-missing cluster.
 *
 * Rust twins:
 *   engine/tests/test_modules/effects/deploy/*.rs
 *   engine/src/ability/move_cards.rs  (execute_move_cards + place_taken_cards)
 *   engine/src/ability/move_cards/placement.rs, selection.rs
 *   engine/src/ability/util.rs::place_card_in_zone
 *
 * Every CHECK below is a translated Rust expectation, not a description of the
 * current C behaviour. */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

static int failures;
static int checks;

#define CHECK(condition, message) do { \
    checks++; \
    if (!(condition)) { fprintf(stderr, "FAIL: %s\n", message); failures++; } \
    else printf("ok: %s\n", message); \
} while (0)
#define CHECK_EQ(actual, expected, message) do { \
    int actual_value = (actual); int expected_value = (expected); \
    checks++; \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else printf("ok: %s\n", message); \
} while (0)

/* Mirrors Rust `v.state.player1.stage.stage = [a, -1, -1]` style setup. */
static void set_stage(TestGame *g, int a, int b, int c) {
    g->state.p[0].stage[0] = a;
    g->state.p[0].stage[1] = b;
    g->state.p[0].stage[2] = c;
}

static void move_effect(AbilityEffect *e, const char *source, const char *dest, int count) {
    memset(e, 0, sizeof(*e));
    e->action = (char *)"move_cards";
    e->source = (char *)source;
    e->destination = (char *)dest;
    e->count = count;
}

static void set_extra(AbilityEffect *e, const char *k, const char *v) {
    int n = e->n_extra;
    if (n >= RB_MAX_EXTRA) return;
    e->extra_k[n] = (char *)k;
    e->extra_v[n] = (char *)v;
    e->n_extra = n + 1;
}

/* ── 1. deploy to stage (keke debut) ──────────────────────────────────────
 * engine/tests/test_modules/effects/deploy/pl_sp_sd1_002_test.rs:8-59
 * Keke (PL!SP-sd1-002-SD) debuts by deploying a cost-4 Liella! card from hand
 * into a stage slot, and the slot is asked for when several are free. */
static void test_deploy_to_stage_keke(void) {
    TestGame g;
    test_game_new(&g);
    int keke = test_new_id(&g, "PL!SP-sd1-002-SD");
    int liella = test_new_id(&g, "PL!SP-sd1-013-SD");
    test_add_to_hand(&g, liella);
    test_add_to_hand(&g, keke);
    for (int i = 0; i < 40; i++) test_add_to_deck(&g, test_id(&g, "PL!-sd1-010-SD"));
    test_give_energy(&g, 20);

    CHECK_EQ(test_play_to_stage(&g, keke, 1), 1, "keke plays from hand to Center");
    CHECK_EQ(g.state.p[0].stage[1], keke, "keke is deployed at Center (rs:54)");
    CHECK_EQ(test_pending_choice_type(&g) && !strcmp(test_pending_choice_type(&g), "SelectCard"),
             1, "keke debut opens a hand SelectCard (rs:28-33)");

    int idx = -1;
    for (int i = 0; i < g.state.p[0].hand.n; i++)
        if (g.state.p[0].hand.cards[i] == liella) { idx = i; break; }
    CHECK_EQ(idx, 0, "the Liella! card is found in hand for selection (rs:34-42)");
    test_resume_choice(&g, idx);

    /* rs:44-52 ”Eseveral slots are still free, so the destination IS asked. */
    CHECK_EQ(test_pending_choice_type(&g) && !strcmp(test_pending_choice_type(&g), "SelectPosition"),
             1, "keke debut asks which stage slot to deploy into (rs:45-49)");
    if (test_has_pending_choice(&g)) test_resume_choice(&g, 0); /* left */
    CHECK_EQ(test_zone_has_id(&g, 0, "stage", liella), 1,
             "the selected Liella! card reaches the stage (rs:55-58)");
}

/* ── 2. deploy into an occupied slot replaces the occupant ────────────────
 * pl_sp_sd1_002_test.rs:63-119 */
static void test_deploy_replaces_occupant(void) {
    TestGame g;
    test_game_new(&g);
    int keke = test_new_id(&g, "PL!SP-sd1-002-SD");
    int liella = test_new_id(&g, "PL!SP-sd1-013-SD");
    int filler = test_new_id(&g, "PL!-sd1-010-SD");
    test_add_to_hand(&g, liella);
    test_add_to_hand(&g, keke);
    set_stage(&g, filler, -1, -1);
    for (int i = 0; i < 40; i++) test_add_to_deck(&g, test_id(&g, "PL!-sd1-010-SD"));
    test_give_energy(&g, 20);

    CHECK_EQ(test_play_to_stage(&g, keke, 1), 1, "keke plays to Center over an occupied board");
    CHECK_EQ(test_pending_choice_type(&g) && !strcmp(test_pending_choice_type(&g), "SelectCard"),
             1, "keke debut still opens the hand SelectCard (rs:84-88)");
    int idx = -1;
    for (int i = 0; i < g.state.p[0].hand.n; i++)
        if (g.state.p[0].hand.cards[i] == liella) { idx = i; break; }
    test_resume_choice(&g, idx);
    CHECK_EQ(test_pending_choice_type(&g) && !strcmp(test_pending_choice_type(&g), "SelectPosition"),
             1, "the occupied Left slot is still offered (rs:101-106)");
    if (test_has_pending_choice(&g)) test_resume_choice(&g, 0); /* left, occupied */
    CHECK_EQ(g.state.p[0].stage[0], liella, "Liella! replaces the occupant at Left (rs:111)");
    CHECK_EQ(g.state.p[0].stage[1], keke, "keke stays at Center (rs:114)");
    CHECK_EQ(test_zone_has_id(&g, 0, "waitroom", filler), 1,
             "the replaced member goes to the waitroom (rs:116-117)");
}

/* ── 3. stage destination with no free slot sends the take to the waitroom ─
 * engine/src/ability/move_cards.rs:1762-1784 */
static void test_deploy_to_full_stage_goes_to_waitroom(void) {
    TestGame g;
    test_game_new(&g);
    int a = test_new_id(&g, "PL!-sd1-010-SD");
    int b = test_new_id(&g, "PL!-sd1-002-SD");
    int c = test_new_id(&g, "PL!SP-sd1-006-SD");
    int m = test_new_id(&g, "PL!-sd1-011-SD");
    test_add_to_stage(&g, 0, a);
    test_add_to_stage(&g, 1, b);
    test_add_to_stage(&g, 2, c);
    test_add_to_hand(&g, m);
    int discard_before = g.state.p[0].discard.n;

    AbilityEffect e;
    move_effect(&e, "hand", "stage", 1);
    rb_execute_effect_ex(&g.state, 0, &e, -1);

    CHECK_EQ(test_has_pending_choice(&g), 0, "a full stage never prompts for a slot (rs:1768)");
    CHECK_EQ(g.state.p[0].stage[0], a, "Left slot is untouched by the blocked deploy");
    CHECK_EQ(g.state.p[0].stage[1], b, "Center slot is untouched by the blocked deploy");
    CHECK_EQ(g.state.p[0].stage[2], c, "Right slot is untouched by the blocked deploy");
    CHECK_EQ(test_zone_has_id(&g, 0, "waitroom", m), 1,
             "the blocked member is sent to the waitroom, not back to hand (rs:1779)");
    CHECK_EQ(test_hand_has(&g, m), 0, "the member leaves the hand even when the deploy is blocked");
    CHECK_EQ(g.state.p[0].discard.n, discard_before + 1, "the waitroom grew by exactly one card");
}

/* ── 4. allow_occupied_stage overrides the full-stage bail-out ───────────
 * engine/src/ability/move_cards.rs:1764-1766 */
static void test_deploy_allow_occupied_stage(void) {
    TestGame g;
    test_game_new(&g);
    int a = test_new_id(&g, "PL!-sd1-010-SD");
    int b = test_new_id(&g, "PL!-sd1-002-SD");
    int c = test_new_id(&g, "PL!SP-sd1-006-SD");
    int m = test_new_id(&g, "PL!-sd1-011-SD");
    test_add_to_stage(&g, 0, a);
    test_add_to_stage(&g, 1, b);
    test_add_to_stage(&g, 2, c);
    test_add_to_hand(&g, m);

    AbilityEffect e;
    move_effect(&e, "hand", "stage", 1);
    set_extra(&e, "allow_occupied_stage", "true");
    rb_execute_effect_ex(&g.state, 0, &e, -1);

    CHECK_EQ(test_hand_has(&g, m), 0, "allow_occupied_stage consumes the hand card (rs:1764-1766)");
    CHECK_EQ(g.state.p[0].discard.n, 0, "allow_occupied_stage does not fall back to the waitroom");
}

/* ── 5. empty_area with no empty slot takes nothing at all ───────────────
 * engine/src/ability/move_cards.rs:1976-1982 */
static void test_empty_area_without_free_slot_is_a_noop(void) {
    TestGame g;
    test_game_new(&g);
    int a = test_new_id(&g, "PL!-sd1-010-SD");
    int b = test_new_id(&g, "PL!-sd1-002-SD");
    int c = test_new_id(&g, "PL!SP-sd1-006-SD");
    int m = test_new_id(&g, "PL!-sd1-011-SD");
    test_add_to_stage(&g, 0, a);
    test_add_to_stage(&g, 1, b);
    test_add_to_stage(&g, 2, c);
    test_add_to_hand(&g, m);
    int hand_before = g.state.p[0].hand.n;

    AbilityEffect e;
    move_effect(&e, "hand", "empty_area", 1);
    rb_execute_effect_ex(&g.state, 0, &e, -1);

    CHECK_EQ(test_has_pending_choice(&g), 0, "no empty-slot prompt when the stage is full (rs:1979-1981)");
    CHECK_EQ(g.state.p[0].hand.n, hand_before,
             "「メンポ�E㝮㝝E��㝝E��リア㝫〝Etakes nothing when no slot is empty (rs:1976-1982)");
    CHECK_EQ(g.state.p[0].discard.n, 0, "a no-op empty_area move does not leak the card to the waitroom");
}

/* ── 6. explicit deck position insert ─────────────────────────────────────
 * engine/src/ability/move_cards.rs:1567-1578 (parse_effect_deck_pos) and
 * 1795-1805 (deck insert at the parsed index) */
static void test_deck_position_insert(void) {
    TestGame g;
    test_game_new(&g);
    int d0 = test_new_id(&g, "PL!-sd1-010-SD");
    int d1 = test_new_id(&g, "PL!-sd1-002-SD");
    int d2 = test_new_id(&g, "PL!SP-sd1-006-SD");
    int m  = test_new_id(&g, "PL!-sd1-011-SD");
    test_add_to_deck(&g, d0);
    test_add_to_deck(&g, d1);
    test_add_to_deck(&g, d2);
    test_add_to_hand(&g, m);

    AbilityEffect e;
    move_effect(&e, "hand", "deck", 1);
    set_extra(&e, "position", "2");   /* 1-based wire value ↝E0-based index 1 */
    rb_execute_effect_ex(&g.state, 0, &e, -1);

    CHECK_EQ(g.state.p[0].deck.n, 4, "the card is inserted, the deck grows by one (rs:1804)");
    CHECK_EQ(g.state.p[0].deck.cards[1], m, "position 2 inserts at deck index 1 (rs:1577,:1803)");
    CHECK_EQ(g.state.p[0].deck.cards[0], d0, "the card above the insert point is preserved");
    CHECK_EQ(g.state.p[0].deck.cards[2], d1, "the card below the insert point is preserved");
    CHECK_EQ(g.state.p[0].deck.cards[3], d2, "the rest of the deck keeps its order");
    CHECK_EQ(test_hand_has(&g, m), 0, "the inserted card leaves the hand");
}

/* ── 7. deck_top / deck_bottom placement ──────────────────────────────── */
static void test_deck_top_and_bottom(void) {
    TestGame g;
    test_game_new(&g);
    int d0 = test_new_id(&g, "PL!-sd1-010-SD");
    int a = test_new_id(&g, "PL!-sd1-011-SD");
    test_add_to_deck(&g, d0);
    test_add_to_hand(&g, a);

    AbilityEffect e;
    move_effect(&e, "hand", "deck_top", 1);
    rb_execute_effect_ex(&g.state, 0, &e, -1);
    CHECK_EQ(g.state.p[0].deck.cards[0], a, "deck_top places at index zero (util.rs:2145-2149)");

    int b = test_new_id(&g, "PL!SP-sd1-006-SD");
    test_add_to_hand(&g, b);
    move_effect(&e, "hand", "deck_bottom", 1);
    rb_execute_effect_ex(&g.state, 0, &e, -1);
    CHECK_EQ(g.state.p[0].deck.cards[2], b, "deck_bottom appends to the end (util.rs:2150-2153)");
    CHECK_EQ(g.state.p[0].deck.n, 3, "both placements conserved the deck card count");
}

/* ── 8. same_area repositioning ──────────────────────────────────────────
 * engine/src/ability/util.rs:2169-2194 */
static void test_same_area_placement(void) {
    TestGame g;
    test_game_new(&g);
    int blocker = test_new_id(&g, "PL!-sd1-010-SD");
    int m = test_new_id(&g, "PL!-sd1-011-SD");
    test_add_to_stage(&g, 0, blocker);
    test_add_to_hand(&g, m);
    g.state.baton_last_vacated_area[0] = 0;   /* the vacated Left slot */

    AbilityEffect e;
    move_effect(&e, "hand", "same_area", 1);
    rb_execute_effect_ex(&g.state, 0, &e, -1);

    CHECK_EQ(g.state.p[0].stage[0], blocker,
             "an occupied vacated area is NOT overwritten (util.rs:2180-2182)");
    CHECK_EQ(g.state.p[0].stage[1], m,
             "same_area falls back to the first empty slot (util.rs:2180-2182)");
    CHECK_EQ(g.state.p[0].discard.n, 0, "same_area never displaces the blocker to the waitroom");
}

static void test_same_area_without_free_slot_goes_to_hand(void) {
    TestGame g;
    test_game_new(&g);
    int a = test_new_id(&g, "PL!-sd1-010-SD");
    int b = test_new_id(&g, "PL!-sd1-002-SD");
    int c = test_new_id(&g, "PL!SP-sd1-006-SD");
    int m = test_new_id(&g, "PL!-sd1-011-SD");
    test_add_to_stage(&g, 0, a);
    test_add_to_stage(&g, 1, b);
    test_add_to_stage(&g, 2, c);
    test_add_to_hand(&g, m);
    g.state.baton_last_vacated_area[0] = 0;

    AbilityEffect e;
    move_effect(&e, "hand", "same_area", 1);
    rb_execute_effect_ex(&g.state, 0, &e, -1);

    CHECK_EQ(g.state.p[0].stage[0], a, "a full stage is left untouched by same_area (util.rs:2183-2185)");
    CHECK_EQ(g.state.p[0].stage[1], b, "Center is left untouched by same_area");
    CHECK_EQ(g.state.p[0].stage[2], c, "Right is left untouched by same_area");
    CHECK_EQ(test_zone_has_id(&g, 0, "hand", m), 1,
             "with no empty slot same_area falls back to hand (util.rs:2184,:2190)");
}

/* ── 9. energy placement under a member ─────────────────────────────────
 * engine/src/ability/move_cards/placement.rs:167-218 */
static void test_energy_under_single_member(void) {
    TestGame g;
    test_game_new(&g);
    int host = test_new_id(&g, "PL!SP-sd1-006-SD");
    int en = test_new_id(&g, "LL-E-001-SD");
    test_add_to_stage(&g, 0, host);
    test_add_to_hand(&g, en);

    AbilityEffect e;
    move_effect(&e, "hand", "under_member", 1);
    rb_execute_effect_ex(&g.state, 0, &e, -1);

    CHECK_EQ(test_has_pending_choice(&g), 0,
             "one member on stage means under_member placement is automatic (placement.rs:179)");
    CHECK_EQ(g.state.p[0].under_cards[0].n, 1, "the energy is tucked under the member");
    CHECK_EQ(g.state.p[0].under_cards[0].cards[0], en, "the tucked card is the moved energy");
}

static void test_energy_under_two_members_asks_which(void) {
    TestGame g;
    test_game_new(&g);
    int host0 = test_new_id(&g, "PL!SP-sd1-006-SD");
    int host1 = test_new_id(&g, "PL!-sd1-002-SD");
    int en = test_new_id(&g, "LL-E-001-SD");
    test_add_to_stage(&g, 0, host0);
    test_add_to_stage(&g, 2, host1);
    test_add_to_hand(&g, en);

    AbilityEffect e;
    move_effect(&e, "hand", "under_member", 1);
    rb_execute_effect_ex(&g.state, 0, &e, -1);

    CHECK_EQ(test_has_pending_choice(&g), 1,
             "two members and a free choice ask which member to tuck under (placement.rs:179-205)");
    CHECK_EQ(g.state.p[0].under_cards[0].n + g.state.p[0].under_cards[2].n, 0,
             "no energy is placed before the host is chosen (placement.rs:192-205)");
}

/* ── 10. zone-to-zone movement shapes ─────────────────────────────────── */
static void test_move_hand_to_discard(void) {
    TestGame g;
    test_game_new(&g);
    int a = test_new_id(&g, "PL!-sd1-010-SD");
    int b = test_new_id(&g, "PL!SP-sd1-006-SD");
    test_add_to_hand(&g, a);
    test_add_to_hand(&g, b);

    AbilityEffect e;
    move_effect(&e, "hand", "discard", 1);
    rb_execute_effect_ex(&g.state, 0, &e, -1);

    CHECK_EQ(test_has_pending_choice(&g), 0, "a mandatory 1-of-2 hand take does not prompt");
    CHECK_EQ(test_zone_has_id(&g, 0, "discard", a), 1, "exactly one hand card reached the waitroom");
    CHECK_EQ(test_hand_has(&g, b), 1, "the untouched hand card stays in hand");
}

static void test_move_discard_to_hand(void) {
    TestGame g;
    test_game_new(&g);
    int a = test_new_id(&g, "PL!-sd1-010-SD");
    int b = test_new_id(&g, "PL!SP-sd1-006-SD");
    test_add_to_discard(&g, a);
    test_add_to_discard(&g, b);

    /* all=True ? Rust classify_selection returns Exact for is_all without any
       prompt (util/selection.rs:39-41). */
    AbilityEffect e;
    move_effect(&e, "discard", "hand", -1);
    rb_execute_effect_ex(&g.state, 0, &e, -1);

    CHECK_EQ(test_has_pending_choice(&g), 0, "an all-of-waitroom take needs no selection prompt");
    CHECK_EQ(test_zone_has_id(&g, 0, "hand", a), 1, "discard �� hand recovers the card (util.rs:2112-2114)");
    CHECK_EQ(test_zone_has_id(&g, 0, "hand", b), 1, "every matching waitroom card is recovered");
    CHECK_EQ(g.state.p[0].discard.n, 0, "the cards leave the waitroom");
}

static void test_move_stage_to_discard(void) {
    TestGame g;
    test_game_new(&g);
    int a = test_new_id(&g, "PL!SP-sd1-006-SD");
    test_add_to_stage(&g, 1, a);

    AbilityEffect e;
    move_effect(&e, "stage", "discard", 1);
    rb_execute_effect_ex(&g.state, 0, &e, -1);

    CHECK_EQ(test_zone_has_id(&g, 0, "discard", a), 1, "a member on stage can be sent to the waitroom");
    CHECK_EQ(g.state.p[0].stage[1], -1, "the vacated stage slot is emptied");
}

static void test_move_energy_deck_to_hand(void) {
    TestGame g;
    test_game_new(&g);
    int e0 = test_new_id(&g, "LL-E-002-SD");
    test_add_to_energy_deck(&g, 0, e0);

    AbilityEffect e;
    move_effect(&e, "energy_deck", "hand", 1);
    rb_execute_effect_ex(&g.state, 0, &e, -1);

    CHECK_EQ(test_zone_has_id(&g, 0, "hand", e0), 1, "energy_deck ↝Ehand recovers the energy");
    CHECK_EQ(g.state.p[0].energy_deck.n, 0, "the energy deck is drained");
}

static void test_move_under_member_to_hand(void) {
    TestGame g;
    test_game_new(&g);
    int host = test_new_id(&g, "PL!SP-sd1-006-SD");
    int en = test_new_id(&g, "LL-E-001-SD");
    test_add_to_stage(&g, 0, host);
    test_place_under(&g, 0, 0, en);

    AbilityEffect e;
    move_effect(&e, "under_member", "hand", 1);
    rb_execute_effect_ex(&g.state, 0, &e, -1);

    CHECK_EQ(test_zone_has_id(&g, 0, "hand", en), 1, "a card tucked under a member can be recovered");
    CHECK_EQ(g.state.p[0].under_cards[0].n, 0, "the under-card slot is emptied");
}

/* ── 11. relay sources and the move-then-resolve chain ─────────────────
 * The first move records the card in `recently_moved`; a following effect with
 * source "recently_moved" must be able to consume that same card. */
static void test_move_then_resolve_recently_moved(void) {
    TestGame g;
    test_game_new(&g);
    int a = test_new_id(&g, "PL!-sd1-010-SD");
    int s0 = test_new_id(&g, "PL!SP-sd1-006-SD");
    int s1 = test_new_id(&g, "PL!-sd1-002-SD");
    test_add_to_discard(&g, a);
    test_add_to_stage(&g, 0, s0);
    test_add_to_stage(&g, 1, s1);   /* only Right stays free, so no slot prompt */

    AbilityEffect e;
    move_effect(&e, "discard", "hand", -1);
    rb_execute_effect_ex(&g.state, 0, &e, -1);

    CHECK_EQ(test_zone_has_id(&g, 0, "hand", a), 1, "the first link of the chain moved the card");
    CHECK_EQ(g.state.n_recently_moved, 1, "a completed move records the card as recently moved (rs:2272)");
    CHECK_EQ(g.state.recently_moved[0], a, "the recorded card is the one that moved (rs:2271-2273)");

    move_effect(&e, "recently_moved", "stage", 1);
    rb_execute_effect_ex(&g.state, 0, &e, -1);

    CHECK_EQ(g.state.p[0].stage[2], a,
             "a follow-up move deploys the recently-moved card (rs:282-318)");
    CHECK_EQ(test_hand_has(&g, a), 0, "the relayed card is no longer in hand");
}

static void test_move_those_cards_relay(void) {
    TestGame g;
    test_game_new(&g);
    int t = test_new_id(&g, "PL!SP-sd1-006-SD");
    g.state.those_cards[g.state.n_those_cards++] = t;

    AbilityEffect e;
    move_effect(&e, "those_cards", "discard", 1);
    rb_execute_effect_ex(&g.state, 0, &e, -1);

    CHECK_EQ(test_zone_has_id(&g, 0, "discard", t), 1, "those_cards can be relayed to the waitroom (rs:454)");
    CHECK_EQ(g.state.p[0].discard.n, 1, "exactly the relayed card lands in the waitroom");
}

static void test_move_selected_cards_relay(void) {
    TestGame g;
    test_game_new(&g);
    int s = test_new_id(&g, "PL!SP-sd1-006-SD");
    g.state.selected_cards[g.state.n_selected_cards++] = s;

    AbilityEffect e;
    move_effect(&e, "selected_cards", "hand", 1);
    rb_execute_effect_ex(&g.state, 0, &e, -1);

    CHECK_EQ(test_zone_has_id(&g, 0, "hand", s), 1, "selected_cards can be relayed to hand (rs:1309)");
}

/* ── 12. distinct card-name filter drops a short deduped take ───────────
 * engine/src/ability/move_cards.rs:2030-2037 */
static void test_distinct_filter_drops_short_take(void) {
    TestGame g;
    test_game_new(&g);
    int a = test_new_id(&g, "PL!-sd1-010-SD");
    int b = test_new_id(&g, "PL!-sd1-010-SD");   /* same card name as a */
    int c = test_new_id(&g, "PL!SP-sd1-006-SD");
    test_add_to_discard(&g, a);
    test_add_to_discard(&g, b);
    test_add_to_discard(&g, c);

    AbilityEffect e;
    move_effect(&e, "discard", "hand", 2);
    set_extra(&e, "distinct", "card_name");
    rb_execute_effect_ex(&g.state, 0, &e, -1);

    CHECK_EQ(test_hand_has(&g, c), 0,
             "a distinct take short of `count` is dropped entirely (rs:2034-2036)");
    CHECK_EQ(g.state.p[0].hand.n, 0, "no card is recovered when the distinct filter comes up short");
}

/* ── 13. a debut fires exactly once for a stage placement ───────────────
 * engine/src/ability/move_cards.rs:2316-2321 (inside finalize_card_movement) */
static void test_stage_deploy_fires_debut_once(void) {
    TestGame g;
    test_game_new(&g);
    int host0 = test_new_id(&g, "PL!SP-sd1-006-SD");
    int host1 = test_new_id(&g, "PL!-sd1-002-SD");
    int m = test_new_id(&g, "PL!-sd1-010-SD");
    test_add_to_stage(&g, 0, host0);
    test_add_to_stage(&g, 1, host1);
    test_add_to_hand(&g, m);
    int before = g.state.debut_count_this_turn[0];

    AbilityEffect e;
    move_effect(&e, "hand", "stage", 1);
    rb_execute_effect_ex(&g.state, 0, &e, -1);

    CHECK_EQ(test_has_pending_choice(&g), 0, "a single free stage slot needs no position prompt");
    CHECK_EQ(g.state.p[0].stage[2], m, "the card reached the one free stage slot");
    CHECK_EQ(g.state.debut_count_this_turn[0] - before, 1,
             "a stage deploy counts exactly one debut (rs:2316-2321)");
}

/* ── 14. same_area is not a zone transition for the debut counter ─────── */
static void test_same_area_does_not_fire_debut_twice(void) {
    TestGame g;
    test_game_new(&g);
    int m = test_new_id(&g, "PL!-sd1-010-SD");
    test_add_to_hand(&g, m);
    int before = g.state.debut_count_this_turn[0];

    AbilityEffect e;
    move_effect(&e, "hand", "same_area", 1);
    rb_execute_effect_ex(&g.state, 0, &e, -1);

    CHECK_EQ(test_zone_has_id(&g, 0, "stage", m), 1, "the card was repositioned onto the stage");
    CHECK_EQ(g.state.debut_count_this_turn[0], before,
             "same_area is not a stage debut destination (rs:2316 requires Zone::Stage)");
}

int main(void) {
    setvbuf(stdout, NULL, _IONBF, 0);
    if (rb_load("src") != 0) {
        fprintf(stderr, "load_cards failed\n");
        return 1;
    }
    test_deploy_to_stage_keke();
    test_deploy_replaces_occupant();
    test_deploy_to_full_stage_goes_to_waitroom();
    test_deploy_allow_occupied_stage();
    test_empty_area_without_free_slot_is_a_noop();
    test_deck_position_insert();
    test_deck_top_and_bottom();
    test_same_area_placement();
    test_same_area_without_free_slot_goes_to_hand();
    test_energy_under_single_member();
    test_energy_under_two_members_asks_which();
    test_move_hand_to_discard();
    test_move_discard_to_hand();
    test_move_stage_to_discard();
    test_move_energy_deck_to_hand();
    test_move_under_member_to_hand();
    test_move_then_resolve_recently_moved();
    test_move_those_cards_relay();
    test_move_selected_cards_relay();
    test_distinct_filter_drops_short_take();
    test_stage_deploy_fires_debut_once();
    test_same_area_does_not_fire_debut_twice();
    rb_unload();
    printf("%d checks, %d failures\n", checks, failures);
    if (failures) return 1;
    printf("ALL PARITY DEPLOY / MOVE-MISSING CHECKS PASSED\n");
    return 0;
}
