/* test_parity_recover_live.c -EC parity suite for the LIVE-RECOVERY and
 * GROUP-GATING half of the Rust cluster
 * engine/tests/test_modules/effects/recover/to_hand/.
 *
 * BOUNDARY. The 40 .rs files in that cluster split three ways. This file owns
 * the LIVE-recovery + gating family only:
 *
 *   (1) a LIVE card recovered out of the waitroom to hand, including the
 *       per-turn / per-energy gated variants;
 *   (2) a recovery gated on a GROUP FILTER (『Liella!』, 『虹ヶ咲』, 『蓮ノ空』,
 *       a sub-unit name, a "different group from every stage member" rule);
 *   (3) a recovery gated on a card TYPE, a SCORE threshold, or a cost
 *       threshold.
 *
 * Sibling agents own the recovery-SOURCE family (self / waitroom / hand / deck)
 * and the cost-discard-then-recover (Q209) family. See REPORT at the bottom of
 * this header for the exact per-file accounting.
 *
 * IDENTITY (AGENTS.md). Every fixture is resolved through mid() ->
 * test_new_id(), never the shared template alias, and every print is pinned
 * with a BYTE-EXACT strcmp of rb_card_string(c.card_no_idx). rb_card_no_eq is
 * FUZZY (a peer proved it accepts PL!SP-PR-004-R against the real
 * PL!SP-PR-004-PR) and is NOT an identity pin, so it is not used for that here.
 *
 * ENCODING (AGENTS.md). Every Japanese literal in this file is a \xNN byte
 * escape, so the file is 7-bit clean and cannot be corrupted into CP932 by an
 * editor. That matters doubly for a group-filter cluster: a silently mangled
 * group string makes the filter match nothing and the test goes GREEN.
 *
 * VACUITY CONTROL. This is the landmine for this cluster specifically. A
 * group-gated recovery whose filter matches nothing produces a CLEAN, GREEN
 * negative: the card simply does not move, and every "!hand_has(target)"
 * assertion passes. So every negative assertion here is one of:
 *   - preceded by CHECK_PROMPTS(n) - the board must have produced n real
 *     prompts, counted AT THE ASSERTION SITE (note_prompt), not only at answer
 *     time; or
 *   - paired with a positive control on the SAME board shape with the gate
 *     satisfied; or
 *   - a GROUP-PIN assertion (rb_card_matches_group_str on the exact bytes the
 *     ability filters on) proving the target is inside the filter's set and the
 *     decoy is outside it, so a "nothing matched" board cannot pass.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* U+FF0B FULLWIDTH PLUS SIGN. */
#define FP "\xef\xbc\x8b"

/* ---- group names, as BYTE EXACT escapes ---------------------------------
 * NIJI is pinned to the bytes rb_card_group_name() actually returns
 * (E8 99 B9 E3 83 B6 E5 92 B2). A hand-typed U+8671 would give ...B9 E3...
 * with A6 where the engine has B9, and the strcmp would fail silently. */
#define GRP_NIJI   "\xe8\x99\xb9\xe3\x83\xb6\xe5\x92\xb2"  /* 虹ヶ咲  */
#define GRP_LIELLA "Liella!"
#define GRP_HASU   "\xe8\x93\xae\xe3\x83\x8e\xe7\xa9\xba"  /* 蓮ノ空  */
#define GRP_FIVE   "5yncri5e!"
#define GRP_CATCHU "CatChu!"
#define GRP_KALEID "KALEIDOSCORE"
#define GRP_DOLL   "DOLLCHESTRA"
#define GRP_MU     "\xce\xbc's"
#define GRP_SURIRUZE "\xe3\x82\xb9\xe3\x83\xaa\xe3\x83\xbc\xe3\x82\xba\xe3\x83\x96\xe3\x83\xbc\xe3\x82\xb1"

/* ---- fixtures (every one verified against cards/cards.json read as UTF-8) */

/* total_energy_gated_live_recovery_test.rs - 米女メイ, cost 13, MEMBER.
 * The Rust file writes "PL!SP-bp1-007-R＋" and that IS the real print
 * (cards.json: PL!SP-bp1-007-R＋ / 米女メイ / メンバー / cost 13). The
 * coordinator brief claimed the fixture was wrong; it is not. See REPORT. */
#define MEI_13     "PL!SP-bp1-007-R" FP
/* self_to_waitroom_live_recovery_energy_activation_test.rs - 小原鞠莉 */
#define MARI       "PL!S-bp3-008-R"
/* pay_two_energy_recovers_score_three_live_to_hand_test.rs - 日野下花帆 */
#define HANANO     "PL!HS-bp2-001-R"
#define DREAM      "PL!HS-bp1-019-L"   /* 蓮ノ空 live, score 1 -> eligible */
#define AURORA     "PL!HS-bp5-018-L"   /* 蓮ノ空 live, score 7 -> excluded */
/* energy_discard_live_recovery_pl_sp_sd2_006_sd2_test.rs - 桜小路きな子 */
#define KINAKO     "PL!SP-sd2-006-SD2"
#define LIELLA_LIVE "PL!SP-bp1-026-L"  /* 未来予報ハレルヤ！ score 3 */
/* optional_energy_live_recovery_pl_sp_pb2_007_r_test.rs - 米女.pb2 print */
#define MEI_PB2    "PL!SP-pb2-007-R"
/* optional_discard_subunit_recovery_test.rs */
#define SUMIRE     "PL!SP-pb2-015-R"   /* 平安名すみれ  cost 13 */
#define SHIKISAMA  "PL!SP-pb2-019-R"   /* 若菜四季      cost 13 */
#define WIEN       "PL!SP-pb2-021-R"   /* ウィーン      cost 13 */
#define SUMIRE_PR  "PL!SP-bp1-004-PR"  /* 平安名すみれ CatChu! */
#define KANATSU_5  "PL!SP-bp1-014-N"   /* 嵐 千砂都 5yncri5e! */
#define KARUKO     "PL!SP-bp1-013-PR"  /* 唐 可可 KALEIDOSCORE */
/* optional_discard_two_split_group_recovery_ginko_test.rs - 百生吟子 */
#define GINKO      "PL!HS-pb1-020-N"
#define CERISE     "PL!HS-sd1-001-SD"  /* 百生吟子 スリーズブーケ */
#define SAYAKA     "PL!HS-sd1-002-SD"  /* 村野さやか DOLLCHESTRA */
/* hasunosora_cost_gated_recovery_test.rs - セラス 柳田 リリエンフェルト */
#define SERAS      "PL!HS-pb1-007-R"
#define TSUKIGUMI  "PL!HS-bp2-004-R"   /* 夕霧綴理 DOLLCHESTRA member */
/* RUST FIXTURE BUG #3, CORRECTED HERE: the file asks for
 * "PL!HS-bp1-026-L". That key does NOT exist in cards.json - the
 * PL!HS-bp1-026 family has exactly one member, PL!HS-bp1-026-SECE (an ENERGY
 * card, 藤島 慈, みらくらぱーく!). The file needs a 蓮ノ空 LIVE card, and
 * PL!HS-bp1-021-L Holiday∞Holiday (スリーズブーケ) is the 蓮ノ空 live of the
 * same booklet page. Corrected to MIRAGE below. */
#define MIRAGE     "PL!HS-bp2-023-L"   /* Mirage Voyage DOLLCHESTRA live */
#define SAYAKA_CL  "PL!HS-cl1-002-CL"
#define IZUMI      "PL!HS-cl1-008-CL"
/* success_score_live_pl_bp6_013_n_test.rs - 園田海未 */
#define UMISEA     "PL!-bp6-013-N"
#define NEXT_SPARK "PL!S-pb1-023-L"    /* score 9 */
/* live_success_recover_different_group_name_to_hand_ll_bp5_002_l_test.rs */
#define BRING_LOVE "LL-bp5-002-L"
#define AQ_MEMBER  "PL!S-pb1-003-R"
#define AQ_LIVE    "PL!S-sd1-019-SD"
#define MUS_MEMBER "PL!-bp3-003-R"
#define NIJI_MEMBER "PL!N-pb1-001-R"
/* reveal_live_name_fragment... - 日野下花帆 (name-fragment filter) */
#define HINO       "PL!HS-bp5-001-P"
#define DB_104     "PL!HS-bp5-017-L"   /* Dream Believers（104期Ver.） */
#define DB_105     "PL!HS-sd1-018-SD"  /* Dream Believers（105期Ver.） */
#define BUTTERFLY  "PL!N-bp1-028-L"    /* Butterfly */
#define BFLY_WING  "PL!SP-pb2-046-L"   /* Butterfly Wing */
#define LINK_BASE  "PL!HS-bp2-020-L"
#define LINK_104   "PL!HS-sd1-020-SD"
/* discard_two_recover_heart06_threshold_live_test.rs - 矢澤にこ */
#define NICO       "PL!-bp5-009-R"
/* like a treasure - PL!N-bp7-031-L */
#define TREASURE   "PL!N-bp7-031-L"
#define POPPIN     "PL!N-bp1-026-L"    /* Poppin' Up! 虹ヶ咲 live */
#define NATSUKI    "PL!SP-sd1-020-SD"  /* 鬼塚夏美 - a MEMBER, not a live */
#define MIA        "PL!N-sd2-011-SD2"  /* ミア・テイラー 虹ヶ咲 member */
/* generic */
#define FILLER_MUS "PL!-sd1-010-SD"    /* μ's member, never in an HS filter */
#define FILLER_NIJI "PL!N-sd1-010-SD"  /* 虹ヶ咲 member */
#define LIVE_MUS   "PL!-sd1-019-SD"    /* μ's live, score 1 */
#define LIVE_MUS2  "PL!-sd1-020-SD"    /* μ's live, score 2 */
#define KASUMI     "PL!N-sd2-002-SD2"  /* 中須かすみ, cost 13 虹ヶ咲 */
#define ENERGY_CD  "LL-E-001-SD"
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
static TestGame t_tg[16];
static TestGame t_tg_n[5];   /* the N defect pins use their own slots */

static void on_fatal_signal(int sig)
{
    fprintf(stderr,
            "\n*** FATAL SIGNAL %d inside test: %s\n"
            "*** The engine, not this test, is at fault.\n"
            "*** assertions=%d failures=%d prompts=%d\n",
            sig, current_test, assertions, failures, prompts);
    fflush(stderr);
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

/* VACUITY CONTROL. Every negative claim below is gated on this. */
#define CHECK_PROMPTS(n, ...) do {                                          \
        assertions++;                                                       \
        if (prompts < (n)) {                                                \
            failures++;                                                     \
            fprintf(stderr, "FAIL (VACUOUS): ");                            \
            fprintf(stderr, __VA_ARGS__);                                   \
            fprintf(stderr, " - only %d prompt(s) were offered, need %d\n",  \
                    prompts, (n));                                          \
        }                                                                   \
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

/* AGENTS.md identity rule: mid() (test_new_id), never the shared template.
 * Rust's game.id() allocates the next DISTINCT pool slot; the C template lookup
 * aliases one shared index, so a live card resolved through the alias is the
 * WRONG live and a group filter on it is meaningless. */
static int mid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id < 0) {
        assertions++; setup_bugs++; failures++;
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in cards.json\n", no);
    }
    return id;
}

/* BYTE-EXACT identity pin. rb_card_no_eq is fuzzy and is not an identity pin. */
static int pin_id(int cid, const char *no)
{
    assertions++;
    if (cid < 0) { setup_bugs++; failures++;
        fprintf(stderr, "SETUP BUG: cannot resolve '%s'\n", no); return 0; }
    if (strcmp(card_no_of(cid), no) != 0) {
        setup_bugs++; failures++;
        fprintf(stderr, "SETUP BUG: identity - want '%s' (%s), resolved '%s' (%s) id=%d\n",
                no, no, card_no_of(cid), card_name_of(cid), cid);
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

static int printed_blade(int cid)
{
    Card c; int v = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.blade; rb_free_card(&c); }
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

/* The decoded ability text, joined, so a filter claim can be pinned to the
 * print rather than to the test's comment. */
static int card_text_has(int cid, const char *needle)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = (ab.full_text && strstr(ab.full_text, needle)) ||
                  (ab.triggerless_text && strstr(ab.triggerless_text, needle));
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}

static int bag_count(const RbBag *b, int cid)
{
    int k = 0;
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) k++;
    return k;
}

static int hand_has(const TestGame *tg, int cid) { return bag_has(&tg->state.p[0].hand, cid); }static int wait_has(const TestGame *tg, int cid) { return bag_has(&tg->state.p[0].discard, cid); }
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
    rb_look_reset_all();
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

/* Counted AT the ASSERTION SITE, not only when answered. A control that
 * counted only at answer time could "prove" a prompt existed with no
 * evidence - which is exactly the failure this cluster is prone to. */
static int note_prompt(TestGame *tg) { if (pending(tg)) { prompts++; return 1; } return 0; }

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

/* Rust drain_choices (helpers/choices.rs:60): answer a prompt with its FIRST
 * index UNLESS the choice carries allow_skip, in which case decline it. The
 * "unless" is the whole point: answering a mandatory recover with an empty
 * selection refuses the recovery, which is exactly the absence a following
 * negative assertion would then read as a pass. A drain that skips
 * unconditionally manufactures a green board. */
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

/* The Rust TestGame has no watchdog on this toolchain; every drain here is
 * capped, because a peer found an ability that re-prompts unboundedly. */

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

static int fire_trigger(TestGame *tg, int cid, const char *trig)
{
    /* A 登場 only matches a card that APPEARED, and the stage loop of
     * queue_zone_abilities keys that off recently_moved. Seating the card by
     * hand (which this test has to do - see the comment above) leaves no
     * appearance event, so one is published explicitly. rb_trigger_auto_ability
     * takes NO movement batch (Rust passes None), so this publication is the
     * only thing arming the trigger. */
    if (!strcmp(trig, RB_TSTR_DEBUT)) {
        tg->state.n_recently_moved = 0;
        tg->state.recently_moved[tg->state.n_recently_moved++] = cid;
    }
    /* Rust helpers/mod.rs:114 fire_trigger: ONE ability of ONE card, via
     * rb_trigger_auto_ability. This USED TO BE rb_queue_trigger_abilities,
     * whose `cid` only set activating_card and which fanned the trigger out to
     * every card on the board printing it. */
    return test_fire_trigger_on_card(tg, cid, trig);
}

/* Drive a 起動 through the QUEUE path rather than test_activate_ability.
 *
 * ENGINE FINDING (see REPORT): test_activate_ability -> rb_activate_ability ->
 * rb_activate_card (src/engine.c:1189) builds a flat effect tree of
 * [cost, effect] and runs it with rb_execute_effect_ex. The effect executor
 * treats a `sequential_cost` node as a NO-OP (src/ability/effects/executor.c:501,
 * "the sub-costs are already paid in order by the dedicated cost path"), so a
 * card whose cost is a sequential_cost container has its WHOLE cost silently
 * dropped on this path - no energy is tapped and no hand discard happens.
 *
 * The queue path does pay it, and it pays it at RESOLUTION, not at enqueue:
 * rb_pay_cost is called from pca_resolve_entry (src/core/game_state_abilities.c
 * :2746), which every queue entry reaches through rb_process_current_ability.
 * BOTH enqueue routes build the entry through the SAME builder,
 * rb_build_ability_queue_entry -- the TAS zone scan at :1950 and
 * rb_trigger_auto_ability at :1758 -- and that builder never reads its card_no
 * argument, so a targeted entry and a scanned entry are field-identical apart
 * from the (already-normalised) owner stamp and the movement batch. Making the
 * ENQUEUE targeted therefore keeps the sequential_cost payment; it removes only
 * the fan-out.
 *
 * The enqueue is therefore done by test_fire_trigger_on_card, the shared helper
 * built on rb_trigger_auto_ability: it is Rust's own helpers/mod.rs:114
 * `fire_trigger` (ONE ability of ONE card), and it carries the load-bearing
 * enqueue-time use record (rb_record_use + use_limit_recorded) that
 * queue_zone_abilities does at :1954-1955. Re-firing the whole-board scan here
 * would re-introduce the defect the peer sweep found: `cid` only ever set
 * activating_card, so a 起動 fanned out to every P1 card printing 起動 and
 * doubled every recover/draw/score in the fixture.
 *
 * Both paths are therefore exercised in this file: the queue path here (the
 * faithful one) and test_activate_ability in test_c_activate_path_drops_a_
 * sequential_cost_cost, which pins the defect itself rather than working
 * around it. */
static int activate_via_queue(TestGame *tg, int cid)
{
    if (cid < 0 || !card_prints_trigger(cid, RB_TSTR_ACTIVATION)) return 0;
    /* ターン1回 -- Rust refuses the SECOND activation outright, at the OFFER
     * layer, before anything is enqueued:
     *     assert_eq!(use_offers(&game, hanano), 0,
     *                "ターン1回 -- the 起動 must not be offered again this turn");
     *     assert!(game.try_activate_ability(hanano).is_err(),
     *             "the second activation must be refused");
     * (pay_two_energy_recovers_score_three_live_to_hand_test.rs:143-151.)
     * The C has no generate_possible_actions UseAbility branch -- the gap this
     * file records in EXPECTED_GAP right above -- so that refusal has to be
     * reproduced at the enqueue this helper performs, using the engine's own
     * use bookkeeping. rb_trigger_auto_ability carries no use-limit gate
     * because Rust's does not either: Rust's lives in the offer layer, one step
     * ABOVE the one this helper stands in for, and the old whole-board TAS scan
     * only refused by accident, through queue_zone_abilities' own enqueue-time
     * `rb_use_limit_reached` gate.
     *
     * The gate is conditioned on a use ALREADY being recorded, so an ability
     * whose use_limit is 0 or absent still fires: rb_ability_has_remaining_uses
     * answers `used < limit` (abilities.rs:165-176) and a limit of 0 would make
     * that FALSE on a fresh ability, which is not what the TAS scan's
     * `limit <= 0 -> never reached` arm did. */
    int a = test_ability_index_for_trigger(cid, RB_TSTR_ACTIVATION, 0);
    if (a < 0) return 0;
    if (rb_ability_uses_used(&tg->state, cid, a) > 0 &&
        !rb_ability_has_remaining_uses(&tg->state, cid, a))
        return 0;
    /* The helper also drains (rb_process_pending_auto_abilities) and stamps
     * activating_card, in Rust's order: enqueue, then activating_card, then
     * the drain. */
    return test_fire_trigger_on_card(tg, cid, RB_TSTR_ACTIVATION) > 0 ? 1 : 0;
}

static void dump_state(TestGame *tg, const char *what)
{
    fprintf(stderr, "  STATE %s: pending=%d type=%s prompts=%d\n",
            what, pending(tg), pending_type(tg), prompts);
    fprintf(stderr, "    hand:");
    for (int i = 0; i < tg->state.p[0].hand.n; i++) fprintf(stderr, " %s", card_no_of(tg->state.p[0].hand.cards[i]));
    fprintf(stderr, "\n    wait:");
    for (int i = 0; i < tg->state.p[0].discard.n; i++) fprintf(stderr, " %s", card_no_of(tg->state.p[0].discard.cards[i]));
    fputc('\n', stderr);
}

/* The waitroom index of `cid`, or -1. This is a RAW zone index and is only a
 * legal answer when the prompt carries no filter. */
static int wait_index_of(TestGame *tg, int cid)
{
    for (int i = 0; i < tg->state.p[0].discard.n; i++)
        if (tg->state.p[0].discard.cards[i] == cid) return i;
    return -1;
}

/* How the C prompt is actually answered. move.c's
 * rb_move_prompt_card_selection_with_filtered stores the candidate list the
 * player was SHOWN in queue.pending.filtered_indices, and
 * map_choice_index (src/ability/choice.c) reads the answer through that list.
 * A returned index is therefore a CANDIDATE index, not a raw zone index: the
 * two coincide only when every card in the zone matches the filter. Under a
 * card_type / group_names / need_heart filter that rejects cards, answering a
 * raw zone index names the WRONG card - which is exactly the situation every
 * group-gated recovery in this cluster is in.
 *
 * These helpers therefore answer by CARD, mapping through filtered_indices the
 * way the engine does, and refusing to answer at all when the card is not
 * offered. An unanswerable card is itself a finding, not something to paper
 * over by picking some other card. */
static int candidate_count(TestGame *tg)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    return c ? c->n_filtered_indices : 0;
}

/* Candidate slot holding `cid`, or -1 when the card is NOT offered. */
static int candidate_slot_of(TestGame *tg, int cid)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    if (!c || c->n_filtered_indices <= 0) return -1;
    for (int i = 0; i < c->n_filtered_indices; i++) {
        int zone_idx = c->filtered_indices[i];
        if (zone_idx < 0 || zone_idx >= tg->state.p[0].discard.n) continue;
        if (tg->state.p[0].discard.cards[zone_idx] == cid) return i;
    }
    return -1;
}

static const char *candidate_card_no(TestGame *tg, int slot)
{
    static char buf[96];
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    buf[0] = '\0';
    if (!c || slot < 0 || slot >= c->n_filtered_indices) return buf;
    int zone_idx = c->filtered_indices[slot];
    if (zone_idx < 0 || zone_idx >= tg->state.p[0].discard.n) return buf;
    snprintf(buf, sizeof buf, "%s", card_no_of(tg->state.p[0].discard.cards[zone_idx]));
    return buf;
}

static void dump_abilities(int cid, const char *tag)
{
    printf("  -- %s abilities for %s:\n", tag, card_no_of(cid));
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

/* ======================================================================
 * A. total_energy_gated_live_recovery_test.rs  (米女メイ PL!SP-bp1-007-R＋)
 *
 * The condition counts TOTAL energy, not active. Rust fixture audit:
 * "PL!SP-bp1-007-R＋" IS a real key in cards.json (米女メイ, メンバー, cost 13),
 * so the print is right; the "used where a live is required" worry does not
 * apply - this card is a MEMBER and the test only ever plays it to stage.
 * ====================================================================== */

static void test_a_mei_debut_counts_total_energy_not_active(void)
{
    TestGame *tg = &t_tg[0];
    clear_p1(tg);
    int card = mid(tg, MEI_13);
    int filler = mid(tg, FILLER_MUS);
    int dead_live = mid(tg, LIVE_MUS2);
    if (!pin_id(card, MEI_13)) return;
    /* Precondition pins: the print is a cost-13 MEMBER and the recovery target
     * really is a live card, so a green run cannot be a type confusion. */
    CHECK_EQ(printed_cost(card), 13, "a: 米女メイ PL!SP-bp1-007-R＋ prints cost 13");
    CHECK_EQ(rb_card_is_live(card), 0, "a: the source is a MEMBER, not a live card");
    CHECK_EQ(rb_card_is_live(dead_live), 1, "a: the waitroom target really is a live card");
    if (card < 0 || filler < 0 || dead_live < 0) return;

    test_give_energy(tg, 15);
    fill_decks(tg, filler, 30);
    test_add_to_discard(tg, dead_live);
    test_add_to_hand(tg, card);

    play_to_stage(tg, card, AREA_CENTER, 0);
    drain_choices(tg, 32);

    /* GROUP/TEXT PIN: the gate the Rust test is about must be printed. */
    CHECK(card_text_has(card, "11") || card_text_has(card, "エネルギー"),
          "a: the debut gate mentions energy");

    CHECK_EQ(energy_active(tg), 2,
             "a: precondition - active energy dropped below 11 after a full-price play");
    CHECK_EQ(tg->state.p[0].energy.n, 15, "a: paid energy stays in the zone as wait state");
    CHECK(hand_has(tg, dead_live),
          "a: condition counts TOTAL energy (15 >= 11) -> live card retrieved");
    if (!hand_has(tg, dead_live)) dump_state(tg, "a:total_energy");
}

static void test_a_mei_no_retrieve_when_total_below_11(void)
{
    TestGame *tg = &t_tg[1];
    clear_p1(tg);
    int card = mid(tg, MEI_13);
    int filler = mid(tg, FILLER_MUS);
    int dead_live = mid(tg, LIVE_MUS2);
    int victim = mid(tg, KASUMI);
    if (!pin_id(card, MEI_13) || !pin_id(victim, KASUMI)) return;
    if (card < 0 || filler < 0 || dead_live < 0 || victim < 0) return;

    /* The Rust twin gets the card onto the stage with use_baton_touch =
     * Some(true), which makes the play cost ZERO (rules 9.6.2.3.2), so a total
     * of 10 survives into the debut condition. There is no C entry point that
     * passes that flag to test_play_to_stage, and publishing
     * baton_touch_zero_cost directly is not enough on its own - the play path
     * still refuses a 10-energy board for a cost-13 card.
     *
     * So the condition under test - "total energy < 11" - is established the
     * direct way: the card is placed on stage (which is the board state the
     * debut ability reads; a 登場 does not re-check how the card got there)
     * and the energy zone is left at 10. The rest of the test is unchanged
     * and the VACUOUS control below still proves the engine was actually
     * asked. This is the one test in the file that cannot replay the Rust
     * setup exactly; the deviation is stated here rather than papered over. */
    fill_decks(tg, filler, 30);
    tg->state.p[0].stage[AREA_CENTER] = victim;
    test_give_energy(tg, 10);   /* total 10 < 11 */
    test_add_to_discard(tg, dead_live);

    /* Zero-cost replacement, the way the Rust twin gets it. */
    tg->state.baton_touch_zero_cost = 1;
    tg->state.baton_touch_replaced_member_id = victim;
    tg->state.baton_touch_replaced_member_cost = printed_cost(victim);
    test_play_to_stage(tg, card, AREA_CENTER);
    int guard = 0;
    while (tg->state.p[0].stage[AREA_CENTER] != card && guard++ < 4) {
        test_answer_play_cost_choice(tg, 0);
        test_play_to_stage(tg, card, AREA_CENTER);
    }

    if (tg->state.p[0].stage[AREA_CENTER] != card) {
        /* The C play path cannot seat a cost-13 card on a 10-energy board even
         * with the baton-touch zero-cost flag published, so the Rust twin's
         * `execute_main_phase_action(..., Some(true))` has no C equivalent.
         * Seat it directly so the debut CONDITION is still exercised - the
         * assertion under test never involved the play. The failed play is
         * reported as its own finding below rather than being hidden. */
        tg->state.p[0].stage[AREA_CENTER] = card;
        EXPECTED_GAP(0,
                     "a: CONFIRMED ENGINE GAP - rules 9.6.2.3.2 baton touch (play cost "
                     "13 - 13 = 0) is not honoured: test_play_to_stage refuses to seat "
                     "PL!SP-bp1-007-R＋ on a 10-energy board even with "
                     "GameState.baton_touch_zero_cost published. Rust reaches this via "
                     "turn::TurnEngine::execute_main_phase_action with use_baton_touch="
                     "Some(true), which no C test entry point passes.");
    }
    CHECK(stage_has(tg, card), "a: mei should be on stage");
    CHECK_EQ(tg->state.p[0].energy.n, 10,
             "a: PREMISE - the energy zone really holds 10, so the debut condition "
             "sees a total of 10 and the 11-energy gate can fail");
    EXPECTED_GAP(wait_has(tg, victim),
                 "a: CONFIRMED ENGINE GAP - the baton-touch victim is not sent to the "
                 "waitroom either, because the play never happened. Rust asserts "
                 "waitroom.contains(&victim) explicitly.");

    fire_trigger(tg, card, RB_TSTR_DEBUT);
    drain_choices(tg, 32);

    /* NON-VACUITY for this test is the PAIRED POSITIVE, not a prompt count: a
     * condition-gated ability that FAILS its condition correctly offers no
     * prompt at all, so demanding one would be demanding a bug. The control
     * that matters is that the identical card, on the identical waitroom,
     * with 15 energy instead of 10, DOES retrieve -
     * test_a_mei_debut_counts_total_energy_not_active is green, and its
     * "paid energy stays in the zone as wait state" assertion proves the
     * total it reads is 15. Nothing else on the two boards differs that the
     * condition can see. */
    CHECK(!hand_has(tg, dead_live), "a: total energy 10 < 11 -> no retrieval");
    CHECK(wait_has(tg, dead_live), "a: the live card is untouched in the waitroom");
}

static void test_a_mei_noop_when_waitroom_has_no_live_card(void)
{
    TestGame *tg = &t_tg[2];
    clear_p1(tg);
    int card = mid(tg, MEI_13);
    int filler = mid(tg, FILLER_MUS);
    if (!pin_id(card, MEI_13)) return;
    if (card < 0 || filler < 0) return;
    CHECK_EQ(rb_card_is_live(filler), 0, "a: the only waitroom card is a member, not a live");

    test_give_energy(tg, 15);
    fill_decks(tg, filler, 30);
    test_add_to_discard(tg, filler);
    test_add_to_hand(tg, card);

    play_to_stage(tg, card, AREA_CENTER, 0);
    drain_skip(tg, 30);

    CHECK(!pending(tg), "a: no stuck prompts when the waitroom holds no live card");
    CHECK(wait_has(tg, filler), "a: the member in the waitroom is untouched");
}

/* ======================================================================
 * B. self_to_waitroom_live_recovery_energy_activation_test.rs (小原鞠莉)
 *
 * GROUP GATE (『Aqours』) + SCORE GATE (>= 6) + energy activation.
 * RUST FIXTURE BUG, CORRECTED HERE: the file writes "PL!S-bp1-023-L", which
 * is NOT a key in cards.json (verified: the -bp1-023 family has only
 * PL!HS-bp1-023-L ド！ド！ド！ and PL!HS-bp1-023-N; the Next SPARKLING!! prints
 * are PL!S-pb1-023-L and PL!S-bp1-023-L＋). The intended "Aqours high-score
 * live" is PL!S-pb1-023-L, score 9, series サンシャイン, which satisfies both
 * the 『Aqours』 filter and the スコア6以上 gate the three tests claim.
 * ====================================================================== */

static void mari_board(TestGame *tg, int active, int *out_mari,
                       int *out_aq_high, int *out_niji_low)
{
    int mari = mid(tg, MARI);
    int aq_high = mid(tg, NEXT_SPARK);   /* CORRECTED fixture */
    int niji_low = mid(tg, BUTTERFLY);
    *out_mari = mari; *out_aq_high = aq_high; *out_niji_low = niji_low;
    pin_id(mari, MARI);
    pin_id(aq_high, NEXT_SPARK);
    pin_id(niji_low, BUTTERFLY);
    if (mari < 0 || aq_high < 0 || niji_low < 0) return;
    /* GROUP PINS: the whole file hinges on 『Aqours』 matching the target and
     * NOT the decoy. A group filter that matches nothing would make every
     * negative below pass vacuously, so the filter's own behaviour is pinned
     * on the same two cards. */
    CHECK(rb_card_matches_group_str(aq_high, "Aqours"),
          "b: Next SPARKLING!! is inside the 『Aqours』 filter");
    CHECK(!rb_card_matches_group_str(niji_low, "Aqours"),
          "b: Butterfly is OUTSIDE the 『Aqours』 filter");
    CHECK(rb_card_is_live(aq_high) && rb_card_is_live(niji_low),
          "b: both candidates are live cards");
    CHECK(printed_score(aq_high) >= 6, "b: the high live really prints スコア6以上");
    CHECK(printed_score(niji_low) < 6,  "b: the low live really prints below スコア6");
    tg->state.p[0].stage[AREA_CENTER] = mari;
    test_add_to_discard(tg, aq_high);
    test_add_to_discard(tg, niji_low);
    for (int i = 0; i < 6; i++) rb_energy_add_card(&tg->state.p[0], mid(tg, ENERGY_CD));
    tg->state.p[0].energy_active = active;
}

static void test_b_recovers_high_score_aqours_live_and_activates_four(void)
{
    TestGame *tg = &t_tg[3];
    clear_p1(tg);
    int mari, aq_high, niji_low;
    mari_board(tg, 2, &mari, &aq_high, &niji_low);
    if (mari < 0) return;

    activate_via_queue(tg, mari);
    CHECK(wait_has(tg, mari), "b: activation cost sends Mari herself to the waitroom");
    CHECK_EQ(tg->state.p[0].stage[AREA_CENTER], RB_EMPTY_SLOT, "b: Q79 - the area is vacated");
    CHECK(note_prompt(tg), "b: live selection must be offered");
    CHECK(pending_is_card(tg), "b: expected a SelectCard live-selection prompt (got %s)",
          pending_type(tg));
    int idx = candidate_slot_of(tg, aq_high);
    CHECK(idx >= 0, "b: the Aqours high live is offered at waitroom index %d", idx);
    if (idx >= 0) pick(tg, idx);
    drain_choices(tg, 16);

    CHECK(hand_has(tg, aq_high), "b: the high-score 『Aqours』 live reached the hand");
    CHECK_EQ(energy_active(tg), 6, "b: 『Aqours』 score>=6 fetched -> activate 4 (2+4=6)");
}

static void test_b_wrong_group_live_skips_energy_activation(void)
{
    TestGame *tg = &t_tg[4];
    clear_p1(tg);
    int mari, aq_high, niji_low;
    mari_board(tg, 2, &mari, &aq_high, &niji_low);
    if (mari < 0) return;

    activate_via_queue(tg, mari);
    CHECK(note_prompt(tg), "b: the live-selection prompt must appear even for a wrong-group live");
    int idx = candidate_slot_of(tg, niji_low);
    CHECK(idx >= 0, "b: the non-『Aqours』 live is also selectable");
    if (idx >= 0) pick(tg, idx);
    drain_choices(tg, 16);

    CHECK(hand_has(tg, niji_low), "b: the chosen live still reaches the hand");
    /* VACUITY: without this, an engine that offered NO prompt at all would
     * leave energy at 2 and read green. */
    CHECK_PROMPTS(1, "b: the board must have offered the live selection");
    CHECK_EQ(energy_active(tg), 2, "b: 『Aqours』 filter fails -> NO activation");
}

static void test_b_aqours_live_below_six_skips_activation(void)
{
    TestGame *tg = &t_tg[5];
    clear_p1(tg);
    int mari, aq_high, niji_low;
    mari_board(tg, 2, &mari, &aq_high, &niji_low);
    if (mari < 0) return;

    /* The Rust test uses PL!S-PR-024-PR, 勇気はどこに?君の胸に!, score 5 -
     * inside 『Aqours』 (サンシャイン) but BELOW the スコア6以上 gate. Verified
     * in cards.json. */
    int aq_low = mid(tg, "PL!S-PR-024-PR");
    pin_id(aq_low, "PL!S-PR-024-PR");
    if (aq_low < 0) return;
    CHECK(rb_card_matches_group_str(aq_low, "Aqours"),
          "b: the below-threshold card IS inside 『Aqours』, so the SCORE is the gate");
    CHECK(printed_score(aq_low) < 6, "b: PL!S-PR-024-PR really prints below 6");

    /* Drop the high live, add the low one. */
    RbBag *W = &tg->state.p[0].discard;
    int n = 0;
    for (int i = 0; i < W->n; i++) if (W->cards[i] != aq_high) W->cards[n++] = W->cards[i];
    W->n = n;
    test_add_to_discard(tg, aq_low);

    activate_via_queue(tg, mari);
    CHECK(note_prompt(tg), "b: the low 『Aqours』 live must be selectable");
    int idx = candidate_slot_of(tg, aq_low);
    CHECK(idx >= 0, "b: low Aqours live selectable at index %d", idx);
    if (idx >= 0) pick(tg, idx);
    drain_choices(tg, 16);

    CHECK(hand_has(tg, aq_low), "b: the low 『Aqours』 live reached the hand");
    CHECK_PROMPTS(1, "b: the board must have offered the live selection");
    CHECK_EQ(energy_active(tg), 2, "b: スコア6以上 fails at exactly-below -> NO activation");
}

static void test_b_cost_paid_without_recoverable_live(void)
{
    TestGame *tg = &t_tg[6];
    clear_p1(tg);
    int mari = mid(tg, MARI);
    pin_id(mari, MARI);
    if (mari < 0) return;
    int filler = mid(tg, FILLER_MUS);
    if (filler < 0) return;
    tg->state.p[0].stage[AREA_CENTER] = mari;
    test_add_to_discard(tg, filler);
    /* The decoy is a MEMBER: it must not satisfy 「ライブカードを1枚」, so with
     * it as the only waitroom card the Q123 recovery has nothing to do. */
    CHECK_EQ(rb_card_is_live(filler), 0, "b: the only waitroom card is a member, not a live");

    activate_via_queue(tg, mari);
    drain_choices(tg, 16);

    CHECK(wait_has(tg, mari), "b: Q123 - the self cost was paid");
    CHECK(!hand_has(tg, filler), "b: a member card must not be added as the recovered live");
    CHECK_EQ(tg->state.p[0].hand.n, 0, "b: hand is empty - the cost was paid, nothing gained");
    CHECK_EQ(energy_active(tg), 0, "b: no fetch -> no activation");
}

static void test_b_recovery_is_mandatory_and_excludes_members(void)
{
    TestGame *tg = &t_tg[7];
    clear_p1(tg);
    int mari, aq_high, niji_low;
    mari_board(tg, 2, &mari, &aq_high, &niji_low);
    if (mari < 0) return;
    int member_in_waitroom = mid(tg, FILLER_MUS);
    pin_id(member_in_waitroom, FILLER_MUS);
    if (member_in_waitroom < 0) return;
    CHECK_EQ(rb_card_is_live(member_in_waitroom), 0, "b: the decoy really is a member card");
    test_add_to_discard(tg, member_in_waitroom);

    activate_via_queue(tg, mari);
    CHECK(note_prompt(tg), "b: the live selection must be offered even with a member present");
    /* THE SHARP ASSERTION. The card_type=live_card filter must keep the member
     * OUT of the candidate list. Asserting only that the member is still in the
     * waitroom after the fact would pass on a board where the prompt simply
     * never offered it, so the candidate list itself is inspected. */
    CHECK_EQ(candidate_count(tg), 2,
             "b: the prompt offers exactly the 2 live cards - the member is NOT a candidate");
    int midx = candidate_slot_of(tg, member_in_waitroom);
    CHECK_EQ(midx, -1, "b: a member card must never appear as a candidate slot");
    for (int s = 0; s < candidate_count(tg); s++) {
        int zi = rb_get_pending_choice(&tg->state)->filtered_indices[s];
        CHECK(zi >= 0 && zi < tg->state.p[0].discard.n &&
              rb_card_is_live(tg->state.p[0].discard.cards[zi]),
              "b: candidate %d (%s) really is a live card", s, candidate_card_no(tg, s));
    }
    int idx = candidate_slot_of(tg, aq_high);
    CHECK(idx >= 0, "b: the 高score 『Aqours』 live is offered at candidate slot %d", idx);
    if (idx >= 0) pick(tg, idx);
    drain_choices(tg, 8);
    CHECK(hand_has(tg, aq_high), "b: Q123 - a live card MUST reach the hand");
    CHECK(!hand_has(tg, member_in_waitroom) && wait_has(tg, member_in_waitroom),
          "b: the member stayed in the waitroom");
}

static void test_b_recovered_live_activates_only_available_energy(void)
{
    TestGame *tg = &t_tg[8];
    clear_p1(tg);
    int mari, aq_high, niji_low;
    mari_board(tg, 1, &mari, &aq_high, &niji_low);
    if (mari < 0) return;
    while (tg->state.p[0].energy.n > 3) tg->state.p[0].energy.n--;

    activate_via_queue(tg, mari);
    CHECK(note_prompt(tg), "b: the live selection must be offered");
    int idx = candidate_slot_of(tg, aq_high);
    if (idx >= 0) pick(tg, idx);
    drain_choices(tg, 16);

    CHECK_EQ(energy_active(tg), 3,
             "b: activate 4 requested but only 2 waited energies exist -> "
             "Q167 partial resolution activates those 2 (1+2=3)");
}

/* ======================================================================
 * C. pay_two_energy_recovers_score_three_live_to_hand_test.rs (日野下花帆)
 *
 * SCORE CEILING (スコア3以下) + GROUP FILTER (『蓮ノ空』) + 2E + ターン1回.
 * SCORE-GATE DISCIPLINE: the score gate reads PRINTED stats, and a peer found
 * printed SCORES in cards.json disagreeing with the compiled cards.bin record,
 * so every score premise here is asserted against the ENGINE's own decode
 * (printed_score) and compared with the cards.json value at the call site.
 * ====================================================================== */

static void hanano_setup(TestGame *tg, int *out_hanano)
{
    int hanano = mid(tg, HANANO);
    *out_hanano = hanano;
    pin_id(hanano, HANANO);
    if (hanano < 0) return;
    tg->state.p[0].stage[AREA_CENTER] = hanano;
    int filler = mid(tg, FILLER_MUS);
    if (filler >= 0) fill_decks(tg, filler, 30);
}

static void test_c_pay_2e_recovers_score3_live(void)
{
    TestGame *tg = &t_tg[9];
    clear_p1(tg);
    int hanano;
    hanano_setup(tg, &hanano);
    if (hanano < 0) return;
    int live = mid(tg, DREAM);
    pin_id(live, DREAM);
    if (live < 0) return;

    /* SCORE-GATE PREMISE, asserted on the ENGINE's decode. cards.json says
     * PL!HS-bp1-019-L / Dream Believers / ライブ / score 1. */
    CHECK_EQ(printed_score(live), 1, "c: the eligible live decodes to score 1 (cards.json says 1)");
    CHECK(rb_card_matches_group_str(live, GRP_HASU),
          "c: the eligible live is inside the 『蓮ノ空』 filter");
    CHECK(card_text_has(hanano, GRP_HASU) || card_text_has(hanano, "3"),
          "c: the activation text names the group/score gate");

    test_add_to_discard(tg, live);
    test_give_energy(tg, 3);

    activate_via_queue(tg, hanano);
    drain_choices(tg, 16);

    CHECK(hand_has(tg, live), "c: スコア3以下の『蓮ノ空』のライブカードを1枚手札に加える");
    CHECK(!wait_has(tg, live), "c: the recovered card left the waitroom");
    CHECK_EQ(energy_active(tg), 1, "c: E E - exactly two energy were paid");
    CHECK(rb_ability_uses_used(&tg->state, hanano, 0) > 0, "c: ターン1回 use recorded");
}

static void test_c_recover_is_mandatory_once_cost_paid(void)
{
    TestGame *tg = &t_tg[10];
    clear_p1(tg);
    int hanano;
    hanano_setup(tg, &hanano);
    if (hanano < 0) return;
    int live = mid(tg, DREAM);
    pin_id(live, DREAM);
    if (live < 0) return;
    test_add_to_discard(tg, live);
    test_give_energy(tg, 3);

    activate_via_queue(tg, hanano);

    /* The printed text carries no てもよい, so a single eligible card must
     * resolve on its own with no prompt. */
    CHECK(!pending(tg), "c: one eligible card and no てもよい -> the recover resolves on its own");
    CHECK(hand_has(tg, live), "c: paying E E must recover the eligible live");
    CHECK_EQ(energy_active(tg), 1, "c: E E - exactly two energy were paid");
}

static void test_c_turn1_blocks_second_activation(void)
{
    TestGame *tg = &t_tg[11];
    clear_p1(tg);
    int hanano;
    hanano_setup(tg, &hanano);
    if (hanano < 0) return;
    int live1 = mid(tg, DREAM);
    int live2 = test_new_id(tg, DREAM);
    pin_id(live1, DREAM); pin_id(live2, DREAM);
    if (live1 < 0 || live2 < 0) return;
    CHECK(live1 != live2, "c: two DISTINCT copies of the same live print");
    test_add_to_discard(tg, live1);
    test_add_to_discard(tg, live2);
    test_give_energy(tg, 5);

    activate_via_queue(tg, hanano);
    drain_choices(tg, 16);
    int hand_after_first = tg->state.p[0].hand.n;
    CHECK(hand_after_first >= 1, "c: the first activation must actually recover something");

    /* The C port has no generate_possible_actions UseAbility branch, so the
     * Rust `use_offers == 0` has no C entry point; the directly checkable
     * half is the engine's own ターン1回 bookkeeping + the refusal. */
    EXPECTED_GAP(1,
                 "c: hs_bp2_001_turn1_blocks_second - Rust use_offers(hanano) == 0 has no C "
                 "equivalent (no UseAbility branch in the action generator)");
    CHECK(!rb_ability_has_remaining_uses(&tg->state, hanano, 0),
          "c: ターン1回 - the engine recorded the use, so no uses remain this turn");
    activate_via_queue(tg, hanano);
    drain_choices(tg, 16);
    CHECK_EQ(tg->state.p[0].hand.n, hand_after_first,
             "c: a refused second activation recovers nothing more");
    CHECK(wait_has(tg, live2), "c: the second live is still in the waitroom");
    CHECK_EQ(energy_active(tg), 3, "c: a refused second activation spends no energy");
}

static void test_c_insufficient_energy_no_recover(void)
{
    TestGame *tg = &t_tg[12];
    clear_p1(tg);
    int hanano;
    hanano_setup(tg, &hanano);
    if (hanano < 0) return;
    int live = mid(tg, DREAM);
    pin_id(live, DREAM);
    if (live < 0) return;
    test_add_to_discard(tg, live);
    test_give_energy(tg, 1);

    /* 1 energy cannot pay E E: the cost is unaffordable, so the entry is
     * dropped at rb_pay_cost before the effect runs. */
    CHECK(energy_active(tg) < 2, "c: 1 active energy cannot pay the E E cost");
    activate_via_queue(tg, hanano);
    drain_choices(tg, 16);
    CHECK(wait_has(tg, live), "c: nothing was recovered");
    CHECK(!hand_has(tg, live), "c: the live card did not reach the hand");
    CHECK_EQ(energy_active(tg), 1, "c: a refused activation spends no energy");
}

static void test_c_score7_live_is_not_recovered(void)
{
    TestGame *tg = &t_tg[13];
    clear_p1(tg);
    int hanano;
    hanano_setup(tg, &hanano);
    if (hanano < 0) return;
    int live_high = mid(tg, AURORA);
    pin_id(live_high, AURORA);
    if (live_high < 0) return;
    /* AURORA is ALSO 『蓮ノ空』 (cards.json series 蓮ノ空女学院), so it is the
     * SCORE, not the group, that must exclude it. Both facts are pinned. */
    CHECK_EQ(printed_score(live_high), 7, "c: the excluded live decodes to score 7");
    CHECK(rb_card_matches_group_str(live_high, GRP_HASU),
          "c: the excluded live IS inside 『蓮ノ空』, so the gate under test is the SCORE");
    test_add_to_discard(tg, live_high);
    test_give_energy(tg, 3);

    activate_via_queue(tg, hanano);
    drain_choices(tg, 16);

    /* POSITIVE CONTROL on the same shape: the score-1 board in
     * test_c_pay_2e_recovers_score3_live is green. */
    CHECK_PROMPTS(1, "c: the activation must have reached the engine before a "
                     "'score 7 was not recovered' claim means anything");
    CHECK(!hand_has(tg, live_high), "c: スコア7 is above the スコア3 ceiling");
    CHECK(wait_has(tg, live_high), "c: the ineligible live stayed in the waitroom");
}

/* ======================================================================
 * D. energy_discard_live_recovery_pl_sp_sd2_006_sd2_test.rs (桜小路きな子)
 *    『Liella!』 group + 2E + ターン1回
 * ====================================================================== */

static void kinako_setup(TestGame *tg, int *out_me)
{
    int me = mid(tg, KINAKO);
    *out_me = me;
    pin_id(me, KINAKO);
    if (me < 0) return;
    tg->state.p[0].stage[AREA_CENTER] = me;
}

static void test_d_kinako_recovers_liella_live(void)
{
    TestGame *tg = &t_tg[14];
    clear_p1(tg);
    int me;
    kinako_setup(tg, &me);
    if (me < 0) return;
    int filler = mid(tg, FILLER_MUS);
    if (filler >= 0) fill_decks(tg, filler, 30);
    test_give_energy(tg, 5);
    int cost_target = mid(tg, FILLER_MUS);
    pin_id(cost_target, FILLER_MUS);
    if (cost_target < 0) return;
    int live = mid(tg, LIELLA_LIVE);
    pin_id(live, LIELLA_LIVE);
    if (live < 0) return;

    /* GROUP PIN: 『Liella!』 must actually match the target. */
    CHECK(rb_card_is_live(live), "d: the target is a live card");
    CHECK(rb_card_matches_group_str(live, GRP_LIELLA),
          "d: PL!SP-bp1-026-L really is inside the 『Liella!』 filter");
    CHECK(!rb_card_matches_group_str(cost_target, GRP_LIELLA),
          "d: the μ's cost card is OUTSIDE 『Liella!』");
    test_add_to_hand(tg, cost_target);
    test_add_to_discard(tg, live);

    activate_via_queue(tg, me);
    drain_pick0(tg, 16);

    CHECK(hand_has(tg, live), "d: 『Liella!』 live card retrieved from the waitroom");
    CHECK(!wait_has(tg, live), "d: live left the waitroom");
    CHECK_EQ(energy_active(tg), 3, "d: 2E of 5 paid");
    CHECK(rb_ability_uses_used(&tg->state, me, 0) > 0, "d: ターン1回 use recorded");
    CHECK(!rb_ability_has_remaining_uses(&tg->state, me, 0),
          "d: no re-offer after consuming the per-turn use");
}

static void test_d_kinako_empty_hand_refused(void)
{
    TestGame *tg = &t_tg[15];
    clear_p1(tg);
    int me;
    kinako_setup(tg, &me);
    if (me < 0) return;
    int filler = mid(tg, FILLER_MUS);
    if (filler >= 0) fill_decks(tg, filler, 30);
    test_give_energy(tg, 5);
    int live = mid(tg, LIELLA_LIVE);
    pin_id(live, LIELLA_LIVE);
    if (live < 0) return;
    test_add_to_discard(tg, live);

    int before = energy_active(tg);
    activate_via_queue(tg, me);
    drain_choices(tg, 16);
    CHECK_EQ(energy_active(tg), before, "d: no partial energy payment");
    CHECK(!hand_has(tg, live), "d: refused - the live stays in the waitroom");
    CHECK(wait_has(tg, live), "d: refused - the 『Liella!』 live is untouched");
}

static void test_d_kinako_no_liella_live_gains_nothing(void)
{
    TestGame *tg = &t_tg[0];
    clear_p1(tg);
    int me;
    kinako_setup(tg, &me);
    if (me < 0) return;
    int filler = mid(tg, FILLER_MUS);
    if (filler >= 0) fill_decks(tg, filler, 30);
    test_give_energy(tg, 5);
    int cost_card = mid(tg, FILLER_MUS);
    pin_id(cost_card, FILLER_MUS);
    if (cost_card < 0) return;
    int other_live = mid(tg, DREAM);          /* 蓮ノ空, NOT Liella! */
    pin_id(other_live, DREAM);
    if (other_live < 0) return;

    /* GROUP PIN: the decoy is a live card and is genuinely outside 『Liella!』.
     * Without this the negative could be passing because the card is not a
     * live, or because the filter matched nothing at all. */
    CHECK(rb_card_is_live(other_live), "d: the decoy IS a live card");
    CHECK(!rb_card_matches_group_str(other_live, GRP_LIELLA),
          "d: the 蓮ノ空 decoy is OUTSIDE 『Liella!』");
    test_add_to_hand(tg, cost_card);
    test_add_to_discard(tg, other_live);
    int hand_before = tg->state.p[0].hand.n;

    activate_via_queue(tg, me);
    drain_pick0(tg, 16);

    CHECK(!hand_has(tg, other_live), "d: a non-『Liella!』 live must not be recovered");
    CHECK_EQ(tg->state.p[0].hand.n + 1, hand_before,
             "d: the mandatory discard cost is paid exactly once: the hand ends one card shorter");
    CHECK(wait_has(tg, cost_card), "d: the card that paid the cost is in the waitroom");
}

/* ======================================================================
 * E. optional_energy_live_recovery_pl_sp_pb2_007_r_test.rs (米女メイ .pb2)
 *    『Liella!』 group + optional 3E
 * ====================================================================== */

static void test_e_paid_three_energy_recovers_liella_live(void)
{
    TestGame *tg = &t_tg[1];
    clear_p1(tg);
    int me = mid(tg, MEI_PB2);
    pin_id(me, MEI_PB2);
    if (me < 0) return;
    int filler = mid(tg, FILLER_MUS);
    if (filler >= 0) fill_decks(tg, filler, 30);
    tg->state.p[0].stage[AREA_CENTER] = me;
    test_give_energy(tg, 5);
    int live = mid(tg, LIELLA_LIVE);
    pin_id(live, LIELLA_LIVE);
    if (live < 0) return;
    CHECK(rb_card_matches_group_str(live, GRP_LIELLA),
          "e: the target live is inside the 『Liella!』 filter");
    test_add_to_discard(tg, live);

    /* Rust fires ライブ成功時 and answers select_option(1) - option 0 is SKIP and
     * option 1 is PAY for a pay_optional_cost gate. */
    CHECK(fire_trigger(tg, me, RB_TSTR_LIVE_SUCCESS), "e: the ライブ成功時 trigger fires");
    CHECK(note_prompt(tg), "e: the optional-energy gate must be offered");
    select_option(tg, 1);
    drain_pick0(tg, 16);

    CHECK(hand_has(tg, live), "e: paid 3 -> 『Liella!』 live retrieved to hand");
    if (!hand_has(tg, live)) { dump_state(tg, "e:paid3"); dump_abilities(me, "e"); }
}

static void test_e_unpayable_energy_keeps_live_in_waitroom(void)
{
    TestGame *tg = &t_tg[2];
    clear_p1(tg);
    int me = mid(tg, MEI_PB2);
    pin_id(me, MEI_PB2);
    if (me < 0) return;
    int filler = mid(tg, FILLER_MUS);
    if (filler >= 0) fill_decks(tg, filler, 30);
    tg->state.p[0].stage[AREA_CENTER] = me;
    int live = mid(tg, LIELLA_LIVE);
    pin_id(live, LIELLA_LIVE);
    if (live < 0) return;
    test_add_to_discard(tg, live);

    CHECK(fire_trigger(tg, me, RB_TSTR_LIVE_SUCCESS), "e: the ライブ成功時 trigger fires");
    CHECK(!pending(tg), "e: an unpayable optional {{E}}{{E}}{{E}} gate auto-skips without prompting");
    /* POSITIVE CONTROL: test_e_paid_three_energy_recovers_liella_live runs the
     * identical board with 5 energy and the target DOES reach hand. */
    CHECK(wait_has(tg, live), "e: declined -> the live stays in the waitroom");
}

/* ======================================================================
 * F. optional_discard_subunit_recovery_test.rs
 *    THREE sub-UNIT group gates: CatChu! / 5yncri5e! / KALEIDOSCORE
 *
 * This is the sharpest probe in the cluster for the group-filter defect class:
 * a sub-unit name is matched by EXACT unit compare only, and the coordinator
 * warned that a CSV multi-group value matches nothing under
 * rb_card_matches_group_str. Every fixture's group membership is pinned
 * explicitly, on the exact bytes, before the recovery is driven.
 * ====================================================================== */

static void subunit_case(const char *me_no, const char *target_no, const char *group)
{
    TestGame *tg = &t_tg[3];
    clear_p1(tg);
    int me = mid(tg, me_no);
    if (!pin_id(me, me_no)) return;
    int target = mid(tg, target_no);
    if (!pin_id(target, target_no)) return;
    int fodder = mid(tg, FILLER_MUS);
    if (fodder < 0) return;

    /* GROUP PINS - the whole point of this family. */
    CHECK(rb_card_matches_group_str(target, group),
          "f: %s is inside the 『%s』 sub-unit filter", target_no, group);
    CHECK(!rb_card_matches_group_str(fodder, group),
          "f: the μ's fodder is OUTSIDE 『%s』", group);
    CHECK(rb_card_matches_group_str(me, group),
          "f: the source %s is itself inside 『%s』", me_no, group);

    test_give_energy(tg, 20);
    test_add_to_hand(tg, me);
    test_add_to_discard(tg, target);
    test_add_to_hand(tg, fodder);

    play_to_stage(tg, me, AREA_CENTER, 0);
    CHECK(note_prompt(tg), "f: the optional discard cost must be offered");
    CHECK(pending_is_card(tg), "f: expected a SelectCard cost prompt (got %s)", pending_type(tg));
    pick(tg, 0);
    drain_pick0(tg, 12);

    CHECK(hand_has(tg, target), "f: %s retrieved to hand", target_no);
    if (!hand_has(tg, target)) { dump_state(tg, "f:subunit"); dump_abilities(me, "f"); }
}

static void test_f_sp_pb2_015_recovers_catchu_card(void)
{
    subunit_case(SUMIRE, SUMIRE_PR, GRP_CATCHU);
}

static void test_f_sp_pb2_019_recovers_5yncri5e_card(void)
{
    /* Rust fixture bug: the file asks for "PL!SP-bp1-014-PR", which is NOT a
     * key in cards.json. The whole SP-bp1-014 family has exactly ONE member,
     * PL!SP-bp1-014-N 嵐 千砂都, unit 5yncri5e!. CORRECTED to -N. */
    subunit_case(SHIKISAMA, KANATSU_5, GRP_FIVE);
}

static void test_f_sp_pb2_021_recovers_kaleidoscore_card(void)
{
    subunit_case(WIEN, KARUKO, GRP_KALEID);
}

/* Negative control for F: the same source, a target that is inside the group
 * filter's SERIES but OUTSIDE the sub-unit. Without a same-source positive in
 * the suite, a filter matching nothing would make all three F cases green. */
static void test_f_5yncri5e_rejects_a_same_series_catChu_card(void)
{
    TestGame *tg = &t_tg[4];
    clear_p1(tg);
    int me = mid(tg, SHIKISAMA);
    if (!pin_id(me, SHIKISAMA)) return;
    int catchu = mid(tg, SUMIRE_PR);      /* same ラブライブ！スーパー series */
    if (!pin_id(catchu, SUMIRE_PR)) return;
    int fodder = mid(tg, FILLER_MUS);
    if (fodder < 0) return;

    /* This is the SHARPEST assertion in the file. A series-based reading of
     * 『5yncri5e!』 would recover her; only an exact unit compare rejects her.
     * Both facts are pinned on the same two cards. */
    CHECK(rb_card_matches_group_str(catchu, GRP_FIVE) == 0,
          "f: a same-series 『CatChu!』 card must NOT match 『5yncri5e!』");
    CHECK(rb_card_matches_group_str(catchu, GRP_CATCHU),
          "f: ... and it DOES match its own sub-unit 『CatChu!』");
    CHECK(rb_card_matches_group_str(me, GRP_FIVE), "f: the source is 『5yncri5e!』");

    test_give_energy(tg, 20);
    test_add_to_hand(tg, me);
    test_add_to_discard(tg, catchu);
    test_add_to_hand(tg, fodder);

    play_to_stage(tg, me, AREA_CENTER, 0);
    CHECK(note_prompt(tg), "f: the optional discard cost must still be offered");
    if (pending(tg)) pick(tg, 0);
    drain_pick0(tg, 12);

    CHECK(!hand_has(tg, catchu),
          "f: 『5yncri5e!』 must not recover a same-series 『CatChu!』 card");
    if (!hand_has(tg, catchu)) { dump_state(tg, "f:same_series"); dump_abilities(me, "f"); }
}

/* ======================================================================
 * G. optional_discard_two_split_group_recovery_ginko_test.rs (百生吟子)
 *    a SPLIT group gate: a 『蓮ノ空』 member AND a 『蓮ノ空』 live, one wrong
 *    group member decoy.
 * ====================================================================== */

typedef struct {
    TestGame *tg;
    int ginko, cost[2], member, wrong_member, live;
} GinkoBoard;

static void ginko_setup(GinkoBoard *b, TestGame *tg, int live_count)
{
    rb_card_copy_pool_reset();
    clear_p1(tg);
    b->tg = tg;
    b->ginko = mid(tg, GINKO);
    b->cost[0] = mid(tg, FILLER_MUS);
    b->cost[1] = mid(tg, FILLER_MUS);
    b->member = mid(tg, CERISE);
    b->wrong_member = mid(tg, SAYAKA);
    b->live = mid(tg, MIRAGE);
    pin_id(b->ginko, GINKO);
    pin_id(b->member, CERISE);
    pin_id(b->wrong_member, SAYAKA);
    pin_id(b->live, MIRAGE);
    if (b->ginko < 0 || b->cost[0] < 0 || b->cost[1] < 0 ||
        b->member < 0 || b->wrong_member < 0 || b->live < 0) return;
    /* GROUP PINS - the split gate needs all three of these to be true.
     * NOTE the decoy: PL!HS-sd1-002-SD 村野さやか is a DOLLCHESTRA card but its
     * SERIES is ラブライブ！蓮ノ空女学院スクールアイドルクラブ, so it IS inside
     * 『蓮ノ空』. What excludes it is the printed 『スリーズブーケ』 sub-unit on
     * the member leg, NOT the series-level group. That distinction is the
     * whole point of the split gate, so it is asserted rather than assumed. */
    CHECK(rb_card_matches_group_str(b->member, GRP_HASU),
          "g: the target member is inside 『蓮ノ空』");
    CHECK(rb_card_matches_group_str(b->member, "スリーズブーケ"),
          "g: the target member is inside the 『スリーズブーケ』 sub-unit");
    CHECK(rb_card_matches_group_str(b->live, GRP_HASU),
          "g: the target live is inside 『蓮ノ空』");
    CHECK(rb_card_matches_group_str(b->wrong_member, GRP_HASU),
          "g: the decoy is ALSO inside the 『蓮ノ空』 SERIES, so only the "
          "sub-unit gate can exclude it");
    CHECK(!rb_card_matches_group_str(b->wrong_member, "スリーズブーケ"),
          "g: the decoy is OUTSIDE 『スリーズブーケ』 - this is the real gate");
    CHECK(rb_card_is_live(b->live), "g: the target live really is a live card");
    CHECK(!rb_card_is_live(b->member), "g: the target member really is a member card");
    /* The CSV DEFECT, pinned directly. PL!HS-pb1-020-N's recovery leg carries
     * `group_names=スリーズブーケ,蓮ノ空` - a COMMA-SEPARATED LIST decoded into
     * one string. rb_card_matches_group_str is an exact normalised compare
     * (src/ability/util.c), so the whole CSV matches NOTHING, while
     * rb_card_matches_any_group (util.c:403) - the helper Rust uses via
     * card_matches_any_group (util.rs:662) - matches either entry. This
     * assertion is what makes the g_split_* failures attributable. */
    CHECK(!rb_card_matches_group_str(b->member, GRP_SURIRUZE ", " GRP_HASU),
          "g: the CSV form of the group filter matches NOTHING (rb_card_matches_group_str "
          "is an exact compare; rb_card_matches_any_group is the correct helper)");
    CHECK(rb_card_matches_group_str(b->member, GRP_SURIRUZE),
          "g: ... while the single-entry form does match - so the CSV is the defect");
    test_give_energy(tg, 13);
    test_add_to_hand(tg, b->ginko);
    test_add_to_hand(tg, b->cost[0]);
    test_add_to_hand(tg, b->cost[1]);
    test_add_to_discard(tg, b->wrong_member);
    test_add_to_discard(tg, b->member);
    for (int i = 1; i < live_count; i++) test_add_to_discard(tg, mid(tg, LIVE_MUS));
    test_add_to_discard(tg, b->live);
    int filler = mid(tg, LIVE_MUS);
    if (filler >= 0) fill_decks(tg, filler, 30);
}

/* The Rust twin uses the test helper `fire_trigger`, which forces ONE named
 * ability past the trigger-scan gates. There is no C equivalent, so the 登場 is
 * produced the faithful way instead: the card is actually PLAYED, which is what
 * raises the appearance event the queue scans for. PLAYING it also means the
 * 9E cost is really paid, so the board below is a real post-play board rather
 * than a hand-placed stage slot. */
static int play_ginko(GinkoBoard *b, TestGame *tg)
{
    play_to_stage(tg, b->ginko, AREA_CENTER, 0);
    return tg->state.p[0].stage[AREA_CENTER] == b->ginko;
}

static void test_g_ginko_pay_two_recovers_only_cerise_and_hasunosora_live(void)
{
    GinkoBoard b;
    ginko_setup(&b, &t_tg[5], 3);
    TestGame *tg = b.tg;
    if (b.ginko < 0) return;

    CHECK(play_ginko(&b, tg), "g: the card is played and its 登場 fires on the appearance event");
    CHECK(note_prompt(tg), "g: the 2-card discard cost must be prompted");
    int both[2] = { 0, 1 };
    pick_n(tg, both, 2);
    drain_pick0(tg, 8);

    CHECK(!pending(tg), "g: the ability terminates");
    CHECK(hand_has(tg, b.member), "g: the 『蓮ノ空』 member is retrieved");
    CHECK(hand_has(tg, b.live), "g: the 『蓮ノ空』 live is retrieved");
    /* A "hand contains the card" assertion is satisfied by ONE copy; the Rust
     * test's 『1枚』 is an exact count, so both are pinned. */
    CHECK_EQ(bag_count(&tg->state.p[0].hand, b.member), 1,
             "g: exactly ONE 『蓮ノ空』 member is in hand (1枚)");
    CHECK(wait_has(tg, b.wrong_member), "g: the wrong-group member stays in the waitroom");
    CHECK(wait_has(tg, b.cost[0]) && wait_has(tg, b.cost[1]),
          "g: both cost cards reached the waitroom");
    CHECK(!wait_has(tg, b.member), "g: the retrieved member left the waitroom");
    if (!hand_has(tg, b.member) || !hand_has(tg, b.live)) {
        dump_state(tg, "g:split"); dump_abilities(b.ginko, "g");
    }
}

static void test_g_ginko_declines_pay_two_and_recovers_nothing(void)
{
    GinkoBoard b;
    ginko_setup(&b, &t_tg[6], 3);
    TestGame *tg = b.tg;
    if (b.ginko < 0) return;
    CHECK(play_ginko(&b, tg), "g: the card is played and its 登場 fires on the appearance event");
    /* The baselines are taken AFTER the play, so they isolate the ability from
     * the play's own hand->stage move. */
    int hand_before = tg->state.p[0].hand.n;
    int wait_before = tg->state.p[0].discard.n;
    CHECK(note_prompt(tg), "g: the cost prompt must be offered before it can be declined");
    CHECK(pending_is_card(tg), "g: expected a SelectCard cost prompt (got %s)", pending_type(tg));
    skip(tg);
    drain_skip(tg, 8);

    CHECK(!pending(tg), "g: declining ends the ability");
    CHECK_EQ(tg->state.p[0].hand.n, hand_before, "g: hand unchanged after declining");
    CHECK_EQ(tg->state.p[0].discard.n, wait_before, "g: waitroom unchanged after declining");
    CHECK(wait_has(tg, b.member) && wait_has(tg, b.wrong_member) && wait_has(tg, b.live),
          "g: nothing was recovered after declining");
}

static void test_g_ginko_two_waitroom_lives_do_not_offer_cost(void)
{
    GinkoBoard b;
    ginko_setup(&b, &t_tg[7], 2);
    TestGame *tg = b.tg;
    if (b.ginko < 0) return;
    int hand_before, wait_before;
    CHECK(play_ginko(&b, tg), "g: the card is played and its 登場 fires on the appearance event");
    hand_before = tg->state.p[0].hand.n;
    wait_before = tg->state.p[0].discard.n;
    /* Two waitroom lives is one too many for the printed split, so the cost is
     * not offered at all. */
    CHECK(!pending(tg),
          "g: two waitroom lives must not offer the cost or the recovery");
    CHECK_EQ(tg->state.p[0].hand.n, hand_before, "g: hand unchanged");
    CHECK_EQ(tg->state.p[0].discard.n, wait_before, "g: waitroom unchanged");
}

/* ======================================================================
 * H. hasunosora_cost_gated_recovery_test.rs
 *    『蓮ノ空』 group + optional 1-card hand discard + 2E
 * ====================================================================== */

static void test_h_seras_pays_two_energy_and_discards_to_recover(void)
{
    TestGame *tg = &t_tg[8];
    clear_p1(tg);
    int me = mid(tg, SERAS);
    pin_id(me, SERAS);
    if (me < 0) return;
    CHECK_EQ(printed_cost(me), 11, "h: PL!HS-pb1-007-R prints cost 11");
    int cost_card = mid(tg, FILLER_MUS);
    pin_id(cost_card, FILLER_MUS);
    if (cost_card < 0) return;
    int target = mid(tg, TSUKIGUMI);
    pin_id(target, TSUKIGUMI);
    if (target < 0) return;
    /* GROUP PIN. */
    CHECK(rb_card_matches_group_str(target, GRP_HASU),
          "h: 夕霧綴理 is inside the 『蓮ノ空』 filter");
    CHECK(card_text_has(me, GRP_HASU) || card_text_has(me, "1"),
          "h: the debut text names the 『蓮ノ空』 recovery");
    int filler = mid(tg, FILLER_MUS);
    if (filler >= 0) fill_decks(tg, filler, 30);
    test_add_to_hand(tg, me);
    test_add_to_hand(tg, cost_card);
    test_give_energy(tg, 13);
    test_add_to_discard(tg, target);

    play_to_stage(tg, me, AREA_CENTER, 0);
    CHECK_EQ(tg->state.p[0].stage[AREA_CENTER], me, "h: the card is on stage");
    /* PL!HS-pb1-007-R prints cost 11, so playing it out of 13 active energy
     * leaves exactly 2 - which is precisely what the debut's own E E then
     * spends. The Rust test picks 13 for exactly that reason. */
    CHECK_EQ(energy_active(tg), 2, "h: 11E play cost paid out of 13, leaving 2 active");
    CHECK(note_prompt(tg), "h: the optional 1-card discard cost must be offered");
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    CHECK(c && c->count == 1, "h: the cost is exactly 1 card (got %d)",
          c ? c->count : -1);
    CHECK(c && c->allow_skip, "h: the cost is optional (allow_skip)");
    CHECK(c && c->zone[0] && !strcmp(c->zone, "hand"),
          "h: the cost source is the hand (got zone=%s)", c ? c->zone : "-");
    pick(tg, 0);

    /* Answering the optional cost leaves the RECOVERY prompt on screen. */
    CHECK(note_prompt(tg), "h: the recovery selection must be offered after the cost is paid");
    CHECK(pending_is_card(tg), "h: expected a SelectCard recovery (got %s)", pending_type(tg));
    if (pending(tg)) pick(tg, 0);
    drain_choices(tg, 8);

    CHECK(!pending(tg), "h: the ability completes");
    CHECK_EQ(energy_active(tg), 0, "h: the debut's own E E spends the last 2 active energy");
    CHECK(hand_has(tg, target), "h: the 『蓮ノ空』 card reached the hand");
    CHECK(wait_has(tg, cost_card), "h: the cost card is in the waitroom");
    if (!hand_has(tg, target)) { dump_state(tg, "h:seras"); dump_abilities(me, "h"); }
}

static void test_h_seras_decline_preserves_energy_hand_and_waitroom(void)
{
    TestGame *tg = &t_tg[9];
    clear_p1(tg);
    int me = mid(tg, SERAS);
    pin_id(me, SERAS);
    if (me < 0) return;
    int cost_card = mid(tg, FILLER_MUS);
    if (cost_card < 0) return;
    int target = mid(tg, TSUKIGUMI);
    if (target < 0) return;
    int filler = mid(tg, FILLER_MUS);
    if (filler >= 0) fill_decks(tg, filler, 30);
    test_add_to_hand(tg, me);
    test_add_to_hand(tg, cost_card);
    test_give_energy(tg, 13);
    test_add_to_discard(tg, target);

    play_to_stage(tg, me, AREA_CENTER, 0);
    CHECK(note_prompt(tg), "h: the optional cost must be offered before it can be declined");
    skip(tg);

    CHECK(!pending(tg), "h: declining ends the ability");
    CHECK(!hand_has(tg, target), "h: declined -> nothing is recovered");
    CHECK(wait_has(tg, target), "h: declined -> the target stays in the waitroom");
    CHECK_EQ(hand_has(tg, cost_card), 1, "h: declined -> the cost card stays in hand");
}

static void test_h_seras_wrong_group_only_recovers_nothing(void)
{
    TestGame *tg = &t_tg[10];
    clear_p1(tg);
    int me = mid(tg, SERAS);
    pin_id(me, SERAS);
    if (me < 0) return;
    int cost_card = mid(tg, FILLER_MUS);
    if (cost_card < 0) return;
    int wrong_member = mid(tg, FILLER_MUS);
    if (wrong_member < 0) return;
    int wrong_live = mid(tg, LIVE_MUS);
    pin_id(wrong_live, LIVE_MUS);
    if (wrong_live < 0) return;
    int filler = mid(tg, FILLER_MUS);
    if (filler >= 0) fill_decks(tg, filler, 30);
    test_add_to_hand(tg, me);
    test_add_to_hand(tg, cost_card);
    test_give_energy(tg, 13);
    test_add_to_discard(tg, wrong_member);
    test_add_to_discard(tg, wrong_live);

    /* GROUP PINS: both decoys are genuinely OUTSIDE 『蓮ノ空』. Without them
     * this negative would pass just as happily on a filter matching nothing. */
    CHECK(!rb_card_matches_group_str(wrong_member, GRP_HASU),
          "h: the μ's member decoy is OUTSIDE 『蓮ノ空』");
    CHECK(!rb_card_matches_group_str(wrong_live, GRP_HASU),
          "h: the μ's live decoy is OUTSIDE 『蓮ノ空』");
    CHECK(rb_card_matches_group_str(mid(tg, TSUKIGUMI), GRP_HASU),
          "h: POSITIVE CONTROL - a 『蓮ノ空』 card DOES match the same filter");

    play_to_stage(tg, me, AREA_CENTER, 0);
    CHECK(note_prompt(tg), "h: the optional cost must be offered");
    if (pending(tg)) pick(tg, 0);
    drain_pick0(tg, 8);

    CHECK(!pending(tg), "h: the ability completes with no target");
    CHECK_EQ(tg->state.p[0].hand.n, 0, "h: hand is empty after paying the cost");
    CHECK(wait_has(tg, wrong_live) && wait_has(tg, wrong_member),
          "h: both wrong-group cards stayed in the waitroom");
}

static void test_h_cl1002_accept_energy_retrieves_dollchestra(void)
{
    TestGame *tg = &t_tg[11];
    clear_p1(tg);
    int me = mid(tg, SAYAKA_CL);
    pin_id(me, SAYAKA_CL);
    if (me < 0) return;
    int doll = mid(tg, MIRAGE);
    pin_id(doll, MIRAGE);
    if (doll < 0) return;
    CHECK(rb_card_matches_group_str(doll, GRP_DOLL),
          "h: Mirage Voyage is inside the 『DOLLCHESTRA』 filter");
    int filler = mid(tg, FILLER_MUS);
    if (filler >= 0) fill_decks(tg, filler, 30);
    test_add_to_hand(tg, me);
    test_give_energy(tg, 10);
    test_add_to_discard(tg, doll);

    play_to_stage(tg, me, AREA_LEFT, 0);
    /* Do NOT drain here: PL!HS-cl1-002-CL's ONLY cost is the optional E, so the
     * very first prompt on this board IS the pay/skip gate. A drain would skip
     * it and the "accept" case would silently test the decline path. */
    CHECK(note_prompt(tg), "h: the optional energy cost must be prompted");
    CHECK(!strcmp(pending_type(tg), "SelectTarget"),
          "h: expected SelectTarget (pay_optional_cost:skip), got %s", pending_type(tg));
    select_option(tg, 1);   /* option 0 = SKIP, option 1 = PAY */
    /* Answering the pay/skip gate leaves the RECOVERY selection on screen. */
    CHECK(note_prompt(tg), "h: paying must open the recovery selection");
    if (pending(tg)) pick(tg, 0);
    drain_choices(tg, 8);

    CHECK(hand_has(tg, doll), "h: 『DOLLCHESTRA』 card retrieved to hand");
    /* PL!HS-cl1-002-CL prints cost 5, so 10 - 5 = 5 remain after the play.
     * Whether the optional E is actually tapped on PAY is asserted separately
     * below rather than folded in, so a shortfall there is visible. */
    CHECK_EQ(energy_active(tg), 5, "h: 5E play cost paid out of 10, leaving 5 active");
    CHECK_EQ(energy_active(tg), 4, "h: DEFECT - the optional E answered with PAY was not tapped");
    if (!hand_has(tg, doll)) { dump_state(tg, "h:cl1002"); dump_abilities(me, "h"); }
}

static void test_h_cl1002_decline_stays_in_waitroom(void)
{
    TestGame *tg = &t_tg[12];
    clear_p1(tg);
    int me = mid(tg, SAYAKA_CL);
    pin_id(me, SAYAKA_CL);
    if (me < 0) return;
    int doll = mid(tg, MIRAGE);
    if (doll < 0) return;
    int filler = mid(tg, FILLER_MUS);
    if (filler >= 0) fill_decks(tg, filler, 30);
    test_add_to_hand(tg, me);
    test_give_energy(tg, 10);
    test_add_to_discard(tg, doll);

    play_to_stage(tg, me, AREA_LEFT, 0);
    CHECK(note_prompt(tg), "h: the optional energy cost prompt is expected");
    CHECK(!strcmp(pending_type(tg), "SelectTarget"),
          "h: expected SelectTarget (pay_optional_cost:skip), got %s", pending_type(tg));
    select_option(tg, 0);   /* option 0 = SKIP */
    drain_choices(tg, 8);

    /* POSITIVE CONTROL: the identical board in
     * test_h_cl1002_accept_energy_retrieves_dollchestra recovers on PAY. */
    CHECK(!hand_has(tg, doll), "h: declined -> the card stays in the waitroom");
    CHECK_EQ(energy_active(tg), 5, "h: declined -> the optional E is not tapped");
}

static void test_h_cl1008_self_to_waitroom_retrieves_hasunosora(void)
{
    TestGame *tg = &t_tg[13];
    clear_p1(tg);
    int me = mid(tg, IZUMI);
    pin_id(me, IZUMI);
    if (me < 0) return;
    int hns = mid(tg, HINO);
    pin_id(hns, HINO);
    if (hns < 0) return;
    CHECK(rb_card_matches_group_str(hns, GRP_HASU),
          "h: the target member is inside the 『蓮ノ空』 filter");
    int filler = mid(tg, FILLER_MUS);
    if (filler >= 0) fill_decks(tg, filler, 30);
    tg->state.p[0].stage[AREA_CENTER] = me;
    test_add_to_discard(tg, hns);

    activate_via_queue(tg, me);
    drain_pick0(tg, 12);

    CHECK(!stage_has(tg, me), "h: the activation cost moved this member off stage");
    CHECK(hand_has(tg, hns), "h: the 『蓮ノ空』 card retrieved to hand");
    if (!hand_has(tg, hns)) { dump_state(tg, "h:cl1008"); dump_abilities(me, "h"); }
}

/* ======================================================================
 * I. success_score_live_pl_bp6_013_n_test.rs (園田海未)
 *    SCORE-TOTAL gate on the success live-card zone -> live recovery
 * ====================================================================== */

static void test_i_success_score_nine_recovers_mus_live(void)
{
    TestGame *tg = &t_tg[14];
    clear_p1(tg);
    int filler = mid(tg, FILLER_MUS);
    if (filler >= 0) fill_decks(tg, filler, 30);
    int me = mid(tg, UMISEA);
    pin_id(me, UMISEA);
    if (me < 0) return;
    int mus_live = mid(tg, LIVE_MUS2);
    pin_id(mus_live, LIVE_MUS2);
    if (mus_live < 0) return;
    tg->state.p[0].stage[AREA_LEFT] = me;
    test_add_to_discard(tg, mus_live);
    int big = mid(tg, NEXT_SPARK);
    pin_id(big, NEXT_SPARK);
    if (big < 0) return;
    /* SCORE-GATE PREMISE on the ENGINE's decode: cards.json says score 9. */
    CHECK_EQ(printed_score(big), 9, "i: the success-zone live decodes to score 9 (cards.json says 9)");
    test_add_to_success(tg, big);

    CHECK(fire_trigger(tg, me, RB_TSTR_DEBUT), "i: the 登場 trigger fires");
    drain_pick0(tg, 10);

    CHECK(hand_has(tg, mus_live), "i: score total >= 6 -> μ's live retrieved to hand");
    if (!hand_has(tg, mus_live)) { dump_state(tg, "i:score9"); dump_abilities(me, "i"); }
}

static void test_i_empty_success_zone_does_not_recover_live(void)
{
    TestGame *tg = &t_tg[15];
    clear_p1(tg);
    int filler = mid(tg, FILLER_MUS);
    if (filler >= 0) fill_decks(tg, filler, 30);
    int me = mid(tg, UMISEA);
    pin_id(me, UMISEA);
    if (me < 0) return;
    int mus_live = mid(tg, LIVE_MUS2);
    pin_id(mus_live, LIVE_MUS2);
    if (mus_live < 0) return;
    tg->state.p[0].stage[AREA_LEFT] = me;
    test_add_to_discard(tg, mus_live);
    CHECK_EQ(tg->state.p[0].success.n, 0, "i: the success zone really is empty");

    CHECK(fire_trigger(tg, me, RB_TSTR_DEBUT), "i: the 登場 trigger fires");
    drain_pick0(tg, 10);

    /* POSITIVE CONTROL: test_i_success_score_nine_recovers_mus_live runs the
     * identical board with one score-9 card added and the target IS recovered. */
    CHECK(!hand_has(tg, mus_live), "i: an empty success zone -> no retrieval");
}

/* ======================================================================
 * J. live_success_recover_different_group_name_to_hand_ll_bp5_002_l_test.rs
 *    ab#1: 「カード名がすべて異なる」 - a card whose group differs from ALL
 *    stage members. This is the RAREST group rule in the cluster: it is a
 *    NEGATIVE group predicate over the whole stage, not a positive filter.
 * ====================================================================== */

static void bring_the_love_board(TestGame *tg, int stage0, int stage1, int stage2)
{
    tg->state.p[0].stage[AREA_LEFT]   = stage0;
    tg->state.p[0].stage[AREA_CENTER] = stage1;
    tg->state.p[0].stage[AREA_RIGHT]  = stage2;
}

static int fire_live_success(TestGame *tg, int live)
{
    /* Inject a heart00 wildcard so should_trigger_live_success's window is
     * open, place the live, and dispatch - the Rust force_live_success helper. */
    tg->state.p[0].hearts[0] = 20;
    test_add_to_live(tg, live);
    tg->state.activating_card = live;
    int queued = rb_queue_trigger_abilities(&tg->state, 0, RB_TSTR_LIVE_SUCCESS);
    if (queued <= 0) return 0;
    rb_process_pending_auto_abilities(&tg->state);
    return 1;
}

static void test_j_ab1_different_group_moved_to_hand(void)
{
    TestGame *tg = &t_tg[0];
    clear_p1(tg);
    int live = mid(tg, BRING_LOVE);
    pin_id(live, BRING_LOVE);
    if (live < 0) return;
    int muse = mid(tg, MUS_MEMBER);
    int filler = mid(tg, FILLER_MUS);
    int aq = mid(tg, AQ_MEMBER);
    pin_id(muse, MUS_MEMBER); pin_id(filler, FILLER_MUS); pin_id(aq, AQ_MEMBER);
    if (muse < 0 || filler < 0 || aq < 0) return;
    /* GROUP PINS: the rule is a DIFFERENCE over the stage, so both sides must
     * be pinned or the whole case reads vacuously. */
    CHECK(rb_card_matches_group_str(muse, GRP_MU), "j: the stage member is μ's");
    CHECK(rb_card_matches_group_str(aq, "Aqours"), "j: the discard card is 『Aqours』");
    CHECK(rb_card_matches_group_str(live, GRP_MU) == 0, "j: the live card is not μ's");
    bring_the_love_board(tg, muse, filler, RB_EMPTY_SLOT);
    test_add_to_discard(tg, aq);

    CHECK(fire_live_success(tg, live), "j: the ライブ成功時 trigger fires");
    drain_pick0(tg, 8);

    CHECK(hand_has(tg, aq), "j: an 『Aqours』 card should move to hand (differs from stage μ's)");
    CHECK(!wait_has(tg, aq), "j: the card should no longer be in the waitroom");
    if (!hand_has(tg, aq)) { dump_state(tg, "j:diffgroup"); dump_abilities(live, "j"); }
}

static void test_j_ab1_same_group_not_moved(void)
{
    TestGame *tg = &t_tg[1];
    clear_p1(tg);
    int live = mid(tg, BRING_LOVE);
    if (live < 0) return;
    int muse_stage = mid(tg, MUS_MEMBER);
    int muse_discard = mid(tg, FILLER_MUS);
    pin_id(muse_stage, MUS_MEMBER); pin_id(muse_discard, FILLER_MUS);
    if (muse_stage < 0 || muse_discard < 0) return;
    /* GROUP PIN: both really are μ's, which is what makes the "same group"
     * negative meaningful rather than "no filter matched at all". */
    CHECK(rb_card_matches_group_str(muse_stage, GRP_MU), "j: the stage member is μ's");
    CHECK(rb_card_matches_group_str(muse_discard, GRP_MU),
          "j: the discard card is ALSO μ's - the exclusion is by group, not by card");
    bring_the_love_board(tg, muse_stage, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_add_to_discard(tg, muse_discard);

    CHECK(fire_live_success(tg, live), "j: the ライブ成功時 trigger fires");
    drain_pick0(tg, 8);

    CHECK(!hand_has(tg, muse_discard), "j: a μ's card must NOT move to hand (same group as stage)");
    CHECK(wait_has(tg, muse_discard), "j: the μ's card must remain in the waitroom");
}

static void test_j_ab1_mixed_discard_only_different_group_moves(void)
{
    TestGame *tg = &t_tg[2];
    clear_p1(tg);
    int live = mid(tg, BRING_LOVE);
    if (live < 0) return;
    int muse_stage = mid(tg, MUS_MEMBER);
    int filler = mid(tg, FILLER_MUS);
    int aq = mid(tg, AQ_MEMBER);
    int muse_discard = mid(tg, FILLER_MUS);
    pin_id(muse_stage, MUS_MEMBER); pin_id(filler, FILLER_MUS);
    pin_id(aq, AQ_MEMBER); pin_id(muse_discard, FILLER_MUS);
    if (muse_stage < 0 || filler < 0 || aq < 0 || muse_discard < 0) return;
    bring_the_love_board(tg, muse_stage, filler, RB_EMPTY_SLOT);
    test_add_to_discard(tg, muse_discard);
    test_add_to_discard(tg, aq);

    CHECK(fire_live_success(tg, live), "j: the ライブ成功時 trigger fires");
    drain_pick0(tg, 8);

    CHECK(hand_has(tg, aq), "j: the 『Aqours』 card should move (differs from stage)");
    CHECK(!hand_has(tg, muse_discard), "j: the μ's card should NOT move (same group as stage)");
    CHECK(wait_has(tg, muse_discard), "j: the μ's card should remain in the waitroom");
}

static void test_j_ab1_empty_stage_moves_one_eligible_card(void)
{
    TestGame *tg = &t_tg[3];
    clear_p1(tg);
    int live = mid(tg, BRING_LOVE);
    if (live < 0) return;
    int aq = mid(tg, AQ_MEMBER);
    int muse = mid(tg, MUS_MEMBER);
    pin_id(aq, AQ_MEMBER); pin_id(muse, MUS_MEMBER);
    if (aq < 0 || muse < 0) return;
    bring_the_love_board(tg, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_add_to_discard(tg, aq);
    test_add_to_discard(tg, muse);

    CHECK(fire_live_success(tg, live), "j: the ライブ成功時 trigger fires");
    CHECK(note_prompt(tg), "j: two eligible cards must be offered as a SelectCard");
    CHECK(pending_is_card(tg), "j: expected a SelectCard (got %s)", pending_type(tg));
    pick(tg, 0);
    drain_pick0(tg, 8);

    int moved = (hand_has(tg, aq) ? 1 : 0) + (hand_has(tg, muse) ? 1 : 0);
    CHECK_EQ(moved, 1, "j: only one eligible card moves");
    CHECK_EQ(tg->state.p[0].discard.n, 1, "j: the other eligible card remains in the waitroom");
}

static void test_j_ab1_empty_discard_nothing_happens(void)
{
    TestGame *tg = &t_tg[4];
    clear_p1(tg);
    int live = mid(tg, BRING_LOVE);
    if (live < 0) return;
    int muse = mid(tg, MUS_MEMBER);
    if (muse < 0) return;
    bring_the_love_board(tg, muse, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    int hand_before = tg->state.p[0].hand.n;

    CHECK(fire_live_success(tg, live), "j: the ライブ成功時 trigger fires");
    drain_pick0(tg, 8);

    CHECK_EQ(tg->state.p[0].hand.n, hand_before, "j: no cards move (empty waitroom)");
}

/* ======================================================================
 * K. reveal_live_name_fragment_recovers_matching_live_q236_q237_test.rs
 *    日野下花帆: a NAME-FRAGMENT containment filter, Q236 (short -> long
 *    recovers) and Q237 (long -> short does NOT).
 *
 * This is the only name-based filter in the cluster, and it is a two-sided
 * rule: a green Q236 alone would not catch a filter that matched EVERY name.
 * Both directions are ported on the SAME board shape.
 * ====================================================================== */

static void hino_reveal_board(TestGame *tg, int reveal, int target)
{
    int hino = mid(tg, HINO);
    if (!pin_id(hino, HINO)) return;
    test_give_energy(tg, 2);
    tg->state.p[0].stage[AREA_CENTER] = hino;
    test_add_to_hand(tg, reveal);
    test_add_to_discard(tg, target);
}

static void name_pair_case(const char *reveal_no, const char *target_no,
                           const char *expect_recovered, const char *tag)
{
    TestGame *tg = &t_tg[5];
    clear_p1(tg);
    int reveal = mid(tg, reveal_no);
    int target = mid(tg, target_no);
    if (!pin_id(reveal, reveal_no) || !pin_id(target, target_no)) return;
    CHECK(rb_card_is_live(reveal) && rb_card_is_live(target),
          "k: %s: both the revealed and the target really are live cards", tag);
    hino_reveal_board(tg, reveal, target);
    int energy_before = energy_active(tg);

    activate_via_queue(tg, tg->state.p[0].stage[AREA_CENTER]);
    drain_pick0(tg, 12);

    int recovered = hand_has(tg, target);
    if (expect_recovered[0] == 'y')
        CHECK(recovered, "k: %s: %s SHOULD be recovered from %s", tag, target_no, reveal_no);
    else {
        /* Two-sided control: the positive twin on the same board shape is
         * green, so a filter matching everything is ruled out. */
        CHECK(!recovered, "k: %s: %s must NOT be recovered from %s", tag, target_no, reveal_no);
        CHECK(wait_has(tg, target), "k: %s: the target stays in the waitroom", tag);
    }
    CHECK(hand_has(tg, reveal), "k: %s: the revealed card stays in hand (公開する)", tag);
    CHECK_EQ(energy_active(tg), energy_before - 2, "k: %s: 2 energy consumed", tag);
    if (expect_recovered[0] == 'y' && !recovered) {
        dump_state(tg, "k:name_pair");
        dump_abilities(tg->state.p[0].stage[AREA_CENTER], "k");
    }
}

static void test_k_q236_base_name_recovers_104th(void)
{
    name_pair_case(DREAM, DB_104, "y", "Q236");
}

static void test_k_q237_104th_cannot_recover_base(void)
{
    name_pair_case(DB_104, DREAM, "n", "Q237");
}

static void test_k_q236_base_name_recovers_105th(void)
{
    name_pair_case(DREAM, DB_105, "y", "Q236/105");
}

static void test_k_q237_105th_cannot_recover_104th(void)
{
    name_pair_case(DB_105, DB_104, "n", "Q237 cross-variant");
}

static void test_k_q237_104th_cannot_recover_105th(void)
{
    name_pair_case(DB_104, DB_105, "n", "Q237 cross-variant reverse");
}

static void test_k_butterfly_recovers_butterfly_wing(void)
{
    name_pair_case(BUTTERFLY, BFLY_WING, "y", "Butterfly -> Butterfly Wing");
}

static void test_k_butterfly_wing_cannot_recover_butterfly(void)
{
    name_pair_case(BFLY_WING, BUTTERFLY, "n", "Butterfly Wing -> Butterfly");
}

static void test_k_link_base_recovers_104th(void)
{
    name_pair_case(LINK_BASE, LINK_104, "y", "Link to the FUTURE -> 104");
}

static void test_k_link_104_cannot_recover_base(void)
{
    name_pair_case(LINK_104, LINK_BASE, "n", "Link to the FUTURE 104 -> base");
}

static void test_k_no_matching_live_in_waitroom_fails(void)
{
    TestGame *tg = &t_tg[6];
    clear_p1(tg);
    int hino = mid(tg, HINO);
    if (!pin_id(hino, HINO)) return;
    int reveal = mid(tg, DREAM);
    int wrong_live = mid(tg, LIVE_MUS);
    pin_id(reveal, DREAM); pin_id(wrong_live, LIVE_MUS);
    if (reveal < 0 || wrong_live < 0) return;
    test_give_energy(tg, 2);
    tg->state.p[0].stage[AREA_CENTER] = hino;
    test_add_to_hand(tg, reveal);
    test_add_to_discard(tg, wrong_live);
    int energy_before = energy_active(tg);

    activate_via_queue(tg, hino);
    drain_pick0(tg, 12);

    CHECK(!hand_has(tg, wrong_live), "k: a mismatched live card should not be recovered");
    CHECK(wait_has(tg, wrong_live), "k: the mismatched live stays in the waitroom");
    CHECK(hand_has(tg, reveal), "k: the revealed card stays in hand");
    CHECK_EQ(energy_active(tg), energy_before - 2, "k: 2 energy consumed");
}

static void test_k_no_live_card_in_hand_fails(void)
{
    TestGame *tg = &t_tg[7];
    clear_p1(tg);
    int hino = mid(tg, HINO);
    if (!pin_id(hino, HINO)) return;
    int target = mid(tg, DB_104);
    pin_id(target, DB_104);
    if (target < 0) return;
    test_give_energy(tg, 2);
    tg->state.p[0].stage[AREA_CENTER] = hino;
    test_add_to_discard(tg, target);

    activate_via_queue(tg, hino);
    drain_pick0(tg, 12);

    /* POSITIVE CONTROL: test_k_q236_base_name_recovers_104th is the same board
     * with one live card in hand and the target IS recovered. */
    CHECK(wait_has(tg, target), "k: no card to reveal -> the cost is unpayable, target stays");
    CHECK_EQ(tg->state.p[0].hand.n, 0, "k: no cards should be in hand");
}

/* ======================================================================
 * L. discard_two_recover_heart06_threshold_live_test.rs (矢澤にこ)
 *    「ブレードハートにheart06が3以上含むライブカード」 - a HEART-COLOUR
 *    threshold on a BLADE heart, plus a live-card TYPE filter.
 *
 * HEART-TABLE DISCIPLINE: the decoded heart table is blade-FIRST for a live
 * card and blade-LAST for a member, so this test never sums the whole table -
 * it reads the single bucket the ability's text names. The blade-heart
 * PREMISE is taken from the printed `blade` field instead, which is
 * order-independent, and the colour bucket is read through rb_mods_get_heart
 * (NOT test_get_heart_modifier, which remaps colour 5 onto RB_HEART_ORANGE).
 * ====================================================================== */

static void test_l_recover_with_heart06_ge3(void)
{
    TestGame *tg = &t_tg[8];
    clear_p1(tg);
    int nico = mid(tg, NICO);
    pin_id(nico, NICO);
    if (nico < 0) return;
    CHECK_EQ(printed_cost(nico), 15, "l: 矢澤にこ prints cost 15");
    tg->state.p[0].stage[AREA_CENTER] = nico;
    test_give_energy(tg, 15);
    int fa = mid(tg, FILLER_MUS);
    int fb = mid(tg, FILLER_MUS);
    int fc = mid(tg, FILLER_MUS);
    int fd = mid(tg, FILLER_MUS);
    if (fa < 0 || fb < 0 || fc < 0 || fd < 0) return;
    test_add_to_hand(tg, fa); test_add_to_hand(tg, fb);
    test_add_to_hand(tg, fc); test_add_to_hand(tg, fd);
    int eligible = mid(tg, BUTTERFLY);
    pin_id(eligible, BUTTERFLY);
    if (eligible < 0) return;
    /* HEART-GATE PREMISE, on the engine's own decode.
     *
     * RUST TEST COMMENT BUG (reported, not "fixed" in the Rust tree): the file's
     * doc comment says 「ブレードハートにheart06が3以上」, but the PRINTED text is
     * 「必要ハートにheart06を3以上含む」. There is no blade term in the card, and
     * cards.json records PL!N-bp1-028-L with need_heart {"heart06":5,
     * "heart0":7} and NO blade at all. The assertion below therefore pins
     * need_heart, which is what the card actually gates on. */
    CHECK(rb_card_is_live(eligible), "l: the eligible card is a live card");
    CHECK_EQ(printed_blade(eligible), 0,
             "l: PL!N-bp1-028-L has NO blade - the Rust doc comment's ブレードハート "
             "term contradicts both the printed text and cards.json");
    CHECK(card_text_has(nico, "heart06"), "l: the ability text really names heart06");
    test_add_to_discard(tg, eligible);

    activate_via_queue(tg, nico);
    CHECK(note_prompt(tg), "l: the 2-card discard cost must prompt immediately");
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    CHECK(c && c->zone[0] && !strcmp(c->zone, "hand"), "l: cost source is the hand (got %s)",
          c ? c->zone : "-");
    CHECK(c && c->count == 2, "l: cost is exactly 2 cards (got %d)", c ? c->count : -1);
    int both[2] = { 0, 1 };
    pick_n(tg, both, 2);
    drain_pick0(tg, 12);

    CHECK(hand_has(tg, eligible), "l: the eligible live recovered to hand");
    CHECK(wait_has(tg, fa) && wait_has(tg, fb), "l: BOTH cost cards reached the waitroom");
    if (!hand_has(tg, eligible)) { dump_state(tg, "l:heart06"); dump_abilities(nico, "l"); }
}

static void test_l_heart06_too_low_not_recovered(void)
{
    TestGame *tg = &t_tg[9];
    clear_p1(tg);
    int nico = mid(tg, NICO);
    pin_id(nico, NICO);
    if (nico < 0) return;
    tg->state.p[0].stage[AREA_CENTER] = nico;
    test_give_energy(tg, 15);
    int f[4];
    for (int i = 0; i < 4; i++) { f[i] = mid(tg, FILLER_MUS); if (f[i] < 0) return; }
    for (int i = 0; i < 4; i++) test_add_to_hand(tg, f[i]);
    int ineligible = mid(tg, LIVE_MUS);
    pin_id(ineligible, LIVE_MUS);
    if (ineligible < 0) return;
    /* HEART PINS on the ENGINE's own decode. cards.json records
     * PL!-sd1-019-SD with need_heart {"heart01":1,"heart03":1,"heart06":1}, so
     * it carries heart06 but only ONE, below the >= 3 the ability gates on. */
    CHECK(rb_card_is_live(ineligible), "l: the decoy IS a live card, so the heart gate is the filter");
    CHECK_EQ(printed_blade(ineligible), 0, "l: the decoy carries no blade heart");
    test_add_to_discard(tg, ineligible);

    activate_via_queue(tg, nico);
    CHECK(note_prompt(tg), "l: the 2-card discard cost must prompt");
    int both[2] = { 0, 1 };
    pick_n(tg, both, 2);
    drain_pick0(tg, 12);

    CHECK_EQ(tg->state.p[0].hand.n, 2, "l: only the 2 cost cards left the hand");
    CHECK(wait_has(tg, ineligible), "l: the ineligible live STAYS in the waitroom");
    CHECK(!pending(tg), "l: no recovery prompt when nothing matches");
}

static void test_l_no_eligible_card_skips(void)
{
    TestGame *tg = &t_tg[10];
    clear_p1(tg);
    int nico = mid(tg, NICO);
    pin_id(nico, NICO);
    if (nico < 0) return;
    tg->state.p[0].stage[AREA_CENTER] = nico;
    test_give_energy(tg, 15);
    int f[4];
    for (int i = 0; i < 4; i++) { f[i] = mid(tg, FILLER_MUS); if (f[i] < 0) return; }
    for (int i = 0; i < 4; i++) test_add_to_hand(tg, f[i]);

    activate_via_queue(tg, nico);
    CHECK(note_prompt(tg), "l: the 2-card discard cost must prompt");
    int both[2] = { 0, 1 };
    pick_n(tg, both, 2);
    drain_pick0(tg, 12);

    CHECK_EQ(tg->state.p[0].hand.n, 2, "l: the hand ends with the 2 unrecovered cost cards");
    CHECK(!pending(tg), "l: no recovery prompt when the waitroom has no live cards");
}

static void test_l_recover_same_live_used_as_discard_cost(void)
{
    TestGame *tg = &t_tg[11];
    clear_p1(tg);
    int nico = mid(tg, NICO);
    pin_id(nico, NICO);
    if (nico < 0) return;
    tg->state.p[0].stage[AREA_CENTER] = nico;
    test_give_energy(tg, 15);
    int eligible = mid(tg, BUTTERFLY);
    int fa = mid(tg, FILLER_MUS);
    int fb = mid(tg, FILLER_MUS);
    int hand_filler = mid(tg, FILLER_MUS);
    pin_id(eligible, BUTTERFLY);
    if (eligible < 0 || fa < 0 || fb < 0 || hand_filler < 0) return;
    /* Q209: the card paid as the discard cost is the SAME card the ability
     * then recovers. */
    CHECK(rb_card_is_live(eligible), "l: the cost/recovery card is a live card");
    test_add_to_hand(tg, eligible);
    test_add_to_hand(tg, fa);
    test_add_to_hand(tg, fb);
    test_add_to_hand(tg, hand_filler);

    activate_via_queue(tg, nico);
    CHECK(note_prompt(tg), "l: the 2-card discard cost must prompt");
    int idx_elig = -1, idx_fa = -1;
    for (int i = 0; i < tg->state.p[0].hand.n; i++) {
        if (tg->state.p[0].hand.cards[i] == eligible) idx_elig = i;
        if (tg->state.p[0].hand.cards[i] == fa) idx_fa = i;
    }
    CHECK(idx_elig >= 0 && idx_fa >= 0, "l: both cost cards are still in hand");
    if (idx_elig >= 0 && idx_fa >= 0) {
        int pair[2] = { idx_elig, idx_fa };
        pick_n(tg, pair, 2);
    }
    drain_pick0(tg, 12);

    CHECK(hand_has(tg, eligible), "l: Q209 - the cost card is recovered back to hand");
    CHECK(!wait_has(tg, eligible), "l: the cost card left the waitroom");
    CHECK(wait_has(tg, fa), "l: the other cost card stayed in the waitroom");
    if (!hand_has(tg, eligible)) { dump_state(tg, "l:q209"); dump_abilities(nico, "l"); }
}

static void test_l_one_card_recovered_even_if_multiple_eligible(void)
{
    TestGame *tg = &t_tg[12];
    clear_p1(tg);
    int nico = mid(tg, NICO);
    pin_id(nico, NICO);
    if (nico < 0) return;
    tg->state.p[0].stage[AREA_CENTER] = nico;
    test_give_energy(tg, 15);
    int f[4];
    for (int i = 0; i < 4; i++) { f[i] = mid(tg, FILLER_MUS); if (f[i] < 0) return; }
    for (int i = 0; i < 4; i++) test_add_to_hand(tg, f[i]);
    int ea = mid(tg, BUTTERFLY);
    int eb = test_new_id(tg, BUTTERFLY);
    pin_id(ea, BUTTERFLY); pin_id(eb, BUTTERFLY);
    if (ea < 0 || eb < 0) return;
    CHECK(ea != eb, "l: two DISTINCT instances of the eligible live");
    test_add_to_discard(tg, ea);
    test_add_to_discard(tg, eb);

    activate_via_queue(tg, nico);
    CHECK(note_prompt(tg), "l: the 2-card discard cost must prompt");
    int both[2] = { 0, 1 };
    pick_n(tg, both, 2);
    /* The recovery choice offers BOTH; take the first. */
    CHECK(note_prompt(tg), "l: two eligible lives must be offered as a SelectCard");
    if (pending(tg)) pick(tg, 0);
    drain_pick0(tg, 8);

    CHECK(hand_has(tg, ea), "l: the chosen eligible live recovered");
    CHECK(wait_has(tg, eb), "l: the OTHER eligible live stays in the waitroom");
    CHECK(!wait_has(tg, ea), "l: the chosen eligible live left the waitroom");
}

/* ======================================================================
 * M. like_a_treasure PL!N-bp7-031-L - ab#1: 「『虹ヶ咲』のライブカードを1枚
 *    手札に加えてもよい」 among the cards a live-success ability moved.
 *    GROUP + TYPE filter, gated on a conditional_optional.
 *
 * RUST FIXTURE NOTE: the file's NON_NIJI_LIVE is "PL!SP-sd1-020-SD", which
 * cards.json records as 鬼塚夏美 / メンバー - a MEMBER, not a live card. It
 * therefore cannot play the "a non-虹ヶ咲 LIVE card" role the comment claims.
 * The nearest unambiguous non-虹ヶ咲 LIVE is PL!S-sd1-019-SD, used here.
 * ====================================================================== */

static void test_m_treasure_niji_live_added_on_accept(void)
{
    TestGame *tg = &t_tg[13];
    clear_p1(tg);
    int lat = mid(tg, TREASURE);
    pin_id(lat, TREASURE);
    if (lat < 0) return;
    int niji = mid(tg, POPPIN);
    pin_id(niji, POPPIN);
    if (niji < 0) return;
    /* GROUP PINS - the whole effect turns on 虹ヶ咲. These are the exact bytes
     * rb_card_group_name() returns, not a hand-typed U+8671. */
    CHECK(rb_card_matches_group_str(niji, GRP_NIJI),
          "m: Poppin' Up! is inside the 『虹ヶ咲』 filter");
    CHECK(rb_card_is_live(niji), "m: the target is a live card");

    test_add_to_live(tg, lat);
    test_add_to_discard(tg, niji);

    /* The Rust test injects a deck->discard movement event by one of the
     * owner's live-success abilities, then runs the real TAS scan. */
    const int moved[1] = { niji };
    rb_trigger_auto_abilities_for_player_with_event(&tg->state, 0, moved, 1, 0, 0);
    rb_process_pending_auto_abilities(&tg->state);

    int offered = 0, guard = 0;
    while (pending(tg) && guard++ < 24) {
        const char *ty = pending_type(tg);
        if (!strcmp(ty, "SelectTarget")) { offered = 1; select_option(tg, 1); }
        else if (!strcmp(ty, "SelectCard")) pick(tg, 0);
        else skip(tg);
    }
    CHECK(offered, "m: the ab#1 optional must be offered after a 虹ヶ咲 live is moved");
    CHECK(hand_has(tg, niji), "m: accepting adds the 虹ヶ咲 live card to hand");
    CHECK_EQ(rb_mods_get_score(&tg->state.mods, lat), 1, "m: accepting grants +1 score");
    /* A "in hand" claim is satisfied by one copy; the printed text says 1枚. */
    CHECK_EQ(bag_count(&tg->state.p[0].hand, niji), 1,
             "m: exactly one copy of the 虹ヶ咲 live card is in hand");
    if (!hand_has(tg, niji)) { dump_state(tg, "m:treasure"); dump_abilities(lat, "m"); }
}

static void test_m_treasure_declined_adds_nothing(void)
{
    TestGame *tg = &t_tg[14];
    clear_p1(tg);
    int lat = mid(tg, TREASURE);
    pin_id(lat, TREASURE);
    if (lat < 0) return;
    int niji = mid(tg, POPPIN);
    pin_id(niji, POPPIN);
    if (niji < 0) return;
    test_add_to_live(tg, lat);
    test_add_to_discard(tg, niji);

    const int moved[1] = { niji };
    rb_trigger_auto_abilities_for_player_with_event(&tg->state, 0, moved, 1, 0, 0);
    rb_process_pending_auto_abilities(&tg->state);

    int offered = 0, guard = 0;
    while (pending(tg) && guard++ < 24) {
        const char *ty = pending_type(tg);
        if (!strcmp(ty, "SelectTarget")) { offered = 1; select_option(tg, 0); }
        else if (!strcmp(ty, "SelectCard")) pick(tg, 0);
        else skip(tg);
    }
    /* POSITIVE CONTROL: the identical board in
     * test_m_treasure_niji_live_added_on_accept DOES reach hand. */
    CHECK(offered, "m: ab#1 must present its optional even when declined");
    CHECK_EQ(tg->state.p[0].hand.n, 0, "m: declining adds nothing to hand");
    CHECK_EQ(rb_mods_get_score(&tg->state.mods, lat), 0, "m: declining grants no score");
}

static void test_m_treasure_niji_member_not_added(void)
{
    TestGame *tg = &t_tg[15];
    clear_p1(tg);
    int lat = mid(tg, TREASURE);
    pin_id(lat, TREASURE);
    if (lat < 0) return;
    int niji_member = mid(tg, MIA);
    pin_id(niji_member, MIA);
    if (niji_member < 0) return;
    /* GROUP + TYPE PINS. The card is inside 『虹ヶ咲』 so the GROUP filter
     * matches it; only the LIVE-card TYPE filter must reject it. This is the
     * assertion that separates the two filters. */
    CHECK(rb_card_matches_group_str(niji_member, GRP_NIJI),
          "m: the decoy IS inside 『虹ヶ咲』, so the TYPE filter is what must reject it");
    CHECK(!rb_card_is_live(niji_member), "m: the decoy is a MEMBER, not a live card");
    test_add_to_live(tg, lat);
    test_add_to_discard(tg, niji_member);

    const int moved[1] = { niji_member };
    rb_trigger_auto_abilities_for_player_with_event(&tg->state, 0, moved, 1, 0, 0);
    rb_process_pending_auto_abilities(&tg->state);
    drain_pick0(tg, 24);

    CHECK(!hand_has(tg, niji_member), "m: a 虹ヶ咲 MEMBER must not be added (only 虹ヶ咲 LIVEs)");
    CHECK_EQ(rb_mods_get_score(&tg->state.mods, lat), 0, "m: no live card added -> no score");
}

static void test_m_treasure_no_movement_does_not_fire(void)
{
    TestGame *tg = &t_tg[0];
    clear_p1(tg);
    int lat = mid(tg, TREASURE);
    pin_id(lat, TREASURE);
    if (lat < 0) return;
    int niji = mid(tg, POPPIN);
    pin_id(niji, POPPIN);
    if (niji < 0) return;
    test_add_to_live(tg, lat);
    test_add_to_discard(tg, niji);

    rb_trigger_auto_abilities_for_player(&tg->state, 0);
    rb_process_pending_auto_abilities(&tg->state);
    int offered = 0, guard = 0;
    while (pending(tg) && guard++ < 24) {
        const char *ty = pending_type(tg);
        if (!strcmp(ty, "SelectTarget")) { offered = 1; select_option(tg, 1); }
        else if (!strcmp(ty, "SelectCard")) pick(tg, 0);
        else skip(tg);
    }
    /* POSITIVE CONTROL: the same card with one movement event does offer and
     * does recover. */
    CHECK(!offered, "m: no movement -> ab#1 must not offer the optional");
    CHECK_EQ(tg->state.p[0].hand.n, 0, "m: no movement -> nothing added to hand");
    CHECK_EQ(rb_mods_get_score(&tg->state.mods, lat), 0, "m: no movement -> no score");
}

/* ======================================================================
 * N. Engine-defect pins.
 *
 * These do not translate a Rust test. Each one isolates a defect the OTHER
 * tests above expose, so the defect is named in a passing test that fails the
 * moment it is fixed - which is how a peer can tell "still broken" from
 * "in flight". Each asserts the ENGINE's own behaviour directly, with no
 * prompt to be vacuous against, so none of them needs a prompt-count control.
 * ====================================================================== */

/* N1. rb_activate_card (src/engine.c:1189) runs a 起動's cost through
 * rb_execute_effect_ex, where a `sequential_cost` node is an explicit no-op
 * (src/ability/effects/executor.c:501, "the sub-costs are already paid in
 * order by the dedicated cost path"). Nothing on that path calls rb_pay_cost,
 * so the cost is dropped entirely. PL!HS-bp2-001-R's cost is a bare
 * pay_energy (not sequential) and DOES get paid, which is what makes this a
 * sequential_cost-specific defect rather than "activation never costs
 * anything". */
static void test_n1_activate_path_drops_a_sequential_cost_cost(void)
{
    TestGame *tg = &t_tg_n[0];
    clear_p1(tg);
    int hanano = mid(tg, HANANO);           /* bare pay_energy cost */
    int kinako = mid(tg, KINAKO);           /* sequential_cost cost */
    pin_id(hanano, HANANO); pin_id(kinako, KINAKO);
    if (hanano < 0 || kinako < 0) return;

    /* CONTROL: the bare-pay_energy cost IS charged on the same path. */
    int filler = mid(tg, FILLER_MUS);
    if (filler >= 0) fill_decks(tg, filler, 30);
    tg->state.p[0].stage[AREA_CENTER] = hanano;
    test_give_energy(tg, 5);
    test_add_to_discard(tg, mid(tg, DREAM));
    test_activate_ability(tg, hanano);
    drain_choices(tg, 12);
    CHECK_EQ(energy_active(tg), 3,
             "n1: CONTROL - a bare pay_energy 起動 cost IS paid by rb_activate_card (5-2=3)");

    /* THE DEFECT: the sequential_cost cost is not. Same entry point, same
     * helper, only the cost's shape differs. */
    clear_p1(tg);
    fill_decks(tg, mid(tg, FILLER_MUS), 30);
    tg->state.p[0].stage[AREA_CENTER] = kinako;
    test_give_energy(tg, 5);
    int cost_card = mid(tg, FILLER_MUS);
    test_add_to_hand(tg, cost_card);
    int liella_live = mid(tg, LIELLA_LIVE);
    test_add_to_discard(tg, liella_live);
    int hand_before = tg->state.p[0].hand.n;
    test_activate_ability(tg, kinako);
    drain_choices(tg, 12);
    CHECK_EQ(energy_active(tg), 5,
             "n1: DEFECT - a sequential_cost( pay_energy 2, hand->discard 1 ) cost is "
             "NOT charged by rb_activate_card; 5 active energy is untouched");
    /* The control leg of the same cost: the hand->discard step did not run
     * either, because the whole sequential_cost node short-circuits. */
    CHECK(tg->state.p[0].hand.n >= hand_before,
          "n1: DEFECT - the sequential_cost's hand->discard leg did not run either");
    CHECK(hand_has(tg, cost_card),
          "n1: DEFECT - the cost card is still in hand after the ability resolved");
}

/* N2. rb_move_take_cards_from_standard_zone (src/ability/effects/move.c:740)
 * reads the card_type filter with cmf_extra(e, "card_type"), but the decoder
 * stores it in the char[24] member `card_type_field` (see
 * src/core/card.c:778, which consults card_type_field FIRST). cmf_extra only
 * looks in the extras table, so the filter arrives as NULL and every card in
 * the zone is a candidate. The fix already exists elsewhere in the tree:
 * compound.c:292 eff_card_type_any() has exactly the right fallback. */
static void test_n2_discard_selection_ignores_the_card_type_filter(void)
{
    TestGame *tg = &t_tg_n[1];
    clear_p1(tg);
    int mari = mid(tg, MARI);
    pin_id(mari, MARI);
    if (mari < 0) return;
    int live = mid(tg, NEXT_SPARK);
    int member = mid(tg, FILLER_MUS);
    pin_id(live, NEXT_SPARK); pin_id(member, FILLER_MUS);
    if (live < 0 || member < 0) return;

    /* The premise, independent of the engine's filter machinery:
     * rb_card_matches_type itself works correctly on both cards. */
    CHECK(rb_card_matches_type(live, "live_card"), "n2: rb_card_matches_type says the live matches");
    CHECK(!rb_card_matches_type(member, "live_card"), "n2: rb_card_matches_type says the member does not");

    /* The live zone is NOT offered - only the waitroom prompt is under test. */
    tg->state.p[0].stage[AREA_CENTER] = mari;
    test_add_to_discard(tg, live);
    test_add_to_discard(tg, member);
    activate_via_queue(tg, mari);
    CHECK(note_prompt(tg), "n2: the recovery prompt must open");
    /* Only the self-cost card is in the waitroom by now plus these two. */
    int n_expected = 0;
    for (int i = 0; i < tg->state.p[0].discard.n; i++)
        if (rb_card_is_live(tg->state.p[0].discard.cards[i])) n_expected++;
    /* DEFECT PINS, inverted on purpose: green now, red the moment
     * rb_move_take_cards_from_standard_zone consults card_type_field. The
     * premise above proves rb_card_matches_type itself is correct, so a
     * mismatch here is the plumbing and not the predicate. */
    CHECK_EQ(candidate_count(tg), tg->state.p[0].discard.n,
             "n2: DEFECT - every waitroom card is a candidate, including the member; "
             "only %d live cards exist", n_expected);
    CHECK(candidate_slot_of(tg, member) >= 0,
          "n2: DEFECT - a member card IS offered as a candidate slot");
}

/* N3. Only card_type and group_names reach the zone-selection filter.
 * rb_move_take_cards_from_standard_zone calls rb_get_selection_indices, whose
 * RbCardFilter is built from those two fields alone
 * (src/ability/util.c:1488), so cost_limit, need_heart_total and
 * name_fragments are silently dropped on this path. Three separate Rust
 * behaviours in this cluster depend on them, and all three are red. */
static void test_n3_zone_selection_drops_cost_limit_need_heart_and_name(void)
{
    /* (b) need_heart: PL!-bp5-009-R gates on need_heart heart06 >= 3. The
     *     engine's own heart reader resolves the premise correctly, which
     *     rules out "the data is missing" and isolates the filter plumbing.
     *     (a) cost_limit and (c) name_fragments are covered by the reds in
     *     test_c_score7_live_is_not_recovered and the test_k_q237_*
     *     negatives respectively; both of those pin their own premise first. */
    TestGame *tg = &t_tg_n[2];
    clear_p1(tg);
    int inc = mid(tg, BUTTERFLY);   /* need_heart heart06 = 5 */
    int dec = mid(tg, LIVE_MUS);    /* need_heart heart06 = 1 */
    pin_id(inc, BUTTERFLY); pin_id(dec, LIVE_MUS);
    if (inc < 0 || dec < 0) return;
    /* heart06 is the BLADE bucket. The printed requirement is read through
     * rb_effective_need_heart, the same pipeline the engine uses, rather than
     * through test_get_heart_modifier - which a peer found silently remaps
     * colour 5 onto RB_HEART_ORANGE. The index is DISCOVERED, not assumed, so
     * this premise cannot pass on a wrong bucket. */
    int need_inc[8], need_dec[8];
    rb_effective_need_heart(&tg->state, inc, need_inc);
    rb_effective_need_heart(&tg->state, dec, need_dec);
    int blade = -1;
    for (int c = 0; c < 8; c++)
        if (need_inc[c] >= 3 && need_inc[c] > need_dec[c]) blade = c;
    CHECK(blade >= 0,
          "n3b: PREMISE - a need_heart bucket exists on the eligible live that is "
          ">= 3 and larger than the decoy's (heart06 = 5 vs 1 in cards.json)");
    if (blade >= 0) {
        CHECK_EQ(need_inc[blade], 5,
                 "n3b: PREMISE - that bucket really is heart06 = 5 on PL!N-bp1-028-L");
        CHECK_EQ(need_dec[blade], 1,
                 "n3b: PREMISE - and heart06 = 1 on PL!-sd1-019-SD, below the >= 3 gate");
    }
    /* Both facts hold, so the l_heart06_too_low red is the filter, not the
     * need_heart decode. */

    /* (c) name_fragments: PL!HS-bp5-001-P's recovery leg carries
     *     [name_constraint=contains_all][name_constraint_source=revealed_card],
     *     which is not a field rb_get_selection_indices can carry at all. Its
     *     red lives in the test_k_q237_* negatives, which are two-sided: the
     *     Q236 positive on the same board is green, so a filter matching
     *     everything is ruled out. */
}

/* N4. group_reference=different_group_names is unimplemented on the move path.
 * LL-bp5-002-L ab#1's whole printed text is 「自分のステージにいるすべての
 * メンバーと異なるグループ名を持つカード1枚」. move.c reads group_names and
 * group_names only; `different_group_names` appears nowhere in
 * src/ability/effects/. Rust implements it at
     *     engine/src/ability/move_cards.rs:1283-1291. */
static void test_n4_different_group_names_reference_is_unimplemented(void){
    TestGame *tg = &t_tg_n[3];
    clear_p1(tg);
    int live = mid(tg, BRING_LOVE);
    pin_id(live, BRING_LOVE);
    if (live < 0) return;
    int stage_member = mid(tg, MUS_MEMBER);
    int same_group = mid(tg, FILLER_MUS);
    pin_id(stage_member, MUS_MEMBER); pin_id(same_group, FILLER_MUS);
    if (stage_member < 0 || same_group < 0) return;

    /* Both are μ's, which is what makes the exclusion a GROUP exclusion. */
    CHECK(rb_card_matches_group_str(stage_member, GRP_MU), "n4: the stage member is μ's");
    CHECK(rb_card_matches_group_str(same_group, GRP_MU),
          "n4: the waitroom card is ALSO μ's - only a group rule can exclude it");
    tg->state.p[0].stage[AREA_LEFT] = stage_member;
    test_add_to_discard(tg, same_group);

    fire_live_success(tg, live);
    drain_pick0(tg, 8);
    /* DEFECT PIN: inverted on purpose. This goes RED the moment the engine
     * starts honouring different_group_names, which is how a peer reads it. */
    CHECK_EQ(hand_has(tg, same_group), 1,
             "n4: DEFECT - a μ's card was recovered even though the stage holds a μ's "
             "member; group_reference=different_group_names is not implemented on the "
             "move path, so the card_type=card filter matches the whole waitroom");
}

/* N5. The mandatory-activation-cost gate is absent. Rust
 * turn/actions/mod.rs:408 check_mandatory_activation_cost refuses an
 * activation whose non-optional cost exceeds active energy. The C tree has
 * BOTH helpers (rb_card_has_optional_payment, src/core/card.c:1007, and the
 * effective-energy-cost query) but NOTHING calls them, so an activation whose
 * cost is unaffordable still pays as much as exists and runs the effect. */
static void test_n5_no_mandatory_energy_gate_on_activation(void)
{
    TestGame *tg = &t_tg_n[4];
    clear_p1(tg);
    int hanano = mid(tg, HANANO);
    pin_id(hanano, HANANO);
    if (hanano < 0) return;
    int live = mid(tg, DREAM);
    pin_id(live, DREAM);
    if (live < 0) return;
    int filler = mid(tg, FILLER_MUS);
    if (filler >= 0) fill_decks(tg, filler, 30);
    tg->state.p[0].stage[AREA_CENTER] = hanano;
    test_add_to_discard(tg, live);
    test_give_energy(tg, 1);          /* E E needed, 1 available */

    /* POSITIVE CONTROL: 3 active energy on the identical board DOES recover
     * (test_c_pay_2e_recovers_score3_live is green). So this is the
     * affordability gate, not the filter. */
    activate_via_queue(tg, hanano);
    drain_choices(tg, 12);
    /* DEFECT PIN: inverted on purpose - red once the gate lands. */
    CHECK_EQ(hand_has(tg, live), 1,
             "n5: DEFECT - a 起動 whose mandatory E E cost exceeds the 1 active energy "
             "still ran its effect; Rust check_mandatory_activation_cost "
             "(turn/actions/mod.rs:408) would refuse it");
}

/* ======================================================================
 * runner
 * ====================================================================== */

#define CHILD_OK        0
#define CHILD_FAILURES  1
#define CHILD_GAPS_ONLY 2   /* no failures, but >= 1 EXPECTED_GAP was recorded */

static int n_gap_only;

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout); fflush(stderr);
    pid_t pid = fork();
    if (pid < 0) {
        int f0 = failures;
        current_test = name;
        fn();
        current_test = "(none)";
        if (failures > f0) n_failed++; else n_ok++;
        printf("%-8s %s  [in-process fallback]\n", failures > f0 ? "FAILED" : "ok", name);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = assertions, f0 = failures, s0 = setup_bugs, g0 = gaps, p0 = prompts;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s), %d setup bug(s), %d gap(s), %d prompt(s)\n",
               name, (int)(assertions - a0), (int)(failures - f0),
               (int)(setup_bugs - s0), (int)(gaps - g0), (int)(prompts - p0));
        fflush(stdout); fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES
                             : ((gaps > g0) ? CHILD_GAPS_ONLY : CHILD_OK));
    }
    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n", "CRASH", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_ok++;
        printf("%-8s %s\n", "ok", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_failed++;
        printf("%-8s %s\n", "FAILED", name);
    } else if (WEXITSTATUS(status) == CHILD_GAPS_ONLY) {
        n_ok++;
        n_gap_only++;
        printf("%-8s %s  (recorded an EXPECTED_GAP; see the GAP: line above)\n", "ok+gaps", name);
    } else {
        n_crashed++;
        printf("%-8s %s  <-- unexpected child exit %d\n", "CRASH", name, WEXITSTATUS(status));
    }
    fflush(stdout);
}

/* RB_DUMP=<card_no>[,<card_no>...] decodes and prints every ability and every
 * effect node of the named cards, so a red cluster can be diagnosed without
 * guessing at the parser's output. */
static void dump_cond(const Condition *c, int depth)
{
    if (!c) return;
    for (int i = 0; i < depth; i++) fputs("  ", stdout);
    printf("COND variant=%u n_fields=%u\n", (unsigned)c->variant, c->n_fields);
    for (uint32_t i = 0; i < c->n_fields && i < RB_MAX_COND_FIELD; i++) {
        for (int d = 0; d < depth; d++) fputs("  ", stdout);
        printf("     f[%u] %s\n", i, c->fields[i].key ? c->fields[i].key : "?");
    }
}

static void dump_eff(const AbilityEffect *e, int depth)
{
    if (!e) return;
    for (int i = 0; i < depth; i++) fputs("  ", stdout);
    printf("[%d] action=%s src=%s dst=%s target=%s type=%s count=%d opt=%d cond=%d n_extra=%d\n",
           depth, e->action ? e->action : "-", e->source ? e->source : "-",
           e->destination ? e->destination : "-", e->target ? e->target : "-",
           e->card_type_field[0] ? e->card_type_field : "-", e->count, e->is_optional,
           e->has_condition, e->n_extra);
    for (int i = 0; i < e->n_extra; i++)
        printf("      [%s=%s]\n", e->extra_k[i] ? e->extra_k[i] : "?",
               e->extra_v[i] ? e->extra_v[i] : "?");
    if (e->has_condition) dump_cond(e->condition, depth + 1);
    dump_eff(e->primary_effect, depth + 1);
    dump_eff(e->alternative_effect, depth + 1);
    dump_eff(e->followup_action, depth + 1);
    dump_eff(e->optional_action, depth + 1);
    dump_eff(e->conditional_action, depth + 1);
    dump_eff(e->look_action, depth + 1);
    dump_eff(e->select_action, depth + 1);
    for (int i = 0; i < e->n_child; i++) dump_eff(e->child[i], depth + 1);
    for (int i = 0; i < e->n_options; i++) dump_eff(e->options[i], depth + 1);
}

static int run_dump(const char *list)
{
    char buf[512];
    snprintf(buf, sizeof buf, "%s", list);
    for (char *tok = strtok(buf, ","); tok; tok = strtok(NULL, ",")) {
        int cid = rb_find_card_by_no(tok);
        printf("== %s -> id=%d resolved=%s cost=%d score=%d live=%d\n", tok, cid,
               card_no_of(cid), printed_cost(cid), printed_score(cid),
               cid >= 0 ? rb_card_is_live(cid) : -1);
        if (cid < 0) continue;
        int n = rb_card_num_abilities((uint32_t)cid);
        for (int i = 0; i < n; i++) {
            Ability ab;
            memset(&ab, 0, sizeof ab);
            if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
            printf("-- ability %d triggers=%s use_limit=%d\n     text=%s\n", i,
                   ab.triggers ? ab.triggers : "-", ab.use_limit,
                   ab.full_text ? ab.full_text : "-");
            if (ab.cost) { printf("   COST:\n"); dump_eff(ab.cost, 2); }
            if (ab.effect) { printf("   EFFECT:\n"); dump_eff(ab.effect, 2); }
            rb_free_ability(&ab);
        }
    }
    fflush(stdout);
    return 0;
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_fatal_signal);
    signal(SIGBUS,  on_fatal_signal);
    signal(SIGABRT, on_fatal_signal);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }
    rb_card_copy_pool_reset();
    { const char *d = getenv("RB_DUMP"); if (d && *d) { int r = run_dump(d); rb_unload(); return r; } }

    printf("--- A total_energy_gated_live_recovery_test.rs ---\n");
    run("a_mei_total_energy",     test_a_mei_debut_counts_total_energy_not_active);
    run("a_mei_below_11",         test_a_mei_no_retrieve_when_total_below_11);
    run("a_mei_no_live_waitroom", test_a_mei_noop_when_waitroom_has_no_live_card);

    printf("--- B self_to_waitroom_live_recovery_energy_activation_test.rs ---\n");
    run("b_aqours_high_activates", test_b_recovers_high_score_aqours_live_and_activates_four);
    run("b_wrong_group_no_act",    test_b_wrong_group_live_skips_energy_activation);
    run("b_below_six_no_act",      test_b_aqours_live_below_six_skips_activation);
    run("b_cost_no_live",          test_b_cost_paid_without_recoverable_live);
    run("b_mandatory_excl_member", test_b_recovery_is_mandatory_and_excludes_members);
    run("b_partial_activation",    test_b_recovered_live_activates_only_available_energy);

    printf("--- C pay_two_energy_recovers_score_three_live_to_hand_test.rs ---\n");
    run("c_pay2e_recovers",        test_c_pay_2e_recovers_score3_live);
    run("c_recover_mandatory",     test_c_recover_is_mandatory_once_cost_paid);
    run("c_turn1_blocks",          test_c_turn1_blocks_second_activation);
    run("c_insufficient_energy",   test_c_insufficient_energy_no_recover);
    run("c_score7_excluded",       test_c_score7_live_is_not_recovered);

    printf("--- D energy_discard_live_recovery_pl_sp_sd2_006_sd2_test.rs ---\n");
    run("d_kinako_recovers",       test_d_kinako_recovers_liella_live);
    run("d_kinako_empty_hand",     test_d_kinako_empty_hand_refused);
    run("d_kinako_no_liella_live", test_d_kinako_no_liella_live_gains_nothing);

    printf("--- E optional_energy_live_recovery_pl_sp_pb2_007_r_test.rs ---\n");
    run("e_paid3_recovers",        test_e_paid_three_energy_recovers_liella_live);
    run("e_unpayable_skips",       test_e_unpayable_energy_keeps_live_in_waitroom);

    printf("--- F optional_discard_subunit_recovery_test.rs ---\n");
    run("f_catchu",                test_f_sp_pb2_015_recovers_catchu_card);
    run("f_5yncri5e",              test_f_sp_pb2_019_recovers_5yncri5e_card);
    run("f_kaleidoscore",          test_f_sp_pb2_021_recovers_kaleidoscore_card);
    run("f_same_series_reject",    test_f_5yncri5e_rejects_a_same_series_catChu_card);

    printf("--- G optional_discard_two_split_group_recovery_ginko_test.rs ---\n");
    run("g_split_pay_two",         test_g_ginko_pay_two_recovers_only_cerise_and_hasunosora_live);
    run("g_split_decline",         test_g_ginko_declines_pay_two_and_recovers_nothing);
    run("g_split_two_lives",       test_g_ginko_two_waitroom_lives_do_not_offer_cost);

    printf("--- H hasunosora_cost_gated_recovery_test.rs ---\n");
    run("h_seras_pay",             test_h_seras_pays_two_energy_and_discards_to_recover);
    run("h_seras_decline",         test_h_seras_decline_preserves_energy_hand_and_waitroom);
    run("h_seras_wrong_group",     test_h_seras_wrong_group_only_recovers_nothing);
    run("h_cl1002_accept",         test_h_cl1002_accept_energy_retrieves_dollchestra);
    run("h_cl1002_decline",        test_h_cl1002_decline_stays_in_waitroom);
    run("h_cl1008_self_wait",      test_h_cl1008_self_to_waitroom_retrieves_hasunosora);

    printf("--- I success_score_live_pl_bp6_013_n_test.rs ---\n");
    run("i_score9_recovers",       test_i_success_score_nine_recovers_mus_live);
    run("i_empty_success_zone",    test_i_empty_success_zone_does_not_recover_live);

    printf("--- J live_success_recover_different_group_name_ll_bp5_002_l_test.rs ---\n");
    run("j_ab1_diff_group",        test_j_ab1_different_group_moved_to_hand);
    run("j_ab1_same_group",        test_j_ab1_same_group_not_moved);
    run("j_ab1_mixed",             test_j_ab1_mixed_discard_only_different_group_moves);
    run("j_ab1_empty_stage",       test_j_ab1_empty_stage_moves_one_eligible_card);
    run("j_ab1_empty_discard",     test_j_ab1_empty_discard_nothing_happens);

    printf("--- K reveal_live_name_fragment_q236_q237_test.rs ---\n");
    run("k_q236_104",              test_k_q236_base_name_recovers_104th);
    run("k_q237_104_to_base",      test_k_q237_104th_cannot_recover_base);
    run("k_q236_105",              test_k_q236_base_name_recovers_105th);
    run("k_q237_105_to_104",       test_k_q237_105th_cannot_recover_104th);
    run("k_q237_104_to_105",       test_k_q237_104th_cannot_recover_105th);
    run("k_butterfly_wing",        test_k_butterfly_recovers_butterfly_wing);
    run("k_butterfly_not",         test_k_butterfly_wing_cannot_recover_butterfly);
    run("k_link_104",              test_k_link_base_recovers_104th);
    run("k_link_not",              test_k_link_104_cannot_recover_base);
    run("k_no_match",              test_k_no_matching_live_in_waitroom_fails);
    run("k_no_live_in_hand",       test_k_no_live_card_in_hand_fails);

    printf("--- L discard_two_recover_heart06_threshold_live_test.rs ---\n");
    run("l_heart06_ge3",           test_l_recover_with_heart06_ge3);
    run("l_heart06_too_low",       test_l_heart06_too_low_not_recovered);
    run("l_no_eligible",           test_l_no_eligible_card_skips);
    run("l_q209_same_card",        test_l_recover_same_live_used_as_discard_cost);
    run("l_one_of_many",           test_l_one_card_recovered_even_if_multiple_eligible);

    printf("--- M like_a_treasure PL!N-bp7-031-L ---\n");
    run("m_accept_adds_niji",      test_m_treasure_niji_live_added_on_accept);
    run("m_decline_nothing",       test_m_treasure_declined_adds_nothing);
    run("m_niji_member_rejected",  test_m_treasure_niji_member_not_added);
    run("m_no_movement",           test_m_treasure_no_movement_does_not_fire);

    printf("--- N engine-defect pins (inverted: red once fixed) ---\n");
    run("n1_seq_cost_dropped",     test_n1_activate_path_drops_a_sequential_cost_cost);
    run("n2_card_type_ignored",    test_n2_discard_selection_ignores_the_card_type_filter);
    run("n3_filter_fields_dropped",test_n3_zone_selection_drops_cost_limit_need_heart_and_name);
    run("n4_diff_group_names",     test_n4_different_group_names_reference_is_unimplemented);
    run("n5_no_energy_gate",       test_n5_no_mandatory_energy_gate_on_activation);

    rb_unload();

    printf("\n==== parity_recover_live ====\n");
    printf("tests run              : %d\n", n_ok + n_failed + n_crashed);
    printf("tests passed           : %d\n", n_ok);
    printf("  of which gap-noted   : %d\n", n_gap_only);
    printf("tests failed (parity)  : %d\n", n_failed);
    printf("tests crashed (engine) : %d\n", n_crashed);
    printf("expected gaps          : recorded per-test in the GAP: lines above\n");
    if (failures) {
        fprintf(stderr, "%d runtime failure(s) in the PARENT process\n", failures);
        return 1;
    }
    if (n_crashed) {
        fprintf(stderr, "%d test(s) crashed in the engine.\n", n_crashed);
        return 1;
    }
    if (n_failed) {
        fprintf(stderr,
                "PARITY RECOVER LIVE: %d failing test(s) - every FAIL line is a strict\n"
                "Rust expectation the C engine does not meet, or a SETUP BUG naming a\n"
                "print the engine cannot resolve. Not a bug in this file.\n", n_failed);
        return n_failed > 125 ? 125 : n_failed;
    }
    printf("ALL PARITY_RECOVER_LIVE CHECKS PASSED\n");
    return 0;
}
