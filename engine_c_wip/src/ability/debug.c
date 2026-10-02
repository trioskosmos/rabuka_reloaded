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

/* Trace lines are heap-allocated and kept at their exact length. They used to
   be `char[512]` and formatted with `snprintf(line, 512)`, which clipped the
   TEXT line at 511 bytes: 13 of the 985 real ability texts are longer, so
   13 abilities' printed text reached the log cut mid-sentence with no marker.
   That is the 119-character Out-File incident in miniature, and it sat on the
   one field a reader most needs whole. Rust's ABILITY_LOG_BUFFER is a
   `Mutex<Vec<String>>` (debug.rs:66) — unbounded, and `p` builds the line with
   `format!`, which does not truncate. The C must not truncate either. */
static char *g_debug_buf[RB_DEBUG_BUF_MAX];
static int   g_debug_buf_n = 0;

/* Line-drop accounting, the same defect one layer down: a full trace buffer
   used to `return` at 256 lines in complete silence. Counted, and announced
   ungated on stderr, for the reasons spelled out in include/rabuka.h. */
static unsigned long g_debug_dropped = 0;
static int g_debug_overflow_reported = 0;

int  rb_abdebug_dropped_count(void) { return (int)g_debug_dropped; }
void rb_abdebug_dropped_reset(void) { g_debug_dropped = 0; g_debug_overflow_reported = 0; }

/* `dropped_len` is the SIZE of the evidence lost, not the evidence itself:
   re-dumping a 635-byte ability text into stderr to announce that a copy of it
   was thrown away is the wrong trade, and a self-referential "last tag" field
   grew the diagnostic to four figures on its first firing. */
static void rb_debug_report_overflow(const char *tag, size_t dropped_len) {
    g_debug_dropped++;
    if (g_debug_overflow_reported && (g_debug_dropped % 64) != 0) return;
    g_debug_overflow_reported = 1;
    fprintf(stderr,
            "[AB_DEBUG_OVERFLOW] trace buffer full (%d lines); dropped=%lu "
            "dropping NEWEST (oldest kept) - last tag=%s len=%d\n",
            RB_DEBUG_BUF_MAX, g_debug_dropped, tag ? tag : "?", (int)dropped_len);
}

static void rb_debug_buf_push_line(char *owned_line) {
    if (g_debug_buf_n >= RB_DEBUG_BUF_MAX) {
        rb_debug_report_overflow(NULL, owned_line ? strlen(owned_line) : 0);
        rb_free(owned_line);
        return;
    }
    g_debug_buf[g_debug_buf_n++] = owned_line;
}

static void rb_debug_buf_drain(const char *kind) {
    for (int i = 0; i < g_debug_buf_n; i++) {
        rb_log_push_verdict(g_debug_buf[i], kind, 1);
        rb_free(g_debug_buf[i]);
        g_debug_buf[i] = NULL;
    }
    g_debug_buf_n = 0;
    g_debug_overflow_reported = 0;
}

static void rb_debug_buf_drain_to_rule_log(void) {
    rb_debug_buf_drain("rule_log");
}

static void rb_debug_buf_drain_to_structured_log(void) {
    rb_debug_buf_drain("debug");
}

void rb_abdebug_init(RbAbDebug *d) {
    if (d) d->indent = 0;
}

void rb_abdebug_p(RbAbDebug *d, const char *tag, const char *msg) {
    if (!rb_ability_debug_enabled()) return;
    if (!tag || !msg) return;

    /* `"  ".repeat(self.indent)` (debug.rs:103) — sized from the indent, not
       from a fixed 64-byte pad, so a deep trace cannot silently lose its
       leading spaces either. */
    int indent = d ? d->indent : 0;
    if (indent < 0) indent = 0;
    size_t pad_len = (size_t)indent * 2;
    size_t tag_len = strlen(tag);
    size_t msg_len = strlen(msg);
    size_t total = 3 /* "[AB]" */ + pad_len + tag_len + 1 /* " " */ + msg_len + 1;

    char *line = (char *)rb_malloc(total);
    if (!line) { rb_debug_report_overflow(tag, msg_len); return; }
    size_t w = 0;
    memcpy(line + w, "[AB]", 3); w += 3;
    for (int i = 0; i < indent; i++) { line[w++] = ' '; line[w++] = ' '; }
    memcpy(line + w, tag, tag_len); w += tag_len;
    line[w++] = ' ';
    memcpy(line + w, msg, msg_len); w += msg_len;
    line[w] = '\0';
    rb_debug_buf_push_line(line);
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
    /* Same guard as `condition`/`cost_pay`/`effect` (debug.rs:120). Without it
       this formats two strings on EVERY ability resolution and discards both
       in `p`. */
    if (!rb_ability_debug_enabled()) return;
    if (!ability) return;

    /* Rust prints the PRINTED number as well as the internal id
       (debug.rs:126-129, `"{}" [{}] ({})`): an id alone cannot be checked
       against cards/cards.json, which is the first thing you want when a
       verdict is about the wrong card. This line used to drop `card_no`
       entirely, so the one field that makes the identity checkable was
       missing. Built at its exact length: a card name is corpus data, and a
       fixed `buf[320]` would clip it exactly the way the TEXT line used to be
       clipped. */
    const char *nm = card_name ? card_name : "?";
    const char *no = card_no ? card_no : "?";
    const char *cid_s = card_id ? card_id : "?";
    size_t buf_len = strlen(nm) + strlen(no) + strlen(cid_s) + 8;
    char *buf = (char *)rb_malloc(buf_len);
    if (!buf) { rb_debug_report_overflow("ABILITY", 0); return; }
    snprintf(buf, buf_len, "\"%s\" [%s] (%s)", nm, no, cid_s);
    rb_abdebug_p(d, "ABILITY", buf);
    rb_free(buf);

    /* debug.rs:130 — `self.indent += 1` with NO matching decrement.
       Deliberately reproduced unbalanced: this layer exists so its output can
       be diffed against Rust's, and Rust's TRIGGER/TEXT lines and every later
       RESULT line of the same resolution carry indent 1. Re-adding a decrement
       here would silently re-diverge the trace; this is the reference
       behaviour, not a port slip. */
    if (d) d->indent++;

    const char *trigger_str = ability->triggers ? ability->triggers : "none";
    char limit_buf[64] = {0};
    if (ability->use_limit >= 0) {
        snprintf(limit_buf, sizeof(limit_buf), "%d/turn", ability->use_limit);
    }

    size_t trig_len = strlen(trigger_str) + strlen(limit_buf) + 2;
    char *trig_buf = (char *)rb_malloc(trig_len);
    if (!trig_buf) { rb_debug_report_overflow("TRIGGER", 0); return; }
    snprintf(trig_buf, trig_len, "%s %s", trigger_str, limit_buf);
    rb_abdebug_p(d, "TRIGGER", trig_buf);
    rb_free(trig_buf);

    if (ability->full_text && ability->full_text[0]) {
        rb_abdebug_p(d, "TEXT", ability->full_text);
    }
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
    /* debug.rs:159, `format_args!("{} \u{2192} {status}", cost.action)`. The
     * separator is U+2192 RIGHTWARDS ARROW, three UTF-8 bytes E2 86 92, NOT the
     * ASCII two-character "->" this line used to emit: every COST trace line
     * differed from Rust by exactly those two bytes, so a trace diff reported
     * every cost line as differing and the real differences were lost in it.
     *
     * The arrow is written as an explicit \xHH\xHH\xHH byte escape and NEVER as
     * a literal character. This repo has been bitten twice by literal UTF-8 in
     * C sources: a hand-transcribed hiragana where katakana belonged produced
     * 1,483 phantom disagreements, and a file re-saved through Out-File had its
     * bytes re-encoded. An escape is ASCII, so no editor, no encoding
     * conversion and no codepage can change what the compiler emits. The same
     * convention is already used at resolver.c:1028 (`\xE2\x9C\x93`) and
     * resolver.c:1105 (`\xE2\x80\x94`). Verified: the emitted bytes are
     * E2 86 92. */
    snprintf(buf, sizeof(buf), "%s \xE2\x86\x92 %s", action, ok ? "OK" : "FAIL");
    rb_abdebug_p(d, "COST", buf);
}

void rb_abdebug_effect(RbAbDebug *d, const AbilityEffect *effect) {
    if (!rb_ability_debug_enabled()) return;
    if (!effect) return;

    const char *action = effect->action ? effect->action : "?";
    rb_abdebug_p(d, "EFFECT", action);
}
