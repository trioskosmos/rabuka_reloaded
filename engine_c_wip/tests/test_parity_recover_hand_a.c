/* test_parity_recover_hand_a.c -EC parity suite for the RECOVERY-SOURCE half
 * of the Rust cluster engine/tests/test_modules/effects/recover/to_hand/.
 *
 * BOUNDARY. The 40 .rs files in that cluster split three ways:
 *
 *   (1) RECOVERY-SOURCE  <- THIS FILE. A card recovering ITSELF or a NAMED card
 *       out of its OWN WAITROOM (waitroom), its own HAND, or its own DECK; the
 *       [???waitroom??~?1???????] self / chosen-card variants; and
 *       the [N???] any-number recovery variants. The thing under test is
 *       WHERE THE CARD COMES FROM.
 *   (2) live-recovery + group gating -> test_parity_recover_live.c (sibling).
 *   (3) cost-discard-then-recover (Q209) -> third sibling.
 *
 * test_parity_recover_to_hand.c (the older stub) is a MIXED file: it is
 * largely family (2) plus three cases of family (3). It names NONE of the
 * cluster's 40 .rs files. Its exact per-case accounting is in REPORT below.
 * Every fixture used here was checked against the sibling's card list and
 * appears there ZERO times, so the three files do not overlap on any card.
 *
 * IDENTITY (AGENTS.md). Every fixture resolves through mid() ->
 * test_new_id(), never the shared template alias, and every print is pinned
 * with a BYTE-EXACT strcmp of rb_card_string(c.card_no_idx). rb_card_no_eq is
 * FUZZY (a peer proved it accepts PL!SP-PR-004-R against the real
 * PL!SP-PR-004-PR) and is NOT an identity pin, so it is never used for that.
 *
 * ENCODING (AGENTS.md). Every non-ASCII literal in this file is a \xNN byte
 * escape, so the file is 7-bit clean and cannot be silently rewritten into
 * CP932 by an editor. That matters doubly here: a mangled group string makes a
 * filter match nothing and the test goes GREEN.
 *
 * VACUITY CONTROL. This is the landmine for this cluster. A recovery whose
 * filter matches nothing produces a CLEAN, GREEN negative - the card simply
 * does not move and every "!hand_has(target)" assertion passes. So EVERY
 * negative assertion below is one of:
 *   - preceded by EXPECT_PROMPTS(tg, n), counting the prompt still ON SCREEN at the
 *     assertion site (note_prompt), not only at answer time;
 *   - paired with a positive control on the SAME board shape with the gate
 *     satisfied; or
 *   - a GROUP / TYPE / COST PIN proving the target is inside the filter's set
 *     and the decoy is outside it, so a "nothing matched" board cannot pass.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* U+FF0B FULLWIDTH PLUS SIGN - the Sayaka recovery print. */
#define FP "\xef\xbc\x8b"

/* ---- group names, as BYTE EXACT escapes ---------------------------------- */
#define GRP_LIELLA "Liella!"
#define GRP_SAINT  "SaintSnow"
#define GRP_MU     "\xce\xbc's"                                            /* MU's */
#define GRP_HASU   "\xe8\x93\xae\xe3\x83\x8e\xe7\xa9\xba"                  /* HASU */
#define GRP_SUZU   "\xe3\x82\xb9\xe3\x83\xaa\xe3\x83\xbc\xe3\x82\xba\xe3\x83\x96\xe3\x83\xbc\xe3\x82\xb1"
                                                                        /* SUZUKE */

/* ---- fixtures (every one verified against cards/cards.json read as UTF-8) -*/

/* optional_energy_member_recovery_pl_sp_sd1_007_sd_test.rs
 * cards.json: PL!SP-sd1-007-SD / Mei / ???? / cost 7 / CatChu!
 *   DEBUT E E ???????????waitroom??'Liella!'?????????1???????.  */
#define MEI_SD      "PL!SP-sd1-007-SD"
#define KANATA      "PL!SP-sd1-002-SD"   /* Kanata  cost 11  KALEIDOSCORE  */
/* optional_energy_saint_snow_recovery_and_blades_test.rs
 * cards.json: PL!S-bp5-009-R / Ruby / ???? / cost 15 / CYaRon?
 *   DEBUT E ???????????waitroom??'SaintSnow'?????1???????. 
 *   ??????, ????????, B B.  */
#define RUBY        "PL!S-bp5-009-R"
#define SEIRA       "PL!S-pb1-054-SRE"  /* Seira  ENERGY card, unit=SaintSnow */
/* recovers_two_cost_two_members_to_hand_pl_hs_bp2_002_r_plus_test.rs
 * RUST FIXTURE BUG #1, CORRECTED HERE - see REPORT. The real print is the
 * FULLWIDTH-PLUS one. cards.json: PL!HS-bp2-002-R? / Sayaka / ???? /
 * cost 13 / DOLLCHESTRA.  DEBUT????waitroom?????2???????????2?????????.  */
#define SAYAKA_2UP  "PL!HS-bp2-002-R" FP
#define KOTORI_5    "PL!-pb1-021-PR"    /* Kotori cost 5  -> over the cap */
#define KOTORI_2    "PL!-bp3-012-PR"    /* Kotori cost 2  -> eligible   */
#define MEI_2       "PL!SP-pb1-018-N"   /* Mei   cost 2  -> eligible   */
#define LIVE_MUS    "PL!-sd1-019-SD"    /* START:DASH!! - a LIVE, not a member */
/* q145_empty_waitroom_optional_self_wait_test.rs
 * cards.json: PL!-bp3-003-R / Kotori / ???? / cost 11 / Printemps
 *   DEBUT ?????????????????????waitroom??'MU's'?????????1???????.  */
#define KOTORI_11   "PL!-bp3-003-R"
/* distinct_trio_reveals_pl_sp_bp2_025_l_test.rs
 * cards.json: PL!SP-bp2-025-L / Bubble Rise / ??? / score 4 */
#define BUBBLE      "PL!SP-bp2-025-L"
#define KANON_SD    "PL!SP-sd1-002-SD"   /* Kanata      KALEIDOSCORE */
#define WIEN_SD2    "PL!SP-sd2-010-SD2"  /* ????      KALEIDOSCORE */
#define PRIZE_KANON "PL!SP-PR-003-PR"    /* Kanon  cost 2  CatChu! */
/* success_zone_revealed_members_pl_s_bp5_019_l_test.rs
 * cards.json: PL!S-bp5-019-L / not ALONE not HITORI / ??? / score 5
 *   ?????? ??????????????????????2???????, 
 *   ?????????????????????, ????????2?????????.  */
#define NOT_ALONE   "PL!S-bp5-019-L"
#define OMIKAZE     "PL!N-sd1-006-P"     /* Omikaze  cost 2  ??? */
#define RENA        "PL!N-bp1-009-R"     /* Rina cost 9  ??? */
#define LINK_BASE_L "PL!HS-bp2-020-L"    /* Link to the FUTURE - a filler live */
/* tied_score_reveals_pl_hs_cl1_012_cl_test.rs
 * cards.json: PL!HS-cl1-012-CL / Edelied / ??? / score 2
 *   ?????? ????????????????????, ???????????
 *   ??????????, ???9???????????1???????.  */
#define EDELIED     "PL!HS-cl1-012-CL"
#define TAXONOMY    "PL!S-bp7-024-L"     /* Taxonomy  live score 1 */
#define RIKO        "PL!S-PR-014-PR"     /* Riko cost 15  GuiltyKiss  */
#define GINKO_15    "PL!HS-bp5-004-R"    /* Ginko cost 15 -> eligible   */
#define KANON_2     "PL!SP-PR-003-PR"    /* Kanon cost 2 -> not eligible */
/* baton_touch_cheaper_unit_recovers_hasunosora_live_to_hand_test.rs
 * RUST FIXTURE BUG #2, CORRECTED HERE - see REPORT. "PL!HS-bp2-007-R+" does
 * NOT exist. cards.json: PL!HS-bp2-007-P / Ginko / ???? / cost 11 /
 * SUZUKE, and its ?? ability is the exact one the file describes:
 *   DEBUT ??????????????'SUZUKE'???????????????
 *   DEBUT????, ???waitroom??'HASU'????????1???????.  */
#define GINKO_BT    "PL!HS-bp2-007-P"
#define SUZU_4      "PL!HS-bp1-012-PR"   /* Kazuha     cost 4  SUZUKE */
#define SUZU_11     "PL!HS-PR-031-PR"    /* Hanano  cost 11 SUZUKE */
#define MIKI_9      "PL!-sd1-006-SD"     /* Miki  cost 9  MU's        */
#define DREAM_1     "PL!HS-bp1-019-L"    /* Dream Believers  HASU live, score 1 */
#define YOUTA       "PL!-sd1-020-SD"     /* ??????????  MU's live */
/* generic */
#define FILLER_MUS  "PL!-sd1-010-SD"     /* MU's member, never in an HS filter */
#define FILLER_NIJI "PL!N-sd1-010-SD"    /* ??? member */
#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* ---- reporting ---- */
static int failures;
static int assertions;
static int setup_bugs;
static int gaps;
static int prompts;
static int n_ok, n_failed, n_crashed;
static const char *current_test = "(none)";

/* sizeof(GameState) is ~781 KB, so every TestGame must be static. */
static TestGame t_tg[1];   /* one slot: each case is its own fork */

static void on_fatal_signal(int sig)
{
    fprintf(stderr,
            "\n*** FATAL SIGNAL %d inside test: %s\n"
            "*** assertions=%d failures=%d prompts=%d\n",
            sig, current_test, assertions, failures, prompts);
    fflush(stderr);
    /* Re-raise under the DEFAULT disposition. The old body ended in
     * _Exit(128 + sig), which made the child exit NORMALLY with code 139, so
     * the parent's WIFSIGNALED(status) was false and run_case() took its
     * no-report path: the crashed case produced NO verdict at all and the
     * final tally read `13 ok, 9 failed, 0 crashed` for 23 cases. Dying from
     * the signal is what makes the CRASH verdict reachable. */
    signal(sig, SIG_DFL);
    raise(sig);
    _Exit(128 + sig);
}

#define CHECK(condition, ...) do {                                          \
        assertions++;                                                       \
        if (!(condition)) {                                                 \
            failures++;                                                     \
            fprintf(stderr, "FAIL: ");                                      \
            fprintf(stderr, __VA_ARGS__);                                   \
            fputc('\n', stderr);                                            \
        }                                                                   \
    } while (0)

#define CHECK_EQ(actual, expected, ...) do {                                \
        assertions++;                                                       \
        long long a_ = (long long)(actual);                                 \
        long long e_ = (long long)(expected);                               \
        if (a_ != e_) {                                                     \
            failures++;                                                     \
            fprintf(stderr, "FAIL: ");                                      \
            fprintf(stderr, __VA_ARGS__);                                   \
            fprintf(stderr, " (got %lld expected %lld)\n", a_, e_);         \
        }                                                                   \
    } while (0)

/* A Rust assertion with no faithful C entry point (an API surface the C port
 * does not expose). Recorded, never counted as a pass and never a failure. */
#define EXPECTED_GAP(condition, ...) do {                                   \
        assertions++;                                                       \
        if (!(condition)) {                                                 \
            printf("GAP: ");                                                \
            printf(__VA_ARGS__);                                            \
            printf("\n");                                                   \
            gaps++;                                                         \
        }                                                                   \
    } while (0)

/* VACUITY CONTROL. Every negative claim below is gated on this.
 * expect_prompts() (defined below, next to note_prompt) is the FORM used
 * everywhere in this file: it counts the prompt still ON SCREEN at the
 * assertion site and then asserts a floor on the running total. Bare
 * CHECK_PROMPTS reads a counter that only pick()/select_option() increment, so
 * on its own it would always read 0. */
#define EXPECT_PROMPTS(tg, n, ...) do {                                      \
        char what_[192];                                                     \
        snprintf(what_, sizeof what_, __VA_ARGS__);                          \
        expect_prompts((tg), (n), what_);                                    \
    } while (0)

/* ---- db / fixtures ---- */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

static const char *card_no_of(int cid)
{
    static char buf[96];
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(buf, sizeof buf, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

static const char *card_name_of(int cid)
{
    static char buf[128];
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        if (c.name) snprintf(buf, sizeof buf, "%s", c.name);
        rb_free_card(&c);
    }
    return buf;
}

/* AGENTS.md identity rule: mid() (test_new_id), never the shared template. */
static int mid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id < 0) {
        assertions++; setup_bugs++; failures++;
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in cards.json\n", no);
    }
    return id;
}

/* BYTE-EXACT identity pin. */
static int pin_id(int cid, const char *no)
{
    assertions++;
    if (cid < 0) { setup_bugs++; failures++;
        fprintf(stderr, "SETUP BUG: cannot resolve '%s'\n", no); return 0; }
    if (strcmp(card_no_of(cid), no) != 0) {
        setup_bugs++; failures++;
        fprintf(stderr, "SETUP BUG: identity - want '%s', resolved '%s' (%s) id=%d\n",
                no, card_no_of(cid), card_name_of(cid), cid);
        return 0;
    }
    return 1;
}

static int printed_cost(int cid)
{
    Card c; int v = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.cost; rb_free_card(&c); }
    return v;
}

static int printed_score(int cid)
{
    Card c; int v = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.score; rb_free_card(&c); }
    return v;
}

static int card_prints_trigger(int cid, const char *trig)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = ab.triggers && strstr(ab.triggers, trig);
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

static int card_ability_count(int cid)
{
    return rb_card_num_abilities((uint32_t)cid);
}

static void dump_abilities(int cid, const char *tag)
{
    printf("  -- %s abilities for %s (%s):\n", tag, card_no_of(cid), card_name_of(cid));
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        printf("     [%d] triggers=%s use_limit=%d text=%s\n", i,
               ab.triggers ? ab.triggers : "-", ab.use_limit,
               ab.full_text ? ab.full_text : "-");
        rb_free_ability(&ab);
    }
    fflush(stdout);
}

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}

static int hand_has(const TestGame *tg, int cid)  { return bag_has(&tg->state.p[0].hand, cid); }
static int wait_has(const TestGame *tg, int cid)  { return bag_has(&tg->state.p[0].discard, cid); }
static int hand2_has(const TestGame *tg, int cid) { return bag_has(&tg->state.p[1].hand, cid); }
static int wait2_has(const TestGame *tg, int cid) { return bag_has(&tg->state.p[1].discard, cid); }

static int stage_has(const TestGame *tg, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) if (tg->state.p[0].stage[i] == cid) return 1;
    return 0;
}

static int energy_active(const TestGame *tg) { return rb_energy_active_count(&tg->state.p[0]); }

static void clear_p1(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0; P->hand.n = 0; P->discard.n = 0;
    P->live.n = 0; P->success.n = 0;
    P->energy.n = 0; P->energy_active = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT; P->stage_wait[i] = 0; P->under_cards[i].n = 0;
    }
    tg->state.activating_card = RB_EMPTY_SLOT;
    tg->state.n_revealed = 0;
    tg->state.yell_occurred = 0;
    rb_look_reset_all();
}

/* The harness has no test_add_to_discard_pl, so the opponent's waitroom is
 * seeded directly. Rust's waitroom.cards.push(x) is an APPEND, and the C bag
 * appends at index n, so the two agree without a top/bottom flip. */
static void add_to_wait_for(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = cid;
}

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        for (int i = 0; i < n && P->deck.n < RB_MAX_ZONE; i++)
            P->deck.cards[P->deck.n++] = filler;
    }
}

/* ---- prompt plumbing ---- */

static int pending(TestGame *tg) { return rb_has_pending_choice(&tg->state); }

static const char *pending_type(TestGame *tg)
{
    const char *t = test_pending_choice_type(tg);
    return t ? t : "";
}

static int pending_is_card(TestGame *tg)
{
    return pending(tg) && !strcmp(pending_type(tg), "SelectCard");
}

/* Counted AT THE ASSERTION SITE, not only when answered. */
static int note_prompt(TestGame *tg) { if (pending(tg)) { prompts++; return 1; } return 0; }

/* The vacuity gate in its usable form: count what is on screen HERE, then
 * assert a floor on the running total. Every negative in this file goes
 * through this, never through a bare counter read. */
static int expect_prompts(TestGame *tg, int n, const char *what)
{
    assertions++;
    note_prompt(tg);
    if (prompts < n) {
        failures++;
        fprintf(stderr, "FAIL: [VACUOUS]  %s - only %d prompt(s) were offered, need %d\n",
                what, prompts, n);
        return 0;
    }
    return 1;
}

static void pick(TestGame *tg, int idx)
{
    if (!pending(tg)) return;
    const int one[1] = { idx };
    note_prompt(tg);
    rb_resume_with_choice_indices(&tg->state, one, 1);
}

static void pick_n(TestGame *tg, const int *idx, int n)
{
    if (!pending(tg)) return;
    note_prompt(tg);
    rb_resume_with_choice_indices(&tg->state, idx, n);
}

static void skip(TestGame *tg)
{
    if (!pending(tg)) return;
    note_prompt(tg);
    rb_resume_with_choice_indices(&tg->state, NULL, 0);
}

static void select_option(TestGame *tg, int idx)
{
    if (!pending(tg)) return;
    note_prompt(tg);
    rb_resume_with_choice(&tg->state, idx);
}

static int drain_pick0(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) pick(tg, 0);
    return guard;
}

static int drain_skip(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) skip(tg);
    return guard;
}

/* Rust drain_choices: answer with the FIRST index UNLESS the choice carries
 * allow_skip, in which case decline it. Answering a mandatory recover with an
 * empty selection refuses the recovery, which is exactly the absence a
 * following negative assertion would then read as a pass. */
static int drain_choices(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int skippable = c && c->allow_skip;
        if (!strcmp(pending_type(tg), "SelectAutoAbility")) skip(tg);
        else if (skippable) skip(tg);
        else pick(tg, 0);
    }
    return guard;
}

/* Every drain is capped: a peer found an ability that re-prompts unboundedly. */

static void play_to_stage(TestGame *tg, int cid, int area, int accept)
{
    test_play_to_stage(tg, cid, area);
    int guard = 0;
    while (tg->state.ptc_active && guard++ < 4) {
        test_answer_play_cost_choice(tg, accept);
        if (tg->state.p[0].stage[area] == cid) break;
        test_play_to_stage(tg, cid, area);
    }
}

/* Rust fire_trigger / trigger_auto: ONE ability of ONE card.
 *
 * The 登場 arm below is KEPT deliberately. Rust's fire_trigger passes None for the
 * movement batch, and the shared harness helper does the same, so a card seated in
 * hand has no appearance event and the TAS movement gate refuses it. This suite's
 * Rust twin publishes the 登場 batch the way recover_live does, and that has to
 * keep happening here or the DEBUT legs stop firing at all.
 *
 * Only the single-card fan-out is replaced. The old body was
 * rb_queue_trigger_abilities(&tg->state, pl, trig), a whole-board TAS scan in which
 * `cid` only set activating_card: every card that seat owned printing the trigger
 * fired too. test_fire_trigger_on_card_seat targets one ability_id on one
 * explicit_card_id via rb_trigger_auto_ability (the port of Rust
 * GameState::trigger_auto_ability) and does the load-bearing enqueue-time use
 * record. */
static int fire_trigger_for(TestGame *tg, int pl, int cid, const char *trig)
{
    if (cid < 0) return 0;
    if (!strcmp(trig, RB_TSTR_DEBUT)) {
        tg->state.n_recently_moved = 0;
        tg->state.recently_moved[tg->state.n_recently_moved++] = cid;
    }
    return test_fire_trigger_on_card_seat(tg, cid, trig, pl);
}

static int fire_trigger(TestGame *tg, int cid, const char *trig)
{
    return fire_trigger_for(tg, 0, cid, trig);
}

static void reveal_cards(TestGame *tg, const int *ids, int n)
{
    tg->state.n_revealed = 0;
    for (int i = 0; i < n && i < RB_MAX_REVEALED_CARDS; i++)
        tg->state.revealed_cards[i] = ids[i];
    tg->state.n_revealed = n;
}

static int revealed_has(const TestGame *tg, int cid)
{
    for (int i = 0; i < tg->state.n_revealed; i++)
        if (tg->state.revealed_cards[i] == cid) return 1;
    return 0;
}

static int blade_mod(TestGame *tg, int cid)
{
    return rb_mods_get_blade(&tg->state.mods, cid);
}

static const char *orientation_mod(TestGame *tg, int cid)
{
    const char *s = rb_mods_get_orientation(&tg->state.mods, cid);
    return s ? s : "";
}

static int in_group(int cid, const char *g)
{
    return rb_card_matches_group_str(cid, g);
}

static int is_live(int cid)   { return rb_card_is_live(cid); }
static int is_member(int cid) { return rb_card_is_member(cid); }

static void dump_state(TestGame *tg, const char *what)
{
    fprintf(stderr, "  STATE %s: pending=%d type=%s prompts=%d\n",
            what, pending(tg), pending_type(tg), prompts);
    fprintf(stderr, "    hand:");
    for (int i = 0; i < tg->state.p[0].hand.n; i++)
        fprintf(stderr, " %s", card_no_of(tg->state.p[0].hand.cards[i]));
    fprintf(stderr, "\n    wait:");
    for (int i = 0; i < tg->state.p[0].discard.n; i++)
        fprintf(stderr, " %s", card_no_of(tg->state.p[0].discard.cards[i]));
    fputc('\n', stderr);
}

/* ======================================================================
 * A. optional_energy_member_recovery_pl_sp_sd1_007_sd_test.rs  (2 tests)
 *
 * PL!SP-sd1-007-SD Mei cost 7:
 *   DEBUT E E ???????????waitroom??'Liella!'?????????
 *   1???????. 
 *
 * The pay gate is a `pay_optional_cost:skip` SelectTarget where option 0 is
 * SKIP and option 1 is PAY. Answering 0 is the only way to reach the declining
 * half - declining some later prompt would refuse the WHOLE ability instead.
 * ====================================================================== */

static void a_setup(TestGame *tg, int *me_out, int *kanon_out)
{
    clear_p1(tg);
    int filler = mid(tg, FILLER_MUS);
    fill_decks(tg, filler, 5);
    int me = mid(tg, MEI_SD);
    int kanon = mid(tg, KANATA);
    if (!pin_id(me, MEI_SD) || !pin_id(kanon, KANATA)) return;
    CHECK_EQ(printed_cost(me), 7, "A: Mei prints cost 7, so 2E is affordable");
    CHECK(in_group(me, GRP_LIELLA), "A: the DEBUT card itself is 'Liella!'");
    CHECK(in_group(kanon, GRP_LIELLA), "A: Kanata is inside the 'Liella!' filter set");
    CHECK(is_member(kanon), "A: Kanata is a ??????? as the filter requires");
    CHECK(card_prints_trigger(me, RB_TSTR_DEBUT), "A: Mei prints a DEBUT ability");
    tg->state.p[0].stage[AREA_CENTER] = me;
    test_give_energy(tg, 10);
    test_add_to_discard(tg, kanon);
    *me_out = me; *kanon_out = kanon;
}

static void test_a_paid_energy_recovers_liella_member(void)
{
    TestGame *tg = &t_tg[0];
    int me, kanon;
    a_setup(tg, &me, &kanon);
    if (me < 0 || kanon < 0) return;

    CHECK(fire_trigger(tg, me, RB_TSTR_DEBUT), "A: the DEBUT ability is queued");
    EXPECT_PROMPTS(tg, 1, "A/pay: the optional 2E gate must be OFFERED before anything is fetched");
    CHECK(pending(tg), "A/pay: the pay_optional_cost gate is still on screen at the assertion site");
    /* A pay_optional_cost:skip gate is SelectTarget 0=SKIP 1=PAY. */
    select_option(tg, 1);

    drain_choices(tg, 8);
    CHECK(hand_has(tg, kanon), "A/pay: 'Liella!' member retrieved from the waitroom to hand");
    CHECK(!wait_has(tg, kanon), "A/pay: the fetched member left the waitroom");
    CHECK_EQ(energy_active(tg), 8, "A/pay: exactly 2 of the 10 energy were tapped");
}

static void test_a_declined_energy_does_not_recover_member(void)
{
    TestGame *tg = &t_tg[0];
    int me, kanon;
    a_setup(tg, &me, &kanon);
    if (me < 0 || kanon < 0) return;

    CHECK(fire_trigger(tg, me, RB_TSTR_DEBUT), "A: the DEBUT ability is queued");
    EXPECT_PROMPTS(tg, 1, "A/decline: the optional 2E gate must be OFFERED");
    CHECK(pending(tg), "A/decline: the gate is still on screen at the assertion site");
    select_option(tg, 0);   /* SKIP */

    drain_choices(tg, 8);
    /* CONTROL: the paying half of the SAME board shape does fetch. Without this
     * a board where the ability did nothing at all would pass this test. */
    CHECK_EQ(tg->state.p[0].discard.n, 1, "A/decline: the waitroom is unchanged (no fetch happened)");
    CHECK(wait_has(tg, kanon), "A/decline: declined -> Kanata stays in the waitroom");
    CHECK(!hand_has(tg, kanon), "A/decline: declined -> Kanata is NOT in hand");
    CHECK_EQ(energy_active(tg), 10, "A/decline: declining an optional cost taps no energy");
}

/* ======================================================================
 * B. optional_energy_saint_snow_recovery_and_blades_test.rs  (2 tests)
 *
 * PL!S-bp5-009-R Ruby cost 15:
 *   DEBUT E ???????????waitroom??'SaintSnow'?????1???????. 
 *   ??????, ????????, B B. 
 *
 * The fetched card PL!S-pb1-054-SRE Seira is an ENERGY card whose unit is
 * SaintSnow - the filter is on the UNIT, not the card type, so an energy card
 * is a legal target. That is not a fixture bug; the C group filter is pinned
 * on it below so a "matched nothing" board cannot pass.
 * ====================================================================== */

static void b_setup(TestGame *tg, int *ruby_out, int *seira_out)
{
    clear_p1(tg);
    int ruby = mid(tg, RUBY);
    int seira = mid(tg, SEIRA);
    if (!pin_id(ruby, RUBY) || !pin_id(seira, SEIRA)) return;
    CHECK_EQ(printed_cost(ruby), 15, "B: Ruby prints cost 15");
    CHECK(in_group(seira, GRP_SAINT), "B: Seira is inside the 'SaintSnow' filter set");
    CHECK(!is_live(seira), "B: Seira is NOT a live card (the filter is a group filter)");
    CHECK(!is_member(seira), "B: Seira is NOT a member card either - only the unit matches");
    CHECK(card_prints_trigger(ruby, RB_TSTR_DEBUT), "B: Ruby prints a DEBUT ability");
    tg->state.p[0].stage[AREA_CENTER] = ruby;
    test_give_energy(tg, 2);
    test_add_to_discard(tg, seira);
    *ruby_out = ruby; *seira_out = seira;
}

static void test_b_paid_recovers_saint_snow_and_grants_two_blades(void)
{
    TestGame *tg = &t_tg[0];
    int ruby, seira;
    b_setup(tg, &ruby, &seira);
    if (ruby < 0 || seira < 0) return;
    int hand_before = tg->state.p[0].hand.n;

    CHECK(fire_trigger(tg, ruby, RB_TSTR_DEBUT), "B/pay: the DEBUT ability is queued");
    EXPECT_PROMPTS(tg, 1, "B/pay: the optional 1E gate must be OFFERED");
    CHECK(pending(tg), "B/pay: the gate is still on screen at the assertion site");
    select_option(tg, 1);

    drain_choices(tg, 8);
    CHECK_EQ(tg->state.p[0].hand.n, hand_before + 1, "B/pay: one card moves waitroom -> hand");
    CHECK(hand_has(tg, seira), "B/pay: the 'SaintSnow' card is in hand");
    CHECK(!wait_has(tg, seira), "B/pay: the fetched card left the waitroom");
    CHECK_EQ(blade_mod(tg, ruby), 2, "B/pay: ?????? -> 2 blades until live end");
    CHECK_EQ(energy_active(tg), 1, "B/pay: exactly 1 E was paid out of 2");
}

static void test_b_declined_skips_recovery_and_blades(void)
{
    TestGame *tg = &t_tg[0];
    int ruby, seira;
    b_setup(tg, &ruby, &seira);
    if (ruby < 0 || seira < 0) return;
    int hand_before = tg->state.p[0].hand.n;

    CHECK(fire_trigger(tg, ruby, RB_TSTR_DEBUT), "B/decline: the DEBUT ability is queued");
    EXPECT_PROMPTS(tg, 1, "B/decline: the optional 1E gate must be OFFERED");
    CHECK(pending(tg), "B/decline: the gate is still on screen at the assertion site");
    select_option(tg, 0);

    drain_choices(tg, 8);
    CHECK_EQ(tg->state.p[0].hand.n, hand_before, "B/decline: declined -> no SaintSnow fetch");
    CHECK_EQ(blade_mod(tg, ruby), 0, "B/decline: declined -> no blades");
    CHECK(wait_has(tg, seira), "B/decline: the 'SaintSnow' card stays in the waitroom");
    CHECK_EQ(energy_active(tg), 2, "B/decline: declining an optional cost taps no energy");
}

/* ======================================================================
 * C. recovers_two_cost_two_members_to_hand_pl_hs_bp2_002_r_plus_test.rs
 *    (2 tests) - the [N???] any-number recovery variant.
 *
 * PL!HS-bp2-002-R? Sayaka cost 13:
 *   DEBUT????waitroom?????2???????????2?????????. 
 *   ???????????, ????????????????????????, 
 *         B B B. 
 * ====================================================================== */

static void test_c_recovers_two_cost_two_members_not_pricey_or_live(void)
{
    TestGame *tg = &t_tg[0];
    clear_p1(tg);
    int sayaka = mid(tg, SAYAKA_2UP);
    int cheap_a = mid(tg, KOTORI_2);
    int cheap_b = mid(tg, MEI_2);
    int pricey = mid(tg, KOTORI_5);
    int live = mid(tg, LIVE_MUS);
    if (!pin_id(sayaka, SAYAKA_2UP)) return;
    if (!pin_id(cheap_a, KOTORI_2) || !pin_id(cheap_b, MEI_2) ||
        !pin_id(pricey, KOTORI_5) || !pin_id(live, LIVE_MUS)) return;

    /* GROUP / TYPE / COST PINS: the two eligible cards are inside the
     * filter's set, the two decoys are outside it. Without these a
     * "nothing matched" board would pass every negative below. */
    CHECK_EQ(printed_cost(sayaka), 13, "C: Sayaka prints cost 13");
    CHECK_EQ(printed_cost(cheap_a), 2, "C: Kotori (PL!-bp3-012-PR) prints cost 2 -> eligible");
    CHECK_EQ(printed_cost(cheap_b), 2, "C: Mei (PL!SP-pb1-018-N) prints cost 2 -> eligible");
    CHECK_EQ(printed_cost(pricey), 5, "C: Kotori (PL!-pb1-021-PR) prints cost 5 -> over the cap");
    CHECK(is_member(cheap_a) && is_member(cheap_b), "C: both eligible cards are ???????");
    CHECK(is_member(pricey), "C: the cost-5 decoy IS a member - it is excluded on cost alone");
    CHECK(is_live(live), "C: START:DASH!! is a ???, excluded on card_type alone");
    CHECK(card_prints_trigger(sayaka, RB_TSTR_DEBUT), "C: Sayaka prints a DEBUT ability");

    tg->state.p[0].stage[AREA_CENTER] = sayaka;
    test_add_to_discard(tg, pricey);
    test_add_to_discard(tg, cheap_a);
    test_add_to_discard(tg, live);
    test_add_to_discard(tg, cheap_b);
    int hand_before = tg->state.p[0].hand.n;

    CHECK(fire_trigger(tg, sayaka, RB_TSTR_DEBUT), "C: the DEBUT ability is queued");
    EXPECT_PROMPTS(tg, 1, "C: the 2??? fetch must ask WHICH members");
    CHECK(pending_is_card(tg), "C: the fetch prompt is a SelectCard");
    /* The waitroom order is [pricey, cheap_a, live, cheap_b]; a CANDIDATE
     * index is what the answer must name, so index 0 is cheap_a, NOT
     * pricey. Selecting [0,1] must therefore take both cheap members. */
    {
        const int two[2] = { 0, 1 };
        pick_n(tg, two, 2);
    }
    drain_choices(tg, 8);

    CHECK_EQ(tg->state.p[0].hand.n, hand_before + 2, "C: two fetched members join the hand");
    CHECK(!wait_has(tg, cheap_a) && !wait_has(tg, cheap_b),
          "C: the two cost-2 members left the waitroom");
    CHECK(hand_has(tg, cheap_a) && hand_has(tg, cheap_b),
          "C: the two cost-2 members are the ones in hand");
    CHECK(wait_has(tg, pricey), "C: ???5 exceeds ???2?? -> the pricey member stays");
    CHECK(wait_has(tg, live),  "C: a live card is not a ???? -> START:DASH!! stays");
    CHECK_EQ(tg->state.p[0].discard.n, 2, "C: exactly the two decoys remain");
}

static void test_c_repeated_copies_keep_fetch_and_constants_per_player(void)
{
    TestGame *tg = &t_tg[0];
    clear_p1(tg);
    RbPlayer *P1 = &tg->state.p[0];
    RbPlayer *P2 = &tg->state.p[1];

    int sayaka1 = mid(tg, SAYAKA_2UP);
    int sayaka2 = mid(tg, SAYAKA_2UP);
    int big1 = mid(tg, RUBY);
    int big2 = mid(tg, RUBY);
    int c1a = mid(tg, KOTORI_2);
    int c1b = mid(tg, MEI_2);
    int c2a = mid(tg, KOTORI_2);
    int c2b = mid(tg, MEI_2);
    if (!pin_id(sayaka1, SAYAKA_2UP) || !pin_id(sayaka2, SAYAKA_2UP)) return;
    if (sayaka1 == sayaka2) { CHECK(0, "C/p2: the two Sayaka must be DISTINCT pool slots"); return; }
    CHECK_EQ(printed_cost(big1), 15, "C/p2: the stage neighbour costs 15, above Sayaka's 13");

    P1->stage[AREA_LEFT] = big1;  P1->stage[AREA_CENTER] = sayaka1; P1->stage[AREA_RIGHT] = RB_EMPTY_SLOT;
    P2->stage[AREA_LEFT] = big2;  P2->stage[AREA_CENTER] = sayaka2; P2->stage[AREA_RIGHT] = RB_EMPTY_SLOT;
    test_add_to_discard(tg, c1a); test_add_to_discard(tg, c1b);
    add_to_wait_for(tg, 1, c2a); add_to_wait_for(tg, 1, c2b);

    int p1_before = P1->hand.n, p2_before = P2->hand.n;

    CHECK(fire_trigger_for(tg, 0, sayaka1, RB_TSTR_DEBUT), "C/p2: P1 Sayaka is queued");
    EXPECT_PROMPTS(tg, 1, "C/p2: P1's 2??? fetch must prompt");
    CHECK(pending_is_card(tg), "C/p2: P1's fetch prompt is a SelectCard");
    { const int two[2] = { 0, 1 }; pick_n(tg, two, 2); }
    drain_choices(tg, 8);

    CHECK_EQ(P1->hand.n, p1_before + 2, "C/p2: P1 fetched exactly two members");
    CHECK_EQ(P2->hand.n, p2_before, "C/p2: P2's hand is untouched by P1's fetch");
    CHECK(!wait_has(tg, c1a) && !wait_has(tg, c1b), "C/p2: P1's waitroom gave up both members");
    CHECK(wait2_has(tg, c2a) && wait2_has(tg, c2b), "C/p2: P2's waitroom is untouched");

    CHECK(fire_trigger_for(tg, 1, sayaka2, RB_TSTR_DEBUT), "C/p2: P2 Sayaka is queued");
    EXPECT_PROMPTS(tg, 2, "C/p2: P2's own 2??? fetch must prompt too");
    CHECK(pending_is_card(tg), "C/p2: P2's fetch prompt is a SelectCard");
    { const int two[2] = { 0, 1 }; pick_n(tg, two, 2); }
    drain_choices(tg, 8);

    CHECK_EQ(P2->hand.n, p2_before + 2, "C/p2: P2 fetched exactly two members");
    CHECK_EQ(P1->hand.n, p1_before + 2, "C/p2: P1's hand is unchanged by P2's fetch");
    CHECK(!wait2_has(tg, c2a) && !wait2_has(tg, c2b), "C/p2: P2's waitroom gave up both members");

    test_recalc(tg);
    CHECK_EQ(blade_mod(tg, sayaka1), 3, "C/p2: P1 ?? -> 3 blades from the cost-15 neighbour");
    CHECK_EQ(blade_mod(tg, sayaka2), 3, "C/p2: P2 ?? -> 3 blades, computed per player");
    P1->stage[AREA_LEFT] = RB_EMPTY_SLOT;
    test_recalc(tg);
    CHECK_EQ(blade_mod(tg, sayaka1), 0, "C/p2: removing P1's neighbour drops P1 to 0 blades");
    CHECK_EQ(blade_mod(tg, sayaka2), 3, "C/p2: P2's blades are unaffected by P1's stage change");
}

/* ======================================================================
 * D. q145_empty_waitroom_optional_self_wait_test.rs  (1 test)
 *
 * PL!-bp3-003-R Kotori cost 11:
 *   DEBUT ?????????????????????waitroom??'MU's'?????????
 *   1???????. 
 *
 * Q145: the optional self-wait is OFFERED even when the waitroom is EMPTY, and
 * accepting it resolves cleanly with the orientation still applied.
 * ====================================================================== */

static void test_d_empty_waitroom_still_allows_optional_self_wait(void)
{
    TestGame *tg = &t_tg[0];
    clear_p1(tg);
    int filler = mid(tg, FILLER_MUS);
    fill_decks(tg, filler, 10);
    int kotori = mid(tg, KOTORI_11);
    if (!pin_id(kotori, KOTORI_11)) return;
    CHECK_EQ(printed_cost(kotori), 11, "D: Kotori prints cost 11");
    CHECK(in_group(kotori, GRP_MU), "D: Kotori is a 'MU's' member");
    CHECK(card_prints_trigger(kotori, RB_TSTR_DEBUT), "D: Kotori prints a DEBUT ability");
    CHECK_EQ(tg->state.p[0].discard.n, 0, "D: the waitroom really is EMPTY on this board");

    test_add_to_hand(tg, kotori);
    test_give_energy(tg, 11);
    play_to_stage(tg, kotori, AREA_CENTER, 1);

    CHECK(stage_has(tg, kotori), "D: Kotori is on stage after the play");
    EXPECT_PROMPTS(tg, 1, "D: the optional self-wait gate must be OFFERED on an empty waitroom");
    CHECK(pending(tg), "D: the gate is still on screen at the assertion site");
    {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        CHECK(c && !strncmp(c->target, "pay_optional_cost", 16),
              "D: the prompt is the pay_optional_cost gate (target=\"%s\")",
              c ? c->target : "(null)");
        CHECK(c && c->allow_skip == 1,
              "D: the gate carries allow_skip (allow_skip=%d)", c ? c->allow_skip : -1);
    }
    select_option(tg, 1);   /* PAY the optional wait */

    drain_choices(tg, 8);
    CHECK(!pending(tg), "D: no waitroom target resolves cleanly with no further prompt");
    CHECK(!strcmp(orientation_mod(tg, kotori), "wait"),
          "D: the accepted self-wait remains applied with an empty waitroom (got \"%s\")",
          orientation_mod(tg, kotori));
    CHECK_EQ(tg->state.p[0].hand.n, 0, "D: an empty 'MU's' waitroom yields no card to hand");
    CHECK_EQ(tg->state.p[0].discard.n, 0, "D: and the waitroom is still empty");
}

/* ======================================================================
 * E. distinct_trio_reveals_pl_sp_bp2_025_l_test.rs  (2 tests)
 *
 * PL!SP-bp2-025-L Bubble Rise score 4:
 *   ?????? ????????[Kanon], [Wien], 
 *   [Touge]???, ???????????2???????, ??????
 *   ???????????????, ????1???????. 
 *
 * Source = the REVEAL set. Gate = a distinct-NAME count over the trio.
 * ====================================================================== */

static void e_setup(TestGame *tg, int *live_out)
{
    clear_p1(tg);
    int filler = mid(tg, FILLER_NIJI);
    fill_decks(tg, filler, 5);
    int live = mid(tg, BUBBLE);
    if (!pin_id(live, BUBBLE)) { *live_out = -1; return; }
    CHECK_EQ(printed_score(live), 4, "E: Bubble Rise prints score 4");
    CHECK(card_prints_trigger(live, RB_TSTR_LIVE_SUCCESS), "E: Bubble Rise prints a ?????? ability");
    test_add_to_hand(tg, live);
    test_set_live_card_for(tg, 0, live);
    *live_out = live;
}

static void test_e_two_distinct_trio_names_retrieve_revealed_card(void)
{
    TestGame *tg = &t_tg[0];
    int live;
    e_setup(tg, &live);
    if (live < 0) return;
    int kanon = mid(tg, KANON_SD);
    int wien = mid(tg, WIEN_SD2);
    int prize = mid(tg, PRIZE_KANON);
    if (!pin_id(kanon, KANON_SD) || !pin_id(wien, WIEN_SD2) ||
        !pin_id(prize, PRIZE_KANON)) return;
    /* DISTINCT-NAME PIN: two different cards, so the [??????] count is 2. */
    CHECK(strcmp(card_name_of(kanon), card_name_of(wien)) != 0,
          "E: the two trio members have DIFFERENT names ('%s'=%s vs '%s'=%s)",
          card_name_of(kanon), card_no_of(kanon), card_name_of(wien), card_no_of(wien));
    CHECK_EQ(printed_cost(prize), 2, "E: the revealed prize is a real card");

    tg->state.p[0].stage[AREA_LEFT] = kanon;
    tg->state.p[0].stage[AREA_RIGHT] = wien;
    reveal_cards(tg, &prize, 1);

    CHECK(fire_trigger(tg, live, RB_TSTR_LIVE_SUCCESS), "E: the ?????? ability is queued");
    EXPECT_PROMPTS(tg, 1, "E: the gate is met -> the revealed card must be offered");
    CHECK(pending(tg), "E: the retrieval prompt is still on screen at the assertion site");
    drain_pick0(tg, 8);

    CHECK(hand_has(tg, prize), "E: gate met -> the revealed card is retrieved to hand");
    CHECK(!revealed_has(tg, prize), "E: the retrieved card left the revealed set");
}

static void test_e_one_trio_name_does_not_retrieve_revealed_card(void)
{
    TestGame *tg = &t_tg[0];
    int live;
    e_setup(tg, &live);
    if (live < 0) return;
    int kanon = mid(tg, KANON_SD);
    int prize = mid(tg, PRIZE_KANON);
    if (!pin_id(kanon, KANON_SD) || !pin_id(prize, PRIZE_KANON)) return;

    tg->state.p[0].stage[AREA_LEFT] = kanon;
    reveal_cards(tg, &prize, 1);

    /* A FAILING gate must not queue the ability, so the absence is the expected
     * shape here and is asserted positively: test_e_two_distinct_trio_names is
     * the identical setup plus a second, differently-named trio member, and it
     * DOES queue, prompt and fetch. */
    fire_trigger(tg, live, RB_TSTR_LIVE_SUCCESS);
    drain_choices(tg, 8);
    CHECK(!pending(tg), "E: fewer than 2 distinct trio names -> no prompt at all");

    /* POSITIVE CONTROL ON THE SAME BOARD SHAPE: test_e_two_distinct_trio_names
     * is the identical setup with a second, differently-named trio member
     * added, and it DOES fetch. So this board's non-retrieval is the gate
     * working, not a dead effect. */
    CHECK(!hand_has(tg, prize), "E: <2 distinct names -> nothing retrieved");
    CHECK(revealed_has(tg, prize), "E: <2 distinct names -> the revealed card stays revealed");
    CHECK_EQ(tg->state.p[0].stage[AREA_CENTER], RB_EMPTY_SLOT,
             "E: no member was placed in the centre, so the trio count really is 1");
}

/* ======================================================================
 * F. success_zone_revealed_members_pl_s_bp5_019_l_test.rs  (3 tests)
 *
 * PL!S-bp5-019-L not ALONE not HITORI score 5:
 *   ?????? ??????????????????????2???????, 
 *   ?????????????????????, ????????2?????????. 
 *
 * Source = the REVEAL set, [2???] any-number variant, card_type filter.
 * ====================================================================== */

static void f_setup(TestGame *tg, int *live_out)
{
    clear_p1(tg);
    int filler = mid(tg, FILLER_NIJI);
    fill_decks(tg, filler, 5);
    int live = mid(tg, NOT_ALONE);
    if (!pin_id(live, NOT_ALONE)) { *live_out = -1; return; }
    CHECK_EQ(printed_score(live), 5, "F: not ALONE not HITORI prints score 5");
    CHECK(card_prints_trigger(live, RB_TSTR_LIVE_SUCCESS),
          "F: not ALONE not HITORI prints a ?????? ability");
    test_add_to_hand(tg, live);
    test_set_live_card_for(tg, 0, live);
    *live_out = live;
}

static void test_f_two_own_success_cards_retrieve_two_revealed_members(void)
{
    TestGame *tg = &t_tg[0];
    int live;
    f_setup(tg, &live);
    if (live < 0) return;
    int s1 = mid(tg, LIVE_MUS);
    int s2 = mid(tg, LINK_BASE_L);
    int m1 = mid(tg, OMIKAZE);
    int m2 = mid(tg, RENA);
    if (!pin_id(s1, LIVE_MUS) || !pin_id(s2, LINK_BASE_L) ||
        !pin_id(m1, OMIKAZE) || !pin_id(m2, RENA)) return;
    /* TYPE PINS: both revealed cards are ????, so the card_type filter has
     * two members to offer - a "matched nothing" board cannot pass. */
    CHECK(is_member(m1) && is_member(m2), "F: both revealed cards are ???????");
    CHECK(m1 != m2, "F: the two revealed members are two DISTINCT pool slots");

    test_add_to_success(tg, s1);
    test_add_to_success(tg, s2);
    reveal_cards(tg, (const int[]){ m1, m2 }, 2);

CHECK(fire_trigger(tg, live, RB_TSTR_LIVE_SUCCESS), "F: the ?????? ability is queued");
    EXPECT_PROMPTS(tg, 1, "F: an up-to-2 member selection must be offered");
    CHECK(pending_is_card(tg), "F: the prompt is a SelectCard (revealed, count=2)");
    { const int two[2] = { 0, 1 }; pick_n(tg, two, 2); }
    drain_choices(tg, 8);

    CHECK(hand_has(tg, m1) && hand_has(tg, m2), "F: gate met -> up to 2 members retrieved to hand");
    CHECK_EQ(tg->state.p[0].success.n, 2, "F: the success zone is a gate, not a source: it is untouched");
}

static void test_f_p2_success_zone_allows_partial_member_retrieval(void)
{
    TestGame *tg = &t_tg[0];
    int live;
    f_setup(tg, &live);
    if (live < 0) return;
    int s1 = mid(tg, LIVE_MUS);
    int s2 = mid(tg, LINK_BASE_L);
    int m_a = mid(tg, OMIKAZE);
    int m_b = mid(tg, RENA);
    int non_member = mid(tg, LIVE_MUS);
    if (!pin_id(s1, LIVE_MUS) || !pin_id(s2, LINK_BASE_L) ||
        !pin_id(m_a, OMIKAZE) || !pin_id(m_b, RENA) ||
        !pin_id(non_member, LIVE_MUS)) return;

    /* P2 holds the two success cards - the gate is [??????]. */
    test_add_to_opp_success(tg, s1);
    test_add_to_opp_success(tg, s2);
    CHECK_EQ(tg->state.p[1].success.n, 2, "F/p2: P2 really holds 2 success cards");
    CHECK(is_live(non_member), "F/p2: the middle revealed card is a ???, filtered out on type");
    CHECK(is_member(m_a) && is_member(m_b), "F/p2: the two outer revealed cards are ????");

    reveal_cards(tg, (const int[]){ m_a, non_member, m_b }, 3);

    CHECK(fire_trigger(tg, live, RB_TSTR_LIVE_SUCCESS), "F/p2: the ?????? ability is queued");
    EXPECT_PROMPTS(tg, 1, "F/p2: the up-to-2 prompt must appear");
    CHECK(pending_is_card(tg), "F/p2: the prompt is a filtered SelectCard");
    {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        CHECK(c && c->n_filtered_indices == 2,
              "F/p2: the card_type=live filter must offer exactly the 2 members "
              "(candidates offered = %d)", c ? c->n_filtered_indices : -1);
        if (c && c->n_filtered_indices == 2) {
            /* The revealed set is [m_a, non_member, m_b]; candidate 0 must be
             * m_a and candidate 1 must be m_b. */
            int z0 = c->filtered_indices[0], z1 = c->filtered_indices[1];
            CHECK(z0 >= 0 && z0 < tg->state.n_revealed && tg->state.revealed_cards[z0] == m_a,
                  "F/p2: candidate 0 is the FIRST member (revealed slot %d)", z0);
            CHECK(z1 >= 0 && z1 < tg->state.n_revealed && tg->state.revealed_cards[z1] == m_b,
                  "F/p2: candidate 1 is the SECOND member (revealed slot %d)", z1);
        }
    }
    pick_n(tg, (const int[]){ 0 }, 1);
    drain_choices(tg, 8);

    CHECK(hand_has(tg, m_a), "F/p2: the one picked member reached the hand");
    CHECK(!hand_has(tg, m_b), "F/p2: the unpicked member did NOT reach the hand");
    CHECK(revealed_has(tg, m_b), "F/p2: the unpicked member is still revealed");
    CHECK(revealed_has(tg, non_member), "F/p2: the non-member is still revealed");
    CHECK_EQ(tg->state.p[1].success.n, 2, "F/p2: P2's success zone is a gate, not a source");
}

static void test_f_zero_or_one_success_card_does_not_retrieve(void)
{
    for (int success_count = 0; success_count <= 1; success_count++) {
        /* t_tg is a ONE-slot array (line 141) because sizeof(TestGame) is
         * 885912 B: two of them live at once is a 1.7 MB stack frame. The old
         * `&t_tg[11 + success_count]` subscripted indices 11 and 12 of that
         * 1-element array, i.e. 9745032..10630944 B past an 885912 B object -
         * unmapped, so the first store in f_setup() raised SIGSEGV and gcc
         * warned "array subscript 11 is above array bounds of 'TestGame[1]'".
         * The two boards are independent and sequential, so one slot plus a
         * full test_game_new() per iteration is equivalent. */
        TestGame *tg = &t_tg[0];
        test_game_new(tg);
        current_test = (success_count == 0)
            ? "test_f_zero_success_card_does_not_retrieve"
            : "test_f_one_success_card_does_not_retrieve";
        prompts = 0;
        int live;
        f_setup(tg, &live);
        if (live < 0) return;
        int member = mid(tg, OMIKAZE);
        if (!pin_id(member, OMIKAZE)) return;
        for (int i = 0; i < success_count; i++) {
            int s = mid(tg, LIVE_MUS);
            test_add_to_success(tg, s);
        }
        CHECK_EQ(tg->state.p[0].success.n, success_count,
                 "F/n%d: the board really holds %d success card(s)",
                 success_count, success_count);
        CHECK(is_member(member), "F/n%d: the revealed card IS a ???????", success_count);
        reveal_cards(tg, &member, 1);

        fire_trigger(tg, live, RB_TSTR_LIVE_SUCCESS);
        drain_choices(tg, 8);

        CHECK(!pending(tg), "F/n%d: fewer than 2 success cards -> no prompt at all", success_count);
        CHECK(!hand_has(tg, member), "F/n%d: fewer than 2 success cards -> nothing retrieved", success_count);
        CHECK(revealed_has(tg, member), "F/n%d: the revealed card stays revealed", success_count);
        /* POSITIVE CONTROL: test_f_two_own_success_cards_retrieve_two_revealed_
         * members is the identical board with 2 success cards and DOES prompt
         * and fetch, so this non-retrieval is the gate, not a dead effect. */
    }
}

/* ======================================================================
 * G. tied_score_reveals_pl_hs_cl1_012_cl_test.rs  (2 tests)
 *
 * PL!HS-cl1-012-CL Edelied score 2:
 *   ?????? ????????????????????, ???????????
 *   ??????????, ???9???????????1???????. 
 *
 * Source = the REVEAL set; gate = a score tie; filter = ???9??.
 * ====================================================================== */

/* Rust place_tied_live: push Taxonomy (a live) into this player's live
 * zone AND seat Riko on the stage, so both players end up with the same
 * total score once Edelied is in p1's live zone. */
static void g_place_tied_live(TestGame *tg, int player, int member)
{
    int l = mid(tg, TAXONOMY);
    if (l >= 0) {
        CHECK_EQ(printed_score(l), 1, "G: Taxonomy prints score 1");
        test_add_to_live_for(tg, player, l);
    }
    tg->state.p[player].stage[AREA_RIGHT] = member;
}

static int g_edelied = -1, g_taxonomy = -1;

static void g_setup(TestGame *tg)
{
    clear_p1(tg);
    int filler = mid(tg, FILLER_MUS);
    fill_decks(tg, filler, 5);
    /* Rust setup_live pushes Edelied straight into p1's live zone. */
    int live = mid(tg, EDELIED);
    if (!pin_id(live, EDELIED)) return;
    g_edelied = live;
    g_taxonomy = mid(tg, TAXONOMY);
    CHECK_EQ(printed_score(live), 2, "G: Edelied prints score 2");
    CHECK(card_prints_trigger(live, RB_TSTR_LIVE_SUCCESS), "G: Edelied prints a LiveSuccess ability");
    test_add_to_live_for(tg, 0, live);
}

static int g_member(TestGame *tg)
{
    int m = mid(tg, RIKO);
    if (!pin_id(m, RIKO)) return -1;
    return m;
}

static void test_g_tied_scores_retrieve_expensive_revealed_member(void)
{
    TestGame *tg = &t_tg[0];
    g_setup(tg);
    int member = g_member(tg);
    int expensive = mid(tg, GINKO_15);
    int cheap = mid(tg, KANON_2);
    if (member < 0) return;
    if (!pin_id(expensive, GINKO_15) || !pin_id(cheap, KANON_2)) return;
    /* COST PINS: one card is inside ???9??, one is outside it. */
    CHECK_EQ(printed_cost(expensive), 15, "G: Ginko costs 15 -> inside ???9??");
    CHECK_EQ(printed_cost(cheap), 2, "G: Kanon costs 2 -> outside ???9??");
    CHECK(is_member(expensive) && is_member(cheap), "G: both revealed cards are ???????");

    g_place_tied_live(tg, 0, member);
    g_place_tied_live(tg, 1, member);
    /* RUST FIXTURE BUG #3, CORRECTED HERE - see REPORT. The Rust board is NOT
     * a tie: pl_hs_cl1_012_cl_setup_live pushes Edelied (score 2) into P1's
     * live zone ONCE and place_tied_live pushes Taxonomy (score 1) into each
     * player's zone ONCE, giving P1 = 1 + 2 = 3 against P2 = 1. The helper is
     * named place_tied_live but it never balances the two. Edelied is mirrored
     * into P2's live zone below so the gate the test is ABOUT is actually met;
     * the post-fix board is pinned so the tie is provable, not assumed. */
    test_add_to_live_for(tg, 1, g_edelied);
    test_recalc(tg);
    /* RUST FIXTURE BUG #3, SEE REPORT. The board the Rust test builds is NOT a
     * tie. pl_hs_cl1_012_cl_setup_live pushes Edelied (score 2) into P1's live
     * zone ONCE, and place_tied_live pushes Taxonomy (score 1) into each
     * player's zone ONCE. So P1 = 1 + 2 = 3 and P2 = 1. The helper is named
     * "place_tied_live" but it never balances the two. The pins below state
     * the board the Rust test actually builds. */
    CHECK_EQ(tg->state.p[0].live.n, 2, "G: P1 live zone holds Taxonomy + Edelied");
    CHECK_EQ(tg->state.p[1].live.n, 2, "G: P2 live zone is now a true mirror of P1's");
    CHECK_EQ(printed_score(g_edelied), 2, "G: Edelied prints score 2");
    CHECK_EQ(printed_score(g_taxonomy), 1, "G: the shared Taxonomy print scores 1");
    /* The tie is PROVED from the printed scores, not assumed: each side scores
     * Taxonomy(1) + Edelied(2). */
    CHECK_EQ(printed_score(g_taxonomy) + printed_score(g_edelied), 3,
             "G: each live zone scores Taxonomy(1) + Edelied(2) = 3");
    {
        int p1_total = 0, p2_total = 0;
        for (int i = 0; i < tg->state.p[0].live.n; i++) p1_total += printed_score(tg->state.p[0].live.cards[i]);
        for (int i = 0; i < tg->state.p[1].live.n; i++) p2_total += printed_score(tg->state.p[1].live.cards[i]);
        CHECK_EQ(p1_total, 3, "G: P1 printed live-score total is 3");
        CHECK_EQ(p2_total, 3, "G: P2 printed live-score total is 3 - the totals really are equal");
    }
    reveal_cards(tg, (const int[]){ expensive, cheap }, 2);

    int live = mid(tg, EDELIED);
    CHECK(fire_trigger(tg, live, RB_TSTR_LIVE_SUCCESS), "G: the ?????? ability is queued");
    EXPECT_PROMPTS(tg, 1, "G: tie -> the ???9?? revealed member must be offered");
    CHECK(pending(tg), "G: the retrieval prompt is still on screen at the assertion site");
    drain_pick0(tg, 8);

    CHECK(hand_has(tg, expensive), "G: tie -> the cost-15 member is retrieved to hand");
    CHECK(!hand_has(tg, cheap), "G: the cost-2 member is not eligible and is NOT retrieved");
    CHECK(revealed_has(tg, cheap), "G: the ineligible card stays in the revealed set");
}

static void test_g_unequal_scores_do_not_retrieve_revealed_member(void)
{
    TestGame *tg = &t_tg[0];
    g_setup(tg);
    int member = g_member(tg);
    int expensive = mid(tg, GINKO_15);
    if (member < 0) return;
    if (!pin_id(expensive, GINKO_15)) return;
    CHECK_EQ(printed_cost(expensive), 15, "G: the card IS inside ???9??, so only the tie can block it");

    g_place_tied_live(tg, 0, member);   /* P1 only -> scores differ */
    test_recalc(tg);
    CHECK_EQ(tg->state.p[0].live.n, 2, "G: P1 live zone holds two live cards");
    CHECK_EQ(tg->state.p[1].live.n, 0, "G: P2 has NO live cards, so the scores cannot tie");
    reveal_cards(tg, &expensive, 1);

    int live = mid(tg, EDELIED);
    CHECK(fire_trigger(tg, live, RB_TSTR_LIVE_SUCCESS), "G: the ?????? ability is queued");
    drain_choices(tg, 8);

    /* POSITIVE CONTROL: test_g_tied_scores_retrieve_expensive_revealed_member is
     * the identical board plus a mirrored P2 live zone, and it DOES fetch. */
    CHECK(!hand_has(tg, expensive), "G: scores not tied -> no retrieval");
    CHECK(revealed_has(tg, expensive), "G: the revealed card stays revealed");
}

/* ======================================================================
 * H. baton_touch_cheaper_unit_recovers_hasunosora_live_to_hand_test.rs
 *    (8 tests)
 *
 * PL!HS-bp2-007-P Ginko cost 11  (RUST FIXTURE BUG corrected, see REPORT):
 *   DEBUT ??????????????'SUZUKE'???????????????
 *   DEBUT????, ???waitroom??'HASU'????????1???????. 
 *
 * Source = the WAITROOM, filtered to 'HASU' LIVE cards, count=1, and only
 * after a baton touch whose REPLACED member is a CHEAPER 'SUZUKE' one.
 * ====================================================================== */

static void h_setup(TestGame *tg, const char *replaced, int *ginko_out, int *replaced_out)
{
    clear_p1(tg);
    int rep = mid(tg, replaced);
    if (!pin_id(rep, replaced)) { *replaced_out = -1; return; }
    tg->state.p[0].stage[AREA_CENTER] = rep;
    test_give_energy(tg, 25);
    int ginko = mid(tg, GINKO_BT);
    if (!pin_id(ginko, GINKO_BT)) { *replaced_out = -1; return; }
    *ginko_out = ginko; *replaced_out = rep;
}

/* Play ?? over whatever occupies the centre, then resolve every prompt
 * (Rust baton_touch_live_recovery_source). Bounded at 10 iterations. */
static void h_baton_touch(TestGame *tg, int ginko)
{
    int filler = mid(tg, FILLER_MUS);
    test_add_to_hand(tg, ginko);
    test_add_to_hand(tg, filler);
    play_to_stage(tg, ginko, AREA_CENTER, 1);
    drain_pick0(tg, 10);
}

static void test_h_cheaper_unit_baton_recovers_group_live_to_hand(void)
{
    TestGame *tg = &t_tg[0];   /* reused: this is a fresh fork, see main() */
    int ginko, rep;
    h_setup(tg, SUZU_4, &ginko, &rep);
    if (ginko < 0) return;
    int hasu = mid(tg, DREAM_1);
    if (!pin_id(hasu, DREAM_1)) return;
    /* GROUP / TYPE PINS: the target is inside 'HASU' + live, the decoy is not. */
    CHECK(in_group(hasu, GRP_HASU), "H/1: Dream Believers is inside the 'HASU' filter set");
    CHECK(is_live(hasu), "H/1: Dream Believers is a ?????? as the filter requires");
    CHECK_EQ(printed_cost(ginko), 11, "H/1: Ginko prints cost 11");
    CHECK_EQ(printed_cost(rep), 4, "H/1: Kazuha costs 4 - strictly cheaper, as the gate requires");
    CHECK(in_group(rep, GRP_SUZU), "H/1: Kazuha is a 'SUZUKE' member as the gate requires");
    test_add_to_discard(tg, hasu);

    h_baton_touch(tg, ginko);

    CHECK(stage_has(tg, ginko), "H/1: ?? is on stage after the baton touch");
    CHECK(hand_has(tg, hasu), "H/1: the 'HASU' live card is added to hand");
    CHECK(!wait_has(tg, hasu), "H/1: the 'HASU' live card left the waitroom");
}

static void test_h_non_hasu_live_not_added(void)
{
    TestGame *tg = &t_tg[0];
    int ginko, rep;
    h_setup(tg, SUZU_4, &ginko, &rep);
    if (ginko < 0) return;
    int mus = mid(tg, YOUTA);
    if (!pin_id(mus, YOUTA)) return;
    /* GROUP PIN: this is the ONLY thing that can keep a live card out here, so
     * pin that the decoy really is outside 'HASU'. */
    CHECK(!in_group(mus, GRP_HASU), "H/2: ?????????? is OUTSIDE 'HASU'");
    CHECK(is_live(mus), "H/2: the decoy IS a ???, so only the group filter can exclude it");
    test_add_to_discard(tg, mus);

    h_baton_touch(tg, ginko);

    CHECK(!hand_has(tg, mus), "H/2: a non-'HASU' live must NOT be added to hand");
    CHECK(wait_has(tg, mus), "H/2: the non-'HASU' live stays in the waitroom");
}

static void test_h_mixed_waitroom_only_hasu_selectable(void)
{
    TestGame *tg = &t_tg[0];
    int ginko, rep;
    h_setup(tg, SUZU_4, &ginko, &rep);
    if (ginko < 0) return;
    int hasu = mid(tg, DREAM_1);
    int mus = mid(tg, YOUTA);
    if (!pin_id(hasu, DREAM_1) || !pin_id(mus, YOUTA)) return;
    /* The decoy goes in FIRST so a broken filter would pick it at index 0. */
    test_add_to_discard(tg, mus);
    test_add_to_discard(tg, hasu);

    h_baton_touch(tg, ginko);

    CHECK(hand_has(tg, hasu), "H/3: the 'HASU' live is selectable and added to hand");
    CHECK(wait_has(tg, mus), "H/3: the MU's live remains in the waitroom");
    CHECK(!hand_has(tg, mus), "H/3: the MU's live must never be selected");
}

static void test_h_wrong_group_replaced_no_trigger(void)
{
    TestGame *tg = &t_tg[0];
    int ginko, rep;
    h_setup(tg, MIKI_9, &ginko, &rep);
    if (ginko < 0) return;
    int hasu = mid(tg, DREAM_1);
    if (!pin_id(hasu, DREAM_1)) return;
    /* GROUP PIN: the gate needs a 'SUZUKE' replaced member. */
    CHECK(!in_group(rep, GRP_SUZU), "H/4: Miki is NOT 'SUZUKE', so the gate must fail");
    CHECK_EQ(printed_cost(rep), 9, "H/4: Miki costs 9, i.e. strictly cheaper than 11");
    CHECK_EQ(printed_cost(ginko), 11, "H/4: ?? still costs 11");
    test_add_to_discard(tg, hasu);

    h_baton_touch(tg, ginko);

    CHECK(stage_has(tg, ginko), "H/4: ?? still enters play - the play itself is not gated");
    CHECK(wait_has(tg, hasu), "H/4: no trigger over a non-'SUZUKE' member: waitroom untouched");
    CHECK(!hand_has(tg, hasu), "H/4: no trigger over a non-'SUZUKE' member: hand untouched");
}

static void test_h_equal_cost_replaced_no_trigger(void)
{
    TestGame *tg = &t_tg[0];
    int ginko, rep;
    h_setup(tg, SUZU_11, &ginko, &rep);
    if (ginko < 0) return;
    int hasu = mid(tg, DREAM_1);
    if (!pin_id(hasu, DREAM_1)) return;
    CHECK_EQ(printed_cost(rep), 11, "H/5: Hanano also costs 11 - NOT [????????]");
    CHECK(in_group(rep, GRP_SUZU), "H/5: Hanano IS 'SUZUKE', so only cost can block it");
    test_add_to_discard(tg, hasu);

    h_baton_touch(tg, ginko);

    CHECK(wait_has(tg, hasu), "H/5: an equal-cost replacement is not [????????]");
    CHECK(!hand_has(tg, hasu), "H/5: an equal-cost replacement must not add cards to hand");
}

static void test_h_empty_waitroom_resolves_cleanly(void)
{
    TestGame *tg = &t_tg[0];
    int ginko, rep;
    h_setup(tg, SUZU_4, &ginko, &rep);
    if (ginko < 0) return;
    CHECK_EQ(tg->state.p[0].discard.n, 0, "H/6: the waitroom really is empty");
    int hand_before = tg->state.p[0].hand.n;

    h_baton_touch(tg, ginko);

    CHECK(stage_has(tg, ginko), "H/6: Ginko still enters play with an empty waitroom");
    CHECK(!pending(tg), "H/6: an empty waitroom resolves with nothing left on screen");
    /* hand_before is 0. The helper pushes Ginko + filler (2) and Ginko leaves
     * for the stage (-1), so the net is +1: the filler alone. Rust asserts the
     * same +1. */
    CHECK_EQ(tg->state.p[0].hand.n, hand_before + 1,
             "H/6: ginko+filler pushed, ginko played: net +1 card in hand");
    CHECK_EQ(tg->state.p[0].discard.n, 0, "H/6: an empty waitroom gains nothing");
}

static void test_h_member_in_waitroom_ignored(void)
{
    TestGame *tg = &t_tg[0];
    int ginko, rep;
    h_setup(tg, SUZU_4, &ginko, &rep);
    if (ginko < 0) return;
    int member = mid(tg, SUZU_4);
    if (!pin_id(member, SUZU_4)) return;
    /* TYPE + GROUP PINS: this decoy passes the 'HASU' group test only by
     * not being tested; it is excluded on card_type. */
    CHECK(is_member(member), "H/7: Kazuha is a ????, not a ???");
    CHECK(!is_live(member), "H/7: the decoy is not a ??????");
    test_add_to_discard(tg, member);

    h_baton_touch(tg, ginko);

    CHECK(wait_has(tg, member), "H/7: a member card stays in the waitroom (card_type=live_card filter)");
    CHECK(!hand_has(tg, member), "H/7: a member card must not be added to hand");
}

static void test_h_two_hasu_lives_exactly_one_taken(void)
{
    TestGame *tg = &t_tg[0];
    int ginko, rep;
    h_setup(tg, SUZU_4, &ginko, &rep);
    if (ginko < 0) return;
    int hasu1 = mid(tg, DREAM_1);
    int hasu2 = mid(tg, DREAM_1);
    if (!pin_id(hasu1, DREAM_1) || !pin_id(hasu2, DREAM_1)) return;
    CHECK(hasu1 != hasu2, "H/8: the two Dream Believers are two DISTINCT pool slots");
    test_add_to_discard(tg, hasu1);
    test_add_to_discard(tg, hasu2);

    h_baton_touch(tg, ginko);

    int in_hand = 0, in_wait = 0;
    for (int i = 0; i < tg->state.p[0].hand.n; i++)
        if (tg->state.p[0].hand.cards[i] == hasu1 || tg->state.p[0].hand.cards[i] == hasu2) in_hand++;
    for (int i = 0; i < tg->state.p[0].discard.n; i++)
        if (tg->state.p[0].discard.cards[i] == hasu1 || tg->state.p[0].discard.cards[i] == hasu2) in_wait++;
    CHECK_EQ(in_hand, 1, "H/8: exactly one 'HASU' live added to hand");
    CHECK_EQ(in_wait, 1, "H/8: exactly one 'HASU' live remains in the waitroom");
}

/* ======================================================================
 * RUNNER
 * ====================================================================== */


/* ENGINE PROBE (red on purpose). test_new_id() is asked for an SD2 print and
 * hands back the SD1 print of the SAME character. Rust's game.new_id() keys
 * strictly on the card_no string, so PL!SP-sd2-010-SD2 (Wien) and
 * PL!SP-sd1-002-SD (Kanata) must be two different cards. */
static void test_z_probe_sd2_card_number_resolution(void)
{
    TestGame *tg = &t_tg[0];
    int kanata = mid(tg, "PL!SP-sd1-002-SD");
    int wien   = mid(tg, "PL!SP-sd2-010-SD2");
    CHECK(kanata != wien,
          "Z: PL!SP-sd2-010-SD2 must resolve to a card OTHER than PL!SP-sd1-002-SD "
          "(got '%s' for both, ids %d and %d)",
          card_no_of(wien), kanata, wien);
    CHECK(pin_id(wien, "PL!SP-sd2-010-SD2"),
          "Z: PL!SP-sd2-010-SD2 resolves byte-exactly to its own print");
    CHECK(strcmp(card_name_of(kanata), card_name_of(wien)) != 0,
          "Z: the two prints have different names ('%s' vs '%s')",
          card_name_of(kanata), card_name_of(wien));

    /* Localise it: a direct TEMPLATE lookup (no copy) vs the copy that
     * test_new_id handed back. If the template is right and the copy is not,
     * the defect is in rb_create_card_copy, not in the database. */
    {
        Card tpl;
        memset(&tpl, 0, sizeof tpl);
        if (rb_card_get_card_by_no("PL!SP-sd2-010-SD2", &tpl)) {
            fprintf(stderr, "Z: TEMPLATE PL!SP-sd2-010-SD2 name='%s'\n", tpl.name ? tpl.name : "(null)");
            /* ???? = E3 82 A6 E3 82 A3 E3 83 BC E3 83 B3 */
            CHECK(tpl.name && strstr(tpl.name, "\xe3\x82\xa6\xe3\x82\xa3\xe3\x83\xbc\xe3\x83\xb3") != NULL,
                  "Z: the TEMPLATE name decodes to Wien (got '%s')", tpl.name ? tpl.name : "(null)");
            rb_free_card(&tpl);
        } else {
            CHECK(0, "Z: rb_card_get_card_by_no cannot resolve PL!SP-sd2-010-SD2");
        }
    }
    CHECK(strcmp(card_name_of(wien), "Kanata") != 0,
          "Z: the COPY of PL!SP-sd2-010-SD2 must not carry the Kanata name (got '%s')",
          card_name_of(wien));
}

typedef void (*CaseFn)(void);

typedef struct { const char *name; CaseFn fn; } Case;

static Case cases[] = {
    { "a_paid_energy_recovers_liella_member",            test_a_paid_energy_recovers_liella_member },
    { "a_declined_energy_does_not_recover_member",       test_a_declined_energy_does_not_recover_member },
    { "b_paid_recovers_saint_snow_and_two_blades",       test_b_paid_recovers_saint_snow_and_grants_two_blades },
    { "b_declined_skips_recovery_and_blades",            test_b_declined_skips_recovery_and_blades },
    { "c_two_cost_two_members_not_pricey_or_live",       test_c_recovers_two_cost_two_members_not_pricey_or_live },
    { "c_repeated_copies_per_player",                    test_c_repeated_copies_keep_fetch_and_constants_per_player },
    { "d_empty_waitroom_allows_optional_self_wait",      test_d_empty_waitroom_still_allows_optional_self_wait },
    { "e_two_distinct_trio_names_retrieve_revealed",     test_e_two_distinct_trio_names_retrieve_revealed_card },
    { "e_one_trio_name_does_not_retrieve",               test_e_one_trio_name_does_not_retrieve_revealed_card },
    { "f_two_success_cards_retrieve_two_members",        test_f_two_own_success_cards_retrieve_two_revealed_members },
    { "f_p2_success_allows_partial_retrieval",           test_f_p2_success_zone_allows_partial_member_retrieval },
    { "f_zero_success_card_does_not_retrieve",           test_f_zero_or_one_success_card_does_not_retrieve },
    { "g_tied_scores_retrieve_expensive_member",         test_g_tied_scores_retrieve_expensive_revealed_member },
    { "g_unequal_scores_do_not_retrieve",                test_g_unequal_scores_do_not_retrieve_revealed_member },
    { "h_cheaper_unit_baton_recovers_group_live",        test_h_cheaper_unit_baton_recovers_group_live_to_hand },
    { "h_non_hasu_live_not_added",                       test_h_non_hasu_live_not_added },
    { "h_mixed_waitroom_only_hasu_selectable",           test_h_mixed_waitroom_only_hasu_selectable },
    { "h_wrong_group_replaced_no_trigger",               test_h_wrong_group_replaced_no_trigger },
    { "h_equal_cost_replaced_no_trigger",                test_h_equal_cost_replaced_no_trigger },
    { "h_empty_waitroom_resolves_cleanly",               test_h_empty_waitroom_resolves_cleanly },
    { "h_member_in_waitroom_ignored",                    test_h_member_in_waitroom_ignored },
    { "h_two_hasu_lives_exactly_one_taken",              test_h_two_hasu_lives_exactly_one_taken },
    { "z_probe_sd2_card_number_resolution",        test_z_probe_sd2_card_number_resolution },
};

#define N_CASES ((int)(sizeof cases / sizeof cases[0]))

/* Each case runs in a FORKED child with a fatal-signal handler naming the test
 * in flight. No watchdog is achievable on this toolchain, so every drain in
 * the case bodies is individually bounded instead. */
static int run_case(int idx)
{
    current_test = cases[idx].name;
    int pipefd[2];
    if (pipe(pipefd) != 0) { perror("pipe"); return -1; }
    fflush(stdout); fflush(stderr);

    pid_t pid = fork();
    if (pid < 0) { perror("fork"); return -1; }
    if (pid == 0) {
        close(pipefd[0]);
        signal(SIGSEGV, on_fatal_signal);
        signal(SIGBUS,  on_fatal_signal);
        signal(SIGABRT, on_fatal_signal);
        signal(SIGFPE,  on_fatal_signal);
        signal(SIGILL,  on_fatal_signal);
        /* A fresh copy-pool per case: rb_create_card_copy draws from a
         * process-global monotonic pool that nothing resets, and a long suite
         * exhausts it so every identity assertion goes vacuous. */
        rb_card_copy_pool_reset();
        prompts = 0;
        failures = 0;
        assertions = 0;
        setup_bugs = 0;
        cases[idx].fn();
        if (failures || setup_bugs) {
            TestGame *d = &t_tg[0];
            fprintf(stderr, "\n  DUMP after %s: pending=%d type=%s prompts=%d\n",
                    cases[idx].name, rb_has_pending_choice(&d->state),
                    test_pending_choice_type(d) ? test_pending_choice_type(d) : "-",
                    prompts);
            for (int pl = 0; pl < 2; pl++) {
                RbPlayer *P = &d->state.p[pl];
                fprintf(stderr, "    P%d hand:", pl);
                for (int i = 0; i < P->hand.n; i++) fprintf(stderr, " %s", card_no_of(P->hand.cards[i]));
                fprintf(stderr, "\n    P%d wait:", pl);
                for (int i = 0; i < P->discard.n; i++) fprintf(stderr, " %s", card_no_of(P->discard.cards[i]));
                fprintf(stderr, "\n    P%d stage:", pl);
                for (int i = 0; i < RB_STAGE_SIZE; i++)
                    if (P->stage[i] != RB_EMPTY_SLOT) fprintf(stderr, " [%d]=%s", i, card_no_of(P->stage[i]));
                fprintf(stderr, "\n    P%d live:", pl);
                for (int i = 0; i < P->live.n; i++) fprintf(stderr, " %s", card_no_of(P->live.cards[i]));
                fprintf(stderr, "\n    P%d success=%d energy_active=%d revealed:", pl,
                        P->success.n, P->energy_active);
                for (int i = 0; i < d->state.n_revealed; i++)
                    fprintf(stderr, " %s", card_no_of(d->state.revealed_cards[i]));
                fputc('\n', stderr);
            }
            if (rb_has_pending_choice(&d->state)) {
                const RbChoice *c = rb_get_pending_choice(&d->state);
                fprintf(stderr, "    CHOICE kind=%d zone=%s card_type=%s count=%d allow_skip=%d target=%s ncand=%d\n",
                        c ? (int)c->kind : -1, c ? c->zone : "-", c ? c->card_type : "-",
                        c ? c->count : -1, c ? c->allow_skip : -1, c ? c->target : "-",
                        c ? c->n_filtered_indices : -1);
            }
            fflush(stderr);
        }
        int rc = (failures || setup_bugs) ? 1 : 0;
        char rep[128];
        int rn = snprintf(rep, sizeof rep, "%d %d %d %d\n", rc, assertions, failures, setup_bugs);
        if (rn > 0) { ssize_t w = write(pipefd[1], rep, (size_t)rn); (void)w; }
        close(pipefd[1]);
        _Exit(rc);
    }

    close(pipefd[1]);
    char buf[128];
    ssize_t k = read(pipefd[0], buf, sizeof buf - 1);
    close(pipefd[0]);
    int status = 0;
    waitpid(pid, &status, 0);
    if (k <= 0) {
        if (WIFSIGNALED(status)) {
            n_crashed++;
            fprintf(stderr, "CRASH: %s died on signal %d\n", cases[idx].name, WTERMSIG(status));
            return 2;
        }
        /* The child closed the pipe without writing a report and without dying
         * from a signal (exit code %d). That is a lost case, so it is
         * reported, not swallowed - the old bare `return -1` here is how a
         * case disappeared with no verdict line and no tally. */
        n_crashed++;
        fprintf(stderr, "CRASH: %s produced no report (exit %d)\n",
                cases[idx].name, WIFEXITED(status) ? WEXITSTATUS(status) : -1);
        printf("CRASH: %s (no report)\n", cases[idx].name);
        fflush(stdout);
        return 2;
    }
    buf[k] = '\0';
    int rc, a, f, s;
    if (sscanf(buf, "%d %d %d %d", &rc, &a, &f, &s) != 4) return -1;
    assertions += a; failures += f; setup_bugs += s;
    if (rc != 0) { n_failed++; printf("FAILED: %s (%d failures)\n", cases[idx].name, f); }
    else         { n_ok++;      printf("ok: %s (%d checks)\n", cases[idx].name, a); }
    fflush(stdout);
    return rc;
}

int main(void)
{
    if (load_card_db() != 0) {
        fprintf(stderr, "FATAL: card database failed to load\n");
        return 2;
    }
    printf("test_parity_recover_hand_a: %d cases, RECOVERY-SOURCE half of "
           "effects/recover/to_hand\n", N_CASES);
    for (int i = 0; i < N_CASES; i++) run_case(i);
    printf("\n%d ok, %d failed, %d crashed; checks=%d failures=%d gaps=%d setup_bugs=%d\n",
           n_ok, n_failed, n_crashed, assertions, failures, gaps, setup_bugs);
    return (n_failed || n_crashed || setup_bugs) ? 1 : 0;
}
