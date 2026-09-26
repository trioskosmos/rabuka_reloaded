/* Parity suite for engine/tests/test_modules/effects/look_select/ (incl. reveal/).
   Ported from the Rust cluster; exercises look.rs behaviour through the C
   look.c port: look-at with counts/filters, reveal per group, reveal_until,
   select from a looked-at set, look_and_select, and the discard-unmatched
   path. */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

static int failures;
static int checks;

#define CHECK(condition, message) do { \
    checks++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    checks++; \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* ── fixtures ─────────────────────────────────────────────────────────── */
#define MEMBER_MUS      "PL!-sd1-010-SD"  /* cost 4,  hearts red,green,green  */
#define MEMBER_MUS_HI   "PL!S-sd1-001-SD"  /* cost 17, hearts yellow,blue,purple */
#define MEMBER_NIJI     "PL!N-bp3-004-R"  /* cost 13, hearts red,green,blue,purple,purple */
#define MEMBER_LIELLA_9 "PL!SP-bp1-013-PR"/* cost 9,  hearts yellow,green,orange  */
#define MEMBER_LIELLA_2 "PL!SP-PR-003-PR" /* cost 2,  hearts red,red */
#define MEMBER_DIA      "PL!S-PR-016-PR"   /* cost 9,  hearts yellow,blue */
#define LIVE_NIJI       "PL!N-bp1-027-L"   /* hearts green,green,pink   */
#define LIVE_BLUE       "PL!HS-sd1-020-SD" /* hearts red,blue,purple,pink */
#define LIVE_MUS        "PL!-sd1-020-SD"  /* hearts all,red,green,pink */

static void clear_player(RbPlayer *p)
{
    p->deck.n = 0;
    p->hand.n = 0;
    p->discard.n = 0;
    p->live.n = 0;
    p->success.n = 0;
}

static void fx_set(AbilityEffect *e, int i, const char *k, const char *v)
{
    e->extra_k[i] = (char *)k;
    e->extra_v[i] = (char *)v;
    if (i + 1 > e->n_extra) e->n_extra = i + 1;
}

static int bag_has(const RbBag *bag, int cid)
{
    for (int i = 0; i < bag->n; i++) if (bag->cards[i] == cid) return 1;
    return 0;
}

static int revealed_has(const GameState *g, int cid)
{
    for (int i = 0; i < g->n_revealed; i++) if (g->revealed_cards[i] == cid) return 1;
    return 0;
}

static int pool_get(int pl, int *out, int max)
{
    return rb_looked_at_pool(pl, out, max);
}

/* ═══════════════════════════════════════════════════════════════════════
   1. look_at — counts and source zones
   (Rust: look.rs::execute_look_at / fetch_look_pool / look_at_with_refresh)
   ═══════════════════════════════════════════════════════════════════════ */
static void test_look_at_counts_and_zones(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int a = test_new_id(&tg, MEMBER_MUS);
    int b = test_new_id(&tg, MEMBER_MUS_HI);
    int c = test_new_id(&tg, MEMBER_NIJI);
    CHECK(a >= 0 && b >= 0 && c >= 0, "look_at fixtures resolve");
    if (a < 0 || b < 0 || c < 0) return;
    test_add_to_deck(&tg, a);
    test_add_to_deck(&tg, b);
    test_add_to_deck(&tg, c);

    /* deck source drains exactly `count` from the top */
    AbilityEffect e = {0};
    e.action = "look_at";
    e.source = "deck";
    e.count = 2;
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    int pool[8];
    CHECK_EQ(pool_get(0, pool, 8), 2, "look_at from deck fills the pool with count cards");
    CHECK(pool[0] == a && pool[1] == b, "look_at from deck preserves top-down order");
    CHECK_EQ(tg.state.p[0].deck.n, 1, "look_at from deck drains the looked cards");
    rb_resume_with_choice(&tg.state, -1);

    /* `all` takes the whole zone regardless of count (look.rs:1023-1029) */
    e.count = 1;
    fx_set(&e, 0, "all", "true");
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(pool_get(0, pool, 8), 3, "look_at with all=true takes the whole zone");
    rb_resume_with_choice(&tg.state, -1);

    /* hand source peeks — it must not remove the card from the hand */
    test_add_to_hand(&tg, a);
    test_add_to_hand(&tg, b);
    e.count = 2;
    e.n_extra = 0;
    e.source = "hand";
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(pool_get(0, pool, 8), 2, "look_at from hand fills the pool");
    CHECK_EQ(tg.state.p[0].hand.n, 2, "look_at from hand only peeks");
    rb_resume_with_choice(&tg.state, -1);
    CHECK_EQ(tg.state.p[0].hand.n, 2, "hand-sourced look is fully restored on decline");

    /* stage source skips empty slots and respects count */
    tg.state.p[0].stage[0] = a;
    tg.state.p[0].stage[2] = c;
    e.source = "stage";
    e.count = 5;
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(pool_get(0, pool, 8), 2, "look_at from stage skips the empty slot");
    CHECK(pool[0] == a && pool[1] == c, "look_at from stage preserves slot order");
    rb_resume_with_choice(&tg.state, -1);

    /* discard source */
    tg.state.p[0].discard.n = 0;
    test_add_to_discard(&tg, b);
    e.source = "discard";
    e.count = 3;
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(pool_get(0, pool, 8), 1, "look_at from the waitroom takes the whole zone");
    rb_resume_with_choice(&tg.state, -1);
}

/* Q85 / rule 10.2.2.2 — look N from a short deck draws, refreshes, draws the rest. */
static void test_look_at_short_deck_refresh(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int top = test_new_id(&tg, MEMBER_MUS);
    int w1 = test_new_id(&tg, MEMBER_MUS_HI);
    int w2 = test_new_id(&tg, MEMBER_NIJI);
    CHECK(top >= 0 && w1 >= 0 && w2 >= 0, "refresh fixtures resolve");
    if (top < 0 || w1 < 0 || w2 < 0) return;
    test_add_to_deck(&tg, top);
    test_add_to_discard(&tg, w1);
    test_add_to_discard(&tg, w2);
    AbilityEffect e = {0};
    e.action = "look_at";
    e.source = "deck_top";
    e.count = 3;
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    int pool[8];
    CHECK_EQ(pool_get(0, pool, 8), 3, "short look refreshes the waitroom to complete the count");
    CHECK_EQ(tg.state.p[0].discard.n, 0, "the refresh empties the waitroom");
    rb_resume_with_choice(&tg.state, -1);
}

/* ═══════════════════════════════════════════════════════════════════════
   2. reveal — offer_reveal_choice / reveal_available (look.rs:205-400)
   ═══════════════════════════════════════════════════════════════════════ */
static void test_reveal_hand_offer_and_metadata(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int a = test_new_id(&tg, MEMBER_MUS);
    int b = test_new_id(&tg, LIVE_NIJI);
    int c = test_new_id(&tg, MEMBER_NIJI);
    CHECK(a >= 0 && b >= 0 && c >= 0, "reveal hand fixtures resolve");
    if (a < 0 || b < 0 || c < 0) return;
    test_add_to_hand(&tg, a);
    test_add_to_hand(&tg, b);
    test_add_to_hand(&tg, c);

    AbilityEffect e = {0};
    e.action = "reveal";
    e.source = "hand";
    e.count = 1;
    e.is_optional = 1;
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK(rb_has_pending_choice(&tg.state), "reveal from hand with count < available prompts");
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch && ch->zone[0] && !strcmp(ch->zone, "hand"), "reveal prompt addresses the hand zone");
    CHECK_EQ(ch ? ch->count : -1, 1, "reveal prompt offers the effect count");
    CHECK_EQ(ch ? ch->allow_skip : -1, 1, "an optional reveal allows skip");
    CHECK_EQ(ch ? ch->is_reveal : -1, 1, "the reveal prompt is flagged is_reveal");
    CHECK(ch && ch->target_player_id[0] == 'p' && ch->target_player_id[1] == '1',
          "the reveal prompt names the resolving player");
    CHECK_EQ(tg.state.n_revealed, 0, "an offered reveal does not pre-record the cards");

    /* any_number offers the whole zone */
    tg.state.queue.has_pending = 0;
    e.n_extra = 0;
    fx_set(&e, 0, "any_number", "true");
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    ch = rb_get_pending_choice(&tg.state);
    CHECK_EQ(ch ? ch->count : -1, 3, "any_number reveal offers every available card");
    CHECK_EQ(ch ? ch->allow_skip : -1, 1, "any_number reveal allows skip");
    rb_resume_with_choice(&tg.state, -1);
}

static void test_reveal_hand_automatic_when_count_covers_all(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int a = test_new_id(&tg, MEMBER_MUS);
    int b = test_new_id(&tg, LIVE_NIJI);
    CHECK(a >= 0 && b >= 0, "automatic reveal fixtures resolve");
    if (a < 0 || b < 0) return;
    test_add_to_hand(&tg, a);
    test_add_to_hand(&tg, b);
    AbilityEffect e = {0};
    e.action = "reveal";
    e.source = "hand";
    e.count = 2;
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK(!rb_has_pending_choice(&tg.state),
          "a mandatory reveal covering the whole zone needs no prompt");
    CHECK_EQ(tg.state.n_revealed, 2, "the whole hand is revealed without a prompt");
    CHECK(revealed_has(&tg.state, a) && revealed_has(&tg.state, b),
          "the automatic reveal records the hand contents");
    CHECK_EQ(tg.state.p[0].hand.n, 2, "reveal never removes cards from the source zone");
}

/* look.rs:377-379 — `blind` only decorates the prompt; the revealed pool is
   still recorded on the automatic path. */
static void test_reveal_blind_still_records(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int a = test_new_id(&tg, MEMBER_MUS);
    int b = test_new_id(&tg, LIVE_NIJI);
    CHECK(a >= 0 && b >= 0, "blind reveal fixtures resolve");
    if (a < 0 || b < 0) return;
    test_add_to_hand(&tg, a);
    test_add_to_hand(&tg, b);
    AbilityEffect e = {0};
    e.action = "reveal";
    e.source = "hand";
    e.count = 2;
    fx_set(&e, 0, "blind", "true");
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.n_revealed, 2, "a blind reveal still records the revealed pool");
}

/* look.rs:353-373 — the deck arm of reveal is a PEEK, the card stays until a
   conditional move_cards consumes it. */
static void test_reveal_deck_is_a_peek(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int a = test_new_id(&tg, MEMBER_MUS);
    int b = test_new_id(&tg, MEMBER_NIJI);
    int c = test_new_id(&tg, LIVE_NIJI);
    CHECK(a >= 0 && b >= 0 && c >= 0, "deck peek fixtures resolve");
    if (a < 0 || b < 0 || c < 0) return;
    test_add_to_deck(&tg, a);
    test_add_to_deck(&tg, b);
    test_add_to_deck(&tg, c);
    AbilityEffect e = {0};
    e.action = "reveal";
    e.source = "deck";
    e.count = 2;
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.n_revealed, 2, "reveal from the deck reveals count cards");
    CHECK(revealed_has(&tg.state, a) && revealed_has(&tg.state, b),
          "reveal from the deck peeks at the top of the deck");
    CHECK_EQ(tg.state.p[0].deck.n, 3, "reveal from the deck does not drain it");
}

/* look.rs:364-370 + 443-465 — the looked_at arm filters by card type and
   heart colours before revealing. */
static void test_reveal_looked_at_filters_candidates(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int blue_live = test_new_id(&tg, LIVE_BLUE);
    int green_live = test_new_id(&tg, LIVE_NIJI);
    int member = test_new_id(&tg, MEMBER_DIA);
    CHECK(blue_live >= 0 && green_live >= 0 && member >= 0, "looked_at reveal fixtures resolve");
    if (blue_live < 0 || green_live < 0 || member < 0) return;
    rb_look_clear(0);
    rb_look_add(0, blue_live);
    rb_look_add(0, green_live);
    rb_look_add(0, member);

    AbilityEffect e = {0};
    e.action = "reveal";
    e.source = "looked_at";
    e.count = 3;
    strcpy(e.card_type_field, "live_card");
    fx_set(&e, 0, "heart_colors", "heart04");
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.n_revealed, 1, "looked_at reveal keeps only the cards passing the filter");
    CHECK(revealed_has(&tg.state, blue_live), "the blue live is the revealed card");
    CHECK(!revealed_has(&tg.state, green_live) && !revealed_has(&tg.state, member),
          "non-matching looked_at cards are not revealed");
}

/* ═══════════════════════════════════════════════════════════════════════
   3. reveal_per_group (look.rs:1136-1197)
   ═══════════════════════════════════════════════════════════════════════ */
static void test_reveal_per_group_sources(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int a = test_new_id(&tg, MEMBER_MUS);
    int b = test_new_id(&tg, MEMBER_NIJI);
    int c = test_new_id(&tg, LIVE_MUS);
    CHECK(a >= 0 && b >= 0 && c >= 0, "reveal_per_group fixtures resolve");
    if (a < 0 || b < 0 || c < 0) return;

    /* hand arm reveals the whole hand */
    test_add_to_hand(&tg, a);
    test_add_to_hand(&tg, b);
    AbilityEffect e = {0};
    e.action = "reveal_per_group";
    e.source = "hand";
    e.count = 1;
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.n_revealed, 2, "reveal_per_group from hand reveals every card");
    CHECK_EQ(tg.state.p[0].hand.n, 2, "reveal_per_group does not drain the hand");

    /* deck arm takes `count` from the top without draining */
    tg.state.n_revealed = 0;
    test_add_to_deck(&tg, a);
    test_add_to_deck(&tg, b);
    test_add_to_deck(&tg, c);
    e.source = "deck";
    e.count = 2;
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.n_revealed, 2, "reveal_per_group from deck respects count");
    CHECK_EQ(tg.state.p[0].deck.n, 3, "reveal_per_group from deck does not drain");

    /* looked_at arm reveals the pool */
    tg.state.n_revealed = 0;
    rb_look_clear(0);
    rb_look_add(0, a);
    rb_look_add(0, c);
    e.source = "looked_at";
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.n_revealed, 2, "reveal_per_group from looked_at reveals the pool");

    /* waitroom arm reveals the whole waitroom */
    tg.state.n_revealed = 0;
    tg.state.p[0].discard.n = 0;
    test_add_to_discard(&tg, a);
    test_add_to_discard(&tg, b);
    e.source = "waitroom";
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.n_revealed, 2, "reveal_per_group from the waitroom reveals it all");

    /* an unsupported source yields nothing (look.rs:1162 `_ => vec![]`) */
    tg.state.n_revealed = 0;
    e.source = "stage";
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.n_revealed, 0, "reveal_per_group ignores unsupported sources");
}

/* ═══════════════════════════════════════════════════════════════════════
   4. reveal_until (look.rs:1201-1367)
   ═══════════════════════════════════════════════════════════════════════ */
static void test_reveal_until_live_card(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int m1 = test_new_id(&tg, MEMBER_MUS);
    int m2 = test_new_id(&tg, MEMBER_NIJI);
    int live = test_new_id(&tg, LIVE_NIJI);
    int m3 = test_new_id(&tg, MEMBER_MUS_HI);
    CHECK(m1 >= 0 && m2 >= 0 && live >= 0 && m3 >= 0, "reveal_until fixtures resolve");
    if (m1 < 0 || m2 < 0 || live < 0 || m3 < 0) return;
    test_add_to_deck(&tg, m1);
    test_add_to_deck(&tg, m2);
    test_add_to_deck(&tg, live);
    test_add_to_deck(&tg, m3);
    AbilityEffect e = {0};
    e.action = "reveal_until_live_card";
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    int pool[8];
    CHECK_EQ(pool_get(0, pool, 8), 3, "reveal_until_live_card stops on the live card");
    CHECK(pool[0] == m1 && pool[1] == m2 && pool[2] == live,
          "the looked-at pool keeps every revealed card in draw order");
    CHECK_EQ(tg.state.p[0].deck.n, 1, "reveal_until draws only up to the live card");
    CHECK_EQ(tg.state.n_revealed, 3, "reveal_until records each revealed card");
    CHECK(!rb_has_pending_choice(&tg.state),
          "reveal_until only fills looked-at for its structural parent");
}

static void test_reveal_until_live_card_refreshes(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int m1 = test_new_id(&tg, MEMBER_MUS);
    int live = test_new_id(&tg, LIVE_NIJI);
    CHECK(m1 >= 0 && live >= 0, "reveal_until refresh fixtures resolve");
    if (m1 < 0 || live < 0) return;
    test_add_to_deck(&tg, m1);
    test_add_to_discard(&tg, live);
    AbilityEffect e = {0};
    e.action = "reveal_until_live_card";
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    int pool[8];
    CHECK_EQ(pool_get(0, pool, 8), 2, "reveal_until refreshes the waitroom when the deck empties");
    CHECK_EQ(tg.state.p[0].discard.n, 0, "the refresh consumes the waitroom");
    CHECK(pool[0] == m1, "the pre-refresh card stays first in the pool");
}

static void test_reveal_until_chosen_card_type(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int live = test_new_id(&tg, LIVE_NIJI);
    int m1 = test_new_id(&tg, MEMBER_MUS);
    int m2 = test_new_id(&tg, MEMBER_NIJI);
    CHECK(live >= 0 && m1 >= 0 && m2 >= 0, "reveal_until_chosen fixtures resolve");
    if (live < 0 || m1 < 0 || m2 < 0) return;
    test_add_to_deck(&tg, live);
    test_add_to_deck(&tg, m1);
    test_add_to_deck(&tg, m2);
    AbilityEffect e = {0};
    e.action = "reveal_until_chosen_card";
    fx_set(&e, 0, "card_type", "member_card");
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    int pool[8];
    CHECK_EQ(pool_get(0, pool, 8), 2, "reveal_until_chosen_card stops on the chosen card type");
    CHECK(pool[0] == live && pool[1] == m1, "the pool is the draw order up to the match");
}

/* look.rs:1326-1367 — execute_reveal_until_target moves the matched card to
   the FRONT of the looked-at pool and applies the member_card cost gate. */
static void test_reveal_until_target_cost_gate(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int costly = test_new_id(&tg, MEMBER_NIJI);   /* cost 13 */
    int cheap = test_new_id(&tg, MEMBER_LIELLA_9); /* cost 9  */
    int filler = test_new_id(&tg, MEMBER_MUS);
    CHECK(costly >= 0 && cheap >= 0 && filler >= 0, "reveal_until_target fixtures resolve");
    if (costly < 0 || cheap < 0 || filler < 0) return;
    test_add_to_deck(&tg, costly);
    test_add_to_deck(&tg, cheap);
    test_add_to_deck(&tg, filler);
    AbilityEffect e = {0};
    e.action = "reveal_until_target";
    fx_set(&e, 0, "card_type", "member_card");
    fx_set(&e, 1, "cost_limit", "10");
    fx_set(&e, 2, "cost_limit_operator", "<=");
    rb_effect_reveal_until_target(&tg.state, 0, &e);
    int pool[8];
    CHECK_EQ(pool_get(0, pool, 8), 2, "reveal_until_target stops at the first card passing cost");
    CHECK(pool[0] == cheap && pool[1] == costly,
          "the matched card is moved to the front of the looked-at pool");
    CHECK_EQ(tg.state.n_revealed, 2, "reveal_until_target records every revealed card");
}

static void test_reveal_until_target_no_match_clears_pool(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int a = test_new_id(&tg, MEMBER_MUS);
    int b = test_new_id(&tg, MEMBER_NIJI);
    CHECK(a >= 0 && b >= 0, "reveal_until_target miss fixtures resolve");
    if (a < 0 || b < 0) return;
    test_add_to_deck(&tg, a);
    test_add_to_deck(&tg, b);
    AbilityEffect e = {0};
    e.action = "reveal_until_target";
    fx_set(&e, 0, "card_type", "live_card");
    rb_effect_reveal_until_target(&tg.state, 0, &e);
    int pool[8];
    CHECK_EQ(pool_get(0, pool, 8), 0, "an unmatched reveal_until_target empties the pool");
    CHECK_EQ(tg.state.p[0].deck.n, 0, "the failed search still drains the deck");
    CHECK_EQ(tg.state.n_revealed, 2, "the failed search still records what it revealed");
}

/* ═══════════════════════════════════════════════════════════════════════
   5. select from a zone (look.rs:405-483 execute_select)
   ═══════════════════════════════════════════════════════════════════════ */
static void test_select_offers_filtered_pool(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int m1 = test_new_id(&tg, MEMBER_MUS);
    int l1 = test_new_id(&tg, LIVE_NIJI);
    int m2 = test_new_id(&tg, MEMBER_NIJI);
    CHECK(m1 >= 0 && l1 >= 0 && m2 >= 0, "select fixtures resolve");
    if (m1 < 0 || l1 < 0 || m2 < 0) return;
    test_add_to_hand(&tg, m1);
    test_add_to_hand(&tg, l1);
    test_add_to_hand(&tg, m2);

    AbilityEffect e = {0};
    e.action = "select";
    e.source = "hand";
    e.count = 1;
    strcpy(e.card_type_field, "member_card");
    rb_effect_select(&tg.state, 0, &e);
    CHECK(rb_has_pending_choice(&tg.state), "select from a zone emits a prompt");
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch && !strcmp(ch->zone, "hand"), "the select prompt names the source zone");
    CHECK_EQ(ch ? ch->count : -1, 1, "the select prompt offers the effect count");
    int pool[8];
    CHECK_EQ(pool_get(0, pool, 8), 2,
             "select narrows the pool to the cards passing the card_type filter");
    CHECK(pool[0] == m1 && pool[1] == m2, "the select pool keeps the source-zone order");
    rb_resume_with_choice(&tg.state, 0);
    CHECK_EQ(tg.state.n_selected_cards, 1, "a resolved select records the chosen card");
    CHECK_EQ(tg.state.selected_cards[0], m1, "the select records the card the pool pointed at");
}

static void test_select_count_clamped_to_pool(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int a = test_new_id(&tg, MEMBER_MUS);
    int b = test_new_id(&tg, MEMBER_NIJI);
    CHECK(a >= 0 && b >= 0, "select clamp fixtures resolve");
    if (a < 0 || b < 0) return;
    test_add_to_hand(&tg, a);
    test_add_to_hand(&tg, b);
    AbilityEffect e = {0};
    e.action = "select";
    e.source = "hand";
    e.count = 5;
    rb_effect_select(&tg.state, 0, &e);
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK_EQ(ch ? ch->count : -1, 2, "the select count is clamped to the pool size");
    rb_resume_with_choice(&tg.state, -1);
}

static void test_select_looked_at_and_stage(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int a = test_new_id(&tg, MEMBER_MUS);
    int b = test_new_id(&tg, MEMBER_NIJI);
    CHECK(a >= 0 && b >= 0, "select looked_at/stage fixtures resolve");
    if (a < 0 || b < 0) return;

    rb_look_clear(0);
    rb_look_add(0, a);
    rb_look_add(0, b);
    AbilityEffect e = {0};
    e.action = "select";
    e.source = "looked_at";
    e.count = 2;
    rb_effect_select(&tg.state, 0, &e);
    int pool[8];
    CHECK_EQ(pool_get(0, pool, 8), 2, "select from looked_at offers the looked-at set");
    CHECK(rb_has_pending_choice(&tg.state), "select from looked_at prompts");
    rb_resume_with_choice(&tg.state, -1);

    tg.state.p[0].stage[0] = a;
    tg.state.p[0].stage[2] = b;
    e.source = "stage";
    rb_effect_select(&tg.state, 0, &e);
    CHECK_EQ(pool_get(0, pool, 8), 2, "select from stage offers the occupied slots only");
    CHECK(pool[0] == a && pool[1] == b, "the stage select pool keeps slot order");
    rb_resume_with_choice(&tg.state, -1);
}

/* ═══════════════════════════════════════════════════════════════════════
   6. select_cards as an independent sequential step (look.rs:681-901)
   ═══════════════════════════════════════════════════════════════════════ */
static void test_select_cards_from_looked_at(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int a = test_new_id(&tg, MEMBER_MUS);
    int b = test_new_id(&tg, MEMBER_NIJI);
    int c = test_new_id(&tg, MEMBER_MUS_HI);
    CHECK(a >= 0 && b >= 0 && c >= 0, "select_cards fixtures resolve");
    if (a < 0 || b < 0 || c < 0) return;
    rb_look_clear(0);
    rb_look_add(0, a);
    rb_look_add(0, b);
    rb_look_add(0, c);

    AbilityEffect e = {0};
    e.action = "select_cards";
    e.destination = "hand";
    e.count = 5;
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(rb_has_pending_choice(&tg.state), "select_cards prompts over the looked-at set");
    CHECK_EQ(ch ? ch->count : -1, 3, "the select_cards count is clamped to the looked-at count");
    CHECK(ch && ch->n_filtered_indices == 3,
          "an unfiltered select_cards exposes every looked-at index");
    int indices[2] = {0, 2};
    rb_resume_with_choice_indices(&tg.state, indices, 2);
    CHECK(bag_has(&tg.state.p[0].hand, a) && bag_has(&tg.state.p[0].hand, c),
          "the chosen looked-at cards reach the destination");
    CHECK_EQ(tg.state.n_selected_cards, 2, "select_cards records every chosen card");
}

static void test_select_cards_filters_and_clamps(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int m1 = test_new_id(&tg, MEMBER_MUS);
    int l1 = test_new_id(&tg, LIVE_NIJI);
    int m2 = test_new_id(&tg, MEMBER_NIJI);
    CHECK(m1 >= 0 && l1 >= 0 && m2 >= 0, "filtered select_cards fixtures resolve");
    if (m1 < 0 || l1 < 0 || m2 < 0) return;
    rb_look_clear(0);
    rb_look_add(0, m1);
    rb_look_add(0, l1);
    rb_look_add(0, m2);

    AbilityEffect e = {0};
    e.action = "select_cards";
    e.destination = "hand";
    e.count = 2;
    strcpy(e.card_type_field, "member_card");
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch && ch->n_filtered_indices == 2,
          "select_cards exposes only the looked-at indices passing the filter");
    CHECK(ch && ch->filtered_indices[0] == 0 && ch->filtered_indices[1] == 2,
          "the filtered indices keep their original looked-at positions");
    rb_resume_with_choice_indices(&tg.state, ch->filtered_indices, 2);
    CHECK(bag_has(&tg.state.p[0].hand, m1) && bag_has(&tg.state.p[0].hand, m2),
          "only the member cards are selectable out of the looked-at set");
    CHECK(!bag_has(&tg.state.p[0].hand, l1), "the live card cannot be selected");
}

/* look.rs:804-812 — nothing matches: the whole looked-at pool is discarded and
   no impossible prompt is offered. */
static void test_select_cards_unmatched_discards_pool(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int a = test_new_id(&tg, MEMBER_MUS);
    int b = test_new_id(&tg, MEMBER_NIJI);
    CHECK(a >= 0 && b >= 0, "unmatched select_cards fixtures resolve");
    if (a < 0 || b < 0) return;
    rb_look_clear(0);
    rb_look_add(0, a);
    rb_look_add(0, b);

    AbilityEffect e = {0};
    e.action = "select_cards";
    e.destination = "hand";
    e.count = 1;
    strcpy(e.card_type_field, "live_card");
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK(!rb_has_pending_choice(&tg.state),
          "select_cards with no match offers no impossible prompt");
    CHECK_EQ(tg.state.p[0].discard.n, 2, "the unmatched looked-at pool is discarded");
    int pool[8];
    CHECK_EQ(pool_get(0, pool, 8), 0, "the unmatched pool is emptied");
}

/* ═══════════════════════════════════════════════════════════════════════
   7. look_and_select (look.rs:16-203)
   ═══════════════════════════════════════════════════════════════════════ */
static void test_look_and_select_options_are_or(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int m1 = test_new_id(&tg, MEMBER_MUS);
    int l1 = test_new_id(&tg, LIVE_NIJI);
    int m2 = test_new_id(&tg, MEMBER_NIJI);
    int l2 = test_new_id(&tg, LIVE_BLUE);
    CHECK(m1 >= 0 && l1 >= 0 && m2 >= 0 && l2 >= 0, "options fixtures resolve");
    if (m1 < 0 || l1 < 0 || m2 < 0 || l2 < 0) return;
    test_add_to_deck(&tg, m1);
    test_add_to_deck(&tg, l1);
    test_add_to_deck(&tg, m2);
    test_add_to_deck(&tg, l2);

    AbilityEffect opt_a = {0};
    strcpy(opt_a.card_type_field, "member_card");
    AbilityEffect opt_b = {0};
    strcpy(opt_b.card_type_field, "live_card");
    AbilityEffect look = {0};
    look.action = "look_at";
    look.source = "deck_top";
    look.count = 4;
    AbilityEffect select = {0};
    select.action = "select_cards";
    select.destination = "hand";
    select.count = 2;
    select.options[0] = &opt_a;
    select.options[1] = &opt_b;
    select.n_options = 2;
    fx_set(&select, 0, "discard_remaining", "true");
    AbilityEffect parent = {0};
    parent.action = "look_and_select";
    parent.look_action = &look;
    parent.select_action = &select;
    rb_execute_effect_ex(&tg.state, 0, &parent, -1);
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(rb_has_pending_choice(&tg.state), "a look_and_select with options prompts");
    CHECK_EQ(ch ? ch->count : -1, 2, "the option filter ORs so every looked-at card matches");
    CHECK(ch && ch->n_filtered_indices == 4,
          "every looked-at index matches one of the two options");
    int indices[2] = {0, 1};
    rb_resume_with_choice_indices(&tg.state, indices, 2);
    CHECK(bag_has(&tg.state.p[0].hand, m1) && bag_has(&tg.state.p[0].hand, l1),
          "the option-filtered picks reach the destination");
    CHECK_EQ(tg.state.p[0].discard.n, 2, "the unselected remainder goes to the waitroom");
}

static void test_look_and_select_heart_color_filter(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int blue = test_new_id(&tg, MEMBER_DIA);      /* hearts yellow,blue */
    int nocolor = test_new_id(&tg, MEMBER_LIELLA_2); /* hearts red,red   */
    int filler = test_new_id(&tg, MEMBER_MUS);
    CHECK(blue >= 0 && nocolor >= 0 && filler >= 0, "heart filter fixtures resolve");
    if (blue < 0 || nocolor < 0 || filler < 0) return;
    test_add_to_deck(&tg, nocolor);
    test_add_to_deck(&tg, blue);
    test_add_to_deck(&tg, filler);

    AbilityEffect look = {0};
    look.action = "look_at";
    look.source = "deck_top";
    look.count = 3;
    AbilityEffect select = {0};
    select.action = "select_cards";
    select.destination = "hand";
    select.count = 1;
    strcpy(select.card_type_field, "member_card");
    fx_set(&select, 0, "heart_colors", "heart04");
    fx_set(&select, 1, "discard_remaining", "true");
    AbilityEffect parent = {0};
    parent.action = "look_and_select";
    parent.look_action = &look;
    parent.select_action = &select;
    rb_execute_effect_ex(&tg.state, 0, &parent, -1);
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch && ch->n_filtered_indices == 1 && ch->filtered_indices[0] == 1,
          "only the heart-colour match is selectable from the looked-at set");
    CHECK_EQ(ch ? ch->count : -1, 1, "the offered count follows the matching cards");
    rb_resume_with_choice_indices(&tg.state, ch->filtered_indices, 1);
    CHECK(bag_has(&tg.state.p[0].hand, blue), "the heart-colour match reaches the hand");
    CHECK_EQ(tg.state.p[0].discard.n, 2, "the non-matching remainder is discarded");
}

static void test_look_and_select_cost_limit_filter(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int costly = test_new_id(&tg, MEMBER_NIJI);      /* cost 13 */
    int cheap = test_new_id(&tg, MEMBER_LIELLA_2);   /* cost 2  */
    CHECK(costly >= 0 && cheap >= 0, "cost filter fixtures resolve");
    if (costly < 0 || cheap < 0) return;
    test_add_to_deck(&tg, costly);
    test_add_to_deck(&tg, cheap);

    AbilityEffect look = {0};
    look.action = "look_at";
    look.source = "deck_top";
    look.count = 2;
    AbilityEffect select = {0};
    select.action = "select_cards";
    select.destination = "hand";
    select.count = 1;
    strcpy(select.card_type_field, "member_card");
    fx_set(&select, 0, "cost_limit", "4");
    fx_set(&select, 1, "cost_limit_operator", "<=");
    fx_set(&select, 2, "discard_remaining", "true");
    AbilityEffect parent = {0};
    parent.action = "look_and_select";
    parent.look_action = &look;
    parent.select_action = &select;
    rb_execute_effect_ex(&tg.state, 0, &parent, -1);
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch && ch->n_filtered_indices == 1 && ch->filtered_indices[0] == 1,
          "the cost gate removes the over-priced looked-at card");
    rb_resume_with_choice_indices(&tg.state, ch->filtered_indices, 1);
    CHECK(bag_has(&tg.state.p[0].hand, cheap), "the in-budget card is taken");
    CHECK_EQ(tg.state.p[0].discard.n, 1, "the over-priced card is discarded");
    CHECK(bag_has(&tg.state.p[0].discard, costly), "the over-priced card is the discarded one");
}

static void test_look_and_select_any_number_and_max(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int a = test_new_id(&tg, MEMBER_MUS);
    int b = test_new_id(&tg, MEMBER_NIJI);
    int c = test_new_id(&tg, MEMBER_MUS_HI);
    CHECK(a >= 0 && b >= 0 && c >= 0, "any_number fixtures resolve");
    if (a < 0 || b < 0 || c < 0) return;
    test_add_to_deck(&tg, a);
    test_add_to_deck(&tg, b);
    test_add_to_deck(&tg, c);

    AbilityEffect look = {0};
    look.action = "look_at";
    look.source = "deck_top";
    look.count = 3;
    AbilityEffect select = {0};
    select.action = "select_cards";
    select.destination = "hand";
    select.count = 1;
    fx_set(&select, 0, "any_number", "true");
    fx_set(&select, 1, "discard_remaining", "true");
    AbilityEffect parent = {0};
    parent.action = "look_and_select";
    parent.look_action = &look;
    parent.select_action = &select;
    rb_execute_effect_ex(&tg.state, 0, &parent, -1);
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK_EQ(ch ? ch->count : -1, 3, "any_number offers the whole matching looked-at set");
    CHECK_EQ(ch ? ch->allow_skip : -1, 1, "any_number allows skipping the pick");
    rb_resume_with_choice_indices(&tg.state, NULL, 0);
    CHECK_EQ(tg.state.p[0].hand.n, 0, "declining an any_number pick fetches nothing");
    CHECK_EQ(tg.state.p[0].discard.n, 3, "the declined looked-at set goes to the waitroom");

    /* `max` with a count larger than the pool clamps to the pool */
    rb_look_clear(0);
    tg.state.p[0].discard.n = 0;
    test_add_to_deck(&tg, a);
    test_add_to_deck(&tg, b);
    select.n_extra = 0;
    select.count = 4;
    fx_set(&select, 0, "max", "true");
    rb_execute_effect_ex(&tg.state, 0, &parent, -1);
    ch = rb_get_pending_choice(&tg.state);
    CHECK_EQ(ch ? ch->count : -1, 2, "a max select is clamped to the matching looked-at count");
    CHECK_EQ(ch ? ch->allow_skip : -1, 1, "a max select allows skipping");
    rb_resume_with_choice_indices(&tg.state, NULL, 0);
}

/* look.rs:60-81 discard_unmatched_looked_at */
static void test_look_and_select_unmatched_path(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int a = test_new_id(&tg, MEMBER_MUS);
    int b = test_new_id(&tg, MEMBER_NIJI);
    int d = test_new_id(&tg, MEMBER_MUS_HI);
    CHECK(a >= 0 && b >= 0 && d >= 0, "unmatched fixtures resolve");
    if (a < 0 || b < 0 || d < 0) return;
    test_add_to_deck(&tg, a);
    test_add_to_deck(&tg, b);
    test_add_to_deck(&tg, d);

    AbilityEffect look = {0};
    look.action = "look_at";
    look.source = "deck_top";
    look.count = 2;
    AbilityEffect select = {0};
    select.action = "select_cards";
    select.destination = "hand";
    select.count = 1;
    strcpy(select.card_type_field, "live_card");
    AbilityEffect followup = {0};
    followup.action = "modify_score";
    followup.count = 5;
    AbilityEffect parent = {0};
    parent.action = "look_and_select";
    parent.look_action = &look;
    parent.select_action = &select;
    parent.followup_action = &followup;
    rb_execute_effect_ex(&tg.state, 0, &parent, -1);
    CHECK(!rb_has_pending_choice(&tg.state), "the unmatched path offers no prompt");
    CHECK_EQ(tg.state.p[0].discard.n, 2, "the whole looked-at pool is discarded");
    CHECK_EQ(tg.state.p[0].deck.n, 1, "the deck below the looked-at set is untouched");
    CHECK_EQ(tg.state.p[0].score, 5, "the followup action still runs on the unmatched path");
}

/* look.rs:176-203 — a look_action that is not look_at runs as a nested effect. */
static void test_look_and_select_nested_look_action(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int a = test_new_id(&tg, MEMBER_MUS);
    int b = test_new_id(&tg, MEMBER_NIJI);
    int d = test_new_id(&tg, MEMBER_MUS_HI);
    CHECK(a >= 0 && b >= 0 && d >= 0, "nested look_action fixtures resolve");
    if (a < 0 || b < 0 || d < 0) return;
    test_add_to_deck(&tg, a);
    test_add_to_deck(&tg, b);
    test_add_to_deck(&tg, d);

    AbilityEffect look = {0};
    look.action = "reveal_until_live_card";
    look.source = "deck_top";
    AbilityEffect select = {0};
    select.action = "select_cards";
    select.destination = "hand";
    select.count = 1;
    fx_set(&select, 0, "discard_remaining", "true");
    AbilityEffect parent = {0};
    parent.action = "look_and_select";
    parent.look_action = &look;
    parent.select_action = &select;
    /* No live card anywhere: the nested reveal drains the deck and the whole
       revealed run becomes the looked-at set (look.rs:1313-1324), so the
       select step still offers a pick. */
    rb_execute_effect_ex(&tg.state, 0, &parent, -1);
    int pool[8];
    CHECK_EQ(pool_get(0, pool, 8), 3, "the nested reveal_until fills the looked-at set");
    CHECK(rb_has_pending_choice(&tg.state), "the select step runs over the nested pool");
    CHECK_EQ(tg.state.p[0].deck.n, 0, "the nested reveal_until drained the deck");
    rb_resume_with_choice_indices(&tg.state, NULL, 0);
}

/* Remainder routing — keep_shuffle_under finalisation. */
static void test_look_and_select_remainder_routes(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int a = test_new_id(&tg, MEMBER_MUS);
    int b = test_new_id(&tg, MEMBER_NIJI);
    int c = test_new_id(&tg, MEMBER_MUS_HI);
    CHECK(a >= 0 && b >= 0 && c >= 0, "remainder fixtures resolve");
    if (a < 0 || b < 0 || c < 0) return;
    test_add_to_deck(&tg, a);
    test_add_to_deck(&tg, b);
    test_add_to_deck(&tg, c);

    AbilityEffect look = {0};
    look.action = "look_at";
    look.source = "deck_top";
    look.count = 3;
    AbilityEffect select = {0};
    select.action = "select_cards";
    select.destination = "hand";
    select.count = 1;
    fx_set(&select, 0, "remainder_destination", "deck_bottom");
    AbilityEffect parent = {0};
    parent.action = "look_and_select";
    parent.look_action = &look;
    parent.select_action = &select;
    rb_execute_effect_ex(&tg.state, 0, &parent, -1);
    rb_resume_with_choice_indices(&tg.state, (const int[]){0}, 1);
    CHECK(bag_has(&tg.state.p[0].hand, a), "the pick reaches the hand");
    CHECK_EQ(tg.state.p[0].discard.n, 0, "remainder_destination keeps the rest out of the waitroom");
    CHECK(bag_has(&tg.state.p[0].deck, b) && bag_has(&tg.state.p[0].deck, c),
          "remainder_destination=deck_bottom returns the rest under the deck");
}

/* ═══════════════════════════════════════════════════════════════════════
   8. Real decoded ability: 国木田花丸 PL!S-bp5-007-R
   ライブ成功時：自分のデッキの上から4枚を見る。その中から、
   ハートがハート04のメンバーカードを1枚選び、手札に加え、
   残りを控え室に置く。
   ═══════════════════════════════════════════════════════════════════════ */
static void test_real_decoded_hanamaru_ability(void)
{
    int id = rb_find_card_by_no("PL!S-bp5-007-R");
    CHECK(id >= 0, "the Hanamaru fixture resolves");
    if (id < 0) return;
    Ability ability;
    memset(&ability, 0, sizeof(ability));
    if (!rb_decode_card_ability((uint32_t)id, 0, &ability) || !ability.effect ||
        !ability.effect->look_action || !ability.effect->select_action) {
        CHECK(0, "the Hanamaru look_and_select ability decodes with both branches");
        rb_free_ability(&ability);
        return;
    }
    CHECK(1, "the Hanamaru look_and_select ability decodes with both branches");
    AbilityEffect *look = ability.effect->look_action;
    AbilityEffect *select = ability.effect->select_action;
    CHECK(look && !strcmp(look->action, "look_at") && !strcmp(look->source, "deck_top") &&
              look->count == 4,
          "the decoded look_action looks at four deck-top cards");

    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int plain = test_new_id(&tg, MEMBER_MUS);
    int dia = test_new_id(&tg, MEMBER_DIA);
    int live = test_new_id(&tg, LIVE_NIJI);
    int cost17 = test_new_id(&tg, MEMBER_MUS_HI);
    CHECK(plain >= 0 && dia >= 0 && live >= 0 && cost17 >= 0,
          "the Hanamaru deck fixtures resolve");
    if (plain < 0 || dia < 0 || live < 0 || cost17 < 0) {
        rb_free_ability(&ability);
        return;
    }
    test_add_to_deck(&tg, plain);
    test_add_to_deck(&tg, dia);
    test_add_to_deck(&tg, live);
    test_add_to_deck(&tg, cost17);
    rb_execute_effect_ex(&tg.state, 0, ability.effect, dia);
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(rb_has_pending_choice(&tg.state), "the Hanamaru ability prompts over the looked four");
    CHECK_EQ(ch ? ch->count : -1, 1, "the Hanamaru pick is a single card");
    CHECK(ch && ch->n_filtered_indices == 1 && ch->filtered_indices[0] == 1,
          "only the member carrying a heart04 is selectable");
    rb_resume_with_choice_indices(&tg.state, ch->filtered_indices, 1);
    CHECK(bag_has(&tg.state.p[0].hand, dia), "the heart04 member joins the hand");
    CHECK_EQ(tg.state.p[0].discard.n, 3, "the other three looked cards go to the waitroom");
    CHECK_EQ(tg.state.p[0].deck.n, 0, "the whole top four left the deck");
    rb_free_ability(&ability);
}

/* ═══════════════════════════════════════════════════════════════════════
   9. Real decoded ability with no match: 黒澤ダイヤ PL!S-pb1-013-N
   ═══════════════════════════════════════════════════════════════════════ */
static void test_real_decoded_dia_no_match(void)
{
    int id = rb_find_card_by_no("PL!S-pb1-013-N");
    CHECK(id >= 0, "the Dia fixture resolves");
    if (id < 0) return;
    Ability ability;
    memset(&ability, 0, sizeof(ability));
    if (!rb_decode_card_ability((uint32_t)id, 0, &ability) || !ability.effect) {
        CHECK(0, "the Dia look_and_select ability decodes");
        rb_free_ability(&ability);
        return;
    }
    CHECK(1, "the Dia look_and_select ability decodes");
    TestGame tg;
    test_game_new(&tg);
    clear_player(&tg.state.p[0]);
    int a = test_new_id(&tg, MEMBER_MUS);
    int b = test_new_id(&tg, MEMBER_NIJI);
    int c = test_new_id(&tg, MEMBER_MUS_HI);
    int d = test_new_id(&tg, MEMBER_LIELLA_2);
    CHECK(a >= 0 && b >= 0 && c >= 0 && d >= 0, "the Dia deck fixtures resolve");
    if (a < 0 || b < 0 || c < 0 || d < 0) {
        rb_free_ability(&ability);
        return;
    }
    test_add_to_deck(&tg, a);
    test_add_to_deck(&tg, b);
    test_add_to_deck(&tg, c);
    test_add_to_deck(&tg, d);
    rb_execute_effect_ex(&tg.state, 0, ability.effect, d);
    CHECK(!rb_has_pending_choice(&tg.state),
          "an all-heart04-less deck yields no pick prompt");
    CHECK_EQ(tg.state.p[0].discard.n, 4, "all four looked cards go to the waitroom");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "no card joins the hand");
    CHECK_EQ(tg.state.p[0].deck.n, 0, "the four looked cards left the deck");
    rb_free_ability(&ability);
}

/* ═══════════════════════════════════════════════════════════════════════ */
int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    test_look_at_counts_and_zones();
    test_look_at_short_deck_refresh();
    test_reveal_hand_offer_and_metadata();
    test_reveal_hand_automatic_when_count_covers_all();
    test_reveal_blind_still_records();
    test_reveal_deck_is_a_peek();
    test_reveal_looked_at_filters_candidates();
    test_reveal_per_group_sources();
    test_reveal_until_live_card();
    test_reveal_until_live_card_refreshes();
    test_reveal_until_chosen_card_type();
    test_reveal_until_target_cost_gate();
    test_reveal_until_target_no_match_clears_pool();
    test_select_offers_filtered_pool();
    test_select_count_clamped_to_pool();
    test_select_looked_at_and_stage();
    test_select_cards_from_looked_at();
    test_select_cards_filters_and_clamps();
    test_select_cards_unmatched_discards_pool();
    test_look_and_select_options_are_or();
    test_look_and_select_heart_color_filter();
    test_look_and_select_cost_limit_filter();
    test_look_and_select_any_number_and_max();
    test_look_and_select_unmatched_path();
    test_look_and_select_nested_look_action();
    test_look_and_select_remainder_routes();
    test_real_decoded_hanamaru_ability();
    test_real_decoded_dia_no_match();
    rb_unload();
    printf("checks=%d failures=%d\n", checks, failures);
    if (failures) return 1;
    printf("ALL LOOK REVEAL PARITY CHECKS PASSED\n");
    return 0;
}
