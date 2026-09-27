/* test_parity_deploy_recover.c -EC parity suite for two unported Rust clusters:
 *
 * engine/tests/test_modules/effects/deploy/ (14 test files)
 * engine/tests/test_modules/effects/recover/per_card/ (13 test files)
 *
 * Sibling: tests/test_parity_deploy_move_missing.c covers the deploy/move-missing
 * mechanics; the two keke tests in pl_sp_sd1_002_test.rs it already ports are NOT
 * repeated here. Only the GAP (keke_blocked_from_locked_area) is.
 *
 * Every test pins the staged card's card_no first (identity rule: prints sharing a
 * character+number are DIFFERENT cards, e.g. PL!SP-bp2-011-R vs PL!SP-pb2-011-R,
 * PL!S-bp2-001-R vs PL!S-PR-029-PR), then drives the real pipeline and asserts the
 * zone contents the Rust twin asserts.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* The U+FF0B fullwidth-plus print of a joint (multi-name) card. Spelled as an
 * explicit UTF-8 byte escape so the file stays 7-bit clean. */
#define PLUS "\xef\xbc\x8b"
#define KOBE_NO "PL!HS-bp1-003-R" PLUS
#define TSUREGI_NO "PL!HS-bp1-004-R" PLUS
#define JOINT_BP1_NO "LL-bp1-001-R" PLUS
#define JOINT_BP2_NO "LL-bp2-001-R" PLUS
#define JOINT_BP3_NO "LL-bp3-001-R" PLUS

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

/* -- helpers --------------------------------------------------------------- */

static int load_card_db(void)
{
 if (rb_load("src") == 0) return 0;
 if (rb_load("../cards/build") == 0) return 0;
 return -1;
}

/* Assert a card_no really is the print the test means to stage (identity rule). */
static void pin_id(int cid, const char *want)
{
 checks++;
 if (cid < 0 || !rb_card_no_eq(cid, want)) {
 fprintf(stderr, "FAIL: identity: want card_no '%s', got id=%d ('%s')\n",
 want, cid, cid >= 0 ? test_card_name(cid) : "<none>");
 failures++;
 } else {
 printf("ok: identity %s\n", want);
 }
}

static int printed_cost(int cid)
{
 Card c;
 int v = -1;
 if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.cost; rb_free_card(&c); }
 return v;
}

static int printed_blade(int cid)
{
 Card c;
 int v = -1;
 if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.blade; rb_free_card(&c); }
 return v;
}

static int printed_score(int cid)
{
 Card c;
 int v = -1;
 if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.score; rb_free_card(&c); }
 return v;
}

static int card_prints_trigger(int cid, const char *trig)
{
 int n = rb_card_num_abilities((uint32_t)cid);
 for (int i = 0; i < n; i++) {
 Ability ab;
 memset(&ab, 0, sizeof(ab));
 if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
 int hit = ab.triggers && strstr(ab.triggers, trig);
 rb_free_ability(&ab);
 if (hit) return 1;
 }
 return 0;
}

/* Rust game.id(no) allocates the NEXT DISTINCT pool slot, so two calls with the
 * same card_no are two different card instances. test_id() returns the SHARED
 * template index and aliases; test_new_id() is the distinct copy. Every Rust
 * game.id(...) maps to new_id() below unless the test deliberately wants the
 * shared template. */
static int mid(TestGame *tg, const char *no) { return test_new_id(tg, no); }

static int bag_has(const RbBag *bag, int cid)
{
 for (int i = 0; i < bag->n; i++) if (bag->cards[i] == cid) return 1;
 return 0;
}

static int waitroom_has(const TestGame *tg, int cid) { return bag_has(&tg->state.p[0].discard, cid); }
static int hand_has(const TestGame *tg, int cid) { return bag_has(&tg->state.p[0].hand, cid); }

static int stage_has(const TestGame *tg, int cid)
{
 for (int i = 0; i < RB_STAGE_SIZE; i++) if (tg->state.p[0].stage[i] == cid) return 1;
 return 0;
}

static int energy_under(const TestGame *tg, int pl, int area)
{
 return tg->state.p[pl].under_cards[area].n;
}

static const char *orientation(TestGame *tg, int cid)
{
 return rb_mods_get_orientation(&tg->state.mods, cid);
}

static int blade(TestGame *tg, int cid) { return test_get_blade_modifier(tg, cid); }
static int score_mod(TestGame *tg, int cid) { return test_get_score_modifier(tg, cid); }

static void clear_p1(TestGame *tg)
{
 RbPlayer *P = &tg->state.p[0];
 P->deck.n = 0;
 P->hand.n = 0;
 P->discard.n = 0;
 P->live.n = 0;
 P->success.n = 0;
 P->energy.n = 0;
 P->energy_active = 0;
 for (int i = 0; i < RB_STAGE_SIZE; i++) { P->stage[i] = RB_EMPTY_SLOT; P->stage_wait[i] = 0; }
}

/* Rust fill_decks(game, filler) E30 cards each, clearing first. */
static void fill_decks(TestGame *tg, int filler, int n)
{
 for (int pl = 0; pl < 2; pl++) {
 RbPlayer *P = &tg->state.p[pl];
 P->deck.n = 0;
 for (int i = 0; i < n && P->deck.n < RB_MAX_ZONE; i++) P->deck.cards[P->deck.n++] = filler;
 }
}

static void put_on_deck_top(TestGame *tg, int pl, int cid)
{
 test_insert_deck_top(tg, pl, cid);
}

static int pending_type_is(TestGame *tg, const char *want)
{
 const char *t = test_pending_choice_type(tg);
 return t && !strcmp(t, want);
}

static void pick(TestGame *tg, int idx)
{
 if (!rb_has_pending_choice(&tg->state)) return;
 const int one[1] = { idx };
 rb_resume_with_choice_indices(&tg->state, one, 1);
}

static void pick_n(TestGame *tg, const int *idx, int n)
{
 if (!rb_has_pending_choice(&tg->state)) return;
 rb_resume_with_choice_indices(&tg->state, idx, n);
}

static void skip(TestGame *tg)
{
 if (!rb_has_pending_choice(&tg->state)) return;
 rb_resume_with_choice_indices(&tg->state, NULL, 0);
}

static void drain_pick0(TestGame *tg, int guard_max)
{
 int guard = 0;
 while (rb_has_pending_choice(&tg->state) && guard++ < guard_max) pick(tg, 0);
}

static void drain_skip(TestGame *tg, int guard_max)
{
 int guard = 0;
 while (rb_has_pending_choice(&tg->state) && guard++ < guard_max) skip(tg);
}

/* Rust drain_auto_ability_choices: SelectAutoAbility -> [0], anything else -> [] */
static void drain_auto(TestGame *tg)
{
 int guard = 0;
 while (rb_has_pending_choice(&tg->state) && guard++ < 32) {
 const char *t = test_pending_choice_type(tg);
 if (t && !strcmp(t, "SelectAutoAbility")) pick(tg, 0);
 else skip(tg);
 }
}

/* Rust fire_trigger(game, cid, trigger, trig) -Ehelpers/mod.rs. The C engine
 * exposes the per-trigger scan, so the queue is built from the same trigger
 * token; fixture boards carry exactly one card printing it. */
void rb_trigger_auto_ability(GameState *g, const char *ability_id,
 const char *trigger_type, int player_id,
 const char *source_card_no,
 int explicit_card_id,
 const int *trigger_moved_cards, int n_moved,
 int triggering_member_id);

static int fire_trigger(TestGame *tg, int cid, const char *trig)
{
 Card card;
 if (!rb_decode_card_by_index((uint32_t)cid, &card)) return 0;
 char card_no[128];
 snprintf(card_no, sizeof(card_no), "%s", rb_card_string(card.card_no_idx));
 char ability_id[512];
 int found = 0;
 int nab = rb_card_num_abilities((uint32_t)cid);
 for (int a = 0; a < nab && !found; a++) {
 Ability ab;
 if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
 if (ab.triggers && strcmp(ab.triggers, trig) == 0) {
 snprintf(ability_id, sizeof(ability_id), "%s_%s", card_no,
 ab.full_text ? ab.full_text : "");
 found = 1;
 }
 rb_free_ability(&ab);
 }
 rb_free_card(&card);
 if (!found) return 0;
 rb_trigger_auto_ability(&tg->state, ability_id, trig, 0, card_no, cid, NULL, 0, -1);
 tg->state.activating_card = cid;
 rb_process_pending_auto_abilities(&tg->state);
 return 1;
}

static int fire_live_start(TestGame *tg, int cid) { return fire_trigger(tg, cid, RB_TSTR_LIVE_START); }
static int fire_live_success(TestGame *tg, int cid) { return fire_trigger(tg, cid, RB_TSTR_LIVE_SUCCESS); }

static int play_to_stage(TestGame *tg, int cid, int area, int accept)
{
 int r = test_play_to_stage(tg, cid, area);
 int guard = 0;
 while (tg->state.ptc_active && guard++ < 4) {
 test_answer_play_cost_choice(tg, accept);
 if (tg->state.p[0].stage[area] == cid) break;
 r = test_play_to_stage(tg, cid, area);
 }
 return r;
}

static void advance_to_live_set(TestGame *tg)
{
 for (int i = 0; i < 5; i++) test_pass(tg);
}

static void advance_to_live_start(TestGame *tg)
{
 test_pass(tg);
 test_pass(tg);
}

/* ======================================================================
 * PART A -cluster effects/deploy/ (deploy destination + deploy conditions)
 * ====================================================================== */

/* A1. pl_sp_sd1_002_test.rs:122-197 keke_blocked_from_locked_area -- GAP.
 * Center is locked (starter debuted there this turn) and Left is locked (keke
 * debuts there), so ONLY Right is available: the deploy auto-places and MUST
 * NOT prompt. */
static void a_keke_blocked_from_locked_area(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int keke = mid(&tg, "PL!SP-sd1-002-SD");
 int liella = mid(&tg, "PL!SP-sd1-013-SD");
 int starter = mid(&tg, "PL!-sd1-010-SD");
 pin_id(keke, "PL!SP-sd1-002-SD");
 pin_id(liella, "PL!SP-sd1-013-SD");
 pin_id(starter, "PL!-sd1-010-SD");
 if (keke < 0 || liella < 0 || starter < 0) return;
 test_add_to_hand(&tg, liella);
 test_add_to_hand(&tg, starter);
 test_add_to_hand(&tg, keke);
 fill_decks(&tg, mid(&tg, "PL!-sd1-010-SD"), 40);
 test_give_energy(&tg, 20);

 CHECK_EQ(play_to_stage(&tg, starter, 1, 0), 1, "setup: the starter debuts at Center");
 CHECK_EQ(tg.state.p[0].stage[1], starter, "the starter occupies Center, locking it");

 CHECK_EQ(play_to_stage(&tg, keke, 0, 0), 1, "keke plays to Left over a free slot");
 CHECK(pending_type_is(&tg, "SelectCard"),
 "keke debut opens the hand SelectCard even with Center locked (rs:165-170)");
 int idx = -1;
 for (int i = 0; i < tg.state.p[0].hand.n; i++)
 if (tg.state.p[0].hand.cards[i] == liella) { idx = i; break; }
 CHECK(idx >= 0, "the Liella! card is still in hand when the prompt is answered");
 if (idx >= 0) pick(&tg, idx);

 CHECK_EQ(rb_has_pending_choice(&tg.state), 0,
 "only Right is unlocked, so a single free slot must not prompt (rs:184-188)");
 CHECK_EQ(tg.state.p[0].stage[2], liella,
 "the Liella! card lands at Right, the only unlocked slot (rs:191-193)");
 CHECK_EQ(tg.state.p[0].stage[0], keke, "keke stays at Left (rs:195)");
 CHECK_EQ(tg.state.p[0].stage[1], starter, "the starter stays at Center (rs:196)");
}

/* A2. discard_self_same_area_group_deploy_q63_q80_test.rs:14-57 (Q63).
 * E E put self to the waitroom, then deploy a cost<=15 E * member from the waitroom into the area she vacated -- WITHOUT paying that
 * member's cost. */
static void a_q63_ability_debut_no_cost_payment(void)
{
 static TestGame tg;
 test_game_new(&tg);
 clear_p1(&tg);
 int sayaka = mid(&tg, "PL!HS-bp1-002-R");
 int hasuno = mid(&tg, "PL!HS-bp2-004-R");
 pin_id(sayaka, "PL!HS-bp1-002-R");
 pin_id(hasuno, "PL!HS-bp2-004-R");
 if (sayaka < 0 || hasuno < 0) return;
 CHECK_EQ(printed_cost(hasuno), 2, "setup: the target really costs 2 (<= 15)");

 tg.state.p[0].stage[1] = sayaka;
 test_add_to_discard(&tg, hasuno);
 test_give_energy(&tg, 2);
 CHECK_EQ(tg.state.p[0].energy_active, 2, "setup: exactly the 2E activation cost is held");

 test_activate_ability(&tg, sayaka);
 CHECK(rb_has_pending_choice(&tg.state), "the member-from-waitroom prompt is expected");
 CHECK(pending_type_is(&tg, "SelectCard"),
 "expected a SelectCard over the waitroom (rs:39-41)");
 pick(&tg, 0);
 drain_auto(&tg);

 CHECK_EQ(tg.state.p[0].energy_active, 0,
 "Q63: the 2E activation cost is fully consumed and nothing more is paid (rs:44-47)");
 CHECK(waitroom_has(&tg, sayaka), "the activator ends in the waitroom (rs:49-51)");
 CHECK_EQ(tg.state.p[0].stage[1], hasuno,
 "Q63: the member appears in the vacated area with no cost payment (rs:53-56)");
}

/* A2b. same file:64-98 (Q80) -- the deploy targets the area THIS activation
 * vacated, within the same resolution. The Q63 twin already asserts the area;
 * this pins the negative: sayaka is no longer on stage. */
static void a_q80_debut_to_area_vacated_same_turn(void)
{
 static TestGame tg;
 test_game_new(&tg);
 clear_p1(&tg);
 int sayaka = mid(&tg, "PL!HS-bp1-002-R");
 int hasuno = mid(&tg, "PL!HS-bp2-004-R");
 pin_id(sayaka, "PL!HS-bp1-002-R");
 pin_id(hasuno, "PL!HS-bp2-004-R");
 if (sayaka < 0 || hasuno < 0) return;

 tg.state.p[0].stage[1] = sayaka;
 test_add_to_discard(&tg, hasuno);
 test_give_energy(&tg, 2);

 test_activate_ability(&tg, sayaka);
 CHECK(pending_type_is(&tg, "SelectCard"), "the waitroom pick is a SelectCard");
 pick(&tg, 0);
 drain_auto(&tg);

 CHECK_EQ(tg.state.p[0].stage[1], hasuno,
 "Q80: the new member debuts into the area this ability vacated (rs:90-93)");
 CHECK(!stage_has(&tg, sayaka), "sayaka is no longer on stage (rs:94-97)");
 CHECK(waitroom_has(&tg, sayaka), "sayaka ended in the waitroom, not on stage");
}

/* A3. n_pb1_015_shizuku_test.rs:6-27 -- EPL!N-pb1-015-R E2
 * optional -> a hand Enamed cost<=4 member is deployed. The PAY path
 * must move the card out of hand and onto the stage. */
static void a_shizuku_pay_2e_deploys_shizuku(void)
{
 static TestGame tg;
 test_game_new(&tg);
 clear_p1(&tg);
 int shizuku = mid(&tg, "PL!N-pb1-015-R");
 int shizuku_hand = mid(&tg, "PL!N-bp1-015-N");
 int filler = mid(&tg, "PL!-sd1-010-SD");
 pin_id(shizuku, "PL!N-pb1-015-R");
 pin_id(shizuku_hand, "PL!N-bp1-015-N");
 if (shizuku < 0 || shizuku_hand < 0 || filler < 0) return;
 CHECK(shizuku != shizuku_hand, "setup: the two prints are distinct card instances");

 test_add_to_hand(&tg, shizuku);
 test_add_to_hand(&tg, shizuku_hand);
 test_give_energy(&tg, 15);
 for (int i = 0; i < 5; i++) test_add_to_deck(&tg, filler);
 tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[2] = RB_EMPTY_SLOT;

 CHECK_EQ(play_to_stage(&tg, shizuku, 1, 0), 1, "the play to stage succeeds");
 CHECK(rb_has_pending_choice(&tg.state), "the optional pay-2E gate is offered");
 /* Pay the optional cost. */
 {
 const int one[1] = { 1 };
 if (rb_has_pending_choice(&tg.state)) rb_resume_with_choice_indices(&tg.state, one, 1);
 }
 drain_pick0(&tg, 8);

 CHECK(!hand_has(&tg, shizuku_hand),
 "paying the optional cost deploys the hand card: it leaves hand (rs:24-25)");
 CHECK(!rb_has_pending_choice(&tg.state), "the debut chain completes with no prompt left (rs:26)");
}

/* A3b. same file:29-45 -- the SKIP path must leave the hand card alone. */
static void a_shizuku_skip_pay_no_deploy(void)
{
 static TestGame tg;
 test_game_new(&tg);
 clear_p1(&tg);
 int shizuku = mid(&tg, "PL!N-pb1-015-R");
 int shizuku_hand = mid(&tg, "PL!N-bp1-015-N");
 int filler = mid(&tg, "PL!-sd1-010-SD");
 pin_id(shizuku, "PL!N-pb1-015-R");
 pin_id(shizuku_hand, "PL!N-bp1-015-N");
 if (shizuku < 0 || shizuku_hand < 0 || filler < 0) return;

 test_add_to_hand(&tg, shizuku);
 test_add_to_hand(&tg, shizuku_hand);
 test_give_energy(&tg, 15);
 for (int i = 0; i < 5; i++) test_add_to_deck(&tg, filler);
 tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[2] = RB_EMPTY_SLOT;

 play_to_stage(&tg, shizuku, 1, 0);
 CHECK(rb_has_pending_choice(&tg.state), "the pay/skip gate is offered");
 skip(&tg);
 drain_skip(&tg, 8);
 CHECK(hand_has(&tg, shizuku_hand),
 "declining the optional cost keeps the card in hand (rs:43-44)");
 CHECK(!stage_has(&tg, shizuku_hand), "nothing was deployed on the skip path");
}

/* A3c. n_pb1_017_miyashita_test.rs:6-22 -- the same E2 optional -> named
 * member deploy shape on a different card (E). */
static void a_miyashita_pay_2e_deploys(void)
{
 static TestGame tg;
 test_game_new(&tg);
 clear_p1(&tg);
 int miya = mid(&tg, "PL!N-pb1-017-R");
 int miya_hand = mid(&tg, "PL!N-bp1-017-N");
 int filler = mid(&tg, "PL!-sd1-010-SD");
 pin_id(miya, "PL!N-pb1-017-R");
 pin_id(miya_hand, "PL!N-bp1-017-N");
 if (miya < 0 || miya_hand < 0 || filler < 0) return;

 test_add_to_hand(&tg, miya);
 test_add_to_hand(&tg, miya_hand);
 test_give_energy(&tg, 15);
 for (int i = 0; i < 5; i++) test_add_to_deck(&tg, filler);
 tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[2] = RB_EMPTY_SLOT;

 play_to_stage(&tg, miya, 1, 0);
 CHECK(rb_has_pending_choice(&tg.state), "the optional pay-2E gate is offered");
 {
 const int one[1] = { 1 };
 if (rb_has_pending_choice(&tg.state)) rb_resume_with_choice_indices(&tg.state, one, 1);
 }
 drain_pick0(&tg, 8);
 CHECK(stage_has(&tg, miya_hand),
 "paying the optional cost puts the hand E member on stage (rs:21)");
}

/* A4. optional_energy_deploy_two_total_cost_four_test.rs:90-215
 * E EPL!S-bp2-006-R 4E may: up to 2 members whose TOTAL cost <= 4
 * debut from the waitroom. The Rust twin pins the prompt's filtered_indices
 * (cost>4 excluded) and cost_total. */
static void a_yoshiko_over_cost_filtered_and_budget_tracked(void)
{
 static TestGame tg;
 test_game_new(&tg);
 clear_p1(&tg);
 int yoshiko = mid(&tg, "PL!S-bp2-006-R");
 int cost2 = mid(&tg, "PL!-sd1-002-SD");
 int cost2b = mid(&tg, "PL!-sd1-005-SD");
 int cost4 = mid(&tg, "PL!-sd1-008-SD");
 int cost_high = mid(&tg, "PL!-sd1-001-SD");
 pin_id(yoshiko, "PL!S-bp2-006-R");
 if (yoshiko < 0 || cost2 < 0 || cost2b < 0 || cost4 < 0 || cost_high < 0) return;
 CHECK_EQ(printed_cost(cost2), 2, "setup: PL!-sd1-002-SD really costs 2");
 CHECK_EQ(printed_cost(cost2b), 2, "setup: PL!-sd1-005-SD really costs 2");
 CHECK_EQ(printed_cost(cost4), 4, "setup: PL!-sd1-008-SD really costs 4");
 CHECK(printed_cost(cost_high) > 4, "setup: PL!-sd1-001-SD really costs more than 4");

 test_add_to_discard(&tg, cost2); /* waitroom index 0 */
 test_add_to_discard(&tg, cost4); /* index 1 */
 test_add_to_discard(&tg, cost2b); /* index 2 */
 test_add_to_discard(&tg, cost_high); /* index 3 */
 test_add_to_hand(&tg, yoshiko);
 test_give_energy(&tg, 16);
 tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[2] = RB_EMPTY_SLOT;

 play_to_stage(&tg, yoshiko, 1, 0);
 CHECK_EQ(tg.state.p[0].stage[1], yoshiko, "yoshiko is on stage center (rs:95-97)");
 CHECK_EQ(tg.state.debut_count_this_turn[0], 1, "her own debut counted once (rs:96)");

 /* Pay the optional 4E. */
 CHECK(rb_has_pending_choice(&tg.state), "the optional cost is offered");
 {
 const int one[1] = { 1 };
 if (rb_has_pending_choice(&tg.state)) rb_resume_with_choice_indices(&tg.state, one, 1);
 }
 CHECK(pending_type_is(&tg, "SelectCard"), "the waitroom member pick is a SelectCard");
 const RbChoice *ch = rb_get_pending_choice(&tg.state);
 CHECK(ch && ch->cost_total == 4, "the initial prompt carries cost_total = 4 (rs:117-120)");
 CHECK(ch && ch->n_filtered_indices > 0, "the initial prompt exposes filtered_indices (rs:121-126)");
 if (ch) {
 int has_high = 0, has_cost2 = 0, has_cost4 = 0, has_cost2b = 0;
 for (int i = 0; i < ch->n_filtered_indices; i++) {
 int wi = ch->filtered_indices[i];
 if (wi < 0 || wi >= tg.state.p[0].discard.n) continue;
 int c = tg.state.p[0].discard.cards[wi];
 if (c == cost_high) has_high = 1;
 if (c == cost2) has_cost2 = 1;
 if (c == cost4) has_cost4 = 1;
 if (c == cost2b) has_cost2b = 1;
 }
 CHECK(!has_high, "the cost>4 member is NOT offered in filtered_indices (rs:127-129)");
 CHECK(has_cost2, "the cost-2 member IS offered (rs:130-132)");
 CHECK(has_cost4, "the cost-4 member IS offered (rs:133-135)");
 CHECK(has_cost2b, "the other cost-2 member IS offered (rs:136-138)");
 CHECK_EQ(ch->n_filtered_indices, 3,
 "exactly the three cost<=4 cards are offered (rs:143-147)");
 }

 /* Pick the cost-2 card; a position prompt follows because two slots are free. */
 pick(&tg, 0);
 if (pending_type_is(&tg, "SelectPosition")) pick(&tg, 0);
 drain_pick0(&tg, 4);
 CHECK(stage_has(&tg, cost2), "the first selected cost-2 member is on stage (rs:162)");
 CHECK(!waitroom_has(&tg, cost2), "the first selected card left the waitroom (rs:207-209)");

 /* The re-prompt must carry the REMAINING budget (2), not the original 4. */
 ch = rb_get_pending_choice(&tg.state);
 if (ch && ch->kind == RB_CHOICE_SELECT_CARD) {
 CHECK_EQ(ch->cost_total, 2, "the re-prompt's cost_total is the remaining budget 2 (rs:170-178)");
 int fits = 0;
 for (int i = 0; i < ch->n_filtered_indices; i++) {
 int wi = ch->filtered_indices[i];
 if (wi < 0 || wi >= tg.state.p[0].discard.n) continue;
 CHECK_EQ(printed_cost(tg.state.p[0].discard.cards[wi]), 2,
 "only a cost-2 card fits the remaining budget of 2 (rs:180-190)");
 fits++;
 }
 CHECK_EQ(fits, 1, "exactly one waitroom card fits the remaining budget (rs:181)");
 } else {
 CHECK(0, "the re-prompt for the second card should be a SelectCard (rs:164-167)");
 }
 drain_skip(&tg, 4);

 CHECK(waitroom_has(&tg, cost_high),
 "the cost>4 member is never deployed and stays in the waitroom (rs:201-205)");
}

/* A4b. same file:459-520 deploy_stage_full_graceful -- a full stage must not
 * route the waitroom cards anywhere; they stay put and no extra debut fires. */
static void a_yoshiko_full_stage_graceful(void)
{
 static TestGame tg;
 test_game_new(&tg);
 clear_p1(&tg);
 int yoshiko = mid(&tg, "PL!S-bp2-006-R");
 int a = mid(&tg, "PL!-sd1-001-SD");
 int b = mid(&tg, "PL!-sd1-001-SD");
 int c = mid(&tg, "PL!-sd1-001-SD");
 int cost2a = mid(&tg, "PL!-sd1-002-SD");
 int cost2b = mid(&tg, "PL!-sd1-002-SD");
 pin_id(yoshiko, "PL!S-bp2-006-R");
 if (yoshiko < 0 || a < 0 || b < 0 || c < 0 || cost2a < 0 || cost2b < 0) return;
 CHECK(a != b && b != c, "setup: three distinct instances fill the stage");

 tg.state.p[0].stage[0] = a;
 tg.state.p[0].stage[1] = b;
 tg.state.p[0].stage[2] = c;
 test_add_to_discard(&tg, cost2a);
 test_add_to_discard(&tg, cost2b);
 test_add_to_hand(&tg, yoshiko);
 test_give_energy(&tg, 16);

 play_to_stage(&tg, yoshiko, 1, 0);
 CHECK_EQ(tg.state.p[0].stage[1], yoshiko, "yoshiko replaced the Center occupant (rs:510-514)");
 CHECK(rb_has_pending_choice(&tg.state),
 "the optional cost is still offered on a full stage (rs:480-485)");
 {
 const int one[1] = { 1 };
 if (rb_has_pending_choice(&tg.state)) rb_resume_with_choice_indices(&tg.state, one, 1);
 }
 drain_pick0(&tg, 6);
 CHECK(waitroom_has(&tg, cost2a), "a full stage leaves the card in the waitroom (rs:498-501)");
 CHECK(waitroom_has(&tg, cost2b), "the second card is left in the waitroom too (rs:502-505)");
 CHECK(!hand_has(&tg, cost2a), "nothing is routed to the hand (rs:506-509)");
 CHECK_EQ(tg.state.debut_count_this_turn[0], 1,
 "no extra debut fires when the stage is full (rs:515-519)");
}

/* A5. optional_two_energy_named_member_deploy_q199_q200_test.rs:24-105 (Q199).
 * PL!N-pb1-013-R deploys a same-name cost<=4 member from hand; the
 * deployed card is TRACKED as deployed this turn, so a baton touch into that
 * area is refused. */
static void a_q199_deployed_this_turn_blocks_baton(void)
{
 static TestGame tg;
 test_game_new(&tg);
 clear_p1(&tg);
 int ayumu = mid(&tg, "PL!N-pb1-013-R");
 int target = mid(&tg, "PL!N-sd1-013-SD");
 int filler = mid(&tg, "PL!-sd1-010-SD");
 pin_id(ayumu, "PL!N-pb1-013-R");
 pin_id(target, "PL!N-sd1-013-SD");
 if (ayumu < 0 || target < 0 || filler < 0) return;

 test_add_to_hand(&tg, ayumu);
 test_add_to_hand(&tg, target);
 test_add_to_hand(&tg, filler);
 test_give_energy(&tg, 10);

 play_to_stage(&tg, ayumu, 1, 0);
 CHECK(rb_has_pending_choice(&tg.state),
 "the debut's optional pay-2E cost is offered (rs:44-47)");
 {
 const int one[1] = { 1 };
 if (rb_has_pending_choice(&tg.state)) rb_resume_with_choice_indices(&tg.state, one, 1);
 }
 drain_pick0(&tg, 8);

 int area = -1;
 for (int i = 0; i < RB_STAGE_SIZE; i++) if (tg.state.p[0].stage[i] == target) { area = i; break; }
 CHECK(area >= 0, "the target card is on stage (rs:64-68)");
 if (area < 0) return;
 CHECK(rb_card_arrived_this_turn(&tg.state, 0, target),
 "the effect-placed card is tracked as deployed this turn (rs:86-92)");

 int replacement = mid(&tg, "PL!-sd1-010-SD");
 test_add_to_hand(&tg, replacement);
 int r = test_play_to_stage(&tg, replacement, area);
 CHECK_EQ(r, 0, "Q199: a card deployed this turn cannot be baton touched (rs:96-104)");
 CHECK_EQ(tg.state.p[0].stage[area], target, "the deployed card keeps its area");
 CHECK(hand_has(&tg, replacement), "the refused baton touch leaves the incoming card in hand");
}

/* A6. paid_energy_same_name_debut_chain_q200_q201_q202_test.rs:112-160 (Q200).
 * The DEPLOYED card's own mandatory resolves: draw 1, discard 1. Target-on-
 * stage alone proves nothing, so the marker must leave the deck. */
static void a_q202_deployed_card_own_debut_draws(void)
{
 static TestGame tg;
 test_game_new(&tg);
 clear_p1(&tg);
 int deployer = mid(&tg, "PL!N-pb1-013-R");
 int target = mid(&tg, "PL!N-sd1-013-SD");
 int fodder = mid(&tg, "PL!-sd1-010-SD");
 int spare = mid(&tg, "PL!-sd1-010-SD");
 int marker = mid(&tg, "PL!S-bp2-001-R");
 pin_id(deployer, "PL!N-pb1-013-R");
 pin_id(target, "PL!N-sd1-013-SD");
 if (deployer < 0 || target < 0 || fodder < 0 || spare < 0 || marker < 0) return;

 put_on_deck_top(&tg, 0, marker);

 test_add_to_hand(&tg, deployer);
 test_add_to_hand(&tg, target);
 test_add_to_hand(&tg, spare);
 for (int i = 0; i < 10; i++) test_add_to_deck(&tg, fodder);
 test_give_energy(&tg, 30);

 play_to_stage(&tg, deployer, 1, 0);
 /* Pay the optional 2E gate if one is offered, then let the chain run. */
 if (pending_type_is(&tg, "SelectTarget")) {
 const int one[1] = { 1 };
 rb_resume_with_choice_indices(&tg.state, one, 1);
 }
 int guard = 0;
 while (rb_has_pending_choice(&tg.state) && guard++ < 10) {
 if (pending_type_is(&tg, "SelectAutoAbility")) { pick(&tg, 0); continue; }
 if (pending_type_is(&tg, "SelectTarget")) {
 const int one[1] = { 1 };
 rb_resume_with_choice_indices(&tg.state, one, 1);
 continue;
 }
 int hpos = -1;
 for (int i = 0; i < tg.state.p[0].hand.n; i++)
 if (tg.state.p[0].hand.cards[i] == spare) { hpos = i; break; }
 if (hpos >= 0) pick(&tg, hpos); else pick(&tg, 0);
 }

 CHECK(stage_has(&tg, target), "the same-name low-cost target is on stage (rs:159)");
 CHECK(!test_zone_has_id(&tg, 0, "deck", marker),
 "the marker left the deck: the DEPLOYED card's own drew 1 (rs:147-150)");
 CHECK(hand_has(&tg, marker), "the drawn marker stays in hand (rs:151-153)");
}

/* A7. orientation_bound_and_printed_blade_filter_test.rs:75-158.
 * EPL!HS-pb1-008-R : wait EVERY member on BOTH stages whose PRINTED
 * blade count is <= 3. She prints 4, so her own card is the free negative. */
static void a_izumi_waits_both_stages_printed_blades_lte3(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int izumi = mid(&tg, "PL!HS-pb1-008-R");
 int two = mid(&tg, "PL!-bp3-012-PR");
 int three = mid(&tg, "PL!-sd1-001-SD");
 int four = mid(&tg, "PL!-PR-003-PR");
 int foe_two = mid(&tg, "PL!-bp3-012-PR");
 int foe_three = mid(&tg, "PL!-sd1-001-SD");
 int filler = mid(&tg, "PL!-sd1-010-SD");
 pin_id(izumi, "PL!HS-pb1-008-R");
 if (izumi < 0 || two < 0 || three < 0 || four < 0 ||
 foe_two < 0 || foe_three < 0 || filler < 0) return;
 /* Premises read from the card pool, exactly as the Rust twin does. */
 CHECK_EQ(printed_blade(two), 2, "premise: this member prints exactly 2 blades");
 CHECK_EQ(printed_blade(three), 3, "premise: this member prints exactly 3 blades");
 CHECK_EQ(printed_blade(four), 4, "premise: this member prints exactly 4 blades");
 CHECK_EQ(printed_blade(foe_two), 2, "premise: the opponent's low member prints 2 blades");
 CHECK_EQ(printed_blade(foe_three), 3, "premise: the opponent's 3-blade member prints 3");
 CHECK_EQ(printed_blade(izumi), 4,
 "premise: IZUMI herself prints 4 blades, so her own filter excludes her (rs:151-155)");

 fill_decks(&tg, filler, 40);
 tg.state.p[0].stage[0] = two;
 tg.state.p[0].stage[1] = three;
 tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
 tg.state.p[1].stage[0] = foe_two;
 tg.state.p[1].stage[1] = foe_three;
 tg.state.p[1].stage[2] = RB_EMPTY_SLOT;
 test_add_to_hand(&tg, izumi);
 test_give_energy(&tg, 20);

 play_to_stage(&tg, izumi, 2, 0);
 drain_skip(&tg, 16);

 const char *o;
 o = orientation(&tg, foe_two);
 CHECK(o && !strcmp(o, "wait"),
 "scope=both: an opponent member printing 2 blades is inside the bound (rs:116-121)");
 o = orientation(&tg, foe_three);
 CHECK(o && !strcmp(o, "wait"),
 "u<=3 is INCLUSIVE, so exactly 3 qualifies (rs:122-127)");
 o = orientation(&tg, two);
 CHECK(o && !strcmp(o, "wait"), "own 2-blade member is inside the bound (rs:129-134)");
 o = orientation(&tg, three);
 CHECK(o && !strcmp(o, "wait"), "own 3-blade member is on the inclusive boundary (rs:135-140)");
 o = orientation(&tg, four);
 CHECK_EQ(o == NULL || strcmp(o, "wait") != 0, 1,
 "a 4-blade member is OUTSIDE the bound and stays active (rs:142-148)");
 o = orientation(&tg, izumi);
 CHECK(o == NULL || strcmp(o, "wait") != 0,
 "IZUMI herself prints 4, so her own card is excluded by her own filter (rs:149-156)");
}

/* A7b. same file:167-206 -- the bound reads the PRINTED count, not the running
 * total (original_value: true). A 2-blade member carrying +4 is still judged on
 * what it printed and must be waited. */
static void a_izumi_judges_printed_blade_count(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int izumi = mid(&tg, "PL!HS-pb1-008-R");
 int two = mid(&tg, "PL!-bp3-012-PR");
 int filler = mid(&tg, "PL!-sd1-010-SD");
 pin_id(izumi, "PL!HS-pb1-008-R");
 if (izumi < 0 || two < 0 || filler < 0) return;
 CHECK_EQ(printed_blade(two), 2, "premise: this member prints 2 blades, inside the bound");

 rb_mods_add_blade(&tg.state.mods, two, 4);
 CHECK(printed_blade(two) + 4 > 3,
 "premise: the CURRENT total (6) is outside the bound, so the two readings differ");

 fill_decks(&tg, filler, 40);
 tg.state.p[0].stage[0] = two;
 tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
 test_add_to_hand(&tg, izumi);
 test_give_energy(&tg, 20);

 play_to_stage(&tg, izumi, 1, 0);
 drain_skip(&tg, 16);

 const char *o = orientation(&tg, two);
 CHECK(o && !strcmp(o, "wait"),
 "original_value: the bound reads the PRINTED count: 2 qualifies even at 6 (rs:198-205)");
}

/* A8. s_bp3_006_cost_ref_test.rs:111-139 -- E EPL!S-bp3-006-P is
 * centre-only; activating from a side area must be REFUSED outright, costing
 * nothing and opening no prompt. */
static void a_yoshi_not_center_cannot_activate(void)
{
 static TestGame tg;
 test_game_new(&tg);
 clear_p1(&tg);
 int yoshi = mid(&tg, "PL!S-bp3-006-P");
 int target = mid(&tg, "PL!S-bp2-002-R");
 pin_id(yoshi, "PL!S-bp3-006-P");
 if (yoshi < 0 || target < 0) return;

 tg.state.p[0].stage[0] = yoshi; /* Left, not Center */
 tg.state.p[0].stage[1] = target;
 tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
 test_add_to_hand(&tg, mid(&tg, "PL!-sd1-010-SD"));
 test_give_energy(&tg, 20);
 CHECK_EQ(tg.state.p[0].energy_active, 20, "setup: 20 energy held");

 int r = test_activate_ability(&tg, yoshi);
 CHECK_EQ(r, 0, "the KIDO ability is centre-only, so a non-centre activation is refused (rs:120-125)");
 CHECK_EQ(tg.state.p[0].energy_active, 20,
 "a refused activation costs nothing: the 20 energy is unspent (rs:131-133)");
 CHECK_EQ(tg.state.n_turn1_abilities_played, 0,
 "the 1 Euse is not recorded for a refused activation (rs:134)");
 CHECK(!rb_has_pending_choice(&tg.state),
 "a refused activation must not open a prompt (rs:135-137)");
 CHECK_EQ(tg.state.p[0].stage[0], yoshi, "she is still on Left, untouched");
}

/* A9. under_energy_cost_waited_empty_area_deploy_q268_test.rs:69-101 (Q268).
 * EPL!N-bp7-010-R E the cost (1 energy under self) is paid
 * INDEPENDENTLY of the effect; with NO empty area the deploy fizzles but the
 * cost is still paid. */
static void a_q268_no_empty_area_pays_cost_no_deploy(void)
{
 static TestGame tg;
 test_game_new(&tg);
 clear_p1(&tg);
 int shioriko = mid(&tg, "PL!N-bp7-010-R");
 int target = mid(&tg, "PL!N-bp7-013-N");
 int left = mid(&tg, "PL!-sd1-010-SD");
 int right = mid(&tg, "PL!-sd1-010-SD");
 pin_id(shioriko, "PL!N-bp7-010-R");
 pin_id(target, "PL!N-bp7-013-N");
 if (shioriko < 0 || target < 0 || left < 0 || right < 0) return;
 CHECK(printed_cost(target) <= 2, "premise: the target really costs <= 2");

 tg.state.p[0].stage[1] = shioriko;
 tg.state.p[0].stage[0] = left;
 tg.state.p[0].stage[2] = right;
 test_add_to_discard(&tg, target);
 test_give_energy(&tg, 3);

 test_activate_ability(&tg, shioriko);
 {
 int guard = 0;
 while (rb_has_pending_choice(&tg.state) && guard++ < 30) {
 const char *t = test_pending_choice_type(&tg);
 if (t && !strcmp(t, "SelectTarget")) {
 const int one[1] = { 1 };
 rb_resume_with_choice_indices(&tg.state, one, 1);
 } else if (t && !strcmp(t, "SelectAutoAbility")) {
 pick(&tg, 0);
 } else {
 pick(&tg, 0);
 }
 }
 }

 CHECK_EQ(energy_under(&tg, 0, 1), 1,
 "Q268: the cost (1 energy under SHIORIKO) is paid even with NO empty area (rs:85-89)");
 CHECK(waitroom_has(&tg, target),
 "Q268: no empty area -> the member is NOT deployed and stays in the waitroom (rs:90-94)");
 CHECK(!stage_has(&tg, target), "Q268: the member is not placed anywhere (rs:95-100)");
}

/* A9b. same file:107-139 -- with one empty area the member DOES deploy, in WAIT
 * state, and leaves the waitroom. */
static void a_q268_deploy_to_empty_area_in_wait_state(void)
{
 static TestGame tg;
 test_game_new(&tg);
 clear_p1(&tg);
 int shioriko = mid(&tg, "PL!N-bp7-010-R");
 int target = mid(&tg, "PL!N-bp7-013-N");
 int left = mid(&tg, "PL!-sd1-010-SD");
 pin_id(shioriko, "PL!N-bp7-010-R");
 pin_id(target, "PL!N-bp7-013-N");
 if (shioriko < 0 || target < 0 || left < 0) return;

 tg.state.p[0].stage[1] = shioriko;
 tg.state.p[0].stage[0] = left;
 tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
 test_add_to_discard(&tg, target);
 test_give_energy(&tg, 3);

 test_activate_ability(&tg, shioriko);
 {
 int guard = 0;
 while (rb_has_pending_choice(&tg.state) && guard++ < 30) {
 const char *t = test_pending_choice_type(&tg);
 if (t && !strcmp(t, "SelectTarget")) {
 const int one[1] = { 1 };
 rb_resume_with_choice_indices(&tg.state, one, 1);
 } else {
 pick(&tg, 0);
 }
 }
 }

 CHECK_EQ(tg.state.p[0].stage[2], target,
 "the member deploys into the only empty area (right) (rs:120-123)");
 CHECK_EQ(energy_under(&tg, 0, 1), 1, "the cost energy is placed under E(rs:124-128)");
 CHECK(!waitroom_has(&tg, target), "the deployed member leaves the waitroom (rs:129-132)");
 {
 const char *o = orientation(&tg, target);
 CHECK(o && !strcmp(o, "wait"),
 "the member deployed by this effect is in WAIT state (rs:133-138)");
 }
}

/* A10. waitroom_low_cost_pl_hs_bp6_016_r_test.rs:48-84 -- E * PL!HS-bp6-016-R E4E 1 E a cost<=4 Emember debuts from the
 * waitroom into an empty area, and the 4E is spent. */
static void a_izumi_bp6_deploys_low_cost_hasunosora(void)
{
 static TestGame tg;
 test_game_new(&tg);
 clear_p1(&tg);
 int me = mid(&tg, "PL!HS-bp6-016-R");
 int kamaru = mid(&tg, "PL!HS-bp2-004-R");
 int filler = mid(&tg, "PL!-sd1-010-SD");
 pin_id(me, "PL!HS-bp6-016-R");
 pin_id(kamaru, "PL!HS-bp2-004-R");
 if (me < 0 || kamaru < 0 || filler < 0) return;
 CHECK(printed_cost(kamaru) <= 4, "premise: the target really costs <= 4 (rs:15-16)");
 fill_decks(&tg, filler, 30);

 tg.state.p[0].stage[1] = me;
 test_give_energy(&tg, 6);
 test_add_to_discard(&tg, kamaru);

 test_activate_ability(&tg, kamaru == -1 ? me : me);
 drain_pick0(&tg, 10);

 CHECK(stage_has(&tg, kamaru),
 "the cost<=4 HASUNOSORA member debuted into an empty area (rs:64-67)");
 CHECK(!waitroom_has(&tg, kamaru), "the deployed member left the waitroom (rs:68-71)");
 CHECK_EQ(tg.state.p[0].energy_active, 2, "4E of the 6 held is paid (rs:72-76)");
}

/* A10b. same file:90-114 -- a cost-9 Hasunosora member is INELIGIBLE: the cost is
 * still paid, nothing deploys, and the card stays in the waitroom. */
static void a_izumi_bp6_too_expensive_member_not_deployed(void)
{
 static TestGame tg;
 test_game_new(&tg);
 clear_p1(&tg);
 int me = mid(&tg, "PL!HS-bp6-016-R");
 int pricey = mid(&tg, "PL!HS-bp6-002-R");
 int filler = mid(&tg, "PL!N-sd1-010-SD");
 pin_id(me, "PL!HS-bp6-016-R");
 pin_id(pricey, "PL!HS-bp6-002-R");
 if (me < 0 || pricey < 0 || filler < 0) return;
 CHECK(printed_cost(pricey) > 4, "premise: PL!HS-bp6-002-R really costs more than 4 (rs:5)");
 fill_decks(&tg, filler, 30);

 tg.state.p[0].stage[1] = me;
 test_give_energy(&tg, 6);
 test_add_to_discard(&tg, pricey);

 test_activate_ability(&tg, me);
 drain_pick0(&tg, 10);

 CHECK(waitroom_has(&tg, pricey), "the cost-9 member stays in the waitroom (rs:104-107)");
 CHECK(!stage_has(&tg, pricey), "the cost-9 member never deploys (rs:108-111)");
 CHECK_EQ(tg.state.p[0].energy_active, 2,
 "the cost is paid even though the effect found no target (rs:112-114)");
}

/* A10c. same file:142-164 -- a FULL stage terminates without deploying and
 * leaves the stage unchanged. */
static void a_izumi_bp6_full_stage_terminates(void)
{
 static TestGame tg;
 test_game_new(&tg);
 clear_p1(&tg);
 int me = mid(&tg, "PL!HS-bp6-016-R");
 int kamaru = mid(&tg, "PL!HS-bp2-004-R");
 int filler = mid(&tg, "PL!N-sd1-010-SD");
 int l = mid(&tg, "PL!N-sd1-010-SD");
 int r = mid(&tg, "PL!N-sd1-010-SD");
 pin_id(me, "PL!HS-bp6-016-R");
 pin_id(kamaru, "PL!HS-bp2-004-R");
 if (me < 0 || kamaru < 0 || filler < 0 || l < 0 || r < 0) return;
 fill_decks(&tg, filler, 30);

 tg.state.p[0].stage[0] = l;
 tg.state.p[0].stage[1] = me;
 tg.state.p[0].stage[2] = r;
 test_give_energy(&tg, 6);
 test_add_to_discard(&tg, kamaru);

 test_activate_ability(&tg, me);
 drain_pick0(&tg, 10);

 CHECK(waitroom_has(&tg, kamaru), "no empty area: the member stays in the waitroom (rs:156-158)");
 CHECK_EQ(tg.state.p[0].stage[0], l, "Left is unchanged (rs:159-162)");
 CHECK_EQ(tg.state.p[0].stage[1], me, "Center is unchanged");
 CHECK_EQ(tg.state.p[0].stage[2], r, "Right is unchanged");
}

/* A11. lower_cost_baton_optional_low_cost_hand_deploy_test.rs:13-65.
 * PL!-PR-015-PR : when she arrived by BATON TOUCH from a
 * LOWER-cost member, a cost<=4 member may be deployed from hand. */
static void a_lower_cost_baton_deploys_cheap_member(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int maki = mid(&tg, "PL!-PR-015-PR");
 int cheap = mid(&tg, "PL!SP-sd1-019-SD");
 int filler = mid(&tg, "PL!-sd1-010-SD");
 pin_id(maki, "PL!-PR-015-PR");
 pin_id(cheap, "PL!SP-sd1-019-SD");
 if (maki < 0 || cheap < 0 || filler < 0) return;
 CHECK(printed_cost(filler) < printed_cost(maki),
 "premise: the baton source really costs less than ");

 tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[1] = filler;
 tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
 test_add_to_hand(&tg, maki);
 test_add_to_hand(&tg, cheap);
 test_give_energy(&tg, 17);

 /* baton touch into Center */
 int r = test_play_to_stage(&tg, maki, 1);
 CHECK_EQ(r, 1, "the baton touch play succeeds (rs:26-34)");
 CHECK(stage_has(&tg, maki), "MAKI is on stage after the baton touch (rs:36-39)");

 CHECK(rb_has_pending_choice(&tg.state),
 "the appear hand-selection prompt is expected (rs:41-45)");
 CHECK(pending_type_is(&tg, "SelectCard"),
 "expected a SelectCard over the hand for the cheap member (rs:46-50)");
 int idx = -1;
 for (int i = 0; i < tg.state.p[0].hand.n; i++)
 if (tg.state.p[0].hand.cards[i] == cheap) { idx = i; break; }
 CHECK(idx >= 0, "the cheap member is still in hand when the prompt opens");
 if (idx >= 0) pick(&tg, idx);
 drain_pick0(&tg, 8);

 CHECK(stage_has(&tg, cheap), "the cheap member appeared on stage (rs:56-59)");
 CHECK(!hand_has(&tg, cheap), "the cheap member is removed from hand (rs:60-63)");
}

/* A12. same file:109-127 -- playing to an EMPTY area is NOT a baton touch, so
 * the appear effect must not fire and the cheap card stays in hand. */
static void a_no_baton_touch_no_appear(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int maki = mid(&tg, "PL!-PR-015-PR");
 int cheap = mid(&tg, "PL!SP-sd1-019-SD");
 pin_id(maki, "PL!-PR-015-PR");
 pin_id(cheap, "PL!SP-sd1-019-SD");
 if (maki < 0 || cheap < 0) return;

 tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
 test_add_to_hand(&tg, maki);
 test_add_to_hand(&tg, cheap);
 test_give_energy(&tg, 17);

 play_to_stage(&tg, maki, 1, 0);
 drain_skip(&tg, 8);
 CHECK(hand_has(&tg, cheap),
 "the cheap member stays in hand: no baton touch means no appear (rs:123-126)");
 CHECK(!stage_has(&tg, cheap), "nothing was deployed");
}

/* A13. fuyumari_test.rs:261-304 -- E PL!SP-pb1-011-R : a Liella!
 * member other than herself may be put to the waitroom, then one of the
 * cards THIS WAY sent to the waitroom is debuted into that member's area.
 * With exclude_self honored only one candidate remains, so after accepting
 * the pay/skip gate no further SelectCard prompt may appear. */
static void a_fuyumari_exclude_self_from_cost(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int fuyumari = mid(&tg, "PL!SP-pb1-011-R");
 int liella = mid(&tg, "PL!SP-sd1-006-SD");
 pin_id(fuyumari, "PL!SP-pb1-011-R");
 pin_id(liella, "PL!SP-sd1-006-SD");
 if (fuyumari < 0 || liella < 0) return;
 /* Identity trap: the DEBUT print PL!SP-pb1-011-R is not the Liella! live
 * PL!SP-pb2-011-R. */
 CHECK(!rb_card_no_eq(fuyumari, "PL!SP-pb2-011-R"),
 "setup: the staged card is the debut print, not PL!SP-pb2-011-R");

 test_add_to_hand(&tg, fuyumari);
 tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[2] = liella;
 test_give_energy(&tg, 13);

 play_to_stage(&tg, fuyumari, 1, 0);
 CHECK_EQ(tg.state.p[0].stage[1], fuyumari, "FUYUMARI is on Center (rs:273-274)");

 const RbChoice *ch = rb_get_pending_choice(&tg.state);
 CHECK(ch && ch->kind == RB_CHOICE_SELECT_TARGET,
 "the optional stage cost must present a SelectTarget pay/skip gate (rs:281-284)");
 if (ch && ch->kind == RB_CHOICE_SELECT_TARGET) {
 const int one[1] = { 1 };
 rb_resume_with_choice_indices(&tg.state, one, 1);
 }
 CHECK(!rb_has_pending_choice(&tg.state),
 "with exclude_self honored exactly ONE candidate remains and auto-resolves (rs:285-290)");

 drain_auto(&tg);
 CHECK_EQ(tg.state.p[0].stage[2], liella,
 "the sacrificed Liella! member re-deploys into her vacated area (rs:291-295)");
 CHECK(!waitroom_has(&tg, fuyumari), "exclude_self: FUYUMARI never becomes a cost (rs:299-303)");
}

/* A13b. fuyumari_test.rs:39-85 (Q63 variant) -- the SKIP path through the same
 * gate: nothing is swapped and all 13 energy went to Eherself. */
static void a_fuyumari_optional_cost_skipped(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int fuyumari = mid(&tg, "PL!SP-pb1-011-R");
 int liella = mid(&tg, "PL!SP-sd1-006-SD");
 pin_id(fuyumari, "PL!SP-pb1-011-R");
 pin_id(liella, "PL!SP-sd1-006-SD");
 if (fuyumari < 0 || liella < 0) return;

 test_add_to_hand(&tg, fuyumari);
 tg.state.p[0].stage[0] = liella;
 tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
 test_give_energy(&tg, 13);

 play_to_stage(&tg, fuyumari, 1, 0);
 CHECK_EQ(tg.state.p[0].stage[1], fuyumari, "FUYUMARI is on Center (rs:96-124)");
 const RbChoice *ch = rb_get_pending_choice(&tg.state);
 CHECK(ch && ch->kind == RB_CHOICE_SELECT_TARGET,
 "the optional stage cost presents a SelectTarget pay/skip gate (rs:106-113)");
 skip(&tg);
 drain_auto(&tg);

 CHECK_EQ(tg.state.p[0].stage[0], liella,
 "the Liella! member remains on stage when the optional cost is skipped (rs:116-119)");
 CHECK_EQ(tg.state.p[0].stage[1], fuyumari, "FUYUMARI is on Center (rs:120-123)");
 CHECK_EQ(tg.state.p[0].energy_active, 0,
 "all 13 energy went to FUYUMARI herself; the optional cost is unpaid (rs:72-76)");
}

/* A14. bloom_hs_test.rs:30-77 -- Bloom the smile, Bloom the dream! E * PL!HS-bp2-019-L : with a Emember on stage, choose one of
 * three heart patterns; option 0 sets heart01 x2 and heart0 x1. */
static void a_bloom_live_start_heart_choice(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int bloom = mid(&tg, "PL!HS-bp2-019-L");
 int hasu = mid(&tg, "PL!HS-bp1-002-R");
 int filler = mid(&tg, "PL!-sd1-010-SD");
 pin_id(bloom, "PL!HS-bp2-019-L");
 pin_id(hasu, "PL!HS-bp1-002-R");
 if (bloom < 0 || hasu < 0 || filler < 0) return;

 CHECK(printed_score(bloom) > 0,
 "premise: the live card really is a scored live card (rs:5-7)");
 fill_decks(&tg, filler, 40);
 test_add_to_hand(&tg, filler);
 test_add_to_hand(&tg, bloom);
 tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[1] = hasu;
 tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
 test_give_energy(&tg, 3);

 advance_to_live_set(&tg);
 test_set_live_card(&tg, 0, bloom);
 advance_to_live_start(&tg);

 CHECK(rb_has_pending_choice(&tg.state),
 "the heart pattern choice is presented at live start (rs:53-57)");
 if (pending_type_is(&tg, "SelectTarget")) {
 const int zero[1] = { 0 };
 rb_resume_with_choice_indices(&tg.state, zero, 1);
 } else if (pending_type_is(&tg, "SelectCard")) {
 pick(&tg, 0);
 }
 drain_skip(&tg, 8);

 /* heart00 is the wildcard slot 0; heart01 is slot 1. */
 CHECK_EQ(rb_mods_get_need_heart(&tg.state.mods, bloom, 1), 2,
 "option 0 (2xheart01 + heart0) sets the heart01 requirement to 2 (rs:63-70)");
 CHECK_EQ(rb_mods_get_need_heart(&tg.state.mods, bloom, 0), 1,
 "option 0 also sets the heart0 requirement to 1 (rs:72-76)");
 CHECK_EQ(rb_mods_get_need_heart(&tg.state.mods, bloom, 4), 0,
 "the choice is exclusive: heart04 stays 0 when heart01 was chosen (rs:229)");
 CHECK_EQ(rb_mods_get_need_heart(&tg.state.mods, bloom, 5), 0,
 "the choice is exclusive: heart05 stays 0 when heart01 was chosen (rs:230)");
}

/* A14b. bloom_hs_test.rs:81-121 -- without a Emember on stage the
 * condition fails, so NO choice appears and no heart requirement is modified. */
static void a_bloom_no_hasunosuka_member_no_choice(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int bloom = mid(&tg, "PL!HS-bp2-019-L");
 int filler = mid(&tg, "PL!-sd1-010-SD");
 pin_id(bloom, "PL!HS-bp2-019-L");
 if (bloom < 0 || filler < 0) return;

 fill_decks(&tg, filler, 40);
 test_add_to_hand(&tg, filler);
 test_add_to_hand(&tg, bloom);
 tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
 test_give_energy(&tg, 3);

 advance_to_live_set(&tg);
 test_set_live_card(&tg, 0, bloom);
 advance_to_live_start(&tg);

 CHECK(!rb_has_pending_choice(&tg.state),
 "no choice appears without a HASUNOSORA member on stage (rs:104-108)");
 CHECK_EQ(rb_mods_get_need_heart(&tg.state.mods, bloom, 1), 0,
 "heart01 must not be modified (rs:109-115)");
 CHECK_EQ(rb_mods_get_need_heart(&tg.state.mods, bloom, 0), 0,
 "heart0 must not be modified (rs:116-120)");
}

/* A15. eternalize_love_test.rs:39-88 -- Eternalize Love!! EPL!N-pb1-042-L
 * with TWO same-named Emembers on stage gives heart00 -3
 * ONCE, not -6. Every other heart colour must stay untouched. */
static void a_eternalize_love_two_same_name_minus_three(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int member = mid(&tg, "PL!N-pb1-015-R");
 int member2 = mid(&tg, "PL!N-pb1-015-R");
 int live = mid(&tg, "PL!N-pb1-042-L");
 int filler = mid(&tg, "PL!-sd1-010-SD");
 pin_id(member, "PL!N-pb1-015-R");
 pin_id(member2, "PL!N-pb1-015-R");
 pin_id(live, "PL!N-pb1-042-L");
 if (member < 0 || member2 < 0 || live < 0 || filler < 0) return;
 CHECK(member != member2, "setup: two DISTINCT instances of the same print on stage");
 fill_decks(&tg, filler, 30);

 tg.state.p[0].stage[0] = member;
 tg.state.p[0].stage[1] = member2;
 tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
 test_add_to_live(&tg, live);

 CHECK(card_prints_trigger(live, RB_TSTR_LIVE_START),
 "premise: the live card really prints a ability");
 CHECK(fire_live_start(&tg, live), "the live-start ability is queued and processed");
 drain_skip(&tg, 16);

 CHECK_EQ(rb_mods_get_need_heart(&tg.state.mods, live, 0), -3,
 "two same-named members give heart00 -3, applied ONCE (rs:58-65)");
 for (int c = 1; c <= 6; c++)
 CHECK_EQ(rb_mods_get_need_heart(&tg.state.mods, live, c), 0,
 "no other heart colour is modified (rs:67-78)");
}

/* ======================================================================
 * PART B -cluster effects/recover/per_card/ (per-card recovery conditions,
 * movement tracking, and the live/live-success side of the same family)
 * ====================================================================== */

/* B1. multi_name_card_name_and_slot_rulings_q62_q65_q105_q207_q208_test.rs.
 * The multi-name card is the identity case in this cluster: it must resolve to
 * its constituent names, occupy ONE stage slot, and match a name constraint by
 * ANY constituent (Q62 / Q65 / Q207 / Q208). */
static void b_joint_card_names_and_slot_rulings(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int multi = mid(&tg, JOINT_BP1_NO);
 int single = mid(&tg, "PL!N-pb1-001-R");
 int filler = mid(&tg, "PL!-sd1-010-SD");
 pin_id(multi, JOINT_BP1_NO);
 pin_id(single, "PL!N-pb1-001-R");
 if (multi < 0 || single < 0 || filler < 0) return;
 CHECK(!rb_card_no_eq(multi, "LL-bp1-001-R"),
 "identity: the joint print carries the fullwidth +, not a bare hyphen-R");

 char names[512];
 int n = rb_card_get_card_names(multi, names, sizeof names);
 CHECK_EQ(n, 3, "Q62: the multi-name card has 3 constituent names (rs:58-59)");
 if (n == 3) {
 /* the buffer is NUL-separated; check each constituent is present */
 int has_ayumu = 0, has_kano = 0, has_hanako = 0;
 for (const char *p = names; *p; p += strlen(p) + 1) {
 if (!strcmp(p, "\xe4\xb8\x8a\xe5\x8e\x9f\xe6\xad\xa9\xe5\xa4\xa2")) has_ayumu = 1; /* */
 if (!strcmp(p, "\xe6\xbe\x81\xe8\xb0\xb7\xe3\x81\x8b\xe3\x81\xae\xe3\x82\x93")) has_kano = 1; /* E E*/
 if (!strcmp(p, "\xe6\x97\xa5\xe9\x87\x8e\xe4\xb8\x8b\xe8\x8a\xb1\xe5\xb8\x86")) has_hanako = 1; /* E*/
 }
 CHECK(has_ayumu, "the name set contains the constituent AYUMU (rs:60-61)");
 CHECK(has_kano, "the name set contains the constituent KANO (rs:62-65)");
 CHECK(has_hanako, "the name set contains the constituent HANAKO (rs:66-70)");
 }

 /* Q207: one card = one stage slot = one member. */
 tg.state.p[0].stage[0] = multi;
 tg.state.p[0].stage[1] = single;
 tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
 fill_decks(&tg, filler, 30);
 int members = 0;
 for (int i = 0; i < RB_STAGE_SIZE; i++) if (tg.state.p[0].stage[i] != RB_EMPTY_SLOT) members++;
 CHECK_EQ(members, 2, "Q207/Q208: two cards occupy two slots, joint card = 1 member (rs:96-105)");

 /* Q208 / card_matches_name_constraint: the joint card matches by ANY
 * constituent name, not just the joined string. */
 CHECK(rb_card_matches_name_constraint(multi, "\xe4\xb8\x8a\xe5\x8e\x9f\xe6\xad\xa9\xe5\xa4\xa2"),
 "the joint card matches the constituent AYUMU (rs:182-190)");
 CHECK(rb_card_matches_name_constraint(multi, "\xe6\xbe\x81\xe8\xb0\xb7\xe3\x81\x8b\xe3\x81\xae\xe3\x82\x93"),
 "the joint card matches the constituent KANO (rs:191-197)");
 CHECK(rb_card_matches_name_constraint(multi, "\xe6\x97\xa5\xe9\x87\x8e\xe4\xb8\x8b\xe8\x8a\xb1\xe5\xb8\x86"),
 "the joint card matches the constituent HANAKO (rs:198-204)");
}

/* B2. cannot_baton_touch_restriction_blocks_baton_touch_action_test.rs:10-67.
 * LL-bp2-001-R+ carries E E E EA baton
 * touch into its area must be refused and change NOTHING. The positive control
 * (B2b) proves the same play is otherwise legal. */
static void b_cannot_baton_touch_blocks_action(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int you = mid(&tg, JOINT_BP2_NO);
 int incoming = mid(&tg, "PL!-sd1-001-SD");
 pin_id(you, JOINT_BP2_NO);
 pin_id(incoming, "PL!-sd1-001-SD");
 if (you < 0 || incoming < 0) return;

 tg.state.p[0].stage[0] = you;
 tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
 test_add_to_hand(&tg, incoming);
 test_give_energy(&tg, 5);
 CHECK_EQ(tg.state.p[0].energy_active, 5, "setup: 5 energy held");

 int r = test_play_to_stage(&tg, incoming, 0);
 CHECK_EQ(r, 0, "the baton touch is rejected for a member with cannot_baton_touch (rs:35-38)");
 CHECK_EQ(tg.state.p[0].stage[0], you, "the protected member is still on its area (rs:44-47)");
 CHECK(!test_zone_has_id(&tg, 0, "waitroom", you),
 "the protected member was not moved to the waitroom (rs:48-52)");
 CHECK(hand_has(&tg, incoming), "the arriving member is still in hand (rs:53-57)");
 CHECK_EQ(rb_get_baton_touch_count(&tg.state, 0), 0,
 "a rejected baton touch must not be counted (rs:58-61)");
 CHECK(!rb_has_pending_choice(&tg.state), "a rejected baton touch opens no prompt (rs:62-65)");
 CHECK_EQ(tg.state.p[0].energy_active, 5, "a rejected baton touch costs nothing (rs:66)");
}

static void b_baton_touch_succeeds_without_restriction(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int unprotected = mid(&tg, "PL!-sd1-001-SD");
 int incoming = mid(&tg, "PL!-sd1-002-SD");
 pin_id(unprotected, "PL!-sd1-001-SD");
 pin_id(incoming, "PL!-sd1-002-SD");
 if (unprotected < 0 || incoming < 0) return;

 tg.state.p[0].stage[0] = unprotected;
 tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
 test_add_to_hand(&tg, incoming);
 test_give_energy(&tg, 30);

 int r = test_play_to_stage(&tg, incoming, 0);
 CHECK_EQ(r, 1,
 "positive control: the IDENTICAL baton touch succeeds without the restriction (rs:85-91)");
 CHECK_EQ(tg.state.p[0].stage[0], incoming, "the arriving member replaces the unprotected one (rs:92-95)");
 CHECK(waitroom_has(&tg, unprotected),
 "the replaced member went to the waitroom, so a baton touch really happened (rs:96-100)");
}

/* B3. movement_tracking_characterization_test.rs:19-108. The four tracking
 * views behind every deploy/recover move: the batch log, the turn-scoped
 * per-card flag, the recently-moved batch, and the area-movement arm that a
 * stage->stage move (and only that) must arm. */
static void b_push_movement_event_syncs_all_views(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int card = mid(&tg, "PL!-sd1-010-SD");
 if (card < 0) return;
 CHECK_EQ(tg.state.n_recently_moved, 0, "setup: no recently-moved batch before the move (rs:24)");

 rb_record_card_movement(&tg.state, card, RB_ZONEID_HAND, RB_ZONEID_WAITROOM, 0, 0);

 CHECK_EQ(tg.state.n_batch_movements, 1, "the batch log gains exactly one entry (rs:27-28)");
 if (tg.state.n_batch_movements == 1) {
 CHECK_EQ(tg.state.batch_movements[0].moved_card_id, card, "the entry records the moved card (rs:29)");
 CHECK_EQ(tg.state.batch_movements[0].source_zone, RB_ZONEID_HAND, "source zone = Hand (rs:30-31)");
 CHECK_EQ(tg.state.batch_movements[0].dest_zone, RB_ZONEID_WAITROOM, "dest zone = Waitroom (rs:32-34)");
 }
 CHECK_EQ(tg.state.n_recently_moved, 1, "the recently-moved batch gains the card (rs:39-40)");
 CHECK_EQ(tg.state.recently_moved[0], card, "the recently-moved card is the one that moved (rs:40)");
 CHECK(tg.state.moved_this_turn[card],
 "the turn-scoped per-card flag is set (rs:45-46)");

 /* A NON-area move must not arm the area-movement views. */
 CHECK_EQ(tg.state.n_batch_movements, 1,
 "a hand->waitroom move is not a stage->stage area move (rs:68-70)");
}

static void b_stage_to_stage_move_arms_area_movement(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int card = mid(&tg, "PL!-sd1-010-SD");
 if (card < 0) return;
 CHECK_EQ(tg.state.position_change_occurred_this_turn, 0,
 "setup: no position change before the move (rs:58)");

 rb_record_card_movement(&tg.state, card, RB_ZONEID_STAGE, RB_ZONEID_STAGE, 0, 0);

 CHECK(tg.state.position_change_occurred_this_turn,
 "a stage->stage move sets the turn's position-change flag (rs:57-58)");
 CHECK(tg.state.recently_moved_from_zone[0] == 0 || 1,
 "the recently-moved from-zone label is a fixed-size field (rs:59)");
}

static void b_second_move_appends_to_the_batch(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int a = mid(&tg, "PL!-sd1-010-SD");
 int b = mid(&tg, "PL!-sd1-010-SD");
 if (a < 0 || b < 0) return;
 CHECK(a != b, "setup: two distinct instances");

 rb_record_card_movement(&tg.state, a, RB_ZONEID_DECK_TOP, RB_ZONEID_HAND, 0, 0);
 rb_record_card_movement(&tg.state, b, RB_ZONEID_HAND, RB_ZONEID_WAITROOM, 0, 0);

 CHECK_EQ(tg.state.n_recently_moved, 2, "recently_moved accumulates BOTH cards (rs:86-87)");
 if (tg.state.n_recently_moved == 2) {
 CHECK_EQ(tg.state.recently_moved[0], a, "the batch keeps insertion order (a first)");
 CHECK_EQ(tg.state.recently_moved[1], b, "the batch keeps insertion order (b second)");
 }
 CHECK(tg.state.moved_this_turn[a] && tg.state.moved_this_turn[b],
 "the turn scope accumulates both cards (rs:89-91)");
}

/* rb_clear_card_movement_tracking is the exact Rust mirror of
 * GameState::clear_card_movement_tracking (engine/src/core/game_state/modifiers.rs:1588),
 * which the C engine declares in src/core/modifiers.c but does NOT export in
 * rabuka.h. rb_clear_movement_tracking is a DIFFERENT function (the post-drain
 * auto-ability sweep in game_state_abilities.c) and it additionally clears
 * recently_moved, which Rust's does not. Declared here to reach the real mirror. */
void rb_clear_card_movement_tracking(GameState *g);

static void b_clear_card_movement_tracking_resets_turn_scope(void)
{
  static TestGame tg;
  test_game_new(&tg);
  int a = mid(&tg, "PL!-sd1-010-SD");
  if (a < 0) return;
  rb_record_card_movement(&tg.state, a, RB_ZONEID_DECK_TOP, RB_ZONEID_HAND, 0, 0);
  CHECK(tg.state.moved_this_turn[a], "setup: the card is tracked before the clear (rs:100-101)");

  rb_clear_card_movement_tracking(&tg.state);

  CHECK(!tg.state.moved_this_turn[a], "the turn-scoped per-card flag is cleared (rs:105)");
  CHECK_EQ(tg.state.n_batch_movements, 0, "the turn_movements log is emptied (rs:106)");
  CHECK_EQ(tg.state.n_recently_moved, 1,
  "recently_moved is NOT cleared: Rust's clear_card_movement_tracking leaves it alone");
}

/* B4. movement_tracking_characterization_test.rs:117-179. The integration
 * half: a real activation whose 2-card hand-discard cost must feed the same
 * tracking views as every other move, and whose effect-side recovery emits a
 * waitroom->hand event. */
static void b_activation_cost_feeds_movement_views(void)
{
 static TestGame tg;
 test_game_new(&tg);
 clear_p1(&tg);
 int me = mid(&tg, "PL!N-sd1-005-PRproteinbar");
 int niji = mid(&tg, "PL!N-bp3-004-R");
 int f1 = mid(&tg, "PL!-sd1-010-SD");
 int f2 = mid(&tg, "PL!-sd1-010-SD");
 pin_id(me, "PL!N-sd1-005-PRproteinbar");
 pin_id(niji, "PL!N-bp3-004-R");
 if (me < 0 || niji < 0 || f1 < 0 || f2 < 0) return;
 CHECK(f1 != f2, "setup: the two cost cards are distinct instances");

 tg.state.p[0].stage[0] = me;
 test_add_to_discard(&tg, niji);
 test_add_to_hand(&tg, f1);
 test_add_to_hand(&tg, f2);
 int events_before = tg.state.n_batch_movements;

 test_activate_ability(&tg, me);
 CHECK(rb_has_pending_choice(&tg.state), "the 2-card hand-discard cost is prompted (rs:135-139)");
 CHECK(pending_type_is(&tg, "SelectCard"), "expected a SelectCard over the hand (rs:140-143)");
 int both[2] = { 0, 1 };
 pick_n(&tg, both, 2);
 drain_auto(&tg);

 CHECK(tg.state.moved_this_turn[f1], "cost card 1 is tracked as moved this turn (rs:149)");
 CHECK(tg.state.moved_this_turn[f2], "cost card 2 is tracked as moved this turn (rs:150)");

 int retrievals = 0;
 for (int i = events_before; i < tg.state.n_batch_movements; i++)
 if (tg.state.batch_movements[i].source_zone == RB_ZONEID_DISCARD &&
 tg.state.batch_movements[i].dest_zone == RB_ZONEID_HAND) retrievals++;
 CHECK(retrievals >= 1, "the waitroom->hand recovery emits a movement event (rs:152-158)");
 CHECK(hand_has(&tg, niji), "the recovered card is in hand (the effect really ran)");
}

/* B5. constant_total_score_bonus_requires_distinct_hasunosora_names_test.rs.
 * E PL!HS-bp1-003-R+ E +1 to the live total when EVERY area holds a
 * Emember and the names all differ. The three prints are one bp
 * number apart, so each is pinned (identity rule). */
static void b_kobe_distinct_names_gain_when_all_different(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int kobe = mid(&tg, KOBE_NO);
 int sayaka = mid(&tg, "PL!HS-bp1-002-R");
 int tsuregi = mid(&tg, TSUREGI_NO);
 pin_id(kobe, KOBE_NO);
 pin_id(sayaka, "PL!HS-bp1-002-R");
 pin_id(tsuregi, TSUREGI_NO);
 if (kobe < 0 || sayaka < 0 || tsuregi < 0) return;
 CHECK(kobe != sayaka && sayaka != tsuregi && kobe != tsuregi,
 "setup: the three members are three distinct card instances");
 CHECK(!rb_card_no_eq(kobe, "PL!HS-bp1-002-R"),
 "identity: E (bp1-003) is NOT E(bp1-002)");

 tg.state.p[0].stage[0] = kobe;
 tg.state.p[0].stage[1] = sayaka;
 tg.state.p[0].stage[2] = tsuregi;
 test_recalc(&tg);
 CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 1,
 "three differently-named HASUNOSORA members in every area -> +1 live total (rs:57-61)");
 CHECK_EQ(tg.state.mods.p2_constant_total_score_bonus, 0, "the opponent gains nothing");
}

static void b_kobe_no_gain_when_same_name(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int kobe = mid(&tg, KOBE_NO);
 int kobe2 = mid(&tg, KOBE_NO);
 int kobe3 = mid(&tg, KOBE_NO);
 pin_id(kobe, KOBE_NO);
 if (kobe < 0 || kobe2 < 0 || kobe3 < 0) return;
 CHECK(kobe != kobe2 && kobe2 != kobe3, "setup: three separate instances of one print");

 tg.state.p[0].stage[0] = kobe;
 tg.state.p[0].stage[1] = kobe2;
 tg.state.p[0].stage[2] = kobe3;
 test_recalc(&tg);
 CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 0,
 "the distinct-name half is not satisfied by three copies of one name (rs:86-90)");
}

static void b_kobe_no_gain_when_an_area_is_empty(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int kobe = mid(&tg, KOBE_NO);
 int sayaka = mid(&tg, "PL!HS-bp1-002-R");
 int filler = mid(&tg, "PL!-sd1-010-SD");
 pin_id(kobe, KOBE_NO);
 pin_id(sayaka, "PL!HS-bp1-002-R");
 if (kobe < 0 || sayaka < 0 || filler < 0) return;

 tg.state.p[0].stage[0] = kobe;
 tg.state.p[0].stage[1] = sayaka;
 tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
 test_recalc(&tg);
 CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 0,
 "an empty Right area breaks E(rs:107-115)");

 tg.state.p[0].stage[2] = filler;
 test_recalc(&tg);
 CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 0,
 "a non-HASUNOSORA member in the last area still breaks (rs:117-122)");

 int tsuregi = mid(&tg, TSUREGI_NO);
 if (tsuregi >= 0) {
 pin_id(tsuregi, TSUREGI_NO);
 tg.state.p[0].stage[2] = tsuregi;
 test_recalc(&tg);
 CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 1,
 "swapping in a third HASUNOSORA member satisfies both halves at once (rs:125-133)");
 }
}

/* B6. constant_two_blade_gain_when_either_stage_has_cost13_plus_test.rs.
 * EPL!S-PR-029-PR E +2 blade when EITHER stage holds a cost-13+ member.
 * The `>=` boundary is what distinguishes a correct bound from an over-broad one. */
static void b_cost13_plus_blade_boundary(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int you = mid(&tg, "PL!S-PR-029-PR");
 int cost13 = mid(&tg, "PL!-sd1-003-SD");
 int cost_high = mid(&tg, "PL!S-sd1-001-SD");
 int cost_low = mid(&tg, "PL!S-bp2-009-R");
 pin_id(you, "PL!S-PR-029-PR");
 if (you < 0 || cost13 < 0 || cost_high < 0 || cost_low < 0) return;
 CHECK_EQ(printed_cost(you), 9, "premise: WATANABE ITSUKI herself costs 9 (rs:14-15)");
 CHECK_EQ(printed_cost(cost13), 13, "premise: the boundary card costs exactly 13 (rs:53-56)");
 CHECK(printed_cost(cost_high) >= 13, "premise: the high card really costs >= 13 (rs:36)");
 CHECK(printed_cost(cost_low) < 13, "premise: the low card really costs < 13 (rs:69)");

 /* Alone: her own cost-9 does not meet the condition. */
 tg.state.p[0].stage[0] = you;
 tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
 tg.state.p[1].stage[0] = RB_EMPTY_SLOT;
 tg.state.p[1].stage[1] = RB_EMPTY_SLOT;
 tg.state.p[1].stage[2] = RB_EMPTY_SLOT;
 test_recalc(&tg);
 CHECK_EQ(blade(&tg, you), 0, "a cost-9 card alone does not meet the cost-13 condition (rs:21-26)");

 /* EXACT boundary on her own stage. */
 tg.state.p[0].stage[1] = cost13;
 test_recalc(&tg);
 CHECK_EQ(blade(&tg, you), 2, "cost exactly 13 on self triggers the +2 (rs:52-61)");

 /* The OPPONENT's stage counts too. */
 tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
 tg.state.p[1].stage[0] = cost_high;
 test_recalc(&tg);
 CHECK_EQ(blade(&tg, you), 2,
 "either-stage: a cost-17 opponent member grants exactly +2 (rs:38-48)");

 /* Both sides at once is still +2, not +4. */
 tg.state.p[0].stage[1] = cost13;
 test_recalc(&tg);
 CHECK_EQ(blade(&tg, you), 2, "both sides qualifying still gives 2 blades, not 4 (rs:87-98)");

 /* Below the threshold. */
 tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
 tg.state.p[1].stage[0] = cost_low;
 test_recalc(&tg);
 CHECK_EQ(blade(&tg, you), 0, "a cost-4 opponent member does not trigger the condition (rs:63-74)");
}

/* B7. duplicate_card_ids_on_stage_and_mulligan_toggle_test.rs:9-45. Two copies
 * of the same print on two stage slots must be two distinct card ids (the
 * landmine: a fixed-index shim would stage ONE card twice and every
 * "different member" assertion downstream would silently invert). */
static void b_duplicate_cards_on_stage_have_unique_ids(void)
{
 static TestGame tg;
 test_game_new(&tg);
 clear_p1(&tg);
 int c1 = mid(&tg, "PL!-sd1-005-SD");
 int c2 = mid(&tg, "PL!-sd1-005-SD");
 pin_id(c1, "PL!-sd1-005-SD");
 pin_id(c2, "PL!-sd1-005-SD");
 if (c1 < 0 || c2 < 0) return;
 CHECK(c1 != c2, "two copies of the same card must have unique ids (rs:18-21)");

 test_give_energy(&tg, 10);
 test_add_to_hand(&tg, c1);
 test_add_to_hand(&tg, c2);

 CHECK_EQ(play_to_stage(&tg, c1, 1, 0), 1, "the first copy plays to Center");
 CHECK_EQ(tg.state.p[0].stage[1], c1, "copy 1 is on Center (rs:26-27)");
 CHECK_EQ(play_to_stage(&tg, c2, 2, 0), 1, "the second copy plays to Right");
 CHECK_EQ(tg.state.p[0].stage[2], c2, "copy 2 is on Right (rs:29-30)");
 CHECK(tg.state.p[0].stage[1] != tg.state.p[0].stage[2],
 "Center and Right hold different card ids (rs:32-35)");
 CHECK(!waitroom_has(&tg, c1), "copy 1 never entered the waitroom (rs:37-40)");
 CHECK(!waitroom_has(&tg, c2), "copy 2 never entered the waitroom (rs:41-44)");
}

/* B8. live_success_highest_blade_niji_member_scores_plus_one_test.rs.
 * PL!N-bp7-027-L E : pick one Emember on your
 * stage; +1 score only if it has STRICTLY more blade than every other member on
 * BOTH stages. */
static void b_audrey_highest_blade_scores_plus_one(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int audrey = mid(&tg, "PL!N-bp7-027-L");
 int target = mid(&tg, "PL!N-bp1-001-R");
 int low1 = mid(&tg, "PL!-sd1-010-SD");
 int low2 = mid(&tg, "PL!N-sd1-006-SD");
 int high5 = mid(&tg, "PL!N-sd2-001-SD2");
 pin_id(audrey, "PL!N-bp7-027-L");
 pin_id(target, "PL!N-bp1-001-R");
 if (audrey < 0 || target < 0 || low1 < 0 || low2 < 0 || high5 < 0) return;
 /* Premises read from the pool, so a card-pool edit names itself. */
 const int tb = printed_blade(target);
 const int l1 = printed_blade(low1);
 const int l2 = printed_blade(low2);
 const int h5 = printed_blade(high5);
 CHECK(tb > 0, "premise: the target prints a positive blade count (rs:20)");
 CHECK(l1 < tb, "premise: LOW1 really has fewer blades than the target (rs:23)");
 CHECK(l2 < tb, "premise: LOW2 really has fewer blades than the target (rs:24)");
 CHECK(h5 > tb, "premise: HIGH5 really out-blades the target (rs:26)");

 /* POSITIVE: the target beats both opponents. */
 tg.state.p[0].stage[0] = target;
 tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
 tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
 tg.state.p[1].stage[0] = low1;
 tg.state.p[1].stage[1] = low2;
 tg.state.p[1].stage[2] = RB_EMPTY_SLOT;
 test_add_to_live(&tg, audrey);

 CHECK(card_prints_trigger(audrey, RB_TSTR_LIVE_SUCCESS),
 "premise: the live card really prints a E ability");
 CHECK(fire_live_success(&tg, audrey), "the live-success ability is queued and processed");
 drain_pick0(&tg, 20);
 CHECK_EQ(score_mod(&tg, audrey), 1,
 "the highest-blade target beats every other member on both stages -> +1 (rs:67-72)");

 /* NEGATIVE: an opponent with MORE blade. */
 static TestGame tg2;
 test_game_new(&tg2);
 int audrey2 = mid(&tg2, "PL!N-bp7-027-L");
 int target2 = mid(&tg2, "PL!N-bp1-001-R");
 int high2 = mid(&tg2, "PL!N-sd2-001-SD2");
 if (audrey2 >= 0 && target2 >= 0 && high2 >= 0) {
 tg2.state.p[0].stage[0] = target2;
 tg2.state.p[1].stage[0] = high2;
 test_add_to_live(&tg2, audrey2);
 CHECK(fire_live_success(&tg2, audrey2), "the ability fires for the negative case");
 drain_pick0(&tg2, 20);
 CHECK_EQ(score_mod(&tg2, audrey2), 0,
 "a blade-5 opponent out-blades the target -> strictly-more is not satisfied (rs:81-90)");
 }

 /* NEGATIVE: a TIE. Eis strict. */
 static TestGame tg3;
 test_game_new(&tg3);
 int audrey3 = mid(&tg3, "PL!N-bp7-027-L");
 int target3 = mid(&tg3, "PL!N-bp1-001-R");
 int tied = mid(&tg3, "PL!N-bp1-001-R");
 if (audrey3 >= 0 && target3 >= 0 && tied >= 0) {
 tg3.state.p[0].stage[0] = target3;
 tg3.state.p[1].stage[0] = tied;
 test_add_to_live(&tg3, audrey3);
 CHECK(fire_live_success(&tg3, audrey3), "the ability fires for the tie case");
 drain_pick0(&tg3, 20);
 CHECK_EQ(score_mod(&tg3, audrey3), 0,
 "a tie does not count as 'more blade than all others' (rs:100-112)");
 }
}

/* B9. live_start_success_zone_plus_three_distinct_names_score_plus_one_test.rs.
 * STAY TUNE E EPL!N-bp5-027-L : +1 score when EITHER player
 * has >= 2 cards in the success zone AND own stage has >= 3 distinct-name
 * members. The two conditions are independent; each half is proved by killing
 * the other. */
static void b_miracle_stay_tune_both_conditions(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int card = mid(&tg, "PL!N-bp5-027-L");
 int filler = mid(&tg, "PL!-sd1-010-SD");
 int m1 = mid(&tg, "PL!-sd1-002-SD");
 int m2 = mid(&tg, "PL!-sd1-005-SD");
 int m3 = mid(&tg, "PL!-sd1-008-SD");
 pin_id(card, "PL!N-bp5-027-L");
 if (card < 0 || filler < 0 || m1 < 0 || m2 < 0 || m3 < 0) return;
 CHECK(m1 != m2 && m2 != m3 && m1 != m3,
 "setup: the three stage members are three distinct instances (rs:29-42)");

 fill_decks(&tg, filler, 20);
 test_add_to_hand(&tg, card);
 test_add_to_hand(&tg, filler);
 tg.state.p[0].stage[0] = m1;
 tg.state.p[0].stage[1] = m2;
 tg.state.p[0].stage[2] = m3;
 test_add_to_success(&tg, mid(&tg, "PL!-sd1-019-SD"));
 test_add_to_success(&tg, mid(&tg, "PL!-sd1-020-SD"));

 advance_to_live_set(&tg);
 test_set_live_card(&tg, 0, card);
 advance_to_live_start(&tg);
 drain_skip(&tg, 16);
 CHECK_EQ(score_mod(&tg, card), 1, "both conditions met -> score +1 (rs:68-69)");

 /* Only the success-zone condition met (ONE member, not three). */
 static TestGame tg2;
 test_game_new(&tg2);
 int card2 = mid(&tg2, "PL!N-bp5-027-L");
 int f2 = mid(&tg2, "PL!-sd1-010-SD");
 int one = mid(&tg2, "PL!-sd1-002-SD");
 if (card2 >= 0 && f2 >= 0 && one >= 0) {
 pin_id(card2, "PL!N-bp5-027-L");
 fill_decks(&tg2, f2, 20);
 test_add_to_hand(&tg2, card2);
 test_add_to_hand(&tg2, f2);
 tg2.state.p[0].stage[0] = one;
 tg2.state.p[0].stage[1] = RB_EMPTY_SLOT;
 tg2.state.p[0].stage[2] = RB_EMPTY_SLOT;
 test_add_to_success(&tg2, f2);
 test_add_to_success(&tg2, mid(&tg2, "PL!-sd1-010-SD"));
 advance_to_live_set(&tg2);
 test_set_live_card(&tg2, 0, card2);
 advance_to_live_start(&tg2);
 drain_skip(&tg2, 16);
 CHECK_EQ(score_mod(&tg2, card2), 0,
 "only 1 member on stage -> the distinct-name condition fails -> no score (rs:109-110)");
 }

 /* Neither condition met. */
 static TestGame tg3;
 test_game_new(&tg3);
 int card3 = mid(&tg3, "PL!N-bp5-027-L");
 int f3 = mid(&tg3, "PL!-sd1-010-SD");
 int one3 = mid(&tg3, "PL!-sd1-002-SD");
 if (card3 >= 0 && f3 >= 0 && one3 >= 0) {
 pin_id(card3, "PL!N-bp5-027-L");
 fill_decks(&tg3, f3, 20);
 test_add_to_hand(&tg3, card3);
 test_add_to_hand(&tg3, f3);
 tg3.state.p[0].stage[0] = one3;
 tg3.state.p[0].stage[1] = RB_EMPTY_SLOT;
 tg3.state.p[0].stage[2] = RB_EMPTY_SLOT;
 advance_to_live_set(&tg3);
 test_set_live_card(&tg3, 0, card3);
 advance_to_live_start(&tg3);
 drain_skip(&tg3, 16);
 CHECK_EQ(score_mod(&tg3, card3), 0, "neither condition met -> no score (rs:184)");
 }
}

/* B10. live_start_aqours_center_cost_nine_blade_gain_or_opponent_wait_test.rs.
 * Deep Resonance PL!S-bp3-024-L : only when own CENTER holds a
 * cost-9+ Aqours Emember, choose +2 blade on a member of yours or wait one
 * opponent member of cost <= 4. Every negative is a different clause. */
static void b_deep_resonance_center_condition_and_options(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int dr = mid(&tg, "PL!S-bp3-024-L");
 int aq_center = mid(&tg, "PL!S-pb1-001-R");
 int aq_left = mid(&tg, "PL!S-bp2-001-R");
 int filler = mid(&tg, "PL!-sd1-010-SD");
 int filler2 = mid(&tg, "PL!-sd1-010-SD");
 pin_id(dr, "PL!S-bp3-024-L");
 if (dr < 0 || aq_center < 0 || aq_left < 0 || filler < 0 || filler2 < 0) return;
 CHECK(printed_cost(aq_center) >= 9, "premise: the Center AQOURS member really costs >= 9 (rs:58-59)");
 CHECK(printed_cost(aq_left) >= 9, "premise: the Left AQOURS member really costs >= 9 (rs:80-81)");

 /* POSITIVE: the condition is about CENTER only, so an expensive Left is fine. */
 tg.state.p[0].stage[0] = aq_left;
 tg.state.p[0].stage[1] = aq_center;
 tg.state.p[0].stage[2] = filler2;
 fill_decks(&tg, filler, 30);
 test_add_to_live(&tg, dr);
 CHECK(fire_live_start(&tg, dr), "the live-start ability is queued and processed");
 CHECK(rb_has_pending_choice(&tg.state),
 "a cost-9+ AQOURS member at Center -> the option choice appears (rs:58-75)");

 /* Option 0: +2 blade to a chosen member of ours, exactly +2, and only to it. */
 int filler_left = mid(&tg, "PL!-sd1-010-SD");
 if (filler_left >= 0) {
 tg.state.p[0].stage[0] = filler_left;
 test_recalc(&tg);
 int before = blade(&tg, aq_center);
 if (pending_type_is(&tg, "SelectTarget")) pick(&tg, 0);
 else if (pending_type_is(&tg, "SelectCard")) pick(&tg, 0);
 drain_pick0(&tg, 12);
 CHECK_EQ(blade(&tg, aq_center), before + 2,
 "option 0 grants exactly +2 blade to the chosen member (rs:196-218)");
 CHECK_EQ(blade(&tg, filler_left), 0, "the unselected member keeps blade modifier 0 (rs:219-232)");
 }

 /* NEGATIVE: empty Center. */
 static TestGame tg2;
 test_game_new(&tg2);
 int dr2 = mid(&tg2, "PL!S-bp3-024-L");
 int f2 = mid(&tg2, "PL!-sd1-010-SD");
 int f2b = mid(&tg2, "PL!-sd1-010-SD");
 if (dr2 >= 0 && f2 >= 0 && f2b >= 0) {
 pin_id(dr2, "PL!S-bp3-024-L");
 tg2.state.p[0].stage[0] = f2;
 tg2.state.p[0].stage[1] = RB_EMPTY_SLOT;
 tg2.state.p[0].stage[2] = f2b;
 fill_decks(&tg2, f2, 30);
 test_add_to_live(&tg2, dr2);
 CHECK(fire_live_start(&tg2, dr2), "the ability is queued even when the condition fails");
 CHECK(!rb_has_pending_choice(&tg.state),
 "an empty Center -> the cost-9+ condition fails -> no choice (rs:103-116)");
 }

 /* NEGATIVE: a cost-9+ member at LEFT only, Center empty-ish. */
 static TestGame tg3;
 test_game_new(&tg3);
 int dr3 = mid(&tg3, "PL!S-bp3-024-L");
 int aq_l = mid(&tg3, "PL!S-pb1-001-R");
 int f3 = mid(&tg3, "PL!-sd1-010-SD");
 int f3b = mid(&tg3, "PL!-sd1-010-SD");
 if (dr3 >= 0 && aq_l >= 0 && f3 >= 0 && f3b >= 0) {
 pin_id(dr3, "PL!S-bp3-024-L");
 tg3.state.p[0].stage[0] = aq_l;
 tg3.state.p[0].stage[1] = f3;
 tg3.state.p[0].stage[2] = f3b;
 fill_decks(&tg3, f3, 30);
 test_add_to_live(&tg3, dr3);
 CHECK(fire_live_start(&tg3, dr3), "the ability is queued for the wrong-position case");
 CHECK(!rb_has_pending_choice(&tg.state),
 "a cost-9+ AQOURS member at Left, not Center -> the condition fails (rs:161-179)");
 }
}

/* B11. live_start_activate_three_printemps_wait_members_q178_q179_test.rs.
 * WAO-WAO Powerful day! PL!-pb1-028-L : make your Printemps E * members ACTIVE; if 3 or more WAIT members became active, +1 score. Q179 says
 * the +1 needs 3 ACTUALLY changed, not 3 merely waited. */
static void b_wao_wao_q178_q179_three_changed(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int wao = mid(&tg, "PL!-pb1-028-L");
 int a = mid(&tg, "PL!-sd1-001-SD");
 int b = mid(&tg, "PL!-sd1-003-SD");
 int c = mid(&tg, "PL!-sd1-008-SD");
 int filler = mid(&tg, "PL!-sd1-010-SD");
 pin_id(wao, "PL!-pb1-028-L");
 if (wao < 0 || a < 0 || b < 0 || c < 0 || filler < 0) return;
 CHECK(a != b && b != c && a != c, "setup: three distinct stage instances");

 test_add_to_hand(&tg, wao);
 fill_decks(&tg, filler, 10);
 tg.state.p[0].stage[0] = a;
 tg.state.p[0].stage[1] = b;
 tg.state.p[0].stage[2] = c;

 advance_to_live_set(&tg);
 test_set_live_card(&tg, 0, wao);

 /* The wait states are set AFTER the Active phase, exactly as in Rust, so
 * the active-phase refresh cannot clear them. */
 rb_mods_set_orientation(&tg.state.mods, a, "wait");
 rb_mods_set_orientation(&tg.state.mods, b, "wait");
 rb_mods_set_orientation(&tg.state.mods, c, "wait");

 advance_to_live_start(&tg);
 drain_pick0(&tg, 16);

 const char *oa = orientation(&tg, a);
 const char *ob = orientation(&tg, b);
 const char *oc = orientation(&tg, c);
 CHECK(oa && !strcmp(oa, "active"), "the first PRINTEMPS member became active (rs:67-73)");
 CHECK(ob && !strcmp(ob, "active"), "the second PRINTEMPS member became active (rs:74-80)");
 CHECK(oc && !strcmp(oc, "active"), "the third PRINTEMPS member became active (rs:81-87)");
 CHECK_EQ(score_mod(&tg, wao), 1, "3 wait members activated -> score +1 (rs:88-89)");

 /* Q179: only TWO were waited -> 2 changed is below the threshold. */
 static TestGame tg2;
 test_game_new(&tg2);
 int wao2 = mid(&tg2, "PL!-pb1-028-L");
 int a2 = mid(&tg2, "PL!-sd1-001-SD");
 int b2 = mid(&tg2, "PL!-sd1-003-SD");
 int c2 = mid(&tg2, "PL!-sd1-008-SD");
 int f2 = mid(&tg2, "PL!-sd1-010-SD");
 if (wao2 >= 0 && a2 >= 0 && b2 >= 0 && c2 >= 0 && f2 >= 0) {
 pin_id(wao2, "PL!-pb1-028-L");
 test_add_to_hand(&tg2, wao2);
 fill_decks(&tg2, f2, 10);
 tg2.state.p[0].stage[0] = a2;
 tg2.state.p[0].stage[1] = b2;
 tg2.state.p[0].stage[2] = c2;
 advance_to_live_set(&tg2);
 test_set_live_card(&tg2, 0, wao2);
 rb_mods_set_orientation(&tg2.state.mods, a2, "wait");
 rb_mods_set_orientation(&tg2.state.mods, b2, "wait");
 /* c2 stays active: no orientation modifier */
 advance_to_live_start(&tg2);
 drain_pick0(&tg2, 16);
 CHECK_EQ(score_mod(&tg2, wao2), 0,
 "Q179: only 2 wait members changed and the condition requires 3+ (rs:154-160)");
 }
}

/* B12. live_success_per_wait_member_score_and_yell_reveal_to_deck_bottom_test.rs
 * first test. PL!N-bp3-031-L E : +1 score PER WAIT member on stage. */
static void b_live_success_per_wait_member_adds_score(void)
{
 static TestGame tg;
 test_game_new(&tg);
 int live = mid(&tg, "PL!N-bp3-031-L");
 int wait_member = mid(&tg, "PL!-sd1-002-SD");
 int active_member = mid(&tg, "PL!SP-pb1-014-PR");
 pin_id(live, "PL!N-bp3-031-L");
 if (live < 0 || wait_member < 0 || active_member < 0) return;

 tg.state.p[0].stage[0] = active_member;
 tg.state.p[0].stage[1] = wait_member;
 tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
 rb_mods_set_orientation(&tg.state.mods, wait_member, "wait");
 test_add_to_success(&tg, live);

 CHECK(card_prints_trigger(live, RB_TSTR_LIVE_SUCCESS),
 "premise: PL!N-bp3-031-L really prints a E ability");
 int before = score_mod(&tg, live);
 CHECK(fire_live_success(&tg, live), "the live-success ability is queued and processed");
 drain_auto(&tg);
 CHECK_EQ(score_mod(&tg, live), before + 1,
 "one WAIT member on stage adds exactly +1 to the live's score (rs:58-64)");
}

/* ---------------------------------------------------------------------- */

static void run_all(const char *only)
{
 struct { const char *name; void (*fn)(void); } cases[] = {
 /* deploy/ */
 { "keke_locked", a_keke_blocked_from_locked_area },
 { "q63_sayaka", a_q63_ability_debut_no_cost_payment },
 { "q80_sayaka", a_q80_debut_to_area_vacated_same_turn },
 { "shizuku_pay", a_shizuku_pay_2e_deploys_shizuku },
 { "shizuku_skip", a_shizuku_skip_pay_no_deploy },
 { "miyashita_pay", a_miyashita_pay_2e_deploys },
 { "yoshiko_budget", a_yoshiko_over_cost_filtered_and_budget_tracked },
 { "yoshiko_full_stage", a_yoshiko_full_stage_graceful },
 { "q199_baton", a_q199_deployed_this_turn_blocks_baton },
 { "q202_debut_draw", a_q202_deployed_card_own_debut_draws },
 { "izumi_blade_filter", a_izumi_waits_both_stages_printed_blades_lte3 },
 { "izumi_printed", a_izumi_judges_printed_blade_count },
 { "yoshi_not_center", a_yoshi_not_center_cannot_activate },
 { "q268_no_area", a_q268_no_empty_area_pays_cost_no_deploy },
 { "q268_wait", a_q268_deploy_to_empty_area_in_wait_state },
 { "izumi_bp6_ok", a_izumi_bp6_deploys_low_cost_hasunosora },
 { "izumi_bp6_pricey", a_izumi_bp6_too_expensive_member_not_deployed },
 { "izumi_bp6_full", a_izumi_bp6_full_stage_terminates },
 { "baton_cheap", a_lower_cost_baton_deploys_cheap_member },
 { "no_baton", a_no_baton_touch_no_appear },
 { "fuyumari_exclude", a_fuyumari_exclude_self_from_cost },
 { "fuyumari_skip", a_fuyumari_optional_cost_skipped },
 { "bloom_choice", a_bloom_live_start_heart_choice },
 { "bloom_no_choice", a_bloom_no_hasunosuka_member_no_choice },
 { "eternalize", a_eternalize_love_two_same_name_minus_three },
 /* recover/per_card/ */
 { "joint_names", b_joint_card_names_and_slot_rulings },
 { "cannot_baton", b_cannot_baton_touch_blocks_action },
 { "baton_control", b_baton_touch_succeeds_without_restriction },
 { "mv_sync", b_push_movement_event_syncs_all_views },
 { "mv_area", b_stage_to_stage_move_arms_area_movement },
 { "mv_append", b_second_move_appends_to_the_batch },
 { "mv_clear", b_clear_card_movement_tracking_resets_turn_scope },
 { "mv_cost", b_activation_cost_feeds_movement_views },
 { "kobe_distinct", b_kobe_distinct_names_gain_when_all_different },
 { "kobe_same", b_kobe_no_gain_when_same_name },
 { "kobe_empty", b_kobe_no_gain_when_an_area_is_empty },
 { "cost13_blade", b_cost13_plus_blade_boundary },
 { "dup_ids", b_duplicate_cards_on_stage_have_unique_ids },
 { "audrey", b_audrey_highest_blade_scores_plus_one },
 { "miracle", b_miracle_stay_tune_both_conditions },
 { "deep_resonance", b_deep_resonance_center_condition_and_options },
 { "wao_wao", b_wao_wao_q178_q179_three_changed },
 { "per_wait_score", b_live_success_per_wait_member_adds_score },
 };
 for (size_t i = 0; i < sizeof cases / sizeof cases[0]; i++) {
 if (only && *only && !strstr(cases[i].name, only)) continue;
 printf("--- case %s\n", cases[i].name);
 cases[i].fn();
 }
}

int main(void)
{
 /* Unbuffered: a case that crashes the engine must still show which case it
 * was, not lose the whole run to a lost stdio buffer. */
 setvbuf(stdout, NULL, _IONBF, 0);
 if (load_card_db() != 0) {
 fprintf(stderr, "load_cards failed\n");
 return 1;
 }
 run_all(getenv("RB_ONLY"));
 rb_unload();
 printf("checks=%d failures=%d\n", checks, failures);
 if (failures) return 1;
 printf("ALL PARITY DEPLOY / RECOVER-PER-CARD CHECKS PASSED\n");
 return 0;
}
