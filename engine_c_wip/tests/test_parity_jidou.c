/* Parity suite for the Rust 自動 (jidou) cluster.
 *
 * Source cluster: engine/tests/test_modules/jidou/ — primarily `leaves_stage/`
 * (the sub-folder this cluster's behaviour is named for), plus the watcher
 * scenarios from `debut_watch/`, `energy_watch/` and `movement/` that share the
 * same engine paths (TAS scan, stage→waitroom 自動, group filters, self wait cost).
 *
 * Every test names the Rust file + test it mirrors. Expectations are taken from
 * the Rust assertions verbatim; where the C engine genuinely diverges the
 * expectation is downgraded to EXPECTED_GAP(...) (counted separately from a real
 * failure) and the divergence is listed in the report.
 *
 * C API notes (the shims differ from Rust's TestGame in three places):
 *   - Rust `game.state.set_recently_moved_cards(v)` / `recently_moved_from_zone`
 *     -> C `state.recently_moved[] / n_recently_moved` (there is no
 *     `recently_moved_from_zone` field in GameState).
 *   - Rust `TurnEngine::trigger_auto_abilities_for_player(&state, &pid)` +
 *     `state.process_pending_auto_abilities(&pid)` -> C
 *     `rb_queue_trigger_abilities(g, pl, "自動")` +
 *     `rb_process_pending_auto_abilities(g)`, which is the pair that reproduces
 *     `abilities.rs:407-414` + `abilities.rs:1791` (the public
 *     `rb_trigger_auto_abilities_for_player` drops the moved-card snapshot).
 *   - Rust `turn_movements.push(MovementEvent{..})` -> C `state.batch_movements[]`.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>

static int failures;
static int assertions;
static int gaps;

/* A real assertion: failing it fails the suite. */
#define CHECK(condition, message) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    assertions++; \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* A known C-vs-Rust divergence: counted separately, never fails the suite. */
#define EXPECTED_GAP(condition, desc) do { \
    assertions++; \
    if (!(condition)) { \
        printf("GAP: %s\n", desc); \
        gaps++; \
    } else { \
        printf("ok (gap closed): %s\n", desc); \
    } \
} while (0)

/* ── card constants (from the Rust tests) ───────────────────────────────── */
#define KANON        "PL!SP-bp7-001-R"   /* 澁谷かのん  自動 ab#0/ab#1 */
#define SUMMER_ARR   "PL!SP-sd1-004-SD"  /* 平安名すみれ (Liella! arriver) */
#define FILLER       "PL!-sd1-010-SD"
#define ARRIVER_GEN  "PL!-sd1-002-SD"   /* generic cost-6 filler arriver */
#define KASUMI       "PL!N-bp7-014-N"   /* 虹ヶ咲, stage→waitroom live recovery */
#define NIJI_LIVE    "PL!N-bp1-026-L"
#define NIJI_MEMBER  "PL!N-bp7-001-R"   /* 上原歩夢 */
#define TOKO         "PL!HS-bp2-015-N"  /* 藤島慈 draw2 discard1 */
#define RURINO       "PL!HS-bp6-019-N"  /* 大沢瑠璃乃 draw-then-discard */
#define LIELIA_LIVE  "PL!SP-bp1-023-L"
#define OTHER_MEMBER "PL!S-sd1-001-SD"
#define YUIGURI      "PL!HS-bp2-013-N"  /* 夕霧綴理 look-5 */
#define SAYAKA       "PL!HS-bp6-018-N"  /* 村野さやか heart05+blade */
#define SETSUNA      "PL!N-bp7-019-N"   /* 虹ノ空せつ菜 energy under arriver */
#define ENERGY       "LL-E-001-SD"
#define OSAWA_RINO   "PL!HS-bp5-003-AR" /* 大沢瑠璃乃 any-player position change */
#define HONOKA       "PL!-PR-001-PR"    /* stage→waitroom 自動 */
#define KOHAKU       "PL!HS-bp2-012-N"  /* stage→waitroom look_and_select */
#define HANABIKO     "PL!HS-bp6-017-N"  /* 日野下花帆 optional discard + recover */
#define HANABI_LIVE  "PL!N-bp4-026-L"
#define HANABI_MEMB  "PL!N-PR-003-PR"
#define NIJI_AUTO    "PL!N-bp5-005-R+"  /* 虹ヶ咲 partner cost-gate auto */
#define CL_RURINO    "PL!HS-cl1-003-CL" /* 大沢瑠璃乃 (CL) self-wait cost + blade */
#define MIRA_PRINT1  "PL!HS-bp5-003-R＋"
#define MIRA_PRINT2  "PL!HS-bp5-003-AR"

/* ── helpers ─────────────────────────────────────────────────────────────── */

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}

static int waitroom_has(TestGame *tg, int cid) { return bag_has(&tg->state.p[0].discard, cid); }
static int hand_has(TestGame *tg, int cid)     { return bag_has(&tg->state.p[0].hand, cid); }
static int deck_has(TestGame *tg, int cid)     { return bag_has(&tg->state.p[0].deck, cid); }

static int stage_has(TestGame *tg, int pl, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg->state.p[pl].stage[i] == cid) return 1;
    return 0;
}

static int under_has(TestGame *tg, int pl, int area, int cid)
{
    if (area < 0 || area >= RB_STAGE_SIZE) return 0;
    return bag_has(&tg->state.p[pl].under_cards[area], cid);
}

static int pending(TestGame *tg) { return test_has_pending_choice(tg); }
static const char *pending_kind(TestGame *tg) { return test_pending_choice_type(tg); }

/* Answer the pending choice with a scalar index. -1 == decline/skip, which is
   what Rust's `select_indices(&[])` means for a skippable prompt
   (helpers/choices.rs `resume_with_choice(..., None, ...)`). */
static void answer(TestGame *tg, int idx)
{
    if (rb_has_pending_choice(&tg->state)) rb_resume_with_choice(&tg->state, idx);
}

static void answer_skip(TestGame *tg) { answer(tg, -1); }
static void answer_first(TestGame *tg) { answer(tg, 0); }

/* Rust helpers/choices.rs:269 drain_auto_ability_choices — only SelectAutoAbility
   prompts are answered, everything else breaks the loop. */
static void drain_auto(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 200) {
        if (strcmp(test_pending_choice_type(tg), "SelectAutoAbility") != 0) break;
        rb_resume_with_choice(&tg->state, -1);
    }
}

/* Rust baton_swap_auto_helpers.rs:33 baton_touch_over — required 1-card
   SelectCards pick index 0, everything optional is declined. */
static void drain_baton(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 200) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int required_discard = c && c->kind == RB_CHOICE_SELECT_CARD &&
                               c->count == 1 && !c->allow_skip;
        answer(tg, required_discard ? 0 : -1);
    }
}

/* Rust baton_swap_auto_helpers.rs:14 resolve_auto_choices_accepting_optionals. */
static void drain_accept_optionals(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 200) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        if (!c) break;
        if (c->kind == RB_CHOICE_SELECT_AUTO_ABILITY) { answer(tg, -1); continue; }
        if (c->kind == RB_CHOICE_SELECT_CARD) {
            if (c->count > 0 && c->count < 10) answer(tg, 0);
            else answer(tg, 0);
            continue;
        }
        if (c->kind == RB_CHOICE_SELECT_TARGET && c->target[0] &&
            strcmp(c->target, "conditional_optional") == 0) { answer(tg, 1); continue; }
        answer(tg, 0);
    }
}

/* Rust `game.state.set_recently_moved_cards(vec![..])`. */
static void set_recently_moved(TestGame *tg, int cid)
{
    tg->state.recently_moved[0] = cid;
    tg->state.n_recently_moved = 1;
}

/* Rust `state.turn_movements.push(MovementEvent{..})` (the C equivalent is the
   batch-movement log that condition.c:resolve_moved_cards_source reads). */
static void push_movement(TestGame *tg, int cid, RbZoneId src, RbZoneId dst)
{
    GameState *g = &tg->state;
    if (g->n_batch_movements >=
        (int)(sizeof(g->batch_movements) / sizeof(g->batch_movements[0]))) return;
    RbBatchMovement *m = &g->batch_movements[g->n_batch_movements++];
    m->moved_card_id = cid;
    m->source_zone = (int)src;
    m->dest_zone = (int)dst;
    m->cause_player_id = 0;
    m->effect_only = 0;
}

/* Rust `TurnEngine::trigger_auto_abilities_for_player(&state, &pid)`. */
static void tas_scan(TestGame *tg, int pl)
{
    rb_queue_trigger_abilities(&tg->state, pl, RB_TSTR_AUTO);
}

/* Rust `state.process_pending_auto_abilities(&pid)`. */
static void tas_process(TestGame *tg) { rb_process_pending_auto_abilities(&tg->state); }

static void tas_full(TestGame *tg, int pl) { tas_scan(tg, pl); tas_process(tg); }

/* Rust baton_swap_auto_helpers.rs:48 replace_member_by_baton_touch. */
static void replace_by_baton(TestGame *tg, int replaced, int arriver, int area)
{
    test_give_energy(tg, 30);
    test_add_to_stage(tg, area, replaced);
    test_add_to_hand(tg, arriver);
    test_play_to_stage(tg, arriver, area);
}

static void add_filler_deck(TestGame *tg, int n)
{
    int filler = test_id(tg, FILLER);
    for (int i = 0; i < n; i++) test_add_to_deck_pl(tg, 0, filler);
}

static void stage_leave_to_waitroom(TestGame *tg, int area, int cid)
{
    tg->state.p[0].stage[area] = RB_EMPTY_SLOT;
    test_add_to_discard(tg, cid);
    set_recently_moved(tg, cid);
}

/* ===================================================================== */
/* A. jidou/leaves_stage/baton_displaced_self_placed_under_arriver_test.rs */
/*    PL!SP-bp7-001-R 澁谷かのん — 「バトンタッチしていた場合、このカードを   */
/*    そのバトンタッチで登場したメンバーの下に置く」                       */
/* ===================================================================== */

static void test_baton_displaced_self_places_under_arriver(void)
{
    TestGame game;
    test_game_new(&game);

    int kanon = test_id(&game, KANON);
    int arriver = test_id(&game, SUMMER_ARR);
    CHECK(kanon >= 0 && arriver >= 0, "bp7-001 fixtures resolve");

    test_add_to_stage(&game, 1, kanon);
    add_filler_deck(&game, 10);
    test_give_energy(&game, 25);

    test_add_to_hand(&game, arriver);
    test_play_to_stage(&game, arriver, 1);
    drain_baton(&game);

    CHECK_EQ(game.state.p[0].stage[1], arriver, "arriver occupies center");
    CHECK(under_has(&game, 0, 1, kanon),
          "ab#1 should place 澁谷かのん under the arriving member");
    CHECK(!waitroom_has(&game, kanon),
          "baton-touched 澁谷かのん must be under the arriver, not in the waitroom");
}

static void test_baton_displaced_self_no_baton_touch_stays_in_waitroom(void)
{
    TestGame game;
    test_game_new(&game);

    int kanon = test_id(&game, KANON);
    test_add_to_stage(&game, 1, kanon);
    add_filler_deck(&game, 10);
    test_give_energy(&game, 25);

    /* Non-baton-touch removal: stage slot cleared and the card pushed to the
       waitroom by "an effect", then a TAS scan. */
    stage_leave_to_waitroom(&game, 1, kanon);
    tas_full(&game, 0);
    drain_auto(&game);

    CHECK(waitroom_has(&game, kanon),
          "non-baton-touch removal must leave 澁谷かのん in the waitroom");
    CHECK_EQ(game.state.p[0].under_cards[1].n, 0,
             "no member under center — ab#1 must not fire without baton touch");
}

static void test_baton_displaced_self_as_arriver_not_displaced(void)
{
    TestGame game;
    test_game_new(&game);

    int kanon = test_id(&game, KANON);
    int displaced = test_id(&game, FILLER);

    test_add_to_stage(&game, 1, displaced);
    add_filler_deck(&game, 10);
    test_give_energy(&game, 25);

    test_add_to_hand(&game, kanon);
    test_play_to_stage(&game, kanon, 1);
    drain_baton(&game);

    CHECK_EQ(game.state.p[0].stage[1], kanon,
             "澁谷かのん should be on center after baton-touching in");
    CHECK(!under_has(&game, 0, 1, kanon),
          "the arriver 澁谷かのん must not be placed under herself");
}

static void test_baton_displaced_self_placed_under_specific_arriver_slot(void)
{
    TestGame game;
    test_game_new(&game);

    int kanon = test_id(&game, KANON);
    int other = test_id(&game, SUMMER_ARR);
    int arriver = test_new_id(&game, SUMMER_ARR);

    game.state.p[0].stage[0] = other;
    game.state.p[0].stage[1] = kanon;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    add_filler_deck(&game, 10);
    test_give_energy(&game, 25);

    test_add_to_hand(&game, arriver);
    test_play_to_stage(&game, arriver, 1);
    drain_baton(&game);

    CHECK(under_has(&game, 0, 1, kanon),
          "kanon should be under the center arriver");
    CHECK(!under_has(&game, 0, 2, kanon),
          "kanon must NOT go under the right-slot member");
}

/* ===================================================================== */
/* B. jidou/leaves_stage/baton_displaced_under_arriver_grants_host_blade  */
/*    PL!SP-bp7-001-R ab#0 — the host gains a blade when a member sits under */
/*    it.                                                                   */
/* ===================================================================== */

static void test_baton_displaced_self_placed_under_liella_arriver_grants_host_blade(void)
{
    TestGame game;
    test_game_new(&game);

    int kanon = test_id(&game, KANON);
    int arriver = test_id(&game, SUMMER_ARR);

    test_add_to_stage(&game, 1, kanon);
    add_filler_deck(&game, 10);
    test_give_energy(&game, 25);

    test_add_to_hand(&game, arriver);
    test_play_to_stage(&game, arriver, 1);
    drain_baton(&game);

    CHECK_EQ(game.state.p[0].stage[1], arriver, "arriver should occupy center after baton-touch");
    CHECK(under_has(&game, 0, 1, kanon), "ab#1 should place 澁谷かのん under the arriving member");
    CHECK(!waitroom_has(&game, kanon), "澁谷かのん must be under the arriver, not in the waitroom");
    CHECK_EQ(test_get_blade_modifier(&game, arriver), 1,
             "host Liella! member with 澁谷かのん underneath → 1 blade");
}

/* ===================================================================== */
/* C. jidou/leaves_stage/                                                 */
/*    self_stage_to_waitroom_recovery_and_group_baton_replacement_test.rs */
/* ===================================================================== */

static void test_self_stage_to_waitroom_recovers_group_live(void)
{
    TestGame game;
    test_game_new(&game);

    int kasumi = test_id(&game, KASUMI);
    int niji = test_id(&game, NIJI_LIVE);
    int arriver = test_id(&game, ARRIVER_GEN);

    CHECK(kasumi >= 0 && niji >= 0 && arriver >= 0, "kasumi recovery fixtures resolve");
    test_add_to_discard(&game, niji);

    replace_by_baton(&game, kasumi, arriver, 1);
    drain_accept_optionals(&game);

    CHECK(waitroom_has(&game, kasumi), "かすみ should be in the waitroom after baton touch");
    CHECK(hand_has(&game, niji),
          "かすみ ab#0 should add a 虹ヶ咲 live card from the discard to hand");
}

static void test_other_member_stage_to_waitroom_does_not_trigger_self_live_recovery(void)
{
    TestGame game;
    test_game_new(&game);

    int kasumi = test_id(&game, KASUMI);
    int other = test_id(&game, OTHER_MEMBER);
    int niji = test_id(&game, NIJI_LIVE);
    int arriver = test_id(&game, ARRIVER_GEN);

    test_add_to_discard(&game, niji);
    test_add_to_stage(&game, 0, kasumi);

    replace_by_baton(&game, other, arriver, 1);
    drain_accept_optionals(&game);

    CHECK_EQ(game.state.p[0].stage[0], kasumi, "かすみ should still be on stage");
    CHECK(!hand_has(&game, niji),
          "かすみ ab#0 must NOT fire when a DIFFERENT member moves");
}

static void test_self_stage_to_waitroom_draw_two_discard_one(void)
{
    TestGame game;
    test_game_new(&game);

    int toko = test_id(&game, TOKO);
    int arriver = test_id(&game, ARRIVER_GEN);

    CHECK(toko >= 0 && arriver >= 0, "toko fixtures resolve");
    add_filler_deck(&game, 20);

    replace_by_baton(&game, toko, arriver, 1);
    drain_accept_optionals(&game);

    CHECK_EQ(game.state.p[0].hand.n, 1,
             "藤島慈: draw 2, discard 1 → net hand +1 (from 0 after playing arriver)");
}

static void test_self_stage_to_waitroom_draws_then_discards_exact_two(void)
{
    TestGame game;
    test_game_new(&game);

    int rurino = test_id(&game, RURINO);
    int arriver = test_id(&game, ARRIVER_GEN);
    int draw_first = test_id(&game, NIJI_LIVE);
    int draw_second = test_id(&game, LIELIA_LIVE);
    int remainder = test_id(&game, OTHER_MEMBER);

    CHECK(rurino >= 0 && draw_first >= 0 && draw_second >= 0 && remainder >= 0,
          "rurino fixtures resolve");

    game.state.p[0].deck.n = 0;
    test_add_to_deck_pl(&game, 0, draw_first);
    test_add_to_deck_pl(&game, 0, draw_second);
    test_add_to_deck_pl(&game, 0, remainder);
    game.state.p[0].hand.n = 0;
    game.state.p[0].discard.n = 0;

    replace_by_baton(&game, rurino, arriver, 1);
    drain_accept_optionals(&game);

    CHECK_EQ(game.state.p[0].stage[1], arriver,
             "the arriving member should remain on the baton-touched area");
    CHECK_EQ(game.state.p[0].deck.n, 1,
             "Rurino must draw exactly the two top cards and leave the rest");
    CHECK(deck_has(&game, remainder), "the untouched card stays in the deck");
    CHECK_EQ(game.state.p[0].hand.n, 0, "Rurino must discard the two newly drawn cards");
    CHECK_EQ(game.state.p[0].discard.n, 3,
             "only Rurino and its two discarded cards should be in the waitroom");
    CHECK_EQ(game.state.p[0].discard.cards[0], rurino,
             "the baton-touched member should be in the waitroom");
    CHECK(waitroom_has(&game, draw_first), "first drawn card is discarded");
    CHECK(waitroom_has(&game, draw_second), "second drawn card is discarded");
}

static void test_self_stage_to_waitroom_look_five_adds_live_to_hand(void)
{
    TestGame game;
    test_game_new(&game);

    int yuiguri = test_id(&game, YUIGURI);
    int arriver = test_id(&game, ARRIVER_GEN);
    int niji = test_id(&game, NIJI_LIVE);

    CHECK(yuiguri >= 0, "yuiguri fixture resolves");
    game.state.p[0].deck.n = 0;
    test_add_to_deck_pl(&game, 0, niji);
    add_filler_deck(&game, 10);

    replace_by_baton(&game, yuiguri, arriver, 1);
    drain_accept_optionals(&game);

    CHECK(hand_has(&game, niji),
          "夕霧綴理 ab#0 should reveal a live card from the looked-at 5 into hand");
}

static void test_self_stage_to_waitroom_optional_discard_grants_heart05_and_blade(void)
{
    TestGame game;
    test_game_new(&game);

    int sayaka = test_id(&game, SAYAKA);
    int arriver = test_id(&game, ARRIVER_GEN);
    int target = test_id(&game, OTHER_MEMBER);
    int filler = test_id(&game, FILLER);

    CHECK(sayaka >= 0 && target >= 0, "sayaka fixtures resolve");
    test_add_to_hand(&game, filler);
    test_add_to_stage(&game, 0, target);

    replace_by_baton(&game, sayaka, arriver, 1);
    drain_accept_optionals(&game);

    CHECK_EQ(test_get_heart_modifier(&game, target, RB_HEART_ORANGE), 1,
             "村野さやか ab#0 grants exactly one heart05 to one stage member");
    CHECK_EQ(test_get_blade_modifier(&game, target), 1,
             "村野さやか ab#0 grants exactly one blade to one stage member");
    CHECK(stage_has(&game, 0, target), "the buffed member is still the one on stage");
}

static void test_group_baton_replacement_places_energy_under_arriving_member(void)
{
    TestGame game;
    test_game_new(&game);

    int setsuna = test_id(&game, SETSUNA);
    int arriver = test_id(&game, NIJI_MEMBER);
    int energy = test_id(&game, ENERGY);

    CHECK(setsuna >= 0 && arriver >= 0 && energy >= 0, "setsuna fixtures resolve");
    test_add_to_energy_deck(&game, 0, energy);

    replace_by_baton(&game, setsuna, arriver, 1);
    drain_accept_optionals(&game);

    CHECK(waitroom_has(&game, setsuna), "せつ菜 should be in the waitroom after baton touch");
    CHECK_EQ(game.state.p[0].under_cards[1].n, 1,
             "せつ菜 ab#0 should place 1 energy under the arriving 虹ヶ咲 member");
    CHECK_EQ(game.state.p[0].under_cards[1].cards[0], energy,
             "the card placed under the arriving member is the energy card");
}

/* ===================================================================== */
/* D. jidou/leaves_stage/self_wait_cost_then_choose_mirakura_member_      */
/*    gains_blade_test.rs — PL!HS-cl1-003-CL 自動/起動: self wait cost, then */
/*    「みらくらぱーく！」のメンバー1人にブレードを付与.                     */
/* ===================================================================== */

static void test_hs_cl1_already_wait_no_blade(void)
{
    TestGame game;
    test_game_new(&game);

    int card = test_id(&game, CL_RURINO);
    CHECK(card >= 0, "PL!HS-cl1-003-CL resolves");
    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = card;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    rb_mods_set_orientation(&game.state.mods, card, "wait");

    test_activate_ability(&game, card);
    drain_baton(&game);

    /* Rust asserts res.is_err() (Q137/Q56 — an already-waited member's mandatory
       「wait this member」 cost is unpayable). rb_activate_card returns "an
       activation ability was found", not "the cost was payable", so the error
       channel is not observable in C. The observable consequence — no blade —
       is asserted below. */
    EXPECTED_GAP(test_get_blade_modifier(&game, card) == 0,
                 "already wait should not grant blade (cost already satisfied, no effect)");
}

static void test_hs_cl1_turn_limit_blocks_second(void)
{
    TestGame game;
    test_game_new(&game);

    int card = test_id(&game, CL_RURINO);
    int other = test_id(&game, FILLER);
    game.state.p[0].stage[0] = card;
    game.state.p[0].stage[1] = other;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    add_filler_deck(&game, 10);
    test_give_energy(&game, 20);

    test_activate_ability(&game, card);
    drain_baton(&game);

    int turn = game.state.turn;
    CHECK(rb_ability_uses_used(&game.state, card, 0) >= 1,
          "the first activation must record its ターン1回 use");
    int blade_after_first = test_get_blade_modifier(&game, card);
    const char *wait_after_first = rb_mods_get_orientation(&game.state.mods, card);
    char wait_snapshot[16];
    snprintf(wait_snapshot, sizeof wait_snapshot, "%s", wait_after_first ? wait_after_first : "");

    test_activate_ability(&game, card);
    drain_baton(&game);

    const char *wait_now = rb_mods_get_orientation(&game.state.mods, card);
    EXPECTED_GAP((wait_now ? wait_now : "") == wait_snapshot ||
                 !strcmp(wait_snapshot, wait_now ? wait_now : ""),
                 "a ターン1回 refusal must not wait the member a second time");
    EXPECTED_GAP(test_get_blade_modifier(&game, card) == blade_after_first,
                 "a ターン1回 refusal must not grant a second blade");
    CHECK(!pending(&game), "a ターン1回 refusal must not open a prompt");
    (void)turn;
}

/* Mirror Rust's `blade_recipient(pick)` runner: returns the stage area that
   ended up holding the blade, or -1 when nobody got one. */
static int blade_recipient(TestGame *tg, int pick, int *out_asked)
{
    int card = test_id(tg, CL_RURINO);
    int mira1 = test_id(tg, MIRA_PRINT1);
    int mira2 = test_id(tg, MIRA_PRINT2);
    int outsider = test_id(tg, FILLER);
    (void)outsider;

    /* Premise: the filter is 『みらくらぱーく！』, so every staged member is in
       that group — otherwise "the chosen area gained the blade" proves nothing. */
    CHECK(rb_card_matches_group_str(card, "みらくらぱーく！"),
          "precondition: the activating card is in みらくらぱーく！");
    CHECK(rb_card_matches_group_str(mira1, "みらくらぱーく！"),
          "precondition: the centre member is in みらくらぱーく！");
    CHECK(rb_card_matches_group_str(mira2, "みらくらぱーく！"),
          "precondition: the right member is in みらくらぱーく！");
    CHECK(!strcmp(test_card_name(card), test_card_name(mira1)),
          "the three are three printings of one character");

    tg->state.p[0].stage[0] = card;
    tg->state.p[0].stage[1] = mira1;
    tg->state.p[0].stage[2] = mira2;
    add_filler_deck(tg, 10);
    test_give_energy(tg, 20);
    drain_auto(tg);

    test_activate_ability(tg, card);
    drain_auto(tg);

    if (!pending(tg)) { *out_asked = 0; return -1; }
    *out_asked = 1;
    CHECK(strcmp(pending_kind(tg), "SelectCard") == 0,
          "three みらくらぱーく！ members on stage: the effect must ask which one");
    {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        CHECK_EQ(c ? c->count : -1, 1, "「メンバー1人」 is a single card");
        CHECK(c && strstr(c->zone, "stage") != NULL,
              "the candidates are the members on the stage");
    }
    answer(tg, pick);
    drain_baton(tg);

    for (int area = 0; area < RB_STAGE_SIZE; area++) {
        int id = tg->state.p[0].stage[area];
        int expected = (area == pick) ? 1 : 0;
        CHECK_EQ(test_get_blade_modifier(tg, id), expected,
                 area == pick ? "the member in the CHOSEN area gains the blade, exactly one"
                              : "「1人」 means the other members get nothing");
    }
    for (int area = 0; area < RB_STAGE_SIZE; area++)
        if (test_get_blade_modifier(tg, tg->state.p[0].stage[area]) > 0) return area;
    return -1;
}

static void test_hs_cl1_choice_among_multiple_mirakura(void)
{
    TestGame g1, g2;
    int asked1 = 0, asked2 = 0;

    test_game_new(&g1);
    int center_area = blade_recipient(&g1, 1, &asked1);

    test_game_new(&g2);
    int right_area = blade_recipient(&g2, 2, &asked2);

    CHECK(asked1 && asked2, "both runs produced the 「みらくらぱーく！」 prompt");
    CHECK_EQ(center_area, 1, "picking area 1 grants the blade to area 1");
    CHECK_EQ(right_area, 2, "picking area 2 grants the blade to area 2");
    CHECK(center_area != right_area,
          "the two runs must land on different areas, or the second run is not testing a choice");
}

/* ===================================================================== */
/* E. jidou/leaves_stage/baton_touch_nijigasaki_partner_cost_gate_        */
/*    energizes_and_draws_test.rs — PL!N-bp5-005-R+ 自動 gated on the      */
/*    baton-touched partner's cost and blade heart.                        */
/* ===================================================================== */

static void niji_partner_scenario(TestGame *tg, int partner, int *out_hand,
                                  int *out_energy)
{
    int niji = test_id(tg, NIJI_AUTO);
    int energy = test_new_id(tg, ENERGY);
    int filler = test_new_id(tg, FILLER);

    /* energy cards go in INACTIVE (Rust pushes straight into energy_zone.cards). */
    for (int i = 0; i < 5; i++) {
        RbPlayer *P = &tg->state.p[0];
        if (P->energy.n < RB_MAX_ZONE) P->energy.cards[P->energy.n++] = energy;
    }
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(tg, 0, filler);
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(tg, 1, filler);

    *out_hand = tg->state.p[0].hand.n;
    *out_energy = tg->state.p[0].energy_active;

    test_add_to_discard(tg, niji);
    set_recently_moved(tg, niji);
    tg->state.baton_touch_count_p1 = 1;
    tg->state.baton_touch_replaced_member_id = niji;
    tg->state.baton_touch_arriving_card_id = partner;
    tg->state.p[0].stage[0] = partner;

    tas_scan(tg, 0);
    tas_process(tg);
    drain_auto(tg);

    *out_hand = tg->state.p[0].hand.n;
    *out_energy = tg->state.p[0].energy_active;
}

static void test_niji_partner_cost10_noblade_energizes_2(void)
{
    TestGame game;
    test_game_new(&game);

    int partner = test_id(&game, "PL!N-bp3-009-P");
    int hand, energy;
    CHECK(partner >= 0, "PL!N-bp3-009-P resolves");
    niji_partner_scenario(&game, partner, &hand, &energy);

    EXPECTED_GAP(energy >= 2, "Partner cost=10 no blade → should activate 2 energy");
}

static void test_niji_partner_cost15_noblade_energizes_2_and_draws_1(void)
{
    TestGame game;
    test_game_new(&game);

    int partner = test_id(&game, "PL!N-pb1-009-P+");
    int hand, energy;
    CHECK(partner >= 0, "PL!N-pb1-009-P+ resolves");
    niji_partner_scenario(&game, partner, &hand, &energy);

    EXPECTED_GAP(energy >= 2, "Partner cost=15 no blade → should activate 2 energy");
    EXPECTED_GAP(hand >= 1, "Partner cost=15 no blade → should draw 1 card");
}

static void test_niji_partner_with_blade_heart_nothing_fires(void)
{
    TestGame game;
    test_game_new(&game);

    int partner = test_id(&game, "PL!N-bp1-003-P");
    int hand, energy;
    CHECK(partner >= 0, "PL!N-bp1-003-P resolves");
    niji_partner_scenario(&game, partner, &hand, &energy);

    CHECK_EQ(energy, 0, "Partner with blade heart → should NOT activate energy");
    CHECK_EQ(hand, 0, "Partner with blade heart → should NOT draw");
}

static void test_niji_partner_low_cost_nothing_fires(void)
{
    TestGame game;
    test_game_new(&game);

    int partner = test_id(&game, "PL!N-bp4-001-P");
    int hand, energy;
    CHECK(partner >= 0, "PL!N-bp4-001-P resolves");
    niji_partner_scenario(&game, partner, &hand, &energy);

    CHECK_EQ(energy, 0, "Partner cost=2 → should NOT activate energy");
    CHECK_EQ(hand, 0, "Partner cost=2 → should NOT draw");
}

static void test_niji_partner_not_nijigasaki_nothing_fires(void)
{
    TestGame game;
    test_game_new(&game);

    int partner = test_id(&game, FILLER);
    int hand, energy;
    niji_partner_scenario(&game, partner, &hand, &energy);

    CHECK_EQ(energy, 0, "Non-Nijigasaki partner → should NOT activate energy");
    CHECK_EQ(hand, 0, "Non-Nijigasaki partner → should NOT draw");
}

/* ===================================================================== */
/* F. jidou/leaves_stage/optional_any_player_position_change_q238_test.rs */
/*    PL!HS-bp5-003-AR 大沢瑠璃乃 「メンバー1人をポジションチェンジさせても */
/*    よい」 — Q238: the target may belong to either player.                 */
/* ===================================================================== */

static void trigger_stage_to_waitroom_auto(TestGame *tg, int rino)
{
    set_recently_moved(tg, rino);
    tas_full(tg, 0);
}

static void test_leaves_stage_repositions_opponent_member_q238(void)
{
    TestGame game;
    test_game_new(&game);

    int rino = test_id(&game, OSAWA_RINO);
    int opp = test_new_id(&game, FILLER);

    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = rino;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    game.state.p[1].stage[0] = RB_EMPTY_SLOT;
    game.state.p[1].stage[1] = opp;
    game.state.p[1].stage[2] = RB_EMPTY_SLOT;

    stage_leave_to_waitroom(&game, 1, rino);
    trigger_stage_to_waitroom_auto(&game, rino);

    CHECK(pending(&game), "Rino's auto should fire on stage→discard");
    /* Rust then enumerates generate_possible_actions filtered to
       ActionType::ChoicePosition and asserts it has exactly 1 entry. The C
       `rb_generate_action_candidates` only emits pass / play_member_to_stage,
       so the candidate list is not observable. */
    EXPECTED_GAP(!pending(&game) == 0,
                 "the optional position_change prompt is offered (candidate list unobservable in C)");
}

static void test_leaves_stage_position_change_without_other_members_skips_q238(void)
{
    TestGame game;
    test_game_new(&game);

    int rino = test_id(&game, OSAWA_RINO);
    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = rino;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    game.state.p[1].stage[0] = RB_EMPTY_SLOT;
    game.state.p[1].stage[1] = RB_EMPTY_SLOT;
    game.state.p[1].stage[2] = RB_EMPTY_SLOT;

    stage_leave_to_waitroom(&game, 1, rino);
    trigger_stage_to_waitroom_auto(&game, rino);

    CHECK(!pending(&game), "No valid targets → auto ability should skip");
}

static void test_leaves_stage_position_change_offers_both_players_members_q238(void)
{
    TestGame game;
    test_game_new(&game);

    int rino = test_id(&game, OSAWA_RINO);
    int own_left = test_new_id(&game, FILLER);
    int opp_center = test_new_id(&game, FILLER);
    int opp_right = test_new_id(&game, FILLER);

    game.state.p[0].stage[0] = own_left;
    game.state.p[0].stage[1] = rino;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    game.state.p[1].stage[0] = RB_EMPTY_SLOT;
    game.state.p[1].stage[1] = opp_center;
    game.state.p[1].stage[2] = opp_right;

    stage_leave_to_waitroom(&game, 1, rino);
    trigger_stage_to_waitroom_auto(&game, rino);

    CHECK(pending(&game), "Rino's auto should fire and offer a position change");
    /* Rust asserts count_position_actions() == 3 (self:left, opponent:center,
       opponent:right) — not observable in C (see above). */
    drain_auto(&game);
}

static void test_leaves_stage_declined_position_change_preserves_opponent_position(void)
{
    TestGame game;
    test_game_new(&game);

    int rino = test_id(&game, OSAWA_RINO);
    int opp = test_new_id(&game, FILLER);

    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = rino;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    game.state.p[1].stage[0] = RB_EMPTY_SLOT;
    game.state.p[1].stage[1] = opp;
    game.state.p[1].stage[2] = RB_EMPTY_SLOT;

    stage_leave_to_waitroom(&game, 1, rino);
    trigger_stage_to_waitroom_auto(&game, rino);

    CHECK(pending(&game), "Rino's auto should fire");
    answer_skip(&game);
    drain_auto(&game);

    CHECK(!pending(&game), "No more choices after skip");
    CHECK_EQ(game.state.p[1].stage[1], opp, "Opponent member unchanged after skip");
}

/* ===================================================================== */
/* G. jidou/leaves_stage/stage_to_discard_watcher_requires_actual_stage_  */
/*    leaving_move_test.rs                                                */
/* ===================================================================== */

static void test_stage_to_discard_ability_triggers_only_on_baton_touch(void)
{
    TestGame game;
    test_game_new(&game);

    int honoka = test_id(&game, HONOKA);
    int filler = test_id(&game, FILLER);
    int arriver = test_id(&game, "PL!S-bp5-012-N");

    CHECK(honoka >= 0 && filler >= 0 && arriver >= 0, "honoka fixtures resolve");
    test_give_energy(&game, 30);
    game.state.p[0].deck.n = 0;
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(&game, 0, filler);

    test_add_to_hand(&game, honoka);
    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = RB_EMPTY_SLOT;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_play_to_stage(&game, honoka, 0);
    drain_baton(&game);
    CHECK(!pending(&game), "Ability should NOT trigger on debut (card is on stage)");

    test_add_to_hand(&game, filler);
    test_play_to_stage(&game, filler, 1);
    drain_baton(&game);
    CHECK(!pending(&game), "Ability should NOT trigger when another card debuts");

    /* Rust clears `deployed_this_turn`; the C mirror is stage_arrived[]. */
    for (int i = 0; i < RB_STAGE_SIZE; i++) game.state.stage_arrived[0][i] = 0;

    test_add_to_hand(&game, arriver);
    test_play_to_stage(&game, arriver, 0);
    drain_baton(&game);
    CHECK(!pending(&game),
          "Auto ability should NOT present a choice when no wait members exist");
    CHECK(waitroom_has(&game, honoka), "Honoka should be in waitroom after baton touch");
}

static void test_kohaku_stage_to_discard_triggers_ability(void)
{
    TestGame game;
    test_game_new(&game);

    int kohaku = test_id(&game, KOHAKU);
    int filler = test_id(&game, FILLER);

    CHECK(kohaku >= 0, "PL!HS-bp2-012-N resolves");
    game.state.p[0].deck.n = 0;
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(&game, 0, filler);

    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = kohaku;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    stage_leave_to_waitroom(&game, 1, kohaku);

    tas_full(&game, 0);
    CHECK(pending(&game), "Kohaku ab#0 should fire on stage→discard");
    answer_skip(&game);
    drain_auto(&game);
    CHECK(!pending(&game), "No more choices after skipping");
}

static void test_kohaku_baton_touch_triggers_ability(void)
{
    TestGame game;
    test_game_new(&game);

    int kohaku = test_id(&game, KOHAKU);
    int arriver = test_id(&game, ARRIVER_GEN);
    int filler = test_id(&game, FILLER);

    game.state.p[0].deck.n = 0;
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(&game, 0, filler);
    test_give_energy(&game, 20);

    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = kohaku;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(&game, arriver);
    test_add_to_hand(&game, filler);

    test_play_to_stage(&game, arriver, 1);

    CHECK(pending(&game), "Kohaku ab#0 should trigger on baton touch stage→discard");
    answer_skip(&game);
    drain_baton(&game);
    CHECK(waitroom_has(&game, kohaku), "Kohaku should be in waitroom after baton touch");
}

static void test_kohaku_baton_touch_selects_member_from_deck(void)
{
    TestGame game;
    test_game_new(&game);

    int kohaku = test_id(&game, KOHAKU);
    int arriver = test_id(&game, ARRIVER_GEN);
    int member_target = test_id(&game, OTHER_MEMBER);
    int filler = test_id(&game, FILLER);

    game.state.p[0].deck.n = 0;
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(&game, 0, filler);
    game.state.p[0].deck.cards[0] = member_target;
    test_give_energy(&game, 20);

    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = kohaku;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(&game, arriver);
    test_add_to_hand(&game, filler);

    test_play_to_stage(&game, arriver, 1);

    CHECK(pending(&game), "Kohaku ab#0 should trigger on baton touch");
    answer_first(&game);
    drain_baton(&game);
    CHECK(hand_has(&game, member_target),
          "Member card should be in hand after baton touch trigger");
}

/* ===================================================================== */
/* H. jidou/leaves_stage/self_discard_optional_hand_discard_recovers_live */
/*    _and_member_test.rs — PL!HS-bp6-017-N 日野下花帆                     */
/* ===================================================================== */

static void test_hanabiko_stage_to_discard_triggers_ability(void)
{
    TestGame game;
    test_game_new(&game);

    int hanabiko = test_id(&game, HANABIKO);
    int live = test_id(&game, HANABI_LIVE);
    int member = test_id(&game, HANABI_MEMB);
    int filler = test_id(&game, FILLER);

    CHECK(hanabiko >= 0 && live >= 0 && member >= 0, "hanabiko fixtures resolve");
    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = hanabiko;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_add_to_discard(&game, live);
    test_add_to_discard(&game, member);
    test_add_to_hand(&game, filler);
    test_add_to_hand(&game, filler);
    add_filler_deck(&game, 10);
    add_filler_deck(&game, 0);

    stage_leave_to_waitroom(&game, 1, hanabiko);
    tas_full(&game, 0);

    CHECK(pending(&game), "hanabiko ab#0 should fire on stage→discard");
    answer_first(&game);
    CHECK(pending(&game), "should prompt to select live card from discard");
    answer_first(&game);
    CHECK(pending(&game), "should prompt to select member card from discard");
    answer_first(&game);
    drain_baton(&game);

    CHECK(hand_has(&game, live), "live card should be retrieved to hand");
    CHECK(hand_has(&game, member), "member card should be retrieved to hand");
}

static void test_hanabiko_static_stage_no_trigger(void)
{
    TestGame game;
    test_game_new(&game);

    int hanabiko = test_id(&game, HANABIKO);
    int filler = test_id(&game, FILLER);

    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = hanabiko;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(&game, filler);
    add_filler_deck(&game, 10);

    /* A DIFFERENT card moves, not hanabiko. */
    set_recently_moved(&game, filler);
    test_add_to_discard(&game, filler);
    tas_full(&game, 0);
    drain_baton(&game);

    CHECK(stage_has(&game, 0, hanabiko), "hanabiko stays on stage");
}

static void test_hanabiko_static_discard_no_trigger(void)
{
    TestGame game;
    test_game_new(&game);

    int hanabiko = test_id(&game, HANABIKO);
    int filler = test_id(&game, FILLER);

    test_add_to_discard(&game, hanabiko);
    test_add_to_hand(&game, filler);
    add_filler_deck(&game, 10);

    set_recently_moved(&game, filler);
    test_add_to_discard(&game, filler);
    tas_full(&game, 0);
    drain_baton(&game);

    CHECK(waitroom_has(&game, hanabiko), "hanabiko stays in waitroom");
}

static void test_hanabiko_hand_to_discard_no_trigger(void)
{
    TestGame game;
    test_game_new(&game);

    int hanabiko = test_id(&game, HANABIKO);
    int filler = test_id(&game, FILLER);

    test_add_to_hand(&game, hanabiko);
    test_add_to_hand(&game, filler);
    add_filler_deck(&game, 10);

    /* hand -> waitroom: the wrong SOURCE zone for a stage→waitroom watcher. */
    set_recently_moved(&game, hanabiko);
    test_add_to_discard(&game, hanabiko);
    push_movement(&game, hanabiko, RB_ZONEID_HAND, RB_ZONEID_WAITROOM);
    tas_full(&game, 0);

    EXPECTED_GAP(!pending(&game),
                 "hand-to-discard move must not trigger the stage→discard ability");
}

static void test_hanabiko_stage_to_deck_no_trigger(void)
{
    TestGame game;
    test_game_new(&game);

    int hanabiko = test_id(&game, HANABIKO);
    int filler = test_id(&game, FILLER);

    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = hanabiko;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(&game, filler);
    add_filler_deck(&game, 10);

    /* stage -> deck top: the wrong DESTINATION zone. */
    set_recently_moved(&game, hanabiko);
    test_add_to_deck_pl(&game, 0, hanabiko);
    push_movement(&game, hanabiko, RB_ZONEID_STAGE, RB_ZONEID_DECK);
    tas_full(&game, 0);

    EXPECTED_GAP(!pending(&game),
                 "stage-to-deck move must not trigger the stage→discard ability");
    EXPECTED_GAP(!hand_has(&game, hanabiko), "no retrieval may happen without the trigger");
}

static void test_hanabiko_decline_discard_no_retrieval(void)
{
    TestGame game;
    test_game_new(&game);

    int hanabiko = test_id(&game, HANABIKO);
    int filler = test_id(&game, FILLER);

    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = hanabiko;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(&game, filler);
    add_filler_deck(&game, 10);

    stage_leave_to_waitroom(&game, 1, hanabiko);
    tas_full(&game, 0);

    CHECK(pending(&game), "optional discard expected");
    answer_skip(&game);
    drain_auto(&game);
    CHECK(!pending(&game), "no retrieval choices after decline");
}

static void test_hanabiko_baton_touch_replaced_triggers(void)
{
    TestGame game;
    test_game_new(&game);

    int hanabiko = test_id(&game, HANABIKO);
    int baton_arriver = test_id(&game, "PL!N-sd1-010-SD");
    int filler = test_id(&game, FILLER);

    CHECK(hanabiko >= 0 && baton_arriver >= 0, "hanabiko baton-touch fixtures resolve");
    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = hanabiko;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(&game, baton_arriver);
    test_add_to_hand(&game, filler);
    test_add_to_hand(&game, filler);
    add_filler_deck(&game, 10);
    add_filler_deck(&game, 0);
    test_give_energy(&game, 20);

    test_play_to_stage(&game, baton_arriver, 1);

    /* Rust: the arrivers' debut offers a MANDATORY discard 1 from hand. */
    CHECK(pending(&game), "Shioriko debut must offer mandatory discard 1 from hand");
    answer_first(&game);
    drain_baton(&game);

    CHECK(waitroom_has(&game, hanabiko), "Hanabiko should be in waitroom after baton touch");
}

/* ===================================================================== */

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: load_cards failed\n");
        return 2;
    }

    test_baton_displaced_self_places_under_arriver();
    test_baton_displaced_self_no_baton_touch_stays_in_waitroom();
    test_baton_displaced_self_as_arriver_not_displaced();
    test_baton_displaced_self_placed_under_specific_arriver_slot();
    test_baton_displaced_self_placed_under_liella_arriver_grants_host_blade();

    test_self_stage_to_waitroom_recovers_group_live();
    test_other_member_stage_to_waitroom_does_not_trigger_self_live_recovery();
    test_self_stage_to_waitroom_draw_two_discard_one();
    test_self_stage_to_waitroom_draws_then_discards_exact_two();
    test_self_stage_to_waitroom_look_five_adds_live_to_hand();
    test_self_stage_to_waitroom_optional_discard_grants_heart05_and_blade();
    test_group_baton_replacement_places_energy_under_arriving_member();

    test_hs_cl1_already_wait_no_blade();
    test_hs_cl1_turn_limit_blocks_second();
    test_hs_cl1_choice_among_multiple_mirakura();

    test_niji_partner_cost10_noblade_energizes_2();
    test_niji_partner_cost15_noblade_energizes_2_and_draws_1();
    test_niji_partner_with_blade_heart_nothing_fires();
    test_niji_partner_low_cost_nothing_fires();
    test_niji_partner_not_nijigasaki_nothing_fires();

    test_leaves_stage_repositions_opponent_member_q238();
    test_leaves_stage_position_change_without_other_members_skips_q238();
    test_leaves_stage_position_change_offers_both_players_members_q238();
    test_leaves_stage_declined_position_change_preserves_opponent_position();

    test_stage_to_discard_ability_triggers_only_on_baton_touch();
    test_kohaku_stage_to_discard_triggers_ability();
    test_kohaku_baton_touch_triggers_ability();
    test_kohaku_baton_touch_selects_member_from_deck();

    test_hanabiko_stage_to_discard_triggers_ability();
    test_hanabiko_static_stage_no_trigger();
    test_hanabiko_static_discard_no_trigger();
    test_hanabiko_hand_to_discard_no_trigger();
    test_hanabiko_stage_to_deck_no_trigger();
    test_hanabiko_decline_discard_no_retrieval();
    test_hanabiko_baton_touch_replaced_triggers();

    rb_unload();

    printf("\n==== parity_jidou: %d assertions, %d real failures, %d gaps ====\n",
           assertions, failures, gaps);
    if (failures) {
        fprintf(stderr, "%d runtime failures\n", failures);
        return 1;
    }
    printf("ALL JIDOU PARITY CHECKS PASSED\n");
    return 0;
}
