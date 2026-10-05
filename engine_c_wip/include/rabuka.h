#ifndef RABUKA_H
#define RABUKA_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

/* ── Tag bytes (mirror engine/src/ability/vm.rs) ── */
#define RB_TAG_NULL      0x00
#define RB_TAG_FALSE     0x01
#define RB_TAG_TRUE      0x02
#define RB_TAG_I64       0x03
#define RB_TAG_F64       0x04
#define RB_TAG_STR       0x06
#define RB_TAG_ARRAY     0x07
#define RB_TAG_OBJECT    0x08
#define RB_TAG_OBJVAR    0x09

/* ── Heart colors (compile_cards.py HEART_COLORS) ── */
typedef enum {
    RB_HEART_PINK = 0, RB_HEART_RED, RB_HEART_YELLOW, RB_HEART_GREEN,
    RB_HEART_BLUE, RB_HEART_PURPLE, RB_HEART_ORANGE, RB_HEART_ALL,
    RB_HEART_DRAW, RB_HEART_SCORE, RB_HEART_ANY
} RbHeartColor;

typedef enum { RB_CT_SCHOOL = 0, RB_CT_LIVE, RB_CT_PROMISE } RbCardType;

/* ── Condition tree (decoded from bytecode; generic key/value node) ── */
#define RB_MAX_COND_FIELD 64
#define RB_MAX_COND_ARR   32

typedef struct CondValue {
    uint8_t tag;
    int64_t i;
    int     b;
    char   *s;
    struct Condition *cond;        /* for nested condition */
    struct CondValue *arr;         /* for array of values */
    uint32_t arr_n;
} CondValue;

typedef struct CondField {
    char    *key;
    CondValue v;
} CondField;

typedef struct Condition {
    uint8_t   variant;             /* OBJVAR discriminant (0..19) */
    CondField fields[RB_MAX_COND_FIELD];
    uint32_t  n_fields;
} Condition;

/* Condition variant  Emirrors the discriminant order of the Rust
   `engine/src/core/card.rs:Condition` enum (Compound=0, Location=1, …).
   The bytecode serializer writes this exact index as the variant byte, so
   these enumerators are NOT arbitrary magic numbers: they are the same
   Rust enum tags, given names the way Rust's `match` would. Switch on the
   named constants instead of bare ints. */
typedef enum {
    RB_COND_COMPOUND = 0,   /* compound / or_condition */
    RB_COND_LOCATION,       /* card_count_condition / location_condition */
    RB_COND_COMPARISON,     /* comparison / both / all_cost / highest_cost_on_stage */
    RB_COND_MOVEMENT,       /* movement_condition / has_moved / not_moved */
    RB_COND_GROUP,          /* group_condition */
    RB_COND_APPEARANCE,     /* appearance_condition */
    RB_COND_TEMPORAL,       /* temporal_condition */
    RB_COND_STATE,          /* state / energy_state / state_change */
    RB_COND_RESOURCE,       /* resource_condition / card_blade_condition */
    RB_COND_ABILITY_FILTER, /* ability_filter_condition */
    RB_COND_SCORE_THRESHOLD,/* score_threshold_condition */
    RB_COND_CHOICE,         /* choice_condition / position_change_condition */
    RB_COND_COMPLEX,        /* complex_condition */
    RB_COND_POSITION,       /* position_condition */
    RB_COND_OPPONENT_CHOICE,/* opponent_choice_condition */
    RB_COND_OPPONENT_LIVE_SUCCESS, /* opponent_live_success */
    RB_COND_NO_EXCESS_HEART,/* no_excess_heart */
    RB_COND_ALWAYS_TRUE,    /* otherwise / action_success / custom */
    RB_COND_ANY_OF,         /* any_of_condition */
    RB_COND_ALL_REVEALED,   /* all_revealed_match_heart_color */
    /* Rust's 20th variant. `Condition::Unsupported` (core/card.rs:506-508) makes the
     * condition FAIL - it is not a neutral no-op. The C enum previously stopped at 19,
     * so variant 20 had no name and condition.c had to pin it with a local enumerator;
     * that also meant `gcc -Wswitch` warned on the real value. Appended last so every
     * existing discriminant is unchanged. */
    RB_COND_UNSUPPORTED = 20
} RbConditionVariant;

/* ── Ability effect tree (decoded from bytecode) ── */
#define RB_MAX_CHILD 64
#define RB_MAX_EXTRA 32

typedef struct AbilityEffect {
    char *text;
    char *action;       /* action wire string, NULL if absent */
    char *source;
    char *destination;
    char *target;
    int   count;        /* -1 = none */
    int   has_condition;
    Condition *condition;/* decoded condition tree (NULL if none) */
    int   is_optional;
    int   is_further;
    struct AbilityEffect *child[RB_MAX_CHILD];
    int   n_child;
    struct AbilityEffect *options[RB_MAX_CHILD];
    int   n_options;
    char *extra_k[RB_MAX_EXTRA];
    char *extra_v[RB_MAX_EXTRA];
    int   n_extra;
    /* compound sub-effects — mirror AbilityEffect::compound (ability/types.rs).
       These are decoded from their own wire keys (primary_effect / alternative_effect
       / followup_action / optional_action / conditional_action / result_condition /
       alternative_condition) instead of child[] so branch ordering is unambiguous and
       the generic pre-order walk in rb_execute_effect_ex never double-executes them. */
    struct AbilityEffect *primary_effect;
    struct AbilityEffect *alternative_effect;
    struct AbilityEffect *look_action;
    struct AbilityEffect *select_action;
    struct AbilityEffect *followup_action;
    struct AbilityEffect *optional_action;
    struct AbilityEffect *conditional_action;
    struct AbilityEffect *gained_effect;
    struct AbilityEffect *resource_on_select;
    struct AbilityEffect *opponent_action;
    Condition *result_condition;       /* a Condition, not an effect */
    Condition *alternative_condition;   /* a Condition, not an effect */
    /* activation_condition_parsed (ability/types.rs, mirrored by
       AbilityEffect::activation_condition_parsed_any). Checked on its own by
       AbilityResolver::can_activate_effect (resolver.rs:456-494) IN ADDITION TO
       `condition`: 13 abilities carry one, and on 9 of them it is the ONLY gate
       (`condition` is NULL). ADDITIVE field: no existing member's type or order
       changed. Owned by the effect; released in effect_free (vm.c). */
    Condition *activation_condition;
    int   repeat_limit;                /* repeat_procedure: max ADDITIONAL iterations */
    int   conditional_flag;            /* effect.compound.conditional */
    int   conditional_negation;        /* effect.compound.conditional_negation (on_optional) */
    int   per_unit;                    /* per_unit count (0 = none) */
    int   per_unit_count;
    int   cost_reduction_per_group;    /* cost_reduction_per_group (0 = none) */
    int   distinct_flag;               /* per-unit distinct filter */
    char  id_field[32];                /* effect id (step output ref key) */
    char  self_target_field[8];        /* "true"/"false" */
    char  card_type_field[24];         /* member_card / live_card / energy_card */
} AbilityEffect;

#define RB_MAX_ABILITY_KEYWORDS 64

typedef struct Ability {
    char *full_text;
    char *triggerless_text;
    char *triggers;
    int   use_limit;    /* -1 = none */
    int   is_null;
    AbilityEffect *cost;    /* nullable */
    AbilityEffect *effect;  /* nullable */
    int   has_keywords;
    int   n_keywords;
    unsigned char keywords[RB_MAX_ABILITY_KEYWORDS];
} Ability;

/* ── Portable allocator ── */
void *rb_malloc(size_t n);
void  rb_free(void *p);
char *rb_strdup2(const char *s);

/* ── Constants (engine/src/core/constants.rs) ── */
#define RB_STAGE_SIZE          3
#define RB_MAX_ENERGY_CARDS    12
#define RB_MAX_LIVE_CARDS      3
#define RB_VICTORY_CARD_COUNT  3
#define RB_MAX_ZONE            512
#define RB_MAX_HAND            40
#define RB_MAX_DECK            60
#define RB_MAX_HEART_COLORS    11
#define RB_SCORE_WIN           7
#define RB_ENERGY_CAP          7
#define RB_MAX_CARD_IDS        4096
#define RB_EMPTY_SLOT          (-1)
#define RB_MAX_USED            256

static inline uint8_t rb_saturate_u8(int v) {
    if (v < 0) return 0;
    if (v > 255) return 255;
    return (uint8_t)v;
}
static inline int16_t rb_saturate_i16(int v) {
    if (v < -32768) return -32768;
    if (v > 32767) return 32767;
    return (int16_t)v;
}

/* Forward decl for condition eval (GameState defined below) */
struct GameState;
int  rb_eval_condition(const struct GameState *g, int actor, const Condition *c);
int  rb_eval_condition_for_host(const struct GameState *g, int actor, int host_cid, const Condition *c);

/* ── Modifiers (engine/src/core/game_modifiers.rs) ── */
typedef struct { int16_t set; int16_t add; } RbModifierEntry;
static inline int rb_modifier_total(RbModifierEntry e) { return (int)e.set + (int)e.add; }

/* Trace record — mirrors Rust AbilityApplication pushed by the *_with_trace
   modifier helpers (engine/src/core/game_modifiers.rs). Used for snapshot
   attribution; the portable core keeps a bounded ring (no consumer yet). */
#define RB_MODS_TRACE_CAP 64
#define RB_MODS_TRACE_TEXT 128
typedef enum {
    RB_EFFECT_BLADE_BONUS = 0,
    RB_EFFECT_HEART_BONUS = 1,
    RB_EFFECT_SCORE_BONUS,
    RB_EFFECT_SCORE_SET,
    RB_EFFECT_TRANSFORM,
    RB_EFFECT_NEED_HEART_MOD,
    RB_EFFECT_HEART_OVERRIDE,
    RB_EFFECT_COST_BONUS,
    RB_EFFECT_COST_SET,
    RB_EFFECT_BLADE_SET,
    RB_EFFECT_BLADE_TYPE_SET
} RbEffectType;
typedef struct {
    int16_t source_card_id;
    int16_t target_card_id;
    int16_t amount;
    int8_t  effect_type;   /* RbEffectType */
    int8_t  heart_color;   /* -1 = none */
    char    ability_text[RB_MODS_TRACE_TEXT];
} RbAbilityTraceEntry;

typedef struct {
    RbModifierEntry blade[RB_MAX_CARD_IDS];
    RbModifierEntry heart[RB_MAX_CARD_IDS][8];      /* per-color (0..7) */
    RbModifierEntry need_heart[RB_MAX_CARD_IDS][8];
    RbModifierEntry score[RB_MAX_CARD_IDS];
    RbModifierEntry cost[RB_MAX_CARD_IDS];
    uint8_t         orientation[RB_MAX_CARD_IDS]; /* 0 none, 1 active, 2 wait */
    uint8_t         delayed_cannot_active[RB_MAX_CARD_IDS]; /* remaining turns */
    /* constant-derived attribution (cleared on recalc) */
    int16_t         constant_blade[RB_MAX_CARD_IDS];
int16_t         constant_score[RB_MAX_CARD_IDS];
int16_t         constant_cost[RB_MAX_CARD_IDS];
/* Success-zone bonus ledgers — Rust GameModifiers (game_modifiers.rs:244/247/250).
 * These are what makes E1 `evaluate_success_zone_constant_modifiers`
 * (modifiers.rs:2188) portable: its withdraw-then-reapply ledger needs to know
 * what it previously contributed, and without these it cannot.
 *
 * CORRECTION 2026-10-04 (field census). The previous text here read "They are
 * NOT cosmetic: Rust CLEARS and re-applies them on every live
 * (modifiers.rs:2194-2212), so in C a previous live's success-zone bonus can leak
 * into the next one." The leak warning is TRUE; the framing around it was false,
 * because it read as though the ledgers were live here. They are NOT: all three
 * are 0 writers / 0 readers in the C. Rust's live use is exact and local to ONE
 * function that has no C twin at all —
 * `evaluate_success_zone_constant_modifiers` takes each ledger with
 * `core::mem::take` (modifiers.rs:2200 / :2204 / :2211) and immediately withdraws
 * what it finds through `remove_blade_modifier` / `remove_heart_modifier` /
 * `remove_score_modifier` (:2201-2214). Outside that function the only other
 * mention of the three is a `log::debug!` inequality test (:2293-2302), so the
 * ledgers are load-bearing *for E1* and for nothing else.
 * Reachability: 214 constant effect nodes on member cards carry one of E1's four
 * actions. Mirrors `constant_cost` in shape. APPENDED, never inserted.
 * BLOCKED-ON: src/core/modifiers.c — port
 * `evaluate_success_zone_constant_modifiers` (Rust modifiers.rs:2188-2305) with
 * this take/withdraw/reapply prologue, and call it at the tail of
 * `rb_recalc_constants` (the modifiers.rs:1398 position). Until that lands these
 * three arrays stay 0W/0R and E1/E2 stay blocked, exactly as
 * PARITY_CHECKLIST.md items E1/E2 record. The header half of E1's blocker is
 * therefore ALREADY DONE — the stale text above used to ask for these lines. */
int16_t         success_zone_blade_bonuses[RB_MAX_CARD_IDS];
int16_t         success_zone_heart_bonuses[RB_MAX_CARD_IDS][8];
int16_t         success_zone_score_bonuses[RB_MAX_CARD_IDS];
    int16_t         constant_heart[RB_MAX_CARD_IDS][8];  /* per-color, cleared on recalc */
    int16_t         constant_need_heart[RB_MAX_CARD_IDS][8];
    /* Per-player live TOTAL score bonus (Rust game_modifiers.rs
       p1/p2_constant_total_score_bonus). Constant scan with target="live_total"
       and runtime execute_modify_score live_total both feed these; consumed by
       live score (rb_live_calculate_score / allocate_and_verdict). */
    int16_t         p1_constant_total_score_bonus;
    int16_t         p2_constant_total_score_bonus;
    int16_t         heart_copy[RB_MAX_CARD_IDS];   /* target→source */
    int8_t          heart_multiplier[RB_MAX_CARD_IDS]; /* -1 none, else colour 0..7 */
    int8_t          heart_multiplier_amt[RB_MAX_CARD_IDS]; /* multiplier applied to that colour (default 2) */
    int8_t          blade_type[RB_MAX_CARD_IDS];   /* -1 none, else BladeColor idx */
    int8_t          heart_color_override[RB_MAX_CARD_IDS]; /* -1 none (specify_heart_color); else all base hearts counted as this colour */
    uint8_t         heart_override_count[RB_MAX_CARD_IDS];
    int             last_cost_discard_count;       /* cards discarded as part of the last cost payment */
    int             last_cost_moved_card_ids[8]; /* ids moved for the last cost (mirrors Rust mods.last_cost_moved_card_ids) */
    int             n_last_cost_moved_card_ids;
    /* last_under_move_host_ids — mirrors game_modifiers.rs last_under_move_host_ids
        (SmallVec<[i16;4]>). Stage member ids that hosted the cards pulled by the
        most recent move_from_under_member, so a following 「そうした場合、そのメンバーは…」
        gain step can target them specifically (resolve_gain_resource_targets). */
    int16_t         last_under_move_host_ids[4];
    int             n_last_under_move_host_ids;
    /* snapshot-trace ring (mirrors AbilityApplication buffer of *_with_trace) */
    RbAbilityTraceEntry trace[RB_MODS_TRACE_CAP];
    int             n_trace;
    /* Rust game_modifiers.rs:227: SmallVec<[u64;4]>. Keys fold in
     * movement_event_counter so they are PER-MOVE, not per-card; pushed at
     * abilities.rs:1159-1162, cleared at modifiers.rs:2018. C's
     * batch_triggered_keys is a different, weaker scope. */
    uint64_t opp_cause_fired_keys[16];
    int n_opp_cause_fired_keys;

} RbMods;

void rb_mods_init(RbMods *m);
void rb_mods_clear_card(RbMods *m, int card_id);
int  rb_mods_get_blade(RbMods *m, int card_id);
void rb_mods_add_blade(RbMods *m, int card_id, int delta);
void rb_mods_set_blade(RbMods *m, int card_id, int value);
int  rb_mods_get_heart(RbMods *m, int card_id, int color);
void rb_mods_add_heart(RbMods *m, int card_id, int color, int delta);
int  rb_mods_get_need_heart(RbMods *m, int card_id, int color);
void rb_mods_add_need_heart(RbMods *m, int card_id, int color, int delta);
void rb_mods_set_need_heart(RbMods *m, int card_id, int color, int value);
int  rb_mods_get_score(RbMods *m, int card_id);
void rb_mods_add_score(RbMods *m, int card_id, int delta);
void rb_mods_set_score(RbMods *m, int card_id, int value);
int  rb_mods_get_cost(RbMods *m, int card_id);
void rb_mods_add_cost(RbMods *m, int card_id, int delta);
void rb_mods_set_cost(RbMods *m, int card_id, int value);
const char *rb_mods_get_orientation(RbMods *m, int card_id);
void rb_mods_set_orientation(RbMods *m, int card_id, const char *s);
int  rb_mods_is_delayed_cannot_active(RbMods *m, int card_id);
void rb_mods_add_delayed_cannot_active(RbMods *m, int card_id, uint8_t turns);
void rb_mods_tick_delayed_for(RbMods *m, const int *owned, int n_owned);
/* set-override accessors (mirror get_*_set_modifier / get_cost_modifier_set) */
int  rb_mods_get_blade_set(RbMods *m, int card_id);
void rb_mods_clear_blade_set(RbMods *m, int card_id);
int  rb_mods_get_score_set(RbMods *m, int card_id);
void rb_mods_clear_score_set(RbMods *m, int card_id);
int  rb_mods_get_cost_set(RbMods *m, int card_id);
void rb_mods_clear_cost_set(RbMods *m, int card_id);
/* remove a previously-added delta (mirror remove_*_modifier) */
void rb_mods_remove_blade(RbMods *m, int card_id, int delta);
void rb_mods_remove_heart(RbMods *m, int card_id, int color, int delta);
/* rb_mods_remove_need_heart was DELETED 2026-10-03. Rust has NO
 * remove_need_heart_modifier anywhere in engine/src (0 hits); the faithful form
 * is rb_mods_add_need_heart(cid, color, -delta) per game_state/modifiers.rs:615-619.
 * Do NOT re-add this declaration - there is no definition. */
void rb_mods_remove_score(RbMods *m, int card_id, int delta);
void rb_mods_remove_cost(RbMods *m, int card_id, int delta);
/* heart_override / heart_copy / blade_type / heart_color_multiplier
   (mirror GameModifiers set/get/clear accessors for those fields) */
void rb_mods_set_heart_override(RbMods *m, int card_id, int color, int count);
void rb_mods_remove_heart_override(RbMods *m, int card_id);
int  rb_mods_get_heart_override(RbMods *m, int card_id, int *out_count);
void rb_mods_set_heart_copy(RbMods *m, int target_card_id, int source_card_id);
int  rb_mods_get_heart_copy(RbMods *m, int target_card_id);
void rb_mods_set_blade_type(RbMods *m, int card_id, int color);
void rb_mods_clear_blade_type(RbMods *m, int card_id);
int  rb_mods_get_blade_type(RbMods *m, int card_id);
void rb_mods_set_heart_color_multiplier(RbMods *m, int card_id, int color);
int  rb_mods_get_heart_color_multiplier(RbMods *m, int card_id);
/* *_with_trace — mirror add_*_modifier_with_trace (push an AbilityApplication
   onto the bounded snapshot-trace ring in addition to applying the modifier). */
void rb_mods_add_blade_with_trace(RbMods *m, int card_id, int delta,
                                  int source_card_id, const char *ability_text);
void rb_mods_add_heart_with_trace(RbMods *m, int card_id, int color, int delta,
                                  int source_card_id, const char *ability_text);
int  rb_mods_trace_len(const RbMods *m);
void rb_mods_trace_push(RbMods *m, int source_card_id, const char *ability_text,
                        int effect_type, int target_card_id, int heart_color, int amount);

typedef struct { uint8_t slot; int32_t delta; } RbYellMod;

/* ── Card (decoded from cards.bin) ── */
#define RB_MAX_HEARTS 64
typedef struct {
    uint16_t card_no_idx, name_idx, series_idx, group_idx;
    uint16_t unit_idx, img_idx, product_idx, rare_idx, ability_idx;
    uint8_t  type_flags;
    uint8_t  cost, blade, score, num_base, num_blade, num_need;
    int      has_special;
    uint8_t  special_color, special_count;
    uint8_t  heart_color[RB_MAX_HEARTS];
    uint8_t  heart_count[RB_MAX_HEARTS];
    int      n_hearts;
    char    *name;
    Ability *ability;   /* nullable, owned */
} Card;

/* ── Lifecycle ── */
int  rb_load(const char *data_dir);   /* load cards.bin + abilities_strings.bin */
int  rb_load_streaming(const char *dir,
                       unsigned char *(*read_fn)(const char *path, long *out_len)); /* alt I/O */
int  rb_load_gen_data(const unsigned char *buf, long len); /* populate offset tables from storage */
#ifdef RB_ROM_STRINGS
/* ROM-embedded build: cards.bin / abilities_strings.bin live in ROM; the
   caller passes their addresses and rb_load_rom() builds pointer tables that
   index directly into the blobs (no per-string copy). */
int  rb_load_rom(const unsigned char *cards_blob, long cards_len,
                 const unsigned char *abstr_blob, long abstr_len);
#endif
void rb_unload(void);
uint32_t rb_num_cards(void);
uint32_t rb_num_abilities(void);

/* ── String table (abilities) ── */
const char *rb_get_string(uint32_t idx);
const char *get_string(uint32_t idx);
const unsigned char *bytecode_slice(uint8_t ci, uint32_t start, uint32_t len);

typedef struct { uint8_t chunk; uint16_t start; uint16_t len; } AbilityLoc;
extern const AbilityLoc ABILITY_LOCS[];

extern const uint32_t NUM_ABILITIES;
extern const uint16_t *OFFSET_DELTAS;
extern const uint32_t *STRINGS_OFFSETS;
extern const uint16_t *CARD_ABILITY_PAIRS;
extern const unsigned char *BYTECODE;
extern const uint32_t BYTECODE_LEN;
extern const uint32_t DECOMPRESSED_LEN;

/* ── Decode-fallback audit (mirrors vm.rs DECODE_FALLBACKS) ── */
#define RB_DECODE_AUDIT_MAX 4096
void rb_note_decode_fallback(int ability, const char *field, const char *value);
uint32_t rb_decode_fallback_count(void);
int rb_decode_fallback_abilities(uint32_t *out, int max);

/* ── Decode ── */
int  rb_decode_ability(uint32_t idx, Ability *out);     /* returns 1 on success */
int  rb_get_ability(uint32_t idx, Ability *out);         /* returns 1 even for empty/default */
int  rb_count_empty_bytecode_abilities(void);            /* audit: empty slices */
void rb_free_ability(Ability *a);
void rb_effect_free(AbilityEffect *e);
AbilityEffect *rb_effect_deep_clone(const AbilityEffect *e);
void rb_free_condition(Condition *c);

/* ── Trigger system (mirrors engine/src/triggers.rs) ── */
typedef enum {
    RB_TK_ACTIVATION = 0, RB_TK_AUTO, RB_TK_CONSTANT, RB_TK_DEBUT,
    RB_TK_LIVE_START, RB_TK_LIVE_SUCCESS, RB_TK_MAIN, RB_TK_BATON_TOUCH, RB_TK_COUNT
} RbTriggerKind;

/* Trigger string constants (engine/src/triggers.rs) */
#define RB_TSTR_ACTIVATION    "起動"
#define RB_TSTR_AUTO          "自動"
#define RB_TSTR_CONSTANT      "常時"
#define RB_TSTR_DEBUT         "登場"
#define RB_TSTR_LIVE_START    "ライブ開始時"
#define RB_TSTR_LIVE_SUCCESS  "ライブ成功時"
#define RB_TSTR_MAIN          "メイン"
#define RB_TSTR_BATON_TOUCH   "baton touch"
#define RB_TSTR_DEBUT_EN      "Debut"
#define RB_TSTR_LIVE_SUCCESS_EN "live_success"

RbTriggerKind rb_trigger_from_token(const char *s);
int rb_parse_triggers(const char *triggers, RbTriggerKind *out, int max);
const char *rb_canonical_trigger(const char *raw);
const char *rb_trigger_zone_id(int is_stage);
const char *rb_trigger_zone_label(int is_stage);
/* rb_trigger_to_texticon WAS HERE and was DELETED 2026-10-05, together with
 * its definition in src/turn/triggers.c. Its only caller engine-wide is
 * Rust game/display.rs:1569 and :1599 (the `bonus_triggers` badge map), and
 * the C has no display layer, so it had zero reachable callers here. The
 * tombstone and the full deleted body are at src/turn/triggers.c, which is
 * where the Rust citation (triggers.rs:139-149) is recorded. */
int rb_ability_has_trigger(const Ability *a, RbTriggerKind kind);
const char *rb_ability_triggerless_text(const Ability *a);
const char *rb_card_short_label(int card_id);
typedef enum {
    RB_KW_TURN1 = 0, RB_KW_TURN2, RB_KW_DEBUT, RB_KW_LIVE_START,
    RB_KW_LIVE_SUCCESS, RB_KW_CENTER, RB_KW_LEFT_SIDE, RB_KW_RIGHT_SIDE,
    RB_KW_POSITION_CHANGE, RB_KW_FORMATION_CHANGE, RB_KW_COUNT
} RbKeyword;
RbKeyword rb_keyword_from_str(const char *s);
int rb_decode_keywords(const unsigned char *arr, uint32_t arr_len, RbKeyword *out, int max);
int rb_decode_card_by_index(uint32_t i, Card *out);    /* 0..num_cards-1 */
void rb_free_card(Card *c);
int rb_card_get_card_by_id(int card_id, Card *out);
int rb_card_get_card_by_no(const char *card_no, Card *out);
int rb_card_get_card(const char *card_no);
int rb_card_get_card_names(int card_id, char *out, size_t out_sz);
int rb_card_database_create_copy(int template_id);
uint16_t rb_card_ability_idx(uint32_t i);   /* 0xFFFF if none  Efirst ability only (legacy) */
const unsigned char *rb_card_record(uint32_t i);
uint32_t rb_card_record_len(uint32_t i);
const unsigned char *rb_bc_slice(uint32_t idx, uint32_t *out_len);
const char *rb_card_string(uint16_t idx);
int rb_find_card_by_no(const char *card_no); /* linear scan cards.bin card_no strings, -1 if not found */
int rb_card_get_card_id(const char *card_no);
int rb_register_card_copy(int copy_id, int template_id);
int rb_create_card_copy(int template_id);
void rb_card_copy_pool_reset(void);
/* Multi-ability support  Ecards can have 1..N abilities (e.g. hanayo debut+constant).
   The pairs table RBKA_CARD_ABILITY_PAIRS maps card_no string idx ↁEability idx.
   Use these to iterate all abilities for a card (mirrors Rust Card.abilities:Vec). */
extern const uint16_t RBKA_CARD_ABILITY_PAIRS[];
int rb_card_num_abilities(uint32_t card_idx); /* count of abilities for card */
int rb_card_get_ability_idx(uint32_t card_idx, int n, uint32_t *out_ability_idx); /* nth ability idx */
int rb_decode_card_ability(uint32_t card_idx, int n, Ability *out); /* decode nth ability */

/* ════════════════════════════════════════════════════════════════════
    Engine  Egame state + turn loop + faithful effect execution.
    The decoder (above) is byte-identical to the Rust VM. The execution
    below is a real, working port of the core rules (constants.rs /
    phases.rs / actions.rs): zones, a 3-position stage, energy, the
    Live/performance heart loop, and a broad action-verb dispatch.
    Host I/O (fopen) lives only in data.c:rb_load; bare-metal ports
    provide rb_load_streaming with a custom read_fn and compile with
    -DRB_NO_MALLOC (bump arena in src/alloc.c).
    ════════════════════════════════════════════════════════════════════ */

typedef enum {
    RB_ZONE_HAND = 0,
    RB_ZONE_DECK,
    RB_ZONE_STAGE,
    RB_ZONE_DISCARD,    /* waitroom */
    RB_ZONE_ENERGY,
    RB_ZONE_LIVE,       /* live card zone */
    RB_ZONE_SUCCESS,    /* success live card zone */
    RB_ZONE_RESOLUTION
} RbZone;

/* Compact zone identifier (mirrors engine/src/core/types.rs::ZoneId).
     Replaces free-form zone strings in movement/position events so the engine
     can compare, alias, and serialize zones without heap-allocated strings.
     The wire names match the Rust as_str() mapping (stage/hand/deck/...). */
typedef enum {
    RB_ZONEID_STAGE = 0,
    RB_ZONEID_HAND,
    RB_ZONEID_DECK,
    RB_ZONEID_DECK_TOP,
    RB_ZONEID_DECK_BOTTOM,
    RB_ZONEID_DISCARD,
    RB_ZONEID_WAITROOM,
    RB_ZONEID_ENERGY,
    RB_ZONEID_ENERGY_ZONE,
    RB_ZONEID_ENERGY_DECK,
    RB_ZONEID_SUCCESS_ZONE,
    RB_ZONEID_LIVE_CARD_ZONE,
    RB_ZONEID_SUCCESS_LIVE_ZONE,
    RB_ZONEID_EMPTY_AREA,
    RB_ZONEID_SAME_AREA,
    RB_ZONEID_UNDER_MEMBER,
    RB_ZONEID_LOOKED_AT,
    RB_ZONEID_REVEALED_CARDS,
    RB_ZONEID_SELECTED_CARDS,
    RB_ZONEID_RESOLUTION,
    RB_ZONEID_EXCLUSION_ZONE,
    RB_ZONEID_DECK_TOP_OR_BOTTOM,
    RB_ZONEID_LOOKED_AT_REMAINING,
    RB_ZONEID_THOSE_CARDS,
    RB_ZONEID_PRECEDING_MOVED,
    RB_ZONEID_RECENTLY_MOVED,
    RB_ZONEID_FRONT,
    RB_ZONEID_LIVE_TOTAL,
    RB_ZONEID_UNKNOWN
} RbZoneId;

/* Ability zone identifier (mirrors engine/src/core/ability/enums.rs::Zone).
   Used only for the from_ability_zone / to_ability_zone conversion functions.
   Discriminant order matches the Rust enum exactly. */
typedef enum {
    RB_ABILITY_ZONE_HAND = 0,
    RB_ABILITY_ZONE_STAGE,
    RB_ABILITY_ZONE_STAGE_CENTER,
    RB_ABILITY_ZONE_STAGE_LEFT,
    RB_ABILITY_ZONE_STAGE_RIGHT,
    RB_ABILITY_ZONE_DISCARD,
    RB_ABILITY_ZONE_WAITROOM,
    RB_ABILITY_ZONE_ENERGY,
    RB_ABILITY_ZONE_ENERGY_ZONE,
    RB_ABILITY_ZONE_DECK,
    RB_ABILITY_ZONE_DECK_TOP,
    RB_ABILITY_ZONE_DECK_BOTTOM,
    RB_ABILITY_ZONE_SUCCESS_ZONE,
    RB_ABILITY_ZONE_LIVE_CARD_ZONE,
    RB_ABILITY_ZONE_SUCCESS_LIVE_ZONE,
    RB_ABILITY_ZONE_ENERGY_DECK,
    RB_ABILITY_ZONE_EMPTY_AREA,
    RB_ABILITY_ZONE_SAME_AREA,
    RB_ABILITY_ZONE_UNDER_MEMBER,
    RB_ABILITY_ZONE_LOOKED_AT,
    RB_ABILITY_ZONE_REVEALED_CARDS,
    RB_ABILITY_ZONE_SELECTED_CARDS,
    RB_ABILITY_ZONE_RESOLUTION,
    RB_ABILITY_ZONE_EXCLUSION_ZONE,
    RB_ABILITY_ZONE_PRECEDING_MOVED,
    RB_ABILITY_ZONE_RECENTLY_MOVED,
    RB_ABILITY_ZONE_THOSE_CARDS,
    RB_ABILITY_ZONE_LOOKED_AT_REMAINING,
    RB_ABILITY_ZONE_DECK_TOP_OR_BOTTOM,
    RB_ABILITY_ZONE_FRONT,
    RB_ABILITY_ZONE_UNKNOWN,
    RB_ABILITY_ZONE_LIVE_TOTAL
} RbAbilityZone;

/* ── ability/enums.rs: typed ability enums ── */
typedef enum {
    RB_TARGET_SELF = 0,
    RB_TARGET_OPPONENT,
    RB_TARGET_BOTH,
    RB_TARGET_EITHER
} RbTargetPlayer;

typedef enum {
    RB_PLACEMENT_FILTER_SELF_AS_SOURCE = 0,
    RB_PLACEMENT_UNDER_THIS_MEMBER,
    RB_PLACEMENT_UNDER_CHOSEN_MEMBER
} RbPlacementTarget;

typedef enum {
    RB_ACTION_DRAW_CARD = 0,
    RB_ACTION_DRAW_UNTIL_COUNT,
    RB_ACTION_MOVE_CARDS,
    RB_ACTION_DISCARD_CARD,
    RB_ACTION_SELECT,
    RB_ACTION_SELECT_NUMBER,
    RB_ACTION_SELECT_CARDS,
    RB_ACTION_LOOK_AND_SELECT,
    RB_ACTION_LOOK_AT,
    RB_ACTION_REVEAL,
    RB_ACTION_REVEAL_PER_GROUP,
    RB_ACTION_REVEAL_UNTIL_LIVE_CARD,
    RB_ACTION_REVEAL_UNTIL_CHOSEN_CARD,
    RB_ACTION_CHANGE_STATE,
    RB_ACTION_POSITION_CHANGE,
    RB_ACTION_ROTATION,
    RB_ACTION_PLACE_ENERGY_UNDER_MEMBER,
    RB_ACTION_MODIFY_REQUIRED_HEARTS_SUCCESS,
    RB_ACTION_GAIN_RESOURCE,
    RB_ACTION_PAY_ENERGY,
    RB_ACTION_GAIN_ABILITY,
    RB_ACTION_GAIN_ABILITY_FROM_SOURCE,
    RB_ACTION_INVALIDATE_ABILITY,
    RB_ACTION_SUPPRESS_ABILITY_TRIGGER,
    RB_ACTION_ACTIVATE_ABILITY,
    RB_ACTION_MODIFY_COST,
    RB_ACTION_MODIFY_YELL_SOURCE,
    RB_ACTION_SET_COST,
    RB_ACTION_SET_CARD_IDENTITY,
    RB_ACTION_SET_COST_TO_USE,
    RB_ACTION_MODIFY_SCORE,
    RB_ACTION_MODIFY_REQUIRED_HEARTS,
    RB_ACTION_SET_BLADE_TYPE,
    RB_ACTION_SET_BLADE_COUNT,
    RB_ACTION_SET_HEART_TYPE,
    RB_ACTION_SPECIFY_HEART_COLOR,
    RB_ACTION_CHOOSE_REQUIRED_HEARTS,
    RB_ACTION_SEQUENTIAL,
    RB_ACTION_CONDITIONAL_ALTERNATIVE,
    RB_ACTION_CONDITIONAL_ON_RESULT,
    RB_ACTION_CONDITIONAL_ON_OPTIONAL,
    RB_ACTION_RESTRICTION,
    RB_ACTION_ACTIVATION_RESTRICTION,
    RB_ACTION_MODIFY_LIMIT,
    RB_ACTION_SHUFFLE,
    RB_ACTION_RE_YELL,
    RB_ACTION_CUSTOM,
    RB_ACTION_DO_NOTHING,
    RB_ACTION_CHOICE,
    RB_ACTION_REPEAT_PROCEDURE,
    RB_ACTION_DISCARD_UNTIL_COUNT,
    RB_ACTION_ALL_BLADE_TIMING,
    RB_ACTION_REDUCE_LIVE_CARD_SET_LIMIT,
    RB_ACTION_CHOOSE_TARGET_PLAYER,
    RB_ACTION_PLAY_BATON_TOUCH,
    RB_ACTION_MODIFY_REQUIRED_HEARTS_GLOBAL,
    RB_ACTION_MODIFY_YELL_COUNT,
    RB_ACTION_ACTIVATION_COST,
    RB_ACTION_PERFORM_YELL,
    RB_ACTION_CONDITIONAL_OPTIONAL,
    RB_ACTION_COMPOUND_ACTION,
    RB_ACTION_OPPONENT_ACTION,
    RB_ACTION_ACTION_BY,
    RB_ACTION_SEQUENTIAL_COST,
    RB_ACTION_CHOICE_CONDITION,
    RB_ACTION_ENERGY_CONDITION,
    /* Appended, so nothing above renumbers. RbGeneratedAction.action_type is
     * only ever compared symbolically against these constants, never persisted
     * or serialised, so the new ordinals are safe.
     * engine/src/ability/enums.rs:332 — a REPLACEMENT rule resolved once at
     * placement by GameState::replacement_appear_wait_applies (turn/phases.rs:
     * 1281), NOT a dispatch arm: dispatch.rs:126 is `=> Ok(())`. */
    RB_ACTION_REPLACEMENT_APPEAR_WAIT,
    /* engine/src/ability/enums.rs:340 — a counting WEIGHT consumed by
     * GameState::weighted_zone_count (core/game_state/modifiers.rs:259, via
     * condition/card.rs:2124), not a dispatch arm: dispatch.rs:129 is `=> Ok(())`. */
    RB_ACTION_COUNT_AS_MULTIPLE
} RbActionType;

typedef enum {
    RB_CONDTYPE_COMPOUND = 0,
    RB_CONDTYPE_COMPARISON,
    RB_CONDTYPE_LOCATION,
    RB_CONDTYPE_CARD_COUNT,
    RB_CONDTYPE_CARD_BLADE,
    RB_CONDTYPE_GROUP,
    RB_CONDTYPE_POSITION,
    RB_CONDTYPE_APPEARANCE,
    RB_CONDTYPE_TEMPORAL,
    RB_CONDTYPE_STATE,
    RB_CONDTYPE_ENERGY_STATE,
    RB_CONDTYPE_MOVEMENT,
    RB_CONDTYPE_ABILITY_FILTER,
    RB_CONDTYPE_OR,
    RB_CONDTYPE_ANY_OF,
    RB_CONDTYPE_SCORE_THRESHOLD,
    RB_CONDTYPE_CHOICE,
    RB_CONDTYPE_POSITION_CHANGE,
    RB_CONDTYPE_STATE_CHANGE,
    RB_CONDTYPE_OPPONENT_CHOICE,
    RB_CONDTYPE_OPPONENT_LIVE_SUCCESS,
    RB_CONDTYPE_COMPLEX,
    RB_CONDTYPE_NO_EXCESS_HEART,
    RB_CONDTYPE_OTHERWISE,
    RB_CONDTYPE_NOT_MOVED,
    RB_CONDTYPE_HAS_MOVED,
    RB_CONDTYPE_RESOURCE,
    RB_CONDTYPE_ACTION_SUCCESS,
    RB_CONDTYPE_ALL_COST_COMPARISON,
    RB_CONDTYPE_HIGHEST_COST_ON_STAGE,
    RB_CONDTYPE_BOTH,
    RB_CONDTYPE_ALL_REVEALED_MATCH_HEART_COLOR,
    RB_CONDTYPE_CUSTOM
} RbConditionType;

typedef enum {
    RB_STK_CHOICE = 0,
    RB_STK_CHOICE_STRING,
    RB_STK_PAY_OPTIONAL_COST_SKIP_OPTIONAL_COST,
    RB_STK_DOUBLE_BATON_TOUCH,
    RB_STK_PRIMARY_ALTERNATIVE,
    RB_STK_APPLY_REPLACEMENT,
    RB_STK_CHOOSE_REQUIRED_HEARTS,
    RB_STK_POSITION_DESTINATION,
    RB_STK_HEART_COLOR,
    RB_STK_CHOICE_TYPE,
    RB_STK_CHOICE_CONDITION,
    RB_STK_CONDITIONAL_OPTIONAL,
    RB_STK_DRAW_ANY_NUMBER,
    RB_STK_ORDER,
    RB_STK_SELF_OR_OPPONENT,
    RB_STK_PAY_COST_ALL_DISCARD
} RbSelectTargetKind;

typedef enum {
    RB_ECT_MEMBER_CARD = 0,
    RB_ECT_LIVE_CARD,
    RB_ECT_ENERGY_CARD,
    RB_ECT_OTHER
} RbEffectCardType;

typedef enum {
    RB_ES_ACTIVE = 0,
    RB_ES_WAIT,
    RB_ES_OTHER
} RbEffectState;

/* TargetPlayer helpers */
int rb_target_player_from_str(const char *s);
const char *rb_target_player_to_str(int tp);
const char *rb_target_player_as_str(int tp);

/* ActionType helpers */
int rb_action_type_from_str(const char *s);
const char *rb_action_type_to_str(int at);
const char *rb_action_type_label(int at);
int rb_action_type_default(void);

/* ConditionType helpers */
int rb_condition_type_from_str(const char *s);
const char *rb_condition_type_to_str(int ct);

/* SelectTargetKind helpers */
int rb_select_target_kind_from_str(const char *s);
const char *rb_select_target_kind_to_str(int stk);

/* EffectCardType helpers */
int rb_effect_card_type_from_str(const char *s);
const char *rb_effect_card_type_as_str(int ect);
RbEffectCardType rb_effect_card_type_default(void);

/* EffectState helpers */
int rb_effect_state_from_str(const char *s);
const char *rb_effect_state_as_str(int es);
RbEffectState rb_effect_state_default(void);

/* AbilityZone helpers (RbAbilityZone enum declared above) */
int rb_ability_zone_from_str(const char *s);
const char *rb_ability_zone_to_str(int z);
const char *rb_ability_zone_as_str(int z);
int rb_ability_zone_from_source_str(const char *s);

/* A zone is just a bag of card indices (card_no index into the database). */
typedef struct {
    int cards[RB_MAX_ZONE];
    int n;
} RbBag;

/* A hand is just a bag of card indices (mirrors engine/src/core/player.rs
    Player::hand: Hand — the C engine models every card collection as an RbBag,
    so RbHand is a named alias for readability at call sites). */
typedef RbBag RbHand;

typedef struct {
    RbBag     hand;
    RbBag     deck;
    int       stage[RB_STAGE_SIZE];   /* card_no index or -1 */
    int       stage_wait[RB_STAGE_SIZE]; /* 1 if member is in "wait" state */
    RbBag     under_cards[RB_STAGE_SIZE]; /* cards placed under each stage member */
    RbBag     energy;                 /* energy cards in energy zone */
    RbBag     energy_deck;            /* energy deck (draw pile for energy) */
    int       energy_active;          /* count of active energy */
    RbBag     live;                   /* live card zone */
    RbBag     success;                /* success live card zone */
    RbBag     discard;                /* waitroom */
    int       deck_refreshed_this_turn; /* Rule 10.2.2.1 mid-effect refresh flag */
    int       score;
    int       life;                   /* life points (HP)  Etracked for parity with Rust engine */
    int       hearts[RB_MAX_HEARTS]; /* hearts-by-color on this player */
    int       yell_note_icons;        /* hearts produced during performance */
    int       yell_from_bottom;       /* G8: yell reveal from deck bottom (tracking.rs) */
} RbPlayer;

typedef enum {
    RB_PHASE_RPS = 0,
    RB_PHASE_OPENING,     /* draw opening hands + mulligan */
    RB_PHASE_ACTIVE,      /* activate wait members */
    RB_PHASE_ENERGY,      /* draw 1 energy card */
    RB_PHASE_DRAW,        /* draw 1 card */
    RB_PHASE_MAIN,        /* play cards / activate abilities */
    RB_PHASE_LIVE_SET,    /* choose live cards */
    RB_PHASE_PERFORMANCE, /* yell + heart resolution */
    RB_PHASE_VICTORY,     /* success determination + turn rollover */
    RB_PHASE_DONE,
    RB_PHASE_MULLIGAN_FIRST,
    RB_PHASE_MULLIGAN_SECOND,
    RB_PHASE_LIVE_SET_SECOND,
    RB_PHASE_PERFORMANCE_SECOND
} RbPhase;

const char *rb_phase_name(int phase);
const char *rb_rps_choice_name(int choice);
void rb_push_rps_log(struct GameState *g, int p1, int p2, const char *winner_str);

/* ── Turn-phase grouping (mirrors engine/src/core/types.rs::TurnPhase) ──
    Used to classify the broad phase of a turn: first-attacker normal play,
    second-attacker normal play, or the Live/performance phase. */
typedef enum {
    RB_TURNP_NORMAL_FIRST = 0,
    RB_TURNP_NORMAL_SECOND,
    RB_TURNP_LIVE
} RbTurnPhase;

/* ── Game result (mirrors engine/src/core/types.rs::GameResult) ── */
typedef enum {
    RB_RESULT_FIRST_ATTACKER_WINS = 0,
    RB_RESULT_SECOND_ATTACKER_WINS,
    RB_RESULT_DRAW,
    RB_RESULT_ONGOING
} RbGameResult;

/* ── Ability trigger kinds (mirrors engine/src/core/types.rs::AbilityTrigger) ──
    The trigger verb is still carried as a string on Ability::triggers, but this
    enumeration gives the canonical set of trigger categories the engine knows. */
typedef enum {
    RB_TRIGGER_ACTIVATION = 0,
    RB_TRIGGER_DEBUT,
    RB_TRIGGER_LIVE_START,
    RB_TRIGGER_LIVE_SUCCESS,
    RB_TRIGGER_CONSTANT,
    RB_TRIGGER_AUTO
} RbAbilityTrigger;

/* ── Modifier/ability duration (mirrors engine/src/core/types.rs::Duration) ──
    Note: the resolver's temporary-effect expiry uses the RB_TEMP_* integer
    constants (see RbTempEffect::dur) which correspond to the first three
    variants; AsLongAs/Unless are conditions, not expiry kinds. */
typedef enum {
    RB_DURATION_LIVE_END = 0,
    RB_DURATION_THIS_TURN,
    RB_DURATION_THIS_LIVE,
    RB_DURATION_PERMANENT,
    RB_DURATION_AS_LONG_AS,
    RB_DURATION_UNLESS
} RbDuration;

/* ── Choice / ability queue (engine/src/ability/choice.rs + ability_queue.rs) ── */
typedef enum {
    RB_CHOICE_NONE = 0,
    RB_CHOICE_SELECT_CARD,
    RB_CHOICE_SELECT_TARGET, /* pay_skip / position|destination / double_baton etc. */
    RB_CHOICE_SELECT_HEART_COLOR,
    RB_CHOICE_SELECT_NUMBER,  /* ability/choice.rs select_number (count choice) */
    RB_CHOICE_SELECT_POSITION, /* stage position selection */
    RB_CHOICE_SELECT_AUTO_ABILITY /* auto-ability ordering */
} RbChoiceKind;

/* ChoiceRoute  Emirrors engine/src/ability/types.rs::ChoiceRoute. Tags which
   cost/choice gate produced a pending choice so resume can route correctly. */
typedef enum {
    RB_ROUTE_NONE = 0,
    RB_ROUTE_OPTIONAL_COST,   /* pay/skip an optional cost */
    RB_ROUTE_CHOICE_COST,     /* a ChoiceCondition cost option */
    RB_ROUTE_SELECT_CARDS,    /* select_cards / look_and_select */
    RB_ROUTE_SELECT_TARGET,   /* select_target (position/destination) */
    RB_ROUTE_CONDITIONAL_CHOICE
} RbChoiceRoute;

/* ── Full ChoiceRoute with all Rust variants (including Raw) ─────────── */
typedef enum {
    RB_ROUTEK_CHOICE      = 0,
    RB_ROUTEK_CHOICE_STRING,
    RB_ROUTEK_CHOICE_COST,
    RB_ROUTEK_OPTIONAL_COST,
    RB_ROUTEK_CHANGE_STATE,
    RB_ROUTEK_RAW
} RbChoiceRouteKind;

/* ── StageSelectIntent (mirrors engine/src/ability/types.rs::StageSelectIntent) */
typedef enum {
    RB_SSI_CHANGE_STATE_WAIT = 0,
    RB_SSI_UNDER_MEMBER_MOVE = 1,
    RB_SSI_COLLECT_TARGETS   = 2
} RbStageSelectIntent;

/* ── RbChoiceResultKind (mirrors engine/src/ability/types.rs::ChoiceResult) */
typedef enum {
    RB_CR_CARD_SELECTED     = 0,
    RB_CR_TARGET_SELECTED   = 1,
    RB_CR_POSITION_SELECTED = 2,
    RB_CR_HEART_COLOR       = 3,
    RB_CR_HEART_TYPE        = 4,
    RB_CR_AUTO_ABILITY      = 5,
    RB_CR_LIVE_SUCCESS      = 6,
    RB_CR_SKIP              = 7
} RbChoiceResultKind;

/* ── RbExecutionContextKind (mirrors engine/src/ability/types.rs::ExecutionContext) */
typedef enum {
    RB_EC_NONE                = 0,
    RB_EC_SINGLE_EFFECT       = 1,
    RB_EC_LOOK_AND_SELECT     = 2,
    RB_EC_MOVE_CARDS_POSITION = 3
} RbExecutionContextKind;

/* ── RbLookAndSelectStepKind (mirrors engine/src/ability/types.rs::LookAndSelectStep) */
typedef enum {
    RB_LAS_LOOK     = 0,
    RB_LAS_SELECT   = 1,
    RB_LAS_FINALIZE = 2
} RbLookAndSelectStepKind;

/* ── AbilityError codes — mirrors engine/src/ability/types.rs::AbilityError */
typedef enum {
    RB_AE_NO_MEMBER_IN_TARGET_AREA = 0,
    RB_AE_AREA_LOCKED,
    RB_AE_BATON_TOUCH_PROTECTION,
    RB_AE_INSUFFICIENT_ENERGY,
    RB_AE_INVALID_HAND_INDEX,
    RB_AE_NOT_MEMBER_CARD,
    RB_AE_CARD_NOT_FOUND,
    RB_AE_CANNOT_PLACE,
    RB_AE_GENERIC,
    RB_AE_ZONE_FULL,
    RB_AE_OTHER
} RbAbilityError;

/* QueueState  Emirrors engine/src/ability_queue.rs::QueueState FSM. */
typedef enum {
    RB_QUEUE_IDLE = 0,
    RB_QUEUE_RESOLVING,
    RB_QUEUE_PAYING_COST,
    RB_QUEUE_AWAITING_CHOICE,
    RB_QUEUE_DRAINING
} RbQueueState;

/* Forward declarations for types defined in choice.c */
typedef struct RbAbilityResolver RbAbilityResolver;
typedef struct RbSelectionContext RbSelectionContext;

/* ── Flat pending-choice struct (used by RbAbilityQueue.pending) ─────── */
typedef struct {
    RbChoiceKind kind;
    char zone[32];
    char card_type[32];
    int  count;
    int  allow_skip;
    char target[64];
    char description[512];
    char description_en[512];
    char description_ja[512];
    RbChoiceRoute route;
    char filter_group[32];
    int  filtered_indices[RB_MAX_ZONE];
    int  n_filtered_indices;
    int  filter_heart;
    char heart_options[8][24];
    int  n_heart_options;
    int  cost_limit;
    char cost_limit_op[8];
    int  cost_total;
    char cost_total_op[8];
    char characters[16][32];
    int  n_characters;
    char target_player_id[16];
    int  blind;
    int  is_reveal;
    int  actor;  /* 0=p1, 1=p2 - which player made the choice */
} RbChoice;

/* ── Queue entry (mirrors engine/src/ability/ability_queue.rs::AbilityQueueEntry) ── */
#define RB_ENTRY_PENDING_CAP 8
typedef struct {
    int card_id;
    int ability_idx;
    int cost_paid;
    int cost;  /* ability cost for pay_cost_all_discard etc. */
    int effect_started;
    int completed;
    int  optional_cost_result;
    int  cost_paid_index;
    int  choice_card_no;
    /* Rust's `entry.choice_card_no` is `Option<ChoiceRoute>`, and ChoiceRoute has a
       `Raw(String)` variant (types.rs:55-62) used for dynamic routes — e.g. a
       position_change route that carries the opponent target. An int tag cannot
       represent that string, so the raw text is kept here alongside the tag.
       queue.rs:340-345 matches `is_route_opponent` on this prefix:
           "position_change:opponent:"
       `has_choice_route_raw` distinguishes "no Raw route" from "empty Raw route". */
    char choice_route_raw[64];
    int  has_choice_route_raw;
    int  pending_actions_n;
    /* owned pending-action snapshots (mirrors Rust entry.pending_actions):
        deep-cloned AbilityEffect trees parked across a pause, executed by
        rb_queue_resume_pending_actions, freed on entry clear. */
    AbilityEffect *pending_actions[RB_ENTRY_PENDING_CAP];
    int  has_pending_choice;
    RbChoice pending_choice;   /* per-entry parked choice (Rust entry.pending_choice) */
    char spawn_target[64];     /* resolver spawn_context.target for this entry */
    int  has_resolver;         /* persistent resolver is attached to this entry */
    int  triggering_member_id;
    int  use_limit_recorded;
    int  optional_moves_all_moved; /* -1=None, 0=false, 1=true (mirrors Rust Option<bool>) */
    char trigger[32];
    int  trigger_moved_cards[16];
    int  n_trigger_moved_cards;
#define RB_COND_CACHE_CAP 8
    int  cond_cache_keys[RB_COND_CACHE_CAP];
    int  cond_cache_vals[RB_COND_CACHE_CAP];
    int  n_cond_cache;
    char player_id[16];
    char choice_player_id[16];
} RbQueueEntry;

/* ── TriggerEvent (engine/src/ability/types.rs::TriggerEvent) ────────── */
#define RB_TE_MAX_MOVED        16
#define RB_TE_MAX_APPEARED     16
#define RB_TE_ZONE_SZ          32
#define RB_TE_PLAYER_SZ        16
typedef struct {
    int   moved_cards[RB_TE_MAX_MOVED];
    int   n_moved_cards;
    char  moved_from_zone[RB_TE_ZONE_SZ];
    int   has_moved_from_zone;
    int   position_change_occurred;
    struct { int card_id; char source_zone[RB_TE_ZONE_SZ]; } appeared_cards[RB_TE_MAX_APPEARED];
    int   n_appeared_cards;
    int   energy_placed_by_effect;
    char  energy_placed_by_player[RB_TE_PLAYER_SZ];
    int   has_energy_placed_by_player;
} RbTriggerEvent;

/* ── EffectSpawnContext (engine/src/ability/types.rs::EffectSpawnContext) */
#define RB_ESC_SZ 64
typedef struct {
    char target[RB_ESC_SZ];
    char destination[RB_ESC_SZ];
    char source[RB_ESC_SZ];
    int  position;
} RbEffectSpawnContext;

/* ── StepOutput (engine/src/ability/types.rs::StepOutput) ───────────── */
#define RB_SO_MAX_CARDS 128
typedef struct {
    int  cards[RB_SO_MAX_CARDS];
    int  n_cards;
    int  has_value;
    int  value;
    int  has_accepted;
    int  accepted;
} RbStepOutput;

/* ── ValueRef (engine/src/ability/types.rs::ValueRef) ───────────────── */
typedef enum {
    RB_VR_LITERAL       = 0,
    RB_VR_STEP_VALUE    = 1,
    RB_VR_STEP_ACCEPTED = 2,
    RB_VR_STEP_OFFSET   = 3
} RbValueRefKind;
typedef struct {
    RbValueRefKind kind;
    int            literal_value;
    char           step_id[64];
    int            offset;
} RbValueRef;

/* ── ZoneSnapshot (engine/src/ability/types.rs::ZoneSnapshot) ────────── */
typedef struct {
    int hand_count;
    int stage_count;
    int waitroom_count;
    int energy_count;
    int active_energy_count;
    int deck_count;
} RbZoneSnapshot;

/* ── AbilityTraceNode (engine/src/ability/types.rs::AbilityTraceNode) ── */
#define RB_TRACE_MAX_CHILDREN 64
typedef struct RbAbilityTraceNode RbAbilityTraceNode;
struct RbAbilityTraceNode {
    char label[128];
    char card[64];
    int  has_card;
    RbZoneSnapshot before;
    int  has_before;
    RbZoneSnapshot after;
    int  has_after;
    RbAbilityTraceNode *children[RB_TRACE_MAX_CHILDREN];
    int  n_children;
};

/* ── EffectPipeline (engine/src/ability/types.rs::EffectPipeline) ────── */
typedef struct { RbAbilityTraceNode *trace; } RbEffectPipeline;

/* ── StepState (engine/src/ability/types.rs::StepState) ──────────────── */
#define RB_SS_MAX_RESULTS 256
typedef struct {
    char  step_id[64];
    RbStepOutput output;
    int  has_output;
} RbStepResultEntry;
typedef struct {
    RbStepResultEntry entries[RB_SS_MAX_RESULTS];
    int  n_entries;
    int  last_draw_count;
} RbStepState;

/* ── Full Choice enum (RbFullChoice mirrors engine/src/ability/types.rs::Choice) ── */
#define RB_MAX_CC_STRINGS   32
#define RB_MAX_CC_U8S       64
#define RB_MAX_CC_CARDS     64
typedef enum {
    RB_CC_SELECT_CARD        = 0,
    RB_CC_SELECT_TARGET      = 1,
    RB_CC_SELECT_POSITION    = 2,
    RB_CC_SELECT_HEART_COLOR = 3,
    RB_CC_SELECT_HEART_TYPE  = 4,
    RB_CC_SELECT_AUTO_ABILITY= 5,
    RB_CC_SELECT_LIVE_SUCCESS= 6
} RbFullChoiceKind;
typedef struct { uint8_t values[RB_MAX_CC_U8S]; int n_values; } RbFullChoiceU8Vec;
typedef struct { char strings[RB_MAX_CC_STRINGS][64]; int n_strings; } RbFullChoiceStringVec;
typedef struct RbFullChoice {
    RbFullChoiceKind kind;
    char zone[32]; char card_type[32]; int has_card_type; int count;
    char description[512]; char description_en[512]; char description_ja[512];
    int  allow_skip; int cost_limit; char cost_limit_op[8]; int has_cost_limit;
    int  cost_total; char cost_total_op[8]; int has_cost_total;
    RbFullChoiceU8Vec cost_values; char group[32];
    RbFullChoiceStringVec characters; int has_characters;
    RbFullChoiceStringVec heart_colors; int require_all_heart_colors;
    RbFullChoiceStringVec name_fragments; char target_player_id[16];
    int  blind; int is_reveal; char picker[32]; char destination[32];
    int  discard_remaining; int is_select_action;
    int  filtered_indices[RB_MAX_ZONE]; int n_filtered; int has_filtered;
    char target[64]; RbFullChoiceStringVec options; char position[32];
    RbFullChoiceStringVec hc_options;
    RbFullChoiceStringVec aa_options; RbFullChoiceStringVec ls_options;
} RbFullChoice;

/* ── ChoiceResult (engine/src/ability/types.rs::ChoiceResult) ───────── */
typedef struct {
    RbChoiceResultKind kind;
    int   card_indices[RB_MAX_CC_CARDS]; int n_card_indices;
    char  target[64]; char position[32];
    char  heart_colors[RB_MAX_CC_STRINGS][24]; int n_heart_colors;
    char  heart_types[RB_MAX_CC_STRINGS][24];  int n_heart_types;
    int   auto_ability_queue_index; int live_success_card_index;
} RbChoiceResult;

/* ── LookAndSelectStep (engine/src/ability/types.rs::LookAndSelectStep) ── */
typedef struct {
    RbLookAndSelectStepKind kind;
    int  look_count; char look_source[32];
    int  select_count; int select_max_per_group; int has_select_max_per_group;
    char finalize_destination[32]; char finalize_source_zone[32];
} RbLookAndSelectStep;

/* ── ChoiceBuilder (engine/src/ability/types.rs::ChoiceBuilder) ──────── */
typedef struct RbChoiceBuilder {
    RbFullChoice ch;
} RbChoiceBuilder;

#define RB_QUEUE_DEPTH 16
#define RB_USE_TRACK  256

typedef struct {
    RbChoice pending;
    int      has_pending;      /* 1 if choice is waiting for player input */
    int      snapshot_requested;
    int      actor;            /* player who must answer (0/1) */
    AbilityEffect *deferred;   /* sequential remainder after pay_skip gate */
    RbQueueEntry entries[RB_QUEUE_DEPTH];
    int      n_entries;
    int      pending_actions_n;    /* count of pending sequential actions */
    int      cur;              /* index of current entry being resolved */
    int      use_keys[RB_USE_TRACK];
    int      use_counts[RB_USE_TRACK];
    int      n_uses;
    int      use_turn;         /* turn number for which use_keys are valid */
    RbQueueState state;        /* QueueState FSM (Idle/Resolving/AwaitingChoice/Draining) */
    /* choice-resume continuation: when an effect node emits a choice it stores
       itself here so rb_resume_with_choice can re-enter handle_action for that
       node (applying the selection) instead of re-traversing its children. */
    AbilityEffect *resume_eff;
    int      resume_actor;
    int      resume_host;
    /* DEDICATED slot for the success-zone replacement card, added 2026-10-05.
     *
     * Rust keeps it in its own field: set at `turn/actions/mod.rs:894` (`.take()`)
     * and `turn/live.rs:1761`. The C instead overloaded the SHARED `resume_host`
     * (`live.c:1638`), and that slot is clobbered between the prompt being raised
     * and the answer arriving — measured on the corpus as 7 of 13 prompt/result
     * pairs carrying `prompt_original=293` but `result_original=494`. The
     * replacement therefore read the wrong card, which is
     * "CROSSROADS is in the waitroom (replaced)" in `parity_zone_source`.
     *
     * This mirrors Rust rather than fixing the symptom. -1 is the "no pending
     * replacement" sentinel; use `>= 0` to test it, because 0 is a legal card
     * index. APPENDED — never inserted (see the field-order note above). */
    int      pending_success_replacement_card_id;   /* -1 = none */
    int      resume_active;    /* 1 while re-running a resumed effect node */
    int      auto_ability;     /* 1 if pending choice is an auto-ability (drainable) */
    int      choice_result;    /* selected index stored for the resumed node */
    int      resume_mode;      /* 0=deferred, 1=position_change, 2=select_card, 3=auto_ability, 4=draw gate */
    int      resume_is_select; /* 1 if the select_card choice is a select_cards/select/look_and_select
                                   (kept card recorded into g->selected_cards). Set by the emitter, not
                                   derived from resume_eff (which may dangle after the source Card is freed). */
    /* selection filter snapshot (mirrors SelectionContext group/heart filter) —
        copied from the pending choice before it is cleared so rb_look_resume can
        validate the kept card. Empty/negative = no filter. */
    char     resume_filter_group[32];
    int      resume_filter_heart;
    int      resume_look_owner;
    AbilityEffect *resume_after_look;
    /* optional-cost continuation: when an optional pay_energy/cost gate emits a
       choice, the executing effect's parent + the index of that gate are stashed
       here so the resume can run the ability's remaining sibling effects. */
    const AbilityEffect *resume_parent;
    int      resume_child;
    /* heart-color choice result (mirrors Rust execute_choice → conditional_choice =
        Str(color) consumed by execute_gain_resource). Set when a select/choice with a
        heart_color extra is answered; the following gain_resource reads it. -1 = none. */
    int      selected_heart_color;
    AbilityEffect *target_selection_eff;
    /* optional-draw gate resume (mirror draw.rs:execute_draw_wrapper +
        emit_pay_skip_gate). On resume we perform the draw directly instead of
        re-executing the effect (which would re-emit the gate → infinite loop). */
    int      resume_draw_count;     /* resolved count to draw on accept */
    int      resume_draw_target;    /* 0/1 single player, 2 = both */
    char     resume_draw_source[32];
    char     resume_draw_dest[32];
    char     resume_draw_ctype[32];
    int      resume_draw_self_id;   /* self_target_id, or -1 */
    int      just_completed_ability_key; /* (card_id<<16)|ability_idx of last completed ability (prevents re-trigger) */
    /* pending sequential-action storage (ability_queue.rs entry continuations):
       owned deep clones parked across a pause; rb_queue_resume_pending_actions
       executes and frees them. */
    AbilityEffect *pending_repeat_actions[RB_ENTRY_PENDING_CAP];
    int pending_repeat_actions_n;
    int has_pending_reprompt_choice;
    RbChoice pending_reprompt_choice;
    AbilityEffect *choice_options[RB_ENTRY_PENDING_CAP];
    int choice_options_n;
    int choice_reprompt_pending;
    char resume_move_destination[32];
    char resume_move_state[16];
    /* Rust resolver.cancel_remaining_commands: set when a failed deferred cost
       must discard the parked batch. */
    int cancel_remaining_commands;
    /* deferred sub-costs of a sequential cost (cost.rs pay_deferred_costs):
       owned deep clones parked until the player confirms the choice gate. */
    AbilityEffect *pending_deferred_costs[RB_MAX_CHILD];
    int n_pending_deferred_costs;
} RbAbilityQueue;

int  rb_queue_push(RbAbilityQueue *q, int card_id, int ability_idx);
int  rb_queue_push_with_trigger(RbAbilityQueue *q, int card_id, int ability_idx,
                                const char *trigger, const int *moved_cards, int n_moved);
void rb_queue_set_entry_trigger(struct GameState *g, int entry_index, const char *trigger,
                                const int *moved_cards, int n_moved);
void rb_queue_clear(RbAbilityQueue *q);
int  rb_queue_has_pending(const RbAbilityQueue *q);
RbQueueState rb_queue_state(const RbAbilityQueue *q);
void rb_queue_set_state(RbAbilityQueue *q, RbQueueState s);
int rb_use_limit_reached(RbAbilityQueue *q, int card_id, int ability_idx, int limit, int cur_turn);
void rb_record_use(RbAbilityQueue *q, int card_id, int ability_idx, int cur_turn);
int rb_use_count(RbAbilityQueue *q, int card_id, int ability_idx, int cur_turn);
void rb_choice_set_route(RbChoice *ch, RbChoiceRoute r);
void rb_choice_set_description(RbChoice *ch, const char *desc);

/* One flattened (card_id, colour, entry) triple of Rust's
 * PerformanceSnapshot.performance_need_heart_modifiers. */
typedef struct { int16_t card_id; int8_t color; RbModifierEntry entry; } RbPerfNeedHeartMod;
#define RB_MAX_PERF_NEED_HEART_MODS 64

/* MemberContribution (engine/src/core/types.rs:515-531), populated at
 * live.rs:1891-1908. It is the struct that makes rb_effective_blade_parts,
 * rb_member_heart_detail and rb_need_satisfied in core/stats_pipeline.c
 * REACHABLE: their only Rust callers are build_member_contributions
 * (live.rs:1871, :1882), which returns a Vec<MemberContribution>.
 * base_notes/bonus_notes/transform_delta are hard-zero at construction and only
 * filled later at live.rs:3035. ability_heart_bonuses / ability_blade_bonuses are
 * deliberately NOT reproduced - wire them once AbilityBonus's shape is confirmed.
 *
 * MOVED HERE 2026-10-02: this is a member of RbLiveSnapshot, and C requires the
 * complete type before the enclosing struct. It previously lived ~1170 lines
 * below, which is why RbLiveSnapshot could not carry it and live.c parked the
 * data in a file-static `s_member_contrib[2][RB_STAGE_SIZE]`. */
typedef struct {
    int         source_id;       /* i16 */
    uint8_t     slot;            /* u8::try_from(stage index) */
    uint8_t     base_hearts[8];
    uint8_t     bonus_hearts[8];
    uint8_t     base_blades;     /* effective_blade_parts .0 */
    uint8_t     bonus_blades;    /* effective_blade_parts .1 */
    uint8_t     base_notes;
    uint8_t     bonus_notes;
    uint8_t     draw_icons;      /* HeartColor::Draw entries of card.special_heart */
    const char *card_no;         /* owned by the snapshot */
    int         is_wait;         /* mods.orientation[cid]==Wait, live.rs:1886-89 */
    uint8_t     transform_delta[8];
    /* CORRECTED 2026-10-02. `spare` / `base_score_total` / `card_bonus_total` were
     * briefly added HERE, which is WRONG: they are not per-member. Rust's shapes are
     *   PerformanceSnapshot.base_score_total : u8      (types.rs:498)
     *   PerformanceSnapshot.card_bonus_total : u8      (types.rs:500)
     *   LiveCardResult.spare                : [u8; 8] (types.rs:512)
     * i.e. the first two are per-SNAPSHOT and the third is per-LIVE-CARD. As members
     * of RbMemberContribution no faithful body could write them, and they would read
     * as covered when they were not. They now live on RbLiveSnapshot as
     * base_score_total / card_bonus_total / live_spare[]. */
} RbMemberContribution;

typedef struct {
    int player; /* 0/1 */
    int turn;
    int total_hearts[8];
    int lives[RB_MAX_LIVE_CARDS];
    int n_lives;
    int total_score;
    int success; /* 1 if all_pass */
    int surplus_hearts; /* total_pool - total_required, -1 on fail */
    int note_icons;
    int live_passed[RB_MAX_LIVE_CARDS]; /* per-live verdict (populate_live_verdicts) */
    /* per-live allocation detail (mirror live.rs::populate_live_verdicts:
        lives[i].required / filled / score). Filled by allocate_and_verdict. */
    int live_required[RB_MAX_LIVE_CARDS][8];
    int live_filled[RB_MAX_LIVE_CARDS][8];
    int live_score_detail[RB_MAX_LIVE_CARDS];
     int surplus_per_color[8];
     int yell_cards[RB_MAX_LIVE_CARDS * 3];
     int n_yell_cards;
     int yell_blade_hearts[8];
    /* ── Verified-missing GameState members (mod.rs; shapes from an audit that
     * checked each against its Rust twin) ──────────────────────────────────
     * ONLY fields with a confirmed Rust counterpart are added here. Two that were
     * requested are deliberately NOT added:
     *   current_turn_phase - Rust has a real field, but RbGscTurnPhase is fully
     *     DERIVABLE from g->phase + g->first_attacker/g->second_attacker, and
     *     rb_gsc_current_turn_phase already derives it. Adding it would create a
     *     second source of truth.
     *   mods.last_cost_energy_count - no Rust field of that name could be found in
     *     game_state/modifiers.rs or game_state/mod.rs. Do not add until someone
     *     confirms the twin exists. */
    /* Rust PerformanceSnapshot.performance_need_heart_modifiers (types.rs:481),
     * captured at phases.rs:324-333 and :621-632, consumed by
     * restore_performance_need_heart_modifiers (game_state/modifiers.rs:2136-2159).
     * The capture is a FLATTENING of the whole map, not a delta:
     *   mods.need_heart_modifiers.iter()
     *     .flat_map(|(&card_id, colors)| colors.iter().map(|(&c, e)| (card_id, c, *e)))
     *     .collect()
     * The C source shape already exists as RbMods.need_heart[RB_MAX_CARD_IDS][8]
     * of RbModifierEntry{int16_t set; int16_t add;} (rabuka.h:198), with
     * rb_mods_add_need_heart / rb_mods_set_need_heart / rb_mods_get_need_heart
     * already exported. Only ever ITERATED, never keyed.
     * NOTE: the whole need_heart_modifiers EFFECT FAMILY is still absent from the
     * C tree - a repo-wide grep finds only the "need_heart_mod" string-table entry
     * and one classification. Nothing writes or reads mods.need_heart yet, so this
     * field is the missing restore SIDE only; the granting side still has to be
     * ported or restoring is a no-op. */
    RbPerfNeedHeartMod performance_need_heart_modifiers[RB_MAX_PERF_NEED_HEART_MODS];
    int n_performance_need_heart_modifiers;
    /* TOMBSTONE 2026-10-04 (field census): `base_score_total`, `card_bonus_total`
   and `live_spare[RB_MAX_LIVE_CARDS][8]` are DELETED. All three were 0 writers
   and 0 readers, and the note they carried was RIGHT about Rust and was still
   the wrong reason to keep them: they are WRITE-ONLY IN RUST, so a C producer
   would have had nothing to feed.
     `PerformanceSnapshot.base_score_total` (core/types.rs:498) and
     `.card_bonus_total` (:500) are written once, in `finalize_snapshot_fields`
     (turn/live.rs:771-782), and read NOWHERE in engine/src -- not by the
     condition evaluator, not by the live-success check, not by the display
     projection. The only readers anywhere are the characterisation test
     tests/test_modules/characterization/performance_snapshot_audit_test.rs:82/144.
     `LiveCardResult.spare` (types.rs:512) is written at live.rs:733-741 and
     initialised empty at live.rs:3153; a search of all of engine/src finds the
     identifier `spare` only in those two places and in unrelated bot-local
     variables. Nothing consumes it.
   So all three existed "for the web/JSON surface" (the old comment's words) while
   the web surface in the C reads `live_score_detail` / `surplus_per_color`, which
   are live. The superseded note asked src/turn/live.c for the three bodies "three
   lines each"; those three lines would have written values no consumer in either
   engine can observe, which is precisely the "producer for something nobody
   consumes" defect this census set out to remove. `member_contributions` below is a
   DIFFERENT case and is live -- see its own comment. */
    /* Rust LivePerformanceData.member_contributions (types.rs:515-531), built by
     * build_member_contributions (live.rs:1844-1915) which returns a Vec that
     * .iter_mut().find(|m| m.source_id == app.target_card_id) walks in
     * enrich_from_applications (live.rs:1329). ADDED 2026-10-02: without this
     * field live.c parked the same data in a file-static, so nothing outside
     * live.c could reach it and rb_enrich_from_applications was empty. */
    RbMemberContribution member_contributions[2][RB_STAGE_SIZE];
    int n_member_contributions[2];
} RbLiveSnapshot;


/* A modifier granted by a trigger with a Duration that must be reverted when
   the duration expires. Mirrors engine/src/core/game_state/mod.rs TemporaryEffect.
   dur: 0 = permanent (no expiry), 1 = live_end/during_live (revert at live
   phase end), 2 = until_end_of_turn/first_turn (revert at turn rollover). */
#define RB_MAX_TEMP_EFFECTS 64
#define RB_TEMP_PERM      0
#define RB_TEMP_LIVE_END  1
#define RB_TEMP_TURN_END  2
#define RB_REVERT_NONE            0
#define RB_REVERT_BLADE_SET       1
#define RB_REVERT_BLADE_TYPE      2
#define RB_REVERT_HEART_MULT      3
#define RB_REVERT_HEART_COPY      4
#define RB_REVERT_COST_SET        5
#define RB_REVERT_TEXT            6
#define RB_MAX_DELAYED_GAINED 16
typedef struct {
    int card_id;
    AbilityEffect *effect;
} RbDelayedGainedEffect;
typedef struct {
    int card_id;            /* host card the effect belongs to */
    int dur;                /* RB_TEMP_* duration kind */
    int blade;
    int score;
    int cost;
    int heart[8];
    int need_heart[8];
    int gained_card_id;
    int gained_index;
    int revert_kind;
    int previous_value;
    int previous_value2;
    char revert_text[48];
} RbTempEffect;

/* Replacement effect (used by GameState at line 819). */
typedef struct {
    int card_id;
    int player_id;
    char original_event[64];
    int is_choice_based;
    int applied_this_event;
} RbReplacementEffect;

#define RB_MAX_ABILITY_INVALIDATIONS 64
typedef struct {
    int card_id;
    char trigger[32];
    char duration[32];
    int created_turn;
} RbAbilityInvalidation;

#define RB_MAX_SNAPSHOTS 64
#define RB_MAX_RECENTLY_MOVED 8
#define RB_MAX_REVEALED_CARDS RB_MAX_ZONE
/* Slots in the per-GameState keep-alive ring that backs rb_entry_cost() /
   rb_entry_effect() / rb_entry_destination() (see the entry-accessor block at
   the bottom of this header). 4 decoded abilities is enough for the deepest
   observed call nesting (2) with headroom. */
#define RB_ENTRY_KEEPALIVE_SLOTS 4

/* batch_movements / position_change_events entry types (must precede GameState) */
typedef struct {
    int moved_card_id;
    int source_zone;
    int dest_zone;
    int cause_player_id;
    /* Rust MovementEvent.cause_card_id (game_state/mod.rs). Distinct from
     * cause_player_id: `eval_group`'s temporal this_turn arm keys on the CAUSE
     * CARD (card.rs:1642-1655), so without this the arm cannot be ported
     * faithfully. */
    int cause_card_id;
    int effect_only;
} RbBatchMovement;
typedef struct {
    int card_id;
    int position;
    /* Rust PositionChangeEvent.old_area / old_position, needed by the
     * `area_direction:"from"` condition arm (2 corpus nodes). Added 2026-10-04:
     * without it the "from" direction cannot be evaluated at all, because the C
     * only recorded the DESTINATION. APPENDED - never inserted; see the note on
     * field order above GameState. */
    int old_position;
} RbPositionChangeEvent;

/* Rust GameState.rule_log capacity: LOG_BOUND_RULE = 500 (mod.rs:1602-1606),
 * newest-kept. Defined HERE, before GameState, because GameState's rule_log
 * member is sized by them - a #define after the struct is a compile error. */
#define RB_GSC_RULE_LOG_MAX 500
/* A Vec<String> needs the strings, not just a count. */
#define RB_GSC_RULE_LOG_LINE_MAX 256

typedef struct GameState {
     RbPlayer p[2];
     int      stage_hearts[2][8];
     RbMods   mods;

    RbAbilityQueue queue;
    /* Keep-alive for an in-flight activation that pends a choice mid-resolution.
       rb_activate_card wraps cost+effect into a heap effect tree and stores the
       decoded ability here so its cost/effect pointers stay valid across the
       choice round-trip (mirrors Rust's persistent resolver). Freed when the
       activation's pending choice fully resolves (rb_resume_with_choice). */
    Ability  activation_keepalive;
    int      activation_keepalive_valid;
    AbilityEffect *activation_act;
    int      live_set_limit_reduction[2];
    int      yell_count_mod[2];   /* per-player modify_yell_count delta (live.c do_yell) */
    int      activating_card;     /* currently resolving card id (mirrors gs.activating_card) */
    /* TOMBSTONE 2026-10-04 (field census): `pending_energy_payment` is DELETED.
   It had 0 writers and 0 readers tree-wide. The behaviour it claimed to hold
   is ALREADY LIVE and lives somewhere else: Rust's twin is NOT on GameState at
   all but on the resolver -- `AbilityResolver.awaiting.energy_payment: Option<u8>`
   (engine/src/ability/resolver.rs:198), written at effects/energy.rs:568 and
   cleared at ability/cost/handlers.rs:1404 / :1427 / :1676. The C carries that
   state as the file-static `s_pending_energy_payment` in src/ability/cost.c
   (:25 declared, :2083 read, :1774 / :2053 cleared), which is LIVE. A GameState
   member was therefore never a twin of anything: it was a second, unwritten
   copy of a resolver-local that already exists. */
    int16_t  last_cost_waited_members[RB_STAGE_SIZE]; /* members put to wait by last cost */
    int      n_last_cost_waited_members;
    char     yell_source[2][16];   /* per-player modify_yell_source override (live.c do_yell) */
    /* ── Moved here from RbLiveSnapshot. These are GameState data (Rust
     * core/game_state/mod.rs) that was parked on the snapshot struct, so nothing
     * could read them: `g->cards_paid_cost_this_turn` failed to compile with
     * "'GameState' has no member named ...".
     * Until they lived here, condition.c had NO `paid_cost_this_turn` branch at
     * all, so `comparison_type:"cost"` was a `cost >= 0` tautology on
     * PL!-pb2-021-N. Rust's arm is a separate early branch at
     * ability/condition/card/comparison.rs:60-70, AHEAD of the cost path at :74/:90. */
    /* record_cost_paid (mod.rs:404-414): current_entry().card_id, pushed if not
     * already present. 5 Rust callers: cost/handlers.rs:1389,1477,
     * choice.rs:734,763,948. Also unblocks condition.c's `comparison_type:"cost"`
     * arm, which is currently a `cost >= 0` tautology on PL!-pb2-021-N. */
    int  cards_paid_cost_this_turn[RB_MAX_ZONE];
    int  n_cards_paid_cost_this_turn;
    /* Rust GameState.cards_activated_this_turn: Vec<i16> (game_state/mod.rs:165),
     * a PER-TURN SET OF CARD IDS - pushed by record_ability_activation, drained
     * by the PB2-010-P＋ / PB2-040-L auto-ability scans
     * (abilities.rs:3061/3110/3164/3173), cleared at modifiers.rs:2024.
     * RB_MAX_CARD_IDS is the right bound: this is a per-card set within one
     * turn, the same shape the engine already sizes moved_this_turn[] and
     * cards_appeared_this_turn[] to, so it adds ~8 KB to a 781 KB GameState.
     * NOTE: the reader at abilities.rs:3061+ has NO C counterpart at all yet -
     * those are the clear_blade_set / clear_score_set / clear_blade_type /
     * remove_cost_modifier_set calls that undo a temporary effect, and they are
     * the correct wiring targets for the nine rb_mods_* setters/clearers that
     * still have 0 C callers. */
    int  cards_activated_this_turn[RB_MAX_CARD_IDS];
    int  n_cards_activated_this_turn;
    /* cheer_revealed_cards / _mut (mod.rs:931, :1037) are SmallVec<[i16;8]>;
     * produced at phases.rs:405, consumed at move/mod.rs:1583.
     * STILL 0 writers / 0 readers (field census 2026-10-04) — kept, not deleted,
     * because Rust reads them for real: `cheer_revealed_cards_mut` fills the
     * performer's buffer at turn/phases.rs:405-408, and TWO in-scope consumers
     * read it — `resolve_source_revealed_cards` (ability/effects/move/mod.rs:1580-1621,
     * which DRAINS the buffer and falls back to a zone-filtered `revealed_cards`
     * only when it is empty) and `revealed_count` (ability/dynamic_count.rs:159-180,
     * the `count_type == "revealed_cards"` arm). So this is a missing twin, not a
     * phantom. All three legs are located:
     *   PRODUCER  src/turn/live.c:676-679 — immediately after the `g->revealed_cards`
     *            push (the twin of phases.rs:396-398) and before
     *            `g->yell_occurred = n_yell > 0;` (the twin of phases.rs:421), append
     *            each `yell_cards[i]` to the performer's cheer buffer by seat.
     *            `set_cheer_count_by_id` (phases.rs:409) is the same site and its C
     *            twin `rb_gsc_set_cheer_count_by_id` already exists
     *            (src/core/game_state_core.c:423) but has no caller.
     *   CONSUMER 1 src/ability/effects/move.c:1780 `rb_move_resolve_source_revealed_cards`
     *            — add the `cheer_is_empty` branch ahead of the :1783-1807 filter loop.
     *   CONSUMER 2 src/ability/dynamic_count.c:681 — the C comment there states the C
     *            "does not maintain a separate cheer pool, so step 1 always falls
     *            through"; that is the gap, and closing it needs this field.
     * BLOCKED-ON: src/ability/effects/move.c:1780 AND
     *   src/ability/dynamic_count.c:683 — both consumers. The producer site
     *   (src/turn/live.c:676-679) is MINE and is deliberately NOT landed alone:
     *   a producer with no consumer is the same defect wearing a different hat. */
    int  player1_cheer_revealed_cards[8];
    int  n_player1_cheer_revealed_cards;
    int  player2_cheer_revealed_cards[8];
    int  n_player2_cheer_revealed_cards;
    /* push_revealed_cost_card (mod.rs:1091-1110); callers handlers.rs:1286,
     * choice.rs:1673,1928. Cost-reveal provenance.
     * STILL 0 writers / 0 readers (field census 2026-10-04) — kept, not deleted,
     * because Rust reads it for real in FOUR in-scope places:
     * `remove_revealed_cost_card` (core/game_state/modifiers.rs:2055-2059) and
     * `remove_from_source_hands` (:2080-2082), both of which are E5/E6 and are
     * themselves blocked on this producer; `choice.rs:3377` (`.last()`); and
     * `move/mod.rs:368 / :1831 / :1955` (`.iter()` / `.contains()`).
     * BLOCKED-ON: src/ability/cost.c — port `GameState::push_revealed_cost_card`
     *   (core/game_state/mod.rs:1091-1110: set semantics, newest-last, bounded) and
     *   call it from the twin of ability/cost/handlers.rs:1286. Second producer site:
     *   src/ability/choice.c at the twins of ability/choice.rs:1673 and :1928 (that
     *   file is contested by a re-fire fix). Readers to wire once produced:
     *   src/ability/effects/move.c at the twins of move/mod.rs:368 / :1831 / :1955,
     *   and src/core/modifiers.c at :2055 / :2080 for E5/E6.
     * Note Rust also drops the parallel `revealed_cost_card_meta[index]` in this
     *   push (mod.rs:1101-1109) and C has no field for it — same shape as
     *   `revealed_card_meta`, already recorded above `clear_revealed_cards`. It is
     *   display-only in Rust, so it is NOT part of the port. */
    int  revealed_cost_cards[RB_MAX_ZONE];
    int  n_revealed_cost_cards;

    /* Rust GameState.turn_area_movements: SmallVec<[MovementEvent;4]>
     * (game_state/mod.rs:250), pushed only in push_movement_event's stage->stage
     * arm (modifiers.rs:1938), read at abilities.rs:549 and mod.rs:986-1000,
     * cleared at modifiers.rs:2015. C had only the scalars last_area_move_*,
     * which cannot express "this turn, in order". */
    RbBatchMovement turn_area_movements[RB_MAX_RECENTLY_MOVED];
    int n_turn_area_movements;
    RbLiveSnapshot snapshots[RB_MAX_SNAPSHOTS];
    int      n_snapshots;
    int      recently_moved[RB_MAX_RECENTLY_MOVED];
    int      n_recently_moved;
    int      those_cards[RB_MAX_RECENTLY_MOVED]; /* cards moved by the immediately preceding move_cards action (Rust `those_cards` relay) */
    int      n_those_cards;
    int      recently_appeared[RB_MAX_RECENTLY_MOVED]; /* cards that appeared this turn */
    int      n_recently_appeared;
    int      recently_state_changed[RB_MAX_RECENTLY_MOVED]; /* cards whose state changed this turn */
    int      n_recently_state_changed;
    RbBatchMovement batch_movements[16];
    int n_batch_movements;
    RbPositionChangeEvent position_change_events[16];
    int      n_position_change_events;
    int      selected_cards[RB_MAX_RECENTLY_MOVED]; /* cards chosen by a select_cards/select/look_and_select choice */
    int      n_selected_cards;
    /* TOMBSTONE 2026-10-04 (field census): `assignment[]` + `n_assignment` are
   DELETED. 0 writers, 0 readers, and -- unlike the rest of this group -- no
   Rust member either: a whole-tree search of engine/src + engine/tests finds
   ZERO occurrences of an `assignment` field or accessor. Rust's alt-cost solver
   never puts the chosen assignment on the state either: `find_distinct_assignment_k`
   (turn/phases.rs:1603-1629) returns `Option<Vec<i16>>` by value, and the C's
   faithful twin `rb_find_distinct_assignment_k` (src/turn/phase.c:1090-1105)
   writes into a CALLER-SUPPLIED `out[]` and returns the count. So the value
   never lives on GameState in either engine and the member was reading as
   coverage for a buffer that provably cannot be filled from anywhere.
   NB `n_assignment` is distinct from `ptc_card`/`ptc_hand`/`ptc_area`, which are
   the live play-time-alt-cost context and DO have readers. */
     int      live_success[2];
     int      live_score[2];
     int      live_batch_mode;
     int      live_victory_pending;
     int      live_victory_stage;
     int      live_pre_score[2][RB_MAX_CARD_IDS];
     int      live_pre_valid[2];

    int      p1_live_won;       /* Rule 8.4.13: P1 won the live (placed to success) this turn */
    int      p2_live_won;       /* Rule 8.4.13: P2 won the live (placed to success) this turn */
    /* live-surplus / no-excess flags (mirror live.rs::compute_surplus_and_flags +
        record_pretrigger_live_results). Zeroed by rb_game_init. */
    int      self_live_surplus_count;
    int      opponent_live_surplus_count;
    int      live_surplus_ready_this_turn;
    int      p1_live_success_no_excess;
    /* Rust ConditionContext::no_excess_heart_flag (ability/condition/state.rs:15-24)
     * is a TWO-VALUE read of these two GameState flags, not a reconstruction from
     * score snapshots:
     *   if player.is_first_attacker { self_no_excess_heart_this_turn }
     *   else { opponent_live_no_excess_heart_this_turn }
     * The C has no counterpart and was approximating the answer from score
     * snapshots, which is why `eval_no_excess` is not faithful. */
    int      self_no_excess_heart_this_turn;
    int      opponent_live_no_excess_heart_this_turn;
    int      p2_live_success_no_excess;
    /* state_change_condition tracking (mirrors Rust recently_state_changed /
       turn_state_changes). Set when a member's orientation actually flips during
       rb_effect_change_state; cleared at turn rollover. from/to are orientation
        indices (0=active/none,1=wait) keyed by card id. */
    int      turn_state_changes[64][4]; /* [activating_card, target_card, from_state[0], to_state[0]] */
    int      n_turn_state_changes;
    int8_t   state_change_from[RB_MAX_CARD_IDS];
    int8_t   state_change_to[RB_MAX_CARD_IDS];
    int      state_change_triggering;
    int      last_wait_to_active_count; /* count of wait→active flips this turn */
    int      revealed_cards[RB_MAX_REVEALED_CARDS]; /* cards revealed by yell/re_yell */
     int      n_revealed;
     int      yell_occurred;
     int      performance_resume_pending;
     int      performance_resume_player;
     int      performance_resume_yell_cards[RB_MAX_ZONE];
     int      performance_resume_n_yell_cards;
     int      performance_resume_blade_hearts[8];
     int      performance_resume_note_icons;
     int      performance_resume_yell_flag;
     int      last_draw_count;   /* mirror AbilityResolver.step_state.last_draw_count */
    int      last_surplus_loss_count[2]; /* gain_surplus_heart: surplus hearts gained/lost this live (misc.rs) */
     int      re_yell_occurred;  /* a re_yell effect fired this live */
     int      re_yell_blade_hearts[8]; /* hearts harvested by perform_yell, applied to live */
     int      re_yell_note_icons;
     int      re_yell_pending;
     int      re_yell_owner;
    int      last_energy_placed_by_effect;
    int      last_energy_placed_by_player;
    int      last_area_move_card_id;
    int      last_area_move_by_player;
    int      active;
    int      first_attacker;
    int      second_attacker;
    int      turn;
    int      winner;
    RbPhase  phase;
    int      rps[2];
    int      rps_winner;           /* -1=none, 0=p1, 1=p2, 2=tie */
    int      player1_rps_choice;   /* RPS choice for player 1: -1=none, 0=rock, 1=paper, 2=scissors */
    int      player2_rps_choice;   /* RPS choice for player 2: -1=none, 0=rock, 1=paper, 2=scissors */
    int      mulligan_selecting[2]; /* 1 if mulligan selection in progress per player */
    int      mulligan_done[2];     /* per-player mulligan done flag */
    int      live_set_player;
    RbBag    resolution;             /* resolution zone (temp holding) */
    /* Rust ResolutionZone.owners (core/game_state/mod.rs), the half of the zone
     * that says WHO yelled each card. check_invalid_resolution_zone
     * (game_state_core.c) returns each card to its yeller; without this the C
     * assumes the active seat, so a P2 yell card lands in P1's waitroom -
     * already observable, counted inside parity_draw_flat and parity_sequential.
     * Parallel array, index-aligned with `resolution`. */
    uint8_t  resolution_owners[RB_MAX_ZONE];
    int      n_resolution_owners;
    RbBag    exclusion_zone;         /* cards excluded from the game */
     RbTempEffect temp_effects[RB_MAX_TEMP_EFFECTS];
     int      n_temp_effects;
     RbDelayedGainedEffect delayed_gained_effects[RB_MAX_DELAYED_GAINED];
     int      n_delayed_gained_effects;

    int      stage_arrived[2][RB_STAGE_SIZE]; /* set when a member was deployed this turn (baton arrival-ban, Rule 9.6.2.1.2.1) */
    int      baton_touch_used[2];             /* baton used this play-action (once-per-action limit) */
    int      baton_last_vacated_area[2];      /* stage area vacated by the most recent baton (mirrors Rust last_vacated_stage_area) */
    int      current_is_baton;                /* context flag threaded to effect/trigger evaluation */
    int      player_cannot_activate[2];      /* restriction: that player may not activate abilities */
    int      cannot_active_cards[RB_MAX_ZONE]; /* restriction: per-card cannot-activate (delayed/next-turn) */
    int      n_cannot_active_cards;
    char     prohibition[64][48];            /* restriction: "type:destination" prohibition notes */
    int      n_prohibition;
    char     prohibition_effects[64][48];    /* dynamic prohibition effects (e.g. cannot_place) */
    int      n_prohibition_effects;
    RbReplacementEffect replacement_effects[32]; /* mirrors Rust replacement_effects */
    int      n_replacement_effects;
    /* ── tracking.rs (ported) ── */
    int      turn1_abilities_played[64]; int n_turn1_abilities_played;
    /* TOMBSTONE 2026-10-04 (field census): `turn2_abilities_played[64]` is DELETED
   (0 writers, 0 readers). Rust does not have a live turn-2 ledger either:
   `GameState.turn2_abilities_played` (core/game_state/mod.rs:132) is only
   initialised (:609) and cleared (core/game_state/tracking.rs:8) -- there is NO
   push site in engine/src -- and its only reader is the display projection
   `game/display.rs:1928-1929`, i.e. T8 Q2 read-only projection, OUT OF SCOPE.
   So this is WRITE-ONLY in Rust as well as in C: a twin of nothing.
   `n_turn2_abilities_played` is KEPT because src/core/tracking.c:41 still
   writes it and that file is another agent's; it remains 1 writer / 0 readers.
   BLOCKED-ON: src/core/tracking.c:41 -- delete `g->n_turn2_abilities_played = 0;`
   once this array deletion has landed everywhere. Keeping the counter is
   harmless (it is never read) but it is the shape the census warns about: a
   writer maintaining a length for a buffer nothing fills. */
    int      n_turn2_abilities_played;
    int      player1_cheer_blade_heart_count;
    int      player2_cheer_blade_heart_count;
    RbBag    last_resolution_cards_p1;
    RbBag    last_resolution_cards_p2;
    int      cheer_check_base; /* -1 = None */
    int      cheer_checks_required;
    int      cheer_checks_done;
    int      cheer_check_completed;
    RbYellMod yell_count_modifiers[32]; int n_yell_count_modifiers;
    int      baton_touch_count_p1;
    int      baton_touch_count_p2;
    int      baton_touch_arriving_card_ids[16]; int n_baton_touch_arriving_card_ids;
    int      baton_touch_zero_cost;
    int      baton_touch_replaced_member_cost; /* -1 = None */
    int      baton_touch_replaced_member_id;  /* -1 = None */
    int      baton_touch_arriving_card_id; /* -1 = None */
    /* TOMBSTONE 2026-10-04 (field census): `areas_placed_this_turn[16]` is DELETED
   (0 writers, 0 readers). Rust's twin is `GameState.areas_placed_this_turn:
   SmallVec<[String; 8]>` (mod.rs:156), initialised at :622 and cleared by
   `clear_area_placement_tracking` (core/game_state/modifiers.rs:1701-1703) --
   and nothing else in engine/src touches it: no push site, no condition or
   effect reads it. Its ONLY reader is `game/display.rs:1849` (T8 Q2, OUT OF
   SCOPE). So the C member was a 0W/0R twin of a Rust field that is itself
   write-nothing/read-for-display.
   `n_areas_placed_this_turn` is KEPT (src/core/tracking.c:21 writes it, that
   file is another agent's); it stays 1 writer / 0 readers.
   BLOCKED-ON: src/core/tracking.c:21 -- delete the line. */
    int      n_areas_placed_this_turn;
    int      cards_appeared_this_turn[64]; int n_cards_appeared_this_turn;
    /* TOMBSTONE 2026-10-04 (field census): `auto_ability_trigger_counts[32]` is
   DELETED (0 writers, 0 readers). Rust's twin (mod.rs:154 `HashMap<CardId, u8>`)
   is initialised at :621 and cleared by `clear_auto_ability_trigger_tracking`
   (modifiers.rs:1729-1731) and nothing else: no push site in engine/src, no
   gameplay reader. Sole reader is `game/display.rs:1885-1886` (T8 Q2, OUT OF
   SCOPE). `n_auto_ability_trigger_counts` is KEPT (src/core/tracking.c:10
   writes it). BLOCKED-ON: src/core/tracking.c:10 -- delete the line. */
    int      n_auto_ability_trigger_counts;
    int      position_change_occurred_this_turn;
    int      formation_change_occurred_this_turn;
    int      opponent_live_success_this_turn;
    int      opponent_choice_declined;
    /* CORRECTED 2026-10-04. This was `int game_state_history[64]`, but
     * rb_check_permanent_loop stores a **uint64_t FNV hash** here
     * (game_state_abilities.c:1439) and Rust's equivalent is `Vec<u64>`
     * (core/game_state/mod.rs:117). Truncating a 64-bit hash into `int` and then
     * sign-extending it on compare means **repetition detection can never fire**:
     * the stored value and the recomputed one disagree in the high 32 bits far
     * too often to ever compare equal. Rust-side this is already red.
     * Widened, not appended — this is a mid-struct change, so every object file
     * compiled against the previous layout is invalid. That is the same hazard the
     * append-only rule exists to contain; it is unavoidable for a width fix.
     * If you change this, move RB_GAMESTATE_BYTES_PINNED (test_harness_self.c)
     * and Makefile:3 in the SAME change. */
    uint64_t game_state_history[64]; int n_game_state_history;
    int      loop_detected;
    /* just_completed_ability_key — mirrors Rust's GameState.just_completed_ability_key.
        Set by rb_drain_ability_queue after each ability resolves so the auto-trigger
        scan can skip re-enqueueing the very ability that just fired (prevents an
        auto ability from recursively re-triggering itself). Key = (card_id<<16)|ability_idx. */
    int      just_completed_ability_key;
    /* batch_triggered_keys — per auto-trigger scan, the numeric (card_id<<16)|ability_idx
        keys already enqueued this batch. Mirrors Rust's this_batch_triggered_ability_ids
        so a single trigger event never enqueues the same watcher more than once. */
    int      batch_triggered_keys[64]; int n_batch_triggered_keys;
    /* ── temporal-condition tracking (mirrors GameState.has_card_moved_this_turn /
        debut_count_this_turn; position_change_occurred_this_turn already declared above) ── */
    int      moved_this_turn[RB_MAX_CARD_IDS]; /* per-card: moved during current turn */
    /* TOMBSTONE 2026-10-04 (field census): `energy_placed_this_turn[2]` is DELETED
   (0 writers, 0 readers) and has NO Rust member at all -- zero occurrences of
   the identifier anywhere in engine/src or engine/tests. The per-player energy
   fact Rust actually keeps is not an array but a pair of accessors on GameState:
   `last_energy_placed_by_effect()` / `last_energy_placed_by_player()`
   (core/game_state/mod.rs:1015 / :1024), consumed at turn/actions/mod.rs:1504
   and at the each_time scan (game_state/abilities.rs:1842, :2268). Those have
   their own C members and are tracked separately -- note they are currently
   0-WRITER/10-reader and 0-WRITER/2-reader in the C, which is the NEXT defect in
   this family and is reported, not fixed here, because their writers live under
   src/ability/effects/ and that directory is another agent's. */
    int      debut_count_this_turn[2];         /* members debuted this turn per player */
    /* auto_event_mask[pl] — bitmask of auto/event triggers that occurred since the
        last trigger_auto_abilities_for_player scan (mirrors Rust's per-event
        auto-trigger queueing). rb_record_event sets bits; rb_fire_recorded_auto
        fires only the recorded triggers then clears the mask. */
    int      auto_event_mask[2];
    /* play recursion depth — guards rb_play_member against unbounded re-entrancy
        when a debut/baton effect itself places a member (which re-enters this fn). */
    int      play_depth;
    /* ── C6 keep-N-shuffle-rest (draw.rs::execute_both_hand_keep_shuffle_under) ──
        Resolver-persistent phase state. Phase 0 snapshots self's hand and prompts;
        phase 1 (after self's choice) moves self's non-selected under deck and
        prompts opponent; phase 2 moves opponent's non-selected under deck and
        resets. Snapshots are the hand at selection time; selected holds the kept
        positions chosen in each SELECT_CARD choice. */
    int      keep_shuffle_under_phase;
    int      keep_shuffle_under_count;
    int      keep_shuffle_under_snapshot[2][RB_MAX_HAND];
    int      keep_shuffle_under_snapshot_n[2];
    int      keep_shuffle_under_selected[RB_MAX_HAND];
    int      keep_shuffle_under_selected_n;
    /* Play-time alternative cost (Rust play_time_cost_reduction_hook): when a
        card with a 常時/プレイ時 modify_cost(set) alt-cost is played, rb_play_member
        pauses here and stores the pending play; rb_complete_play_with_cost then
        finishes it at the chosen cost. Not used by cards without such an ability,
        so the synchronous play path is unaffected. */
    int      ptc_active;     /* a play is paused awaiting the alt-cost answer */
    int      ptc_resuming;   /* rb_play_member is completing a paused play */
    int      ptc_card;       /* card id being played */
    int      ptc_hand;       /* hand index of the card */
    int      ptc_area;       /* target stage area */
    int      ptc_set;        /* alternative cost value (accept) */
    int      ptc_base;       /* base cost (decline) */
    /* ── gained_card_abilities (mirrors GameState.gained_card_abilities:
        HashMap<i16, Vec<Ability>>). Runtime-gained abilities (「…を得る」effects)
        keyed by card_id. Stored as a flat array of (card_id, Ability) pairs with
        a count per card, since C has no HashMap. rb_card_gained_ability and
        rb_card_num_gained_abilities read/write this store. */
    int      gained_card_ids[64];          /* card_id for each gained-ability slot */
    Ability  gained_card_abilities[64][4]; /* up to 4 gained abilities per slot */
    int      gained_card_n[64];            /* count of gained abilities per slot */
    /* TOMBSTONE 2026-10-04 (field census): `n_gained_cards` is DELETED (0 writers,
   0 readers) and has no Rust member either -- zero occurrences in engine/src or
   engine/tests. The distinct-card count is implicit in both engines and does not
   need a counter: Rust keys `gained_card_abilities` by CardId in a HashMap and
   keeps `gained_card_ids` index-aligned with it (mod.rs:183-184); the C uses the
   `-1` EMPTY-SLOT SENTINEL in `gained_card_ids[64]` and scans the whole array
   (src/core/game_state_abilities.c:165, :173-175 find/claim the slot;
   src/core/modifiers.c:1670-1677 walks it). `gained_card_n[i]` is the per-slot
   ability count and is live (5 writers / 10 readers). A separate slot count was a
   third, unread, parallel bookkeeping of something the sentinel already answers. */
    RbAbilityInvalidation ability_invalidations[RB_MAX_ABILITY_INVALIDATIONS];
    int      n_ability_invalidations;
    /* ── entry-accessor keep-alive ring (use-after-free fix) ──
       rb_entry_cost() / rb_entry_effect() / rb_entry_destination() return
       pointers INTO one of these slots instead of into a tree they had already
       freed. Each call decodes a fresh ability into the next slot and releases
       the slot the ring is about to reuse, so a returned pointer stays valid
       for RB_ENTRY_KEEPALIVE_SLOTS-1 further entry-accessor calls.
       Zero-initialised by rb_game_init's memset(g, 0, sizeof(*g)) (and by the
       tests' own memset), which is all the initialisation these need: an unused
       slot has entry_keepalive_used[slot] == 0. There is no GameState
       destructor in the C port, so the ring is not explicitly drained at
       teardown; a live GameState holds at most RB_ENTRY_KEEPALIVE_SLOTS decoded
       abilities, matching the pre-existing (unbounded) `activation_keepalive`
       keep-alive at the top of this struct. */
    Ability  entry_keepalive[RB_ENTRY_KEEPALIVE_SLOTS];
    unsigned char entry_keepalive_used[RB_ENTRY_KEEPALIVE_SLOTS];
    int      entry_keepalive_next;
    /* ── recently_moved_from_zone (mirrors GameState.recently_moved_from_zone:
       Option<String>, engine/src/core/game_state/mod.rs:204) ──
       The FROM-zone label of the current recently-moved batch, written only by
       the set_recently_moved_batch choke point (modifiers.rs:1566) and cleared by
       clear_recently_moved_batch. It is the source that populates
       TriggerEvent.moved_from_zone (abilities.rs:424 / actions/mod.rs:1469), which
       is what a 移動時 condition gated on the source zone matches against. An empty
       string means None (no label), which is also the memset-zero state produced by
       rb_game_init.

       PLACED AT THE END OF THE STRUCT ON PURPOSE. It is semantically adjacent to
       `recently_moved` (RB_MAX_RECENTLY_MOVED entries above), but inserting it
       there would shift the offset of every field after it and silently
       invalidate any object file compiled against the previous header. Appending
       keeps all pre-existing offsets and orderings bit-identical; the only cost is
       that GameState grows by RB_TE_ZONE_SZ bytes. */
    char     recently_moved_from_zone[RB_TE_ZONE_SZ];
    /* ── card_appearance_source (mirrors GameState.card_appearance_source:
        SmallVec<[(i16, String); 4]>, engine/src/core/game_state/mod.rs:144) ──
        The zone a card debuted FROM, appended by rb_record_card_appearance
        (modifiers.rs:1377-1388) and dropped wholesale by
        rb_clear_card_appearance_tracking (modifiers.rs:1401-1404). Rust stores
        the source as a String; the C port stores the RbZone value the caller
        already passes to rb_record_card_appearance, and
        rb_get_card_appearance_source + rb_appearance_source_matches translate
        between the two spellings, so no new string allocation enters the
        state struct.

        This is what makes a 控え室から登場 gate expressible: the
        `appearance_source` condition field (decoded from the ability bytecode
        by vm.c build_appearance, e.g. PL!S-bp6-016-N) is compared against the
        zone recorded here for the ACTIVATING card. Before this field existed
        rb_record_card_appearance began with `(void)source;` and a HAND debut
        opened exactly the same prompt as a WAITROOM debut.

        PLACED AT THE END OF THE STRUCT ON PURPOSE, for the same reason as
        recently_moved_from_zone above: this list is semantically adjacent to
        `cards_appeared_this_turn` (line ~1458), but inserting it there would
        shift the offset of every field after it and silently invalidate any
        object file compiled against the previous header. Appending keeps all
        pre-existing offsets and orderings bit-identical. The zero state is
        "no appearance source recorded", which is also what rb_game_init's
        memset(g, 0, sizeof(*g)) produces. */
    int      card_appearance_source_card[32];   /* card_no index per entry */
    uint8_t  card_appearance_source_zone[32];   /* RbZone the card debuted from */
    int      n_card_appearance_source;

    /* Rust GameState.rule_log: Vec<String> (core/game_state/mod.rs:605),
     * bounded by LOG_BOUND_RULE = 500, newest-kept (mod.rs:1602-1606). This is a
     * buffer SEPARATE from the 32-slot condition-verdict ring, which drops on
     * overflow - routing the rule log into that ring makes the two logs destroy
     * each other's entries and keeps nine rb_gsc_* functions unroutable.
     * RB_GSC_RULE_LOG_MAX alone is not enough: it is a count, and a Vec<String>
     * needs the strings. APPENDED, never inserted - see the note above on why
     * field order is load-bearing. */
    char     rule_log[RB_GSC_RULE_LOG_MAX][RB_GSC_RULE_LOG_LINE_MAX];
    uint16_t rule_log_len[RB_GSC_RULE_LOG_MAX];
    int      n_rule_log;

    /* Rust LiveCardZone.face_up. Rust evicts a wrongly-set member AFTER the live
     * (actions/mod.rs:1733); the C evicted BEFORE. SHAPE CORRECTED 2026-10-03:
     * Rust's face_up is ONE FLAG PER ZONE (core/zones.rs:501), not per card —
     * `add_card(card_id, _face_up)` DISCARDS its per-card flag (zones.rs:1038),
     * so no per-card value exists to store. Hence [2], indexed by SEAT.
     * Declaring it [RB_MAX_LIVE_CARDS] invited per-card writes that Rust cannot
     * produce. ORDERING also matters: the flag must be written BEFORE
     * rb_check_timing (Rust phases.rs:343 writes, :349 checks) — writing it
     * afterwards silently does nothing. */
    uint8_t  live_face_up[2];   /* per SEAT */

    /* TOMBSTONE 2026-10-04 (field census): `constant_need_heart_global[4096][8]`
   and `constant_need_heart_global_set[4096][8]` are DELETED -- 98 304 bytes, the
   single largest dead allocation in GameState. Both were 0 writers and 0 readers,
   and the claim in the old comment above them was FALSE on two counts, both
   hand-proven:
     (1) "consumed by modify_required_hearts_global which is reachable from 3
         abilities" -- there are NO consumers. The C's
         `modify_required_hearts_global` (src/turn/triggers.c:954 CONSTANT arm,
         src/ability/effects/executor.c:222 effect arm) applies the amount DIRECTLY
         to each live card and never reads a ledger; Rust's does the same and does
         not read `constant_global_need_heart` either.
     (2) the member was on the WRONG STRUCT. Rust's twin is
         `GameModifiers.constant_global_need_heart` (core/game_modifiers.rs:194),
         i.e. on `mods`, not on GameState.
   Rust's gameplay life for the ledger is a take/compare/reapply inside
   `recalculate_constants` (core/game_state/modifiers.rs:613-626) whose only net
   effect is to re-establish `mods.need_heart_modifiers`, which the C already
   tracks through `mods.constant_need_heart` (the Per-player live TOTAL block
   below) -- that member is live and untouched. Rust's only READER of the field is
   the display projection `game/display.rs:1984-1986` (T8 Q2, OUT OF SCOPE), so
   the field is WRITE-ONLY in Rust too.
   Offset note: these were the LAST two members, so deleting them changes
   `sizeof(GameState)` and nothing else -- no later field shifts. The pin in
   tests/test_harness_self.c section A4 and the Makefile:3 figure moved in the
   same change (measured, not estimated). */
} GameState;

/* ── Tracking (engine/src/core/game_state/tracking.rs) ── */
void rb_reset_keyword_tracking(GameState *g);
void rb_record_baton_touch(GameState *g, int player, int arriving_card_id);
int  rb_get_baton_touch_count(const GameState *g, int player);
void rb_clear_baton_touch_tracking(GameState *g);
void rb_add_yell_count_modifier(GameState *g, uint8_t player_slot, int32_t delta);
void rb_refresh_yell_sources(GameState *g);
uint8_t rb_effective_cheer_checks_required(const GameState *g, const char *player_id, uint8_t base);
int rb_perform_cheer_check(GameState *g, const char *player_id, uint8_t blade_count);
int rb_check_required_hearts(const GameState *g);
int rb_is_action_prohibited(const GameState *g, const char *action);
int rb_calculate_play_cost_reduction(const GameState *g, int pl, int hand_count,
                                    int card_id);

typedef struct {
    int available;
    int is_baton_touch;
} RbGeneratedArea;

typedef struct {
    int card_id;
    const RbGeneratedArea *available_areas;
    int n_available_areas;
} RbGeneratedActionParameters;

typedef struct {
    int action_type;
    RbGeneratedActionParameters parameters;
    int has_parameters;
} RbGeneratedAction;

typedef struct {
    RbGeneratedAction *actions;
    int count;
} RbGeneratedActionList;

RbGeneratedActionList rb_generate_action_candidates(const GameState *state);

/* ── Ability queue drain + owner lookup (engine/src/ability_queue.rs) ── */
int rb_owner_of_card(const GameState *g, int cid);
int rb_drain_ability_queue(GameState *g);
void rb_look_resume(GameState *g, int actor, int selected_idx, const char *destination, int is_select);
void rb_look_resume_indices(GameState *g, int owner, const int *indices, int n_indices,
                            const char *destination, const char *remainder_destination,
                            int discard_remaining, int is_select);
void rb_look_reset_all(void);
int rb_look_remove(int pl, int cid);
void rb_look_add(int pl, int cid);
void rb_look_clear(int pl);
/* Queue introspection / control (mirrors AbilityQueue methods) */
int rb_queue_is_idle(const GameState *g);
int rb_queue_has_entry_with_id(const GameState *g, int card_id, int ability_idx);
int rb_queue_start_next(GameState *g);
void rb_queue_complete_current(GameState *g);
int  rb_queue_make_entry(GameState *g, int card_id, int ability_idx);
int  rb_queue_enqueue(GameState *g, int card_id, int ability_idx,
                      int ability_index, const char *player_id,
                      const char *card_no);
int rb_queue_is_entry_available(const GameState *g, int idx);
int rb_queue_current_entry(const GameState *g);
void rb_queue_promote_entry(GameState *g, int from_index);
void rb_queue_promote_entry_by_abs(GameState *g, int absolute);
void rb_queue_set_current_entry(GameState *g, int absolute);
int rb_queue_has_pending_actions(const GameState *g);
void rb_queue_set_pending_actions(GameState *g, int count);
void rb_queue_save_pending_actions(GameState *g, int count);
void rb_queue_store_pending_actions(GameState *g, AbilityEffect *const *actions, int count);
int rb_queue_resume_pending_actions(GameState *g);
int rb_queue_take_pending_actions(GameState *g);
void rb_resume_position_change(GameState *g, int actor, const AbilityEffect *e, int host_cid, int selected_idx);

/* ── Ability queue: pop_constant_context / take_resolver / has_resolver ──
   Mirrors AbilityQueue methods for constant-evaluation context and resolver
   persistence. The C queue model uses a flat array with no per-entry resolver,
   so take_resolver and has_resolver are no-ops. */
void rb_queue_pop_constant_context(GameState *g);
int  rb_queue_take_resolver(GameState *g);
void rb_queue_set_resolver(GameState *g);
int  rb_queue_has_resolver(const GameState *g);

/* ── RNG (xorshift; deterministic given seed) ── */
void rb_seed(uint32_t s);
uint32_t rb_rand(void);
uint32_t rb_rng_checkpoint(void);
void rb_rng_restore(uint32_t state);
size_t rb_rand_range(size_t max);
typedef struct { uint64_t state; } RbLcg;
RbLcg rb_lcg_new(uint64_t seed);
uint64_t rb_lcg_next_u64(RbLcg *rng);
size_t rb_lcg_range(RbLcg *rng, size_t n);

/* ── Setup ── */
int  rb_game_init(GameState *g, const uint32_t *deck0, int n0,
                  const uint32_t *deck1, int n1);
void rb_setup_initial_energy(GameState *g);
void rb_player_set_energy_deck(GameState *g, int pl, const int *cards, int n);
void rb_turn(GameState *g);            /* advance one full turn */
void rb_print_state(const GameState *g);

/* ── Zone helpers (operate on a player's bags) ── */
int  rb_draw(GameState *g, int pl);                 /* draw 1 to hand */
int  rb_draw_energy(GameState *g, int pl);          /* draw 1 to energy zone */
/* Mirror Player::track_deployment (player.rs) — marks deployment so the
   area cannot be targeted for baton touch this turn. Returns whether found. */
int  rb_zone_track_deployment(GameState *g, int pl, int card_id);
int  rb_draw_cards_for_player(RbPlayer *player, uint8_t count, const char *source,
                             const char *destination, const char *card_type_filter,
                             int is_any_number, void *distinct, void *card_db, int self_target_id);
/* Faithful port of draw.rs:execute_draw_wrapper + execute_draw. Mirrors the
    resolver's draw effect (count resolution: static / dynamic / zero-special;
    target both/self/opponent; optional pay-skip gate; any_number; per_unit;
    card_type filter; source/destination routing). */
int  rb_effect_draw_card(GameState *g, int actor, AbilityEffect *e, int host_cid);
/* Trampoline for draw.rs:execute_draw_wrapper. Calls rb_effect_draw_card. */
int  rb_execute_draw_wrapper(GameState *g, int actor, AbilityEffect *e, int host_cid);
/* draw.rs:AbilityResolver::resolve_dynamic_count — resolves a dynamic count
   reference using the GameState transient context. */
int  rb_draw_resolve_dynamic_count(GameState *g, int actor, const AbilityEffect *e, int host_cid);
/* Ported from draw.rs:execute_select_effect — routes a `select` verb to the
    area / heart-color / C6 keep-shuffle / generic card-selection path. */
void rb_effect_select_effect(GameState *g, int actor, AbilityEffect *e, int host_cid);
/* draw.rs:execute_select_heart_color — emit a heart-color choice (or fix the
    color when only one candidate remains). */
void rb_effect_select_heart_color(GameState *g, int actor, int count,
                                  const char **heart_colors, int n_colors, const char *target);
/* draw.rs:execute_select_number — emit a numeric choice 1..max_cost (+67). */
void rb_effect_select_number(GameState *g, int actor, AbilityEffect *e);
/* draw.rs:execute_area_select — emit an area (left/center/right) choice. */
void rb_effect_area_select(GameState *g, int actor, AbilityEffect *e, int host_cid);
/* draw.rs:execute_both_hand_keep_shuffle_under + make_hand_selection_choice +
    move_non_selected_hand_to_deck_bottom — C6 keep-N-shuffle-rest. */
void rb_effect_both_hand_keep_shuffle_under(GameState *g, int actor, AbilityEffect *e, int host_cid);
/* draw.rs:execute_draw_until_count — draw until hand reaches target_count */
void rb_effect_draw_until_count(GameState *g, int actor, AbilityEffect *e);
/* draw.rs:make_card_effect_data — build single-card effect data for resource grant */
typedef struct {
    int card_id; int amount; char color[24];
} RbEffectDataSingleCard;

typedef struct {
    int16_t card_id;
    int16_t amount;
    char *color;
} RbCardEffectItem;

typedef struct {
    int16_t card_id;
    int16_t amount;
    const char *color;
} RbCardEffectItemRef;

typedef enum {
    RB_EFFECT_DATA_HEART_OVERRIDE,
    RB_EFFECT_DATA_SINGLE_CARD,
    RB_EFFECT_DATA_MULTI_CARD,
    RB_EFFECT_DATA_ALL_CARDS,
    RB_EFFECT_DATA_SET_BLADE_COUNT,
    RB_EFFECT_DATA_SURPLUS_HEART,
    RB_EFFECT_DATA_GAIN_ABILITY
} RbEffectDataKind;

typedef struct {
    RbEffectDataKind kind;
    union {
        struct { int16_t card_id; char *color; uint8_t count; } heart_override;
        RbCardEffectItem single_card;
        struct { RbCardEffectItem *items; size_t count; } multi_card;
        struct { int16_t amount; } all_cards;
        struct { int16_t card_id; } set_blade_count;
        struct { bool is_p1; uint8_t old_value; } surplus_heart;
        struct { int16_t card_id; int16_t amount; bool is_live_total; } gain_ability;
    } value;
} RbEffectData;

RbEffectDataSingleCard rb_make_card_effect_data(int card_id, int amount, const char *color);
/* draw.rs:resolve_gain_heart_color — returns a fixed heart color idx, or -1 if a
    choice was emitted / not a heart resource. */
int  rb_resolve_gain_heart_color(GameState *g, int actor, AbilityEffect *e,
                                 const char *resource, int count,
                                 const char **heart_colors, int n_colors, int heart_selection);
void rb_shuffle(int *a, int n);
int  rb_zone_of_str(const char *s, RbZone *out);    /* map zone wire name */
int  rb_member_area_to_index(const char *area);
const char *rb_member_area_to_str(int idx);

/* Main phase action execution (mirrors TurnEngine::execute_main_phase_action).
   Dispatches UseAbility → activate ability, Pass → advance phase, etc. */
int rb_execute_main_phase_action(GameState *g, int action_type, int card_id,
                                 int target_player, int target_idx, int target_zone);
int  rb_play_card(GameState *g, int pl, int hand_idx);
int  rb_play_member(GameState *g, int pl, int hand_idx, int stage_pos); /* to stage */
int  rb_complete_play_with_cost(GameState *g, int pl, int accept); /* answer a paused play-time alt-cost */
int  rb_activate_ability(GameState *g, int pl, int hand_idx);
int  rb_activate_card(GameState *g, int pl, int card_id); /* run the card's 起動 (Activate) ability: cost + effect */
/* Baton-touch support (replace an occupied stage member). */
int  rb_card_arrived_this_turn(const GameState *g, int pl, int card_id);
int  rb_card_has_restriction(const GameState *g, int incoming_cid, int card_id, const char *restriction);
void rb_send_to_waitroom(GameState *g, int pl, int card_id);
/* Restriction (prohibition / cannot-activate) support. */
int  rb_card_is_cannot_active(const GameState *g, int card_id);

/* ── Triggers / phase / live / stats_pipeline ── */
int  rb_trigger_is(const char *triggers, const char *needle);
int  rb_trigger_debut(GameState *g, int pl, int card_id);
void rb_fire_debut(GameState *g, int pl, int card_id);
int  rb_trigger_live_start(GameState *g, int pl);
int  rb_is_trigger_suppressed(GameState *g, int pl, const char *trigger_name);
int  rb_trigger_live_success(GameState *g, int pl);
int  rb_should_trigger_live_success(const GameState *g, int pl);
int  rb_drain_live_success_choices(GameState *g);
int  rb_queue_trigger_abilities(GameState *g, int pl, const char *trigger);
int  rb_fire_auto(GameState *g, int pl);
int  rb_fire_all_auto(GameState *g, int pl);
int  rb_fire_auto_and_pending(GameState *g, int pl);
void rb_record_event(GameState *g, int pl, const char *trig);
int  rb_fire_recorded_auto(GameState *g, int pl);
int rb_process_current_ability(GameState *g);
int rb_process_pending_auto_abilities(GameState *g);
void rb_recalc_constants(GameState *g);
int  rb_register_gained_ability(GameState *g, int card_id, const Ability *ability);
int  rb_remove_gained_ability(GameState *g, int card_id, int index);
int  rb_card_has_ability_trigger_for(GameState *g, int card_id, const char *trigger);
int  rb_ability_is_invalidated(const GameState *g, int card_id, const char *trigger);
int  rb_try_add_ability_invalidation(GameState *g, int card_id, const char *trigger,
                                     const char *duration);
int  rb_card_num_gained_abilities(const GameState *g, int card_id);
const Ability *rb_card_gained_ability(const GameState *g, int card_id, int index);

/* Condition-scalar / group-filter readers used by the constant scans. Both   live in src/core/game_state_abilities.c and are declared here so the
   under-card constant pass in src/turn/triggers.c reads the same fields the
   TAS scan does. `rb_condition_tree_group_names` mirrors
   abilities.rs::condition_tree_group_names — the first non-empty group filter
   anywhere in the condition tree, `group_names` arriving as a string ARRAY. */
const char *rb_cond_get_str(const Condition *c, const char *key);
int  rb_condition_tree_group_names(const Condition *c, const char **out, int max);

/* Gained-aware (card_id, ability_idx) resolution. Printed index below
   RB_GAINED_ABILITY_INDEX_BASE, gained index at or above it. `out` is a
   caller-owned Ability (release with rb_free_ability). The queue processor
   (src/ability/ability_queue.c) must use this, not rb_decode_card_ability,
   or a queued GAINED ability silently resolves to nothing. */
int  rb_resolve_card_ability_at(const GameState *g, int card_id, int ability_idx,
                                Ability *out);
void rb_check_expired_effects(GameState *g, int which);
void rb_advance_phase(GameState *g);

/* rb_rules_clear_turn_state is DEFINED in src/ability/effects/rules.c:2500
 * and CALLED from src/turn/phase.c:753 in the turn rollover, right beside the
 * GameState-side folds of the same three Rust per-turn sets.
 *
 * The prototype lived mid-function inside phase.c and, before the body
 * existed, inside a COMMENT block in both phase.c and live.c -- so the
 * symbol once had two declarations, no definition, and no caller, while the
 * checklist still read `[x]`. The definition now exists, so this is the
 * single declaration and phase.c's local one is redundant.
 *
 * What it clears, against Rust turn/phases.rs:313-317:
 *   :314 turn_limited_abilities_used.clear()
 *        no clear needed -- the C use-limit table is keyed by turn
 *        (ability_queue.c:1258/1267/1282 test q->use_turn != cur_turn),
 *        which is the same guarantee the explicit clear buys Rust.
 *   :315 cannot_activate_members.clear()  -> cannot_activate_members[2] = 0
 *   :316 cannot_live_players.clear()
 *        NOT cleared here, by design: the flag carries a turn stamp
 *        (rules.c:2432-2433 write cannot_live_turn) and its only reader
 *        (rb_rules_cannot_live_player, rules.c:2445) returns 0 when the
 *        stamp is stale. A stamp is what BOTH Rust clear sites produce.
 *   :317 wait_immune_members.clear()      -> n_wait_immune = 0
 * The GameState-side members of the same sets (player_cannot_activate[2],
 * n_cannot_active_cards, n_prohibition_effects) are cleared by phase.c
 * itself, on either side of the call. */
void rb_rules_clear_turn_state(GameState *g);

/* The two readers this clear exists for. They are declared HERE for the
 * first time: both were defined in rules.c (rb_rules_cannot_activate_player
 * at :2451, rb_rules_cannot_live_player at :2441) and called from
 * src/turn/live.c and src/turn/triggers.c, which each had to spell the
 * prototype out locally -- so a caller in any other file, including a
 * test, would get an implicit int() declaration and, at
 * -Wimplicit-function-declaration, no argument checking at all. Same
 * defect class as the rb_gsc_* prototypes above.
 *
 * Both consult a TURN STAMP and return 0 when the stamp is stale, which is
 * what makes rb_rules_clear_turn_state's omission of cannot_live_players
 * correct: the rollover produces the same observable result as Rust's
 * explicit `cannot_live_players.clear()`.
 */
int  rb_rules_cannot_activate_player(GameState *g, int pl);
int  rb_rules_cannot_live_player(GameState *g, int pl);

/* ── check_timing + integrity checks (engine/src/turn/actions.rs:check_timing) ── */
void rb_check_timing(GameState *g);
void rb_check_victory_condition(GameState *g);
void rb_check_invalid_live_cards(GameState *g, int is_p1);
void rb_check_invalid_energy_cards(GameState *g, int pl);
void rb_check_orphaned_under_cards(GameState *g, int pl);
void rb_check_invalid_resolution_zone(GameState *g);
int  rb_check_permanent_loop(GameState *g);
void rb_player_refresh(GameState *g, int pl);

/* ── Card classification (mirrors Rust Card::is_live / is_energy) ── */
int rb_card_is_live(int card_id);
int rb_card_is_energy(int card_id);
int rb_card_is_member(int card_id);

/* ── Card string/classification helpers (mirror engine/src/core/card.rs) ── */
int  rb_card_type_from_str(const char *s);   /* "member_card"→0, "live_card"→1,
                                                 "energy_card"→2, else -1 */
const char *rb_card_type_str(int t);         /* inverse of rb_card_type_from_str */
void rb_card_normalize_no(const char *src, char *out, size_t out_sz);   /* CardDatabase::normalize_card_no */
void rb_card_normalize_name(const char *src, char *out, size_t out_sz); /* CardDatabase::normalize_name */
void rb_map_series_to_group(const char *series, char *out, size_t out_sz); /* map_series_to_group */
int  rb_card_equivalent_rarity(const char *rarity, char *out, size_t out_sz);

/* ── Card impl methods (mirror engine/src/core/card.rs Card impl block) ── */
int rb_card_total_hearts(const Card *c);
int rb_card_has_blade_heart(const Card *c);
int rb_card_has_blade_heart_strict(const Card *c);
int rb_card_has_score_icon(const Card *c);
int rb_card_has_all_blade(const Card *c);
int rb_card_get_score(const Card *c);
const char *rb_effect_exclude_heart_colors_any(const AbilityEffect *e);
const char *rb_effect_heart_colors_any(const AbilityEffect *e);
const char *rb_effect_per_unit_heart_colors_any(const AbilityEffect *e);
int rb_effect_options_any(const AbilityEffect *e, const AbilityEffect **out, int max);
int rb_effect_repeat_limit_any(const AbilityEffect *e);
const char *rb_effect_destination_any(const AbilityEffect *e);
int rb_effect_is_under_self(const AbilityEffect *e);
const char *rb_effect_action_by_any(const AbilityEffect *e);
const char *rb_effect_action_by(const AbilityEffect *e);
const char *rb_effect_picker_any(const AbilityEffect *e);
int rb_effect_non_stackable_any(const AbilityEffect *e, int *out);
int rb_card_need_heart_satisfied(const Card *c, const int *need, const int *provided);
int  rb_check_heart_requirement(const int *need, const int *provided);
int  rb_live_calculate_score(const GameState *g, int pl, int cheer_blade_heart_count,
                              const int *stage_hearts, int constant_total_score_bonus);


/* HeartColor — mirrors engine/src/core/card.rs HeartColor enum + impl */
int rb_heart_color_index(int color);
static inline int rb_heart_index(int color) { return rb_heart_color_index(color); }
int rb_heart_color_from_index(int i);
const char *rb_heart_color_short_label(int color);
const char *rb_heart_color_as_str(int color);
RbHeartColor rb_parse_heart_color(const char *s);

/* ── CardDatabase methods ──────────────────────────────────────────────── */

/* ── card.rs enum string classification helpers ──
   Mirror the Rust `as_str` / `from_str` impls on the enums defined in
   engine/src/core/card.rs. The int encodings follow the Rust enum discriminant
   order (Active=0/Wait=1; Self=0/Opponent=1; HasBladeHeart=0/HasScoreIcon=1/
   HasAllBlade=2; Score=0/Cost=1/Count=2/Equality=3/EnergyRelative=4;
   NoAbility=0/HasAbility=1/HasAbilityType=2/NoAbilityType=3; Self=0/Opponent=1/
   Both=2/Either=3; MemberCard=0/LiveCard=1/EnergyCard=2; Stage=0/Hand=1/…
   DeckTop=3/Discard=4/EnergyZone=5/LiveCardZone=6/SuccessLiveZone=7/
   UnderMember=8/RevealedCards=9). */
const char *rb_card_state_str(int s);          /* CardState */
int         rb_card_state_from_str(const char *s);
const char *rb_comparison_target_str(int s);   /* ComparisonTarget */
int         rb_comparison_target_from_str(const char *s);
const char *rb_card_property_str(int s);       /* CardProperty */
int         rb_card_property_from_str(const char *s);
const char *rb_placement_order_str(int s);     /* PlacementOrder (any_order) */
const char *rb_distinct_type_str(int s);       /* DistinctType */
const char *rb_comparison_type_str(int s);     /* ComparisonType */
int         rb_comparison_type_from_str(const char *s);
const char *rb_ability_filter_str(int s);      /* AbilityFilter */
int         rb_ability_filter_from_str(const char *s);
const char *rb_condition_target_str(int s);    /* ConditionTarget */
const char *rb_condition_card_type_str(int s); /* ConditionCardType */
int         rb_condition_card_type_from_str(const char *s);
const char *rb_location_str(int s);            /* Location */
/* Free functions mirroring card.rs parse_operator / parse_operation.
   Return the Rust enum discriminant, or -1 for an unknown string. */
int rb_parse_operator(const char *s);
int rb_parse_operation(const char *s);
/* Mirror DistinctInfo::is_distinct — string-form branch (the flat C decode
   stores `distinct` as a string; the Boolean branch is not represented). */
int rb_distinct_info_is_distinct(const char *s);

int rb_need_satisfied(const int base_need[8], const int provided[8], int card_id, const RbMods *mods);
void rb_member_original_hearts(const RbMods *mods, int card_id, int out[8]);
void rb_apply_additive_heart_mods(int hearts[8], const RbModifierEntry *mods);
void rb_effective_blade_parts(RbModifierEntry entry, int printed_blade, int *base, int *additive);
void rb_member_heart_detail(const RbMods *mods, int card_id, uint8_t base_arr[8], uint8_t bonus_arr[8]);
int  rb_effective_blade(int card_id, RbModifierEntry entry);

void rb_calc_stage_hearts(const GameState *g, int pl, int out[8]);
void rb_stage_hearts_pipeline(const GameState *g, int pl, int out[8]);
void rb_effective_need_heart(const GameState *g, int live_cid, int out[8]);
int  rb_perform_live(GameState *g, int pl);
int  rb_backtrack_allocate(const int pool[8], const int card_needs[8], int n_cards,
                           int *out_allocs, int max_allocs);
int  rb_card_ok_with_wildcard(const int filled[8], const int need[8]);
void rb_apply_deferred_reyell(GameState *g);
void rb_execute_live_victory_determination(GameState *g);
void rb_move_live_to_success_and_handle_wins(GameState *g);
void rb_handle_live_success_choice(GameState *g, int pl, int selected_index);
void rb_process_player_live_result(GameState *g, int pl, int won, int must_skip, int can_place);
/* ── live.rs standalone helpers (ported) ── */
/* Mirror live.rs::blade_color_to_heart: map a set_blade_type blade color to the
    heart color its icons become (colored 1..6 → same index; All → icon_all idx 7).
    NB: pink (blade_type -1/none) is never passed here — it stays pink at the call site. */
int  rb_blade_color_to_heart(int bc);
/* Mirror live.rs::TurnEngine::score_delta_since: total of (current - prev) score
    modifiers across the given zone cards (cid-indexed parallel arrays of size
    RB_MAX_CARD_IDS; missing entries default to 0, mirroring HashMap get-or-default). */
int  rb_score_delta_since(const int *current, const int *prev, const int *zone_cards, int n);
/* Mirror live.rs::TurnEngine::compute_pregame_scores: compute each player's live
    score from current stage hearts + granted hearts, folding in p1/p2 extra deltas
    (e.g. LiveSuccess-triggered score grants). Uses the shared allocation/verdict path. */
void rb_compute_pregame_scores(const GameState *g, int p1_extra, int p2_extra,
                               int *p1_score, int *p2_score);
/* ── live.rs verdict/surplus helpers (ported) ── */
/* Mirror live.rs::populate_live_verdicts (operates on the snapshot filled by
    allocate_and_verdict — recomputes each live's pass/fail with the same
    acceptance rules as rb_allocations_pass). */
void rb_populate_live_verdicts(GameState *g);
/* Mirror live.rs::finalize_snapshot_fields — fill each snapshot's total_score /
    success flag from the victory determination result. */
void rb_finalize_snapshot_fields(GameState *g, int p1_won, int p2_won,
                                 int p1_score, int p2_score);
/* Mirror live.rs::compute_surplus_and_flags — per-color surplus into each
    snapshot and the GameState no-excess / surplus-count flags. */
void rb_compute_surplus_and_flags(GameState *g, int p1_won, int p2_won);
/* Effects  Everb handlers */
void rb_effect_move_cards(GameState *g, int actor, AbilityEffect *e);
/* Mirror move_cards.rs::move_from_under_member — pull cards out of a member's
    under_cards and place them into dst. validate(card_id) must return nonzero for
    the card to be moved (NULL = accept all). Returns count moved, or -1 on a
    missing/invalid index. */
int  rb_move_from_under_member(GameState *g, int actor, const int *indices, int n_indices,
                                int (*validate)(int), const char *dst, const char *target);
/* Mirror move_cards.rs::execute_selected_energy_zone_cards — mark the energy-zone
    cards at the given indices as "wait" (clearing their modifiers) and decrement the
    player's active energy count by the marked count. */
void rb_effect_selected_energy_zone_cards(GameState *g, int actor, const int *indices, int n_indices);
/* Mirror move_cards.rs::drain_under_cards_to_energy_zone — route every card
    tucked under the given stage member to the energy zone (if it is an energy
    card, marked wait) or the waitroom. Returns the number of cards moved. */
int  rb_drain_under_cards_to_energy_zone(GameState *g, const char *target, int stage_idx);
void rb_effect_gain_surplus_heart(GameState *g, int actor, const AbilityEffect *e);
/* Mirror cost.rs::handle_pay_cost_all_discard — "may discard your whole hand" cost. */
int  rb_effect_pay_cost_all_discard(GameState *g, int actor, const AbilityEffect *e);
void rb_effect_look_at(GameState *g, int actor, AbilityEffect *e);
void rb_effect_reveal(GameState *g, int actor, AbilityEffect *e);
void rb_effect_reveal_per_group(GameState *g, int actor, AbilityEffect *e);
void rb_effect_reveal_until_live_card(GameState *g, int actor, AbilityEffect *e);
void rb_effect_reveal_until_chosen_card(GameState *g, int actor, AbilityEffect *e);
void rb_effect_reveal_until_target(GameState *g, int actor, AbilityEffect *e);
void rb_effect_select(GameState *g, int actor, AbilityEffect *e);
void rb_effect_select_cards(GameState *g, int actor, AbilityEffect *e);
void rb_effect_look_and_select(GameState *g, int actor, AbilityEffect *e);
int  rb_looked_at_pool(int pl, int *out_ids, int max);
void rb_gain_ability(GameState *g, int actor, AbilityEffect *e);
void rb_gain_ability_from_source(GameState *g, int actor, AbilityEffect *e, int host_cid);
/* ⛔ 2026-10-04, CORRECTED 2026-10-05: four declarations with no definition
 * were deleted from this header - rb_invalidate_ability,
 * rb_activate_ability_effect, rb_effect_modify_hearts, rb_cost_clear_deferred.
 * Their bodies were deleted on 2026-10-02 and the prototypes outlived them, so
 * any caller was a LINK ERROR surfacing as a linker complaint in unrelated
 * code. Do NOT re-add without reading CENSUS_wiring.md. A prototype with no
 * definition is worse than neither.
 *
 * ⚠️ THIS NOTE WAS FALSE AS WRITTEN. It also claimed
 * rb_execute_effect_via_registry and rb_translated_execute_custom_effect had
 * been deleted; both prototypes were still present and both are undefined
 * (52 .o scanned with `nm --defined-only`: zero definitions). They were
 * actually deleted 2026-10-05 -- see the note at their former line.
 *
 * ⚠️ AND IT OVERCLAIMED: rb_invalidate_ability, rb_activate_ability_effect and
 * rb_effect_modify_hearts are NOT deleted. They are still declared, still
 * undefined, and still landmines. Their declarations were out of scope for the
 * 2026-10-05 change and remain open.
 *
 * (rb_load_rom is NOT one of these: its declaration is correctly
 * wrapped in the same #ifdef RB_ROM_STRINGS as its definition.) */
void rb_suppress_ability_trigger(GameState *g, int actor, AbilityEffect *e, int host_cid);
void rb_tick_gained(GameState *g);
int  card_matches_card_type_filter(int card_idx, const char *filter);
void rb_emit_choice(GameState *g, int actor, RbChoiceKind kind,
                    const char *zone, const char *card_type,
                    int count, int allow_skip, const char *target);
void rb_effect_change_state(GameState *g, int actor, AbilityEffect *e, int host_cid);
void rb_effect_position_change(GameState *g, int actor, AbilityEffect *e, int host_cid);
void rb_effect_rotation(GameState *g, int actor, AbilityEffect *e);
void rb_effect_modify_cost(GameState *g, int actor, AbilityEffect *e, int host_cid);
void rb_effect_set_cost(GameState *g, int actor, AbilityEffect *e, int host_cid);
void rb_effect_set_blade_type(GameState *g, int actor, AbilityEffect *e, int host_cid);
void rb_effect_set_blade_count(GameState *g, int actor, AbilityEffect *e, int host_cid);
void rb_effect_set_heart_type(GameState *g, int actor, AbilityEffect *e, int host_cid);
void rb_effect_set_heart_copy_from_under(GameState *g, int actor, AbilityEffect *e, int host_cid);
void rb_effect_set_card_identity(GameState *g, int actor, AbilityEffect *e, int host_cid);
void rb_effect_set_card_identity_all_regions(GameState *g, int actor, AbilityEffect *e, int host_cid);
void rb_effect_reduce_live_card_set_limit(GameState *g, int actor, AbilityEffect *e, int host_cid);
void rb_effect_specify_heart_color(GameState *g, int actor, AbilityEffect *e, int host_cid);
void rb_effect_set_cost_to_use(GameState *g, int actor, AbilityEffect *e, int host_cid);
void rb_effect_all_blade_timing(GameState *g, int actor, AbilityEffect *e, int host_cid);
void rb_effect_activation_cost(GameState *g, int actor, AbilityEffect *e, int host_cid);
/* rb_effect_modify_hearts: declaration DELETED 2026-10-04 (body gone since
   2026-10-02). See the block note above rb_suppress_ability_trigger. */
void rb_effect_energy_placement(GameState *g, int actor, AbilityEffect *e);
void rb_effect_energy_state_change(GameState *g, int actor, AbilityEffect *e);
void rb_util_push_temporary_effect(GameState *g, const char *effect_type,
                                   const char *duration, const char *target_player_id,
                                   const char *description);
int  rb_execute_modify_score(GameState *g, int actor, AbilityEffect *e);
int  rb_execute_modify_required_hearts(GameState *g, int actor, AbilityEffect *e);
void rb_execute_modify_required_hearts_standard(GameState *g, int actor,
        const char *operation, int value, const char **heart_colors, int n_colors,
        const char *target, const char *effect_text);
int  rb_execute_modify_yell_count(GameState *g, int actor, AbilityEffect *e);
int  rb_execute_modify_limit(GameState *g, int actor, AbilityEffect *e);
int  rb_execute_modify_required_hearts_success(GameState *g, int actor, AbilityEffect *e);
void rb_log_set_enabled(int enabled);
void rb_log_push_verdict(const char *text, const char *kind, int passed);
int  rb_log_buffer_len(void);
void rb_log_clear_verdicts(void);

/* ── AbilityLogItem verdict buffer (engine/src/ability/log.rs) ──
   Mirrors the Rust AbilityLogItem tagged enum. Each entry is a RbAbilityLogItem
   with a kind discriminator and a union of the three variant payloads.
   Condition children are a heap-allocated array (mirrors Vec<AbilityLogItem>);
   use rb_log_free_item to release a drained item. */
#define RB_LOG_MAX_CHILDREN 8

typedef struct RbAbilityLogItem RbAbilityLogItem;

#define RB_LOG_KIND_CONDITION 0
#define RB_LOG_KIND_COST      1
#define RB_LOG_KIND_EFFECT    2

struct RbAbilityLogItem {
    int kind;
    union {
        struct {
            char *text;
            char *condition_type;
            char *expectation;
            char *actual;
            int passed;
            int n_children;
            RbAbilityLogItem *children;
        } condition;
        struct {
            char *text;
            char *expectation;
            char *actual;
            int passed;
            int optional;
        } cost;
        struct {
            char *text;
            char *action;
            char *details;
        } effect;
    } as;
};

typedef RbAbilityLogItem RbLogItem; /* backward-compatible alias */

int  rb_log_drain_verdicts(RbAbilityLogItem *out, int max);
/* log.rs: drain only the entries appended at/after `start_index` — a snapshot of
   rb_log_buffer_len() taken before a resolution began. Declared once here so
   resolver.c and the log tests stop hand-declaring it locally (resolver.c once
   carried a wrong one-arg prototype that would have linked against garbage). */
int  rb_log_drain_verdicts_since(int start_index, RbAbilityLogItem *out, int max);

/* Variant-specific push helpers (mirrors AbilityLogItem variants + children). */
void rb_log_push_verdict_condition(const char *text, const char *condition_type,
                                   const char *expectation, const char *actual,
                                   int passed);
void rb_log_push_verdict_cost(const char *text, const char *expectation,
                              const char *actual, int passed, int optional);
void rb_log_push_verdict_effect(const char *text, const char *action, const char *details);
void rb_log_push_verdict_child(int parent_index, const RbAbilityLogItem *child);
void rb_log_push_verdict_item(const RbAbilityLogItem *item);

/* Deep-free a drained item (release children heap array). */
void rb_log_free_item(RbAbilityLogItem *item);

/* ── Ability tracing (engine/src/ability/debug.rs: ABILITY_DEBUG) ──
   Mirrors the Rust `ABILITY_DEBUG` atomic gate: diagnostic traces are compiled
   in permanently (they are part of the engine, per AGENTS.md) but only print
   when tracing is switched on, so a full suite run stays quiet. */
int  rb_ability_debug_enabled(void);
void rb_ability_debug_set(int enabled);

/* ── AbDebug buffered tracer (engine/src/ability/debug.rs: AbDebug) ──
   Per-resolution debug helper that buffers formatted trace lines and flushes
   them to the rule log or structured log. Mirrors Rust AbDebug::p / ability /
   condition / cost_pay / effect + flush helpers. */
/* Complete, not opaque. Rust's AbDebug is a STACK LOCAL passed by &mut
 * (debug.rs:126-133), so callers declare one by value — condition.c does
 * `RbAbDebug dbg;` in three places. An opaque forward declaration makes those
 * declarations fail with "storage size isn't known". */
typedef struct RbAbDebug { int indent; } RbAbDebug;
void rb_abdebug_init(RbAbDebug *d);
void rb_abdebug_p(RbAbDebug *d, const char *tag, const char *msg);
void rb_abdebug_flush_to_rule_log(RbAbDebug *d);
void rb_abdebug_flush_to_structured_log(RbAbDebug *d);
void rb_abdebug_ability(RbAbDebug *d, const char *card_name, const char *card_no,
                        const char *card_id, const Ability *ability);
void rb_abdebug_condition(RbAbDebug *d, const Condition *cond, int actual,
                          int threshold, int passed);
void rb_abdebug_cost_pay(RbAbDebug *d, const AbilityEffect *cost, int ok);
void rb_abdebug_effect(RbAbDebug *d, const AbilityEffect *effect);

/* ── Dynamic count resolution (engine/src/ability/dynamic_count.rs) ── */
int  rb_resolve_dynamic_count(const struct GameState *g, int owner, int host_cid,
                               const char *reference, const char *base_reference,
                               const char *count_type, const char *calculation,
                               int calculation_value, int owner_on_p1,
                               const int *moved, int n_moved,
                                const int *selected, int n_selected,
                                int last_draw_count);
/* Resolve an effect's count: static `count`, or (if -1) decode the DynamicCount
   params stored as extra_kv and call rb_resolve_dynamic_count. */
int rb_effect_count(const GameState *g, int actor, int host_cid, const AbilityEffect *e,
                     int last_draw_count);

/* ── Shared card/zone/comparison helpers (engine/src/ability/util.rs) ── */
int  rb_compare_counts(const char *operator, int actual, int expected);
/* Decode a heart_color from an effect's extra fields (engine.c). */
int heart_color_of(AbilityEffect *e, int dflt);
int  rb_card_matches_type(int card_id, const char *filter);
/* util.rs helpers (effect-field readers / target resolution / cost-threshold) */
int  rb_heart_gain_per_entry(int total, int n_colors);
int  rb_is_all_heart_type(const AbilityEffect *e);
const char *rb_constant_per_unit_zone(const AbilityEffect *e);
int  rb_target_player_index(const char *target, const char *master);
const char *rb_target_player_label(const char *target, const char *master);
int  rb_activation_position_index(const char *p);
int  rb_cost_threshold_met(const Card *card, const AbilityEffect *e);
int  rb_card_matches_cost_limit(int card_id, int cost_limit, const char *comparison);
int  rb_card_matches_heart_colors(int card_id, const char **heart_colors, int n);
int  rb_card_matches_all_heart_colors(int card_id, const char **heart_colors, int n);
int  rb_card_matches_name_fragments(int card_id, const char **fragments, int n);
int  rb_card_matches_characters(int card_id, const char **names, int n);
int  rb_stage_position_index(const char *pos);
int  rb_count_in_zone(const GameState *g, int pl, const char *zone);
int  rb_remove_card_from_zone(GameState *g, int pl, int card_id, const char *zone);
int  rb_place_card_in_zone(GameState *g, int pl, int card_id, const char *zone, int vacated_area);
int  rb_move_card(GameState *g, int pl, int card_id, const char *src, const char *dst, int vacated_area);
int  rb_move_cards(GameState *g, int pl, const int *card_ids, int n, const char *src, const char *dst, int vacated_area);
int  rb_resolve_indices_to_ids(const GameState *g, int pl, const char *zone, const int *indices, int n_idx, int *out);
/* card_property predicates (card.rs::has_blade_heart/has_score_icon/has_all_blade) */
int  rb_orientation_matches_state(const char *orientation, const char *state);
int  rb_card_matches_group_str(int card_id, const char *group_name);
/* Card::unit per card_binary.rs:175-182 — THE canonical unit accessor.
   cards.json carries `unit` on 2,198 of 2,639 cards (0xFFFF in cards.bin marks
   the 441 with no unit), and it is what the 『X』 names in group_names /
   exclude_group_names / counted_by actually name. Use this, never a bare
   rb_card_string(card->unit_idx), which silently yields "" for both "absent"
   and "index 0xFFFF". Returns "" when the card has no unit. */
const char *rb_card_unit_name(int card_id);
/* Card::group per card_binary.rs:153-174. The blob's group string is empty for
   every record, so this derives the group from the series like Rust does.
   Use this, never rb_card_string(card->group_idx), which resolves to "". */
const char *rb_card_group_name(int card_id);
const char *rb_card_series_to_group(const char *series);
int  rb_card_matches_any_group(int card_id, const char **groups, int n);
int  rb_card_matches_name_constraint(int card_id, const char *name_constraint);
void rb_set_card_identity(int cid, const char *name);
int  rb_card_matches_identity_str(int card_id, const char *group_name);
int  rb_card_at_position(const struct GameState *g, int pl, const char *pos);
int  rb_pos_to_area(const char *pos);
int  rb_zone_cards(const struct GameState *g, int pl, const char *zone,
                   int *out_ids, int max);
int  rb_get_selection_indices(const int *cards, int n, const char *card_type,
                              const char *group, int self_target_only,
                              int activating_card, int *out_idxs, int max);
int  rb_classify_selection(const int *indices, int n, int count, int is_all);
int  rb_resolve_selection(const int *cards, int n, const char *card_type,
                          const char *group, int count, int is_all,
                          int self_target_only, int activating_card);
int  rb_zone_remove_at_indices(GameState *g, int pl, const char *zone,
                                const int *indices, int n_indices);
int  rb_stage_first_empty(const int stage[RB_STAGE_SIZE]);

/* ── MemberArea wire helpers (engine/src/core/zones.rs:MemberArea) ── */
uint8_t rb_member_area_to_tag(int idx);    /* 0→1,1→2,2→3; 0 if invalid */
int     rb_member_area_from_tag(uint8_t tag); /* 1→0,2→1,3→2; -1 if invalid */

#define RB_MAX_MATCH_OUT 512

/* Filter struct for matching_ids / constant_per_unit_units (mirrors CardFilter).
   Only the fields needed by the C port are represented; None/unspecified = match anything. */
typedef struct {
    char  card_type[32];
    char  group[64];
    int   has_group;
    int   cost_limit;
    char  cost_op[8];
    int   has_cost_limit;
    char  characters[256];
    int   has_characters;
    char  exclude_characters[256];
    int   has_exclude_characters;
    char  heart_colors[8][24];
    int   n_heart_colors;
    int   require_all_heart_colors;
    int   heart_color_count;
    int   need_heart_total;
    char  need_heart_operator[8];
    char  need_heart_color[24];
    int   has_need_heart_total;
    char  name_fragments[8][64];
    int   n_name_fragments;
    int   original_blade_limit;
    char  original_blade_op[8];
    int   has_original_blade;
    char  ability_filter[32];
    char  ability_filter_triggers[8][32];
    int   n_ability_filter_triggers;
    int   negation;
    int   exclude_self_id;
    int   has_exclude_self;
    int   cost_total;
    char  cost_total_op[8];
    int   has_cost_total;
    int   cost_values[16];
    int   n_cost_values;
    int   distinct;
    int   blade_limit;
    char  blade_op[8];
    int   has_blade_limit;
    int   has_filter;
    /* ── Fields Rust's CardFilter carries and the C filter had NO home for ──────
     * The `or_ability_filters` extra IS decoded (vm.c stringifies the
     * array-of-OBJVAR into JSON), but there was nowhere to put the branches, so
     * every caller going through rb_effect_filter_subset + rb_matching_* silently
     * DROPPED the OR-ability filter. `has_filter`'s corresponding Rust term
     * (util.rs:1443) was therefore unreachable.
     * `card_property` had the same problem: score.rs:358-360's explicit
     * `target_filter.card_property = None` reset was satisfied only ACCIDENTALLY,
     * because there was no member to set.
     * These land so the one true filter builder can carry them; look.c, score.c
     * and move.c each hand-roll a partial build of exactly these and should be
     * reduced to a call once populated. */
    char  or_ability_filters[4][192];
    int   n_or_ability_filters;
    char  card_property[64];
    /* ── CardFilter carriers Rust has and C had NO member for ─────────────────
     * util.rs READS all six (has_filter at :1459-1498, the matching terms at
     * :1518/:1531/:1744/:1790) and card_filter_from_effect CANNOT populate them
     * without these. Until they exist the producer half would be dead code.
     * Keys are all present in the live abilities_strings.bin.
     *   original_heart_total / _operator   util.rs:1441-1443, fed :1906-1907
     *   exclude_group_names                 util.rs:1460, :1531-1537
     *   cost_limit_min / cost_limit_max    util.rs:1462-1463, :1744/:1749
     *   groups (the WHOLE list, vs `group` which is only .first())
     *                                      util.rs:1459, :1518-1529
     *   exclude_cards                       util.rs:1489, :1714
     *   exclude_names                       util.rs:1498, :1717
     */
    int   original_heart_total;
    char  original_heart_operator[8];
    char  exclude_group_names[64];
    int   cost_limit_min;
    int   cost_limit_max;
    int   has_cost_limit_min;
    int   has_cost_limit_max;
    char  groups[8][64];
    int   n_groups;
    int   exclude_cards[RB_MAX_ZONE];
    int   n_exclude_cards;
    char  exclude_names[8][64];
    int   n_exclude_names;
} RbCardFilter;

int rb_effect_filter_subset(const AbilityEffect *e, RbCardFilter *out);
/* rb_condition_filter_subset's declaration is GONE (checklist G3): the
   definition and this line were removed together. It had zero callers and Rust
   has no Condition-side CardFilter builder. */
/* CardFilter subset built from a card-selecting source. This used to be a bare
   `#define rb_card_filter_subset(e, out) rb_effect_filter_subset((e), (out))`,
   which was NOT a dangling declaration — the macro expanded to the fully
   implemented rb_effect_filter_subset (src/core/card.c) and tests/
   test_ported_generated.c calls it that way. But because it had no function
   body of its own, tools/dep_audit.py reported it under "declared in rabuka.h
   but never defined (MISSING)". Spelling it as a real static inline gives it a
   definition, keeps the two-argument call sites source-compatible, and clears
    the audit. A Condition argument has no home here; see the note above. */
static inline int rb_card_filter_subset(const AbilityEffect *e, RbCardFilter *out) {
    return rb_effect_filter_subset(e, out);
}
const char *rb_effect_target_name(const AbilityEffect *e);
const char *rb_effect_source_or(const AbilityEffect *e, const char *default_source);
int rb_effect_count_or(const AbilityEffect *e, int default_count);
int rb_effect_value_or_count(const AbilityEffect *e, int default_value);

/* Heart-all wildcard key ("heart00"). */
#define RB_HEART_ALL_KEY "heart00"

/* Matching helpers using the RbCardFilter struct. */
int rb_matching_ids(const RbCardFilter *f, const int *cards, int n, int *out, int max);
int rb_count_matching_filter(const RbCardFilter *f, const int *cards, int n);
int rb_matching_indices_filter(const RbCardFilter *f, const int *cards, int n, int *out_idx, int max);

/* constant_per_unit_units — compute the units part of a constant per_unit gain. */
int rb_constant_per_unit_units(const AbilityEffect *effect, const GameState *g, int pl,
                               int host_card_id);
/* resolve_per_unit_count — count matching cards in the per-unit zone for scaling. */
int rb_resolve_per_unit_count(const GameState *g, int pl, const char *per_unit_type,
                              const char *card_type, const char *group,
                              const char *state_filter, int host_card_id);

/* Stage-first-empty helper (center-priority: 1, 0, 2). */
int rb_stage_first_empty(const int stage[RB_STAGE_SIZE]);

/* ── Duration / distinct-name helpers (engine/src/ability/util.rs) ── */
/* Mirror util.rs::DistinctType (CardName/True/Distinct are all "dedupe" variants). */
typedef enum {
    RB_DISTINCT_NONE = 0,
    RB_DISTINCT_CARDNAME,
    RB_DISTINCT_TRUE,
    RB_DISTINCT_DISTINCT
} RbDistinctType;

/* Mirror util.rs::parse_duration — maps a duration string to an RB_TEMP_* kind. */
int  rb_parse_duration(const char *s);
/* ⛔ DECLARATION DELETED 2026-10-04. `extern const char *RB_KNOWN_GROUPS[5]`
 * (the canonical group taxonomy, per Rust `util.rs:956`) was declared here and
 * **DEFINED NOWHERE in `src/`** — a dangling extern, i.e. a link error waiting
 * for the first caller. Nothing referenced it, which is exactly why it survived.
 *
 * The taxonomy itself is NOT missing from the engine:
 * `rb_card_series_matches_group` / `card_matches_group_str` recognise the five
 * groups inline. Only the exported table was absent. If a caller needs it, give
 * it a real definition alongside those functions rather than reviving this. */
/* Mirror util.rs::distinct_should_dedupe. */
int  rb_distinct_should_dedupe(RbDistinctType d);
/* Mirror util.rs::count_distinct_member_name_units — Q278/Q279 joint-aware
    distinct-name count (single-name cards dedup; a joint card adds one unit
    only when it introduces a name not already present as a single name). */
int  rb_count_distinct_member_name_units(const int *cards, int n);
/* Mirror util.rs::apply_distinct_filter — dedupes `cards` by normalized name
    when `d` is a dedupe variant, otherwise copies through. Returns out count. */
int  rb_apply_distinct_filter(const int *cards, int n, RbDistinctType d,
                              int *out, int max);
/* Mirror util.rs::CardFilter::check_card_property — single-property predicate
    (has_blade_heart / has_score_icon / has_all_blade), negation-aware. */
int  rb_check_card_property(const char *prop, int negation, const Card *c);
/* Mirror util.rs::filter_current_blade — post-filter candidate ids by their
    CURRENT blade total (base, or set + additive modifiers from GameState.mods). */
int  rb_filter_current_blade(const int *cands, int n, const GameState *g,
                             int blade_limit, const char *op, int *out, int max);

/* ── phases.rs: _3ds_tdbg / log_turn_start ──
    Mirrors TurnEngine debug-trace and turn-start logging helpers. No-op in
    the C port (logging infrastructure not available). */
void rb_3ds_tdbg(const char *msg);
void rb_log_turn_start(GameState *g);

/* ── Stage under-card / placement helpers (engine/src/core/zones.rs:Stage) ── */
void rb_stage_place_under_card(RbPlayer *player, int area, int card_id);
int  rb_stage_get_under_cards(const RbPlayer *player, int area, int *out, int max);
int  rb_stage_under_cards_with_hosts(const RbPlayer *player, int *out_under, int *out_host, int max);
int  rb_stage_recycle_under_cards(GameState *g, int pl, int area,
                                  int *out_wait, int *n_wait,
                                  int *out_energy, int *n_energy, int max);
int  rb_stage_can_place_card(const GameState *g, int pl, int card_id);
int  rb_stage_formation_change(GameState *g, int pl,
                                const int *from_areas, const int *to_areas, int n);
int  rb_resolve_rps_if_both_chosen(GameState *g);

/* ── Zone bag helpers (engine/src/core/zones.rs: Energy/Live/Hand/Waitroom/...) ── */
int  rb_has_cannot_baton_touch_protection(int incoming_card_id, int existing_card_id);
int  rb_energy_can_place_card(const RbPlayer *player, int card_id);
int  rb_energy_add_card(RbPlayer *player, int card_id);
int  rb_energy_pay(RbPlayer *player, int amount);
void rb_energy_activate_all(RbPlayer *player);
int  rb_energy_active_count(const RbPlayer *player);
int  rb_energy_deck_draw(GameState *g, int pl);
int  rb_energy_deck_is_empty(const GameState *g, int pl);
void rb_energy_set_active_count(RbPlayer *player, int count); /* EnergyZone::set_active_count */
void rb_energy_add_active(RbPlayer *player, int delta);       /* EnergyZone::add_active (saturating) */
void rb_energy_sub_active(RbPlayer *player, int delta);       /* EnergyZone::sub_active (saturating) */
int  rb_live_can_place_card(const RbPlayer *player, int card_id);
int  rb_live_add_card(RbPlayer *player, int card_id);
int  rb_live_clear(RbPlayer *player, int *out, int max);
int  rb_live_len(const RbPlayer *player);
void rb_hand_add(RbPlayer *player, int card_id);
int  rb_hand_remove_card(RbPlayer *player, int index);
int  rb_hand_len(const RbPlayer *player);
int  rb_hand_is_empty(const RbPlayer *player);
void rb_waitroom_add(RbPlayer *player, int card_id);
int  rb_waitroom_take_all(RbPlayer *player, int *out, int max);
void rb_waitroom_shuffle(GameState *g, int pl);
int  rb_waitroom_len(const RbPlayer *player);
void rb_waitroom_remove_card(RbPlayer *player, int card_id);
void rb_success_add(RbPlayer *player, int card_id);
int  rb_success_len(const RbPlayer *player);
void rb_resolution_add(GameState *g, int card_id);
/* Owner-recording form. `rb_resolution_add` keeps calling this with owner = the
 * active seat so existing behaviour is unchanged until an owner is threaded. */
void rb_resolution_add_for_owner(GameState *g, int card_id, int owner);
int  rb_resolution_clear(GameState *g, int *out, int max);
int  rb_resolution_len(const GameState *g);
void rb_record_card_movement(GameState *g, int card_id, int from_zone, int to_zone, int causer, int target);

/* ── card appearance source (engine/src/core/game_state/modifiers.rs:1377-1404) ──
   `source` is an RbZone value (RB_ZONE_DISCARD for a 控え室/WAITROOM debut,
   RB_ZONE_HAND for a hand debut); a negative value means "no source" and
   records nothing, mirroring Rust's `if !source.is_empty()` guard. */
void rb_record_card_appearance(GameState *g, int card_id, int source);
int  rb_has_card_appeared_this_turn(GameState *g, int card_id);
void rb_clear_card_appearance_tracking(GameState *g);
/* First recorded source zone for `card_id` (Rust get_card_appearance_source,
   which finds the FIRST match and does not dedupe on push). Returns 0 and
   leaves *out untouched when the card has no recorded source. */
int  rb_get_card_appearance_source(const GameState *g, int card_id, RbZone *out);
/* Does the source recorded for `card_id` satisfy the ability `source` wire
   string (e.g. "discard" / "waitroom" / "hand")? 0 when nothing was recorded,
   which is how a never-recorded debut fails an 控え室から登場 gate. */
int  rb_appearance_source_matches(const GameState *g, int card_id, const char *expected_source);

/* ── Effect execution (public for testing / harness) ── */
void rb_execute_effect(GameState *g, int actor, AbilityEffect *e);
/* Like rb_execute_effect but carries the resolving card id (Rust activating_card)
   so per-card modifiers (blade/heart) attribute correctly. */
void rb_execute_effect_ex(GameState *g, int actor, AbilityEffect *e, int host_cid);

/* ── Choice API (portable shim calls these instead of reading GameState directly) ── */
int       rb_has_pending_choice(const GameState *g);
const RbChoice *rb_get_pending_choice(const GameState *g);
int       rb_resume_with_choice(GameState *g, int selected_idx); /* 0..count-1, -1=skip */
int       rb_resume_with_choice_indices(GameState *g, const int *indices, int n_indices);
void rb_clear_pending_choice(GameState *g);
void rb_queue_set_pending_choice(GameState *g, const RbChoice *choice);
void rb_queue_pause_for_choice(GameState *g, const RbChoice *choice);

/* ── Ability cost payment (engine/src/ability/cost.rs) ── */
int rb_pay_cost(GameState *g, int actor, const AbilityEffect *cost);
int rb_validate_cost(const GameState *g, int actor, const AbilityEffect *cost);
int rb_pay_deferred_costs(GameState *g, int actor, const AbilityEffect *cost);
/* rb_invalidate_ability, rb_activate_ability_effect, rb_effect_modify_hearts:
   DECLARATIONS DELETED 2026-10-05 (main). Their bodies went on 2026-10-02 and
   the prototypes outlived them -- verified zero definitions across all objects
   via `nm --defined-only` plus a source scan. A prototype with no definition is
   a live link landmine: the first caller added anywhere gets a linker error
   that names unrelated code. The 2026-10-04 tombstone claimed these were
   already deleted; they were not. Do NOT re-add without reading
   CENSUS_wiring.md -- they may be dead for a reason. */
/* rb_execute_effect_via_registry and rb_translated_execute_custom_effect:
   DECLARATIONS DELETED 2026-10-05. The 2026-10-04 tombstone below CLAIMED
   these two were already deleted; they were not -- both prototypes were still
   live at this line and neither has a definition anywhere (verified: zero
   definitions across all 52 .o via `nm --defined-only`, and zero definitions
   by source scan). A prototype with no definition is a live link landmine.
   Tombstone also listed rb_invalidate_ability, rb_activate_ability_effect and
   rb_effect_modify_hearts as deleted; those three are STILL declared three
   lines up and are equally undefined -- noted here rather than deleted,
   because they are outside this change's scope. (rb_load_rom is NOT one of
   these: its declaration is correctly wrapped in the same #ifdef RB_ROM_STRINGS
   as its definition.) */
int rb_handle_optional_cost_payment(GameState *g, int actor, const AbilityEffect *cost, int pay);
int rb_handle_pay_cost_all_discard(GameState *g, int actor, const char *selected);
int rb_cost_has_skip_prompt(const AbilityEffect *cost);
int rb_compute_play_cost(const GameState *g, int actor, int card_id, int set_override);
int rb_get_change_state_candidates(const GameState *g, int actor,
                                   int *out_positions, int max);
int rb_pay_cost_move_cards(GameState *g, int actor, const AbilityEffect *cost,
                           int host_cid, int is_activation);
int rb_pay_cost_change_state(GameState *g, int actor, const AbilityEffect *cost,
                             int host_cid, int is_activation);
int rb_effect_place_energy_under_member_non_optional(GameState *g, int actor, const AbilityEffect *e);
void rb_move_execute_move_cards(GameState *g, int actor, AbilityEffect *e);

/* ── Compound / sequential / conditional execution (engine/src/ability/compound.rs) ── */
/* Mirror compound.rs::execute_sequential_effect. `eff` is the sequential effect; its
   children are the action list (the Rust `compound.actions`). A trailing
   repeat_procedure child is treated as the repeat marker (repeat_max = repeat_limit+1). */
int rb_compound_sequential(GameState *g, int actor, const AbilityEffect *eff, int host_cid);
/* Mirror compound.rs::route_conditional_branch — pick the branch index (0/1) for a
   conditional_alternative whose condition has been evaluated. */
int rb_compound_route_branch(const GameState *g, int actor, const AbilityEffect *eff);
/* Mirror compound.rs::execute_conditional_alternative. branch<0 → route via the
   effect's own condition; branch 0 = consequent (alternative_effect), 1 = alternate. */
int rb_compound_conditional_alternative(GameState *g, int actor,
                                          const AbilityEffect *eff, int branch, int host_cid);
/* Mirror compound.rs::execute_conditional_on_result. */
int rb_compound_conditional_on_result(GameState *g, int actor,
                                        const AbilityEffect *eff, int host_cid);
/* Mirror compound.rs::execute_conditional_on_optional. `taken`: -1 = no result yet
   (emit choice, headless auto-skips), 0 = not paid, 1 = paid. */
int rb_compound_conditional_on_optional(GameState *g, int actor,
                                          const AbilityEffect *eff, int taken, int host_cid);
int rb_compound_choice_string(const AbilityEffect *eff, const char *choice);
int rb_compound_choice_action(GameState *g, int actor, const AbilityEffect *eff,
                              int choice_idx, int host_cid);
void rb_compound_save_remaining(GameState *g, int remaining_count);
/* handle_choice_string_selection (compound.rs) — picks a string option;
   if it names a heart color the choice is recorded as a prohibition. */
int rb_compound_handle_choice_string_selection(GameState *g, int actor,
                                               const char *selected,
                                               const char **options, int n_options);
/* handle_choice_string_store (compound.rs) — stores the chosen string value
   in the queue entry (choice_result / resume_draw_ctype) for later use
   (mirrors ConditionalChoice::Str in the Rust port). */
int rb_compound_handle_choice_string_store(GameState *g, int actor,
                                           const char *selected,
                                           const char **options, int n_options);

/* ── Ability resolver frontend (engine/src/ability/resolver.rs) ── */
typedef struct AbilityInfo {
    int cid;            /* card id */
    int ability_idx;    /* index within card's abilities */
    const char *trigger;
} AbilityInfo;
const RbChoice *rb_resolver_get_pending_choice(const GameState *g);
int  rb_resolver_pending_choice(const GameState *g);
int  rb_resolver_current_ability_is_activation(const Ability *ab);
const char *rb_resolver_zone_for_card(const GameState *g, int card_id);
int  rb_resolver_use_limit_reached(const GameState *g, int card_id,
                                    int ability_index, int use_limit);
int  rb_can_activate_effect(const GameState *g, int actor,
                             const AbilityEffect *eff, int host_cid);
int  rb_resolver_trigger_infos(const GameState *g, int actor, const char *trigger,
                                AbilityInfo *out, int max);
int  rb_resolve_ability(GameState *g, int actor, const Ability *ab,
                        int ability_idx, int host_cid, int *resolved);
int  rb_resolver_card_matches_type(int cid, const char *filter);
int  rb_resolver_card_matches_cost_limit(int card_id, int cost_limit, const char *op);
void rb_resolver_store_pending_choice(GameState *g);
void rb_resolver_emit_pay_skip_gate(GameState *g, int actor, const AbilityEffect *e,
                                     const char *description, int optional, const char *route);
void rb_resolver_fmt_card(int cid, char *out, size_t out_sz);
const char *rb_resolver_fmt_ids(const int *ids, int n);

/* ── resolver.rs: condition-verdict cache + verdict-buffer drains ──
   The string-keyed entry points. resolver.rs caches a condition's boolean
   verdict per queue entry so a repeated `cache`-flagged condition is evaluated
   once (cached_condition_verdict / store_condition_verdict), and drained into
   the log buffer (push_verdict / drain_verdicts / drain_verdicts_since).
   Previously resolver.c declared the last three locally and the tests
   re-declared all five, so no single authoritative signature existed. */
/* Returns 1 on a cache hit and writes *result; returns 0 on a miss. */
int  rb_resolver_cached_condition_verdict(const GameState *g, int actor,
                                          const char *cond_text, int *result);
/* Store/replace the cached verdict for `cond_text` on the current queue entry. */
void rb_resolver_store_condition_verdict(GameState *g, int actor,
                                         const char *cond_text, int result);
/* Drain the whole verdict buffer, and the entries since a buffer-length snapshot. */
void rb_resolver_drain_verdicts(GameState *g);
void rb_resolver_drain_verdicts_since(GameState *g, int snapshot);
void rb_resolver_push_verdict(GameState *g, const char *text, const char *kind, int passed);

/* ── choice.rs public API (complete translation, engine_c/src/ability/choice.c) ── */
int  rb_resolver_resume_execution(RbAbilityResolver *self);
int  rb_resolver_resume_execution_with_ctx(RbAbilityResolver *self, void *ctx);
int  rb_resolver_resume_pending_actions(RbAbilityResolver *self);
int  rb_resolver_finalize_choice(RbAbilityResolver *self);
int  rb_resolver_finalize_choice_with_ctx(RbAbilityResolver *self, void *ctx);
void rb_resolver_reveal_selected_looked_at(GameState *g, const int *indices, int n_indices);
int  rb_resolver_provide_choice_result(GameState *g, int selected_idx);
int  rb_resolver_handle_select_card(RbAbilityResolver *self, GameState *g, const char *selected);
int  rb_resolver_handle_hand_selection(RbAbilityResolver *self, GameState *g, const char *selected);
void rb_resolver_handle_reveal_selection(RbAbilityResolver *self, GameState *g,
                                          const RbSelectionContext *ctx, const char *selected);
void rb_resolver_handle_revealed_cards_selection(RbAbilityResolver *self, GameState *g,
                                                  const RbSelectionContext *ctx, const char *selected);
void rb_resolver_handle_success_live_zone_selection(RbAbilityResolver *self, GameState *g,
                                                  const RbSelectionContext *ctx, const char *selected);
void rb_resolver_handle_entry_cost_reveal(RbAbilityResolver *self, GameState *g, const char *selected);
void rb_resolver_handle_looked_at_selection(RbAbilityResolver *self, GameState *g, const char *selected);
void rb_resolver_handle_stage_selection(RbAbilityResolver *self, GameState *g,
                                      const RbSelectionContext *ctx, const char *selected);
void rb_resolver_handle_discard_selection(RbAbilityResolver *self, GameState *g, const char *selected);
void rb_resolver_handle_heart_color_selection(RbAbilityResolver *self, GameState *g, const char *selected);
void rb_resolver_handle_choice_condition(RbAbilityResolver *self, GameState *g, const char *selected);
void rb_resolver_handle_conditional_optional(GameState *g, const char *selected);
void rb_resolver_handle_draw_any_number(GameState *g, const char *selected);
void rb_resolver_handle_order_selection(GameState *g, const char *selected);
void rb_resolver_handle_primary_alternative(RbAbilityResolver *self, GameState *g, const char *selected);
void rb_resolver_handle_position_destination(RbAbilityResolver *self, GameState *g, const char *selected);
void rb_resolver_handle_double_baton_touch(GameState *g, const char *selected);
int  rb_resolver_handle_position_change_choice(RbAbilityResolver *self, GameState *g,
                                               const char *choice_card_no, const char *selected);
void rb_resolver_handle_heart_selection(RbAbilityResolver *self, GameState *g, int count,
                                         const char *const *colors, int n_colors);
void rb_resolver_handle_select_target(RbAbilityResolver *self, GameState *g,
                                      const char *target, const char *selected);
void rb_resolver_execute_selected_cards_from_zone(RbAbilityResolver *self, GameState *g,
                                                   const char *zone, const int *indices, int n_indices,
                                                   const char *card_type, int cost_limit,
                                                   const char *cost_limit_op, int cost_total,
                                                   const char *cost_total_op, const char *group,
                                                   const char *const *characters, int n_characters,
                                                   const char *target_player_id);
void rb_resolver_handle_energy_zone_selection(GameState *g, int actor, const int *indices, int n_indices, const char *destination);
void rb_resolver_handle_select_cards_looked_at(RbAbilityResolver *self, GameState *g,
                                                const int *indices, int n_indices,
                                                const char *destination, int discard_remaining);
void rb_resolver_move_non_selected_hand_to_deck_bottom(GameState *g, const char *target_player,
                                                        const int *snapshot, int snapshot_n);
void rb_resolver_build_reprompt(RbAbilityResolver *self, GameState *g);
int  rb_resolver_build_reprompt_full(RbAbilityResolver *self, GameState *g, const RbSelectionContext *ctx,
                                      const char *zone, int count, const char *en, const char *ja,
                                      int allow_skip, const int *filtered, int n_filtered,
                                      const char *tpid, int cost_total, const char *cost_total_op);
int  rb_resolver_filter_discard_by_budget(RbAbilityResolver *self, GameState *g, int budget);
int  rb_resolver_filter_discard_by_budget_full(RbAbilityResolver *self, GameState *g,
                                                int cost_total, const char *cost_total_op,
                                                int *out_remaining, int *out_indices, int max_out);
void rb_resolver_handle_selection_epilogue(RbAbilityResolver *self, GameState *g);
void rb_resolver_apply_effect_modification(RbAbilityResolver *self, GameState *g,
                                            void (*modifier)(AbilityEffect *));
void rb_resolver_clear_choice_meta(RbAbilityResolver *self);
void rb_resolver_clear_choice_state(RbAbilityResolver *self);
int  rb_resolver_clear_choice_state_and_resume2(RbAbilityResolver *self);
void rb_resolver_store_pending_choice(GameState *g);
void rb_resolver_reschedule_pending_choice(GameState *g);
int  rb_queue_has_pending_actions(const GameState *g);
int  rb_queue_take_pending_actions(GameState *g);
void rb_queue_set_pending_actions(GameState *g, int n);
void rb_set_chosen_target(AbilityEffect *e, const char *target);
int  rb_resume_with_choice(GameState *g, int selected_idx);
void rb_emit_choice(GameState *g, int actor, RbChoiceKind kind,
                    const char *zone, const char *card_type,
                    int count, int allow_skip, const char *target);
void rb_choice_set_route(RbChoice *ch, RbChoiceRoute r);
void rb_choice_set_description(RbChoice *ch, const char *desc);
void rb_choice_set_bilingual_descriptions(RbChoice *ch, const char *en, const char *ja);
int  rb_gained_ability_index(int ability_idx);
const char *rb_choice_description_ja(const RbChoice *ch);
int  rb_choice_allow_skip(const RbChoice *ch);
const char *rb_ability_error_to_string(int err);

/* ── Auto-trigger engine + ability use tracking (core/game_state/abilities.rs) ── */
int  rb_ability_matches_trigger(const Ability *ab, const char *trigger);
int  rb_ability_master_id(const GameState *g);
void rb_record_ability_use(GameState *g, int cid, int idx);
int  rb_collect_constant_hand(const GameState *g, int actor, AbilityEffect *out, int max);
int  rb_collect_live_modifiers(const GameState *g, int actor, AbilityEffect *out, int max);
int  rb_trigger_auto_abilities(GameState *g, int actor, const char *trigger);
int  rb_trigger_auto_abilities_for_movement(GameState *g, int pl);
int  rb_trigger_auto_abilities_for_player_with_event(GameState *g, int pl, const int *moved_cards, int n_moved, int position_change, int energy_placed);
int  rb_trigger_auto_abilities_for_player(GameState *g, int pl);
int  rb_queue_yell_auto_abilities(GameState *g, int pl);
void rb_trigger_each_time_for_member(GameState *g, int pl, const char *trigger_substring, int member_card_id);
void rb_trigger_auto_abilities_for_movement_current(GameState *g);
/* Mirror live.rs::determine_winners — who placed a live this turn (score-tie → both). */
void rb_determine_live_winners(const GameState *g, int *p1_won, int *p2_won);
int  rb_process_player_abilities(GameState *g, int pl);
int  rb_drain_ability_queue(GameState *g);
void rb_check_expired_effects(GameState *g, int which);
int  rb_apply_ability_effects(GameState *g, int actor, const Ability *ab, int host_cid);
int  rb_opponent_id(int pl);
int  rb_distinct_stage_groups(const GameState *g, int pl);
uint64_t rb_opp_cause_key(uint32_t num_key, int moved_card_id, uint16_t seq);
int  rb_can_place_card_in_zone(const GameState *g, int cid, const char *zone);
void rb_clear_movement_tracking(GameState *g);
void rb_process_with_completed_key(GameState *g, int key);
int  rb_ability_uses_used(const GameState *g, int cid, int idx);
int  rb_ability_has_remaining_uses(const GameState *g, int cid, int idx);
int  rb_resolve_target_player(const GameState *g, const char *target);

/* ── Ability Queue Entry Accessors (GameState entry_* methods) ── */
/* The three pointer-returning accessors below are backed by a per-GameState
   keep-alive ring (GameState.entry_keepalive*): the returned tree is owned by
   the GameState and is released only when the ring wraps, so a returned pointer
   is never a dangling pointer into freed memory. It remains valid for
   RB_ENTRY_KEEPALIVE_SLOTS-1 further entry-accessor calls. Prefer the copy-out
   variants below, which hand the caller ownership and no lifetime coupling. */
const AbilityEffect *rb_entry_effect(const GameState *g);
const AbilityEffect *rb_entry_cost(const GameState *g);
const char *rb_entry_destination(const GameState *g);
/* Copy-out: `out` receives a freshly decoded Ability the CALLER owns; release it
   with rb_free_ability(out) (safe on a zeroed/failed result). Returns 1 when
   the current entry decoded. No shared state, no lifetime limit. */
int rb_entry_ability_copy(const GameState *g, Ability *out);
/* Copy-out for the destination string; writes "" and returns 0 when the current
   entry has no destination (or no current entry). */
int rb_entry_destination_copy(const GameState *g, char *buf, size_t buf_len);
int rb_entry_has_pending_choice(const GameState *g);
const RbChoice *rb_get_pending_choice(const GameState *g);
int rb_get_pending_choice_player_id(const GameState *g);
const int *rb_entry_trigger_moved_cards(const GameState *g, int *out_count);
int rb_entry_snapshot_last_energy_placed_by_effect(const GameState *g);
const char *rb_entry_snapshot_last_energy_placed_by_player(const GameState *g);
int rb_entry_snapshot_last_area_move_card_id(const GameState *g);
const char *rb_entry_snapshot_last_area_move_by_player(const GameState *g);

/* ── Constant ability lookup ── */
const AbilityEffect *rb_resolve_constant_ability(const GameState *g, int card_id, int ability_idx);

/* ── Cost reduction ── */
int rb_effective_activation_cost(const GameState *g, int actor, const AbilityEffect *cost);
int rb_effective_activation_cost_for(const GameState *g, int actor, const AbilityEffect *cost, int groups_on_stage);

/* ── Live success trigger ── */
int rb_should_trigger_live_success(const GameState *g, int pl);

/* ── Replacement effects (engine/src/core/game_state/abilities.rs) ── */

/* ── Loop detection ── */
void rb_reset_loop_detection(GameState *g);
int rb_is_loop_detected(const GameState *g);

void rb_exclusion_add(GameState *g, int card_id);
int rb_try_create_success_replacement_choice(GameState *g, int card_id, int player_pl);
int rb_move_maybe_prompt_success_replacement(GameState *g, int actor, int card_id,
                                              const char *dest, const char *target);
void rb_handle_success_replacement_choice(GameState *g, int player_pl,
                                          int original_card_id,
                                          int selected_discard_index,
                                          int accepted);

/* ── Replacement effects (stubs) ── */
void rb_add_replacement_effect(GameState *g, int card_id, int player_id, const char *original_event, const AbilityEffect *replacement_effects, int n_replacement, int is_choice_based);
void rb_reset_replacement_effect_flags(GameState *g);
void rb_mark_replacement_effect_applied(GameState *g, int card_id);

/* ── Turn/live state reset ── */
void rb_set_opponent_live_success(GameState *g, int no_excess_heart);
void rb_reset_change_flags(GameState *g);

/* ── Choice context injection (stub) ── */
void rb_inject_choice_ability_context(GameState *g, char *json_buf, size_t buf_sz);

/* ── AbilityRef (engine/src/ability/ability_store.rs) ──
    A lightweight handle to an ability stored in the bytecode blob.
    RbAbilityRef carries a uint16_t bytecode index. Call rb_ability_ref_resolve()
    to decode (with a per-slot lazy cache mirroring Rust's OnceLock<Arc<Ability>>).
    The returned pointer is static-storage; do NOT free it. There is no flush
    step: the cache lives for the process lifetime, mirroring Rust. */
typedef struct {
    uint16_t idx;
} RbAbilityRef;

RbAbilityRef rb_ability_ref_index(uint16_t idx);
uint16_t      rb_ability_ref_idx(const RbAbilityRef *ref);
const Ability *rb_ability_ref_resolve(const RbAbilityRef *ref); /* cached decode */
int           rb_ability_ref_decode(const RbAbilityRef *ref, Ability *out); /* write to out */
const Ability *rb_ability_ref_to_arc(const RbAbilityRef *ref); /* legacy alias for resolve */
/* rb_ability_ref_flush / rb_ability_ref_flush_slot were REMOVED 2026-10-02
 * (definitions deleted from src/ability/ability.c; Rust's AbilityStore has no
 * release step because its OnceLock<Arc<Ability>> entries live for the process
 * lifetime). Do NOT re-add their declarations. */
int           rb_ability_ref_cache_size(void);                  /* populated slots (diagnostic) */

/* ── Misc effect handlers (engine/src/ability/effects/misc.rs) ── */
int rb_execute_misc_effect(GameState *g, int actor, const RbPlayer *self,
                           const AbilityEffect *e, int *resolved);
/* Same dispatch, but carrying the resolving card id (Rust `activating_card`) so
   handlers that grant per-card resources / read "this member" attribute correctly. */
int rb_execute_misc_effect_ex(GameState *g, int actor, const RbPlayer *self,
                              const AbilityEffect *e, int host_cid, int *resolved);
/* Mirror misc.rs::handle_both_targets — run a target="both" effect for self then
   opponent. Returns 1 when the effect was fully handled ("both" target), else 0. */
int rb_misc_handle_both_targets(GameState *g, int actor, const AbilityEffect *e);
/* Formation-change plan entry — mirrors AbilityResolver.formation_plan
   ((member_id, destination) pairs collected while the player assigns areas).
   dest_area < 0 means "not assigned yet". */
typedef struct { int member_id; int dest_area; } RbFormationSlot;
/* Mirror misc.rs::compute_valid_position_destinations — writes the valid stage
   area indices (0 left / 1 center / 2 right) into out_areas; returns the count. */
int rb_misc_position_destinations(const GameState *g, int actor, const AbilityEffect *e,
                                  int host_cid, const RbFormationSlot *plan, int n_plan,
                                  int *out_areas, int max);
/* Mirror misc.rs::finalize_formation_change — apply every planned move as one
   atomic stage permutation. Returns the number of members that changed area. */
int rb_misc_finalize_formation_change(GameState *g, int actor,
                                      const RbFormationSlot *plan, int n_plan);
/* Mirror misc.rs::execute_position_change_with_destination — move the effect's
   member (source_position / target_member card_no / this_member) to `destination`
   ("same_area" = no-op, "front" = mirrored area per Rule 4.5.7). */
int rb_position_change_with_destination(GameState *g, int actor, const AbilityEffect *e,
                                         const char *destination, int host_cid);

/* ── types.rs: ArcStr serialize/deserialize ──
     Mirrors ArcStr serde impl (C strings are already owned heap values). */
char *rb_arcstr_serialize(const char *s);
void rb_arcstr_deserialize(char *s);

/* ── types.rs: Phase::label_jp ──
     Japanese phase labels for bilingual frontend rendering. */
const char *rb_phase_label_jp(int phase);

/* ── types.rs: EffectData accessors ──
     Mirrors EffectData enum methods on the full RbEffectData tagged union. */
int rb_effect_data_items(const RbEffectData *d, RbCardEffectItemRef **out_items, size_t *out_n);
int rb_effect_data_is_p1(const RbEffectData *d, bool *out_is_p1);
int rb_effect_data_old_value(const RbEffectData *d, uint8_t *out_old_value);
int rb_effect_data_count(const RbEffectData *d, uint8_t *out_count);
int rb_effect_data_color(const RbEffectData *d, const char **out_color);
int rb_effect_data_amount(const RbEffectData *d, int16_t *out_amount);
int rb_effect_data_card_id(const RbEffectData *d, int16_t *out_card_id);
int rb_effect_data_clone(const RbEffectData *source, RbEffectData *out);
void rb_effect_data_free(RbEffectData *d);
const char *rb_turn_phase_display(RbTurnPhase phase);
const char *rb_phase_display(RbPhase phase);

/* ── types.rs: ZoneId::equivalent / matches_source ──
     Zone aliasing for rule-purpose equivalence and zone-change condition matching. */
int rb_zone_equivalent(RbZoneId a, RbZoneId b);
int rb_zone_matches_source(RbZoneId zone, const char *source);

/* ── types.rs: ZoneId::as_str ──
     Converts a ZoneId enum to its wire string. Returns NULL for Unknown. */
const char *rb_zone_id_as_str(RbZoneId z);
/* ── types.rs: ZoneId::from_str ──
     Parse a wire string (incl. "ステージ" alias) into a ZoneId. */
RbZoneId rb_zone_id_from_str(const char *s);

/* ── types.rs: ZoneId::from_ability_zone / to_ability_zone ──
     Convert between ability::enums::Zone and core::types::ZoneId.
     to_ability_zone returns 0 on success, -1 if no mapping exists (Option::None). */
RbZoneId rb_zone_id_from_ability_zone(RbAbilityZone ability_zone);
int rb_zone_id_to_ability_zone(RbZoneId z, RbAbilityZone *out_ability_zone);

/* ── types.rs: EffectType::as_str ──
     Converts an RbEffectType enum to its wire string. */
const char *rb_effect_type_as_str(RbEffectType t);

/* ── card.rs: Card::get (score accessor) ──
     Mirrors Card::get_score — returns the printed score for a card. */
/* ── game_modifiers.rs: ModifierEntry::total ──
      Mirrors ModifierEntry::total — returns set + additive combined. */
int rb_modifier_total_entry(const RbModifierEntry *e);

/* ── types.rs: Constants ─────────────────────────────────────────────── */
extern const char *RB_PAY_SKIP_TARGET;

/* engine/src/ability/types.rs:379-392 —
 *     /// Ability-index offset addressing RUNTIME-GAINED abilities instead of
 *     /// printed card abilities. Encoded as `GAINED_ABILITY_INDEX_BASE +
 *     /// gained_idx` in every (card_id, ability_index) pair that outlives the
 *     /// resolution that produced it — constant scans, trigger scans, and queue
 *     /// entries all share this single encoding.
 *     pub const GAINED_ABILITY_INDEX_BASE: usize = 10_000;
 *
 * THIS IS THE ONE BASE. A gained ability is addressed as
 * RB_GAINED_ABILITY_INDEX_BASE + gained_idx and nowhere else; every producer
 * (queue_gained_trigger, rb_collect_constant_ids_for, the gain_ability_from_source
 * bridge in rb_trigger_auto_ability) and every consumer (rb_resolve_constant_ability,
 * the live-success enqueue branch in rb_trigger_live_success_faithful,
 * rb_gained_ability_index) tests against this one macro. There must never be a
 * second, translation-unit-private copy of the value: the port once had
 * `GAINED_ABILITY_INDEX_BASE 0x8000` private to game_state_abilities.c beside
 * this 10000, and the two index spaces did not intersect, so a pair minted on
 * one side was unreadable on the other.
 *
 * The derived index must also stay < the base (RB_GAINED_PER_SLOT is 4, far
 * below it), which is what makes the `>= base` gained test unambiguous. */
#define RB_GAINED_ABILITY_INDEX_BASE 10000

/* ── types.rs: ChoiceRoute helpers ───────────────────────────────────── */
const char *rb_choice_route_to_str(RbChoiceRouteKind r);
int rb_choice_route_from_str(const char *s, RbChoiceRouteKind *out);
RbChoiceRouteKind rb_choice_route_new(const char *s);
RbChoiceRoute rb_choice_route_from_kind(RbChoiceRouteKind k);
RbChoiceRouteKind rb_choice_route_kind_from_header(RbChoiceRoute r);

/* ── types.rs: RbFullChoice constructors / accessors ─────────────────── */
RbFullChoice *rb_full_choice_new_select_card(const char *zone, const char *description,
                                              int count, int allow_skip);
RbFullChoice *rb_full_choice_new_select_target(const char *target, const char *description,
                                                int allow_skip);
RbFullChoice *rb_full_choice_new_select_position(const char *position, const char *description,
                                                  int allow_skip);
RbFullChoice *rb_full_choice_new_select_heart_color(int count, const char *const *options,
                                                     int n_options, const char *description);
RbFullChoice *rb_repeat_prompt_choice(void);
const char *rb_full_choice_description_ja(const RbFullChoice *ch);
int rb_full_choice_allow_skip(const RbFullChoice *ch);
void rb_full_choice_set_description(RbFullChoice *ch, const char *desc);
void rb_full_choice_set_bilingual(RbFullChoice *ch, const char *en, const char *ja);
void rb_full_choice_set_options(RbFullChoice *ch, const char *const *opts, int n);
void rb_full_choice_set_hc_options(RbFullChoice *ch, const char *const *opts, int n);
void rb_full_choice_set_aa_options(RbFullChoice *ch, const char *const *opts, int n);
void rb_full_choice_set_ls_options(RbFullChoice *ch, const char *const *opts, int n);
void rb_full_choice_to_header(const RbFullChoice *src, RbChoice *dst);
const char *rb_full_choice_to_json(const RbFullChoice *ch, char *buf, size_t buf_sz);
void rb_full_choice_free(RbFullChoice *ch);

/* ── types.rs: ChoiceBuilder ─────────────────────────────────────────── */
RbChoiceBuilder *rb_choice_builder_new(const char *zone, const char *description,
                                        int count, int allow_skip);
RbFullChoice *rb_choice_builder_build(RbChoiceBuilder *b);
void rb_choice_builder_free(RbChoiceBuilder *b);
RbChoiceBuilder *rb_choice_builder_card_type(RbChoiceBuilder *b, const char *v);
RbChoiceBuilder *rb_choice_builder_cost_limit(RbChoiceBuilder *b, int v, const char *op);
RbChoiceBuilder *rb_choice_builder_cost_total(RbChoiceBuilder *b, int v, const char *op);
RbChoiceBuilder *rb_choice_builder_cost_values(RbChoiceBuilder *b, const uint8_t *vals, int n);
RbChoiceBuilder *rb_choice_builder_group(RbChoiceBuilder *b, const char *v);
RbChoiceBuilder *rb_choice_builder_characters(RbChoiceBuilder *b, const char *const *names, int n);
RbChoiceBuilder *rb_choice_builder_heart_colors(RbChoiceBuilder *b, const char *const *colors, int n);
RbChoiceBuilder *rb_choice_builder_require_all_heart_colors(RbChoiceBuilder *b, int v);
RbChoiceBuilder *rb_choice_builder_name_fragments(RbChoiceBuilder *b, const char *const *frags, int n);
RbChoiceBuilder *rb_choice_builder_destination(RbChoiceBuilder *b, const char *v);
RbChoiceBuilder *rb_choice_builder_discard_remaining(RbChoiceBuilder *b, int v);
RbChoiceBuilder *rb_choice_builder_is_select_action(RbChoiceBuilder *b, int v);
RbChoiceBuilder *rb_choice_builder_target_player_id(RbChoiceBuilder *b, const char *v);
RbChoiceBuilder *rb_choice_builder_blind(RbChoiceBuilder *b, int v);
RbChoiceBuilder *rb_choice_builder_is_reveal(RbChoiceBuilder *b, int v);
RbChoiceBuilder *rb_choice_builder_picker(RbChoiceBuilder *b, const char *v);
RbChoiceBuilder *rb_choice_builder_description_en(RbChoiceBuilder *b, const char *v);
RbChoiceBuilder *rb_choice_builder_description_ja(RbChoiceBuilder *b, const char *v);
RbChoiceBuilder *rb_choice_builder_filtered_indices(RbChoiceBuilder *b, const int *indices, int n);

/* ── types.rs: ChoiceResult helpers ──────────────────────────────────── */
RbChoiceResultKind rb_choice_result_kind(const RbChoiceResult *r);
int rb_choice_result_is_skip(const RbChoiceResult *r);
const char *rb_choice_result_kind_to_str(RbChoiceResultKind k);
int rb_choice_result_kind_from_str(const char *s, RbChoiceResultKind *out);
RbChoiceResult *rb_choice_result_new_card_selected(const int *indices, int n);
RbChoiceResult *rb_choice_result_new_skip(void);
RbChoiceResult *rb_choice_result_new_target(const char *target);
void rb_choice_result_free(RbChoiceResult *r);

/* ── types.rs: TriggerEvent ──────────────────────────────────────────── */
void rb_trigger_event_init(RbTriggerEvent *e);
void rb_trigger_event_add_moved(RbTriggerEvent *e, int card_id);
void rb_trigger_event_add_appeared(RbTriggerEvent *e, int card_id, const char *source_zone);
int rb_trigger_event_has_moved(const RbTriggerEvent *e);
int rb_trigger_event_has_appeared(const RbTriggerEvent *e);
int rb_trigger_event_has_position_change(const RbTriggerEvent *e);
int rb_trigger_event_has_energy_placed(const RbTriggerEvent *e);
void rb_trigger_event_copy(RbTriggerEvent *dst, const RbTriggerEvent *src);

/* ── types.rs: EffectSpawnContext ────────────────────────────────────── */
void rb_effect_spawn_context_init(RbEffectSpawnContext *ctx);

/* ── types.rs: StepOutput ────────────────────────────────────────────── */
void rb_step_output_init(RbStepOutput *out);
RbStepOutput *rb_step_output_from_value(int value);
void rb_step_output_merge(RbStepOutput *self, const RbStepOutput *other);
void rb_step_output_add_card(RbStepOutput *out, int card_id);
int rb_step_output_has_cards(const RbStepOutput *out);
int rb_step_output_value(const RbStepOutput *out);
int rb_step_output_accepted(const RbStepOutput *out);
const char *rb_step_output_to_json(const RbStepOutput *out, char *buf, size_t buf_sz);

/* ── types.rs: ValueRef ──────────────────────────────────────────────── */
void rb_value_ref_init_literal(RbValueRef *ref, int value);
void rb_value_ref_init_step(RbValueRef *ref, const char *step_id);
void rb_value_ref_init_accepted(RbValueRef *ref, const char *step_id);
void rb_value_ref_init_offset(RbValueRef *ref, const char *step_id, int offset);
const char *rb_value_ref_kind_str(const RbValueRef *ref);
const char *rb_value_ref_kind_to_str(RbValueRefKind k);
int rb_value_ref_kind_from_str(const char *s, RbValueRefKind *out);
int rb_value_ref_resolve(const RbValueRef *ref, int (*lookup)(const char *, int *, int *, void *),
                          void *lookup_ctx, int fallback);
void rb_value_ref_merge(RbValueRef *self, const RbValueRef *other);
int rb_value_ref_is_literal(const RbValueRef *ref);

/* ── types.rs: ZoneSnapshot ──────────────────────────────────────────── */
RbZoneSnapshot rb_zone_snapshot_make(int hand, int stage, int waitroom,
                                     int energy, int active_energy, int deck);
RbZoneSnapshot rb_zone_snapshot_from_game_state(const GameState *g);

/* ── types.rs: AbilityTraceNode ──────────────────────────────────────── */
RbAbilityTraceNode *rb_trace_node_new(const char *label);
RbAbilityTraceNode *rb_trace_node_with_card(RbAbilityTraceNode *node, const char *card);
RbAbilityTraceNode *rb_trace_node_with_before(RbAbilityTraceNode *node, RbZoneSnapshot before);
RbAbilityTraceNode *rb_trace_node_with_after(RbAbilityTraceNode *node, RbZoneSnapshot after);
RbAbilityTraceNode *rb_trace_node_from_game_state(const char *label, const GameState *g);
int rb_trace_node_add_child(RbAbilityTraceNode *parent, RbAbilityTraceNode *child);
void rb_trace_node_free(RbAbilityTraceNode *node);

/* ── types.rs: EffectPipeline ────────────────────────────────────────── */
RbEffectPipeline *rb_effect_pipeline_new(void);
void rb_effect_pipeline_free(RbEffectPipeline *p);

/* ── types.rs: StepState ─────────────────────────────────────────────── */
void rb_step_state_init(RbStepState *ss);
RbStepState *rb_step_state_new(void);
void rb_step_state_record(RbStepState *ss, const char *effect_id, const RbStepOutput *output);
RbStepOutput rb_step_state_get(const RbStepState *ss, const char *step_id);
void rb_step_state_clear(RbStepState *ss);
void rb_step_state_free(RbStepState *ss);
int rb_step_state_record_value(RbStepState *ss, const char *effect_id, int value);
int rb_step_state_record_cards(RbStepState *ss, const char *effect_id, const int *card_ids, int n);

/* ── types.rs: AbilityError ──────────────────────────────────────────── */
void rb_ability_error_format(int err, char *out, size_t out_sz,
                             int p1, int p2, int p3, const char *detail);
int rb_gained_ability_index(int ability_idx);

/* ── types.rs: ExecutionContext / LookAndSelectStep helpers ──────────── */
const char *rb_exec_context_kind_to_str(RbExecutionContextKind k);
int rb_exec_context_kind_from_str(const char *s, RbExecutionContextKind *out);
const char *rb_las_kind_to_str(RbLookAndSelectStepKind k);
int rb_las_kind_from_str(const char *s, RbLookAndSelectStepKind *out);
RbLookAndSelectStep rb_look_and_select_step_look(int count, const char *source);
RbLookAndSelectStep rb_look_and_select_step_select(int count, int max_per_group);
RbLookAndSelectStep rb_look_and_select_step_finalize(const char *destination, const char *source_zone);

int rb_executor_has_executor(const char *action);
int rb_executor_is_structural(const char *action);
int rb_executor_execute(GameState *g, int actor, AbilityEffect *effect, int host_cid);
/* rb_execute_effect_via_registry: declaration DELETED 2026-10-04 (body gone
   since 2026-10-02). See the block note above rb_suppress_ability_trigger. */
int rb_translated_execute_gain_ability_effect(GameState *g, int actor, AbilityEffect *effect, int host_cid);
int rb_translated_execute_activate_ability(GameState *g, int actor, AbilityEffect *effect, int host_cid);
int rb_translated_execute_invalidate_ability(GameState *g, int actor, AbilityEffect *effect);
int rb_translated_execute_suppress_ability_trigger(GameState *g, int actor, AbilityEffect *effect);
int rb_translated_execute_set_card_identity_effect(GameState *g, int actor, AbilityEffect *effect, int host_cid);
int rb_translated_execute_gain_ability_from_source(GameState *g, int actor, AbilityEffect *effect, int host_cid);
int rb_execute_custom_effect(GameState *g, int actor, AbilityEffect *effect, const char *action_str);

/* ── Ability description renderer (engine/src/ability/describe.rs) ─────────
 * Ported into src/ability/describe.c. Verified against the real Rust golden
 * test_output/describe_dump.json (936 rows) at 931/936 byte-identical before
 * the vm.c decoder gaps were fixed.
 *
 * OWNERSHIP: every char* returned by rb_desc_effect / rb_desc_cost and their
 * _en/_ja variants is rb_malloc'd — free it with rb_desc_free. The label
 * helpers (rb_desc_zone_label, rb_desc_card_type_label, rb_desc_state_verb,
 * rb_desc_resource_label, rb_desc_duration_label, rb_desc_choice_prompt_
 * templates_en) return BORROWED pointers — a literal or the caller's own
 * string. Never free those. */
void  rb_desc_free(char *s);
char *rb_desc_effect(const AbilityEffect *effect, int ja);
char *rb_desc_effect_en(const AbilityEffect *effect);
char *rb_desc_effect_ja(const AbilityEffect *effect);
char *rb_desc_cost(const AbilityEffect *cost, int ja);
char *rb_desc_cost_en(const AbilityEffect *cost);
char *rb_desc_cost_ja(const AbilityEffect *cost);
char *rb_desc_sequential_cost(const AbilityEffect *const *costs, int n_costs, int choice_index, int ja);
char *rb_desc_sequential_cost_en(const AbilityEffect *const *costs, int n_costs, int choice_index);
char *rb_desc_sequential_cost_ja(const AbilityEffect *const *costs, int n_costs, int choice_index);
const char *rb_desc_zone_label(const char *zone);
const char *rb_desc_zone_label_ja(const char *zone);
const char *rb_desc_card_type_label(const char *ct);
const char *rb_desc_card_type_label_ja(const char *ct);
const char *rb_desc_state_verb(const char *state, int ja);
const char *rb_desc_state_verb_ja(const char *state);
const char *rb_desc_resource_label(const char *resource, int ja);
const char *rb_desc_resource_label_ja(const char *resource);
const char *rb_desc_duration_label(const char *duration, int ja);
const char *rb_desc_duration_label_ja(const char *duration);
void  rb_desc_apply_bilingual(RbFullChoice *ch, const AbilityEffect *effect, int ja_first);
char *rb_desc_translate_choice_prompt_en_to_ja(const char *en); /* NULL == Rust's None */
extern const char *const rb_desc_choice_prompt_templates_en[];
extern const int rb_desc_choice_prompt_templates_en_count;

/* ── Selection-grid renderer (engine/src/choice_renderer.rs) ────────────────
 * NOTE: choice_renderer.rs is the 3DS graphical card-choice-grid front-end
 * built on the PlatformUi trait. It never reads RbChoice; its option list
 * arrives as caller-supplied slices. Ported into
 * src/ability/choice_renderer.c. The UI backend is a NULL-able callback
 * table — an all-NULL table degrades to Rust's text-only default. */
typedef struct RbRendUi {
    void (*clear_screen)(void);
    void (*println)(const char *s);
    void (*swap_buffers)(void);
    void (*poll_input)(void);
    void (*wait_vblank)(void);
    int  (*just_pressed_a)(void);
    int  (*just_pressed_b)(void);
    int  (*just_pressed_l)(void);
    int  (*just_pressed_r)(void);
    int  (*just_pressed_up)(void);
    int  (*just_pressed_down)(void);
    int  (*just_pressed_left)(void);
    int  (*just_pressed_right)(void);
    void (*draw_card_image)(const char *card_no, int x, int y);
    void (*show_board_overlay)(const GameState *gs);
    void (*show_detail_screen)(const char *title, const char *body);
    void (*reset_vram)(void);
    void (*open_start_menu)(void);
} RbRendUi;
int rb_rend_render_card_choice_grid(const RbRendUi *ui, const GameState *gs,
                                    const char *title,
                                    const char *const *items, int n_items,
                                    const char *const *card_nos, int n_card_nos,
                                    int allow_skip,
                                    const unsigned char *dimmed, int n_dimmed);

/* ── Cross-file symbols that exist and are exported but were never declared ──
 * Hoisted here from private externs that several files had been duplicating. */
const RbQueueEntry *rb_queue_get_entry(const GameState *g, int idx);
int rb_heartmap_len(const Card *c, int from, int len);
int rb_heartmap_key_at(const Card *c, int from, int len, int i);
int rb_heartmap_value_at(const Card *c, int from, int len, int i);

/* ── Core GameState impl (engine/src/core/game_state/mod.rs) ────────────────
 * Ported into src/core/game_state_core.c as 40 rb_gsc_* functions. */
typedef struct { const char *key; int count; } RbGscZoneSnapshot;
#define RB_GSC_ZONE_SNAPSHOT_KEYS 16
int rb_gsc_zone_snapshot(const GameState *g, RbGscZoneSnapshot *out);

/* ── Pending-action ownership helpers ────────────────────────────────────────
 * Defined in src/ability/ability_queue.c and used from other translation
 * units, but were never declared here, so callers were re-declaring them
 * privately. Ownership contract: _owned detaches the batch AND frees it (the
 * caller now owns the returned effects); _repark puts a batch back.
 *
 * CAVEAT worth reading before calling resume: the C entry path ALREADY
 * executes pending_actions, so routing a batch through these from a resume
 * point double-counts. Two faithful implementations were built and both were
 * measured to REGRESS (cce_gaps +8, sequential +7), so this is recorded rather
 * than shipped. See the comment at rb_resolver_resume_pending_actions in
 * src/ability/choice.c. */
int rb_queue_take_pending_actions_owned(GameState *g, AbilityEffect **out, int max_out);
void rb_queue_repark_pending_actions(GameState *g, AbilityEffect *const *actions,
                                     int n, int resume_index);

/* ── Deferred conditional gate (compound.c -> choice.c) ──────────────────────
 * Rust carries this as part of the resolver's in-flight state. C has
 * `g_deferred_conditional_gate` in compound.c, but it is WRITE-ONLY: nothing
 * reads it, so the flag never reaches choice.c's `deferred_conditional_gate`
 * resolver field, and the 「〜したら」 branch never fires. This queue-level
 * carrier is the missing link - one writer (compound.c), one reader
 * (choice.c). Purely additive; no existing field moves. */
/* ── Group-name accessors (util.rs:748-793, card.rs:2462) ───────────────────
 * Rust reads a SINGLE group name from an effect: `group_name()` is
 * `group_names.first()` (card.rs:2462), NOT `any_group` (util.rs:799, a
 * different function). The C had been comparing the whole comma-joined CSV as
 * one token, which matched 0 of 2,639 cards - e.g. "lilywhite,μ's" and
 * "CatChu!,KALEIDOSCORE,5yncri5e!" both matched nothing.
 *
 * These were `static` in util.c while four files carried LOCAL TWINS of the
 * same logic - the same forking hazard that produced three copies of
 * custom.c. Exported so the logic has one home. The `buf`/`bufsz` pair is
 * caller-owned scratch so the scan loops stay allocation-free.
 *
 * NOTE: an equivalence audit over all 2,639 cards x 45 group strings
 * (118,755 pairs) confirmed rb_card_matches_group_str agrees with Rust on
 * every pair, in both directions. It is NOT too permissive. The CSV bug above
 * was a string-equality artefact: a reject-everything filter, which cannot
 * imply permissiveness. */
const char *rb_group_name_first(const char *group_names, char *buf, size_t bufsz);
const char *rb_effect_group_name(const AbilityEffect *e, char *buf, size_t bufsz);
int rb_card_matches_effect_group(const AbilityEffect *e, int card_id,
                                 char *buf, size_t bufsz);
/* Defined once, non-static, in src/ability/util.c. It previously had only
 * file-scope forward declarations in src/ability/util.c and
 * src/ability/effects/move.c; declaring it here lets the four hand-rolled local
 * twins in cost.c / score.c / misc.c / ability_effects.c be deleted. */
int rb_get_selection_indices_filter(const int *cards, int n, const RbCardFilter *filter,
                                    int self_target_only, int activating_card,
                                    int *out_idx, int max);

extern int g_deferred_conditional_gate;

/* ── Raw choice route (Rust `ChoiceRoute::Raw`) ─────────────────────────────
 * RbQueueEntry.choice_route_raw / has_choice_route_raw are the C home of Rust's
 * `entry.choice_card_no = Some(ChoiceRoute::Raw(s))` (queue.rs). Needed because
 * queue.rs:340-345 matches `is_route_opponent` on the PREFIX
 * "position_change:opponent:", which an int tag cannot represent.
 * No-ops when there is no current entry, mirroring Rust's
 * `current_entry_mut()` returning None. Defined in src/ability/ability_queue.c. */
void rb_queue_set_choice_route_raw(GameState *g, const char *raw);

/* ── GameState.wait_immune_members (Rust core/game_state/mod.rs) ─────────────
 * Rust writes this (member, owner) pair list from effects/restriction.rs:191
 * and reads it in change_state.rs:458-467 and :585-599 to suppress a wait that
 * would otherwise apply. C HAS NO SUCH FIELD AT ALL - there is no `immune`
 * symbol anywhere in the tree - so a wait-immunity restriction is silently
 * ignored. Added so the rule can be implemented; it has no writer yet. */
#define RB_WAIT_IMMUNE_MAX 16
typedef struct { int member_cid; int owner_player; } RbWaitImmune;
extern RbWaitImmune g_wait_immune_members[RB_WAIT_IMMUNE_MAX];
extern int g_n_wait_immune_members;

/* Max double-baton pairs a single play candidate can carry. Mirrors the
 * pairs table game_setup.rs:1569 sizes; declared here because the candidate
 * struct below is now visible tree-wide. */
#ifndef RB_SETUP_MAX_DOUBLE_BATON
#define RB_SETUP_MAX_DOUBLE_BATON 8
#endif

/* ── Setup actions and candidate rows (engine/src/game/game_setup.rs) ─────────
 * These lived INSIDE src/game/game_setup.c, which made them unusable from any
 * other translation unit — no other file could construct an RbSetupAction to
 * pass to rb_setup_execute_action, so that function and seven siblings were
 * uncallable BY CONSTRUCTION. src/turn/phase.c had to mirror the candidate
 * structs field-for-field, which is a divergence hazard. They live here now. */
typedef enum {
    RB_SETUP_ACT_PASS = 0, RB_SETUP_ACT_ROCK_CHOICE, RB_SETUP_ACT_PAPER_CHOICE,
    RB_SETUP_ACT_SCISSORS_CHOICE, RB_SETUP_ACT_CHOOSE_FIRST_ATTACKER,
    RB_SETUP_ACT_CHOOSE_SECOND_ATTACKER, RB_SETUP_ACT_MULLIGAN_HEADER,
    RB_SETUP_ACT_SELECT_MULLIGAN, RB_SETUP_ACT_CONFIRM_MULLIGAN,
    RB_SETUP_ACT_SKIP_MULLIGAN, RB_SETUP_ACT_LIVE_CARD_HEADER,
    RB_SETUP_ACT_SELECT_LIVE_CARD, RB_SETUP_ACT_CONFIRM_LIVE_CARD_SET,
    RB_SETUP_ACT_SKIP_LIVE_CARD_SET, RB_SETUP_ACT_PLAY_MEMBER_TO_STAGE,
    RB_SETUP_ACT_USE_ABILITY, RB_SETUP_ACT_SET_LIVE_CARD,
    RB_SETUP_ACT_FINISH_LIVE_CARD_SET, RB_SETUP_ACT_DRAW_CARD,
    RB_SETUP_ACT_CHOICE_DECISION, RB_SETUP_ACT_CHOICE_SELECT,
    RB_SETUP_ACT_CHOICE_SKIP, RB_SETUP_ACT_CHOICE_OPTION,
    RB_SETUP_ACT_CHOICE_POSITION, RB_SETUP_ACT_ENERGY_CHARGE,
    RB_SETUP_ACT_PASS_REMAINING, RB_SETUP_ACT_CONCEDE, RB_SETUP_ACT_COUNT
} RbSetupActionType;

/* Headless action: only the fields the ENGINE consumes. The display fields
 * Rust carries (description, card_name, card_no, source_ability, base_cost,
 * final_cost, double_baton_pairs, disabled, selected) are menu data and are not
 * reproduced; the decision-relevant costs are reported by the *_candidates()
 * entry points instead. */
typedef struct {
    RbSetupActionType type;
    int has_card_id;
    int card_id;
    int card_indices[RB_MAX_HAND];
    int n_card_indices;
    const char *stage_area;      /* NULL = unset */
    int has_stage_area_index;
    int stage_area_index;
    int use_baton_touch;
    int   ability_index;
    int   has_ability_index;
    int   cost_total;
    int   has_cost_total;
} RbSetupAction;

/* game_setup.rs:1617-1626 AreaCandidate, minus the display-only existing_member_name. */
typedef struct {
    int slot;                  /* 0=left 1=center 2=right */
    int available;
    int cost;
    int is_baton_touch;
    int existing_member_id;
} RbSetupAreaCandidate;

/* game_setup.rs:320-324 DoubleBatonOption */
typedef struct {
    int area_index[2];
    int placement_area;
    int cost;
} RbSetupDoubleBatonOption;

/* Decision table for one hand member — the rules content of game_setup.rs:1690
 * generate_main_phase_actions. */
typedef struct {
    int card_id;
    int hand_index;
    int card_cost;             /* card.cost.unwrap_or(0) */
    int reduction;
    int effective_cost;        /* card_cost.saturating_sub(reduction) */
    RbSetupAreaCandidate areas[RB_STAGE_SIZE];
    int any_area_available;
    RbSetupDoubleBatonOption pairs[RB_SETUP_MAX_DOUBLE_BATON];
    int n_pairs;
} RbSetupPlayCandidate;

/* One offerable activation — the rules content of game_setup.rs:2078-2348. */
typedef struct {
    int card_id;
    int ability_index;         /* printed, or RB_GAINED_ABILITY_INDEX_BASE+n */
    int from_stage;            /* 1 = stage scan, 0 = discard scan */
    int stage_slot;            /* 0..2 when from_stage, else RB_EMPTY_SLOT */
    int base_cost;             /* AbilityEffect::energy_cost_total */
    int effective_cost;        /* effective_activation_cost_for(.., groups) */
    int groups_on_stage;
    int has_optional_payment;
} RbSetupAbilityCandidate;

/* Mulligan / live-card selection rows (game_setup.rs:1467 and :2353). */
typedef struct {
    int hand_index;
    int card_id;
    int selected;
    int selectable;            /* live-card rows only: false at the limit */
} RbSetupHandCandidate;

/* ── Overflow visibility (log.c / debug.c) ──────────────────────────────────
 * The 32-slot verdict ring and the 256-line trace buffer are DROP-NEWEST with
 * no eviction of the oldest — Rust's buffers are unbounded (log.rs:69 Vec,
 * debug.rs:66 Mutex<Vec<String>>), so no fixed C bound is faithful and neither
 * drop policy is. The honest position is to stop hiding the loss, which is what
 * these counters do. Verified: 200 pushes -> len=32 dropped=168, and the
 * surviving sequence range is [0..31], i.e. the OLDEST is kept.
 * Both [ABILITY_LOG_OVERFLOW] and [AB_DEBUG_OVERFLOW] fire UNGATED on stderr
 * on the first drop of an episode and every 64th after.
 * The four prototypes were missing even though the counters themselves existed;
 * nothing in src/ or tests/ could call them. */
int  rb_log_dropped_count(void);
void rb_log_dropped_reset(void);
int  rb_abdebug_dropped_count(void);
void rb_abdebug_dropped_reset(void);


/* ── engine/src/core/constants.rs:8 numeric helpers ──────────────────────────
 * These are the CANONICAL arithmetic, so no call site should re-spell them.
 * A peer verified all 8 against an inline Rust reference over a 20-point grid
 * plus edge grids (saturation, round-half-away-from-zero on +/-0.5, NaN->0).
 * They had NO prototype here, which is why all 8 had ZERO callers: game_state_core.c
 * worked around the missing `rb_phase_is_live_phase` with a local forward
 * declaration, TWICE (lines 248 and 276). An unprototyped canonical helper is
 * exactly how the ActionType and ZoneId tables drifted, so declare them. */
int rb_score_to_i32(double v);
int rb_score_to_i32_rounded(double v);
uint8_t rb_effective_stat(uint8_t printed, int16_t modifier);
uint8_t rb_overridden_stat(int16_t set, int16_t modifier);
uint8_t rb_floored_cost(uint8_t base, int16_t modifier);
uint32_t rb_count_u32(size_t v);
uint32_t rb_id_u32(int16_t v);
float rb_f32_count(double v);
int rb_phase_is_live_phase(int phase);
int     rb_zone_matches_source(RbZoneId zone, const char *source);
int     rb_zone_matches_dest(RbZoneId zone, const char *dest);
const char *rb_step_output_to_json(const RbStepOutput *out, char *buf, size_t buf_sz);

/* Card::has_draw_icon (card.rs:4358), consumed by check_card_property
 * (util.rs:1654-1673). Its three siblings are declared nearby; this one was
 * missing, so callers had to invent a local extern. */
int rb_card_has_draw_icon(const Card *c);

/* Rust GameState.rule_log: Vec<String> (core/game_state/mod.rs:605) - a buffer
 * SEPARATE from the 32-slot condition-verdict ring, which drops on overflow.
 * Bound LOG_BOUND_RULE, newest-kept (mod.rs:1602-1606); desktop default 500.
 * Nine functions in core/game_state_core.c are unroutable until this exists -
 * routing them into the verdict ring would evict condition verdicts.
 * The capacity macros live BEFORE GameState, next to RbPositionChangeEvent. */

/* ── game_state_core.c public surface (core/game_state/mod.rs) ────────────────
 * 45 declarations that were implemented and COMPILING in game_state_core.c with
 * no prototype anywhere. Only ONE of the 53 rb_gsc_* symbols was declared in this
 * header, and 6 more had callers only because a caller hand-wrote a local extern
 * (game/game_setup.c:208) - so the file compiled by accident rather than by
 * contract. Signatures are verbatim from game_state_core.c. Audit: AUDIT_core_state.md §7. */

/* RbGscTurnPhase (was game_state_core.c:93-97), MOVED HERE 2026-10-03 so the
 * rb_gsc_current_turn_phase prototype below has a visible type; the local typedef
 * was deleted in the same change, the same pattern as RbGscZoneSnapshot.
 * Deliberately NOT stored as a GameState field: it is fully derivable from
 * g->phase + g->first_attacker + g->second_attacker, and storing it would create a
 * second source of truth that can disagree with the phase machine.
 * RB_PHASE_RPS covers the RPS->first-attacker step, with g->rps_winner holding the
 * outcome. The mapping is total: every C phase has a Rust counterpart. */
typedef enum {
    RB_GSC_TP_FIRST_ATTACKER_NORMAL = 0,
    RB_GSC_TP_SECOND_ATTACKER_NORMAL,
    RB_GSC_TP_LIVE
} RbGscTurnPhase;

/* ---- seat identity (game_state_core.c:144-180) ---- */
int         rb_gsc_first_attacker_index(const GameState *g);
int         rb_gsc_second_attacker_index(const GameState *g);
int         rb_gsc_active_player_index(const GameState *g);
RbPlayer   *rb_gsc_active_player(GameState *g);
const RbPlayer *rb_gsc_const_active_player(const GameState *g);
const RbPlayer *rb_gsc_seat_player(const GameState *g, int seat);
RbPlayer   *rb_gsc_seat_player_mut(GameState *g, int seat);
const RbPlayer *rb_gsc_seated_pair(const GameState *g, int me, const RbPlayer **out_opp);
const RbPlayer *rb_gsc_try_player_by_id(const GameState *g, const char *id);
RbPlayer   *rb_gsc_try_player_by_id_mut(GameState *g, const char *id);
int         rb_gsc_seat_index_by_id(const char *player_id);

/* ---- turn / phase (game_state_core.c:185-238) ---- */
int         rb_gsc_is_first_turn(const GameState *g);
RbGscTurnPhase rb_gsc_current_turn_phase(const GameState *g);
int         rb_gsc_phase_invariant(const GameState *g);
int         rb_gsc_can_player_act(const GameState *g, int player_id);
int         rb_gsc_opponent_has_performed(const GameState *g, int my_player_idx);
int         rb_gsc_game_has_ended(const GameState *g);
int         rb_gsc_reject_if_ended(const GameState *g);

/* ---- concede ---- */
int  rb_gsc_concede(GameState *g, int player_id);
int  rb_gsc_concede_both(GameState *g);

/* ---- scoring / cheer by id (game_state_core.c:413-438) ---- */
int  rb_gsc_cheer_count_by_id(const GameState *g, const char *player_id);
void rb_gsc_set_cheer_count_by_id(GameState *g, const char *player_id, int count);
int  rb_gsc_seat_cheer_count(const GameState *g, int seat);
int  rb_gsc_score_bonus_by_id(const GameState *g, const char *player_id);
int  rb_gsc_seat_score_bonus(const GameState *g, int seat);

/* ---- per-turn effect tracking (game_state_core.c:440-503) ---- */
void rb_gsc_clear_effect_tracking(GameState *g);
int  rb_gsc_last_area_move_card_id(const GameState *g);
int  rb_gsc_last_area_move_by_player(const GameState *g);
int  rb_gsc_last_cost_wait_member(const GameState *g);
int  rb_gsc_last_energy_placed_by_effect(const GameState *g);
int  rb_gsc_last_energy_placed_by_player(const GameState *g);
int  rb_gsc_current_ability_source_card_id(const GameState *g);
int  rb_gsc_mulligan_owner_index(const GameState *g);
int  rb_gsc_recently_moved_cards(const GameState *g, int *out, int max);
void rb_gsc_set_recently_moved_cards(GameState *g, const int *cards, int n);
void rb_gsc_push_revealed_card(GameState *g, int card_id);
const char *rb_gsc_player_prefix_id(const GameState *g);

/* ---- snapshots / zones / choices (game_state_core.c:529-711) ---- */
int  rb_gsc_zone_is_private(const char *zone);
int  rb_gsc_resolve_index_any_player(const GameState *g, const char *zone, int idx,
                                     char *out, size_t out_sz);
int  rb_gsc_choice_offered_labels(const GameState *g, const RbChoice *ch,
                                  char *out, size_t out_sz);
int  rb_gsc_choice_option_count(const RbChoice *ch);

/* ── Ability queue (ability_queue.c) ──────────────────────────────────────────
 * These two had file-scope `extern` declarations in their callers, so the file
 * compiled by accident rather than by contract — the same defect that hid 45
 * rb_gsc_* prototypes. rb_queue_is_entry_available was already here at :1890. */
void       rb_queue_clear_completed(GameState *g);
const char *rb_queue_entry_player_id(const GameState *g, int index);

/* ---- logging: these four ALSO need the rule_log member added above ----
 * CORRECTED 2026-10-03. This took NO GameState*, and that was not a faithful
 * extraction — it was the bug. Rust's is a `&mut self` method
 * (core/game_state/mod.rs:1535), so the C form STRUCTURALLY CANNOT reach
 * g->rule_log and had to drop the line. My header note claiming it was "the
 * shared sink" was wrong: a sink with no state has nowhere to sink to.
 * Taking GameState* unblocks ~25 existing rb_log_push_verdict(line,"rule_log",1)
 * call sites into one-token swaps. */
void rb_gsc_push_rule_log_fmt(GameState *g, const char *fmt, ...);
void rb_gsc_log_entry(GameState *g, const char *text, const char *player_label,
                      int source_card_id, const char *source_card_name,
                      const char *category);
void rb_gsc_push_choice_offered(const GameState *g, const RbChoice *ch);
void rb_gsc_push_choice_resolved(const GameState *g, const RbChoice *ch,
                                 const char *chosen, int skipped);

/* ---- snapshots / traces (game_state_core.c:778-848) ---- */
void rb_gsc_push_performance_snapshot(GameState *g, const RbLiveSnapshot *snap);
void rb_gsc_record_ability_application(GameState *g, int source_card_id,
                                        const char *ability_text,
                                        const char *effect_type,
                                        int target_card_id, int heart_color,
                                        int amount);
/* The audit doc rendered this parameter as `RbTraceNode`; the real type is
 * RbAbilityTraceNode (game_state_core.c:816). The audit's spelling does not
 * compile. See the note above about the rest of the audit's signatures. */
void rb_gsc_print_trace_node(const RbAbilityTraceNode *node, int indent);

/* ── Cluster E entry points, landed 2026-10-03 in src/core/modifiers.c. ────────
 * Declared HERE rather than beside the other rb_mods_* because they all take a
 * `const GameState *`, and GameState is not defined until ~line 1800 — putting
 * them next to rb_mods_get_cost broke 48 translation units at once.
 *
 * Each has 0 PRODUCTION callers today: its call site lives in a file another
 * cluster owns. They are declared because they are correct and reachable, NOT
 * because they are wired — see PARITY_CHECKLIST.md before assuming E9/E10/E11/
 * E12/E16 are closed. Signatures are verbatim from the definitions. */
void rb_record_stage_position_swap(GameState *g, int source_id, int source_index,
                                   int target_id, int target_index);
unsigned rb_mods_count_as_multiple_weight(const GameState *g, int counted,
                                          int counting_card);
unsigned rb_mods_weighted_zone_count(const GameState *g, const int *cards,
                                      int n_cards, const char *card_type,
                                      const char *group_names, int counting_card);
int rb_mods_replacement_appear_wait_applies(const GameState *g, int incoming,
                                           int area_idx);

/* MemberContribution is declared ABOVE RbLiveSnapshot (it is a member of it).
 * Its definition and rationale live with the other live.rs-derived types. */


/* ── live.rs exports that were `static` and are now cross-file (live.c) ─────
 * process_yell_revealed_card_icons is `pub fn` in Rust (live.rs:112) and is
 * called from TWO places there (live.rs:1997 and reyell.rs:160). The C had
 * THREE hand-rolled copies of that one decode - live.c, rules.c:1992
 * `r_yell_icons`, and phase.c:1369 `ptc_yell_had_draw_icon`. The live.c twin is
 * now exported so the other two can be collapsed onto it. Flat out-param form;
 * returns 1 when the card decoded, 0 when the DB lookup misses. */
int rb_process_yell_revealed_card_icons(const GameState *g,
        int cid, int override_color, int total_hearts[8], int *cheer_count,
        int blade_hearts_out[8], int *note_icons_out, int *draw_icons_out);
/* Rust `format!("P{}", digits)` (live.rs:3185-3197), via log_performance_summary
 * (live.rs:1258-1289) - which is called from execute_live_victory_determination. */
void rb_fmt_player_id(const char *id, char *out, size_t sz);


/* Rust `validate_mandatory_cost` (cost/handlers.rs:281). STRICTLY WEAKER than
 * rb_validate_cost, and it exists as a separate function precisely because Rust's
 * two offer sites pass `Some(card_id)` (game_setup.rs:2188 and :2295) - so a
 * mandatory cost is validated against a KNOWN CARD, not against whatever card the
 * actor could otherwise play. The C used rb_validate_cost at game_setup.c:1199,
 * which is the wrong twin. It is strictly weaker, so switching removes refusals:
 * expect movement. It also belongs in turn/actions/mod.rs:558-564 (after
 * activating_card is published, before hand->waitroom). */
int rb_validate_mandatory_cost(const GameState *g, int actor,
                               const AbilityEffect *cost, int activating_card);

#endif /* RABUKA_H */
