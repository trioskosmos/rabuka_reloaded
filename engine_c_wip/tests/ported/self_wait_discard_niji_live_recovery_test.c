#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int failures;
static int checks;

#define CHECK(cond, msg) do { \
    checks++; \
    if (!(cond)) { \
        fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, (msg)); \
        failures++; \
    } else { printf("ok: %s\n", (msg)); } \
} while (0)

#define CHECK_EQ(actual, expected, msg) do { \
    long a_ = (long)(actual), e_ = (long)(expected); \
    checks++; \
    if (a_ != e_) { \
        fprintf(stderr, "FAIL %s:%d: %s (got %ld expected %ld)\n", __FILE__, __LINE__, (msg), a_, e_); \
        failures++; \
    } else { printf("ok: %s\n", (msg)); } \
} while (0)

#define CHECK_STR(actual, expected, msg) do { \
    const char *a_ = (actual), *e_ = (expected); \
    checks++; \
    if (!a_ || !e_ || strcmp(a_, e_) != 0) { \
        fprintf(stderr, "FAIL %s:%d: %s (got %s expected %s)\n", __FILE__, __LINE__, (msg), a_ ? a_ : "(null)", e_ ? e_ : "(null)"); \
        failures++; \
    } else { printf("ok: %s\n", (msg)); } \
} while (0)

static void pz_drain(TestGame *g)
{
    int guard = 0;
    while (rb_has_pending_choice(&g->state) && guard++ < 256)
        rb_resume_with_choice(&g->state, 0);
}

/* ported from n_bp3_004_activation_rest_and_discard_recovers_niji_live */
static void t_n_bp3_004_activation_rest_and_discard_recovers_niji_live(void)
{
    TestGame game;
    test_game_new(&game);
    int me = test_id(&game, "PL!N-bp3-004-R");
    game.state.p[0].stage[0] = me;
    int live = test_id(&game, "PL!N-bp1-025-L");
    test_add_to_discard(&game, live);
    int f1 = test_new_id(&game, "PL!-sd1-010-SD");
    test_add_to_hand(&game, f1);
    test_activate_ability(&game, me);
    CHECK(test_has_pending_choice(&game), "1-card hand-discard cost must be prompted");
    CHECK_STR(test_pending_choice_type(&game), "SelectCard", "expected SelectCard discard-cost prompt");
    test_resume_choice(&game, 0);
    CHECK_EQ(game.state.mods.orientation[me], 2, "this member was rested as part of the cost");
    CHECK(test_zone_has_id(&game, 0, "hand", live), "虹ヶ咲 live card retrieved to hand");
}

int main(void)
{
    printf("--- n_bp3_004_activation_rest_and_discard_recovers_niji_live ---\n");
    t_n_bp3_004_activation_rest_and_discard_recovers_niji_live();
    printf("\n%s: %d checks, %d failures\n", "self_wait_discard_niji_live_recovery_test", checks, failures);
    return failures ? 1 : 0;
}
