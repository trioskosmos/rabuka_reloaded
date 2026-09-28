/* TEMPORARY private probe for tr_resume — DELETED after the run.
 * Shape: 愛♡スクリ～ム！ option 0 = 「自分と相手は手札を1枚控え室に置く」,
 * 3 cards per hand, count:1, target:"both".
 * Asserts: P1 prompt -> answer -> P1 hand -1, P1 waitroom +1, P2 untouched;
 * then P2 prompt -> answer -> same for P2; no pending choice at the end. */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

static int failures, assertions;
static const char *TC = "(none)";

#define CHECK(c, m) do { assertions++; if (!(c)) { \
    fprintf(stderr, "FAIL [%s]: %s\n", TC, m); failures++; } \
    else printf("  ok: %s\n", m); } while (0)
#define CHECK_EQ(a, e, m) do { assertions++; int a_=(a), e_=(e); \
    if (a_ != e_) { fprintf(stderr, "FAIL [%s]: %s (got %d expected %d)\n", TC, m, a_, e_); \
    failures++; } else printf("  ok: %s\n", m); } while (0)

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    return rb_load("../cards/build");
}

static void bag_clear(RbBag *b) { b->n = 0; }
static void bag_push(RbBag *b, int id) { if (b->n < RB_MAX_ZONE) b->cards[b->n++] = id; }

static void probe(void)
{
    static TestGame tg; test_game_new(&tg);
    int screeam = test_id(&tg, "LL-PR-004-PR");
    int fill     = test_id(&tg, "PL!-sd1-010-SD");
    if (screeam < 0 || fill < 0) { fprintf(stderr, "SETUP: card missing\n"); failures++; return; }

    bag_clear(&tg.state.p[0].deck);
    bag_clear(&tg.state.p[1].deck);
    for (int i = 0; i < 10; i++) { bag_push(&tg.state.p[0].deck, fill); bag_push(&tg.state.p[1].deck, fill); }

    bag_clear(&tg.state.p[0].hand);
    bag_clear(&tg.state.p[1].hand);
    bag_push(&tg.state.p[0].hand, screeam);
    bag_push(&tg.state.p[0].hand, test_new_id(&tg, "PL!-sd1-010-SD"));
    bag_push(&tg.state.p[0].hand, test_new_id(&tg, "PL!-sd1-010-SD"));
    bag_push(&tg.state.p[1].hand, test_new_id(&tg, "PL!-sd1-010-SD"));
    bag_push(&tg.state.p[1].hand, test_new_id(&tg, "PL!-sd1-010-SD"));
    bag_push(&tg.state.p[1].hand, test_new_id(&tg, "PL!-sd1-010-SD"));

    for (int i = 0; i < 5; i++) test_pass(&tg);
    test_set_live_card(&tg, 0, screeam);
    test_pass(&tg);
    test_pass(&tg);

    CHECK(rb_has_pending_choice(&tg.state),
          "PROBE: the 「相手に何が好き？」 answer prompt appeared");
    if (!rb_has_pending_choice(&tg.state)) return;

    /* option 0 = mint/flavour/cookie: both put 1 hand card to the waiting room */
    rb_resume_with_choice(&tg.state, 0);

    printf("[probe] after option 0: pending=%d p1_hand=%d p1_wait=%d p2_hand=%d p2_wait=%d\n",
           rb_has_pending_choice(&tg.state), tg.state.p[0].hand.n, tg.state.p[0].discard.n,
           tg.state.p[1].hand.n, tg.state.p[1].discard.n);

    CHECK(rb_has_pending_choice(&tg.state), "PROBE: P1 got the discard prompt");

    int p1h = tg.state.p[0].hand.n, p1w = tg.state.p[0].discard.n;
    int p2h = tg.state.p[1].hand.n, p2w = tg.state.p[1].discard.n;
    if (tg.state.queue.has_pending)
        printf("[probe] pending: zone=%s count=%d skip=%d actor=%d route=%d mode=%d\n",
               tg.state.queue.pending.zone, tg.state.queue.pending.count,
               tg.state.queue.pending.allow_skip, tg.state.queue.pending.actor,
               tg.state.queue.pending.route, tg.state.queue.resume_mode);

    if (rb_has_pending_choice(&tg.state)) { int i0 = 0; test_select_indices(&tg, &i0, 1); }

    printf("[probe] after P1 answer: pending=%d p1_hand=%d p1_wait=%d p2_hand=%d p2_wait=%d\n",
           rb_has_pending_choice(&tg.state), tg.state.p[0].hand.n, tg.state.p[0].discard.n,
           tg.state.p[1].hand.n, tg.state.p[1].discard.n);

    CHECK_EQ(tg.state.p[0].hand.n, p1h - 1, "PROBE: P1 hand shrank by 1");
    CHECK_EQ(tg.state.p[0].discard.n, p1w + 1, "PROBE: P1 waiting room grew by 1");
    CHECK_EQ(tg.state.p[1].hand.n, p2h, "PROBE: P2 hand untouched after P1's answer");
    CHECK_EQ(tg.state.p[1].discard.n, p2w, "PROBE: P2 waiting room untouched after P1's answer");

    CHECK(rb_has_pending_choice(&tg.state), "PROBE: P2 got the discard prompt next");
    if (tg.state.queue.has_pending)
        printf("[probe] pending: zone=%s count=%d skip=%d actor=%d route=%d mode=%d\n",
               tg.state.queue.pending.zone, tg.state.queue.pending.count,
               tg.state.queue.pending.allow_skip, tg.state.queue.pending.actor,
               tg.state.queue.pending.route, tg.state.queue.resume_mode);

    p2h = tg.state.p[1].hand.n; p2w = tg.state.p[1].discard.n;
    if (rb_has_pending_choice(&tg.state)) { int i0 = 0; test_select_indices(&tg, &i0, 1); }

    printf("[probe] after P2 answer: pending=%d p1_hand=%d p1_wait=%d p2_hand=%d p2_wait=%d\n",
           rb_has_pending_choice(&tg.state), tg.state.p[0].hand.n, tg.state.p[0].discard.n,
           tg.state.p[1].hand.n, tg.state.p[1].discard.n);

    CHECK_EQ(tg.state.p[1].hand.n, p2h - 1, "PROBE: P2 hand shrank by 1");
    CHECK_EQ(tg.state.p[1].discard.n, p2w + 1, "PROBE: P2 waiting room grew by 1");
    CHECK_EQ(rb_has_pending_choice(&tg.state), 0, "PROBE: no pending choice at the end");
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    if (load_card_db() != 0) { fprintf(stderr, "FAIL: no card db\n"); return 1; }
    TC = "probe";
    probe();
    rb_unload();
    printf("\n=== tr_resume probe ===\nassertions: %d\nfailures  : %d\n", assertions, failures);
    return failures ? 1 : 0;
}
