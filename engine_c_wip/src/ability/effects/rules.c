/* rules.c — faithful C ports of the eight unowned Rust effect modules:
 *
 *   engine/src/ability/effects/required_hearts.rs  (6 fns)
 *   engine/src/ability/effects/cost.rs             (4 fns)
 *   engine/src/ability/effects/target.rs           (6 fns, 2 of them free fns)
 *   engine/src/ability/effects/selection.rs        (8 fns, 2 of them private)
 *   engine/src/ability/effects/choice.rs           (3 fns, 1 of them private)
 *   engine/src/ability/effects/restriction.rs      (4 fns)
 *   engine/src/ability/effects/reyell.rs           (3 fns)
 *   engine/src/ability/effects/replacement.rs      (2 fns)
 *
 * Every exported entry point is prefixed `rb_rules_` so nothing here can collide
 * with the sibling executors in misc.c / state.c / draw.c / score.c.  The
 * orchestrator owns the Makefile and rabuka.h; this file is not yet in the
 * build.  It compiles standalone with:
 *
 *   gcc -fsyntax-only -std=c11 -Iinclude -Isrc -Isrc/core/generated \
 *       src/ability/effects/rules.c
 *
 * -- What lives in a side table and why -------------------------------------
 * Rust keeps a handful of pieces of resolver/GameState state that the C
 * GameState has no field for yet (delayed prohibitions, wait immunity,
 * changed_state_members, conditional_choice, choice_repeats_remaining, the
 * replacement bodies, ...).  Rather than drop those branches (which would make
 * the ports stubs) each one is kept in a per-GameState side table, following the
 * precedent already set by state.c's `RbAllBladeTimingNote` and live.c's
 * `RbCardFirstAppearanceCache`.  Each table is exposed through a `rb_rules_*`
 * accessor so the owning files can later be rewired to real GameState fields
 * without changing any call site.
 */

#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "rabuka.h"

/* -- Defined in sibling .c files but not yet declared in rabuka.h ---------- */
extern RbPlayer     *rb_resolve_target_player_mut(GameState *g, const char *target);
extern RbQueueEntry *rb_queue_current_entry_mut(GameState *g);

/* ==========================================================================
   0. Per-GameState side tables
   ========================================================================== */

#define RB_RULES_PROHIB_CAP 48
#define RB_RULES_LIST_CAP   32
#define RB_RULES_MOVED_CAP  16
#define RB_RULES_STR_SZ     64

/* Rust `GameState::wait_immune_members: Vec<(i16, PlayerId)>`. */
typedef struct {
    int  member_id;
    int  owner_pl;          /* which player owns the protected member */
} RbRulesWaitImmune;

/* Rust `GameState::replacement_effects[i].replacement_effects: Vec<AbilityEffect>`.
   The C `RbReplacementEffect` has no body field, so the registered body is
   parked here against the registration slot index. */
typedef struct {
    AbilityEffect *body;
} RbRulesReplacementBody;

typedef struct {
    const GameState *owner;

    /* --- restriction.rs --- */
    char delayed_prohibition[RB_RULES_PROHIB_CAP][96];
    int  n_delayed_prohibition;
    int  cannot_activate_members[2];   /* per player slot */
    int  cannot_live_players[2];
    RbRulesWaitImmune wait_immune[RB_RULES_LIST_CAP];
    int  n_wait_immune;

    /* --- target.rs / selection.rs / choice.rs: the resolver SelectionContext --- */
    int  changed_state_members[RB_RULES_MOVED_CAP];
    int  n_changed_state_members;
    int  moved_cards[RB_RULES_MOVED_CAP];
    int  n_moved_cards;
    int  selected_cards[RB_RULES_MOVED_CAP];
    int  n_selected_cards;
    int  count_at_save;          /* -1 = None (Rust Option<u8>) */
    int  sub_choice_created;

    /* Rust `AbilityQueueEntry.conditional_choice: Option<ConditionalChoice>`.
       Only the `Str` and `Strings` payloads are used by these eight modules. */
    char conditional_choice[128];
    int  has_conditional_choice;

    /* Rust `AbilityQueueEntry.choice_repeats_remaining: Option<u8>`. */
    int  choice_repeats_remaining;   /* -1 = None */

    /* selection.rs C6 keep-N-shuffle-under phases. */
    int  keep_shuffle_phase;
    int  keep_shuffle_count;
    int  keep_shuffle_selected[RB_RULES_MOVED_CAP];
    int  n_keep_shuffle_selected;
    int  keep_shuffle_snapshot[2][RB_MAX_ZONE];
    int  n_keep_shuffle_snapshot[2];

    /* selection.rs `self.owner.executing: Option<AbilityEffect>`. */
    AbilityEffect *owner_executing;

    /* replacement.rs `awaiting.replacement` / `awaiting.resolving_replacement`. */
    int             replacement_index;   /* -1 = None */
    AbilityEffect  *replacement_source;
    int             resolving_replacement;
    RbRulesReplacementBody replacement_body[RB_RULES_LIST_CAP];
    int             n_replacement_body;
} RbRulesState;

static RbRulesState rb_rules_states[4];
static int          rb_rules_n_states = 0;

static RbRulesState *rb_rules_state(GameState *g)
{
    if (!g) return &rb_rules_states[0];
    for (int i = 0; i < rb_rules_n_states; i++)
        if (rb_rules_states[i].owner == (const GameState *)g) return &rb_rules_states[i];
    RbRulesState *s;
    if (rb_rules_n_states < (int)(sizeof(rb_rules_states) / sizeof(rb_rules_states[0]))) {
        s = &rb_rules_states[rb_rules_n_states++];
    } else {
        s = &rb_rules_states[0];
    }
    if (s->owner_executing)     rb_effect_free(s->owner_executing);
    if (s->replacement_source)  rb_effect_free(s->replacement_source);
    for (int i = 0; i < s->n_replacement_body; i++)
        if (s->replacement_body[i].body) rb_effect_free(s->replacement_body[i].body);
    memset(s, 0, sizeof(*s));
    s->owner = g;
    s->count_at_save = -1;
    s->choice_repeats_remaining = -1;
    s->replacement_index = -1;
    return s;
}

/* Drop every side table belonging to `g` (call from the engine's reset path). */
void rb_rules_forget(GameState *g)
{
    for (int i = 0; i < rb_rules_n_states; i++) {
        if (rb_rules_states[i].owner != (const GameState *)g) continue;
        if (rb_rules_states[i].owner_executing)
            rb_effect_free(rb_rules_states[i].owner_executing);
        if (rb_rules_states[i].replacement_source)
            rb_effect_free(rb_rules_states[i].replacement_source);
        memset(&rb_rules_states[i], 0, sizeof(rb_rules_states[i]));
        rb_rules_states[i].count_at_save = -1;
        rb_rules_states[i].choice_repeats_remaining = -1;
        rb_rules_states[i].replacement_index = -1;
    }
}

/* ==========================================================================
   1. Small local helpers
   ========================================================================== */

static const char *r_extra(const AbilityEffect *e, const char *key)
{
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], key)) return e->extra_v[i];
    return NULL;
}

static int r_extra_int(const AbilityEffect *e, const char *key, int dflt)
{
    const char *v = r_extra(e, key);
    return v ? atoi(v) : dflt;
}

static int r_extra_bool(const AbilityEffect *e, const char *key, int dflt)
{
    const char *v = r_extra(e, key);
    if (!v) return dflt;
    return (!strcmp(v, "true") || !strcmp(v, "1"));
}

static const char *r_either(const AbilityEffect *e, const char *a, const char *b)
{
    const char *v = r_extra(e, a);
    return v ? v : r_extra(e, b);
}

static int r_is_true(const char *v)
{
    return v && (!strcmp(v, "true") || !strcmp(v, "1"));
}

/* Rust `effect.value_or_count(default)`. */
static int r_value_or_count(const AbilityEffect *e, int dflt)
{
    return rb_effect_value_or_count(e, dflt);
}

/* Rust `effect.count_or(default)`. */
static int r_count_or(const AbilityEffect *e, int dflt)
{
    return rb_effect_count_or(e, dflt);
}

/* Rust `gs.resolve_target_player(target)` -> player slot 0/1. */
static int r_player(int actor, const char *target)
{
    return rb_target_player_index(target, actor == 0 ? "p1" : "p2");
}

/* Rust `self.player_prefix(gs)` -> "player1" / "player2". */
static void r_player_prefix(const GameState *g, int actor, char *out, size_t n)
{
    int pl = actor;
    if (g && g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
        pl = (g->queue.entries[g->queue.cur].player_id[0] == '2') ? 1 : 0;
    snprintf(out, n, "player%d", pl + 1);
}

/* Rust `self.card_name(gs.activating_card)`. */
static const char *r_activating_name(const GameState *g)
{
    static char buf[64];
    buf[0] = 0;
    if (!g || g->activating_card < 0) return buf;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)g->activating_card, &c)) return buf;
    snprintf(buf, sizeof(buf), "%s", c.name ? c.name : "");
    rb_free_card(&c);
    return buf;
}

/* Rust `gs.push_rule_log_fmt(...)`. */
static void r_log_effect(GameState *g, int actor, const char *key, const char *fmt, ...)
{
    char body[192];
    va_list ap;
    if (!g) return;
    va_start(ap, fmt);
    vsnprintf(body, sizeof(body), fmt, ap);
    va_end(ap);
    char pp[16];
    r_player_prefix(g, actor, pp, sizeof(pp));
    char msg[256];
    snprintf(msg, sizeof(msg), "%s %s: %s", pp, r_activating_name(g), body);
    rb_log_push_verdict(msg, key, 1);
}

/* Rust `gs.record_ability_application(...)`. */
static void r_record(GameState *g, const AbilityEffect *e, int effect_type,
                     int target, int amount)
{
    if (!g || g->activating_card < 0) return;
    rb_mods_trace_push(&g->mods, g->activating_card, e ? e->text : "",
                       effect_type, target, -1, amount);
}

/* Rust `gs.prohibition_effects.push(...)`. */
static void r_prohibition(GameState *g, const char *entry)
{
    if (!g || !entry) return;
    if (g->n_prohibition >= 64) return;
    snprintf(g->prohibition[g->n_prohibition], sizeof(g->prohibition[0]), "%s", entry);
    g->n_prohibition++;
}

/* Split a comma / ideographic-comma separated list; `stride` is the element size. */
static int r_split(const char *s, char *out, int stride, int max)
{
    if (!s || !*s) return 0;
    char buf[512];
    snprintf(buf, sizeof(buf), "%s", s);
    int n = 0;
    char *p = buf;
    while (*p && n < max) {
        char *tok = p;
        while (*p && *p != ',') p++;
        if (*p == ',') { *p = 0; p++; }
        while (*tok == ' ' || *tok == '\t') tok++;
        char *end = tok + strlen(tok);
        while (end > tok && (end[-1] == ' ' || end[-1] == '\t')) end--;
        *end = 0;
        if (!*tok) continue;
        snprintf(out + (size_t)n * stride, stride, "%s", tok);
        n++;
    }
    return n;
}

static int r_heart_colors(const AbilityEffect *e, char out[8][24])
{
    return r_split(rb_effect_heart_colors_any(e), &out[0][0], 24, 8);
}

static int r_exclude_heart_colors(const AbilityEffect *e, char out[8][24])
{
    return r_split(rb_effect_exclude_heart_colors_any(e), &out[0][0], 24, 8);
}

static int r_has_groups(const AbilityEffect *e)      { const char *v = r_extra(e, "group_names");            return v && *v; }
static int r_has_exclude_groups(const AbilityEffect *e) { const char *v = r_extra(e, "exclude_group_names"); return v && *v; }
static int r_has_characters(const AbilityEffect *e)  { const char *v = r_extra(e, "characters");             return v && *v; }
static int r_has_exclude_characters(const AbilityEffect *e) { const char *v = r_extra(e, "exclude_characters"); return v && *v; }
static int r_filter_applies(const AbilityEffect *e)
{
    return r_has_groups(e) || r_has_exclude_groups(e) ||
           r_has_characters(e) || r_has_exclude_characters(e);
}
static int r_distinct(const AbilityEffect *e) { return r_is_true(r_extra(e, "distinct")); }
static int r_self_target(const AbilityEffect *e) { return r_is_true(e->self_target_field); }

static RbDistinctType r_distinct_type(const AbilityEffect *e)
{
    const char *d = r_extra(e, "distinct");
    if (!d) return RB_DISTINCT_NONE;
    if (!strcmp(d, "CardName"))  return RB_DISTINCT_CARDNAME;
    if (!strcmp(d, "True"))      return RB_DISTINCT_TRUE;
    if (!strcmp(d, "Distinct"))  return RB_DISTINCT_DISTINCT;
    return RB_DISTINCT_NONE;
}

/* Rust `effect.position_any().and_then(|p| p.get_position())`. */
static const char *r_position(const AbilityEffect *e)
{
    return r_either(e, "position", "source_position");
}

/* Push `src` into the current entry's pending_actions at index 0. */
static void r_push_pending_front(GameState *g, const AbilityEffect *src)
{
    RbQueueEntry *en = rb_queue_current_entry_mut(g);
    if (!en || !src) return;
    if (en->pending_actions_n >= RB_ENTRY_PENDING_CAP) return;
    for (int i = en->pending_actions_n; i > 0; i--)
        en->pending_actions[i] = en->pending_actions[i - 1];
    en->pending_actions[0] = rb_effect_deep_clone(src);
    en->pending_actions_n++;
}

/* rust `GameState::has_card_moved_this_turn`. */
static int r_has_moved_this_turn(const GameState *g, int card_id)
{
    if (!g || card_id < 0 || card_id >= RB_MAX_CARD_IDS) return 0;
    return g->moved_this_turn[card_id] != 0;
}

/* rust `CardFilter::check_card_property` single-property pass. */
static int r_card_property(const AbilityEffect *e, int card_id)
{
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int r = rb_check_card_property(r_extra(e, "property"),
                                   r_extra_bool(e, "property_negation", 0), &c);
    rb_free_card(&c);
    return r;
}

/* The C RbReplacementEffect has no per-index applied setter; drive the field. */
static void r_mark_replacement_applied(GameState *g, int idx)
{
    if (!g || idx < 0 || idx >= g->n_replacement_effects) return;
    g->replacement_effects[idx].applied_this_event = 1;
}

/* -- PerUnitFilter (engine/src/ability/effects/score.rs:59-148) ------------- */
typedef struct {
    const char *group;
    int         distinct_names;
    const char *timing_condition;
    int         exclude_heart_colors_n;
    char        exclude_heart_colors[8][24];
    int         exclude_self;
} RbRulesPerUnitFilter;

static void r_per_unit_filter(const AbilityEffect *e, int exclude_self,
                              RbRulesPerUnitFilter *f)
{
    memset(f, 0, sizeof(*f));
    f->group = r_has_groups(e) ? r_extra(e, "group_names") : NULL;
    f->distinct_names = r_distinct(e);
    f->timing_condition = r_extra(e, "timing_condition");
    f->exclude_heart_colors_n = r_exclude_heart_colors(e, f->exclude_heart_colors);
    f->exclude_self = exclude_self;
}

/* Rust `PerUnitFilter::claim_distinct_name`. */
static int r_claim_distinct_name(int card_id, char seen[][96], int *n_seen)
{
    char key[96];
    char names[8][64];
    int n_names = rb_card_get_card_names(card_id, names[0], sizeof(names[0]));
    if (n_names <= 1) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
        rb_card_normalize_name(c.name, key, sizeof(key));
        rb_free_card(&c);
    } else {
        key[0] = 0;
        for (int i = 0; i < n_names && i < 8; i++) {
            if (i) strncat(key, "|", sizeof(key) - strlen(key) - 1);
            strncat(key, names[i], sizeof(key) - strlen(key) - 1);
        }
    }
    for (int i = 0; i < *n_seen; i++)
        if (!strcmp(seen[i], key)) return 0;
    if (*n_seen < RB_RULES_MOVED_CAP)
        snprintf(seen[(*n_seen)++], 96, "%s", key);
    return 1;
}

/* Rust `PerUnitFilter::heart_set_all_excluded`. */
static int r_heart_set_all_excluded(int card_id, const char (*excluded)[24], int n_excluded)
{
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int r = 0;
    if (c.num_base > 0) {
        r = 1;
        for (int i = 0; i < c.n_hearts && i < c.num_base; i++) {
            int color = rb_heart_index(c.heart_color[i]);
            const char *name = rb_heart_color_as_str(color);
            int hit = 0;
            for (int k = 0; k < n_excluded; k++)
                if (!strcmp(excluded[k], name)) { hit = 1; break; }
            if (!hit) { r = 0; break; }
        }
    }
    rb_free_card(&c);
    return r;
}

/* Rust `PerUnitFilter::accepts` (score.rs:69-95). */
static int r_per_unit_accepts(GameState *g, const RbRulesPerUnitFilter *f,
                              int card_id, char seen[][96], int *n_seen)
{
    if (f->exclude_self >= 0 && g->activating_card == card_id) return 0;
    if (f->group && *f->group && !rb_card_matches_group_str(card_id, f->group)) return 0;
    if (f->distinct_names && !r_claim_distinct_name(card_id, seen, n_seen)) return 0;
    if (f->timing_condition &&
        !strcmp(f->timing_condition, "appeared_or_moved_this_turn") &&
        !r_has_moved_this_turn(g, card_id) &&
        !rb_has_card_appeared_this_turn(g, card_id))
        return 0;
    if (f->exclude_heart_colors_n > 0 &&
        r_heart_set_all_excluded(card_id, f->exclude_heart_colors,
                                 f->exclude_heart_colors_n))
        return 0;
    return 1;
}

/* Rust `OriginalScoreFilter` (score.rs:150-190). */
typedef struct {
    int         enabled;
    int         count;      /* Option<u8> — -1 = None */
    const char *operator;   /* Option<&str> */
} RbRulesOriginalScore;

static void r_original_score_filter(const AbilityEffect *e, RbRulesOriginalScore *f)
{
    memset(f, 0, sizeof(*f));
    f->enabled = r_extra_bool(e, "original_value", 0);
    f->count = r_extra(e, "original_value") ? atoi(r_extra(e, "original_value")) : -1;
    f->operator = r_extra(e, "original_operator");
}

static int r_original_score_accepts(const RbRulesOriginalScore *f, int card_id)
{
    if (!f->enabled) return 1;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 1;
    int has_score = c.score > 0;
    int score = c.score;
    rb_free_card(&c);
    /* A card with no score, or a threshold the text did not fully state, is
       not filtered out. */
    if (!has_score || f->count < 0 || !f->operator) return 1;
    return rb_compare_counts(f->operator, score, f->count) != 0;
}

/* Rust `capped_units(total, per_unit_count, repeat_limit)` (score.rs:392-410). */
static int r_capped_units(int total, int per_unit_count, int repeat_limit)
{
    if (per_unit_count < 1) per_unit_count = 1;
    int units = total / per_unit_count;
    if (repeat_limit > 0 && units > repeat_limit) units = repeat_limit;
    return units;
}

/* Rust `per_unit_base(max, value)`. */
static int r_per_unit_base(int is_max, int value) { return is_max ? 1 : value; }

/* Rust `per_unit_source_cards(...)`: the id pool a per-unit clause counts.
   `under_member` / hand / deck / discard / waitroom are read off the resolved
   player directly, because RbCardFilter cannot express them. */
static int r_per_unit_source(const GameState *g, int pl, const char *put,
                             const int *stage_ids, int n_stage,
                             int *out, int max)
{
    const RbPlayer *p = &g->p[pl];
    if (!put)                       { memcpy(out, stage_ids, (size_t)n_stage * sizeof(int)); return n_stage; }
    if (!strcmp(put, "stage"))      { memcpy(out, stage_ids, (size_t)n_stage * sizeof(int)); return n_stage; }
    if (!strcmp(put, "hand"))       { memcpy(out, p->hand.cards, sizeof(p->hand.cards));    return p->hand.n; }
    if (!strcmp(put, "discard"))    { memcpy(out, p->discard.cards, sizeof(p->discard.cards)); return p->discard.n; }
    if (!strcmp(put, "waitroom_card") || !strcmp(put, "waitroom")) {
        memcpy(out, p->discard.cards, sizeof(p->discard.cards)); return p->discard.n;
    }
    if (!strcmp(put, "live") || !strcmp(put, "live_card")) {
        memcpy(out, p->live.cards, sizeof(p->live.cards)); return p->live.n;
    }
    if (!strcmp(put, "energy") || !strcmp(put, "energy_card")) {
        memcpy(out, p->energy.cards, sizeof(p->energy.cards)); return p->energy.n;
    }
    if (!strcmp(put, "success") || !strcmp(put, "success_zone")) {
        memcpy(out, p->success.cards, sizeof(p->success.cards)); return p->success.n;
    }
    if (!strcmp(put, "deck")) {
        memcpy(out, p->deck.cards, sizeof(p->deck.cards)); return p->deck.n;
    }
    if (!strcmp(put, "under_member")) {
        int n = 0;
        for (int s = 0; s < RB_STAGE_SIZE && n < max; s++) {
            if (p->stage[s] < 0) continue;
            n += rb_stage_get_under_cards(p, s, out + n, max - n);
        }
        return n;
    }
    memcpy(out, stage_ids, (size_t)n_stage * sizeof(int));
    return n_stage;
}

/* Shared per-unit resolution (PerUnitFilter + OriginalScoreFilter), used by
   required_hearts.rs and by the cost.rs / choice.rs per-unit clauses. */
static int r_per_unit_units(GameState *g, int actor, const AbilityEffect *e, int *hearts)
{
    int pl = r_player(actor, rb_effect_target_name(e));
    const char *put = r_either(e, "per_unit_type", "location");
    int stage_ids[RB_STAGE_SIZE];
    int n_stage = 0;
    for (int s = 0; s < RB_STAGE_SIZE; s++)
        if (g->p[pl].stage[s] >= 0) stage_ids[n_stage++] = g->p[pl].stage[s];
    int pool[RB_MAX_ZONE];
    int n = r_per_unit_source(g, pl, put, stage_ids, n_stage, pool, RB_MAX_ZONE);

    RbRulesPerUnitFilter f;
    r_per_unit_filter(e, g->activating_card, &f);
    RbRulesOriginalScore os;
    r_original_score_filter(e, &os);
    RbCardFilter cf;
    int has_cf = rb_effect_filter_subset(e, &cf) && cf.has_filter;
    const char *state = r_extra(e, "state");

    int survivors[RB_MAX_ZONE];
    int n_surv = 0;
    char seen[RB_RULES_MOVED_CAP][96];
    int n_seen = 0;
    int tmp[1];
    for (int i = 0; i < n; i++) {
        int cid = pool[i];
        if (cid < 0) continue;
        if (has_cf) {
            tmp[0] = cid;
            if (rb_matching_ids(&cf, tmp, 1, tmp, 1) != 1) continue;
        }
        if (state && *state && !r_card_property(e, cid)) continue;
        if (!r_per_unit_accepts(g, &f, cid, seen, &n_seen)) continue;
        if (!r_original_score_accepts(&os, cid)) continue;
        if (n_surv < RB_MAX_ZONE) survivors[n_surv++] = cid;
    }
    RbDistinctType dt = r_distinct_type(e);
    if (rb_distinct_should_dedupe(dt) && n_surv > 0)
        n_surv = rb_apply_distinct_filter(survivors, n_surv, dt, survivors, RB_MAX_ZONE);

    int sum = 0;
    for (int i = 0; i < n_surv; i++) {
        int h[8];
        rb_effective_need_heart(g, survivors[i], h);
        for (int k = 0; k < 8; k++) sum += h[k];
    }
    (void)hearts;
    return r_capped_units(sum, e->per_unit_count > 0 ? e->per_unit_count : 1,
                          rb_effect_repeat_limit_any(e));
}

/* Does the card carry at least one of the listed heart colours? */
static int r_has_heart_color_any(int card_id, const char *const *colors, int n)
{
    if (n <= 0) return 1;
    return rb_card_matches_heart_colors(card_id, (const char **)colors, n);
}

/* ==========================================================================
   2. required_hearts.rs
   ========================================================================== */

/* required_hearts.rs:27 execute_modify_required_hearts
   Returns the number of cards modified, or -1 on a rule error. */
int rb_rules_execute_modify_required_hearts(GameState *g, int actor,
                                            const AbilityEffect *e)
{
    if (!g || !e) return 0;
    const char *operation      = r_either(e, "operation", "effect_type");
    const char *exclude_card_type = r_extra(e, "exclude_card_type");
    const char *target = rb_effect_target_name(e);
    int self_target = r_self_target(e);
    int n = 0;

    if (r_is_true(r_extra(e, "reduce_required_hearts"))) {
        /* required_hearts.rs:50-74 */
        r_log_effect(g, actor, "log_reduce_required_hearts", "%s",
                     operation ? operation : "(unset)");
        int pl = r_player(actor, target);
        const char *hcs = r_either(e, "per_unit_heart_colors", "heart_colors");
        int value = r_value_or_count(e, 0);
        int is_max = r_extra_bool(e, "max", 0);
        int base_unit = r_per_unit_base(is_max, 1);

        if (hcs && *hcs) {
            /* `per_unit == "heart_colors"`: distribute the reduction across the
               member's OWN need-heart columns. */
            char cols[8][24];
            const char *list[8];
            int nc = r_split(hcs, &cols[0][0], 24, 8);
            for (int i = 0; i < nc; i++) list[i] = cols[i];
            int units = r_per_unit_units(g, actor, e, NULL);
            for (int s = 0; s < RB_STAGE_SIZE; s++) {
                int cid = g->p[pl].stage[s];
                if (cid < 0) continue;
                if (self_target && g->activating_card >= 0 &&
                    cid != g->activating_card) continue;
                if (exclude_card_type && !strcmp(exclude_card_type, "live_card") &&
                    rb_card_is_live(cid)) continue;
                if (!r_has_heart_color_any(cid, list, nc)) continue;
                int hv[8];
                rb_effective_need_heart(g, cid, hv);
                int total = 0;
                for (int i = 0; i < 8; i++) total += hv[i];
                if (total == 0) continue;
                for (int i = 0; i < 8; i++) {
                    if (hv[i] <= 0) continue;
                    int dec = hv[i] * value * units / (base_unit * total);
                    if (dec <= 0) dec = 1;
                    rb_mods_add_need_heart(&g->mods, cid, i, -dec);
                    r_record(g, e, RB_EFFECT_NEED_HEART_MOD, cid, -dec);
                }
                n++;
            }
        } else {
            int col = rb_heart_index(rb_parse_heart_color(rb_effect_heart_colors_any(e)));
            int units = (r_extra_bool(e, "per_unit", 0) || e->per_unit)
                            ? r_per_unit_units(g, actor, e, NULL) : 1;
            int v = r_per_unit_base(is_max, value);
            if (v != 0) {
                for (int s = 0; s < RB_STAGE_SIZE; s++) {
                    int cid = g->p[pl].stage[s];
                    if (cid < 0) continue;
                    if (self_target && g->activating_card >= 0 &&
                        cid != g->activating_card) continue;
                    if (exclude_card_type && !strcmp(exclude_card_type, "live_card") &&
                        rb_card_is_live(cid)) continue;
                    if (operation && !strcmp(operation, "decrease")) {
                        rb_mods_add_need_heart(&g->mods, cid, col, -(v * units));
                        r_record(g, e, RB_EFFECT_NEED_HEART_MOD, cid, -(v * units));
                    } else {
                        rb_mods_set_need_heart(&g->mods, cid, col,
                                               (uint8_t)(v * units));
                        r_record(g, e, RB_EFFECT_HEART_OVERRIDE, cid, v * units);
                    }
                    n++;
                }
            }
        }
        return n;
    }

    /* required_hearts.rs:80-112 — the global / standard form. */
    r_log_effect(g, actor, "log_modify_required_hearts", "%s",
                 operation ? operation : "(unset)");
    int col = rb_heart_index(rb_parse_heart_color(rb_effect_heart_colors_any(e)));
    int units = (r_extra_bool(e, "per_unit", 0) || e->per_unit)
                    ? r_per_unit_units(g, actor, e, NULL) : 1;
    int per_color = r_per_unit_base(r_extra_bool(e, "max", 0), r_value_or_count(e, 0));

    if (self_target && g->activating_card >= 0) {
        if (operation && !strcmp(operation, "decrease")) {
            rb_mods_add_need_heart(&g->mods, g->activating_card, col,
                                   -(per_color * units));
            r_record(g, e, RB_EFFECT_NEED_HEART_MOD, g->activating_card,
                     -(per_color * units));
        } else if (operation && !strcmp(operation, "set")) {
            rb_mods_set_need_heart(&g->mods, g->activating_card, col,
                                   (uint8_t)(per_color * units));
            r_record(g, e, RB_EFFECT_HEART_OVERRIDE, g->activating_card,
                     per_color * units);
        } else if (operation && !strcmp(operation, "add")) {
            rb_mods_add_need_heart(&g->mods, g->activating_card, col,
                                   per_color * units);
            r_record(g, e, RB_EFFECT_NEED_HEART_MOD, g->activating_card,
                     per_color * units);
        } else {
            return -1;   /* Rust: Err("Unknown operation") */
        }
        return 1;
    }

    int pl = r_player(actor, target);
    for (int s = 0; s < RB_STAGE_SIZE; s++) {
        int cid = g->p[pl].stage[s];
        if (cid < 0) continue;
        if (!rb_card_is_member(cid)) continue;
        if (r_extra_bool(e, "exclude_self", 0) && cid == g->activating_card) continue;
        if (operation && !strcmp(operation, "decrease")) {
            rb_mods_add_need_heart(&g->mods, cid, col, -(per_color * units));
            r_record(g, e, RB_EFFECT_NEED_HEART_MOD, cid, -(per_color * units));
        } else if (operation && !strcmp(operation, "set")) {
            rb_mods_set_need_heart(&g->mods, cid, col, (uint8_t)(per_color * units));
            r_record(g, e, RB_EFFECT_HEART_OVERRIDE, cid, per_color * units);
        } else if (operation && !strcmp(operation, "add")) {
            rb_mods_add_need_heart(&g->mods, cid, col, per_color * units);
            r_record(g, e, RB_EFFECT_NEED_HEART_MOD, cid, per_color * units);
        } else {
            return -1;   /* Rust: Err("Unknown operation") */
        }
        n++;
    }
    return n;
}

/* required_hearts.rs:116 execute_modify_required_hearts_standard */
int rb_rules_execute_modify_required_hearts_standard(GameState *g, int actor,
        const char *operation, int value, const char **heart_colors, int n_colors,
        const char *target, const char *effect_text)
{
    (void)actor; (void)effect_text;
    if (!g) return 0;
    int colors[8];
    int n = 0;
    for (int i = 0; i < n_colors && i < 8; i++)
        colors[n++] = rb_heart_index(rb_parse_heart_color(heart_colors[i]));
    if (n == 0) colors[n++] = 0;   /* no heart_colours -> Heart00 (pink) */

    if (!(operation && (!strcmp(operation, "set") || !strcmp(operation, "decrease"))))
        return 0;                    /* Rust: Err("Unknown operation") */

    r_log_effect(g, actor, "log_modify_required_hearts_standard", "%s", operation);

    int pl = rb_resolve_target_player(g, target);
    int n_mod = 0;
    for (int s = 0; s < RB_STAGE_SIZE; s++) {
        int cid = g->p[pl].stage[s];
        if (cid < 0) continue;
        if (!rb_card_is_member(cid)) continue;
        for (int i = 0; i < n; i++) {
            if (!strcmp(operation, "set"))
                rb_mods_set_need_heart(&g->mods, cid, colors[i], (uint8_t)value);
            else
                rb_mods_add_need_heart(&g->mods, cid, colors[i], -value);
            n_mod++;
        }
    }
    return n_mod;
}

/* required_hearts.rs:147 execute_modify_required_hearts_success */
int rb_rules_execute_modify_required_hearts_success(GameState *g, int actor,
                                                    const AbilityEffect *e)
{
    if (!g || !e) return 0;
    const char *operation = r_either(e, "operation", "effect_type");
    int value = r_value_or_count(e, 0);
    if (!operation) return 0;                       /* Rust: unknown -> return */
    if (strcmp(operation, "decrease") && strcmp(operation, "set") &&
        strcmp(operation, "add")) return 0;         /* Rust: unknown -> return */
    r_log_effect(g, actor, "log_modify_required_hearts_success", "%s", operation);
    int col = rb_heart_index(rb_parse_heart_color(rb_effect_heart_colors_any(e)));
    const char *ct = e->card_type_field;

    int n = 0;
    for (int pl = 0; pl < 2; pl++) {
        for (int li = 0; li < g->p[pl].live.n; li++) {
            int cid = g->p[pl].live.cards[li];
            if (ct && *ct && !strcmp(ct, "live_card") && !rb_card_is_live(cid)) continue;
            int cur[8];
            rb_effective_need_heart(g, cid, cur);
            int delta;
            if (!strcmp(operation, "decrease")) {
                if (cur[col] + (-value) < 0) continue;
                delta = -value;
                rb_mods_add_need_heart(&g->mods, cid, col, delta);
                r_record(g, e, RB_EFFECT_NEED_HEART_MOD, cid, delta);
            } else if (!strcmp(operation, "set")) {
                delta = value - cur[col];
                rb_mods_set_need_heart(&g->mods, cid, col, (uint8_t)value);
                r_record(g, e, RB_EFFECT_HEART_OVERRIDE, cid, delta);
            } else {
                delta = value;
                rb_mods_add_need_heart(&g->mods, cid, col, delta);
                r_record(g, e, RB_EFFECT_NEED_HEART_MOD, cid, delta);
            }
            if (delta == 0) continue;
            n++;
        }
    }
    return n;
}

/* required_hearts.rs:176 execute_specify_heart_color */
void rb_rules_execute_specify_heart_color(GameState *g, int actor,
                                          const AbilityEffect *e)
{
    RbChoice ch;
    memset(&ch, 0, sizeof(ch));
    ch.kind = RB_CHOICE_SELECT_TARGET;
    snprintf(ch.target, sizeof(ch.target), "heart_color");
    ch.count = 0;
    ch.allow_skip = 0;
    rb_choice_set_description(&ch, "Choose a heart color");
    rb_choice_set_bilingual_descriptions(&ch, "Choose a heart color",
                                         "ハートの色を選択");
    rb_queue_pause_for_choice(g, &ch);
    r_log_effect(g, actor, "log_specify_heart_color", "%s", e ? e->text : "");
}

/* required_hearts.rs:192 resolve_gain_heart_color */
int rb_rules_resolve_gain_heart_color(GameState *g, int count,
                                      const char *const *heart_colors, int n)
{
    if (!g) return 0;
    if (n == 0) return 0;
    if (count >= n) {
        /* Every listed colour is taken: the first one is the answer. */
        g->queue.selected_heart_color = rb_heart_index(rb_parse_heart_color(heart_colors[0]));
        return 1;
    }
    if (n == 1) {
        /* A single colour is not a question. */
        g->queue.selected_heart_color = rb_heart_index(rb_parse_heart_color(heart_colors[0]));
        return 1;
    }
    char uniq[8][24];
    int nu = 0;
    for (int i = 0; i < n; i++) {
        int dup = 0;
        for (int k = 0; k < nu; k++)
            if (!strcmp(uniq[k], heart_colors[i])) { dup = 1; break; }
        if (!dup && nu < 8) snprintf(uniq[nu++], 24, "%s", heart_colors[i]);
    }
    RbChoice ch;
    memset(&ch, 0, sizeof(ch));
    ch.kind = RB_CHOICE_SELECT_HEART_COLOR;
    ch.count = count;
    for (int i = 0; i < nu; i++) snprintf(ch.heart_options[i], 24, "%s", uniq[i]);
    ch.n_heart_options = nu;
    rb_choice_set_description(&ch, "Choose a heart color");
    rb_choice_set_bilingual_descriptions(&ch, "Choose a heart color",
                                         "ハートの色を選択");
    rb_queue_pause_for_choice(g, &ch);
    return 0;
}

/* required_hearts.rs:207 execute_choose_required_hearts */
int rb_rules_execute_choose_required_hearts(GameState *g, int actor,
                                            const AbilityEffect *e)
{
    RbChoice ch;
    memset(&ch, 0, sizeof(ch));
    ch.kind = RB_CHOICE_SELECT_TARGET;
    snprintf(ch.target, sizeof(ch.target), "choose_required_hearts");
    ch.count = 0;
    ch.allow_skip = 0;   /* Rust hard-codes allow_skip: false here. */
    rb_choice_set_description(&ch, "Choose required hearts");
    rb_choice_set_bilingual_descriptions(&ch, "Choose required hearts",
                                         "必要ハートを選択");
    rb_queue_pause_for_choice(g, &ch);
    r_log_effect(g, actor, "log_choose_required_hearts", "%s", e ? e->text : "");
    return 1;
}

/* ==========================================================================
   3. cost.rs
   ========================================================================== */

/* The id pool a cost clause applies to (cost.rs:135-153 / 31-43). */
static int r_cost_targets(const GameState *g, int pl, const AbilityEffect *e,
                          int is_hand_cost, int *out, int max)
{
    const RbPlayer *p = &g->p[pl];
    const char *ct = e->card_type_field;
    int n = 0;
    if (is_hand_cost) {
        memcpy(out, p->hand.cards, sizeof(p->hand.cards));
        n = p->hand.n;
    } else if (ct && !strcmp(ct, "live_card")) {
        memcpy(out, p->live.cards, sizeof(p->live.cards));
        n = p->live.n;
    } else if (ct && !strcmp(ct, "member_card")) {
        for (int s = 0; s < RB_STAGE_SIZE && n < max; s++)
            if (p->stage[s] >= 0) out[n++] = p->stage[s];
    } else if (ct && !strcmp(ct, "energy_card")) {
        memcpy(out, p->energy.cards, sizeof(p->energy.cards));
        n = p->energy.n;
    } else {
        memcpy(out, p->hand.cards, sizeof(p->hand.cards));
        n = p->hand.n;
    }
    return n > max ? max : n;
}

/* cost.rs:96 — filter by the group/character qualifiers. */
static int r_cost_filter(const AbilityEffect *e, int *ids, int n)
{
    if (!r_filter_applies(e)) return n;
    RbCardFilter cf;
    if (!rb_effect_filter_subset(e, &cf) || !cf.has_filter) return n;
    int out[RB_MAX_ZONE];
    int m = rb_matching_ids(&cf, ids, n, out, RB_MAX_ZONE);
    memcpy(ids, out, (size_t)m * sizeof(int));
    return m;
}

/* cost.rs:25 execute_set_cost */
void rb_rules_execute_set_cost(GameState *g, int actor, const AbilityEffect *e)
{
    if (!g || !e) return;
    int value = r_value_or_count(e, 0);
    int pl = r_player(actor, rb_effect_target_name(e));
    int ids[RB_MAX_ZONE];
    int n = r_cost_targets(g, pl, e, 0, ids, RB_MAX_ZONE);
    n = r_cost_filter(e, ids, n);
    r_log_effect(g, actor, "log_set_cost", "%s", "");
    for (int i = 0; i < n; i++) {
        if (ids[i] < 0) continue;
        rb_mods_set_cost(&g->mods, ids[i], (int16_t)value);
        r_record(g, e, RB_EFFECT_COST_SET, ids[i], value);
    }
}

/* cost.rs:82 execute_modify_cost */
void rb_rules_execute_modify_cost(GameState *g, int actor, const AbilityEffect *e)
{
    if (!g || !e) return;
    RbRulesState *st = rb_rules_state(g);
    const char *operation = r_either(e, "operation", "effect_type");
    if (!operation) operation = "add";
    const char *duration = r_extra(e, "duration");
    int value = r_value_or_count(e, 0);
    if (r_extra_bool(e, "per_unit", 0) || e->per_unit)
        value *= r_per_unit_units(g, actor, e, NULL);
    r_log_effect(g, actor, "log_modify_cost", "%s", operation);

    int pl = r_player(actor, rb_effect_target_name(e));
    const char *source = e->source;
    const char *location = r_extra(e, "location");
    int is_hand_cost = (source && !strcmp(source, "hand")) ||
                       (location && !strcmp(location, "hand")) ||
                       (location && !strcmp(location, "deck") &&
                        source && !strcmp(source, "hand"));
    int ids[RB_MAX_ZONE];
    int n = r_cost_targets(g, pl, e, is_hand_cost, ids, RB_MAX_ZONE);
    n = r_cost_filter(e, ids, n);
    if (r_self_target(e) && g->activating_card >= 0) {
        int m = 0;
        for (int i = 0; i < n; i++)
            if (ids[i] == g->activating_card) ids[m++] = ids[i];
        n = m;
    }

    if (!strcmp(operation, "set_from_reference")) {
        /* 「選んだメンバーが元々持つコストよりN低い/高い値」 (cost.rs:194-272) */
        int ref_id = -1;
        if (st->n_selected_cards > 0)      ref_id = st->selected_cards[st->n_selected_cards - 1];
        else if (st->n_moved_cards > 0)    ref_id = st->moved_cards[st->n_moved_cards - 1];
        else if (g->n_recently_moved > 0)  ref_id = g->recently_moved[g->n_recently_moved - 1];
        if (ref_id < 0) return;   /* Rust: no reference -> return */
        int ref_cost = 0;
        Card c;
        if (rb_decode_card_by_index((uint32_t)ref_id, &c)) { ref_cost = c.cost; rb_free_card(&c); }
        int resolved = (int)rb_saturate_u8(ref_cost + r_extra_int(e, "cost_offset", 0));
        int deltas[RB_MAX_ZONE];
        for (int i = 0; i < n; i++) {
            int printed = 0;
            if (rb_decode_card_by_index((uint32_t)ids[i], &c)) { printed = c.cost; rb_free_card(&c); }
            deltas[i] = (int16_t)(resolved - printed);
        }
        for (int i = 0; i < n; i++) {
            if (ids[i] < 0) continue;
            rb_mods_add_cost(&g->mods, ids[i], deltas[i]);
            r_record(g, e, RB_EFFECT_COST_BONUS, ids[i], deltas[i]);
        }
        if (duration && strcmp(duration, "permanent"))
            rb_util_push_temporary_effect(g, "modify_cost", duration,
                                          rb_effect_target_name(e),
                                          "Cost set_from_reference");
        return;
    }

    int delta;
    if      (!strcmp(operation, "add"))       delta =  (int16_t)value;
    else if (!strcmp(operation, "subtract")) delta = -(int16_t)value;
    else if (!strcmp(operation, "set"))      delta =  (int16_t)value;
    else return;                              /* Rust: unknown operation -> return */

    for (int i = 0; i < n; i++) {
        if (ids[i] < 0) continue;
        if (!strcmp(operation, "set")) rb_mods_set_cost(&g->mods, ids[i], delta);
        else                         rb_mods_add_cost(&g->mods, ids[i], delta);
        r_record(g, e, !strcmp(operation, "set") ? RB_EFFECT_COST_SET
                                                  : RB_EFFECT_COST_BONUS,
                 ids[i], delta);
    }
    if (duration && strcmp(duration, "permanent")) {
        char desc[96];
        snprintf(desc, sizeof(desc), "Cost %s %d (%s)", operation, value, duration);
        rb_util_push_temporary_effect(g, "modify_cost", duration,
                                      rb_effect_target_name(e), desc);
    }
}

/* cost.rs:328 execute_activation_cost */
void rb_rules_execute_activation_cost(GameState *g, int actor, const AbilityEffect *e)
{
    if (!g || !e) return;
    const char *operation = r_either(e, "operation", "effect_type");
    if (!operation) operation = "increase";
    int value = r_value_or_count(e, 0);
    const char *target = rb_effect_target_name(e);
    r_log_effect(g, actor, "log_activation_cost", "%s", operation);
    char prohibition[64];
    snprintf(prohibition, sizeof(prohibition), "activation_cost_%s_%d", operation, value);
    if (!strcmp(target, "self") || !strcmp(target, "opponent"))
        r_prohibition(g, prohibition);
    /* Rust pushes the temporary effect for EVERY target, not only self/opponent. */
    char desc[96];
    snprintf(desc, sizeof(desc), "Modify activation cost by %s %d", operation, value);
    rb_util_push_temporary_effect(g, prohibition, r_extra(e, "duration"), target, desc);
}

/* cost.rs:361 execute_set_cost_to_use */
int rb_rules_execute_set_cost_to_use(GameState *g, int actor, const AbilityEffect *e)
{
    if (!g || !e) return 0;
    int value = r_value_or_count(e, 0);
    int card_id = (g->queue.resume_host >= 0) ? g->queue.resume_host : g->activating_card;
    if (card_id >= 0) rb_mods_set_cost(&g->mods, card_id, (int16_t)value);
    r_log_effect(g, actor, "log_set_cost_to_use", "%s", "使用コスト設定");
    return 1;
}

/* ==========================================================================
   4. target.rs
   ========================================================================== */

/* target.rs:33 position_limited_targets */
int rb_rules_position_limited_targets(const RbPlayer *player, const char *position,
                                      const int *targets, int n, int *out, int max)
{
    int idx = position ? rb_stage_position_index(position) : -1;
    if (idx < 0 || idx >= RB_STAGE_SIZE) {
        int m = n < max ? n : max;
        for (int i = 0; i < m; i++) out[i] = targets[i];
        return m;
    }
    int expected = player->stage[idx];
    int m = 0;
    for (int i = 0; i < n && m < max; i++)
        if (targets[i] == expected) out[m++] = targets[i];
    return m;
}

/* target.rs:54 same_name_targets */
int rb_rules_same_name_targets(const int *moved, int n_moved,
                               const int *targets, int n, int *out, int max)
{
    char ref_names[RB_RULES_MOVED_CAP][64];
    int n_ref = 0;
    for (int i = 0; i < n_moved && n_ref < RB_RULES_MOVED_CAP; i++) {
        if (moved[i] < 0) continue;
        Card c;
        if (!rb_decode_card_by_index((uint32_t)moved[i], &c)) continue;
        snprintf(ref_names[n_ref++], 64, "%s", c.name ? c.name : "");
        rb_free_card(&c);
    }
    /* No moved card means nothing can match, so the list empties. */
    if (n_ref == 0) return 0;
    int m = 0;
    for (int i = 0; i < n && m < max; i++) {
        if (targets[i] < 0) continue;
        Card c;
        if (!rb_decode_card_by_index((uint32_t)targets[i], &c)) continue;
        int hit = 0;
        for (int k = 0; k < n_ref; k++)
            if (!strcmp(ref_names[k], c.name ? c.name : "")) { hit = 1; break; }
        rb_free_card(&c);
        if (hit) out[m++] = targets[i];
    }
    return m;
}

/* target.rs:156 is_broadly_self */
int rb_rules_is_broadly_self(const AbilityEffect *e, int is_self_target,
                             int activating_card_id)
{
    if (!e) return 0;
    if (!is_self_target) return 0;
    if (strcmp(rb_effect_target_name(e), "self") != 0) return 0;
    if (activating_card_id < 0) return 0;
    if (e->source && *e->source) return 0;
    if (*e->card_type_field) return 0;
    if (r_extra_bool(e, "target_from_selection", 0)) return 0;
    if (r_extra_bool(e, "multiple_targets", 0)) return 0;
    if (r_has_groups(e)) return 0;
    if (r_extra_bool(e, "all", 0)) return 0;
    return 1;
}

/* target.rs:180 implicit_heart_target */
int rb_rules_implicit_heart_target(GameState *g, const AbilityEffect *e,
                                   int is_self_target, int activating_card_id)
{
    if (!g || !e) return -1;
    const char *pos = r_position(e);
    if (pos && *pos) {
        int stage_idx = rb_stage_position_index(pos);
        if (stage_idx < 0) return -1;
        RbPlayer *p = rb_resolve_target_player_mut(g, rb_effect_target_name(e));
        if (!p) return -1;
        int card_id = p->stage[stage_idx];
        return (card_id != RB_EMPTY_SLOT) ? card_id : -1;
    }
    int unqualified_self = (r_extra(e, "target_count") == NULL) &&
                           ((r_extra(e, "exclude_self") == NULL) ||
                            !strcmp(rb_effect_target_name(e), "self"));
    if (!(unqualified_self ||
          rb_rules_is_broadly_self(e, is_self_target, activating_card_id)))
        return -1;
    return activating_card_id;
}

/* target.rs:210 try_create_target_selection_choice
   Returns 1 when a choice was created (the caller must stop), 0 otherwise.
   `kind_other` selects the ResourceKind::Other early-out. */
int rb_rules_try_create_target_selection_choice(GameState *g, int actor,
        const AbilityEffect *e, int kind_other, const char *resource, int count,
        const char *target, int is_self_target, int per_unit, int exclude_self_id,
        const int *preceding_moved, int n_preceding_moved,
        const int *all_selected, int n_all_selected,
        const int *selected_for_current, int n_selected_for_current)
{
    if (!g || !e) return 0;
    int tc = r_extra_int(e, "target_count", -1);
    if (tc < 0) return 0;
    if (is_self_target) return 0;
    if (per_unit) return 0;
    if (kind_other) return 0;                              /* ResourceKind::Other */
    if (!(n_selected_for_current == 0 || r_extra(e, "distinct") != NULL)) return 0;

    RbRulesState *st = rb_rules_state(g);
    int pl = r_player(actor, target);
    int stage_ids[RB_STAGE_SIZE];
    int n_stage = 0;
    for (int s = 0; s < RB_STAGE_SIZE; s++)
        if (g->p[pl].stage[s] >= 0) stage_ids[n_stage++] = g->p[pl].stage[s];

    RbCardFilter prelim;
    rb_effect_filter_subset(e, &prelim);
    prelim.exclude_self_id = exclude_self_id;
    prelim.has_exclude_self = exclude_self_id >= 0;

    int excl_by_moved = r_extra(e, "exclude_by_name_source") &&
                        !strcmp(r_extra(e, "exclude_by_name_source"), "preceding_moved") &&
                        n_preceding_moved > 0;
    int cands[RB_MAX_ZONE];
    int n_c = 0;
    int tmp[1];
    for (int i = 0; i < n_stage; i++) {
        int cid = stage_ids[i];
        if (cid < 0) continue;
        if (exclude_self_id >= 0 && cid == exclude_self_id) continue;
        if (prelim.has_filter) {
            tmp[0] = cid;
            if (rb_matching_ids(&prelim, tmp, 1, tmp, 1) != 1) continue;
        }
        if (excl_by_moved) {
            /* exclude_by_name_source == "preceding_moved" */
            int dropped = 0;
            for (int k = 0; k < n_preceding_moved && !dropped; k++) {
                if (preceding_moved[k] < 0) continue;
                char nm[64], ref[64];
                Card ca, cb;
                nm[0] = ref[0] = 0;
                if (rb_decode_card_by_index((uint32_t)cid, &ca)) {
                    snprintf(nm, sizeof(nm), "%s", ca.name ? ca.name : "");
                    rb_free_card(&ca);
                }
                if (rb_decode_card_by_index((uint32_t)preceding_moved[k], &cb)) {
                    snprintf(ref, sizeof(ref), "%s", cb.name ? cb.name : "");
                    rb_free_card(&cb);
                }
                if (nm[0] && !strcmp(nm, ref)) dropped = 1;
            }
            if (dropped) continue;
        }
        if (n_all_selected > 0) {
            int in_all = 0;
            for (int k = 0; k < n_all_selected; k++)
                if (all_selected[k] == cid) { in_all = 1; break; }
            if (in_all) continue;
        }
        if (n_c < RB_MAX_ZONE) cands[n_c++] = cid;
    }

    /* has_current_blade_filter */
    const char *bl = r_either(e, "current_blade_limit", "blade_limit");
    if (bl) {
        const char *op = r_either(e, "current_blade_operator", "blade_limit_operator");
        if (n_c > 0)
            n_c = rb_filter_current_blade(cands, n_c, g, atoi(bl), op, cands, RB_MAX_ZONE);
    }

    /* position */
    const char *pos = r_position(e);
    if (pos && *pos) {
        int idx = rb_stage_position_index(pos);
        if (idx >= 0) {
            int expected = g->p[pl].stage[idx];
            int m = 0;
            for (int i = 0; i < n_c; i++) if (cands[i] == expected) cands[m++] = cands[i];
            n_c = m;
        }
    }

    /* group_reference == "same_group_name" */
    if (r_extra(e, "group_reference") &&
        !strcmp(r_extra(e, "group_reference"), "same_group_name") &&
        st->n_moved_cards > 0 && st->moved_cards[0] >= 0) {
        const char *group = rb_card_group_name(st->moved_cards[0]);
        if (group && *group) {
            int m = 0;
            for (int i = 0; i < n_c; i++)
                if (rb_card_matches_group_str(cands[i], group)) cands[m++] = cands[i];
            n_c = m;
        }
    }

    /* same_name */
    if (r_extra_bool(e, "same_name", 0)) {
        if (st->n_moved_cards == 0) {
            n_c = 0;   /* ref_names empty -> candidates clear */
        } else {
            int keep[RB_MAX_ZONE];
            int n_k = rb_rules_same_name_targets(st->moved_cards, st->n_moved_cards,
                                                 cands, n_c, keep, RB_MAX_ZONE);
            memcpy(cands, keep, (size_t)n_k * sizeof(int));
            n_c = n_k;
        }
    }

    if (n_c <= tc) return 0;

    int filtered[RB_MAX_ZONE];
    int n_f = 0;
    for (int i = 0; i < n_c; i++)
        for (int s = 0; s < RB_STAGE_SIZE; s++)
            if (g->p[pl].stage[s] == cands[i]) { filtered[n_f++] = s; break; }

    AbilityEffect *saved = rb_effect_deep_clone(e);
    if (saved) {
        for (int i = 0; i < saved->n_extra; i++) {
            if (saved->extra_k[i] && !strcmp(saved->extra_k[i], "target_count")) {
                free(saved->extra_v[i]);
                saved->extra_v[i] = NULL;
                break;
            }
        }
        st->count_at_save = st->n_selected_cards;
        r_push_pending_front(g, saved);
        rb_effect_free(saved);
    }

    RbChoice ch;
    memset(&ch, 0, sizeof(ch));
    ch.kind = RB_CHOICE_SELECT_CARD;
    snprintf(ch.zone, sizeof(ch.zone), "stage");
    ch.count = tc;
    ch.allow_skip = 0;
    if (*e->card_type_field)
        snprintf(ch.card_type, sizeof(ch.card_type), "%s", e->card_type_field);
    if (r_extra(e, "group"))
        snprintf(ch.filter_group, sizeof(ch.filter_group), "%s", r_extra(e, "group"));
    ch.n_characters = r_split(r_extra(e, "characters"), &ch.characters[0][0], 32, 16);
    for (int i = 0; i < n_f && i < RB_MAX_ZONE; i++) ch.filtered_indices[i] = filtered[i];
    ch.n_filtered_indices = n_f;
    snprintf(ch.target_player_id, sizeof(ch.target_player_id), "%s", target);
    char desc_en[128];
    snprintf(desc_en, sizeof(desc_en), "Select %d card(s) to receive %d %s",
             tc, count, resource ? resource : "");
    rb_choice_set_description(&ch, desc_en);
    rb_choice_set_bilingual_descriptions(&ch, desc_en,
        "リソースを受け取る枚のカードを選択");
    rb_queue_pause_for_choice(g, &ch);
    st->sub_choice_created = 1;
    return 1;
}

/* target.rs:81 handle_both_targets — the recognition half; the existing
   `rb_misc_handle_both_targets` in misc.c runs the self-then-opponent halves. */
int rb_rules_handle_both_targets(GameState *g, int actor, const AbilityEffect *e)
{
    (void)g; (void)actor;
    if (!e) return 0;
    if (!e->target || strcmp(e->target, "both")) return 0;
    if (e->action && !strcmp(e->action, "position_change")) return 0;
    int is_mcb = e->action && !strcmp(e->action, "move_cards") &&
                 r_extra_bool(e, "multiple_targets", 0) &&
                 e->target && !strcmp(e->target, "deck");
    int primary_is_mcb = e->primary_effect && e->primary_effect->action &&
                         !strcmp(e->primary_effect->action, "move_cards") &&
                         r_extra_bool(e->primary_effect, "multiple_targets", 0) &&
                         e->primary_effect->target &&
                         !strcmp(e->primary_effect->target, "deck");
    if (is_mcb || primary_is_mcb) return 0;
    return 1;
}

/* ==========================================================================
   5. selection.rs
   ========================================================================== */

/* selection.rs:90 execute_select_heart_color */
void rb_rules_execute_select_heart_color(GameState *g, int count,
                                         const char *const *heart_colors, int n,
                                         const char *target)
{
    (void)target;
    if (!g) return;
    char uniq[8][24];
    int nu = 0;
    for (int i = 0; i < n; i++) {
        int dup = 0;
        for (int k = 0; k < nu; k++) if (!strcmp(uniq[k], heart_colors[i])) { dup = 1; break; }
        if (!dup && nu < 8) snprintf(uniq[nu++], 24, "%s", heart_colors[i]);
    }
    if (nu == 1) {
        /* One colour is a decision already made: record it, prompt for nothing. */
        RbRulesState *st = rb_rules_state(g);
        snprintf(st->conditional_choice, sizeof(st->conditional_choice), "%s", uniq[0]);
        st->has_conditional_choice = 1;
        g->queue.selected_heart_color = rb_heart_index(rb_parse_heart_color(uniq[0]));
        return;
    }
    RbChoice ch;
    memset(&ch, 0, sizeof(ch));
    ch.kind = RB_CHOICE_SELECT_HEART_COLOR;
    ch.count = count;
    for (int i = 0; i < nu; i++) snprintf(ch.heart_options[i], 24, "%s", uniq[i]);
    ch.n_heart_options = nu;
    rb_choice_set_description(&ch, "Choose a heart color");
    rb_choice_set_bilingual_descriptions(&ch, "Choose a heart color",
                                         "ハートの色を選択");
    rb_queue_pause_for_choice(g, &ch);
}

/* selection.rs:294 make_hand_selection_choice */
static void r_make_hand_selection_choice(GameState *g, int actor,
                                         const char *player_target, int count,
                                         const AbilityEffect *e, RbChoice *out)
{
    (void)e;
    int pl = r_player(actor, player_target);
    int pick = count < g->p[pl].hand.n ? count : g->p[pl].hand.n;
    memset(out, 0, sizeof(*out));
    out->kind = RB_CHOICE_SELECT_CARD;
    snprintf(out->zone, sizeof(out->zone), "hand");
    out->count = pick;
    out->allow_skip = 1;   /* 「まで」 means up to N */
    snprintf(out->description, sizeof(out->description),
             "Select up to %d card(s) to keep", count);
    snprintf(out->description_ja, sizeof(out->description_ja),
             "最大%d枚まで手札に残すカードを選択", count);
    snprintf(out->target_player_id, sizeof(out->target_player_id), "%s", player_target);
}

/* selection.rs:320 move_non_selected_hand_to_deck_bottom */
void rb_rules_move_non_selected_hand_to_deck_bottom(GameState *g, int actor,
                                                    const char *player_target,
                                                    const int *hand_snapshot,
                                                    int n_snapshot)
{
    if (!g || !hand_snapshot) return;
    RbRulesState *st = rb_rules_state(g);
    RbPlayer *p = &g->p[r_player(actor, player_target)];
    int to_move[RB_MAX_ZONE];
    int n_move = 0;
    for (int i = 0; i < n_snapshot; i++) {
        int kept = 0;
        for (int k = 0; k < st->n_keep_shuffle_selected; k++)
            if (st->keep_shuffle_selected[k] == i) { kept = 1; break; }
        if (kept) continue;
        for (int j = 0; j < p->hand.n; j++)
            if (p->hand.cards[j] == hand_snapshot[i]) { p->hand.n--; break; }
        if (n_move < RB_MAX_ZONE) to_move[n_move++] = hand_snapshot[i];
    }
    /* 「シャッフルし、自身のデッキの下に置く」 */
    rb_shuffle(to_move, n_move);
    for (int i = 0; i < n_move; i++)
        if (p->deck.n < RB_MAX_ZONE) p->deck.cards[p->deck.n++] = to_move[i];
}

/* selection.rs:244 execute_both_hand_keep_shuffle_under
   Phase 0 self prompt -> phase 1 opponent prompt -> phase 2 done. */
void rb_rules_execute_both_hand_keep_shuffle_under(GameState *g, int actor,
                                                   const AbilityEffect *e)
{
    if (!g || !e) return;
    RbRulesState *st = rb_rules_state(g);
    int count = r_count_or(e, 1);
    if (st->keep_shuffle_phase == 0) {
        st->keep_shuffle_count = count;
        int pl = r_player(actor, "self");
        st->n_keep_shuffle_snapshot[0] = g->p[pl].hand.n;
        memcpy(st->keep_shuffle_snapshot[0], g->p[pl].hand.cards,
               sizeof(g->p[pl].hand.cards));
        st->n_keep_shuffle_selected = 0;
        RbChoice ch;
        r_make_hand_selection_choice(g, actor, "self", count, e, &ch);
        st->keep_shuffle_phase = 1;
        rb_queue_pause_for_choice(g, &ch);
        return;
    }
    if (st->keep_shuffle_phase == 1) {
        /* Phase 1 is normally advanced by the choice handler, which moves self's
           non-selected cards and prompts the opponent. This fallback only runs
           if that did not happen (direct re-entry). */
        if (st->n_keep_shuffle_snapshot[0] > 0 &&
            st->n_keep_shuffle_snapshot[1] == 0) {
            rb_rules_move_non_selected_hand_to_deck_bottom(g, actor, "self",
                st->keep_shuffle_snapshot[0], st->n_keep_shuffle_snapshot[0]);
            st->n_keep_shuffle_selected = 0;
            int pl = r_player(actor, "opponent");
            st->n_keep_shuffle_snapshot[1] = g->p[pl].hand.n;
            memcpy(st->keep_shuffle_snapshot[1], g->p[pl].hand.cards,
                   sizeof(g->p[pl].hand.cards));
            RbChoice ch;
            r_make_hand_selection_choice(g, actor, "opponent", count, e, &ch);
            st->keep_shuffle_phase = 2;
            rb_queue_pause_for_choice(g, &ch);
        }
        return;
    }
    /* phase 2: the opponent's selection resolved */
    if (st->n_keep_shuffle_snapshot[1] > 0)
        rb_rules_move_non_selected_hand_to_deck_bottom(g, actor, "opponent",
            st->keep_shuffle_snapshot[1], st->n_keep_shuffle_snapshot[1]);
    st->keep_shuffle_phase = 0;
    st->n_keep_shuffle_snapshot[0] = 0;
    st->n_keep_shuffle_snapshot[1] = 0;
    st->n_keep_shuffle_selected = 0;
}

/* selection.rs:118 execute_select_number */
void rb_rules_execute_select_number(GameState *g, int actor, const AbilityEffect *e)
{
    if (!g) return;
    uint32_t n_cards = rb_num_cards();
    int max_cost = 10;
    for (uint32_t i = 0; i < n_cards; i++) {
        Card c;
        if (!rb_decode_card_by_index(i, &c)) continue;
        if (c.cost > max_cost) max_cost = c.cost;
        rb_free_card(&c);
    }
    char options[24][8];
    int n_opt = 0;
    for (int n = 1; n <= max_cost && n_opt < 23; n++)
        snprintf(options[n_opt++], 8, "%d", n);
    snprintf(options[n_opt++], 8, "67");

    char display[128], display_ja[128];
    if (n_opt > 10) {
        snprintf(display, sizeof(display), "%s~%s", options[0], options[n_opt - 1]);
        snprintf(display_ja, sizeof(display_ja), "%s?%s", options[0], options[n_opt - 1]);
    } else {
        display[0] = 0;
        for (int i = 0; i < n_opt; i++) {
            if (i) strncat(display, ", ", sizeof(display) - strlen(display) - 1);
            strncat(display, options[i], sizeof(display) - strlen(display) - 1);
        }
        snprintf(display_ja, sizeof(display_ja), "%s", display);
    }

    RbQueueEntry *en = rb_queue_current_entry_mut(g);
    if (en) { en->choice_card_no = -1; en->has_choice_route_raw = 0; }

    RbChoice ch;
    memset(&ch, 0, sizeof(ch));
    ch.kind = RB_CHOICE_SELECT_TARGET;
    snprintf(ch.target, sizeof(ch.target), "choice_string");
    ch.allow_skip = e ? e->is_optional : 0;
    char en_desc[160];
    snprintf(en_desc, sizeof(en_desc), "Choose a number: %s", display);
    rb_choice_set_description(&ch, en_desc);
    char ja_desc[160];
    snprintf(ja_desc, sizeof(ja_desc), "数値を選択: %s", display_ja);
    rb_choice_set_bilingual_descriptions(&ch, en_desc, ja_desc);
    rb_queue_pause_for_choice(g, &ch);
    r_log_effect(g, actor, "log_number_select", "%s", "");
}

/* selection.rs:174 execute_area_select */
void rb_rules_execute_area_select(GameState *g, int actor, const AbilityEffect *e)
{
    if (!g || !e) return;
    int pl = r_player(actor, rb_effect_target_name(e));
    int activating_id = (g->queue.resume_host >= 0) ? g->queue.resume_host
                                                    : g->activating_card;
    static const char *POS[3] = { "left", "center", "right" };
    const char *valid[3];
    int n = 0;
    for (int i = 0; i < 3; i++) {
        if (activating_id >= 0 && g->p[pl].stage[i] == activating_id) continue;
        valid[n++] = POS[i];
    }
    if (n == 0) return;   /* Rust: Ok(()) with no choice */
    RbChoice ch;
    memset(&ch, 0, sizeof(ch));
    ch.kind = RB_CHOICE_SELECT_TARGET;
    snprintf(ch.target, sizeof(ch.target), "area_select");
    ch.allow_skip = e->is_optional;
    char opts[64] = {0};
    for (int i = 0; i < n; i++) {
        if (i) strncat(opts, ",", sizeof(opts) - strlen(opts) - 1);
        strncat(opts, valid[i], sizeof(opts) - strlen(opts) - 1);
    }
    char en_desc[128], ja_desc[128];
    snprintf(en_desc, sizeof(en_desc), "Choose an area [%s]", opts);
    snprintf(ja_desc, sizeof(ja_desc), "エリアを選択 [%s]", opts);
    rb_choice_set_bilingual_descriptions(&ch, en_desc, ja_desc);
    rb_queue_pause_for_choice(g, &ch);
}

/* selection.rs:211 execute_choose_target_player
   Rust also stores the effect in `self.owner.executing`, which is what the
   follow-up step reads to know the prompt's context. */
void rb_rules_execute_choose_target_player(GameState *g, int actor,
                                           const AbilityEffect *e)
{
    if (!g || !e) return;
    RbRulesState *st = rb_rules_state(g);
    if (st->owner_executing) { rb_effect_free(st->owner_executing); st->owner_executing = NULL; }
    st->owner_executing = rb_effect_deep_clone(e);

    char opts[16][64];
    int n = 0;
    const char *src = r_extra(e, "choice_options");
    if (src) {
        char tmp[16][64];
        n = r_split(src, &tmp[0][0], 64, 16);
        for (int i = 0; i < n; i++) snprintf(opts[i], 64, "%s", tmp[i]);
    } else {
        snprintf(opts[0], 64, "%s", "自分");
        snprintf(opts[1], 64, "%s", "相手");
        n = 2;
    }
    char opt_list[160] = {0};
    for (int i = 0; i < n; i++) {
        if (i) strncat(opt_list, ", ", sizeof(opt_list) - strlen(opt_list) - 1);
        strncat(opt_list, opts[i], sizeof(opt_list) - strlen(opt_list) - 1);
    }
    RbChoice ch;
    memset(&ch, 0, sizeof(ch));
    ch.kind = RB_CHOICE_SELECT_TARGET;
    snprintf(ch.target, sizeof(ch.target), "self_or_opponent");
    ch.allow_skip = 0;   /* Rust hard-codes false */
    char en_desc[160], ja_desc[160];
    snprintf(en_desc, sizeof(en_desc), "Choose self or opponent [%s]", opt_list);
    snprintf(ja_desc, sizeof(ja_desc), "自分または相手を選択 [%s]", opt_list);
    rb_choice_set_bilingual_descriptions(&ch, en_desc, ja_desc);
    rb_queue_pause_for_choice(g, &ch);
    r_log_effect(g, actor, "log_choose_target_player", "%s", "ターゲットプレイヤー選択");
}

/* selection.rs:27 execute_select_effect */
void rb_rules_execute_select_effect(GameState *g, int actor, const AbilityEffect *e)
{
    if (!g || !e) return;
    char hcs[8][24];
    int n_hcs = r_heart_colors(e, hcs);
    const char *hcs_list[8];
    for (int i = 0; i < n_hcs; i++) hcs_list[i] = hcs[i];
    int has_heart_colors = n_hcs > 0;

    if ((!e->source || !*e->source) && !has_heart_colors &&
        !r_extra(e, "or_card_types") && !r_has_characters(e) && !r_has_groups(e)) {
        rb_rules_execute_area_select(g, actor, e);
        return;
    }
    if ((!e->source || !*e->source) &&
        (!*e->card_type_field) && has_heart_colors) {
        rb_rules_execute_select_heart_color(g, r_count_or(e, 1),
                                            (const char *const *)hcs_list, n_hcs,
                                            rb_effect_target_name(e));
        return;
    }
    if (r_extra_bool(e, "keep_shuffle_under", 0)) {
        rb_rules_execute_both_hand_keep_shuffle_under(g, actor, e);
        return;
    }
    /* Falls through to execute_select with the resolved source. */
    const char *source = (*e->card_type_field && !strcmp(e->card_type_field, "member_card") &&
                          (!e->source || !*e->source))
                             ? "stage"
                             : rb_effect_source_or(e, "hand");
    (void)source;
    rb_effect_select(g, actor, (AbilityEffect *)e);
}

/* ==========================================================================
   6. choice.rs
   ========================================================================== */

/* choice.rs:231 per_card_choice_repeats */
int rb_rules_per_card_choice_repeats(GameState *g, int actor, const AbilityEffect *e)
{
    if (!g || !e) return 0;
    if (!r_extra_bool(e, "per_unit", 0) && !e->per_unit) return 0;
    int pl = r_player(actor, rb_effect_target_name(e));
    const char *zone = r_either(e, "location", "per_unit_type");
    RbCardFilter cf;
    rb_effect_filter_subset(e, &cf);
    int qualifying = rb_resolve_per_unit_count(g, pl, zone, cf.card_type,
                                               cf.has_group ? cf.group : NULL,
                                               r_extra(e, "state"),
                                               g->activating_card);
    int per = e->per_unit_count > 0 ? e->per_unit_count : 1;
    if (per < 1) per = 1;
    int offerings = qualifying / per;
    if (offerings > 0) offerings -= 1;   /* the pending prompt is the first */
    return offerings > 0 ? offerings : 0;
}

/* choice.rs:266 make_card_effect_data */
void rb_rules_make_card_effect_data(RbEffectData *out, int card_id, int amount,
                                    const char *color)
{
    if (!out) return;
    memset(out, 0, sizeof(*out));
    out->kind = RB_EFFECT_DATA_SINGLE_CARD;
    out->value.single_card.card_id = (int16_t)card_id;
    out->value.single_card.amount  = (int16_t)amount;
    out->value.single_card.color   = (color && *color) ? rb_strdup2(color) : NULL;
}

/* choice.rs:27 execute_choice */
int rb_rules_execute_choice(GameState *g, int actor, const AbilityEffect *e)
{
    if (!g || !e) return 0;
    RbRulesState *st = rb_rules_state(g);
    const char *choice_options = r_extra(e, "choice_options");
    const char *choice_type    = r_extra(e, "choice_type");
    const char *choice_maker   = r_extra(e, "choice_maker");
    int n_options = 0;
    const AbilityEffect *options[RB_MAX_CHILD];
    for (int i = 0; i < e->n_options && i < RB_MAX_CHILD; i++)
        options[n_options++] = e->options[i];

    char hcs[8][24];
    int n_hcs = r_heart_colors(e, hcs);
    const char *hcs_list[8];
    for (int i = 0; i < n_hcs; i++) hcs_list[i] = hcs[i];

    /* choice.rs:41-59 — a bare heart-colour list is a colour picker. */
    if (n_hcs > 0 && n_options == 0 && !choice_options && !choice_type) {
        rb_rules_execute_select_heart_color(g, r_count_or(e, 1),
                                            (const char *const *)hcs_list, n_hcs,
                                            rb_effect_target_name(e));
        return 1;
    }

    /* choice.rs:62-83 — a selection was already made: execute that option. */
    if (n_options > 0 && st->has_conditional_choice) {
        char *end = NULL;
        long idx = strtol(st->conditional_choice, &end, 10);
        if (end && *end == 0 && idx >= 0 && idx < n_options) {
            rb_execute_effect_ex(g, actor, (AbilityEffect *)options[idx],
                                 g->activating_card);
            return 1;
        }
    }
    if (choice_options && st->has_conditional_choice) {
        char tmp[32][64];
        int n_alt = r_split(choice_options, &tmp[0][0], 64, 32);
        for (int i = 0; i < n_alt; i++)
            if (!strcmp(tmp[i], st->conditional_choice)) return 1;  /* informational */
    }

    /* choice.rs:95-126 — propagate the parent's group_names to its options.
       The discriminator is that the per-card reading ("…のカードN枚につき、")
       carries につき in the text; a shared filter does not. */
    int options_have_own_groups = 0;
    for (int i = 0; i < n_options; i++)
        if (options[i] && r_has_groups(options[i])) { options_have_own_groups = 1; break; }
    int should_propagate_group = r_has_groups(e) &&
                                 r_extra(e, "alternative_count_type") == NULL &&
                                 e->alternative_condition == NULL &&
                                 !options_have_own_groups &&
                                 !(e->text && strstr(e->text, "につき"));
    AbilityEffect *propagated[RB_MAX_CHILD];
    int n_prop = 0;
    if (should_propagate_group) {
        for (int i = 0; i < n_options; i++) {
            propagated[i] = rb_effect_deep_clone(options[i]);
            if (propagated[i] && !r_has_groups(propagated[i])) {
                for (int k = 0; k < propagated[i]->n_extra; k++) {
                    if (propagated[i]->extra_k[k] &&
                        !strcmp(propagated[i]->extra_k[k], "group_names")) {
                        free(propagated[i]->extra_v[k]);
                        propagated[i]->extra_v[k] = rb_strdup2(r_extra(e, "group_names"));
                        break;
                    }
                }
            }
            n_prop++;
        }
    }

    /* choice.rs:132-154 — stamp the entry. */
    RbQueueEntry *en = rb_queue_current_entry_mut(g);
    /* The propagated option bodies are parked on the queue: the C model's
       stand-in for `entry.conditional_choice = Effects(..)`. */
    for (int i = 0; i < g->queue.choice_options_n; i++)
        if (g->queue.choice_options[i]) {
            rb_effect_free(g->queue.choice_options[i]);
            g->queue.choice_options[i] = NULL;
        }
    g->queue.choice_options_n = 0;
    g->queue.choice_reprompt_pending = 0;
    if (n_prop > 0) {
        int cap = RB_ENTRY_PENDING_CAP < n_prop ? RB_ENTRY_PENDING_CAP : n_prop;
        for (int i = 0; i < cap; i++) g->queue.choice_options[i] = propagated[i];
        g->queue.choice_options_n = cap;
        g->queue.choice_reprompt_pending = 1;
    } else {
        for (int i = 0; i < n_prop; i++)
            if (propagated[i]) rb_effect_free(propagated[i]);
    }
    if (en) {
        en->choice_card_no = (n_options > 0 || !choice_options) ? RB_STK_CHOICE
                                                                : RB_STK_CHOICE_STRING;
        st->has_conditional_choice = 0;
        st->conditional_choice[0] = 0;
        if (choice_maker && !strcmp(choice_maker, "opponent"))
            snprintf(en->choice_player_id, sizeof(en->choice_player_id), "%s",
                     en->player_id[0] == '1' ? "player2" : "player1");
        else
            snprintf(en->choice_player_id, sizeof(en->choice_player_id), "%s",
                     en->player_id);
    } else if (n_prop > 0) {
        for (int i = 0; i < n_prop; i++)
            if (propagated[i]) rb_effect_free(propagated[i]);
    }

    /* choice.rs:155-221 — build the question. */
    if (n_options > 0) {
        char desc[128] = {0};
        for (int i = 0; i < n_options; i++) {
            const char *ans = r_extra(options[i], "answers");
            char one[64];
            if (ans && *ans) snprintf(one, sizeof(one), "%s", ans);
            else if (options[i]->text) snprintf(one, sizeof(one), "%s", options[i]->text);
            else one[0] = 0;
            if (i) strncat(desc, " / ", sizeof(desc) - strlen(desc) - 1);
            strncat(desc, one, sizeof(desc) - strlen(desc) - 1);
        }
        RbChoice ch;
        memset(&ch, 0, sizeof(ch));
        ch.kind = RB_CHOICE_SELECT_TARGET;
        snprintf(ch.target, sizeof(ch.target), "choice");
        ch.allow_skip = e->is_optional;
        char ja[160];
        snprintf(ja, sizeof(ja), "選択: %s", desc);
        rb_choice_set_bilingual_descriptions(&ch, desc, ja);
        rb_queue_pause_for_choice(g, &ch);
        st->choice_repeats_remaining = rb_rules_per_card_choice_repeats(g, actor, e);
    } else if (choice_options) {
        char tmp[32][64];
        int n_alt = r_split(choice_options, &tmp[0][0], 64, 32);
        char joined[160] = {0};
        for (int i = 0; i < n_alt; i++) {
            if (i) strncat(joined, ", ", sizeof(joined) - strlen(joined) - 1);
            strncat(joined, tmp[i], sizeof(joined) - strlen(joined) - 1);
        }
        RbChoice ch;
        memset(&ch, 0, sizeof(ch));
        ch.kind = RB_CHOICE_SELECT_TARGET;
        snprintf(ch.target, sizeof(ch.target), "choice_string");
        ch.allow_skip = e->is_optional;
        char en_desc[192], ja_desc[192];
        snprintf(en_desc, sizeof(en_desc), "Choose one: %s", joined);
        snprintf(ja_desc, sizeof(ja_desc), "選択: %s", joined);
        rb_choice_set_bilingual_descriptions(&ch, en_desc, ja_desc);
        rb_queue_pause_for_choice(g, &ch);
    } else if (choice_type) {
        RbChoice ch;
        memset(&ch, 0, sizeof(ch));
        ch.kind = RB_CHOICE_SELECT_TARGET;
        snprintf(ch.target, sizeof(ch.target), "choice");
        ch.allow_skip = e->is_optional;
        char en_desc[160], ja_desc[160];
        snprintf(en_desc, sizeof(en_desc), "Choose: %s", choice_type);
        snprintf(ja_desc, sizeof(ja_desc), "選択: %s", choice_type);
        rb_choice_set_bilingual_descriptions(&ch, en_desc, ja_desc);
        rb_queue_pause_for_choice(g, &ch);
    }
    return 1;
}

/* ==========================================================================
   7. restriction.rs
   ========================================================================== */

/* restriction.rs:26 execute_restriction */
int rb_rules_execute_restriction(GameState *g, int actor, const AbilityEffect *e)
{
    if (!g || !e) return 0;
    RbRulesState *st = rb_rules_state(g);
    const char *restriction_type = r_either(e, "restriction_type", "effect_type");
    const char *restricted_dest  = r_either(e, "restricted_destination", "destination");
    const char *target = rb_effect_target_name(e);
    int delayed = r_extra_bool(e, "delayed", 0);

    r_log_effect(g, actor, "log_restriction", "%s",
                 restriction_type ? restriction_type : "");

    char entry[96];
    snprintf(entry, sizeof(entry), "restriction:%s:%s",
             restriction_type ? restriction_type : "unknown",
             restricted_dest ? restricted_dest : "");
    if (delayed) {
        /* Rust keeps a SEPARATE `delayed_prohibition_effects` list, so a
           delayed prohibition is NOT consulted by the prohibition checks. */
        if (st->n_delayed_prohibition < RB_RULES_PROHIB_CAP)
            snprintf(st->delayed_prohibition[st->n_delayed_prohibition++],
                     sizeof(st->delayed_prohibition[0]), "%s", entry);
    } else {
        r_prohibition(g, entry);
    }

    if (restriction_type &&
        (!strcmp(restriction_type, "cannot_activate_by_effect") ||
         !strcmp(restriction_type, "cannot_active"))) {
        if (delayed) {
            /* Q280: the restriction's victims are the cards THIS ability just
               moved, not the card that owns the ability. changed_state_members
               takes precedence: it was populated most recently by this
               ability's own steps and no move-based flow touches it. */
            if (st->n_changed_state_members > 0) {
                for (int i = 0; i < st->n_changed_state_members; i++) {
                    int cid = st->changed_state_members[i];
                    if (cid >= 0) rb_mods_add_delayed_cannot_active(&g->mods, cid, 1);
                }
                st->n_changed_state_members = 0;
            } else if (g->n_recently_moved > 0) {
                for (int i = 0; i < g->n_recently_moved; i++) {
                    int cid = g->recently_moved[i];
                    if (cid >= 0) rb_mods_add_delayed_cannot_active(&g->mods, cid, 1);
                }
            } else if (g->activating_card >= 0) {
                rb_mods_add_delayed_cannot_active(&g->mods, g->activating_card, 1);
            }
        } else {
            int pl = r_player(actor, target);
            if (pl >= 0 && pl <= 1) {
                st->cannot_activate_members[pl] = 1;
                g->player_cannot_activate[pl] = 1;
            }
        }
    }

    if (restriction_type && !strcmp(restriction_type, "cannot_live")) {
        int pl = r_player(actor, target);
        if (pl >= 0 && pl <= 1) st->cannot_live_players[pl] = 1;
    }

    if (restriction_type && !strcmp(restriction_type, "cannot_wait_by_effect")) {
        /* 「…は相手の効果によってはウェイトしない」 — the members matching the
           filter on the CONTROLLER'S OWN stage are immune to wait effects from
           the opponent. The owner is resolved from the ability controller, not
           the parsed (often misleading) target. */
        int owner_pl = 0;
        if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
            owner_pl = (g->queue.entries[g->queue.cur].player_id[0] == '2') ? 1 : 0;
        char groups[8][64];
        int n_groups = r_split(r_extra(e, "group_names"), &groups[0][0], 64, 8);
        const char *ct = e->card_type_field;
        int blade_limit = r_extra(e, "blade_limit") ? atoi(r_extra(e, "blade_limit"))
                                                    : 255;
        const char *blade_op = r_either(e, "blade_limit_operator", "operator");
        int original_blade = r_extra_bool(e, "original_value", 0);

        for (int s = 0; s < RB_STAGE_SIZE; s++) {
            int cid = g->p[owner_pl].stage[s];
            if (cid < 0) continue;
            int base_blade = 0;
            Card c;
            if (rb_decode_card_by_index((uint32_t)cid, &c)) { base_blade = c.blade; rb_free_card(&c); }
            if (ct && *ct && !rb_card_matches_type(cid, ct)) continue;
            if (n_groups > 0) {
                int hit = 0;
                for (int k = 0; k < n_groups; k++)
                    if (rb_card_matches_group_str(cid, groups[k])) { hit = 1; break; }
                if (!hit) continue;
            }
            /* i32: a negative blade modifier is legal here and must not floor. */
            int current_blade = base_blade + rb_mods_get_blade(&g->mods, cid);
            int blade = original_blade ? base_blade : current_blade;
            if (!rb_compare_counts(blade_op, (int)rb_saturate_u8(blade), blade_limit))
                continue;
            int dup = 0;
            for (int i = 0; i < st->n_wait_immune; i++)
                if (st->wait_immune[i].member_id == cid) { dup = 1; break; }
            if (!dup && st->n_wait_immune < RB_RULES_LIST_CAP) {
                st->wait_immune[st->n_wait_immune].member_id = cid;
                st->wait_immune[st->n_wait_immune].owner_pl = owner_pl;
                st->n_wait_immune++;
            }
        }
    }
    return 1;
}

/* restriction.rs:198 execute_activation_restriction
   Rust pushes ONE marker and does nothing else. It must NOT run the full
   restriction body (the existing C h_activation_restriction does, which wrongly
   sets cannot_activate). */
void rb_rules_execute_activation_restriction(GameState *g, int actor,
                                             const AbilityEffect *e)
{
    if (!g || !e) return;
    const char *target = rb_effect_target_name(e);
    r_log_effect(g, actor, "log_activation_restriction", "target=%s", target);
    char entry[96];
    snprintf(entry, sizeof(entry), "activation_restriction:%s", target);
    r_prohibition(g, entry);
}

/* restriction.rs:217 execute_reduce_live_card_set_limit */
void rb_rules_execute_reduce_live_card_set_limit(GameState *g, int actor,
                                                 const AbilityEffect *e)
{
    if (!g || !e) return;
    int count = r_count_or(e, 1);
    r_log_effect(g, actor, "log_reduce_live_set_limit", "n=%d", count);
    /* Rust resolves `target_player_mut("self")` — the ability controller. */
    int pl = r_player(actor, "self");
    if (pl >= 0 && pl <= 1) g->live_set_limit_reduction[pl] += count;
}

/* restriction.rs:236 execute_modify_limit */
int rb_rules_execute_modify_limit(GameState *g, int actor, const AbilityEffect *e)
{
    if (!g || !e) return 0;
    const char *operation = r_either(e, "operation", "effect_type");
    if (!operation) operation = "decrease";
    int count = r_count_or(e, 0);
    r_log_effect(g, actor, "log_stage_limit", "op=%s,n=%d", operation, count);
    char entry[64];
    if (!strcmp(operation, "decrease")) {
        snprintf(entry, sizeof(entry), "limit_decrease:%d", count);
        r_prohibition(g, entry);
    } else if (!strcmp(operation, "increase")) {
        snprintf(entry, sizeof(entry), "limit_increase:%d", count);
        r_prohibition(g, entry);
    }
    return 1;
}

/* ==========================================================================
   8. reyell.rs
   ========================================================================== */

/* Rust `Player::refresh()` for the yell path (mirror live.c rb_yell_refresh_deck). */
static int r_refresh_deck(GameState *g, int pl)
{
    RbPlayer *p = &g->p[pl];
    if (p->deck.n > 0 || p->discard.n == 0) return 0;
    rb_shuffle(p->discard.cards, p->discard.n);
    int count = p->discard.n;
    if (count > RB_MAX_ZONE - p->deck.n) count = RB_MAX_ZONE - p->deck.n;
    for (int i = 0; i < count; i++) p->deck.cards[p->deck.n++] = p->discard.cards[i];
    p->discard.n -= count;
    p->deck_refreshed_this_turn = 1;
    return count > 0;
}

/* Rust `live.rs::process_yell_revealed_card_icons` used to be re-implemented here
   as `r_yell_icons` (this file's own typedef RbRulesYellIcons plus a 35-line
   copy of the decode) with the note "live.c owns a `static` copy; this local
   mirror keeps rules.c self-contained until live.c exports it (CROSS-FILE
   REQUEST 4)". live.c answered that request: the function is EXPORTED as
   rb_process_yell_revealed_card_icons and declared in include/rabuka.h:3485, so
   both copies are gone and this file calls the canonical one directly.
   Behaviour is identical: the canonical body is the same decode, it accumulates
   into the caller's `total_hearts` and `cheer_count` exactly as r_yell_icons
   did, and the flat out-params carry the per-card tallies the old struct
   returned. */

/* reyell.rs:27 execute_re_yell */
void rb_rules_execute_re_yell(GameState *g, int actor, const AbilityEffect *e)
{
    if (!g || !e) return;
    if (r_extra_bool(e, "lose_blade_hearts", 0)) {
        int pl = r_player(actor, rb_effect_target_name(e));
        int cids[RB_STAGE_SIZE];
        int n = 0;
        for (int s = 0; s < RB_STAGE_SIZE; s++)
            if (g->p[pl].stage[s] >= 0) cids[n++] = g->p[pl].stage[s];
        for (int i = 0; i < n; i++) rb_mods_clear_card(&g->mods, cids[i]);
    }
    /* Clear the old yell cards so perform_yell's new cards replace them.
       initial_yell_revealed_cards was already saved in the phase code. */
    g->n_revealed = 0;
    g->yell_occurred = 1;
    r_prohibition(g, "re_yell");
    r_log_effect(g, actor, "log_re_yell", "%s", "");
}

/* reyell.rs:69 execute_perform_yell
   The two-step re-yell: count -> repeat draw-and-reveal -> harvest the revealed
   cards' icons through the SAME shared helper as the primary yell -> execute the
   draw icons -> stash the rebuild for the phase code to apply. */
void rb_rules_execute_perform_yell(GameState *g, int actor, const AbilityEffect *e)
{
    if (!g || !e) return;
    RbRulesState *st = rb_rules_state(g);
    int count;
    if (r_extra_bool(e, "per_unit", 0) || e->per_unit) {
        /* per_unit with per_unit_source == "previous_moved_cards": sum the costs
           of the cards moved by the preceding action, divide, cap. */
        int total_cost = 0;
        for (int i = 0; i < st->n_moved_cards; i++) {
            if (st->moved_cards[i] < 0) continue;
            Card c;
            if (rb_decode_card_by_index((uint32_t)st->moved_cards[i], &c)) {
                total_cost += c.cost;
                rb_free_card(&c);
            }
        }
        int divisor = e->per_unit_count > 0 ? e->per_unit_count : 1;
        if (divisor < 1) divisor = 1;
        count = total_cost / divisor;
        int cap = rb_effect_repeat_limit_any(e);
        if (cap > 0 && count > cap) count = cap;
    } else if (r_extra(e, "dynamic_count")) {
        count = rb_effect_count(g, actor, g->activating_card, e, 0);
    } else {
        count = r_count_or(e, 1);
    }

    int pl = r_player(actor, rb_effect_target_name(e));

    /* set_blade_type recolouring for the performing player, identical to the
       primary yell path in player_perform_live. */
    int override_color = -1;
    for (int s = 0; s < RB_STAGE_SIZE; s++) {
        int cid = g->p[pl].stage[s];
        if (cid < 0) continue;
        int bt = rb_mods_get_blade_type(&g->mods, cid);
        if (bt >= 0) { override_color = rb_blade_color_to_heart(bt); break; }
    }

    int all_drawn[RB_MAX_ZONE * 2];
    int n_all = 0;
    for (int k = 0; k < count; k++) {
        int tb = 0;
        for (int s = 0; s < RB_STAGE_SIZE; s++) {
            int cid = g->p[pl].stage[s];
            if (cid < 0) continue;
            tb += rb_effective_blade(cid, g->mods.blade[cid]);
        }
        RbPlayer *p = &g->p[pl];
        for (int i = 0; i < tb; i++) {
            /* Q104 / Rule 10.2.1: refresh from the waitroom mid-draw. */
            if (p->deck.n == 0 && p->discard.n > 0) r_refresh_deck(g, pl);
            if (p->deck.n == 0) break;
            int cid = p->deck.cards[0];
            for (int j = 1; j < p->deck.n; j++) p->deck.cards[j - 1] = p->deck.cards[j];
            p->deck.n--;
            if (g->n_revealed < RB_MAX_REVEALED_CARDS)
                g->revealed_cards[g->n_revealed++] = cid;
            if (n_all < (int)(sizeof(all_drawn) / sizeof(all_drawn[0])))
                all_drawn[n_all++] = cid;
        }
    }

    /* Rule 8.3.12 / 8.3.15.1.1: process the re-yelled cards' blade-heart and
       special-heart icons through the same shared helper as the primary yell
       (live.rs:112 process_yell_revealed_card_icons, the exported live.c twin),
       then execute their draws immediately (rule 8.3.12.1). The tallies are
       stashed for the phase code, which applies them before its success check. */
    int total_hearts[8];
    memset(total_hearts, 0, sizeof(total_hearts));
    int cheer = 0, draw_total = 0;
    for (int i = 0; i < n_all; i++) {
        int card_blade_hearts[8], note_icons = 0, draw_icons = 0;
        rb_process_yell_revealed_card_icons(g, all_drawn[i], override_color,
                                            total_hearts, &cheer,
                                            card_blade_hearts, &note_icons,
                                            &draw_icons);
        draw_total += draw_icons;
    }
    for (int i = 0; i < draw_total; i++) rb_draw(g, pl);

    for (int i = 0; i < 8; i++) g->re_yell_blade_hearts[i] = total_hearts[i];
    g->re_yell_note_icons = cheer;
    g->re_yell_owner = pl;
    g->re_yell_pending = 1;

    r_log_effect(g, actor, "log_yell_execute", "n=%d", count);
}

/* reyell.rs:217 execute_modify_yell_count */
void rb_rules_execute_modify_yell_count(GameState *g, int actor, const AbilityEffect *e)
{
    if (!g || !e) return;
    const char *operation = r_either(e, "operation", "effect_type");
    if (!operation) operation = "subtract";
    int count = r_count_or(e, 0);
    r_log_effect(g, actor, "log_yell_count", "op=%s,n=%d", operation, count);
    char pp[16];
    r_player_prefix(g, actor, pp, sizeof(pp));
    int slot = (strchr(pp, '2') != NULL) ? 2 : 1;
    if (!strcmp(operation, "add"))            rb_add_yell_count_modifier(g, (uint8_t)slot, count);
    else if (!strcmp(operation, "subtract"))  rb_add_yell_count_modifier(g, (uint8_t)slot, -count);
    /* anything else: Rust only logs and moves on */
}

/* ==========================================================================
   9. replacement.rs
   ========================================================================== */

/* replacement.rs:112 register_replacement_effect */
int rb_rules_register_replacement_effect(GameState *g, const AbilityEffect *e)
{
    if (!g || !e) return 0;
    const char *effect_type = r_extra(e, "effect_type");
    if (!effect_type || strcmp(effect_type, "replacement")) return 0;
    const char *original_event = r_extra(e, "replaces_event");
    int is_choice_based = r_extra_bool(e, "choice_based", 0);
    int card_id = g->activating_card >= 0 ? g->activating_card : -1;
    /* FirstAttackerNormal -> p1, everything else -> p2. */
    int player_id = (g->phase == RB_PHASE_MAIN) ? 0 : 1;
    if (original_event) {
        rb_add_replacement_effect(g, card_id, player_id, original_event, e, 1,
                                  is_choice_based);
        /* The C RbReplacementEffect carries no body; park the clone so
           run_replacement_effects can execute it. */
        RbRulesState *st = rb_rules_state(g);
        if (st->n_replacement_body < RB_RULES_LIST_CAP) {
            st->replacement_body[st->n_replacement_body].body =
                rb_effect_deep_clone(e);
            st->n_replacement_body++;
        }
    }
    return 1;   /* a "replacement" effect_type is always consumed */
}

/* replacement.rs:30 run_replacement_effects
   Returns 1 when the event was replaced, -1 when a choice-based replacement
   pends a prompt (the caller must stop, mirroring Rust's `Err`). */
int rb_rules_run_replacement_effects(GameState *g, int actor, const AbilityEffect *e)
{
    if (!g || !e || !e->action) return 0;
    RbRulesState *st = rb_rules_state(g);
    if (st->resolving_replacement) return 0;
    st->resolving_replacement = 1;
    rb_reset_replacement_effect_flags(g);

    int idxs[RB_RULES_LIST_CAP];
    int is_choice[RB_RULES_LIST_CAP];
    int n_idx = 0;
    for (int i = 0; i < g->n_replacement_effects && n_idx < RB_RULES_LIST_CAP; i++) {
        RbReplacementEffect *r = &g->replacement_effects[i];
        if (!r->original_event[0]) continue;
        if (strcmp(r->original_event, e->action)) continue;
        if (r->is_choice_based) { idxs[n_idx] = i; is_choice[n_idx] = 1; n_idx++; continue; }
        if (r->applied_this_event) continue;
        idxs[n_idx] = i; is_choice[n_idx] = 0; n_idx++;
    }
    if (n_idx == 0) { st->resolving_replacement = 0; return 0; }

    int mandatory[RB_RULES_LIST_CAP];
    int n_mand = 0;
    for (int k = 0; k < n_idx; k++) {
        int idx = idxs[k];
        if (is_choice[k]) {
            /* Run the mandatory bodies first, then ask. */
            for (int m = 0; m < n_mand; m++) {
                AbilityEffect *b = st->replacement_body[mandatory[m]].body;
                if (b) rb_execute_effect_ex(g, actor, b, g->activating_card);
            }
            st->replacement_index = idx;
            if (st->replacement_source) rb_effect_free(st->replacement_source);
            st->replacement_source = rb_effect_deep_clone(e);
            RbChoice ch;
            memset(&ch, 0, sizeof(ch));
            ch.kind = RB_CHOICE_SELECT_TARGET;
            snprintf(ch.target, sizeof(ch.target), "apply_replacement");
            ch.allow_skip = 0;
            char en_desc[192], ja_desc[192];
            snprintf(en_desc, sizeof(en_desc),
                     "Apply replacement effect for action '%s'?", e->action);
            snprintf(ja_desc, sizeof(ja_desc),
                     "アクション「%s」の置き換え効果を適用？", e->action);
            rb_choice_set_bilingual_descriptions(&ch, en_desc, ja_desc);
            rb_queue_pause_for_choice(g, &ch);
            return -1;   /* pending choice: the caller must stop */
        }
        r_mark_replacement_applied(g, idx);
        if (n_mand < RB_RULES_LIST_CAP) mandatory[n_mand++] = idx;
    }
    for (int m = 0; m < n_mand; m++) {
        if (mandatory[m] >= st->n_replacement_body) continue;
        AbilityEffect *b = st->replacement_body[mandatory[m]].body;
        if (b) rb_execute_effect_ex(g, actor, b, g->activating_card);
    }
    st->resolving_replacement = 0;
    return 1;
}

/* ==========================================================================
   10. Accessors for the side tables (the owning files read these)
   ========================================================================== */

/* restriction.rs wait immunity — the Q275 「…は相手の効果によってはウェイトしない」
   check. `waiter_pl` is the player performing the wait effect. */
int rb_rules_is_wait_immune(GameState *g, int member_cid, int waiter_pl)
{
    if (!g || member_cid < 0 || waiter_pl < 0 || waiter_pl > 1) return 0;
    RbRulesState *st = rb_rules_state(g);
    for (int i = 0; i < st->n_wait_immune; i++) {
        if (st->wait_immune[i].member_id != member_cid) continue;
        if (st->wait_immune[i].owner_pl == waiter_pl) continue;  /* own member */
        return 1;                                               /* opponent's */
    }
    return 0;
}

int rb_rules_cannot_live_player(GameState *g, int pl)
{
    if (!g || pl < 0 || pl > 1) return 0;
    return rb_rules_state(g)->cannot_live_players[pl];
}

int rb_rules_cannot_activate_player(GameState *g, int pl)
{
    if (!g || pl < 0 || pl > 1) return 0;
    return rb_rules_state(g)->cannot_activate_members[pl];
}

int rb_rules_n_delayed_prohibitions(GameState *g)
{
    return g ? rb_rules_state(g)->n_delayed_prohibition : 0;
}

const char *rb_rules_delayed_prohibition(GameState *g, int index)
{
    if (!g) return NULL;
    RbRulesState *st = rb_rules_state(g);
    if (index < 0 || index >= st->n_delayed_prohibition) return NULL;
    return st->delayed_prohibition[index];
}

/* selection.rs / choice.rs feeder hooks. */
void rb_rules_note_changed_state_members(GameState *g, const int *cids, int n)
{
    if (!g || !cids) return;
    RbRulesState *st = rb_rules_state(g);
    st->n_changed_state_members = 0;
    for (int i = 0; i < n && i < RB_RULES_MOVED_CAP; i++)
        st->changed_state_members[st->n_changed_state_members++] = cids[i];
}

void rb_rules_note_moved_cards(GameState *g, const int *cids, int n)
{
    if (!g || !cids) return;
    RbRulesState *st = rb_rules_state(g);
    st->n_moved_cards = 0;
    for (int i = 0; i < n && i < RB_RULES_MOVED_CAP; i++)
        st->moved_cards[st->n_moved_cards++] = cids[i];
}

void rb_rules_note_selected_cards(GameState *g, const int *cids, int n)
{
    if (!g || !cids) return;
    RbRulesState *st = rb_rules_state(g);
    st->n_selected_cards = 0;
    for (int i = 0; i < n && i < RB_RULES_MOVED_CAP; i++)
        st->selected_cards[st->n_selected_cards++] = cids[i];
}

void rb_rules_set_conditional_choice(GameState *g, const char *value)
{
    if (!g) return;
    RbRulesState *st = rb_rules_state(g);
    if (!value || !*value) { st->has_conditional_choice = 0; st->conditional_choice[0] = 0; return; }
    snprintf(st->conditional_choice, sizeof(st->conditional_choice), "%s", value);
    st->has_conditional_choice = 1;
}

const char *rb_rules_get_conditional_choice(GameState *g)
{
    if (!g) return NULL;
    RbRulesState *st = rb_rules_state(g);
    return st->has_conditional_choice ? st->conditional_choice : NULL;
}

int rb_rules_choice_repeats_remaining(GameState *g)
{
    return g ? rb_rules_state(g)->choice_repeats_remaining : -1;
}

int rb_rules_consume_choice_repeat(GameState *g)
{
    if (!g) return 0;
    RbRulesState *st = rb_rules_state(g);
    if (st->choice_repeats_remaining > 0) { st->choice_repeats_remaining--; return 1; }
    return 0;
}

void rb_rules_note_keep_shuffle_selected(GameState *g, const int *positions, int n)
{
    if (!g) return;
    RbRulesState *st = rb_rules_state(g);
    st->n_keep_shuffle_selected = 0;
    for (int i = 0; i < n && i < RB_RULES_MOVED_CAP; i++)
        st->keep_shuffle_selected[st->n_keep_shuffle_selected++] = positions[i];
}

int rb_rules_keep_shuffle_phase(GameState *g)
{ return g ? rb_rules_state(g)->keep_shuffle_phase : 0; }

int rb_rules_keep_shuffle_count(GameState *g)
{ return g ? rb_rules_state(g)->keep_shuffle_count : 0; }

int rb_rules_sub_choice_created(GameState *g)
{ return g ? rb_rules_state(g)->sub_choice_created : 0; }

int rb_rules_count_at_save(GameState *g)
{ return g ? rb_rules_state(g)->count_at_save : -1; }

const AbilityEffect *rb_rules_owner_executing(GameState *g)
{ return g ? rb_rules_state(g)->owner_executing : NULL; }

int rb_rules_resolving_replacement(GameState *g)
{ return g ? rb_rules_state(g)->resolving_replacement : 0; }

int rb_rules_take_pending_replacement(GameState *g, AbilityEffect **out_source)
{
    if (!g) return -1;
    RbRulesState *st = rb_rules_state(g);
    if (st->replacement_index < 0) return -1;
    if (out_source) *out_source = st->replacement_source;
    return st->replacement_index;
}

int rb_rules_apply_pending_replacement(GameState *g, int accept)
{
    if (!g) return 0;
    RbRulesState *st = rb_rules_state(g);
    if (st->replacement_index < 0) return 0;
    int idx = st->replacement_index;
    st->replacement_index = -1;
    if (accept) {
        r_mark_replacement_applied(g, idx);
        if (idx < st->n_replacement_body && st->replacement_body[idx].body)
            rb_execute_effect_ex(g, 0, st->replacement_body[idx].body,
                                 g->activating_card);
    }
    if (st->replacement_source) { rb_effect_free(st->replacement_source); st->replacement_source = NULL; }
    st->resolving_replacement = 0;
    return 1;
}
