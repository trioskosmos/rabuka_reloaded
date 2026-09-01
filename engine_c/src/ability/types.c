#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

/* ====================================================================
 *  Port of engine/src/ability/types.rs
 *  All type definitions live in rabuka.h.  This file provides:
 *    - RB_PAY_SKIP_TARGET constant
 *    - RbChoiceRouteKind string helpers
 *    - RbFullChoice constructors / accessors
 *    - RbChoiceResult helpers
 *    - RbTriggerEvent helpers
 *    - RbEffectSpawnContext helpers
 *    - RbStepOutput helpers
 *    - RbValueRef helpers
 *    - RbZoneSnapshot helpers
 *    - RbAbilityTraceNode helpers
 *    - RbEffectPipeline helpers
 *    - RbStepState helpers
 *    - rb_ability_error_format
 *    - rb_gained_ability_index
 *    - RbExecutionContextKind / RbLookAndSelectStepKind string helpers
 *    - RbLookAndSelectStep constructors
 * ==================================================================== */

/* ── Constants ─────────────────────────────────────────────────────── */

const char *RB_PAY_SKIP_TARGET = "pay_optional_cost:skip_optional_cost";

/* ── ChoiceRoute helpers ────────────────────────────────────────────── */

const char *rb_choice_route_to_str(RbChoiceRouteKind r) {
    switch (r) {
        case RB_ROUTEK_CHOICE:       return "choice";
        case RB_ROUTEK_CHOICE_STRING:return "choice_string";
        case RB_ROUTEK_CHOICE_COST:  return "choice_cost";
        case RB_ROUTEK_OPTIONAL_COST:return "optional_cost";
        case RB_ROUTEK_CHANGE_STATE: return "change_state";
        case RB_ROUTEK_RAW:          return "raw";
    }
    return "unknown";
}

int rb_choice_route_from_str(const char *s, RbChoiceRouteKind *out) {
    if (!s || !out) return -1;
    if (!strcmp(s, "choice"))         { *out = RB_ROUTEK_CHOICE;        return 0; }
    if (!strcmp(s, "choice_string"))  { *out = RB_ROUTEK_CHOICE_STRING; return 0; }
    if (!strcmp(s, "choice_cost"))    { *out = RB_ROUTEK_CHOICE_COST;   return 0; }
    if (!strcmp(s, "optional_cost"))  { *out = RB_ROUTEK_OPTIONAL_COST; return 0; }
    if (!strcmp(s, "change_state"))   { *out = RB_ROUTEK_CHANGE_STATE;  return 0; }
    if (!strcmp(s, "raw"))            { *out = RB_ROUTEK_RAW;            return 0; }
    return -1;
}

RbChoiceRouteKind rb_choice_route_new(const char *s) {
    RbChoiceRouteKind k = RB_ROUTEK_CHOICE;
    if (s && rb_choice_route_from_str(s, &k) != 0)
        k = RB_ROUTEK_RAW;
    return k;
}

RbChoiceRoute rb_choice_route_from_kind(RbChoiceRouteKind k) {
    switch (k) {
        case RB_ROUTEK_CHOICE:        return RB_ROUTE_SELECT_CARDS;
        case RB_ROUTEK_CHOICE_STRING: return RB_ROUTE_SELECT_TARGET;
        case RB_ROUTEK_CHOICE_COST:   return RB_ROUTE_CHOICE_COST;
        case RB_ROUTEK_OPTIONAL_COST: return RB_ROUTE_OPTIONAL_COST;
        case RB_ROUTEK_CHANGE_STATE:  return RB_ROUTE_CONDITIONAL_CHOICE;
        case RB_ROUTEK_RAW:           return RB_ROUTE_SELECT_TARGET;
    }
    return RB_ROUTE_SELECT_CARDS;
}

RbChoiceRouteKind rb_choice_route_kind_from_header(RbChoiceRoute r) {
    switch (r) {
        case RB_ROUTE_NONE:           return RB_ROUTEK_CHOICE;
        case RB_ROUTE_OPTIONAL_COST:  return RB_ROUTEK_OPTIONAL_COST;
        case RB_ROUTE_CHOICE_COST:    return RB_ROUTEK_CHOICE_COST;
        case RB_ROUTE_SELECT_CARDS:   return RB_ROUTEK_CHOICE;
        case RB_ROUTE_SELECT_TARGET:  return RB_ROUTEK_CHOICE_STRING;
        case RB_ROUTE_CONDITIONAL_CHOICE: return RB_ROUTEK_CHANGE_STATE;
    }
    return RB_ROUTEK_RAW;
}

/* ── RbFullChoice constructors ─────────────────────────────────────── */

static void rb_fc_clear(RbFullChoice *ch) {
    if (!ch) return;
    memset(ch, 0, sizeof(*ch));
}

static void rb_fc_set_str(char *dst, size_t dst_sz, const char *src) {
    if (!dst || !src) return;
    strncpy(dst, src, dst_sz - 1);
    dst[dst_sz - 1] = '\0';
}

RbFullChoice *rb_full_choice_new_select_card(const char *zone, const char *description,
                                              int count, int allow_skip) {
    RbFullChoice *ch = (RbFullChoice *)rb_malloc(sizeof(RbFullChoice));
    if (!ch) return NULL;
    rb_fc_clear(ch);
    ch->kind = RB_CC_SELECT_CARD;
    rb_fc_set_str(ch->zone, sizeof(ch->zone), zone ? zone : "");
    rb_fc_set_str(ch->description, sizeof(ch->description), description ? description : "");
    ch->count = count;
    ch->allow_skip = allow_skip ? 1 : 0;
    return ch;
}

RbFullChoice *rb_full_choice_new_select_target(const char *target, const char *description,
                                                int allow_skip) {
    RbFullChoice *ch = (RbFullChoice *)rb_malloc(sizeof(RbFullChoice));
    if (!ch) return NULL;
    rb_fc_clear(ch);
    ch->kind = RB_CC_SELECT_TARGET;
    rb_fc_set_str(ch->target, sizeof(ch->target), target ? target : "");
    rb_fc_set_str(ch->description, sizeof(ch->description), description ? description : "");
    ch->allow_skip = allow_skip ? 1 : 0;
    return ch;
}

RbFullChoice *rb_full_choice_new_select_position(const char *position, const char *description,
                                                  int allow_skip) {
    RbFullChoice *ch = (RbFullChoice *)rb_malloc(sizeof(RbFullChoice));
    if (!ch) return NULL;
    rb_fc_clear(ch);
    ch->kind = RB_CC_SELECT_POSITION;
    rb_fc_set_str(ch->position, sizeof(ch->position), position ? position : "");
    rb_fc_set_str(ch->description, sizeof(ch->description), description ? description : "");
    ch->allow_skip = allow_skip ? 1 : 0;
    return ch;
}

RbFullChoice *rb_full_choice_new_select_heart_color(int count, const char *const *options,
                                                     int n_options, const char *description) {
    RbFullChoice *ch = (RbFullChoice *)rb_malloc(sizeof(RbFullChoice));
    if (!ch) return NULL;
    rb_fc_clear(ch);
    ch->kind = RB_CC_SELECT_HEART_COLOR;
    ch->count = count;
    rb_fc_set_str(ch->description, sizeof(ch->description), description ? description : "");
    if (options && n_options > 0) {
        ch->hc_options.n_strings = n_options < RB_MAX_CC_STRINGS ? n_options : RB_MAX_CC_STRINGS;
        for (int i = 0; i < ch->hc_options.n_strings; i++)
            rb_fc_set_str(ch->hc_options.strings[i], sizeof(ch->hc_options.strings[i]),
                           options[i] ? options[i] : "");
    }
    return ch;
}

RbFullChoice *rb_repeat_prompt_choice(void) {
    RbFullChoice *ch = (RbFullChoice *)rb_malloc(sizeof(RbFullChoice));
    if (!ch) return NULL;
    rb_fc_clear(ch);
    ch->kind = RB_CC_SELECT_TARGET;
    rb_fc_set_str(ch->target, sizeof(ch->target), RB_PAY_SKIP_TARGET);
    rb_fc_set_str(ch->description, sizeof(ch->description), "Repeat effect?");
    rb_fc_set_str(ch->description_en, sizeof(ch->description_en), "Repeat effect?");
    rb_fc_set_str(ch->description_ja, sizeof(ch->description_ja),
                   "効果を繰り返しますか？");
    ch->allow_skip = 1;
    ch->options.n_strings = 2;
    rb_fc_set_str(ch->options.strings[0], sizeof(ch->options.strings[0]), "Stop");
    rb_fc_set_str(ch->options.strings[1], sizeof(ch->options.strings[1]), "Continue");
    return ch;
}

/* ── RbFullChoice accessors ─────────────────────────────────────────── */

const char *rb_full_choice_description_ja(const RbFullChoice *ch) {
    if (!ch) return NULL;
    switch (ch->kind) {
        case RB_CC_SELECT_CARD:
        case RB_CC_SELECT_TARGET:
        case RB_CC_SELECT_POSITION:
        case RB_CC_SELECT_HEART_COLOR:
        case RB_CC_SELECT_HEART_TYPE:
        case RB_CC_SELECT_AUTO_ABILITY:
        case RB_CC_SELECT_LIVE_SUCCESS:
            return ch->description_ja[0] ? ch->description_ja : NULL;
    }
    return NULL;
}

int rb_full_choice_allow_skip(const RbFullChoice *ch) {
    if (!ch) return 0;
    switch (ch->kind) {
        case RB_CC_SELECT_CARD:
        case RB_CC_SELECT_TARGET:
        case RB_CC_SELECT_POSITION:
            return ch->allow_skip;
        default:
            return 0;
    }
}

void rb_full_choice_set_description(RbFullChoice *ch, const char *desc) {
    if (!ch || !desc) return;
    rb_fc_set_str(ch->description, sizeof(ch->description), desc);
}

void rb_full_choice_set_bilingual(RbFullChoice *ch, const char *en, const char *ja) {
    if (!ch) return;
    if (en) rb_fc_set_str(ch->description_en, sizeof(ch->description_en), en);
    if (ja) rb_fc_set_str(ch->description_ja, sizeof(ch->description_ja), ja);
}

void rb_full_choice_free(RbFullChoice *ch) {
    if (!ch) return;
    rb_free(ch);
}

void rb_full_choice_to_header(const RbFullChoice *src, RbChoice *dst) {
    if (!src || !dst) return;
    memset(dst, 0, sizeof(*dst));
    dst->kind = RB_CHOICE_NONE;
    dst->allow_skip = 0;
    dst->count = 0;
    dst->filter_heart = -1;
    dst->n_heart_options = 0;

    switch (src->kind) {
        case RB_CC_SELECT_CARD:
            dst->kind = RB_CHOICE_SELECT_CARD;
            rb_fc_set_str(dst->zone, sizeof(dst->zone), src->zone);
            rb_fc_set_str(dst->card_type, sizeof(dst->card_type), src->card_type);
            dst->count = src->count;
            dst->allow_skip = src->allow_skip;
            rb_fc_set_str(dst->description, sizeof(dst->description), src->description);
            rb_fc_set_str(dst->filter_group, sizeof(dst->filter_group), src->group);
            break;
        case RB_CC_SELECT_TARGET:
            dst->kind = RB_CHOICE_SELECT_TARGET;
            dst->allow_skip = src->allow_skip;
            rb_fc_set_str(dst->target, sizeof(dst->target), src->target);
            rb_fc_set_str(dst->description, sizeof(dst->description), src->description);
            break;
        case RB_CC_SELECT_POSITION:
            dst->kind = RB_CHOICE_SELECT_POSITION;
            dst->allow_skip = src->allow_skip;
            rb_fc_set_str(dst->description, sizeof(dst->description), src->description);
            break;
        case RB_CC_SELECT_HEART_COLOR:
            dst->kind = RB_CHOICE_SELECT_HEART_COLOR;
            dst->count = src->count;
            dst->n_heart_options = src->hc_options.n_strings < 8 ?
                                   src->hc_options.n_strings : 8;
            for (int i = 0; i < dst->n_heart_options; i++)
                rb_fc_set_str(dst->heart_options[i], sizeof(dst->heart_options[i]),
                               src->hc_options.strings[i]);
            rb_fc_set_str(dst->description, sizeof(dst->description), src->description);
            break;
        case RB_CC_SELECT_HEART_TYPE:
            dst->kind = RB_CHOICE_SELECT_HEART_COLOR;
            dst->count = src->count;
            dst->n_heart_options = src->hc_options.n_strings < 8 ?
                                   src->hc_options.n_strings : 8;
            for (int i = 0; i < dst->n_heart_options; i++)
                rb_fc_set_str(dst->heart_options[i], sizeof(dst->heart_options[i]),
                               src->hc_options.strings[i]);
            rb_fc_set_str(dst->description, sizeof(dst->description), src->description);
            break;
        case RB_CC_SELECT_AUTO_ABILITY:
            dst->kind = RB_CHOICE_SELECT_AUTO_ABILITY;
            rb_fc_set_str(dst->description, sizeof(dst->description), src->description);
            break;
        default:
            break;
    }
}

/* Helper: append string options to a RbFullChoiceStringVec */
static void rb_fcvec_set(RbFullChoiceStringVec *vec, const char *const *opts, int n) {
    if (!vec || !opts || n <= 0) return;
    vec->n_strings = n < RB_MAX_CC_STRINGS ? n : RB_MAX_CC_STRINGS;
    for (int i = 0; i < vec->n_strings; i++)
        rb_fc_set_str(vec->strings[i], sizeof(vec->strings[i]),
                       opts[i] ? opts[i] : "");
}

void rb_full_choice_set_options(RbFullChoice *ch, const char *const *opts, int n) {
    if (!ch) return;
    rb_fcvec_set(&ch->options, opts, n);
}

void rb_full_choice_set_hc_options(RbFullChoice *ch, const char *const *opts, int n) {
    if (!ch) return;
    rb_fcvec_set(&ch->hc_options, opts, n);
}

void rb_full_choice_set_aa_options(RbFullChoice *ch, const char *const *opts, int n) {
    if (!ch) return;
    rb_fcvec_set(&ch->aa_options, opts, n);
}

void rb_full_choice_set_ls_options(RbFullChoice *ch, const char *const *opts, int n) {
    if (!ch) return;
    rb_fcvec_set(&ch->ls_options, opts, n);
}

/* ── RbFullChoice to_json ───────────────────────────────────────────── */

const char *rb_full_choice_to_json(const RbFullChoice *ch, char *buf, size_t buf_sz) {
    if (!ch || !buf || buf_sz == 0) { buf[0] = '\0'; return buf; }
    int p = 0;
    p += snprintf(buf + p, buf_sz - (size_t)p, "{");

    switch (ch->kind) {
        case RB_CC_SELECT_CARD: {
            p += snprintf(buf + p, buf_sz - (size_t)p,
                "\"kind\":\"select_card\","
                "\"zone\":\"%s\","
                "\"count\":%d,"
                "\"choose_count\":%d,"
                "\"v_remaining\":%d,"
                "\"title\":\"%s\"",
                ch->zone, ch->count, ch->count, -1, ch->description);
            if (ch->description_en[0])
                p += snprintf(buf + p, buf_sz - (size_t)p, ",\"prompt_en\":\"%s\"", ch->description_en);
            if (ch->description_ja[0])
                p += snprintf(buf + p, buf_sz - (size_t)p, ",\"prompt_ja\":\"%s\"", ch->description_ja);
            if (ch->allow_skip)
                p += snprintf(buf + p, buf_sz - (size_t)p, ",\"allow_skip\":true");
            break;
        }
        case RB_CC_SELECT_TARGET: {
            p += snprintf(buf + p, buf_sz - (size_t)p,
                "\"kind\":\"select_target\","
                "\"target\":\"%s\","
                "\"title\":\"%s\","
                "\"allow_skip\":%s",
                ch->target, ch->description, ch->allow_skip ? "true" : "false");
            if (ch->options.n_strings > 0) {
                p += snprintf(buf + p, buf_sz - (size_t)p, ",\"options\":[");
                for (int i = 0; i < ch->options.n_strings && p < (int)buf_sz - 1; i++)
                    p += snprintf(buf + p, buf_sz - (size_t)p, "\"%s\"%s",
                        ch->options.strings[i], i < ch->options.n_strings - 1 ? "," : "");
                p += snprintf(buf + p, buf_sz - (size_t)p, "]");
            }
            break;
        }
        case RB_CC_SELECT_POSITION: {
            p += snprintf(buf + p, buf_sz - (size_t)p,
                "\"kind\":\"select_position\","
                "\"position\":\"%s\","
                "\"title\":\"%s\","
                "\"allow_skip\":%s",
                ch->position, ch->description, ch->allow_skip ? "true" : "false");
            break;
        }
        case RB_CC_SELECT_HEART_COLOR:
        case RB_CC_SELECT_HEART_TYPE: {
            const char *kind_str = (ch->kind == RB_CC_SELECT_HEART_COLOR) ?
                                   "select_heart_color" : "select_heart_type";
            p += snprintf(buf + p, buf_sz - (size_t)p,
                "\"kind\":\"%s\",\"count\":%d,\"title\":\"%s\",\"options\":[",
                kind_str, ch->count, ch->description);
            for (int i = 0; i < ch->hc_options.n_strings && p < (int)buf_sz - 1; i++)
                p += snprintf(buf + p, buf_sz - (size_t)p, "\"%s\"%s",
                    ch->hc_options.strings[i], i < ch->hc_options.n_strings - 1 ? "," : "");
            p += snprintf(buf + p, buf_sz - (size_t)p, "]");
            break;
        }
        case RB_CC_SELECT_AUTO_ABILITY: {
            p += snprintf(buf + p, buf_sz - (size_t)p,
                "\"kind\":\"select_auto_ability\",\"title\":\"%s\",\"options\":[",
                ch->description);
            for (int i = 0; i < ch->aa_options.n_strings && p < (int)buf_sz - 1; i++)
                p += snprintf(buf + p, buf_sz - (size_t)p, "\"%s\"%s",
                    ch->aa_options.strings[i], i < ch->aa_options.n_strings - 1 ? "," : "");
            p += snprintf(buf + p, buf_sz - (size_t)p, "]");
            break;
        }
        case RB_CC_SELECT_LIVE_SUCCESS: {
            p += snprintf(buf + p, buf_sz - (size_t)p,
                "\"kind\":\"select_live_success\",\"title\":\"%s\",\"options\":[",
                ch->description);
            for (int i = 0; i < ch->ls_options.n_strings && p < (int)buf_sz - 1; i++)
                p += snprintf(buf + p, buf_sz - (size_t)p, "\"%s\"%s",
                    ch->ls_options.strings[i], i < ch->ls_options.n_strings - 1 ? "," : "");
            p += snprintf(buf + p, buf_sz - (size_t)p, "]");
            break;
        }
    }

    if (ch->description_en[0])
        p += snprintf(buf + p, buf_sz - (size_t)p,
            "%s\"prompt_en\":\"%s\"", p > 1 ? "," : "", ch->description_en);
    if (ch->description_ja[0])
        p += snprintf(buf + p, buf_sz - (size_t)p,
            "%s\"prompt_ja\":\"%s\"", p > 1 ? "," : "", ch->description_ja);

    p += snprintf(buf + p, buf_sz - (size_t)p, "}");
    return buf;
}

/* ── RbChoiceBuilder ────────────────────────────────────────────────── */

RbChoiceBuilder *rb_choice_builder_new(const char *zone, const char *description,
                                        int count, int allow_skip) {
    RbChoiceBuilder *b = (RbChoiceBuilder *)rb_malloc(sizeof(RbChoiceBuilder));
    if (!b) return NULL;
    rb_fc_clear(&b->ch);
    b->ch.kind = RB_CC_SELECT_CARD;
    rb_fc_set_str(b->ch.zone, sizeof(b->ch.zone), zone ? zone : "");
    rb_fc_set_str(b->ch.description, sizeof(b->ch.description), description ? description : "");
    b->ch.count = count;
    b->ch.allow_skip = allow_skip ? 1 : 0;
    return b;
}

RbFullChoice *rb_choice_builder_build(RbChoiceBuilder *b) {
    if (!b) return NULL;
    RbFullChoice *out = (RbFullChoice *)rb_malloc(sizeof(RbFullChoice));
    if (!out) return NULL;
    *out = b->ch;
    return out;
}

void rb_choice_builder_free(RbChoiceBuilder *b) {
    if (!b) return;
    rb_free(b);
}

RbChoiceBuilder *rb_choice_builder_card_type(RbChoiceBuilder *b, const char *v) {
    if (!b) return NULL;
    b->ch.has_card_type = v ? 1 : 0;
    rb_fc_set_str(b->ch.card_type, sizeof(b->ch.card_type), v ? v : "");
    return b;
}

RbChoiceBuilder *rb_choice_builder_cost_limit(RbChoiceBuilder *b, int v, const char *op) {
    if (!b) return NULL;
    b->ch.has_cost_limit = 1;
    b->ch.cost_limit = (uint8_t)v;
    rb_fc_set_str(b->ch.cost_limit_op, sizeof(b->ch.cost_limit_op), op ? op : "");
    return b;
}

RbChoiceBuilder *rb_choice_builder_cost_total(RbChoiceBuilder *b, int v, const char *op) {
    if (!b) return NULL;
    b->ch.has_cost_total = 1;
    b->ch.cost_total = v;
    rb_fc_set_str(b->ch.cost_total_op, sizeof(b->ch.cost_total_op), op ? op : "");
    return b;
}

RbChoiceBuilder *rb_choice_builder_cost_values(RbChoiceBuilder *b, const uint8_t *vals, int n) {
    if (!b || !vals || n <= 0) return b;
    b->ch.cost_values.n_values = n < RB_MAX_CC_U8S ? n : RB_MAX_CC_U8S;
    memcpy(b->ch.cost_values.values, vals, (size_t)b->ch.cost_values.n_values * sizeof(uint8_t));
    return b;
}

RbChoiceBuilder *rb_choice_builder_group(RbChoiceBuilder *b, const char *v) {
    if (!b) return NULL;
    rb_fc_set_str(b->ch.group, sizeof(b->ch.group), v ? v : "");
    return b;
}

RbChoiceBuilder *rb_choice_builder_characters(RbChoiceBuilder *b, const char *const *names, int n) {
    if (!b || !names || n <= 0) return b;
    b->ch.has_characters = 1;
    b->ch.characters.n_strings = n < RB_MAX_CC_STRINGS ? n : RB_MAX_CC_STRINGS;
    for (int i = 0; i < b->ch.characters.n_strings; i++)
        rb_fc_set_str(b->ch.characters.strings[i], sizeof(b->ch.characters.strings[i]),
                       names[i] ? names[i] : "");
    return b;
}

RbChoiceBuilder *rb_choice_builder_heart_colors(RbChoiceBuilder *b, const char *const *colors, int n) {
    if (!b || !colors || n <= 0) return b;
    b->ch.heart_colors.n_strings = n < RB_MAX_CC_STRINGS ? n : RB_MAX_CC_STRINGS;
    for (int i = 0; i < b->ch.heart_colors.n_strings; i++)
        rb_fc_set_str(b->ch.heart_colors.strings[i], sizeof(b->ch.heart_colors.strings[i]),
                       colors[i] ? colors[i] : "");
    return b;
}

RbChoiceBuilder *rb_choice_builder_require_all_heart_colors(RbChoiceBuilder *b, int v) {
    if (!b) return NULL;
    b->ch.require_all_heart_colors = v ? 1 : 0;
    return b;
}

RbChoiceBuilder *rb_choice_builder_name_fragments(RbChoiceBuilder *b, const char *const *frags, int n) {
    if (!b || !frags || n <= 0) return b;
    b->ch.name_fragments.n_strings = n < RB_MAX_CC_STRINGS ? n : RB_MAX_CC_STRINGS;
    for (int i = 0; i < b->ch.name_fragments.n_strings; i++)
        rb_fc_set_str(b->ch.name_fragments.strings[i], sizeof(b->ch.name_fragments.strings[i]),
                       frags[i] ? frags[i] : "");
    return b;
}

RbChoiceBuilder *rb_choice_builder_destination(RbChoiceBuilder *b, const char *v) {
    if (!b) return NULL;
    rb_fc_set_str(b->ch.destination, sizeof(b->ch.destination), v ? v : "");
    return b;
}

RbChoiceBuilder *rb_choice_builder_discard_remaining(RbChoiceBuilder *b, int v) {
    if (!b) return NULL;
    b->ch.discard_remaining = v ? 1 : 0;
    return b;
}

RbChoiceBuilder *rb_choice_builder_is_select_action(RbChoiceBuilder *b, int v) {
    if (!b) return NULL;
    b->ch.is_select_action = v ? 1 : 0;
    return b;
}

RbChoiceBuilder *rb_choice_builder_target_player_id(RbChoiceBuilder *b, const char *v) {
    if (!b) return NULL;
    rb_fc_set_str(b->ch.target_player_id, sizeof(b->ch.target_player_id), v ? v : "");
    return b;
}

RbChoiceBuilder *rb_choice_builder_blind(RbChoiceBuilder *b, int v) {
    if (!b) return NULL;
    b->ch.blind = v ? 1 : 0;
    return b;
}

RbChoiceBuilder *rb_choice_builder_is_reveal(RbChoiceBuilder *b, int v) {
    if (!b) return NULL;
    b->ch.is_reveal = v ? 1 : 0;
    return b;
}

RbChoiceBuilder *rb_choice_builder_picker(RbChoiceBuilder *b, const char *v) {
    if (!b) return NULL;
    rb_fc_set_str(b->ch.picker, sizeof(b->ch.picker), v ? v : "");
    return b;
}

RbChoiceBuilder *rb_choice_builder_description_en(RbChoiceBuilder *b, const char *v) {
    if (!b) return NULL;
    rb_fc_set_str(b->ch.description_en, sizeof(b->ch.description_en), v ? v : "");
    return b;
}

RbChoiceBuilder *rb_choice_builder_description_ja(RbChoiceBuilder *b, const char *v) {
    if (!b) return NULL;
    rb_fc_set_str(b->ch.description_ja, sizeof(b->ch.description_ja), v ? v : "");
    return b;
}

RbChoiceBuilder *rb_choice_builder_filtered_indices(RbChoiceBuilder *b, const int *indices, int n) {
    if (!b || !indices || n <= 0) return b;
    b->ch.n_filtered = n < RB_MAX_ZONE ? n : RB_MAX_ZONE;
    b->ch.has_filtered = 1;
    memcpy(b->ch.filtered_indices, indices, (size_t)b->ch.n_filtered * sizeof(int));
    return b;
}

/* ── RbChoiceResult helpers ─────────────────────────────────────────── */

const char *rb_choice_result_kind_to_str(RbChoiceResultKind k) {
    switch (k) {
        case RB_CR_CARD_SELECTED:     return "card_selected";
        case RB_CR_TARGET_SELECTED:   return "target_selected";
        case RB_CR_POSITION_SELECTED: return "position_selected";
        case RB_CR_HEART_COLOR:       return "heart_color";
        case RB_CR_HEART_TYPE:        return "heart_type";
        case RB_CR_AUTO_ABILITY:      return "auto_ability";
        case RB_CR_LIVE_SUCCESS:      return "live_success";
        case RB_CR_SKIP:              return "skip";
    }
    return "unknown";
}

int rb_choice_result_kind_from_str(const char *s, RbChoiceResultKind *out) {
    if (!s || !out) return -1;
    if (!strcmp(s, "card_selected"))     { *out = RB_CR_CARD_SELECTED;     return 0; }
    if (!strcmp(s, "target_selected"))   { *out = RB_CR_TARGET_SELECTED;   return 0; }
    if (!strcmp(s, "position_selected")) { *out = RB_CR_POSITION_SELECTED; return 0; }
    if (!strcmp(s, "heart_color"))       { *out = RB_CR_HEART_COLOR;       return 0; }
    if (!strcmp(s, "heart_type"))        { *out = RB_CR_HEART_TYPE;        return 0; }
    if (!strcmp(s, "auto_ability"))      { *out = RB_CR_AUTO_ABILITY;      return 0; }
    if (!strcmp(s, "live_success"))      { *out = RB_CR_LIVE_SUCCESS;      return 0; }
    if (!strcmp(s, "skip"))              { *out = RB_CR_SKIP;              return 0; }
    return -1;
}

RbChoiceResultKind rb_choice_result_kind(const RbChoiceResult *r) {
    return r ? r->kind : RB_CR_SKIP;
}

int rb_choice_result_is_skip(const RbChoiceResult *r) {
    return r ? (r->kind == RB_CR_SKIP) : 1;
}

RbChoiceResult *rb_choice_result_new_card_selected(const int *indices, int n) {
    RbChoiceResult *r = (RbChoiceResult *)rb_malloc(sizeof(RbChoiceResult));
    if (!r) return NULL;
    memset(r, 0, sizeof(*r));
    r->kind = RB_CR_CARD_SELECTED;
    if (indices && n > 0) {
        r->n_card_indices = n < RB_MAX_CC_CARDS ? n : RB_MAX_CC_CARDS;
        memcpy(r->card_indices, indices, (size_t)r->n_card_indices * sizeof(int));
    }
    return r;
}

RbChoiceResult *rb_choice_result_new_skip(void) {
    RbChoiceResult *r = (RbChoiceResult *)rb_malloc(sizeof(RbChoiceResult));
    if (!r) return NULL;
    memset(r, 0, sizeof(*r));
    r->kind = RB_CR_SKIP;
    return r;
}

RbChoiceResult *rb_choice_result_new_target(const char *target) {
    RbChoiceResult *r = (RbChoiceResult *)rb_malloc(sizeof(RbChoiceResult));
    if (!r) return NULL;
    memset(r, 0, sizeof(*r));
    r->kind = RB_CR_TARGET_SELECTED;
    rb_fc_set_str(r->target, sizeof(r->target), target ? target : "");
    return r;
}

void rb_choice_result_free(RbChoiceResult *r) {
    if (!r) return;
    rb_free(r);
}

/* ── RbTriggerEvent helpers ─────────────────────────────────────────── */

void rb_trigger_event_init(RbTriggerEvent *e) {
    if (!e) return;
    memset(e, 0, sizeof(*e));
}

void rb_trigger_event_add_moved(RbTriggerEvent *e, int card_id) {
    if (!e || e->n_moved_cards >= RB_TE_MAX_MOVED) return;
    e->moved_cards[e->n_moved_cards++] = card_id;
}

void rb_trigger_event_add_appeared(RbTriggerEvent *e, int card_id, const char *source_zone) {
    if (!e || e->n_appeared_cards >= RB_TE_MAX_APPEARED) return;
    e->appeared_cards[e->n_appeared_cards].card_id = card_id;
    rb_fc_set_str(e->appeared_cards[e->n_appeared_cards].source_zone,
                   RB_TE_ZONE_SZ, source_zone ? source_zone : "");
    e->n_appeared_cards++;
}

int rb_trigger_event_has_moved(const RbTriggerEvent *e) {
    return e ? (e->n_moved_cards > 0) : 0;
}

int rb_trigger_event_has_appeared(const RbTriggerEvent *e) {
    return e ? (e->n_appeared_cards > 0) : 0;
}

int rb_trigger_event_has_position_change(const RbTriggerEvent *e) {
    return e ? e->position_change_occurred : 0;
}

int rb_trigger_event_has_energy_placed(const RbTriggerEvent *e) {
    return e ? e->energy_placed_by_effect : 0;
}

void rb_trigger_event_copy(RbTriggerEvent *dst, const RbTriggerEvent *src) {
    if (!dst || !src) return;
    memcpy(dst, src, sizeof(*dst));
}

/* ── RbEffectSpawnContext helpers ───────────────────────────────────── */

void rb_effect_spawn_context_init(RbEffectSpawnContext *ctx) {
    if (!ctx) return;
    memset(ctx, 0, sizeof(*ctx));
}

/* ── RbStepOutput helpers ───────────────────────────────────────────── */

void rb_step_output_init(RbStepOutput *out) {
    if (!out) return;
    memset(out, 0, sizeof(*out));
}

RbStepOutput *rb_step_output_from_value(int value) {
    RbStepOutput *out = (RbStepOutput *)rb_malloc(sizeof(RbStepOutput));
    if (!out) return NULL;
    rb_step_output_init(out);
    out->has_value = 1;
    out->value = value;
    return out;
}

void rb_step_output_merge(RbStepOutput *self, const RbStepOutput *other) {
    if (!self || !other) return;
    for (int i = 0; i < other->n_cards && self->n_cards < RB_SO_MAX_CARDS; i++)
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
    if (!out || out->n_cards >= RB_SO_MAX_CARDS) return;
    out->cards[out->n_cards++] = card_id;
}

int rb_step_output_has_cards(const RbStepOutput *out) {
    return out ? (out->n_cards > 0) : 0;
}

int rb_step_output_value(const RbStepOutput *out) {
    return out && out->has_value ? out->value : 0;
}

int rb_step_output_accepted(const RbStepOutput *out) {
    return out && out->has_accepted ? out->accepted : 0;
}

const char *rb_step_output_to_json(const RbStepOutput *out, char *buf, size_t buf_sz) {
    if (!out || !buf || buf_sz == 0) return "";
    int p = 0;
    p += snprintf(buf + p, buf_sz - (size_t)p, "{");
    if (out->n_cards > 0) {
        p += snprintf(buf + p, buf_sz - (size_t)p, "\"cards\":[");
        for (int i = 0; i < out->n_cards && p < (int)buf_sz - 1; i++)
            p += snprintf(buf + p, buf_sz - (size_t)p, "%d%s", out->cards[i],
                          i < out->n_cards - 1 ? "," : "");
        p += snprintf(buf + p, buf_sz - (size_t)p, "]");
    }
    if (out->has_value)
        p += snprintf(buf + p, buf_sz - (size_t)p,
                      "%s\"value\":%d", out->n_cards > 0 ? "," : "", out->value);
    if (out->has_accepted)
        p += snprintf(buf + p, buf_sz - (size_t)p,
                      "%s\"accepted\":%s",
                      (out->n_cards > 0 || out->has_value) ? "," : "",
                      out->accepted ? "true" : "false");
    p += snprintf(buf + p, buf_sz - (size_t)p, "}");
    return buf;
}

/* ── RbValueRef helpers ─────────────────────────────────────────────── */

void rb_value_ref_init_literal(RbValueRef *ref, int value) {
    if (!ref) return;
    ref->kind = RB_VR_LITERAL;
    ref->literal_value = value;
    ref->step_id[0] = '\0';
    ref->offset = 0;
}

void rb_value_ref_init_step(RbValueRef *ref, const char *step_id) {
    if (!ref) return;
    ref->kind = RB_VR_STEP_VALUE;
    ref->literal_value = 0;
    rb_fc_set_str(ref->step_id, sizeof(ref->step_id), step_id);
    ref->offset = 0;
}

void rb_value_ref_init_accepted(RbValueRef *ref, const char *step_id) {
    if (!ref) return;
    ref->kind = RB_VR_STEP_ACCEPTED;
    ref->literal_value = 0;
    rb_fc_set_str(ref->step_id, sizeof(ref->step_id), step_id);
    ref->offset = 0;
}

void rb_value_ref_init_offset(RbValueRef *ref, const char *step_id, int offset) {
    if (!ref) return;
    ref->kind = RB_VR_STEP_OFFSET;
    ref->literal_value = 0;
    rb_fc_set_str(ref->step_id, sizeof(ref->step_id), step_id);
    ref->offset = offset;
}

const char *rb_value_ref_kind_str(const RbValueRef *ref) {
    if (!ref) return "unknown";
    return rb_value_ref_kind_to_str(ref->kind);
}

const char *rb_value_ref_kind_to_str(RbValueRefKind k) {
    switch (k) {
        case RB_VR_LITERAL:       return "literal";
        case RB_VR_STEP_VALUE:    return "step_value";
        case RB_VR_STEP_ACCEPTED: return "step_accepted";
        case RB_VR_STEP_OFFSET:   return "step_value_offset";
    }
    return "unknown";
}

int rb_value_ref_kind_from_str(const char *s, RbValueRefKind *out) {
    if (!s || !out) return -1;
    if (!strcmp(s, "literal"))           { *out = RB_VR_LITERAL;       return 0; }
    if (!strcmp(s, "step_value"))        { *out = RB_VR_STEP_VALUE;    return 0; }
    if (!strcmp(s, "step_accepted"))     { *out = RB_VR_STEP_ACCEPTED; return 0; }
    if (!strcmp(s, "step_value_offset")) { *out = RB_VR_STEP_OFFSET;   return 0; }
    return -1;
}

/* Resolve to a concrete int32 against a step_results map.
 * step_results is provided as a callback: for each step_id, the callback
 * returns 1 if found (writing value into *out_value) or 0 if absent. */
typedef int (*RbStepLookupFn)(const char *step_id, int *out_value, int *out_accepted, void *ctx);

int rb_value_ref_resolve(const RbValueRef *ref, RbStepLookupFn lookup,
                          void *lookup_ctx, int fallback) {
    if (!ref) return fallback;
    switch (ref->kind) {
        case RB_VR_LITERAL:
            return ref->literal_value;
        case RB_VR_STEP_VALUE: {
            int v = 0;
            if (lookup && lookup(ref->step_id, &v, NULL, lookup_ctx))
                return v;
            return fallback;
        }
        case RB_VR_STEP_ACCEPTED: {
            int accepted = 0;
            if (lookup && lookup(ref->step_id, NULL, &accepted, lookup_ctx))
                return accepted ? 1 : 0;
            return fallback;
        }
        case RB_VR_STEP_OFFSET: {
            int v = 0;
            if (lookup && lookup(ref->step_id, &v, NULL, lookup_ctx))
                return v + ref->offset;
            return fallback;
        }
    }
    return fallback;
}

void rb_value_ref_merge(RbValueRef *self, const RbValueRef *other) {
    if (!self || !other) return;
    if (self->kind == RB_VR_LITERAL && other->kind != RB_VR_LITERAL) {
        self->kind = other->kind;
        rb_fc_set_str(self->step_id, sizeof(self->step_id), other->step_id);
        self->offset = other->offset;
    }
}

int rb_value_ref_is_literal(const RbValueRef *ref) {
    return ref ? (ref->kind == RB_VR_LITERAL) : 0;
}

/* ── RbZoneSnapshot helpers ─────────────────────────────────────────── */

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
    RbZoneSnapshot s;
    memset(&s, 0, sizeof(s));
    if (!g) return s;
    for (int pl = 0; pl < 2; pl++) {
        const RbPlayer *p = &g->p[pl];
        s.hand_count       += p->hand.n;
        s.waitroom_count   += p->discard.n;
        s.energy_count     += p->energy.n;
        s.deck_count       += p->deck.n;
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (p->stage[i] >= 0) s.stage_count++;
        s.active_energy_count += p->energy_active;
    }
    return s;
}

/* ── RbAbilityTraceNode helpers ─────────────────────────────────────── */

RbAbilityTraceNode *rb_trace_node_new(const char *label) {
    RbAbilityTraceNode *n = (RbAbilityTraceNode *)rb_malloc(sizeof(RbAbilityTraceNode));
    if (!n) return NULL;
    rb_fc_set_str(n->label, sizeof(n->label), label ? label : "");
    n->card[0] = '\0';
    n->has_card = 0;
    n->has_before = 0;
    n->has_after = 0;
    n->n_children = 0;
    return n;
}

RbAbilityTraceNode *rb_trace_node_with_card(RbAbilityTraceNode *node, const char *card) {
    if (!node) return NULL;
    node->has_card = 1;
    rb_fc_set_str(node->card, sizeof(node->card), card ? card : "");
    return node;
}

RbAbilityTraceNode *rb_trace_node_with_before(RbAbilityTraceNode *node, RbZoneSnapshot before) {
    if (!node) return NULL;
    node->has_before = 1;
    node->before = before;
    return node;
}

RbAbilityTraceNode *rb_trace_node_with_after(RbAbilityTraceNode *node, RbZoneSnapshot after) {
    if (!node) return NULL;
    node->has_after = 1;
    node->after = after;
    return node;
}

RbAbilityTraceNode *rb_trace_node_from_game_state(const char *label, const GameState *g) {
    RbAbilityTraceNode *n = rb_trace_node_new(label);
    if (!n) return NULL;
    n->has_before = 1;
    n->before = rb_zone_snapshot_from_game_state(g);
    return n;
}

int rb_trace_node_add_child(RbAbilityTraceNode *parent, RbAbilityTraceNode *child) {
    if (!parent || !child || parent->n_children >= RB_TRACE_MAX_CHILDREN) return -1;
    parent->children[parent->n_children++] = child;
    return 0;
}

void rb_trace_node_free(RbAbilityTraceNode *node) {
    if (!node) return;
    for (int i = 0; i < node->n_children; i++)
        rb_trace_node_free(node->children[i]);
    rb_free(node);
}

/* ── RbEffectPipeline helpers ───────────────────────────────────────── */

RbEffectPipeline *rb_effect_pipeline_new(void) {
    RbEffectPipeline *p = (RbEffectPipeline *)rb_malloc(sizeof(RbEffectPipeline));
    if (!p) return NULL;
    p->trace = rb_trace_node_new("root");
    return p;
}

void rb_effect_pipeline_free(RbEffectPipeline *p) {
    if (!p) return;
    rb_trace_node_free(p->trace);
    rb_free(p);
}

/* ── RbStepState helpers ────────────────────────────────────────────── */

void rb_step_state_init(RbStepState *ss) {
    if (!ss) return;
    memset(ss, 0, sizeof(*ss));
}

RbStepState *rb_step_state_new(void) {
    RbStepState *ss = (RbStepState *)rb_malloc(sizeof(RbStepState));
    if (!ss) return NULL;
    rb_step_state_init(ss);
    return ss;
}

static RbStepResultEntry *rb_step_state_find(RbStepState *ss, const char *step_id) {
    if (!ss || !step_id) return NULL;
    for (int i = 0; i < ss->n_entries; i++)
        if (!strcmp(ss->entries[i].step_id, step_id))
            return &ss->entries[i];
    return NULL;
}

void rb_step_state_record(RbStepState *ss, const char *effect_id, const RbStepOutput *output) {
    if (!ss || !effect_id || !output) return;
    RbStepResultEntry *e = rb_step_state_find(ss, effect_id);
    if (e) {
        /* merge into existing entry */
        for (int i = 0; i < output->n_cards && e->output.n_cards < RB_SO_MAX_CARDS; i++)
            e->output.cards[e->output.n_cards++] = output->cards[i];
        if (output->has_value) { e->output.has_value = 1; e->output.value = output->value; }
        if (output->has_accepted) { e->output.has_accepted = 1; e->output.accepted = output->accepted; }
    } else if (ss->n_entries < RB_SS_MAX_RESULTS) {
        RbStepResultEntry *entry = &ss->entries[ss->n_entries++];
        rb_fc_set_str(entry->step_id, sizeof(entry->step_id), effect_id);
        entry->has_output = 1;
        entry->output = *output;
    }
}

RbStepOutput rb_step_state_get(const RbStepState *ss, const char *step_id) {
    RbStepOutput empty;
    rb_step_output_init(&empty);
    if (!ss || !step_id) return empty;
    RbStepResultEntry *e = rb_step_state_find((RbStepState *)ss, step_id);
    if (e && e->has_output) return e->output;
    return empty;
}

void rb_step_state_clear(RbStepState *ss) {
    if (!ss) return;
    ss->n_entries = 0;
    ss->last_draw_count = 0;
}

void rb_step_state_free(RbStepState *ss) {
    if (!ss) return;
    rb_free(ss);
}

int rb_step_state_record_value(RbStepState *ss, const char *effect_id, int value) {
    if (!ss || !effect_id) return -1;
    RbStepOutput out;
    rb_step_output_init(&out);
    out.has_value = 1;
    out.value = value;
    rb_step_state_record(ss, effect_id, &out);
    return 0;
}

int rb_step_state_record_cards(RbStepState *ss, const char *effect_id,
                                const int *card_ids, int n) {
    if (!ss || !effect_id || !card_ids || n <= 0) return -1;
    RbStepOutput out;
    rb_step_output_init(&out);
    for (int i = 0; i < n && out.n_cards < RB_SO_MAX_CARDS; i++)
        out.cards[out.n_cards++] = card_ids[i];
    rb_step_state_record(ss, effect_id, &out);
    return 0;
}

/* ── RbAbilityError helpers ─────────────────────────────────────────── */

void rb_ability_error_format(int err, char *out, size_t out_sz,
                             int p1, int p2, int p3, const char *detail) {
    if (!out || out_sz == 0) return;
    switch (err) {
        case RB_AE_INSUFFICIENT_ENERGY:
            snprintf(out, out_sz,
                     "Could not pay %d energy (only %d active energy available, %d total energy cards)",
                     p1, p2, p3);
            break;
        case RB_AE_CANNOT_PLACE:
            snprintf(out, out_sz, "%s", detail ? detail : "Cannot place card");
            break;
        case RB_AE_GENERIC:
            snprintf(out, out_sz, "%s", detail ? detail : "");
            break;
        case RB_AE_OTHER:
            snprintf(out, out_sz, "%s", detail ? detail : "");
            break;
        default:
            rb_fc_set_str(out, out_sz, rb_ability_error_to_string(err));
            break;
    }
}

int rb_gained_ability_index(int ability_idx) {
    if (ability_idx < RB_GAINED_ABILITY_INDEX_BASE) return -1;
    int g = ability_idx - RB_GAINED_ABILITY_INDEX_BASE;
    return (g < RB_GAINED_ABILITY_INDEX_BASE) ? g : -1;
}

/* ── RbExecutionContextKind helpers ─────────────────────────────────── */

const char *rb_exec_context_kind_to_str(RbExecutionContextKind k) {
    switch (k) {
        case RB_EC_NONE:                return "none";
        case RB_EC_SINGLE_EFFECT:       return "single_effect";
        case RB_EC_LOOK_AND_SELECT:     return "look_and_select";
        case RB_EC_MOVE_CARDS_POSITION: return "move_cards_position";
    }
    return "unknown";
}

int rb_exec_context_kind_from_str(const char *s, RbExecutionContextKind *out) {
    if (!s || !out) return -1;
    if (!strcmp(s, "none"))                { *out = RB_EC_NONE;                return 0; }
    if (!strcmp(s, "single_effect"))       { *out = RB_EC_SINGLE_EFFECT;       return 0; }
    if (!strcmp(s, "look_and_select"))     { *out = RB_EC_LOOK_AND_SELECT;     return 0; }
    if (!strcmp(s, "move_cards_position")) { *out = RB_EC_MOVE_CARDS_POSITION; return 0; }
    return -1;
}

/* ── RbLookAndSelectStepKind helpers ────────────────────────────────── */

const char *rb_las_kind_to_str(RbLookAndSelectStepKind k) {
    switch (k) {
        case RB_LAS_LOOK:     return "look_at";
        case RB_LAS_SELECT:   return "select";
        case RB_LAS_FINALIZE: return "finalize";
    }
    return "unknown";
}

int rb_las_kind_from_str(const char *s, RbLookAndSelectStepKind *out) {
    if (!s || !out) return -1;
    if (!strcmp(s, "look_at"))   { *out = RB_LAS_LOOK;     return 0; }
    if (!strcmp(s, "select"))    { *out = RB_LAS_SELECT;   return 0; }
    if (!strcmp(s, "finalize"))  { *out = RB_LAS_FINALIZE; return 0; }
    return -1;
}

/* ── RbLookAndSelectStep constructors ───────────────────────────────── */

RbLookAndSelectStep rb_look_and_select_step_look(int count, const char *source) {
    RbLookAndSelectStep s;
    memset(&s, 0, sizeof(s));
    s.kind = RB_LAS_LOOK;
    s.look_count = count;
    rb_fc_set_str(s.look_source, sizeof(s.look_source), source ? source : "");
    return s;
}

RbLookAndSelectStep rb_look_and_select_step_select(int count, int max_per_group) {
    RbLookAndSelectStep s;
    memset(&s, 0, sizeof(s));
    s.kind = RB_LAS_SELECT;
    s.select_count = count;
    if (max_per_group >= 0) {
        s.has_select_max_per_group = 1;
        s.select_max_per_group = (uint8_t)max_per_group;
    }
    return s;
}

RbLookAndSelectStep rb_look_and_select_step_finalize(const char *destination, const char *source_zone) {
    RbLookAndSelectStep s;
    memset(&s, 0, sizeof(s));
    s.kind = RB_LAS_FINALIZE;
    rb_fc_set_str(s.finalize_destination, sizeof(s.finalize_destination),
                   destination ? destination : "");
    rb_fc_set_str(s.finalize_source_zone, sizeof(s.finalize_source_zone),
                   source_zone ? source_zone : "");
    return s;
}
