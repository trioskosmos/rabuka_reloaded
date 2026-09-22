/* types.rs port — StepOutput, ValueRef, ZoneSnapshot, AbilityTraceNode,
   EffectPipeline, StepState, ChoiceResult, FullChoice, AbilityError,
   ExecutionContext/LookAndSelectStep helpers.
   Mirrors engine/src/ability/types.rs. */
#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

/* ── StepOutput (types.rs:892-924) ──────────────────────────────────── */

void rb_step_output_init(RbStepOutput *out) {
    memset(out, 0, sizeof(*out));
}

RbStepOutput *rb_step_output_from_value(int value) {
    RbStepOutput *o = (RbStepOutput *)malloc(sizeof(RbStepOutput));
    if (!o) return NULL;
    memset(o, 0, sizeof(*o));
    o->has_value = 1;
    o->value = value;
    return o;
}

void rb_step_output_merge(RbStepOutput *self, const RbStepOutput *other) {
    for (int i = 0; i < other->n_cards; i++)
        if (self->n_cards < RB_SO_MAX_CARDS)
            self->cards[self->n_cards++] = other->cards[i];
    if (other->has_value) {
        self->has_value = 1;
        self->value = other->value;
    }
    if (other->has_accepted) {
        self->has_accepted = 1;
        self->accepted = other->accepted;
    }
}

void rb_step_output_add_card(RbStepOutput *out, int card_id) {
    if (out->n_cards < RB_SO_MAX_CARDS)
        out->cards[out->n_cards++] = card_id;
}

int rb_step_output_has_cards(const RbStepOutput *out) {
    return out->n_cards > 0;
}

int rb_step_output_value(const RbStepOutput *out) {
    return out->has_value ? out->value : 0;
}

int rb_step_output_accepted(const RbStepOutput *out) {
    return out->has_accepted ? out->accepted : 0;
}

const char *rb_step_output_to_json(const RbStepOutput *out, char *buf, size_t buf_sz) {
    size_t used = 0;
    used += (size_t)snprintf(buf + used, buf_sz - used, "{\"cards\":[");
    for (int i = 0; i < out->n_cards && used < buf_sz; i++)
        used += (size_t)snprintf(buf + used, buf_sz - used, "%s%d",
                                 i ? "," : "", out->cards[i]);
    used += (size_t)snprintf(buf + used, buf_sz - used, "],\"value\":");
    if (out->has_value)
        used += (size_t)snprintf(buf + used, buf_sz - used, "%d", out->value);
    else
        used += (size_t)snprintf(buf + used, buf_sz - used, "null");
    used += (size_t)snprintf(buf + used, buf_sz - used, ",\"accepted\":");
    if (out->has_accepted)
        used += (size_t)snprintf(buf + used, buf_sz - used, "%s",
                                 out->accepted ? "true" : "false");
    else
        used += (size_t)snprintf(buf + used, buf_sz - used, "null");
    snprintf(buf + used, buf_sz - used, "}");
    return buf;
}

/* ── ValueRef (types.rs:934-970) ────────────────────────────────────── */

void rb_value_ref_init_literal(RbValueRef *ref, int value) {
    memset(ref, 0, sizeof(*ref));
    ref->kind = RB_VR_LITERAL;
    ref->literal_value = value;
}

void rb_value_ref_init_step(RbValueRef *ref, const char *step_id) {
    memset(ref, 0, sizeof(*ref));
    ref->kind = RB_VR_STEP_VALUE;
    strncpy(ref->step_id, step_id ? step_id : "", sizeof(ref->step_id) - 1);
}

void rb_value_ref_init_accepted(RbValueRef *ref, const char *step_id) {
    memset(ref, 0, sizeof(*ref));
    ref->kind = RB_VR_STEP_ACCEPTED;
    strncpy(ref->step_id, step_id ? step_id : "", sizeof(ref->step_id) - 1);
}

void rb_value_ref_init_offset(RbValueRef *ref, const char *step_id, int offset) {
    memset(ref, 0, sizeof(*ref));
    ref->kind = RB_VR_STEP_OFFSET;
    strncpy(ref->step_id, step_id ? step_id : "", sizeof(ref->step_id) - 1);
    ref->offset = offset;
}

const char *rb_value_ref_kind_to_str(RbValueRefKind k) {
    switch (k) {
    case RB_VR_LITERAL:       return "Literal";
    case RB_VR_STEP_VALUE:    return "StepValue";
    case RB_VR_STEP_ACCEPTED: return "StepAccepted";
    case RB_VR_STEP_OFFSET:   return "StepValueOffset";
    }
    return "Literal";
}

const char *rb_value_ref_kind_str(const RbValueRef *ref) {
    return rb_value_ref_kind_to_str(ref ? ref->kind : RB_VR_LITERAL);
}

int rb_value_ref_kind_from_str(const char *s, RbValueRefKind *out) {
    if (!s || !out) return 0;
    if (!strcmp(s, "Literal"))           { *out = RB_VR_LITERAL; return 1; }
    if (!strcmp(s, "StepValue"))         { *out = RB_VR_STEP_VALUE; return 1; }
    if (!strcmp(s, "StepAccepted"))      { *out = RB_VR_STEP_ACCEPTED; return 1; }
    if (!strcmp(s, "StepValueOffset"))   { *out = RB_VR_STEP_OFFSET; return 1; }
    return 0;
}

int rb_value_ref_resolve(const RbValueRef *ref,
                         int (*lookup)(const char *, int *, int *, void *),
                         void *lookup_ctx, int fallback) {
    if (!ref) return fallback;
    switch (ref->kind) {
    case RB_VR_LITERAL:
        return ref->literal_value;
    case RB_VR_STEP_VALUE:
    case RB_VR_STEP_ACCEPTED:
    case RB_VR_STEP_OFFSET: {
        int value = 0, accepted = 0;
        if (!lookup || !lookup(ref->step_id, &value, &accepted, lookup_ctx))
            return fallback;
        if (ref->kind == RB_VR_STEP_ACCEPTED) return accepted ? 1 : 0;
        if (ref->kind == RB_VR_STEP_OFFSET) return value + ref->offset;
        return value;
    }
    }
    return fallback;
}

void rb_value_ref_merge(RbValueRef *self, const RbValueRef *other) {
    (void)self; (void)other;
}

int rb_value_ref_is_literal(const RbValueRef *ref) {
    return ref && ref->kind == RB_VR_LITERAL;
}

/* ── ZoneSnapshot (types.rs:982-1006) ───────────────────────────────── */

RbZoneSnapshot rb_zone_snapshot_make(int hand, int stage, int waitroom,
                                     int energy, int active_energy, int deck) {
    RbZoneSnapshot s;
    s.hand_count = hand;
    s.stage_count = stage;
    s.waitroom_count = waitroom;
    s.energy_count = energy;
    s.active_energy_count = active_energy;
    s.deck_count = deck;
    return s;
}

RbZoneSnapshot rb_zone_snapshot_from_game_state(const GameState *g) {
    int hand = 0, stage = 0, wait = 0, energy = 0, active = 0, deck = 0;
    for (int pl = 0; pl < 2; pl++) {
        const RbPlayer *P = &g->p[pl];
        hand += P->hand.n;
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] != RB_EMPTY_SLOT) stage++;
        wait += P->discard.n;
        energy += P->energy.n;
        active += P->energy_active;
        deck += P->deck.n;
    }
    return rb_zone_snapshot_make(hand, stage, wait, energy, active, deck);
}

/* ── AbilityTraceNode (types.rs:1014-1043) ──────────────────────────── */

RbAbilityTraceNode *rb_trace_node_new(const char *label) {
    RbAbilityTraceNode *n = (RbAbilityTraceNode *)malloc(sizeof(RbAbilityTraceNode));
    if (!n) return NULL;
    memset(n, 0, sizeof(*n));
    strncpy(n->label, label ? label : "", sizeof(n->label) - 1);
    return n;
}

RbAbilityTraceNode *rb_trace_node_with_card(RbAbilityTraceNode *node, const char *card) {
    if (!node) return NULL;
    if (card) {
        strncpy(node->card, card, sizeof(node->card) - 1);
        node->has_card = 1;
    } else {
        node->card[0] = 0;
        node->has_card = 0;
    }
    return node;
}

RbAbilityTraceNode *rb_trace_node_with_before(RbAbilityTraceNode *node, RbZoneSnapshot before) {
    if (!node) return NULL;
    node->before = before;
    node->has_before = 1;
    return node;
}

RbAbilityTraceNode *rb_trace_node_with_after(RbAbilityTraceNode *node, RbZoneSnapshot after) {
    if (!node) return NULL;
    node->after = after;
    node->has_after = 1;
    return node;
}

RbAbilityTraceNode *rb_trace_node_from_game_state(const char *label, const GameState *g) {
    RbAbilityTraceNode *n = rb_trace_node_new(label);
    if (!n || !g) return n;
    RbZoneSnapshot snap = rb_zone_snapshot_from_game_state(g);
    rb_trace_node_with_before(n, snap);
    return n;
}

int rb_trace_node_add_child(RbAbilityTraceNode *parent, RbAbilityTraceNode *child) {
    if (!parent || !child) return 0;
    if (parent->n_children >= RB_TRACE_MAX_CHILDREN) return 0;
    parent->children[parent->n_children++] = child;
    return 1;
}

void rb_trace_node_free(RbAbilityTraceNode *node) {
    if (!node) return;
    for (int i = 0; i < node->n_children; i++)
        rb_trace_node_free(node->children[i]);
    free(node);
}

/* ── EffectPipeline (types.rs:1051-1067) ────────────────────────────── */

RbEffectPipeline *rb_effect_pipeline_new(void) {
    RbEffectPipeline *p = (RbEffectPipeline *)malloc(sizeof(RbEffectPipeline));
    if (!p) return NULL;
    p->trace = rb_trace_node_new("root");
    return p;
}

void rb_effect_pipeline_free(RbEffectPipeline *p) {
    if (!p) return;
    rb_trace_node_free(p->trace);
    free(p);
}

/* ── StepState (types.rs:1085-1123) ─────────────────────────────────── */

void rb_step_state_init(RbStepState *ss) {
    memset(ss, 0, sizeof(*ss));
}

RbStepState *rb_step_state_new(void) {
    RbStepState *ss = (RbStepState *)malloc(sizeof(RbStepState));
    if (!ss) return NULL;
    rb_step_state_init(ss);
    return ss;
}

static RbStepResultEntry *step_state_entry(RbStepState *ss, const char *id, int create) {
    for (int i = 0; i < ss->n_entries; i++)
        if (!strcmp(ss->entries[i].step_id, id)) return &ss->entries[i];
    if (!create || ss->n_entries >= RB_SS_MAX_RESULTS) return NULL;
    RbStepResultEntry *e = &ss->entries[ss->n_entries++];
    memset(e, 0, sizeof(*e));
    strncpy(e->step_id, id ? id : "", sizeof(e->step_id) - 1);
    return e;
}

void rb_step_state_record(RbStepState *ss, const char *effect_id, const RbStepOutput *output) {
    if (!effect_id || !effect_id[0] || !output) return;
    RbStepResultEntry *e = step_state_entry(ss, effect_id, 1);
    if (!e) return;
    rb_step_output_merge(&e->output, output);
    e->has_output = 1;
}

RbStepOutput rb_step_state_get(const RbStepState *ss, const char *step_id) {
    RbStepOutput empty;
    memset(&empty, 0, sizeof(empty));
    if (!ss || !step_id) return empty;
    for (int i = 0; i < ss->n_entries; i++)
        if (!strcmp(ss->entries[i].step_id, step_id))
            return ss->entries[i].output;
    return empty;
}

void rb_step_state_clear(RbStepState *ss) {
    ss->n_entries = 0;
    ss->last_draw_count = 0;
}

void rb_step_state_free(RbStepState *ss) {
    if (!ss) return;
    rb_step_state_clear(ss);
    free(ss);
}

int rb_step_state_record_value(RbStepState *ss, const char *effect_id, int value) {
    RbStepOutput o;
    memset(&o, 0, sizeof(o));
    o.has_value = 1;
    o.value = value;
    rb_step_state_record(ss, effect_id, &o);
    return value;
}

int rb_step_state_record_cards(RbStepState *ss, const char *effect_id,
                               const int *card_ids, int n) {
    RbStepOutput o;
    memset(&o, 0, sizeof(o));
    for (int i = 0; i < n; i++) rb_step_output_add_card(&o, card_ids[i]);
    rb_step_state_record(ss, effect_id, &o);
    return n;
}

/* ── AbilityError (types.rs:1128-1157) ──────────────────────────────── */

void rb_ability_error_format(int err, char *out, size_t out_sz,
                             int p1, int p2, int p3, const char *detail) {
    (void)p1; (void)p2; (void)p3;
    switch (err) {
    case RB_AE_NO_MEMBER_IN_TARGET_AREA:
        snprintf(out, out_sz, "Cannot baton touch - no member in target area"); return;
    case RB_AE_AREA_LOCKED:
        snprintf(out, out_sz, "Cannot baton touch: area is locked this turn"); return;
    case RB_AE_BATON_TOUCH_PROTECTION:
        snprintf(out, out_sz, "Cannot baton touch: member has baton touch discard protection"); return;
    case RB_AE_INSUFFICIENT_ENERGY:
        snprintf(out, out_sz, "Could not pay %d energy (only %d active energy available, %d total energy cards)",
                 p1, p2, p3); return;
    case RB_AE_INVALID_HAND_INDEX:
        snprintf(out, out_sz, "Invalid hand index"); return;
    case RB_AE_NOT_MEMBER_CARD:
        snprintf(out, out_sz, "Only member cards can be placed on stage"); return;
    case RB_AE_CARD_NOT_FOUND:
        snprintf(out, out_sz, "Card not found in database"); return;
    case RB_AE_ZONE_FULL:
        snprintf(out, out_sz, "Live card zone is full"); return;
    case RB_AE_CANNOT_PLACE:
    case RB_AE_GENERIC:
    case RB_AE_OTHER:
    default:
        snprintf(out, out_sz, "%s", detail ? detail : ""); return;
    }
}

/* ── ExecutionContext / LookAndSelectStep kind strings (types.rs) ───── */

const char *rb_exec_context_kind_to_str(RbExecutionContextKind k) {
    switch (k) {
    case RB_EC_NONE:                return "None";
    case RB_EC_SINGLE_EFFECT:       return "SingleEffect";
    case RB_EC_LOOK_AND_SELECT:     return "LookAndSelect";
    case RB_EC_MOVE_CARDS_POSITION: return "MoveCardsPosition";
    }
    return "None";
}

int rb_exec_context_kind_from_str(const char *s, RbExecutionContextKind *out) {
    if (!s || !out) return 0;
    if (!strcmp(s, "None"))              { *out = RB_EC_NONE; return 1; }
    if (!strcmp(s, "SingleEffect"))      { *out = RB_EC_SINGLE_EFFECT; return 1; }
    if (!strcmp(s, "LookAndSelect"))     { *out = RB_EC_LOOK_AND_SELECT; return 1; }
    if (!strcmp(s, "MoveCardsPosition")) { *out = RB_EC_MOVE_CARDS_POSITION; return 1; }
    return 0;
}

const char *rb_las_kind_to_str(RbLookAndSelectStepKind k) {
    switch (k) {
    case RB_LAS_LOOK:     return "Look";
    case RB_LAS_SELECT:   return "Select";
    case RB_LAS_FINALIZE: return "Finalize";
    }
    return "Look";
}

int rb_las_kind_from_str(const char *s, RbLookAndSelectStepKind *out) {
    if (!s || !out) return 0;
    if (!strcmp(s, "Look"))     { *out = RB_LAS_LOOK; return 1; }
    if (!strcmp(s, "Select"))   { *out = RB_LAS_SELECT; return 1; }
    if (!strcmp(s, "Finalize")) { *out = RB_LAS_FINALIZE; return 1; }
    return 0;
}

/* ── FullChoice (types.rs::Choice) ──────────────────────────────────── */

RbFullChoice *rb_full_choice_new_select_card(const char *zone, const char *description,
                                             int count, int allow_skip) {
    RbFullChoice *ch = (RbFullChoice *)malloc(sizeof(RbFullChoice));
    if (!ch) return NULL;
    memset(ch, 0, sizeof(*ch));
    ch->kind = RB_CC_SELECT_CARD;
    if (zone) strncpy(ch->zone, zone, sizeof(ch->zone) - 1);
    if (description) strncpy(ch->description, description, sizeof(ch->description) - 1);
    ch->count = count;
    ch->allow_skip = allow_skip;
    return ch;
}

RbFullChoice *rb_full_choice_new_select_target(const char *target, const char *description,
                                               int allow_skip) {
    RbFullChoice *ch = (RbFullChoice *)malloc(sizeof(RbFullChoice));
    if (!ch) return NULL;
    memset(ch, 0, sizeof(*ch));
    ch->kind = RB_CC_SELECT_TARGET;
    if (target) strncpy(ch->target, target, sizeof(ch->target) - 1);
    if (description) strncpy(ch->description, description, sizeof(ch->description) - 1);
    ch->allow_skip = allow_skip;
    return ch;
}

RbFullChoice *rb_full_choice_new_select_position(const char *position, const char *description,
                                                 int allow_skip) {
    RbFullChoice *ch = (RbFullChoice *)malloc(sizeof(RbFullChoice));
    if (!ch) return NULL;
    memset(ch, 0, sizeof(*ch));
    ch->kind = RB_CC_SELECT_POSITION;
    if (position) strncpy(ch->position, position, sizeof(ch->position) - 1);
    if (description) strncpy(ch->description, description, sizeof(ch->description) - 1);
    ch->allow_skip = allow_skip;
    return ch;
}

RbFullChoice *rb_full_choice_new_select_heart_color(int count, const char *const *options,
                                                    int n_options, const char *description) {
    RbFullChoice *ch = (RbFullChoice *)malloc(sizeof(RbFullChoice));
    if (!ch) return NULL;
    memset(ch, 0, sizeof(*ch));
    ch->kind = RB_CC_SELECT_HEART_COLOR;
    ch->count = count;
    rb_full_choice_set_hc_options(ch, options, n_options);
    if (description) strncpy(ch->description, description, sizeof(ch->description) - 1);
    return ch;
}

static void string_vec_set(RbFullChoiceStringVec *v, const char *const *opts, int n) {
    v->n_strings = 0;
    if (!opts) return;
    for (int i = 0; i < n && v->n_strings < RB_MAX_CC_STRINGS; i++) {
        strncpy(v->strings[v->n_strings], opts[i] ? opts[i] : "",
                sizeof(v->strings[0]) - 1);
        v->n_strings++;
    }
}

void rb_full_choice_set_description(RbFullChoice *ch, const char *desc) {
    if (!ch || !desc) return;
    strncpy(ch->description, desc, sizeof(ch->description) - 1);
}

void rb_full_choice_set_bilingual(RbFullChoice *ch, const char *en, const char *ja) {
    if (!ch) return;
    if (en) strncpy(ch->description_en, en, sizeof(ch->description_en) - 1);
    if (ja) strncpy(ch->description_ja, ja, sizeof(ch->description_ja) - 1);
}

void rb_full_choice_set_options(RbFullChoice *ch, const char *const *opts, int n) {
    if (ch) string_vec_set(&ch->options, opts, n);
}

void rb_full_choice_set_hc_options(RbFullChoice *ch, const char *const *opts, int n) {
    if (ch) string_vec_set(&ch->hc_options, opts, n);
}

void rb_full_choice_set_aa_options(RbFullChoice *ch, const char *const *opts, int n) {
    if (ch) string_vec_set(&ch->aa_options, opts, n);
}

void rb_full_choice_set_ls_options(RbFullChoice *ch, const char *const *opts, int n) {
    if (ch) string_vec_set(&ch->ls_options, opts, n);
}

const char *rb_full_choice_description_ja(const RbFullChoice *ch) {
    return (ch && ch->description_ja[0]) ? ch->description_ja : NULL;
}

int rb_full_choice_allow_skip(const RbFullChoice *ch) {
    return ch ? ch->allow_skip : 0;
}

void rb_full_choice_to_header(const RbFullChoice *src, RbChoice *dst) {
    if (!src || !dst) return;
    memset(dst, 0, sizeof(*dst));
    switch (src->kind) {
    case RB_CC_SELECT_CARD:        dst->kind = RB_CHOICE_SELECT_CARD; break;
    case RB_CC_SELECT_TARGET:      dst->kind = RB_CHOICE_SELECT_TARGET; break;
    case RB_CC_SELECT_POSITION:    dst->kind = RB_CHOICE_SELECT_POSITION; break;
    case RB_CC_SELECT_HEART_COLOR: dst->kind = RB_CHOICE_SELECT_HEART_COLOR; break;
    case RB_CC_SELECT_HEART_TYPE:  dst->kind = RB_CHOICE_SELECT_HEART_COLOR; break;
    case RB_CC_SELECT_AUTO_ABILITY:dst->kind = RB_CHOICE_SELECT_AUTO_ABILITY; break;
    case RB_CC_SELECT_LIVE_SUCCESS:dst->kind = RB_CHOICE_SELECT_AUTO_ABILITY; break;
    }
    strncpy(dst->zone, src->zone, sizeof(dst->zone) - 1);
    strncpy(dst->card_type, src->card_type, sizeof(dst->card_type) - 1);
    dst->count = src->count > 0 ? src->count : 1;
    dst->allow_skip = src->allow_skip;
    strncpy(dst->description, src->description, sizeof(dst->description) - 1);
    strncpy(dst->target, src->target_player_id[0] ? src->target_player_id : src->target,
            sizeof(dst->target) - 1);
}

const char *rb_full_choice_to_json(const RbFullChoice *ch, char *buf, size_t buf_sz) {
    if (!ch || !buf || buf_sz == 0) return buf;
    snprintf(buf, buf_sz, "{\"kind\":%d,\"zone\":\"%s\",\"count\":%d,\"allow_skip\":%s}",
             (int)ch->kind, ch->zone, ch->count, ch->allow_skip ? "true" : "false");
    return buf;
}

void rb_full_choice_free(RbFullChoice *ch) {
    free(ch);
}

/* ── ChoiceResult (types.rs::ChoiceResult) ──────────────────────────── */

RbChoiceResult *rb_choice_result_new_card_selected(const int *indices, int n) {
    RbChoiceResult *r = (RbChoiceResult *)malloc(sizeof(RbChoiceResult));
    if (!r) return NULL;
    memset(r, 0, sizeof(*r));
    r->kind = RB_CR_CARD_SELECTED;
    for (int i = 0; i < n && r->n_card_indices < RB_MAX_CC_CARDS; i++)
        r->card_indices[r->n_card_indices++] = indices[i];
    return r;
}

RbChoiceResult *rb_choice_result_new_skip(void) {
    RbChoiceResult *r = (RbChoiceResult *)malloc(sizeof(RbChoiceResult));
    if (!r) return NULL;
    memset(r, 0, sizeof(*r));
    r->kind = RB_CR_SKIP;
    return r;
}

RbChoiceResult *rb_choice_result_new_target(const char *target) {
    RbChoiceResult *r = (RbChoiceResult *)malloc(sizeof(RbChoiceResult));
    if (!r) return NULL;
    memset(r, 0, sizeof(*r));
    r->kind = RB_CR_TARGET_SELECTED;
    if (target) strncpy(r->target, target, sizeof(r->target) - 1);
    return r;
}

RbChoiceResultKind rb_choice_result_kind(const RbChoiceResult *r) {
    return r ? r->kind : RB_CR_SKIP;
}

int rb_choice_result_is_skip(const RbChoiceResult *r) {
    return r && r->kind == RB_CR_SKIP;
}

const char *rb_choice_result_kind_to_str(RbChoiceResultKind k) {
    switch (k) {
    case RB_CR_CARD_SELECTED:     return "CardSelected";
    case RB_CR_TARGET_SELECTED:   return "TargetSelected";
    case RB_CR_POSITION_SELECTED: return "PositionSelected";
    case RB_CR_HEART_COLOR:       return "HeartColor";
    case RB_CR_HEART_TYPE:        return "HeartType";
    case RB_CR_AUTO_ABILITY:      return "AutoAbility";
    case RB_CR_LIVE_SUCCESS:      return "LiveSuccess";
    case RB_CR_SKIP:              return "Skip";
    }
    return "Skip";
}

int rb_choice_result_kind_from_str(const char *s, RbChoiceResultKind *out) {
    if (!s || !out) return 0;
    if (!strcmp(s, "CardSelected"))     { *out = RB_CR_CARD_SELECTED; return 1; }
    if (!strcmp(s, "TargetSelected"))   { *out = RB_CR_TARGET_SELECTED; return 1; }
    if (!strcmp(s, "PositionSelected")) { *out = RB_CR_POSITION_SELECTED; return 1; }
    if (!strcmp(s, "HeartColor"))       { *out = RB_CR_HEART_COLOR; return 1; }
    if (!strcmp(s, "HeartType"))        { *out = RB_CR_HEART_TYPE; return 1; }
    if (!strcmp(s, "AutoAbility"))      { *out = RB_CR_AUTO_ABILITY; return 1; }
    if (!strcmp(s, "LiveSuccess"))      { *out = RB_CR_LIVE_SUCCESS; return 1; }
    if (!strcmp(s, "Skip"))             { *out = RB_CR_SKIP; return 1; }
    return 0;
}

void rb_choice_result_free(RbChoiceResult *r) {
    free(r);
}

/* ── ChoiceRoute (types.rs:58-80) ───────────────────────────────────── */

const char *RB_PAY_SKIP_TARGET = "pay_optional_cost:skip_optional_cost";

const char *rb_choice_route_to_str(RbChoiceRouteKind r) {
    switch (r) {
    case RB_ROUTEK_CHOICE:        return "choice";
    case RB_ROUTEK_CHOICE_STRING: return "choice_string";
    case RB_ROUTEK_CHOICE_COST:   return "choice_cost";
    case RB_ROUTEK_OPTIONAL_COST: return "optional_cost";
    case RB_ROUTEK_CHANGE_STATE:  return "change_state";
    case RB_ROUTEK_RAW:           return "raw";
    }
    return "choice";
}

int rb_choice_route_from_str(const char *s, RbChoiceRouteKind *out) {
    if (!s || !out) return 0;
    if (!strcmp(s, "choice"))        { *out = RB_ROUTEK_CHOICE; return 1; }
    if (!strcmp(s, "choice_string")) { *out = RB_ROUTEK_CHOICE_STRING; return 1; }
    if (!strcmp(s, "choice_cost"))   { *out = RB_ROUTEK_CHOICE_COST; return 1; }
    if (!strcmp(s, "optional_cost")) { *out = RB_ROUTEK_OPTIONAL_COST; return 1; }
    if (!strcmp(s, "change_state"))  { *out = RB_ROUTEK_CHANGE_STATE; return 1; }
    return 0;
}

RbChoiceRouteKind rb_choice_route_new(const char *s) {
    RbChoiceRouteKind k = RB_ROUTEK_CHOICE;
    if (rb_choice_route_from_str(s, &k)) return k;
    return RB_ROUTEK_RAW;
}

RbChoiceRoute rb_choice_route_from_kind(RbChoiceRouteKind k) {
    switch (k) {
    case RB_ROUTEK_CHOICE:        return RB_ROUTE_NONE;
    case RB_ROUTEK_CHOICE_COST:   return RB_ROUTE_CHOICE_COST;
    case RB_ROUTEK_OPTIONAL_COST: return RB_ROUTE_OPTIONAL_COST;
    case RB_ROUTEK_CHANGE_STATE:  return RB_ROUTE_CONDITIONAL_CHOICE;
    case RB_ROUTEK_CHOICE_STRING: return RB_ROUTE_CONDITIONAL_CHOICE;
    case RB_ROUTEK_RAW:           return RB_ROUTE_NONE;
    }
    return RB_ROUTE_NONE;
}

RbChoiceRouteKind rb_choice_route_kind_from_header(RbChoiceRoute r) {
    switch (r) {
    case RB_ROUTE_OPTIONAL_COST:       return RB_ROUTEK_OPTIONAL_COST;
    case RB_ROUTE_CHOICE_COST:         return RB_ROUTEK_CHOICE_COST;
    case RB_ROUTE_SELECT_CARDS:        return RB_ROUTEK_CHOICE;
    case RB_ROUTE_SELECT_TARGET:       return RB_ROUTEK_CHOICE;
    case RB_ROUTE_CONDITIONAL_CHOICE:  return RB_ROUTEK_CHOICE_STRING;
    case RB_ROUTE_NONE:                return RB_ROUTEK_CHOICE;
    }
    return RB_ROUTEK_CHOICE;
}

/* ── Repeat prompt (types.rs:39-48) ─────────────────────────────────── */

RbFullChoice *rb_repeat_prompt_choice(void) {
    RbFullChoice *ch = rb_full_choice_new_select_target(RB_PAY_SKIP_TARGET,
                                                        "Repeat effect?", 1);
    if (!ch) return NULL;
    rb_full_choice_set_options(ch, (const char *const[]){ "Stop", "Continue" }, 2);
    rb_full_choice_set_bilingual(ch, "Repeat effect?", "効果を繰り返しますか？");
    return ch;
}

/* ── TriggerEvent (types.rs:13-26) ──────────────────────────────────── */

void rb_trigger_event_init(RbTriggerEvent *e) {
    memset(e, 0, sizeof(*e));
}

void rb_trigger_event_add_moved(RbTriggerEvent *e, int card_id) {
    if (e->n_moved_cards < RB_TE_MAX_MOVED)
        e->moved_cards[e->n_moved_cards++] = card_id;
}

void rb_trigger_event_add_appeared(RbTriggerEvent *e, int card_id, const char *source_zone) {
    if (e->n_appeared_cards >= RB_TE_MAX_APPEARED) return;
    e->appeared_cards[e->n_appeared_cards].card_id = card_id;
    if (source_zone) {
        strncpy(e->appeared_cards[e->n_appeared_cards].source_zone, source_zone,
                RB_TE_ZONE_SZ - 1);
    }
    e->n_appeared_cards++;
}

int rb_trigger_event_has_moved(const RbTriggerEvent *e) {
    return e->n_moved_cards > 0;
}

int rb_trigger_event_has_appeared(const RbTriggerEvent *e) {
    return e->n_appeared_cards > 0;
}

int rb_trigger_event_has_position_change(const RbTriggerEvent *e) {
    return e->position_change_occurred;
}

int rb_trigger_event_has_energy_placed(const RbTriggerEvent *e) {
    return e->energy_placed_by_effect;
}

void rb_trigger_event_copy(RbTriggerEvent *dst, const RbTriggerEvent *src) {
    *dst = *src;
}

/* ── EffectSpawnContext (types.rs) ──────────────────────────────────── */

void rb_effect_spawn_context_init(RbEffectSpawnContext *ctx) {
    memset(ctx, 0, sizeof(*ctx));
}

/* ── ChoiceBuilder (types.rs::ChoiceBuilder → Choice) ───────────────── */

RbChoiceBuilder *rb_choice_builder_new(const char *zone, const char *description,
                                       int count, int allow_skip) {
    RbChoiceBuilder *b = (RbChoiceBuilder *)malloc(sizeof(RbChoiceBuilder));
    if (!b) return NULL;
    memset(b, 0, sizeof(*b));
    b->ch.kind = RB_CC_SELECT_CARD;
    if (zone) strncpy(b->ch.zone, zone, sizeof(b->ch.zone) - 1);
    if (description) strncpy(b->ch.description, description, sizeof(b->ch.description) - 1);
    b->ch.count = count;
    b->ch.allow_skip = allow_skip;
    return b;
}

RbFullChoice *rb_choice_builder_build(RbChoiceBuilder *b) {
    if (!b) return NULL;
    RbFullChoice *ch = (RbFullChoice *)malloc(sizeof(RbFullChoice));
    if (!ch) return NULL;
    *ch = b->ch;
    free(b);
    return ch;
}

void rb_choice_builder_free(RbChoiceBuilder *b) {
    free(b);
}

RbChoiceBuilder *rb_choice_builder_card_type(RbChoiceBuilder *b, const char *v) {
    if (b && v) { strncpy(b->ch.card_type, v, sizeof(b->ch.card_type) - 1); b->ch.has_card_type = 1; }
    return b;
}

RbChoiceBuilder *rb_choice_builder_cost_limit(RbChoiceBuilder *b, int v, const char *op) {
    if (b) {
        b->ch.cost_limit = v;
        if (op) strncpy(b->ch.cost_limit_op, op, sizeof(b->ch.cost_limit_op) - 1);
        b->ch.has_cost_limit = 1;
    }
    return b;
}

RbChoiceBuilder *rb_choice_builder_cost_total(RbChoiceBuilder *b, int v, const char *op) {
    if (b) {
        b->ch.cost_total = v;
        if (op) strncpy(b->ch.cost_total_op, op, sizeof(b->ch.cost_total_op) - 1);
        b->ch.has_cost_total = 1;
    }
    return b;
}

RbChoiceBuilder *rb_choice_builder_cost_values(RbChoiceBuilder *b, const uint8_t *vals, int n) {
    if (b && vals) {
        b->ch.cost_values.n_values = 0;
        for (int i = 0; i < n && b->ch.cost_values.n_values < RB_MAX_CC_U8S; i++)
            b->ch.cost_values.values[b->ch.cost_values.n_values++] = vals[i];
    }
    return b;
}

RbChoiceBuilder *rb_choice_builder_group(RbChoiceBuilder *b, const char *v) {
    if (b && v) strncpy(b->ch.group, v, sizeof(b->ch.group) - 1);
    return b;
}

RbChoiceBuilder *rb_choice_builder_characters(RbChoiceBuilder *b, const char *const *names, int n) {
    if (b && names && n > 0) {
        string_vec_set(&b->ch.characters, names, n);
        b->ch.has_characters = 1;
    }
    return b;
}

RbChoiceBuilder *rb_choice_builder_heart_colors(RbChoiceBuilder *b, const char *const *colors, int n) {
    if (b) string_vec_set(&b->ch.heart_colors, colors, n);
    return b;
}

RbChoiceBuilder *rb_choice_builder_require_all_heart_colors(RbChoiceBuilder *b, int v) {
    if (b) b->ch.require_all_heart_colors = v;
    return b;
}

RbChoiceBuilder *rb_choice_builder_name_fragments(RbChoiceBuilder *b, const char *const *frags, int n) {
    if (b) string_vec_set(&b->ch.name_fragments, frags, n);
    return b;
}

RbChoiceBuilder *rb_choice_builder_destination(RbChoiceBuilder *b, const char *v) {
    if (b && v) strncpy(b->ch.destination, v, sizeof(b->ch.destination) - 1);
    return b;
}

RbChoiceBuilder *rb_choice_builder_discard_remaining(RbChoiceBuilder *b, int v) {
    if (b) b->ch.discard_remaining = v;
    return b;
}

RbChoiceBuilder *rb_choice_builder_is_select_action(RbChoiceBuilder *b, int v) {
    if (b) b->ch.is_select_action = v;
    return b;
}

RbChoiceBuilder *rb_choice_builder_target_player_id(RbChoiceBuilder *b, const char *v) {
    if (b && v) strncpy(b->ch.target_player_id, v, sizeof(b->ch.target_player_id) - 1);
    return b;
}

RbChoiceBuilder *rb_choice_builder_blind(RbChoiceBuilder *b, int v) {
    if (b) b->ch.blind = v;
    return b;
}

RbChoiceBuilder *rb_choice_builder_is_reveal(RbChoiceBuilder *b, int v) {
    if (b) b->ch.is_reveal = v;
    return b;
}

RbChoiceBuilder *rb_choice_builder_picker(RbChoiceBuilder *b, const char *v) {
    if (b && v) strncpy(b->ch.picker, v, sizeof(b->ch.picker) - 1);
    return b;
}

RbChoiceBuilder *rb_choice_builder_description_en(RbChoiceBuilder *b, const char *v) {
    if (b && v) strncpy(b->ch.description_en, v, sizeof(b->ch.description_en) - 1);
    return b;
}

RbChoiceBuilder *rb_choice_builder_description_ja(RbChoiceBuilder *b, const char *v) {
    if (b && v) strncpy(b->ch.description_ja, v, sizeof(b->ch.description_ja) - 1);
    return b;
}

RbChoiceBuilder *rb_choice_builder_filtered_indices(RbChoiceBuilder *b, const int *indices, int n) {
    if (b && indices) {
        b->ch.n_filtered = 0;
        for (int i = 0; i < n && b->ch.n_filtered < RB_MAX_ZONE; i++)
            b->ch.filtered_indices[b->ch.n_filtered++] = indices[i];
        b->ch.has_filtered = 1;
    }
    return b;
}

/* ── ArcStr serialize/deserialize (types.rs) ──────────────────────────
    Mirrors ArcStr's serde impl: serialize wraps in a JSON string; the C
    string table already stores owned values, so deserialize just trims the
    surrounding quotes in place. */

char *rb_arcstr_serialize(const char *s) {
    size_t n = s ? strlen(s) : 0;
    char *out = (char *)malloc(n + 3);
    if (!out) return NULL;
    out[0] = '"';
    if (n) memcpy(out + 1, s, n);
    out[n + 1] = '"';
    out[n + 2] = 0;
    return out;
}

void rb_arcstr_deserialize(char *s) {
    if (!s) return;
    size_t n = strlen(s);
    if (n >= 2 && s[0] == '"' && s[n - 1] == '"') {
        s[n - 1] = 0;
        memmove(s, s + 1, n - 1);
    }
}

/* ── LookAndSelectStep constructors (types.rs::LookAndSelectStep) ───── */

RbLookAndSelectStep rb_look_and_select_step_look(int count, const char *source) {
    RbLookAndSelectStep st;
    memset(&st, 0, sizeof(st));
    st.kind = RB_LAS_LOOK;
    st.look_count = count;
    if (source) strncpy(st.look_source, source, sizeof(st.look_source) - 1);
    return st;
}

RbLookAndSelectStep rb_look_and_select_step_select(int count, int max_per_group) {
    RbLookAndSelectStep st;
    memset(&st, 0, sizeof(st));
    st.kind = RB_LAS_SELECT;
    st.select_count = count;
    if (max_per_group > 0) {
        st.select_max_per_group = max_per_group;
        st.has_select_max_per_group = 1;
    }
    return st;
}

RbLookAndSelectStep rb_look_and_select_step_finalize(const char *destination, const char *source_zone) {
    RbLookAndSelectStep st;
    memset(&st, 0, sizeof(st));
    st.kind = RB_LAS_FINALIZE;
    if (destination) strncpy(st.finalize_destination, destination, sizeof(st.finalize_destination) - 1);
    if (source_zone) strncpy(st.finalize_source_zone, source_zone, sizeof(st.finalize_source_zone) - 1);
    return st;
}
