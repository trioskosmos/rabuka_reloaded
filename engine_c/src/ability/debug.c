#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdatomic.h>

static atomic_bool g_ability_debug = ATOMIC_VAR_INIT(false);

int rb_ability_debug_enabled(void) {
    return atomic_load_explicit(&g_ability_debug, memory_order_relaxed);
}

void rb_ability_debug_set(int enabled) {
    atomic_store_explicit(&g_ability_debug, (atomic_bool)enabled, memory_order_seq_cst);
}

#define RB_DEBUG_BUF_MAX 256
#define RB_DEBUG_BUF_LINE 512

static char g_debug_buf[RB_DEBUG_BUF_MAX][RB_DEBUG_BUF_LINE];
static int g_debug_buf_n = 0;

static void rb_debug_buf_push(const char *line) {
    if (!line || !rb_ability_debug_enabled()) return;
    if (g_debug_buf_n >= RB_DEBUG_BUF_MAX) return;
    strncpy(g_debug_buf[g_debug_buf_n], line, RB_DEBUG_BUF_LINE - 1);
    g_debug_buf[g_debug_buf_n][RB_DEBUG_BUF_LINE - 1] = '\0';
    g_debug_buf_n++;
}

static void rb_debug_buf_drain_to_rule_log(void) {
    for (int i = 0; i < g_debug_buf_n; i++) {
        rb_log_push_verdict(g_debug_buf[i], "rule_log", 1);
    }
    g_debug_buf_n = 0;
}

static void rb_debug_buf_drain_to_structured_log(void) {
    for (int i = 0; i < g_debug_buf_n; i++) {
        rb_log_push_verdict(g_debug_buf[i], "debug", 1);
    }
    g_debug_buf_n = 0;
}

typedef struct RbAbDebug {
    int indent;
} RbAbDebug;

void rb_abdebug_init(RbAbDebug *d) {
    if (d) d->indent = 0;
}

void rb_abdebug_p(RbAbDebug *d, const char *tag, const char *msg) {
    if (!rb_ability_debug_enabled()) return;
    if (!tag || !msg) return;

    char pad[64] = {0};
    for (int i = 0; i < (d ? d->indent : 0) && i < 32; i++) {
        pad[i * 2] = ' ';
        pad[i * 2 + 1] = ' ';
    }

    char line[RB_DEBUG_BUF_LINE];
    snprintf(line, sizeof(line), "[AB]%s%s %s", pad, tag, msg);
    rb_debug_buf_push(line);
}

void rb_abdebug_flush_to_rule_log(RbAbDebug *d) {
    (void)d;
    rb_debug_buf_drain_to_rule_log();
}

void rb_abdebug_flush_to_structured_log(RbAbDebug *d) {
    (void)d;
    rb_debug_buf_drain_to_structured_log();
}

void rb_abdebug_ability(RbAbDebug *d, const char *card_name, const char *card_no,
                        const char *card_id, const Ability *ability) {
    if (!ability) return;

    char buf[256];
    snprintf(buf, sizeof(buf), "\"%s\" (%s)",
             card_name ? card_name : "?",
             card_id ? card_id : "?");
    rb_abdebug_p(d, "ABILITY", buf);

    if (d) d->indent++;

    const char *trigger_str = ability->triggers ? ability->triggers : "none";
    char limit_buf[64] = {0};
    if (ability->use_limit >= 0) {
        snprintf(limit_buf, sizeof(limit_buf), "%d/turn", ability->use_limit);
    }

    char trig_buf[256];
    snprintf(trig_buf, sizeof(trig_buf), "%s %s", trigger_str, limit_buf);
    rb_abdebug_p(d, "TRIGGER", trig_buf);

    if (ability->full_text && ability->full_text[0]) {
        rb_abdebug_p(d, "TEXT", ability->full_text);
    }

    if (d) d->indent--;
}

void rb_abdebug_condition(RbAbDebug *d, const Condition *cond, int actual,
                          int threshold, int passed) {
    if (!rb_ability_debug_enabled()) return;
    if (!cond) return;

    const char *ct = "?";
    switch (cond->variant) {
        case RB_COND_COMPOUND:           ct = "compound"; break;
        case RB_COND_LOCATION:           ct = "location"; break;
        case RB_COND_COMPARISON:         ct = "comparison"; break;
        case RB_COND_MOVEMENT:           ct = "movement"; break;
        case RB_COND_GROUP:              ct = "group"; break;
        case RB_COND_APPEARANCE:         ct = "appearance"; break;
        case RB_COND_TEMPORAL:           ct = "temporal"; break;
        case RB_COND_STATE:              ct = "state"; break;
        case RB_COND_RESOURCE:           ct = "resource"; break;
        case RB_COND_ABILITY_FILTER:     ct = "ability_filter"; break;
        case RB_COND_SCORE_THRESHOLD:    ct = "score_threshold"; break;
        case RB_COND_CHOICE:             ct = "choice"; break;
        case RB_COND_COMPLEX:            ct = "complex"; break;
        case RB_COND_POSITION:           ct = "position"; break;
        case RB_COND_OPPONENT_CHOICE:    ct = "opponent_choice"; break;
        case RB_COND_OPPONENT_LIVE_SUCCESS: ct = "opponent_live_success"; break;
        case RB_COND_NO_EXCESS_HEART:    ct = "no_excess_heart"; break;
        case RB_COND_ALWAYS_TRUE:        ct = "always_true"; break;
        case RB_COND_ANY_OF:             ct = "any_of"; break;
        case RB_COND_ALL_REVEALED:       ct = "all_revealed"; break;
        default:                         ct = "?"; break;
    }

    char buf[256];
    snprintf(buf, sizeof(buf), "%s actual=%d threshold=%d %s",
             ct, actual, threshold, passed ? "PASS" : "FAIL");
    rb_abdebug_p(d, "COND", buf);
}

void rb_abdebug_cost_pay(RbAbDebug *d, const AbilityEffect *cost, int ok) {
    if (!rb_ability_debug_enabled()) return;
    if (!cost) return;

    const char *action = cost->action ? cost->action : "?";
    char buf[256];
    snprintf(buf, sizeof(buf), "%s -> %s", action, ok ? "OK" : "FAIL");
    rb_abdebug_p(d, "COST", buf);
}

void rb_abdebug_effect(RbAbDebug *d, const AbilityEffect *effect) {
    if (!rb_ability_debug_enabled()) return;
    if (!effect) return;

    const char *action = effect->action ? effect->action : "?";
    rb_abdebug_p(d, "EFFECT", action);
}
