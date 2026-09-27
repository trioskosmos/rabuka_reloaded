/* C port of engine/tests/test_modules/rules/trigger_paths/.
 *
 * Covers the five trigger-path areas ported from the Rust suite:
 *   1. DEBUT          - q159_debut_from_discard_self_cost_test.rs
 *   2. LIVE START     - zone_change_gate_test.rs (Reina live-start auto)
 *   3. MOVEMENT       - zone_change_gate_test.rs (Riko live->waitroom,
 *                       Miyamiya baton-touch stage->waitroom)
 *   4. DEBUT WATCHERS - debut_watcher_baton_and_hand_activation_q196..q198.rs
 *   5. ONCE LIMITS    - turn_limited_activation_offered_once_test.rs
 *
 * Rust helpers mirrored here:
 *   helpers::fill_decks        -> fill_decks
 *   helpers::fire_trigger      -> fire_trigger (queue the ability whose
 *                                 `triggers` string equals `trig`, then drain)
 *   game.state.push_movement_event -> rb_trigger_auto_abilities_for_player_with_event
 *   game.state.deployed_this_turn.clear() -> reset_stage_arrived
 *   helpers::select_indices(&[]) -> drain_skip
 *   helpers::answer_choice(0)      -> drain_first
 *
 * Japanese trigger keywords appear as octal escapes so this file stays pure
 * ASCII (the Write tool emits UTF-8 without a BOM; MSVC/GCC both cope, but
 * keeping the source ASCII avoids any encoding round-trip damage).
 *   U+767A U+73FE  = "Debut"        \347\232\273\347\243\276
 *   U+30E9 U+30A4 U+30D6 U+958B U+59CB U+6642 = "LiveStart"
 *                                          \343\203\251\343\202\244\343\203\266\346\225\255\345\216\237\346\231\202
 */

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

/* The byte sequences below are the EXACT UTF-8 of the printed trigger keyword
   each Rust original names. The rescued revision carried two corrupt escapes
   (\343\203\266 for U+30D6 "?" and \350\205\215 for U+8D77 "?"), so every
   strcmp against the decoder's real trigger string missed and the affected
   cases silently tested nothing. Verify with dump_abilities, which prints the
   decoded `triggers` next to the card number.
   TRIG_DEBUT      = U+767A U+73FE           \347\232\273\347\243\276
   TRIG_LIVE_START = U+30E9 U+30A4 U+30D6 U+958B U+59CB U+6642
                                                  \343\203\226 (U+30D6 "?")
                                                   \346\225\255\345\216\237\346\231\202
   TRIG_AUTO       = U+81EA U+52FA           \350\207\252\345\213\225
   TRIG_ACTIVATION = U+8D77 U+52FA           \350\265\267\345\213\225
                                                  (U+8D77 "?" is E8 B5 B7) */
#define TRIG_DEBUT      "\347\232\273\347\243\276"
#define TRIG_LIVE_START "\343\203\251\343\202\244\343\203\226\346\225\255\345\216\237\346\231\202"
#define TRIG_AUTO       "\350\207\252\345\213\225"
#define TRIG_ACTIVATION "\350\265\267\345\213\225"

/* „Ÿ„Ÿ shared helpers „Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ */

#define FILLER "PL!-sd1-010-SD"

static void dump_abilities(int card_id);

static void fill_decks(TestGame *game, int filler) {
    for (int pl = 0; pl < 2; pl++) {
        game->state.p[pl].deck.n = 0;
        for (int i = 0; i < 30; i++) test_add_to_deck_pl(game, pl, filler);
    }
}

/* Answer every pending choice with "skip / nothing selected"
   (Rust: `while game.has_pending_choice() { game.select_indices(&[]); }`).
   SelectAutoAbility prompts are answered "proceed" so a queued auto still
   resolves instead of being declined. */
static void drain_skip(TestGame *game) {
    int guard = 0;
    while (test_has_pending_choice(game) && guard++ < 40) {
        const char *type = test_pending_choice_type(game);
        if (type && strcmp(type, "SelectAutoAbility") == 0) {
            int proceed[] = { 0 };
            test_select_indices(game, proceed, 1);
        } else {
            test_select_indices(game, NULL, 0);
        }
    }
}

/* Rust: `while game.has_pending_choice() { game.select_indices(&[]); }`
   with NO per-type special case. The Q197/Q198 originals use exactly this, so
   a queued auto is DECLINED there; drain_skip's "proceed" policy is a
   different drain and must not be substituted for it. */
static void drain_decline(TestGame *game) {
    int guard = 0;
    while (test_has_pending_choice(game) && guard++ < 40)
        test_select_indices(game, NULL, 0);
}

/* Q159's Rust original uses a per-type policy (rules_test.rs
   q159_debut_from_discard_self_cost_test.rs:59-78): SelectCard and any other
   prompt take index 0, while SelectAutoAbility and SelectTarget DECLINE.
   The rescued C port answered SelectCard with [] and SelectAutoAbility with
   [0] - the exact inverse - so it never even offered Shioriko's debut and the
   waitroom assertion held for the wrong reason. */
static void drain_q159(TestGame *game) {
    int guard = 0;
    while (test_has_pending_choice(game) && guard++ < 40) {
        const char *type = test_pending_choice_type(game);
        if (type && (strcmp(type, "SelectAutoAbility") == 0 ||
                     strcmp(type, "SelectTarget") == 0)) {
            test_select_indices(game, NULL, 0);
        } else {
            int first[] = { 0 };
            test_select_indices(game, first, 1);
        }
    }
}

/* Answer every pending choice with the FIRST offered option
   (Rust: `answer_choice(&mut game, 0)` / `select_generated(0)`). */
static void drain_first(TestGame *game) {
    int guard = 0;
    while (test_has_pending_choice(game) && guard++ < 40) {
        test_resume_choice(game, 0);
    }
}

/* Mirrors helpers::fire_trigger - find the ability whose printed `triggers`
   string is exactly `trig`, enqueue it with the card as the activating card,
   then resolve the pending auto abilities. Returns 0 when the card has no
   such ability (a setup bug the callers assert on). */
static int fire_trigger(TestGame *game, int card_id, const char *trig) {
    int n = rb_card_num_abilities((uint32_t)card_id);
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, i, &ab)) continue;
        int match = ab.triggers && strcmp(ab.triggers, trig) == 0;
        rb_free_ability(&ab);
        if (!match) continue;
        rb_queue_push(&game->state.queue, card_id, i);
        game->state.activating_card = card_id;
        rb_drain_ability_queue(&game->state);
        return 1;
    }
    return 0;
}

/* Mirrors zone_change_gate_test.rs::move_live_to_waitroom - a real
   live_card_zone -> waitroom zone change PLUS the movement event that arms
   the movement watchers (Rust push_movement_event(..., "live_card_zone",
   "discard", None, causer, true)).
   The rescued C port moved the card and then scanned, but never pushed the
   EVENT. Every ?...??...???????? gate reads the zone pair out of
   g->batch_movements (condition.c:2175-2191), so with an empty movement log
   the whole family silently evaluated to "nothing moved" and the Riko cases
   passed/failed for the wrong reason. rb_record_card_movement is that push.
   Takes an explicit seat so P2 ownership is testable, and returns 0 when the
   card was not in that seat's live zone (Rust asserts; a silent return would
   let a setup bug masquerade as an engine verdict). */
static int move_live_to_waitroom_event(TestGame *game, int pl, int card) {
    RbPlayer *P = &game->state.p[pl];
    int found = 0;
    for (int i = 0; i < P->live.n; i++) {
        if (P->live.cards[i] != card) continue;
        for (int j = i; j < P->live.n - 1; j++) P->live.cards[j] = P->live.cards[j + 1];
        P->live.n--;
        found = 1;
        break;
    }
    if (!found) return 0;
    if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = card;
    rb_record_card_movement(&game->state, card, RB_ZONEID_LIVE_CARD_ZONE,
                            RB_ZONEID_DISCARD, pl, 1);
    return 1;
}

/* Rust: TurnEngine::trigger_auto_abilities_for_player(&state, &pid) followed
   by state.process_pending_auto_abilities(&pid). The scan is kept SEPARATE
   from the event push because the Riko ownership case scans P1 first and
   asserts she is silent - folding the scan into the helper would have armed
   P2's copy before P1 was ever asked. */
static void scan_auto_for_player(TestGame *game, int pl) {
    rb_trigger_auto_abilities_for_player(&game->state, pl);
    rb_process_pending_auto_abilities(&game->state);
}

/* Mirrors `game.state.player1.deployed_this_turn.clear()`: forget that the
   first member already arrived, so the second play reads as a fresh arrival
   rather than an illegal second arrival into the same turn. */
static void reset_stage_arrived(TestGame *game, int pl) {
    for (int i = 0; i < RB_STAGE_SIZE; i++) game->state.stage_arrived[pl][i] = 0;
    game->state.n_cards_appeared_this_turn = 0;
    game->state.cards_appeared_this_turn[0] = 0;
}

static int printed_cost(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return -1;
    int cost = c.cost;
    rb_free_card(&c);
    return cost;
}

static int printed_blade_heart(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int has = rb_card_has_blade_heart(&c);
    rb_free_card(&c);
    return has;
}

/* AGENTS.md identity rule. A trigger test that stages the wrong print proves
   nothing, and this fixture set is full of look-alikes that differ only by a
   set-code transposition: PL!N-pb1-009-R (Reina) vs PL!N-bp4-009-R, and
   PL!N-bp5-005-R+ (Miyamiya) vs PL!-bp5-005-R. Every fixture is asserted
   against the exact card_no the Rust original names. */
static void expect_card_no(int cid, const char *no) {
    char msg[192];
    snprintf(msg, sizeof(msg), "fixture cid=%d prints card_no %s", cid, no);
    CHECK(rb_card_no_eq(cid, no), msg);
    if (!rb_card_no_eq(cid, no)) dump_abilities(cid);
}

static int energy_zone_count(TestGame *game, int pl) {
    return game->state.p[pl].energy.n;
}

/* One-shot decoder sanity sweep: prints how many of the compiled ability
   slices decode at all. A blob/offset-table mismatch shows up here as a
   near-100% failure rate rather than as per-card weirdness. */
static void dump_decoder_health(void) {
    uint32_t total = rb_num_abilities();
    uint32_t ok = 0, empty = 0, bad = 0;
    for (uint32_t i = 0; i < total; i++) {
        uint32_t len = 0;
        rb_bc_slice(i, &len);
        if (len == 0) { empty++; continue; }
        Ability ab;
        if (rb_decode_ability(i, &ab)) { ok++; rb_free_ability(&ab); }
        else bad++;
    }
    fprintf(stderr, "  [decoder] abilities=%u ok=%u empty=%u bad=%u\n",
            total, ok, empty, bad);
}

/* Dump a card's abilities to stderr. Setup diagnostics only: a mis-spelled
   card_no or a shifted ability index is invisible otherwise, and AGENTS.md
   is explicit that a test staging the wrong print proves nothing. */
static void dump_abilities(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) {
        fprintf(stderr, "  [abilities] cid=%d cannot decode\n", card_id);
        return;
    }
    fprintf(stderr, "  [abilities] cid=%d no=%s cost=%d n=%d\n",
            card_id, rb_card_string(c.card_no_idx), c.cost,
            rb_card_num_abilities((uint32_t)card_id));
    rb_free_card(&c);
    int n = rb_card_num_abilities((uint32_t)card_id);
    for (int i = 0; i < n; i++) {
        uint32_t aidx = 0xFFFFFFFFu;
        rb_card_get_ability_idx((uint32_t)card_id, i, &aidx);
        uint32_t blen = 0;
        rb_bc_slice(aidx, &blen);
        Ability ab;
        int ok = rb_decode_card_ability((uint32_t)card_id, i, &ab);
        fprintf(stderr, "    #%d ability_idx=%u slice_len=%u decode=%d", i, aidx, blen, ok);
        if (ok) {
            fprintf(stderr, " triggers=%s use_limit=%d action=%s",
                    ab.triggers ? ab.triggers : "(none)", ab.use_limit,
                    (ab.effect && ab.effect->action) ? ab.effect->action : "(no effect)");
            rb_free_ability(&ab);
        }
        fprintf(stderr, "\n");
    }
}

/* Assert that `cid` exposes at least `min_abilities` abilities and that one of
   them carries exactly `trig`. Rust's helpers::fire_trigger panics when the
   card lacks the requested trigger, so a C port that silently no-ops would
   assert nothing at all. */
static void expect_trigger(int cid, const char *trig, int min_abilities) {
    int n = rb_card_num_abilities((uint32_t)cid);
    char msg[160];
    snprintf(msg, sizeof(msg), "fixture cid=%d exposes >= %d abilities", cid,
             min_abilities);
    CHECK(n >= min_abilities, msg);
    int found = 0;
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        if (ab.triggers && strcmp(ab.triggers, trig) == 0) found = 1;
        rb_free_ability(&ab);
    }
    snprintf(msg, sizeof(msg), "fixture cid=%d has a '%s' ability", cid, trig);
    CHECK(found, msg);
    if (!found) {
        /* Two trigger strings that differ by a single UTF-8 byte render
           identically in a mis-encoded console, so print the bytes. */
        const unsigned char *w = (const unsigned char *)trig;
        fprintf(stderr, "  [trigger] want hex:");
        for (const unsigned char *q = w; *q; q++) fprintf(stderr, " %02X", *q);
        fprintf(stderr, "\n");
        for (int i = 0; i < n; i++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
            if (ab.triggers) {
                const unsigned char *h = (const unsigned char *)ab.triggers;
                fprintf(stderr, "  [trigger] got  #%d hex:", i);
                for (const unsigned char *q = h; *q; q++) fprintf(stderr, " %02X", *q);
                fprintf(stderr, "\n");
            }
            rb_free_ability(&ab);
        }
        dump_abilities(cid);
    }
}

/* ??????????????????????????????????????????????????????????????????
   1. DEBUT - Q159 (q159_debut_from_discard_self_cost_test.rs)
   ?????????????????????????????????????????????????????????????????? */

/* Sayaka (cost 9): debut activates a Nijigasaki member from the waitroom. */
#define SAYAKA "PL!N-bp3-003-R"
/* Shioriko (cost 4): debut self-cost is "this member to wait". */
#define SHIORIKO "PL!N-bp3-022-N"
/* A Nijigasaki member at cost <= 4 without a wait self-cost. */
#define KASUMI_NIJI "PL!N-bp7-017-N"

static void q159_debut_from_discard_self_cost_wait_not_payable(void) {
    TestGame game;
    test_game_new(&game);

    int sayaka = test_id(&game, SAYAKA);
    int shioriko = test_id(&game, SHIORIKO);
    int filler = test_id(&game, "PL!-sd1-002-SD");
    int live = test_id(&game, "PL!-sd1-019-SD");
    CHECK(sayaka >= 0 && shioriko >= 0 && filler >= 0 && live >= 0,
          "Q159: fixtures resolve");
    if (sayaka < 0 || shioriko < 0 || filler < 0) return;

    for (int i = 0; i < 3; i++) game.state.p[0].stage[i] = RB_EMPTY_SLOT;
    test_add_to_discard(&game, shioriko);
    test_add_to_discard(&game, filler);
    test_add_to_hand(&game, sayaka);
    test_add_to_hand(&game, live);
    for (int i = 0; i < 5; i++) test_add_to_hand(&game, filler);
    fill_decks(&game, filler);
    test_give_energy(&game, 20);

    expect_card_no(sayaka, SAYAKA);
    expect_card_no(shioriko, SHIORIKO);

    test_play_to_stage(&game, sayaka, 1);
    drain_q159(&game);

    CHECK(test_zone_has_id(&game, 0, "discard", shioriko),
          "Q159: Shioriko stays in the waitroom - a discard debut whose self-cost "
          "is 'this member to wait' cannot be paid from the waitroom");
    CHECK(test_zone_has_id(&game, 0, "stage", sayaka),
          "Q159: Sayaka is on stage after her debut resolved");
}

static void q159_positive_select_prompt_appears(void) {
    TestGame game;
    test_game_new(&game);

    int sayaka = test_id(&game, SAYAKA);
    int kasumi = test_id(&game, KASUMI_NIJI);
    int filler = test_id(&game, "PL!-sd1-002-SD");
    int live = test_id(&game, "PL!-sd1-019-SD");
    CHECK(sayaka >= 0 && kasumi >= 0 && filler >= 0,
          "Q159 positive: fixtures resolve");
    if (sayaka < 0 || kasumi < 0 || filler < 0) return;

    for (int i = 0; i < 3; i++) game.state.p[0].stage[i] = RB_EMPTY_SLOT;
    test_add_to_discard(&game, kasumi);
    test_add_to_hand(&game, sayaka);
    test_add_to_hand(&game, live);
    for (int i = 0; i < 5; i++) test_add_to_hand(&game, filler);
    fill_decks(&game, filler);
    test_give_energy(&game, 22);

    expect_card_no(sayaka, SAYAKA);
    expect_card_no(kasumi, KASUMI_NIJI);

    test_play_to_stage(&game, sayaka, 1);

    CHECK(test_has_pending_choice(&game),
          "Q159 positive: Sayaka's debut presents the waitroom selection");
    int saw_select_card = 0;
    int guard = 0;
    while (test_has_pending_choice(&game) && guard++ < 30) {
        const char *type = test_pending_choice_type(&game);
        if (type && strcmp(type, "SelectCard") == 0) saw_select_card = 1;
        test_resume_choice(&game, 0);
    }
    CHECK(saw_select_card, "Q159 positive: a SelectCard prompt for the waitroom appeared");
    CHECK(test_zone_has_id(&game, 0, "stage", sayaka), "Q159 positive: sayaka on stage");
}

/* ??????????????????????????????????????????????????????????????????
   2. LIVE START - Reina (zone_change_gate_test.rs)
   ?????????????????????????????????????????????????????????????????? */

/* Reina: live-start auto - a member without blade heart left the live-card
   zone this turn -> draw 1 and heart03/05/06 until live end. */
#define REINA "PL!N-pb1-009-R"
#define NO_BLADE_MEMBER "PL!-sd1-001-SD" /* no b_heart icons */
#define BLADE_MEMBER "PL!-sd1-008-SD"    /* has b_heart03 */

static void reina_fires_after_non_blade_member_left_live_zone(void) {
    TestGame game;
    test_game_new(&game);

    int reina = test_id(&game, REINA);
    int hanayo = test_id(&game, NO_BLADE_MEMBER);
    int filler = test_id(&game, FILLER);
    CHECK(reina >= 0 && hanayo >= 0 && filler >= 0, "Reina live-start: fixtures resolve");
    if (reina < 0 || hanayo < 0) return;

    test_add_to_stage(&game, 1, reina);
    test_add_to_live(&game, hanayo);
    fill_decks(&game, filler);
    expect_card_no(reina, REINA);
    expect_card_no(hanayo, NO_BLADE_MEMBER);
    CHECK(printed_blade_heart(hanayo) == 0,
          "Reina live-start precondition: the member that left the live zone "
          "prints NO blade heart");
    expect_trigger(reina, TRIG_LIVE_START, 1);

    CHECK(move_live_to_waitroom_event(&game, 0, hanayo),
          "Reina live-start: the no-blade member was in P1's live zone to move");

    int hand_before = game.state.p[0].hand.n;
    CHECK(fire_trigger(&game, reina, TRIG_LIVE_START),
          "Reina exposes a live-start ability");
    drain_first(&game);

    CHECK(game.state.p[0].hand.n == hand_before + 1,
          "Reina live-start: draws 1 when a blade-heartless member left the live zone");
    CHECK(test_get_heart_modifier(&game, reina, 3) == 1, "Reina live-start: heart03 granted");
    CHECK(test_get_heart_modifier(&game, reina, 5) == 1, "Reina live-start: heart05 granted");
    CHECK(test_get_heart_modifier(&game, reina, 6) == 1, "Reina live-start: heart06 granted");
}

static void reina_silent_when_blade_heart_member_left_live_zone(void) {
    TestGame game;
    test_game_new(&game);

    int reina = test_id(&game, REINA);
    int honoka = test_id(&game, BLADE_MEMBER);
    int filler = test_id(&game, FILLER);
    CHECK(reina >= 0 && honoka >= 0 && filler >= 0, "Reina negative: fixtures resolve");
    if (reina < 0 || honoka < 0) return;

    test_add_to_stage(&game, 1, reina);
    test_add_to_live(&game, honoka);
    fill_decks(&game, filler);
    expect_card_no(reina, REINA);
    expect_card_no(honoka, BLADE_MEMBER);
    CHECK(printed_blade_heart(honoka) == 1,
          "Reina negative precondition: the member that left the live zone "
          "PRINTS a blade heart - that is the property negating the gate");
    CHECK(printed_blade_heart(reina) == 0,
          "Reina negative precondition: REINA prints no blade heart, so this "
          "case really is the twin of the positive one");

    CHECK(move_live_to_waitroom_event(&game, 0, honoka),
          "Reina negative: the blade-heart member was in P1's live zone");

    int hand_before = game.state.p[0].hand.n;
    CHECK(fire_trigger(&game, reina, TRIG_LIVE_START),
          "Reina negative: the ability exists and is fired");
    drain_first(&game);

    CHECK(game.state.p[0].hand.n == hand_before,
          "Reina negative: a blade-heart member leaving the live zone must NOT draw");
    CHECK(test_get_heart_modifier(&game, reina, 3) == 0,
          "Reina negative: no heart03 is granted");
    CHECK(test_get_heart_modifier(&game, reina, 5) == 0,
          "Reina negative: no heart05 is granted");
    CHECK(test_get_heart_modifier(&game, reina, 6) == 0,
          "Reina negative: no heart06 is granted");
}

/* ??????????????????????????????????????????????????????????????????
   3. MOVEMENT - zone-change trigger gates (zone_change_gate_test.rs)
   ?????????????????????????????????????????????????????????????????? */

/* Riko ab#0: an Aqours live card leaving HER live-card zone may be put on the
   top or bottom of the deck. */
#define RIKO "PL!S-bp6-002-R\357\274\213" /* trailing U+FF0B fullwidth plus */
#define AQOURS_LIVE "PL!S-bp2-019-L"       /* WATER BLUE NEW WORLD */
#define MUSE_LIVE "PL!-sd1-019-SD"         /* wrong-group negative */

static void riko_aqours_live_leaving_live_zone_offers_deck_placement(void) {
    TestGame game;
    test_game_new(&game);

    int riko = test_id(&game, RIKO);
    int live = test_id(&game, AQOURS_LIVE);
    int filler = test_id(&game, FILLER);
    CHECK(riko >= 0 && live >= 0 && filler >= 0, "Riko movement: fixtures resolve");
    if (riko < 0 || live < 0) return;

    test_add_to_stage(&game, 1, riko);
    test_add_to_live(&game, live);
    fill_decks(&game, filler);
    expect_card_no(riko, RIKO);
    expect_card_no(live, AQOURS_LIVE);
    expect_trigger(riko, TRIG_AUTO, 1);
    int deck_before = game.state.p[0].deck.n;

    CHECK(move_live_to_waitroom_event(&game, 0, live),
          "Riko: the Aqours live was in P1's live zone to move");

    scan_auto_for_player(&game, 0);

    CHECK(test_has_pending_choice(&game),
          "Riko: the deck-top/bottom placement is offered when an Aqours live leaves");
    drain_first(&game);

    CHECK(test_zone_has_id(&game, 0, "deck", live),
          "Riko: the Aqours live ends up in the deck");
    CHECK(game.state.p[0].deck.n == deck_before + 1,
          "Riko: the deck grew by exactly the placed live card");
    CHECK(!test_zone_has_id(&game, 0, "discard", live),
          "Riko: the live card does not stay in the waitroom");
}

static void riko_ignores_non_aqours_live_leaving_live_zone(void) {
    TestGame game;
    test_game_new(&game);

    int riko = test_id(&game, RIKO);
    int muse_live = test_id(&game, MUSE_LIVE);
    int filler = test_id(&game, FILLER);
    CHECK(riko >= 0 && muse_live >= 0 && filler >= 0, "Riko wrong-group: fixtures resolve");
    if (riko < 0 || muse_live < 0) return;

    test_add_to_stage(&game, 1, riko);
    test_add_to_live(&game, muse_live);
    fill_decks(&game, filler);
    expect_card_no(riko, RIKO);
    expect_card_no(muse_live, MUSE_LIVE);

    CHECK(move_live_to_waitroom_event(&game, 0, muse_live),
          "Riko wrong-group: the mu's live was in P1's live zone");

    scan_auto_for_player(&game, 0);

    CHECK(!test_has_pending_choice(&game),
          "Riko: a non-Aqours live leaving the zone must not arm the ability");
    CHECK(test_zone_has_id(&game, 0, "discard", muse_live),
          "Riko wrong-group: the other-group live stays in the waitroom");
}

static void riko_responds_only_to_own_side_live_zone(void) {
    TestGame game;
    test_game_new(&game);

    int riko_p1 = test_id(&game, RIKO);
    int riko_p2 = test_new_id(&game, RIKO);
    int live_p2 = test_new_id(&game, AQOURS_LIVE);
    int filler = test_id(&game, FILLER);
    CHECK(riko_p1 >= 0 && riko_p2 >= 0 && live_p2 >= 0, "Riko ownership: fixtures resolve");
    if (riko_p1 < 0 || riko_p2 < 0 || live_p2 < 0) return;
    CHECK(riko_p1 != riko_p2 && live_p2 != AQOURS_LIVE && riko_p1 != riko_p2,
          "Riko ownership: P1 and P2 hold DISTINCT card instances");
    expect_card_no(riko_p1, RIKO);
    expect_card_no(riko_p2, RIKO);
    expect_card_no(live_p2, AQOURS_LIVE);

    test_add_to_stage(&game, 1, riko_p1);
    test_set_opp_stage(&game, 1, riko_p2);
    RbPlayer *P2 = &game.state.p[1];
    if (P2->live.n < RB_MAX_ZONE) P2->live.cards[P2->live.n++] = live_p2;
    fill_decks(&game, filler);

    CHECK(move_live_to_waitroom_event(&game, 1, live_p2),
          "Riko ownership: the Aqours live was in P2's live zone");

    /* Scan only P1 - her Riko must stay silent about P2's zone. */
    scan_auto_for_player(&game, 0);
    CHECK(!test_has_pending_choice(&game),
          "Riko: P1's copy must not react to P2's live-zone change");

    /* Scanning P2 arms HIS copy. */
    scan_auto_for_player(&game, 1);
    CHECK(test_has_pending_choice(&game),
          "Riko: P2's own copy reacts to his own live leaving the zone");
    drain_first(&game);
    CHECK(test_zone_has_id(&game, 1, "deck", live_p2),
          "Riko: P2's live card is placed into P2's deck");
}

/* Miyamiya ab#0 (auto): when she is moved stage -> waitroom by a baton touch
   against a cost>=10 blade-heartless Nijigasaki newcomer, activate 2 energy
   (and draw 1 more at cost>=15).
   NOTE: the Rust test spells these two newcomers "PL!N-pb7-003-R+" and
   "PL!N-pb4-007-R+". Those numbers do not exist - the real cards use the `bp`
   set code (bp7 / bp4, not pb7 / pb4). Verified against cards/cards.json:
     PL!N-bp7-003-R+  ?????  cost 15  no blade heart  -> full payoff
     PL!N-bp4-007-R+  ?????  cost 13  no blade heart  -> energy only
     PL!N-bp4-009-R   ?????  cost 13  HAS blade heart -> nothing (negative) */
#define MIYAMIYA "PL!N-bp5-005-R\357\274\213"
#define NEWCOMER_COST15 "PL!N-bp7-003-R\357\274\213"
#define NEWCOMER_COST13 "PL!N-bp4-007-R\357\274\213"
#define NEWCOMER_BLADE_HEART "PL!N-bp4-009-R"

static void miyamiya_baton_touch_case(const char *newcomer_no, const char *label,
                                      int expect_draw) {
    TestGame game;
    test_game_new(&game);

    int ai = test_id(&game, MIYAMIYA);
    int newcomer = test_id(&game, newcomer_no);
    int filler = test_id(&game, FILLER);
    if (ai < 0 || newcomer < 0 || filler < 0) {
        CHECK(0, label);
        return;
    }
    test_add_to_stage(&game, 1, ai);
    fill_decks(&game, filler);
    test_add_to_hand(&game, newcomer);
    test_give_energy(&game, 25);
    /* Leave 5 energy waited so the ability's "activate 2" has cards to flip. */
    test_set_energy_active(&game, 0, 25 - 5);

    expect_card_no(ai, MIYAMIYA);
    expect_card_no(newcomer, newcomer_no);
    CHECK(printed_blade_heart(newcomer) == 0,
          "Miyamiya: the baton-touch newcomer prints no blade heart");
    CHECK(printed_blade_heart(ai) == 0,
          "Miyamiya: MIYAMIYA prints no blade heart, so the gate is the "
          "newcomer's property and not her own");

    int active_before = game.state.p[0].energy_active;
    int hand_before = game.state.p[0].hand.n;
    int net = printed_cost(newcomer) - printed_cost(ai);

    test_play_to_stage(&game, newcomer, 1);
    drain_skip(&game);

    char msg[256];
    snprintf(msg, sizeof(msg),
             "%s: active energy expected %d (before %d, net %d, +2 activated), got %d",
             label, active_before - net + 2, active_before, net,
             game.state.p[0].energy_active);
    CHECK(game.state.p[0].energy_active == active_before - net + 2, msg);

    snprintf(msg, sizeof(msg), "%s: hand expected %d, got %d", label,
             hand_before - 1 + (expect_draw ? 1 : 0), game.state.p[0].hand.n);
    CHECK(game.state.p[0].hand.n == hand_before - 1 + (expect_draw ? 1 : 0), msg);
}

static void miyamiya_baton_touch_cost15_newcomer_full_payoff(void) {
    miyamiya_baton_touch_case(NEWCOMER_COST15,
                              "Miyamiya baton cost-15 newcomer", 1);
}

static void miyamiya_baton_touch_cost13_newcomer_energy_only(void) {
    miyamiya_baton_touch_case(NEWCOMER_COST13,
                              "Miyamiya baton cost-13 newcomer", 0);
}

static void miyamiya_baton_touch_blade_heart_newcomer_nothing(void) {
    /* Negative: the replacing member HAS a blade heart, so the negated
       property fails and nothing is activated or drawn. */
    TestGame game;
    test_game_new(&game);

    int ai = test_id(&game, MIYAMIYA);
    int newcomer = test_id(&game, NEWCOMER_BLADE_HEART);
    int reina_id_for_negative = test_id(&game, REINA);
    int filler = test_id(&game, FILLER);
    CHECK(ai >= 0 && newcomer >= 0 && filler >= 0,
          "Miyamiya blade-heart newcomer: fixtures resolve");
    if (ai < 0 || newcomer < 0) return;

    /* Rust pins BOTH sides of the transposition pair by hand
       (zone_change_gate_test.rs:415-437): PL!N-pb1-009-R and
       PL!N-bp4-009-R are both ?????, differing only by the pb1/bp4
       swap, and the single property separating them is exactly the blade
       heart this negative turns on. A swapped pair INVERTS the test. */
    expect_card_no(newcomer, NEWCOMER_BLADE_HEART);
    expect_card_no(reina_id_for_negative, REINA);
    CHECK(newcomer != reina_id_for_negative,
          "Miyamiya negative: the blade-heart newcomer is a DIFFERENT card id "
          "from Reina (pb4-009 vs pb1-009)");
    CHECK(printed_blade_heart(newcomer) == 1,
          "Miyamiya negative precondition: the newcomer PRINTS a blade heart - "
          "that is what negates the condition");
    CHECK(printed_blade_heart(reina_id_for_negative) == 0,
          "Miyamiya negative precondition: REINA prints no blade heart, or "
          "this is not the twin of the positive case");
    expect_card_no(ai, MIYAMIYA);

    test_add_to_stage(&game, 1, ai);
    fill_decks(&game, filler);
    test_add_to_hand(&game, newcomer);
    test_give_energy(&game, 25);
    test_set_energy_active(&game, 0, 25 - 5);

    int active_before = game.state.p[0].energy_active;
    int hand_before = game.state.p[0].hand.n;
    int net = printed_cost(newcomer) - printed_cost(ai);

    test_play_to_stage(&game, newcomer, 1);
    drain_skip(&game);

    CHECK(game.state.p[0].energy_active == active_before - net,
          "Miyamiya negative: a blade-heart newcomer must not activate any energy");
    CHECK(game.state.p[0].hand.n == hand_before - 1,
          "Miyamiya negative: no draw - only the normal play happened");
}

/* ??????????????????????????????????????????????????????????????????
   4. DEBUT WATCHERS - Q197 / Q198
   ?????????????????????????????????????????????????????????????????? */

/* Ranju (cost 11): auto - when another cost-11 member debuts on my stage,
   place 1 energy from the energy deck in the wait state. */
#define RANJU "PL!N-pb1-012-R"
#define COST11_MEMBER "PL!-sd1-001-SD"
/* Miyashita (cost 2): auto - when a cost-10 member debuts on my stage, draw 1. */
#define MIYASHITA "PL!N-pb1-005-R"
#define COST10_MEMBER "PL!-bp5-005-R"

static void cost11_baton_replacement_does_not_arm_debut_watcher(void) {
    TestGame game;
    test_game_new(&game);

    int ranju = test_id(&game, RANJU);
    int cost11 = test_id(&game, COST11_MEMBER);
    int filler = test_id(&game, FILLER);
    CHECK(ranju >= 0 && cost11 >= 0 && filler >= 0, "Q198: fixtures resolve");
    if (ranju < 0 || cost11 < 0) return;

    test_add_to_hand(&game, ranju);
    test_add_to_hand(&game, cost11);
    test_give_energy(&game, 20);
    for (int i = 0; i < 5; i++) test_add_to_deck_pl(&game, 0, filler);

    expect_card_no(ranju, RANJU);
    expect_card_no(cost11, COST11_MEMBER);

    test_play_to_stage(&game, ranju, 1);
    reset_stage_arrived(&game, 0);

    /* Baton touch: the cost-11 newcomer replaces Ranju in the same area, so
       she leaves stage BEFORE the newcomer appears - the watcher must not arm. */
    int energy_before = energy_zone_count(&game, 0);
    test_play_to_stage(&game, cost11, 1);
    drain_decline(&game);

    CHECK(test_zone_has_id(&game, 0, "discard", ranju),
          "Q198: Ranju is in the waitroom after the baton touch");
    CHECK(energy_zone_count(&game, 0) == energy_before,
          "Q198: the auto does NOT fire - a baton-touched watcher leaves stage "
          "before the newcomer appears");
}

static void cost11_debut_with_watcher_retains_nonempty_energy_zone(void) {
    TestGame game;
    test_game_new(&game);

    int ranju = test_id(&game, RANJU);
    int cost11 = test_id(&game, COST11_MEMBER);
    int filler = test_id(&game, FILLER);
    int energy_card = test_id(&game, "LL-E-001-SD");
    CHECK(ranju >= 0 && cost11 >= 0 && filler >= 0 && energy_card >= 0,
          "Q198 positive: fixtures resolve");
    if (ranju < 0 || cost11 < 0 || energy_card < 0) return;
    expect_card_no(ranju, RANJU);
    expect_card_no(cost11, COST11_MEMBER);
    expect_card_no(energy_card, "LL-E-001-SD");

    test_add_to_hand(&game, ranju);
    test_add_to_hand(&game, cost11);
    test_add_to_energy_deck(&game, 0, energy_card);
    test_give_energy(&game, 25);
    for (int i = 0; i < 5; i++) test_add_to_deck_pl(&game, 0, filler);

    /* Play Ranju to the LEFT area, leaving the centre free for the cost-11
       member (Rust: play_to_stage(ranju, MemberArea::LeftSide)). */
    test_play_to_stage(&game, ranju, 0);
    reset_stage_arrived(&game, 0);

    test_play_to_stage(&game, cost11, 1);
    drain_decline(&game);

    /* Rust asserts the EXACT energy card left the energy deck for the energy
       zone. The rescued C port instead asserted energy_zone_count == 1 while
       the zone held 11 spent-and-remaining energy cards, so the assertion was
       wrong by construction rather than by engine behaviour. */
    CHECK(test_zone_has_id(&game, 0, "energy", energy_card) &&
              !test_zone_has_id(&game, 0, "energy_deck", energy_card),
          "Q198: watcher must move the exact energy from the energy deck to "
          "the energy zone");
    CHECK(test_stage_has(&game, 0, ranju),
          "Q198: Ranju is still on the left area when the newcomer arrives");
}

static void cost10_baton_replacement_does_not_draw_from_departed_watcher(void) {
    TestGame game;
    test_game_new(&game);

    int miya = test_id(&game, MIYASHITA);
    int cost10 = test_id(&game, COST10_MEMBER);
    int filler = test_id(&game, FILLER);
    CHECK(miya >= 0 && cost10 >= 0 && filler >= 0, "Q197: fixtures resolve");
    if (miya < 0 || cost10 < 0) return;

    test_add_to_hand(&game, miya);
    test_add_to_hand(&game, cost10);
    test_give_energy(&game, 15);
    for (int i = 0; i < 5; i++) test_add_to_deck_pl(&game, 0, filler);

    expect_card_no(miya, MIYASHITA);
    expect_card_no(cost10, COST10_MEMBER);

    test_play_to_stage(&game, miya, 1);
    reset_stage_arrived(&game, 0);

    int hand_before = game.state.p[0].hand.n;
    int deck_before = game.state.p[0].deck.n;
    test_play_to_stage(&game, cost10, 1);
    drain_decline(&game);

    /* The play must have happened - otherwise an empty hand would "prove"
       nothing about the watcher. */
    CHECK(test_stage_has(&game, 1, cost10),
          "Q197: the cost-10 member did arrive on stage (baton touch happened)");
    CHECK(test_zone_has_id(&game, 0, "discard", miya),
          "Q197: Miyashita is in the waitroom after the baton touch");
    /* Exact, not "<": the baton-touch consumes the card, so 0 vs 1 is exactly
       the difference between "the auto fired" and "it did not". */
    char msg[160];
    snprintf(msg, sizeof(msg),
             "Q197: hand expected %d (drawn by the departed watcher's auto), got %d",
             hand_before - 1, game.state.p[0].hand.n);
    CHECK(game.state.p[0].hand.n == hand_before - 1, msg);
    snprintf(msg, sizeof(msg),
             "Q197: deck expected %d (no card drawn from the deck), got %d",
             deck_before, game.state.p[0].deck.n);
    CHECK(game.state.p[0].deck.n == deck_before, msg);
}

static void cost10_debut_with_watcher_draws_to_replace_played_card(void) {
    TestGame game;
    test_game_new(&game);

    int miya = test_id(&game, MIYASHITA);
    int cost10 = test_id(&game, COST10_MEMBER);
    int filler = test_id(&game, FILLER);
    CHECK(miya >= 0 && cost10 >= 0 && filler >= 0, "Q197 positive: fixtures resolve");
    if (miya < 0 || cost10 < 0) return;

    test_add_to_hand(&game, miya);
    test_add_to_hand(&game, cost10);
    test_give_energy(&game, 15);
    for (int i = 0; i < 5; i++) test_add_to_deck_pl(&game, 0, filler);

    expect_card_no(miya, MIYASHITA);
    expect_card_no(cost10, COST10_MEMBER);
    CHECK(printed_cost(cost10) == 10,
          "Q197: COST10_MEMBER really prints cost 10 - a cost drift would make "
          "both this and its twin vacuous");

    test_play_to_stage(&game, miya, 0);
    reset_stage_arrived(&game, 0);

    int hand_before = game.state.p[0].hand.n;
    test_play_to_stage(&game, cost10, 1);
    drain_decline(&game);

    char msg[160];
    snprintf(msg, sizeof(msg),
             "Q197 positive: hand expected %d (auto drew 1 to replace the play), got %d",
             hand_before, game.state.p[0].hand.n);
    CHECK(game.state.p[0].hand.n == hand_before,
          "Q197: the watcher is still on stage, so its draw 1 compensates the play");
}

/* Q196 - ????? (PL!N-pb1-003-R, cost 4). The rescued C port's header
   claimed q196..q198 coverage but implemented no Q196 case at all, so two
   Rust tests had no C counterpart. NOTE the identity trap: the Q159 Sayaka
   is PL!N-bp3-003-R, a DIFFERENT print from this PL!N-pb1-003-R. */
#define SHIZUKU "PL!N-pb1-003-R"
#define NIJI_MEMBER "PL!N-sd1-001-SD"

static void hand_activation_empty_stage_resolves_draw_after_discard_choices_q196(void) {
    TestGame game;
    test_game_new(&game);

    int shizuku = test_id(&game, SHIZUKU);
    int filler = test_id(&game, FILLER);
    int sayaka_q159 = test_id(&game, SAYAKA);
    CHECK(shizuku >= 0 && filler >= 0 && sayaka_q159 >= 0,
          "Q196 empty-stage: fixtures resolve");
    if (shizuku < 0 || filler < 0) return;
    expect_card_no(shizuku, SHIZUKU);
    CHECK(shizuku != sayaka_q159,
          "Q196: PL!N-pb1-003-R is a different print from the Q159 PL!N-bp3-003-R");
    CHECK(printed_cost(shizuku) == 4, "Q196: SHIZUKU prints cost 4");

    for (int i = 0; i < 3; i++) game.state.p[0].stage[i] = RB_EMPTY_SLOT;
    test_add_to_hand(&game, shizuku);
    test_add_to_hand(&game, filler);
    test_add_to_hand(&game, filler);
    test_give_energy(&game, 10);
    for (int i = 0; i < 5; i++) test_add_to_deck_pl(&game, 0, filler);

    test_activate_ability(&game, shizuku);

    CHECK(!test_has_pending_choice(&game),
          "Q196 empty-stage: the self-discard cost must resolve without a "
          "card-choice prompt");
    drain_decline(&game);

    char msg[160];
    snprintf(msg, sizeof(msg),
             "Q196 empty-stage: hand expected 3 (3 - self-cost + draw), got %d",
             game.state.p[0].hand.n);
    CHECK(game.state.p[0].hand.n == 3,
          "Q196 empty-stage: hand is 3 after the self-discard cost and draw");
}

static void hand_activation_draw_and_lone_nijigasaki_blade_resolve_q196(void) {
    TestGame game;
    test_game_new(&game);

    int shizuku = test_id(&game, SHIZUKU);
    int niji = test_id(&game, NIJI_MEMBER);
    int filler = test_id(&game, FILLER);
    CHECK(shizuku >= 0 && niji >= 0 && filler >= 0,
          "Q196 blade: fixtures resolve");
    if (shizuku < 0 || niji < 0) return;
    expect_card_no(shizuku, SHIZUKU);
    expect_card_no(niji, NIJI_MEMBER);

    test_add_to_stage(&game, 0, niji);
    test_add_to_hand(&game, shizuku);
    test_add_to_hand(&game, filler);
    test_add_to_hand(&game, filler);
    test_give_energy(&game, 15);
    for (int i = 0; i < 5; i++) test_add_to_deck_pl(&game, 0, filler);

    test_activate_ability(&game, shizuku);

    CHECK(!test_has_pending_choice(&game),
          "Q196 blade: the self-discard cost must resolve without a card-choice prompt");
    CHECK(!test_has_pending_choice(&game),
          "Q196 blade: the lone blade-target candidate must auto-resolve "
          "without prompting");
    drain_decline(&game);

    char msg[160];
    snprintf(msg, sizeof(msg), "Q196 blade: hand expected 3, got %d",
             game.state.p[0].hand.n);
    CHECK(game.state.p[0].hand.n == 3, msg);
    snprintf(msg, sizeof(msg), "Q196 blade: blade modifier on the Nijigasaki member expected > 0, got %d",
             test_get_blade_modifier(&game, niji));
    CHECK(test_get_blade_modifier(&game, niji) > 0,
          "Q196: the on-stage Nijigasaki member gains blade from the activation");
}

/* ??????????????????????????????????????????????????????????????????
   5. ONCE LIMITS - per-turn activation limits
   (turn_limited_activation_offered_once_test.rs)
   ?????????????????????????????????????????????????????????????????? */

/* Kasumi: activation limited to once per turn - cost 2 energy + discard 1. */
#define KASUMI "PL!N-bp5-014-N"

static void kasumi_turn1_ability_used_only_once_per_turn(void) {
    TestGame game;
    test_game_new(&game);

    int kasumi = test_id(&game, KASUMI);
    int filler = test_id(&game, FILLER);
    int niji_live = test_id(&game, "PL!SP-SD2-025-SD2");
    CHECK(kasumi >= 0 && filler >= 0 && niji_live >= 0,
          "Kasumi once-limit: fixtures resolve");
    if (kasumi < 0 || niji_live < 0) return;

    test_add_to_stage(&game, 1, kasumi);
    test_give_energy(&game, 4);
    test_add_to_hand(&game, filler);
    test_add_to_discard(&game, niji_live);
    expect_card_no(kasumi, KASUMI);
    expect_card_no(niji_live, "PL!SP-SD2-025-SD2");
    expect_trigger(kasumi, TRIG_ACTIVATION, 1);

    CHECK(rb_ability_uses_used(&game.state, kasumi, 0) == 0,
          "Kasumi: the once-per-turn budget is untouched on a fresh turn");
    CHECK(rb_ability_has_remaining_uses(&game.state, kasumi, 0),
          "Kasumi: the activation is available on a fresh turn");

    /* Rust use_ability_resolving_choices answers every prompt with
       select_generated(0) - the FIRST option. The rescued C port used the
       "decline everything" drain, which refuses Kasumi's discard-1 cost, so
       the activation never resolved and no use was ever recorded. */
    int activated = test_activate_ability(&game, kasumi);
    drain_first(&game);

    char msg[192];
    snprintf(msg, sizeof(msg),
             "Kasumi: the activation must be accepted on a fresh turn (returned %d)",
             activated);
    CHECK(activated != 0, msg);

    snprintf(msg, sizeof(msg), "Kasumi: uses_used expected 1 after a successful activation, got %d",
             rb_ability_uses_used(&game.state, kasumi, 0));
    CHECK(rb_ability_uses_used(&game.state, kasumi, 0) == 1,
          "Kasumi: the use is recorded after a successful activation");
    CHECK(!rb_ability_has_remaining_uses(&game.state, kasumi, 0),
          "Kasumi: the once-per-turn budget is now consumed");

    /* THE REGRESSION: a second press in the same turn must be refused. */
    int discard_before = game.state.p[0].discard.n;
    int hand_before = game.state.p[0].hand.n;
    int second = test_activate_ability(&game, kasumi);
    drain_first(&game);
    CHECK(second == 0 || rb_ability_uses_used(&game.state, kasumi, 0) == 1,
          "Kasumi: a second activation in the same turn does not resolve");
    snprintf(msg, sizeof(msg),
             "Kasumi: uses_used must stay at 1, got %d",
             rb_ability_uses_used(&game.state, kasumi, 0));
    CHECK(rb_ability_uses_used(&game.state, kasumi, 0) == 1,
          "Kasumi: the use count does not grow past the once-per-turn limit");
    snprintf(msg, sizeof(msg),
             "Kasumi: the refused second activation must cost nothing (discard %d->%d, hand %d->%d)",
             discard_before, game.state.p[0].discard.n,
             hand_before, game.state.p[0].hand.n);
    CHECK(game.state.p[0].discard.n == discard_before &&
          game.state.p[0].hand.n == hand_before,
          "Kasumi: the refused second activation costs nothing");
}

static void kasumi_turn1_ability_available_again_next_turn(void) {
    TestGame game;
    test_game_new(&game);

    int kasumi = test_id(&game, KASUMI);
    int filler = test_id(&game, FILLER);
    int niji_live = test_id(&game, "PL!SP-SD2-025-SD2");
    if (kasumi < 0 || niji_live < 0) {
        CHECK(0, "Kasumi turn reset: fixtures resolve");
        return;
    }

    test_add_to_stage(&game, 1, kasumi);
    test_give_energy(&game, 4);
    test_add_to_hand(&game, filler);
    test_add_to_discard(&game, niji_live);
    expect_card_no(kasumi, KASUMI);

    test_activate_ability(&game, kasumi);
    drain_first(&game);
    CHECK(rb_ability_uses_used(&game.state, kasumi, 0) == 1,
          "Kasumi turn reset: the first turn's use is recorded");

    /* Next turn: the limit resets. Refund the spent cost so the check
       reflects the reset limit, not leftover affordability. */
    game.state.turn++;
    test_give_energy(&game, 2);
    test_add_to_hand(&game, filler);

    CHECK(rb_ability_uses_used(&game.state, kasumi, 0) == 0,
          "Kasumi turn reset: the once-per-turn budget resets on the next turn");
    CHECK(rb_ability_has_remaining_uses(&game.state, kasumi, 0),
          "Kasumi turn reset: the activation is available again");
}

static void kasumi_limit_is_per_instance(void) {
    TestGame game;
    test_game_new(&game);

    int k1 = test_new_id(&game, KASUMI);
    int k2 = test_new_id(&game, KASUMI);
    int filler = test_id(&game, FILLER);
    int niji1 = test_id(&game, "PL!SP-SD2-025-SD2");
    int niji2 = test_id(&game, "PL!SP-SD2-023-SD2");
    CHECK(k1 >= 0 && k2 >= 0 && k1 != k2, "Kasumi per-instance: two distinct copies");
    if (k1 < 0 || k2 < 0 || niji1 < 0 || niji2 < 0) return;
    expect_card_no(k1, KASUMI);
    expect_card_no(k2, KASUMI);

    test_add_to_stage(&game, 0, k1);
    test_add_to_stage(&game, 1, k2);
    test_give_energy(&game, 8);
    for (int i = 0; i < 4; i++) test_add_to_hand(&game, filler);
    test_add_to_discard(&game, niji1);
    test_add_to_discard(&game, niji2);

    CHECK(rb_ability_has_remaining_uses(&game.state, k1, 0) &&
          rb_ability_has_remaining_uses(&game.state, k2, 0),
          "Kasumi per-instance: each copy starts with its own once-per-turn budget");

    test_activate_ability(&game, k1);
    drain_first(&game);

    char msg[192];
    snprintf(msg, sizeof(msg), "Kasumi per-instance: k1 uses_used expected 1, got %d",
             rb_ability_uses_used(&game.state, k1, 0));
    CHECK(rb_ability_uses_used(&game.state, k1, 0) == 1,
          "Kasumi per-instance: k1's use is recorded");
    snprintf(msg, sizeof(msg), "Kasumi per-instance: k2 uses_used expected 0, got %d",
             rb_ability_uses_used(&game.state, k2, 0));
    CHECK(rb_ability_uses_used(&game.state, k2, 0) == 0,
          "Kasumi per-instance: k1's use does not consume k2's budget");
    CHECK(rb_ability_has_remaining_uses(&game.state, k2, 0),
          "Kasumi per-instance: k2 is still available after k1 used its own activation");
}

/* „Ÿ„Ÿ runner „Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ */

int main(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    dump_decoder_health();

    /* 1. debut */
    q159_debut_from_discard_self_cost_wait_not_payable();
    q159_positive_select_prompt_appears();

    /* 2. live start */
    reina_fires_after_non_blade_member_left_live_zone();
    reina_silent_when_blade_heart_member_left_live_zone();

    /* 3. movement */
    riko_aqours_live_leaving_live_zone_offers_deck_placement();
    riko_ignores_non_aqours_live_leaving_live_zone();
    riko_responds_only_to_own_side_live_zone();
    miyamiya_baton_touch_cost15_newcomer_full_payoff();
    miyamiya_baton_touch_cost13_newcomer_energy_only();
    miyamiya_baton_touch_blade_heart_newcomer_nothing();

    /* 4. debut watchers */
    cost11_baton_replacement_does_not_arm_debut_watcher();
    cost11_debut_with_watcher_retains_nonempty_energy_zone();
    cost10_baton_replacement_does_not_draw_from_departed_watcher();
    cost10_debut_with_watcher_draws_to_replace_played_card();

    /* 4b. hand activation */
    hand_activation_empty_stage_resolves_draw_after_discard_choices_q196();
    hand_activation_draw_and_lone_nijigasaki_blade_resolve_q196();

    /* 5. once limits */
    kasumi_turn1_ability_used_only_once_per_turn();
    kasumi_turn1_ability_available_again_next_turn();
    kasumi_limit_is_per_instance();

    rb_unload();
    printf("\n%d checks, %d failures\n", checks, failures);
    if (failures) {
        fprintf(stderr, "TRIGGER PATH CHECKS FAILED\n");
        return 1;
    }
    printf("ALL TRIGGER PATH CHECKS PASSED\n");
    return 0;
}
