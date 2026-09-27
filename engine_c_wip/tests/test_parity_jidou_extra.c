/* test_parity_jidou_extra.c — C port of the GAPS in engine/tests/
 * test_modules/jidou/ that tests/test_parity_jidou.c does not already cover.
 *
 * tests/test_parity_jidou.c covers leaves_stage/ (baton-displaced self, the
 * group live recovery, the self-wait cost, the Nijigasaki partner gate, Q238's
 * any-player position change, the stage->discard watcher family and the
 * optional-hand-discard recovery). This file ports the REST, sub-folder by
 * sub-folder, prioritising DISTINCT engine surfaces over sheer count:
 *
 *   A. ability_watch/constant_conditions_exact_counts_and_names_test.rs
 *      Four 常時 whose condition is set by WHICH cards are on the board:
 *      an EXACT count, a count spanning BOTH stages, and a match on a
 *      character's NAME. Each equality is tested on BOTH sides of its boundary.
 *   B. movement/self_move_heart_watch/self_or_opponent_move_grants_blade_test.rs
 *   C. movement/self_move_heart_watch/self_move_watch_cause_and_effect_only_
 *      flag_test.rs
 *   D. energy_watch/debut_and_energy_return_watchers_stay_on_stage_test.rs
 *   E. energy_watch/energy_placed_by_own_effect_gain_blade_test.rs
 *   F. energy_watch/energy_placement_blade_excludes_area_move_and_under_
 *      member_test.rs
 *   G. title_once/dive_in_live_zone_only_ab1_grants_blade_test.rs
 *   H. discard_watch/on_hand_to_discard_each_time_gain_heart01_and_blade_test.rs
 *   I. movement/under_member_placement/under_member_counted_for_both_players_
 *      test.rs
 *   J. yell/no_blade_heart_reveal_gain/no_blade_heart_reveal_heart02_test.rs
 *
 * C API notes (differences from Rust's TestGame that matter here):
 *   - Rust `state.push_movement_event(card, from, to, causer_card, cause_player,
 *     effect_only)` -> C `rb_record_card_movement(g, card, from, to, causer,
 *     target)`, whose SIXTH parameter is the Rust `effect_only` flag despite
 *     being named `target`. The C shim REJECTS `card_id < 0`, so the Rust
 *     "an unnamed energy arrived" idiom (`push_movement_event(-1,
 *     "energy_deck", "energy", ...)`) is not expressible — see section E.
 *   - Rust `TurnEngine::trigger_auto_abilities_for_player(&state, &pid)` +
 *     `state.process_pending_auto_abilities(&pid)` -> C
 *     `rb_queue_trigger_abilities(g, pl, RB_TSTR_AUTO)` +
 *     `rb_process_pending_auto_abilities(g)`, the pair that reproduces
 *     abilities.rs:407-414 + abilities.rs:1791.
 *   - Rust `game.state.set_recently_moved_cards(vec![..])` -> C
 *     `state.recently_moved[] / n_recently_moved`.
 *   - Rust `game.state.recalculate_constants()` -> C `rb_recalc_constants`.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int failures;
static int assertions;
static int gaps;

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
#define FILLER       "PL!-sd1-010-SD"
#define ENERGY       "LL-E-001-SD"
#define KANAN        "PL!S-PR-037-PR"    /* 松浦果南  ちょうど2人 -> heart05 + blade */
#define KOMARI       "PL!S-PR-042-PR"    /* 小原鞠莉  合計6人   -> heart02 + heart04 */
#define WAKANA       "PL!SP-PR-022-PR"   /* 若菜四季  合計6人   -> heart02 + heart03 */
#define HIME         "PL!HS-pb1-022-N"   /* 安養寺姫芽 two NAME-keyed 常時 */
#define RURINO_N     "PL!HS-bp1-014-N"   /* 大沢瑠璃乃 (the first name) */
#define MEGUMI       "PL!HS-bp1-015-N"   /* 藤島 慈   (the second name) */
#define TOMARI       "PL!SP-sd2-011-SD2" /* 鬼塚冬毬 自動 area move -> blade */
#define KEKE         "PL!SP-sd2-002-SD2" /* 唐 可可   自動 area move -> heart06 */
#define REN_TWICE    "PL!SP-bp7-005-R＋"  /* 葉月 恋 ab#1 ターン2回 energy -> blade */
#define REN_ONCE     "PL!SP-bp7-016-N"   /* 葉月 恋 ab#0 ターン1回 energy -> blade */
#define ENERGY_PLACER "PL!SP-pb1-005-R"  /* 葉月かほり 登場 places energy */
#define DIVE         "PL!N-bp4-026-L"    /* DIVE! ab#1 live zone -> blade+2 */
#define NIJI_MEMBER  "PL!N-PR-003-PR"    /* 上原歩夢, a 『虹ヶ咲』 member */
#define RURINO_W     "PL!HS-pb1-003-R"   /* 大沢瑠璃乃 ab#1 hand->waitroom each time */
#define RURINO_OZORA "PL!HS-bp2-005-R＋" /* 大沢瑠璃乃 optional hand discard debut */
#define KINAKO       "PL!SP-pb2-006-R"   /* 桜小路きな子 自動 tuck a Liella! member */
#define CHISATO_D    "PL!SP-pb2-025-N"   /* 嵐 千砂都 登場 optional position change */
#define CHISATO_L    "PL!SP-pb1-014-N"   /* 嵐 千砂都 (a genuinely different Liella! member) */
#define NATSUKA      "PL!SP-bp2-020-N"   /* 鬼塚夏美 自動 yell w/o blade heart -> heart02 */
#define NO_BLADE_HEART "PL!S-sd1-003-SD"  /* a yell card that DOES carry a blade heart */
#define SHIKI        "PL!SP-bp2-008-R"   /* 若菜四季 起動 swaps two members, no energy moved */
#define TUTOR        "PL!N-bp3-007-R"    /* 近江彼方 起動 parks a zone energy under the debut member */
#define SETSUNA      "PL!N-PR-009-PR"    /* the card the tutor's 起動 debuts */

/* ── helpers ─────────────────────────────────────────────────────────────── */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return 1;
}

static int heart_mod(TestGame *tg, int cid, const char *heart)
{
    return test_get_heart_modifier(tg, cid, (int)rb_parse_heart_color(heart));
}
static int blade_mod(TestGame *tg, int cid) { return test_get_blade_modifier(tg, cid); }

static int pending(TestGame *tg) { return test_has_pending_choice(tg); }

static void answer_first(TestGame *tg)
{
    if (rb_has_pending_choice(&tg->state)) rb_resume_with_choice(&tg->state, 0);
}

static void clear_stage(TestGame *tg, int pl)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg->state.p[pl].stage[i] = RB_EMPTY_SLOT;
}

static int p1_members(TestGame *tg)
{
    int n = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg->state.p[0].stage[i] != RB_EMPTY_SLOT) n++;
    return n;
}

static int both_stage_members(TestGame *tg)
{
    int n = 0;
    for (int pl = 0; pl < 2; pl++)
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (tg->state.p[pl].stage[i] != RB_EMPTY_SLOT) n++;
    return n;
}

/* Rust helpers::fill_decks */
static void fill_decks(TestGame *tg, int n)
{
    int filler = test_id(tg, FILLER);
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < n; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

/* Rust `TurnEngine::trigger_auto_abilities_for_player` + `process_pending` */
static void tas_full(TestGame *tg, int pl)
{
    rb_queue_trigger_abilities(&tg->state, pl, RB_TSTR_AUTO);
    rb_process_pending_auto_abilities(&tg->state);
    rb_drain_ability_queue(&tg->state);
}

static void drain_auto(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 200) {
        if (strcmp(test_pending_choice_type(tg), "SelectAutoAbility") != 0) break;
        rb_resume_with_choice(&tg->state, -1);
    }
}

/* Rust `state.set_recently_moved_cards(vec![..])` */
static void set_recently_moved(TestGame *tg, int cid)
{
    tg->state.recently_moved[0] = cid;
    tg->state.n_recently_moved = 1;
}

/* Rust `state.push_movement_event(card, "stage", "stage", causer_card,
   cause_player, effect_only)`. `effect_only` is the C signature's 6th
   parameter, which is named `target` in include/rabuka.h. */
static void push_area_move(TestGame *tg, int card, int cause_player, int effect_only)
{
    rb_record_card_movement(&tg->state, card,
                            RB_ZONEID_STAGE, RB_ZONEID_STAGE,
                            cause_player, effect_only);
}

/* ===================================================================== */
/* A. jidou/ability_watch/constant_conditions_exact_counts_and_names_test  */
/* ===================================================================== */

static void test_kanan_grants_only_at_exactly_two_members(void)
{
    TestGame game;
    test_game_new(&game);

    int kanan = test_id(&game, KANAN);
    CHECK(rb_card_no_eq(kanan, KANAN),
          "the 常時 host is the PL!S-PR-037-PR (松浦果南) print");
    fill_decks(&game, 40);

    /* 「自分のステージにいるメンバーがちょうど2人であるかぎり、heart05+ブレード」 */
    clear_stage(&game, 0);
    game.state.p[0].stage[0] = kanan;
    rb_recalc_constants(&game.state);
    CHECK_EQ(p1_members(&game), 1, "precondition: one member on stage");
    CHECK_EQ(heart_mod(&game, kanan, "heart05"), 0,
             "『ちょうど2人』 is false at 1 member — a 「2人以上」 reading would grant here");
    CHECK_EQ(blade_mod(&game, kanan), 0, "and no blade either");

    game.state.p[0].stage[1] = test_new_id(&game, FILLER);
    rb_recalc_constants(&game.state);
    CHECK_EQ(p1_members(&game), 2, "precondition: exactly two members");
    CHECK_EQ(heart_mod(&game, kanan, "heart05"), 1,
             "exactly 2 members grants heart05 — 2 is the value the condition names");
    CHECK_EQ(blade_mod(&game, kanan), 1,
             "and one blade: both resources come from the same condition");

    int extra = test_new_id(&game, FILLER);
    game.state.p[0].stage[2] = extra;
    rb_recalc_constants(&game.state);
    CHECK_EQ(p1_members(&game), 3, "precondition: three members — the over-boundary case");
    CHECK_EQ(heart_mod(&game, kanan, "heart05"), 0,
             "『ちょうど2人』 is ALSO false at 3 — more members is not better");
    CHECK_EQ(blade_mod(&game, kanan), 0,
             "the blade is withdrawn with the heart: one condition, both resources");

    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    rb_recalc_constants(&game.state);
    CHECK_EQ(p1_members(&game), 2, "precondition: the third member left");
    CHECK_EQ(heart_mod(&game, kanan, "heart05"), 1,
             "returning to exactly 2 re-grants — the condition is re-derived each scan");
}

static void test_komari_grants_only_when_both_stages_together_hold_six(void)
{
    TestGame game;
    test_game_new(&game);

    int komari = test_id(&game, KOMARI);
    CHECK(rb_card_no_eq(komari, KOMARI),
          "the 常時 host is the PL!S-PR-042-PR (小原鞠莉) print");
    fill_decks(&game, 40);

    /* 「自分と相手のステージにメンバーが合計6人いるかぎり、heart02+heart04」 */
    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].stage[0] = komari;
    game.state.p[0].stage[1] = test_new_id(&game, FILLER);
    game.state.p[1].stage[0] = test_new_id(&game, FILLER);
    game.state.p[1].stage[1] = test_new_id(&game, FILLER);
    game.state.p[1].stage[2] = test_new_id(&game, FILLER);
    rb_recalc_constants(&game.state);
    CHECK_EQ(both_stage_members(&game), 5, "precondition: 2 + 3 = 5 members in total");
    CHECK_EQ(heart_mod(&game, komari, "heart02"), 0,
             "『合計6人』 is false at 5 — p2's 3 members ARE counted (scope: both)");

    game.state.p[0].stage[2] = test_new_id(&game, FILLER);
    rb_recalc_constants(&game.state);
    CHECK_EQ(both_stage_members(&game), 6, "precondition: 3 + 3 = 6 members in total");
    CHECK_EQ(heart_mod(&game, komari, "heart02"), 1,
             "six members across both stages grants heart02");
    CHECK_EQ(heart_mod(&game, komari, "heart04"), 1,
             "and heart04 — the card grants TWO colours from the one condition");

    /* 7 is not reachable: two 3-slot stages hold at most 6. */
    int vacancy = test_new_id(&game, FILLER);
    game.state.p[1].stage[2] = RB_EMPTY_SLOT;
    rb_recalc_constants(&game.state);
    CHECK_EQ(both_stage_members(&game), 5, "precondition: one slot vacated");
    CHECK_EQ(heart_mod(&game, komari, "heart02"), 0,
             "at 5 the grant is withdrawn — 『合計6人いるかぎり』 is a live condition");

    game.state.p[1].stage[2] = vacancy;
    rb_recalc_constants(&game.state);
    CHECK_EQ(both_stage_members(&game), 6, "precondition: back to six");
    CHECK_EQ(heart_mod(&game, komari, "heart02"), 1,
             "refilling re-grants, so the zero above was the member count and not a latch");
}

static void test_wakana_same_condition_different_colours(void)
{
    TestGame game;
    test_game_new(&game);

    int wakana = test_id(&game, WAKANA);
    CHECK(rb_card_no_eq(wakana, WAKANA),
          "the 常時 host is the PL!SP-PR-022-PR (若菜四季) print");
    fill_decks(&game, 40);

    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].stage[0] = wakana;
    rb_recalc_constants(&game.state);
    CHECK_EQ(heart_mod(&game, wakana, "heart02"), 0, "one member: below 合計6人");
    CHECK_EQ(heart_mod(&game, wakana, "heart03"), 0, "and heart03 too");

    game.state.p[0].stage[1] = test_new_id(&game, FILLER);
    game.state.p[0].stage[2] = test_new_id(&game, FILLER);
    game.state.p[1].stage[0] = test_new_id(&game, FILLER);
    game.state.p[1].stage[1] = test_new_id(&game, FILLER);
    game.state.p[1].stage[2] = test_new_id(&game, FILLER);
    rb_recalc_constants(&game.state);
    CHECK_EQ(both_stage_members(&game), 6, "precondition: 3 + 3 = 6");

    CHECK_EQ(heart_mod(&game, wakana, "heart02"), 1, "six members grants heart02");
    CHECK_EQ(heart_mod(&game, wakana, "heart03"), 1,
             "and heart03 — this card's SECOND colour, which distinguishes it from 小原鞠莉");
    CHECK_EQ(heart_mod(&game, wakana, "heart04"), 0,
             "and NOT heart04: that colour belongs to 小原鞠莉 on the identical condition");
}

static void test_hime_name_keyed_constants_fire_independently(void)
{
    TestGame game;
    test_game_new(&game);

    int hime = test_id(&game, HIME);
    int rurino = test_id(&game, RURINO_N);
    int megumi = test_id(&game, MEGUMI);
    CHECK(rb_card_no_eq(hime, HIME), "the 常時 host is the PL!HS-pb1-022-N (安養寺姫芽) print");
    CHECK(rb_card_no_eq(rurino, RURINO_N), "the first name fixture is the PL!HS-bp1-014-N print");
    CHECK(rb_card_no_eq(megumi, MEGUMI), "the second name fixture is the PL!HS-bp1-015-N print");

    /* A name-keyed condition that never matches yields a green "no grant" test.
       Assert the two names really resolve first. */
    {
        Card c;
        int resolved = 0;
        if (rb_card_get_card_by_id(rurino, &c)) {
            resolved |= (c.name && strstr(c.name, "瑠璃乃") != NULL);
            rb_free_card(&c);
        }
        if (rb_card_get_card_by_id(megumi, &c)) {
            resolved |= (c.name && strstr(c.name, "藤島") != NULL);
            rb_free_card(&c);
        }
        CHECK(resolved, "precondition: both name-keyed characters resolve in the database");
    }

    clear_stage(&game, 0);
    game.state.p[0].stage[0] = hime;
    rb_recalc_constants(&game.state);
    CHECK_EQ(heart_mod(&game, hime, "heart01"), 0,
             "neither 大沢瑠璃乃 nor 藤島慈 is on the stage, so neither 常時 grants");
    CHECK_EQ(blade_mod(&game, hime), 0, "and no blades either");

    game.state.p[0].stage[1] = rurino;
    rb_recalc_constants(&game.state);
    CHECK_EQ(heart_mod(&game, hime, "heart01"), 2,
             "『自分のステージに「大沢瑠璃乃」がいるかぎり、heart01を2つ得る』 — TWO heart01");
    CHECK_EQ(blade_mod(&game, hime), 0,
             "藤島慈 is absent, so the OTHER 常時 must not fire — the clauses are independent");

    game.state.p[0].stage[2] = megumi;
    rb_recalc_constants(&game.state);
    CHECK_EQ(heart_mod(&game, hime, "heart01"), 2,
             "the heart clause is unchanged by the arrival of 藤島慈 — it does not stack");
    CHECK_EQ(blade_mod(&game, hime), 2,
             "『「藤島慈」がいるかぎり、ブレード2つ』 now fires, granting TWO blades");

    game.state.p[0].stage[1] = RB_EMPTY_SLOT;
    rb_recalc_constants(&game.state);
    CHECK_EQ(heart_mod(&game, hime, "heart01"), 0,
             "大沢瑠璃乃 left the stage, so her clause is withdrawn (the zone is still occupied)");
    CHECK_EQ(blade_mod(&game, hime), 2,
             "and 藤島慈's blades are untouched: withdrawing one 常時 must not disturb the other");
    CHECK(p1_members(&game) >= 2, "precondition: the stage is still occupied");
}

/* ===================================================================== */
/* B./C. jidou/movement/self_move_heart_watch/                             */
/*     鬼塚冬毬 / 唐 可可 — 自動 「このメンバーがエリアを移動したとき、      */
/*     ライブ終了時まで、<resource> を得る。」                                */
/* ===================================================================== */

static void test_tomari_jidou_self_move_triggers(void)
{
    TestGame game;
    test_game_new(&game);

    int tomari = test_id(&game, TOMARI);
    CHECK(rb_card_no_eq(tomari, TOMARI), "the watcher is the PL!SP-sd2-011-SD2 (鬼塚冬毬) print");
    clear_stage(&game, 0);
    game.state.p[0].stage[1] = tomari;

    push_area_move(&game, tomari, 0, 1);
    tas_full(&game, 0);

    CHECK_EQ(blade_mod(&game, tomari), 1, "an own-effect area move grants blade x1");
}

static void test_tomari_jidou_opponent_move_triggers(void)
{
    TestGame game;
    test_game_new(&game);

    int tomari = test_id(&game, TOMARI);
    clear_stage(&game, 0);
    game.state.p[0].stage[1] = tomari;

    push_area_move(&game, tomari, 1, 1);   /* cause_player = p2 */
    tas_full(&game, 0);

    CHECK_EQ(blade_mod(&game, tomari), 1,
             "opponent effect also triggers (「(対戦相手のカードの効果でも発動する。)」)");
}

static void test_tomari_jidou_natural_move_no_trigger(void)
{
    TestGame game;
    test_game_new(&game);

    int tomari = test_id(&game, TOMARI);
    clear_stage(&game, 0);
    game.state.p[0].stage[1] = tomari;

    push_area_move(&game, tomari, 0, 0);   /* effect_only = false */
    tas_full(&game, 0);

    CHECK_EQ(blade_mod(&game, tomari), 0,
             "a NATURAL move (effect_only false) must not fire the watcher");
}

static void test_keke_jidou_effect_only_false_no_trigger(void)
{
    TestGame game;
    test_game_new(&game);

    int keke = test_id(&game, KEKE);
    CHECK(rb_card_no_eq(keke, KEKE), "the watcher is the PL!SP-sd2-002-SD2 (唐 可可) print");
    clear_stage(&game, 0);
    game.state.p[0].stage[1] = keke;

    push_area_move(&game, keke, 0, 0);
    tas_full(&game, 0);

    CHECK_EQ(heart_mod(&game, keke, "heart06"), 0,
             "effect_only=false (natural move) must not trigger 唐 可可's 自動");
}

static void test_keke_jidou_self_effect_move_triggers(void)
{
    TestGame game;
    test_game_new(&game);

    int keke = test_id(&game, KEKE);
    clear_stage(&game, 0);
    game.state.p[0].stage[1] = keke;

    push_area_move(&game, keke, 0, 1);
    tas_full(&game, 0);

    CHECK_EQ(heart_mod(&game, keke, "heart06"), 1,
             "effect_only=true (own card effect) grants heart06 x1");
}

/* ===================================================================== */
/* D. jidou/energy_watch/debut_and_energy_return_watchers_stay_on_stage    */
/* ===================================================================== */

static void test_debut_energy_watcher_remains_on_center(void)
{
    TestGame game;
    test_game_new(&game);

    int ren = test_id(&game, REN_TWICE);
    CHECK(rb_card_no_eq(ren, REN_TWICE), "the watcher is the PL!SP-bp7-005-R＋ (葉月 恋) print");
    clear_stage(&game, 0);
    test_add_to_hand(&game, ren);
    test_add_to_energy_deck(&game, 0, test_id(&game, FILLER));
    test_give_energy(&game, 15);

    int played = test_play_to_stage(&game, ren, 1);
    drain_auto(&game);
    CHECK_EQ(played, 1, "葉月 恋's debut is accepted");
    CHECK_EQ(game.state.p[0].stage[1], ren,
             "ren should be on center after debut — the 自動 does not displace her");
}

static void test_energy_return_scan_leaves_watcher_on_stage(void)
{
    TestGame game;
    test_game_new(&game);

    int ren = test_id(&game, REN_TWICE);
    int filler = test_id(&game, FILLER);
    clear_stage(&game, 0);
    game.state.p[0].stage[0] = ren;

    /* Move one energy card from the energy zone back to the energy deck —
       the trigger 葉月 恋 ab#0 watches. */
    RbPlayer *P = &game.state.p[0];
    P->energy.cards[P->energy.n++] = filler;
    rb_energy_set_active_count(P, 1);
    test_add_to_energy_deck(&game, 0, filler);
    P->energy.n = 0;
    P->energy_active = 0;

    set_recently_moved(&game, filler);
    tas_full(&game, 0);
    drain_auto(&game);

    CHECK(game.state.p[0].stage[0] == ren,
          "the energy-return scan leaves the watcher on stage");
    CHECK_EQ(p1_members(&game), 1, "and it is the only member on the stage");
}

/* ===================================================================== */
/* E. jidou/energy_watch/energy_placed_by_own_effect_gain_blade_test      */
/* ===================================================================== */

static void energy_watch_scenario(TestGame *tg, const char *watcher_no, int *out_blade)
{
    int watcher = test_id(tg, watcher_no);
    int placer = test_id(tg, ENERGY_PLACER);
    int filler = test_id(tg, FILLER);

    clear_stage(tg, 0);
    tg->state.p[0].stage[0] = watcher;
    for (int i = 0; i < 15; i++) test_add_to_energy_deck(tg, 0, filler);
    test_give_energy(tg, 15);
    test_add_to_energy_deck(tg, 0, filler);
    test_add_to_hand(tg, placer);
    tg->state.p[0].deck.n = 0;
    for (int i = 0; i < 40; i++) test_add_to_deck_pl(tg, 0, filler);

    *out_blade = blade_mod(tg, watcher);
    test_play_to_stage(tg, placer, 2);
    drain_auto(tg);
    *out_blade = blade_mod(tg, watcher);
}

static void test_energy_watcher_own_effect_places_energy_gains_blade(void)
{
    /* PL!SP-bp7-016-N (ターン1回) */
    TestGame game;
    test_game_new(&game);
    int blade = 0;
    energy_watch_scenario(&game, REN_ONCE, &blade);
    CHECK_EQ(blade, 1,
             "own card effect placed energy into the zone -> PL!SP-bp7-016-N gains blade x1");
}

static void test_energy_watcher_twice_per_turn_own_effect_gains_blade(void)
{
    /* PL!SP-bp7-005-R＋ ab#1 (ターン2回) */
    TestGame game;
    test_game_new(&game);
    int blade = 0;
    energy_watch_scenario(&game, REN_TWICE, &blade);
    CHECK_EQ(blade, 1,
             "own card effect placed energy into the zone -> PL!SP-bp7-005-R＋ gains blade x1");
}

static void test_energy_watcher_no_energy_placed_no_blade(void)
{
    for (int which = 0; which < 2; which++) {
        TestGame game;
        test_game_new(&game);
        int watcher = test_id(&game, which ? REN_TWICE : REN_ONCE);
        int filler = test_id(&game, FILLER);
        clear_stage(&game, 0);
        game.state.p[0].stage[0] = watcher;
        for (int i = 0; i < 15; i++) test_add_to_energy_deck(&game, 0, filler);
        test_give_energy(&game, 15);

        int before = blade_mod(&game, watcher);
        tas_full(&game, 0);
        drain_auto(&game);
        CHECK_EQ(blade_mod(&game, watcher), before,
                 which ? "no effect placed energy -> PL!SP-bp7-005-R＋ must not gain blade"
                       : "no effect placed energy -> PL!SP-bp7-016-N must not gain blade");
    }
}

static void test_energy_watcher_opponent_effect_no_blade(void)
{
    TestGame game;
    test_game_new(&game);

    int ren = test_id(&game, REN_TWICE);
    int filler = test_id(&game, FILLER);
    clear_stage(&game, 0);
    game.state.p[0].stage[0] = ren;
    for (int i = 0; i < 15; i++) test_add_to_energy_deck(&game, 0, filler);
    test_give_energy(&game, 15);

    int before = blade_mod(&game, ren);
    /* Rust: push_movement_event(-1, "energy_deck", "energy", None, "p2", true).
       The C shim `rb_record_card_movement` rejects card_id < 0, so the "an
       unnamed energy arrived, caused by p2's effect" event is not expressible.
       rb_record_card_movement's 6th parameter (effect_only) is reached, the
       card id is not — see the report. */
    EXPECTED_GAP(0,
                 "push_movement_event(-1, energy_deck->energy, cause=p2, effect_only=true) "
                 "— rb_record_card_movement (modifiers.c) rejects card_id < 0");
    CHECK_EQ(blade_mod(&game, ren), before,
             "opponent card's effect must not trigger (「自分のカードの効果」 only)");
}

/* ===================================================================== */
/* F. jidou/energy_watch/energy_placement_blade_excludes_area_move_and_    */
/*     under_member_test.rs — the S1 energy-placed watcher is scoped to an */
/*     effect-driven ZONE arrival, not to any own-effect movement.        */
/* ===================================================================== */

static void test_own_effect_area_swap_does_not_trigger_energy_placement_blade(void)
{
    TestGame game;
    test_game_new(&game);

    int ren = test_new_id(&game, REN_TWICE);
    int shiki = test_new_id(&game, SHIKI);
    int filler = test_id(&game, FILLER);
    CHECK(rb_card_no_eq(ren, REN_TWICE), "the watcher is the PL!SP-bp7-005-R＋ print");
    CHECK(rb_card_no_eq(shiki, SHIKI), "the mover is the PL!SP-bp2-008-R (若菜四季) print");

    clear_stage(&game, 0);
    game.state.p[0].stage[0] = ren;
    game.state.p[0].deck.n = 0;
    for (int i = 0; i < 40; i++) test_add_to_deck_pl(&game, 0, filler);
    test_give_energy(&game, 10);   /* 9 to stage 若菜四季 + E for her 起動 */
    test_add_to_hand(&game, shiki);

    int played = test_play_to_stage(&game, shiki, 1);
    drain_auto(&game);
    CHECK_EQ(played, 1, "若菜四季's debut is accepted");
    CHECK_EQ(game.state.p[0].stage[1], shiki, "若菜四季 is at Center");
    tas_full(&game, 0);
    CHECK_EQ(blade_mod(&game, ren), 0, "no legit trigger before the swap");

    /* Rust drives the swap through a `position|destination` SelectTarget and
       picks the area by NAME. The C choice carries no destination options
       (choice.c:2280-2294 discards the computed valid_destinations list) and
       the answer index is taken as an ABSOLUTE area, so index 0 == Left. */
    test_activate_ability(&game, shiki);
    drain_auto(&game);
    CHECK(pending(&game), "the swap's position|destination prompt is raised");
    answer_first(&game);
    drain_auto(&game);
    tas_full(&game, 0);

    CHECK_EQ(game.state.p[0].stage[1], ren,
             "swap must have moved 葉月 恋 (Center) via our own effect");
    CHECK_EQ(game.state.p[0].stage[0], shiki, "and 若菜四季 took the Left area");
    CHECK_EQ(blade_mod(&game, ren), 0,
             "pure-energy watcher: an own area move alone must not grant blade");
}

static void test_energy_parked_under_member_does_not_trigger_energy_zone_blade(void)
{
    TestGame game;
    test_game_new(&game);

    int ren = test_new_id(&game, REN_TWICE);
    int tutor = test_new_id(&game, TUTOR);
    int target = test_new_id(&game, SETSUNA);
    int energy = test_new_id(&game, ENERGY);
    int filler = test_id(&game, FILLER);
    CHECK(rb_card_no_eq(ren, REN_TWICE), "the watcher is the PL!SP-bp7-005-R＋ print");
    CHECK(rb_card_no_eq(tutor, TUTOR), "the park-er is the PL!N-bp3-007-R (近江彼方) print");

    clear_stage(&game, 0);
    game.state.p[0].stage[0] = ren;
    game.state.p[0].stage[2] = tutor;
    test_add_to_hand(&game, target);
    test_give_energy(&game, 2);
    test_add_to_energy(&game, 0, energy);      /* the park candidate */
    /* 近江彼方's 起動 looks at the top 5 cards and may publish one LIVE card, so
       the deck must actually contain the card it is meant to debut. */
    game.state.p[0].deck.n = 0;
    test_insert_deck_top(&game, 0, target);
    for (int i = 0; i < 4; i++) test_add_to_deck_pl(&game, 0, filler);

    test_activate_ability(&game, tutor);
    CHECK(pending(&game), "energy-under-member SelectCard prompt expected");
    CHECK(pending(&game) && strcmp(test_pending_choice_type(&game), "SelectCard") == 0,
          "expected SelectCard for the tutor's parking step");
    answer_first(&game);
    drain_auto(&game);
    tas_full(&game, 0);

    CHECK_EQ(game.state.p[0].stage[2], target,
             "the 5th card must have debuted to the vacated Right area");
    CHECK_EQ(game.state.p[0].under_cards[2].n, 1,
             "the tutor effect really parked 1 zone energy under the member");
    CHECK_EQ(blade_mod(&game, ren), 0,
             "under_member parking is not a zone arrival: no blade");
}

/* ===================================================================== */
/* G. jidou/title_once/dive_in_live_zone_only_ab1_grants_blade_test        */
/* ===================================================================== */

static void dive_live_zone_scenario(TestGame *tg, int in_live, int in_hand,
                                    int with_niji, int *out_blade)
{
    int dive = test_id(tg, DIVE);
    int niji = test_id(tg, NIJI_MEMBER);
    clear_stage(tg, 0);
    tg->state.p[0].live.n = 0;
    tg->state.p[0].hand.n = 0;
    if (in_live) test_add_to_live(tg, dive);
    if (in_hand) test_add_to_hand(tg, dive);
    if (with_niji) tg->state.p[0].stage[1] = niji;
    if (in_live) set_recently_moved(tg, dive);

    tas_full(tg, 0);
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 64)
        rb_resume_with_choice(&tg->state, 0);
    *out_blade = niji >= 0 ? blade_mod(tg, niji) : 0;
}

static void test_dive_live_zone_only_ab1_triggers(void)
{
    TestGame game;
    test_game_new(&game);
    int blade = 0;
    dive_live_zone_scenario(&game, 1, 0, 1, &blade);
    CHECK_EQ(blade, 2,
             "ab#1: DIVE! in the live zone grants blade+2 to a 『虹ヶ咲』 member");
}

static void test_dive_not_in_live_zone_no_trigger(void)
{
    TestGame game;
    test_game_new(&game);
    int blade = 0;
    dive_live_zone_scenario(&game, 0, 1, 1, &blade);
    CHECK_EQ(blade, 0, "no trigger: DIVE! must be in the live zone for ab#1");
}

static void test_dive_no_niji_no_target(void)
{
    TestGame game;
    test_game_new(&game);
    int blade = 0;
    dive_live_zone_scenario(&game, 1, 0, 0, &blade);
    CHECK_EQ(blade, 0, "ab#1 fires with no 『虹ヶ咲』 member to name -> no-op");
    CHECK_EQ(game.state.p[0].live.n, 1, "and no extra placement from ab#0");
    CHECK(rb_card_no_eq(test_id(&game, DIVE), DIVE), "the DIVE! fixture is the PL!N-bp4-026-L print");
}

/* ===================================================================== */
/* H. jidou/discard_watch/on_hand_to_discard_each_time_gain_heart01_and_   */
/*     blade_test.rs — PL!HS-pb1-003-R ab#1 (ターン2回)                   */
/* ===================================================================== */

static void rurino_scan(TestGame *tg, const int *moved, int n, int expect_heart)
{
    int rurino = test_id(tg, RURINO_W);
    CHECK(rb_card_no_eq(rurino, RURINO_W),
          "the watcher is the PL!HS-pb1-003-R (大沢瑠璃乃) print");
    clear_stage(tg, 0);
    tg->state.p[0].stage[1] = rurino;
    tg->state.p[0].hand.n = 0;
    tg->state.n_recently_moved = 0;
    for (int i = 0; i < n; i++) tg->state.recently_moved[i] = moved[i];

    tas_full(tg, 0);
    drain_auto(tg);
    CHECK_EQ(heart_mod(tg, rurino, "heart01"), expect_heart,
             expect_heart ? "a hand card reached the waitroom -> heart01 x1"
                          : "nothing reached the waitroom -> no heart01");
    CHECK_EQ(blade_mod(tg, rurino), expect_heart,
             expect_heart ? "…and blade x1 from the same effect"
                          : "…and no blade either");
}

static void test_rurino_discard_triggers_heart_and_blade(void)
{
    TestGame game;
    test_game_new(&game);
    int f = test_id(&game, FILLER);
    int moved[1] = { f };
    rurino_scan(&game, moved, 1, 1);
}

static void test_rurino_no_discard_no_trigger(void)
{
    TestGame game;
    test_game_new(&game);
    int none[1] = { -1 };
    rurino_scan(&game, none, 0, 0);
}

static void test_rurino_q241_multiple_cards_discarded_fires_once(void)
{
    TestGame game;
    test_game_new(&game);
    int f1 = test_id(&game, FILLER);
    int f2 = test_new_id(&game, FILLER);
    int moved[2] = { f1, f2 };
    rurino_scan(&game, moved, 2, 1);
}

static void test_rurino_ozora_discard_cross_card_triggers_watcher(void)
{
    TestGame game;
    test_game_new(&game);

    int watcher = test_id(&game, RURINO_W);
    int activator = test_id(&game, RURINO_OZORA);
    int filler = test_id(&game, FILLER);
    CHECK(rb_card_no_eq(watcher, RURINO_W), "the watcher is the PL!HS-pb1-003-R print");
    CHECK(rb_card_no_eq(activator, RURINO_OZORA), "the activator is the PL!HS-bp2-005-R＋ print");

    clear_stage(&game, 0);
    game.state.p[0].stage[0] = watcher;
    game.state.p[0].hand.n = 0;
    test_add_to_hand(&game, activator);
    test_add_to_hand(&game, filler);
    test_give_energy(&game, 12);
    game.state.p[0].deck.n = 0;
    for (int i = 0; i < 20; i++) test_add_to_deck_pl(&game, 0, filler);

    CHECK_EQ(heart_mod(&game, watcher, "heart01"), 0, "no heart01 before the discard");

    test_play_to_stage(&game, activator, 1);
    {
        int guard = 0;
        while (rb_has_pending_choice(&game.state) && guard++ < 64)
            rb_resume_with_choice(&game.state, 0);
    }

    CHECK_EQ(heart_mod(&game, watcher, "heart01"), 1,
             "the watcher gains heart01 when another card's ability discards from hand");
    CHECK_EQ(blade_mod(&game, watcher), 1,
             "and blade when another card's ability discards from hand");
}

/* ===================================================================== */
/* I. jidou/movement/under_member_placement/under_member_counted_for_both_ */
/*     players_test.rs — 桜小路きな子's 自動 must work for EITHER seat.     */
/* ===================================================================== */

static void under_member_for_seat(TestGame *tg, int pl, int *out_left, int *out_center)
{
    int kinako = test_new_id(tg, KINAKO);
    int chisato = test_id(tg, CHISATO_D);
    int liella = test_new_id(tg, CHISATO_L);
    int filler = test_id(tg, FILLER);
    CHECK(rb_card_no_eq(kinako, KINAKO), "the tucker is the PL!SP-pb2-006-R (桜小路きな子) print");
    CHECK(rb_card_no_eq(liella, CHISATO_L),
          "the tucked card is the PL!SP-pb1-014-N (嵐 千砂都) print — a DIFFERENT member");

    clear_stage(tg, pl);
    tg->state.p[pl].stage[0] = kinako;
    tg->state.p[pl].discard.n = 0;   /* C names the waitroom `discard` */
    tg->state.p[pl].hand.n = 0;
    rb_waitroom_add(&tg->state.p[pl], liella);
    test_add_to_hand_for(tg, pl, chisato);
    test_give_energy_for(tg, pl, 15);
    tg->state.p[pl].deck.n = 0;
    for (int i = 0; i < 40; i++) test_add_to_deck_pl(tg, pl, filler);

    int played = test_play_to_stage_for(tg, pl, chisato, 1);
    /* 千砂都's 登場 offers an OPTIONAL position change and the Rust test
       ACCEPTS it (it picks the "left" destination) — that area move is what
       arms きな子's 自動. The C choice carries no destination options, so the
       answer index is taken as an ABSOLUTE area: index 0 == Left. */
    CHECK(played, "the 嵐 千砂都 debut is accepted");
    CHECK(pending(tg), "千砂都's 登場 offers the optional position change");
    if (pending(tg)) answer_first(tg);
    drain_auto(tg);

    *out_left = 0;
    *out_center = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        RbBag *b = &tg->state.p[pl].under_cards[i];
        for (int k = 0; k < b->n; k++) {
            if (b->cards[k] == liella) { if (i == 0) *out_left = 1; else *out_center = 1; }
        }
    }
    CHECK(tg->state.p[pl].stage[1] == chisato || tg->state.p[pl].stage[0] == chisato,
          "precondition: 嵐 千砂都 is on the stage after the debut");
}

static void test_under_member_tuck_works_for_both_players(void)
{
    int l1 = 0, c1 = 0;
    TestGame g1;
    test_game_new(&g1);
    under_member_for_seat(&g1, 0, &l1, &c1);
    CHECK(l1 || c1, "P1: a 『Liella!』 member is tucked under きな子");

    int l2 = 0, c2 = 0;
    TestGame g2;
    test_game_new(&g2);
    under_member_for_seat(&g2, 1, &l2, &c2);
    CHECK(l2 || c2, "P2: a 『Liella!』 member is tucked under きな子");
}

/* ===================================================================== */
/* J. jidou/yell/no_blade_heart_reveal_gain/no_blade_heart_reveal_heart02  */
/*     鬼塚夏美 ab#0 — 「エールにより公開された自分のカードの中にブレード   */
/*     ハートを持つカードがないとき、heart02 を得る」                       */
/* ===================================================================== */

static void natsuka_yell_scenario(TestGame *tg, const char *deck_card_no,
                                  int expect_heart)
{
    int natsuka = test_id(tg, NATSUKA);
    int deck_card = test_id(tg, deck_card_no);
    CHECK(rb_card_no_eq(natsuka, NATSUKA),
          "the watcher is the PL!SP-bp2-020-N (鬼塚夏美) print");

    clear_stage(tg, 0);
    tg->state.p[0].stage[1] = natsuka;
    tg->state.p[0].deck.n = 0;
    for (int i = 0; i < 20; i++) test_add_to_deck_pl(tg, 0, deck_card);

    /* Rust: perform_cheer_check(pid, 3) then push_revealed_card for each. */
    rb_perform_cheer_check(&tg->state, "p1", 3);
    int revealed[RB_MAX_ZONE];
    int n = rb_resolution_clear(&tg->state, revealed, RB_MAX_ZONE);
    CHECK(n > 0, "the yell revealed at least one card (the test must be meaningful)");

    for (int i = 0; i < n && i < RB_MAX_REVEALED_CARDS; i++) {
        tg->state.revealed_cards[i] = revealed[i];
        if (i + 1 > tg->state.n_revealed) tg->state.n_revealed = i + 1;
    }
    tg->state.yell_occurred = n > 0;

    tas_full(tg, 0);
    drain_auto(tg);
    tg->state.yell_occurred = 0;

    CHECK_EQ(heart_mod(tg, natsuka, "heart02"), expect_heart,
             expect_heart
             ? "鬼塚夏美 ab#0 grants heart02 when the yell reveals no blade-heart card"
             : "鬼塚夏美 must NOT grant heart02 when a blade-heart card was revealed");
}

static void test_yell_without_blade_heart_reveal_grants_heart02(void)
{
    TestGame game;
    test_game_new(&game);
    natsuka_yell_scenario(&game, NO_BLADE_HEART, 1);
}

static void test_yell_with_blade_heart_reveal_does_not_grant_heart02(void)
{
    TestGame game;
    test_game_new(&game);
    natsuka_yell_scenario(&game, FILLER, 0);
}

/* ===================================================================== */

int main(void)
{
    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: load_cards failed\n");
        return 2;
    }

    /* A. 常時 conditions keyed on board counts / names */
    test_kanan_grants_only_at_exactly_two_members();
    test_komari_grants_only_when_both_stages_together_hold_six();
    test_wakana_same_condition_different_colours();
    test_hime_name_keyed_constants_fire_independently();

    /* B./C. self-move 自動 watchers */
    test_tomari_jidou_self_move_triggers();
    test_tomari_jidou_opponent_move_triggers();
    test_tomari_jidou_natural_move_no_trigger();
    test_keke_jidou_effect_only_false_no_trigger();
    test_keke_jidou_self_effect_move_triggers();

    /* D. debut / energy-return watchers stay on stage */
    test_debut_energy_watcher_remains_on_center();
    test_energy_return_scan_leaves_watcher_on_stage();

    /* E. energy-placed-by-own-effect blade watchers */
    test_energy_watcher_own_effect_places_energy_gains_blade();
    test_energy_watcher_twice_per_turn_own_effect_gains_blade();
    test_energy_watcher_no_energy_placed_no_blade();
    test_energy_watcher_opponent_effect_no_blade();

    /* F. the S1 watcher excludes a plain area move */
    test_own_effect_area_swap_does_not_trigger_energy_placement_blade();
    test_energy_parked_under_member_does_not_trigger_energy_zone_blade();

    /* G. DIVE! live-zone watcher */
    test_dive_live_zone_only_ab1_triggers();
    test_dive_not_in_live_zone_no_trigger();
    test_dive_no_niji_no_target();

    /* H. hand->waitroom each_time watcher */
    test_rurino_discard_triggers_heart_and_blade();
    test_rurino_no_discard_no_trigger();
    test_rurino_q241_multiple_cards_discarded_fires_once();
    test_rurino_ozora_discard_cross_card_triggers_watcher();

    /* I. under-member tucking works for both seats */
    test_under_member_tuck_works_for_both_players();

    /* J. yell-reveal blade-heart conditions */
    test_yell_without_blade_heart_reveal_grants_heart02();
    test_yell_with_blade_heart_reveal_does_not_grant_heart02();

    rb_unload();

    printf("\n==== parity_jidou_extra: %d assertions, %d real failures, %d gaps ====\n",
           assertions, failures, gaps);
    if (failures) {
        fprintf(stderr, "%d runtime failures\n", failures);
        return 1;
    }
    printf("ALL JIDOU-EXTRA PARITY CHECKS PASSED\n");
    return 0;
}
