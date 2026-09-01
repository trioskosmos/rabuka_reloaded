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
    RB_COND_ALL_REVEALED    /* all_revealed_match_heart_color */
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
    struct AbilityEffect *followup_action;
    struct AbilityEffect *optional_action;
    struct AbilityEffect *conditional_action;
    Condition *result_condition;       /* a Condition, not an effect */
    Condition *alternative_condition;   /* a Condition, not an effect */
    int   repeat_limit;                /* repeat_procedure: max ADDITIONAL iterations */
    int   conditional_flag;            /* effect.compound.conditional */
    int   conditional_negation;        /* effect.compound.conditional_negation (on_optional) */
    int   per_unit;                    /* per_unit count (0 = none) */
    int   per_unit_count;
    int   distinct_flag;               /* per-unit distinct filter */
    char  id_field[32];                /* effect id (step output ref key) */
    char  self_target_field[8];        /* "true"/"false" */
    char  card_type_field[24];         /* member_card / live_card / energy_card */
} AbilityEffect;

typedef struct Ability {
    char *full_text;
    char *triggerless_text;
    char *triggers;
    int   use_limit;    /* -1 = none */
    int   is_null;
    AbilityEffect *cost;    /* nullable */
    AbilityEffect *effect;  /* nullable */
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
#define RB_MODS_TRACE_TEXT 48
typedef enum {
    RB_EFFECT_BLADE_BONUS = 0,
    RB_EFFECT_HEART_BONUS = 1
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
    int16_t         constant_heart[RB_MAX_CARD_IDS][8];  /* per-color, cleared on recalc */
    int16_t         constant_need_heart[RB_MAX_CARD_IDS][8];
    int16_t         heart_copy[RB_MAX_CARD_IDS];   /* target→source */
    int8_t          heart_multiplier[RB_MAX_CARD_IDS]; /* -1 none, else colour 0..7 */
    int8_t          heart_multiplier_amt[RB_MAX_CARD_IDS]; /* multiplier applied to that colour (default 2) */
    int8_t          blade_type[RB_MAX_CARD_IDS];   /* -1 none, else BladeColor idx */
    int8_t          heart_color_override[RB_MAX_CARD_IDS]; /* -1 none (specify_heart_color); else all base hearts counted as this colour */
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
void rb_mods_remove_score(RbMods *m, int card_id, int delta);
void rb_mods_remove_cost(RbMods *m, int card_id, int delta);
/* heart_override / heart_copy / blade_type / heart_color_multiplier
   (mirror GameModifiers set/get/clear accessors for those fields) */
void rb_mods_set_heart_override(RbMods *m, int card_id, int color);
void rb_mods_remove_heart_override(RbMods *m, int card_id);
int  rb_mods_get_heart_override(RbMods *m, int card_id);
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

/* ── Decode ── */
int  rb_decode_ability(uint32_t idx, Ability *out);     /* returns 1 on success */
int  rb_get_ability(uint32_t idx, Ability *out);         /* returns 1 even for empty/default */
int  rb_count_empty_bytecode_abilities(void);            /* audit: empty slices */
void rb_free_ability(Ability *a);
void rb_free_condition(Condition *c);

/* ── Trigger system (mirrors engine/src/triggers.rs) ── */
typedef enum {
    RB_TK_ACTIVATION = 0, RB_TK_AUTO, RB_TK_CONSTANT, RB_TK_DEBUT,
    RB_TK_LIVE_START, RB_TK_LIVE_SUCCESS, RB_TK_MAIN, RB_TK_BATON_TOUCH, RB_TK_COUNT
} RbTriggerKind;
RbTriggerKind rb_trigger_from_token(const char *s);
int rb_parse_triggers(const char *triggers, RbTriggerKind *out, int max);
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
int  rb_decode_card_by_index(uint32_t i, Card *out);    /* 0..num_cards-1 */
void rb_free_card(Card *c);
uint16_t rb_card_ability_idx(uint32_t i);   /* 0xFFFF if none  Efirst ability only (legacy) */
const unsigned char *rb_card_record(uint32_t i);
const unsigned char *rb_bc_slice(uint32_t idx, uint32_t *out_len);
const char *rb_card_string(uint16_t idx);
int rb_find_card_by_no(const char *card_no); /* linear scan cards.bin card_no strings, -1 if not found */
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
    RB_ABILITY_ZONE_UNKNOWN
} RbAbilityZone;

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
    RB_PHASE_DONE
} RbPhase;

const char *rb_phase_name(int phase);

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

/* AbilityError codes — mirrors engine/src/ability/types.rs::AbilityError */
typedef enum {
    RB_AE_NO_MEMBER_IN_TARGET_AREA = 0,
    RB_AE_AREA_LOCKED,
    RB_AE_BATON_TOUCH_PROTECTION,
    RB_AE_INVALID_HAND_INDEX,
    RB_AE_NOT_MEMBER_CARD,
    RB_AE_CARD_NOT_FOUND,
    RB_AE_ZONE_FULL
} RbAbilityError;

/* QueueState  Emirrors engine/src/ability_queue.rs::QueueState FSM. */
typedef enum {
    RB_QUEUE_IDLE = 0,
    RB_QUEUE_RESOLVING,
    RB_QUEUE_PAYING_COST,
    RB_QUEUE_AWAITING_CHOICE,
    RB_QUEUE_DRAINING
} RbQueueState;

typedef struct {
    RbChoiceKind kind;
    char zone[32];        /* e.g. "hand", "looked_at" */
    char card_type[32];   /* member_card / live_card / energy_card */
    int  count;           /* how many to pick */
    int  allow_skip;      /* 1 = may skip */
    char target[64];      /* for SELECT_TARGET: "pay_optional_cost:skip..." etc. */
    char description[128];
    RbChoiceRoute route;  /* which gate produced this choice (ChoiceRoute) */
    /* selection filter — mirrors engine/src/ability/choice.rs SelectionContext
        (card_type is already above; group_names + heart_colors narrow the pool
        further so a host UI / test picks a valid card). Empty/negative = no filter. */
    char filter_group[32];
    int  filter_heart;     /* heart color idx, -1 = none */
    /* heart-color choice option list (mirrors Choice::SelectHeartColor.options).
        Populated by rb_effect_select_heart_color so the host can present the
        exact palette; rb_resume_with_choice maps the picked index back to a
        color and stores it in queue.selected_heart_color. */
    char heart_options[8][24];
    int  n_heart_options;
} RbChoice;

typedef struct {
    int card_id;      /* activating card's deck index (0..4095) */
    int ability_idx;  /* 0..n */
    int cost_paid;    /* 1 after cost emitted */
    int effect_started;
    int completed;    /* 1 after entry is done (mirrors Rust entry.completed) */
    int optional_cost_result; /* -1 none, 0 declined, 1 paid (mirrors Rust entry.optional_cost_result) */
    int pending_actions_n;    /* count of pending sequential actions */
    /* condition_cache — mirrors Rust AbilityQueueEntry.condition_cache: Vec<(String,bool)> for cache:true conditions.
       The C port stores up to 8 stringified condition keys (hash of condition pointer + variant) + verdict. */
#define RB_COND_CACHE_CAP 8
    int  cond_cache_keys[RB_COND_CACHE_CAP]; /* hash of condition pointer (or content hash) */
    int  cond_cache_vals[RB_COND_CACHE_CAP]; /* 0/1 verdict */
    int  n_cond_cache;
    char player_id[16]; /* mirrors Rust entry.player_id (e.g. "player1") for cost-reduction hooks */
} RbQueueEntry;

#define RB_QUEUE_DEPTH 16
#define RB_USE_TRACK  256  /* (card_id<<4|ability_idx) slots per turn */

typedef struct {
    RbChoice pending;
    int      has_pending;      /* 1 if choice is waiting for player input */
    int      actor;            /* player who must answer (0/1) */
    AbilityEffect *deferred;   /* sequential remainder after pay_skip gate */
    RbQueueEntry entries[RB_QUEUE_DEPTH];
    int      n_entries;
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
    /* optional-cost continuation: when an optional pay_energy/cost gate emits a
       choice, the executing effect's parent + the index of that gate are stashed
       here so the resume can run the ability's remaining sibling effects. */
    const AbilityEffect *resume_parent;
    int      resume_child;
    /* heart-color choice result (mirrors Rust execute_choice → conditional_choice =
        Str(color) consumed by execute_gain_resource). Set when a select/choice with a
        heart_color extra is answered; the following gain_resource reads it. -1 = none. */
    int      selected_heart_color;
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
} RbAbilityQueue;

int  rb_queue_push(RbAbilityQueue *q, int card_id, int ability_idx);
void rb_queue_clear(RbAbilityQueue *q);
int  rb_queue_has_pending(const RbAbilityQueue *q);
RbQueueState rb_queue_state(const RbAbilityQueue *q);
void rb_queue_set_state(RbAbilityQueue *q, RbQueueState s);
int rb_use_limit_reached(RbAbilityQueue *q, int card_id, int ability_idx, int limit, int cur_turn);
void rb_record_use(RbAbilityQueue *q, int card_id, int ability_idx, int cur_turn);
int rb_use_count(RbAbilityQueue *q, int card_id, int ability_idx, int cur_turn);
void rb_choice_set_route(RbChoice *ch, RbChoiceRoute r);
void rb_choice_set_description(RbChoice *ch, const char *desc);

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
    int surplus_per_color[8];          /* per-color surplus (compute_surplus_and_flags) */
} RbLiveSnapshot;

/* A modifier granted by a trigger with a Duration that must be reverted when
   the duration expires. Mirrors engine/src/core/game_state/mod.rs TemporaryEffect.
   dur: 0 = permanent (no expiry), 1 = live_end/during_live (revert at live
   phase end), 2 = until_end_of_turn/first_turn (revert at turn rollover). */
#define RB_MAX_TEMP_EFFECTS 64
#define RB_TEMP_PERM      0
#define RB_TEMP_LIVE_END  1
#define RB_TEMP_TURN_END  2
typedef struct {
    int card_id;            /* host card the effect belongs to */
    int dur;                /* RB_TEMP_* duration kind */
    int blade;
    int score;
    int cost;
    int heart[8];
    int need_heart[8];
} RbTempEffect;

#define RB_MAX_SNAPSHOTS 64
#define RB_MAX_RECENTLY_MOVED 8
typedef struct GameState {
    RbPlayer p[2];
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
    char     yell_source[2][16];   /* per-player modify_yell_source override (live.c do_yell) */
    RbLiveSnapshot snapshots[RB_MAX_SNAPSHOTS];
    int      n_snapshots;
    int      recently_moved[RB_MAX_RECENTLY_MOVED];
    int      n_recently_moved;
    int      those_cards[RB_MAX_RECENTLY_MOVED]; /* cards moved by the immediately preceding move_cards action (Rust `those_cards` relay) */
    int      n_those_cards;
    int      selected_cards[RB_MAX_RECENTLY_MOVED]; /* cards chosen by a select_cards/select/look_and_select choice */
    int      n_selected_cards;
    int      assignment[RB_MAX_RECENTLY_MOVED]; /* distinct-name assignment for alt-cost (phases.rs) */
    int      n_assignment;
    int      live_success[2];   /* per player: did this player pass their live this turn */
    int      live_score[2];      /* per player: total score from the most recent live performance */
    int      p1_live_won;       /* Rule 8.4.13: P1 won the live (placed to success) this turn */
    int      p2_live_won;       /* Rule 8.4.13: P2 won the live (placed to success) this turn */
    /* live-surplus / no-excess flags (mirror live.rs::compute_surplus_and_flags +
        record_pretrigger_live_results). Zeroed by rb_game_init. */
    int      self_live_surplus_count;
    int      opponent_live_surplus_count;
    int      live_surplus_ready_this_turn;
    int      p1_live_success_no_excess;
    int      p2_live_success_no_excess;
    /* state_change_condition tracking (mirrors Rust recently_state_changed /
       turn_state_changes). Set when a member's orientation actually flips during
       rb_effect_change_state; cleared at turn rollover. from/to are orientation
       indices (0=active/none,1=wait) keyed by card id. */
    int8_t   state_change_from[RB_MAX_CARD_IDS];
    int8_t   state_change_to[RB_MAX_CARD_IDS];
    int      last_wait_to_active_count; /* count of wait→active flips this turn */
    int      revealed_cards[RB_MAX_RECENTLY_MOVED]; /* cards revealed by yell/re_yell */
    int      n_revealed;
    int      last_draw_count;   /* mirror AbilityResolver.step_state.last_draw_count */
    int      last_surplus_loss_count[2]; /* gain_surplus_heart: surplus hearts gained/lost this live (misc.rs) */
    int      re_yell_occurred;  /* a re_yell effect fired this live */
    int      re_yell_blade_hearts[8]; /* hearts harvested by perform_yell, applied to live */
    int      re_yell_note_icons;
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
    RbTempEffect temp_effects[RB_MAX_TEMP_EFFECTS];
    int      n_temp_effects;
    int      stage_arrived[2][RB_STAGE_SIZE]; /* set when a member was deployed this turn (baton arrival-ban, Rule 9.6.2.1.2.1) */
    int      baton_touch_used[2];             /* baton used this play-action (once-per-action limit) */
    int      baton_last_vacated_area[2];      /* stage area vacated by the most recent baton (mirrors Rust last_vacated_stage_area) */
    int      current_is_baton;                /* context flag threaded to effect/trigger evaluation */
    int      player_cannot_activate[2];      /* restriction: that player may not activate abilities */
    int      cannot_active_cards[RB_MAX_ZONE]; /* restriction: per-card cannot-activate (delayed/next-turn) */
    int      n_cannot_active_cards;
    char     prohibition[64][48];            /* restriction: "type:destination" prohibition notes */
    int      n_prohibition;
    /* ── tracking.rs (ported) ── */
    int      turn1_abilities_played[64]; int n_turn1_abilities_played;
    int      turn2_abilities_played[64]; int n_turn2_abilities_played;
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
    int      areas_placed_this_turn[16]; int n_areas_placed_this_turn;
    int      cards_appeared_this_turn[64]; int n_cards_appeared_this_turn;
    int      auto_ability_trigger_counts[32]; int n_auto_ability_trigger_counts;
    int      position_change_occurred_this_turn;
    int      formation_change_occurred_this_turn;
    int      opponent_live_success_this_turn;
    int      game_state_history[64]; int n_game_state_history;
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
    int      energy_placed_this_turn[2];       /* energy placed by player this turn */
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
    int      n_gained_cards;               /* number of distinct cards with gains */
} GameState;

/* ── Tracking (engine/src/core/game_state/tracking.rs) ── */
void rb_reset_keyword_tracking(GameState *g);
void rb_add_yell_count_modifier(GameState *g, uint8_t player_slot, int32_t delta);
void rb_refresh_yell_sources(GameState *g);
uint8_t rb_effective_cheer_checks_required(const GameState *g, const char *player_id, uint8_t base);
int rb_perform_cheer_check(GameState *g, const char *player_id, uint8_t blade_count);
int rb_check_required_hearts(const GameState *g);
int rb_is_action_prohibited(const GameState *g, const char *action);

/* ── Ability queue drain + owner lookup (engine/src/ability_queue.rs) ── */
int rb_owner_of_card(const GameState *g, int cid);
int rb_drain_ability_queue(GameState *g);
void rb_look_resume(GameState *g, int actor, int selected_idx, const char *destination, int is_select);
int rb_look_remove(int pl, int cid);
void rb_look_add(int pl, int cid);
void rb_look_clear(int pl);
/* Queue introspection / control (mirrors AbilityQueue methods) */
int rb_queue_is_idle(const GameState *g);
int rb_queue_has_entry_with_id(const GameState *g, int card_id, int ability_idx);
int rb_queue_start_next(GameState *g);
void rb_queue_complete_current(GameState *g);
int rb_queue_make_entry(GameState *g, int card_id, int ability_idx);
int rb_queue_is_entry_available(const GameState *g, int idx);
int rb_queue_current_entry(const GameState *g);
void rb_queue_promote_entry(GameState *g, int from_index);
void rb_queue_promote_entry_by_abs(GameState *g, int absolute);
void rb_queue_set_current_entry(GameState *g, int absolute);
int rb_queue_has_pending_actions(const GameState *g);
void rb_queue_set_pending_actions(GameState *g, int count);
void rb_queue_save_pending_actions(GameState *g, int count);
int rb_queue_take_pending_actions(GameState *g);
void rb_resume_position_change(GameState *g, int actor, const AbilityEffect *e, int host_cid, int selected_idx);

/* ── Ability queue: pop_constant_context / take_resolver / has_resolver ──
   Mirrors AbilityQueue methods for constant-evaluation context and resolver
   persistence. The C queue model uses a flat array with no per-entry resolver,
   so take_resolver and has_resolver are no-ops. */
void rb_queue_pop_constant_context(GameState *g);
int  rb_queue_take_resolver(GameState *g);
int  rb_queue_has_resolver(const GameState *g);

/* ── RNG (xorshift; deterministic given seed) ── */
void rb_seed(uint32_t s);
uint32_t rb_rand(void);

/* ── Setup ── */
int  rb_game_init(GameState *g, const uint32_t *deck0, int n0,
                  const uint32_t *deck1, int n1);
void rb_turn(GameState *g);            /* advance one full turn */
void rb_print_state(const GameState *g);

/* ── Zone helpers (operate on a player's bags) ── */
int  rb_draw(GameState *g, int pl);                 /* draw 1 to hand */
int  rb_draw_energy(GameState *g, int pl);          /* draw 1 to energy zone */
int  rb_draw_cards_for_player(RbPlayer *player, uint8_t count, const char *source,
                             const char *destination, const char *card_type_filter,
                             int is_any_number, void *distinct, void *card_db, int self_target_id);
/* Faithful port of draw.rs:execute_draw_wrapper + execute_draw. Mirrors the
    resolver's draw effect (count resolution: static / dynamic / zero-special;
    target both/self/opponent; optional pay-skip gate; any_number; per_unit;
    card_type filter; source/destination routing). */
int  rb_effect_draw_card(GameState *g, int actor, AbilityEffect *e, int host_cid);
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
typedef struct { int card_id; int amount; char color[24]; } RbEffectDataSingleCard;
RbEffectDataSingleCard rb_make_card_effect_data(int card_id, int amount, const char *color);
/* draw.rs:resolve_gain_heart_color — returns a fixed heart color idx, or -1 if a
    choice was emitted / not a heart resource. */
int  rb_resolve_gain_heart_color(GameState *g, int actor, AbilityEffect *e,
                                 const char *resource, int count,
                                 const char **heart_colors, int n_colors, int heart_selection);
void rb_shuffle(int *a, int n);
int  rb_zone_of_str(const char *s, RbZone *out);    /* map zone wire name */

/* ── Play a card from hand ── */
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
int  rb_trigger_live_success(GameState *g, int pl);
int  rb_should_trigger_live_success(const GameState *g, int pl);
int  rb_drain_live_success_choices(GameState *g);
int  rb_queue_trigger_abilities(GameState *g, int pl, const char *trigger);
int  rb_fire_auto(GameState *g, int pl);
int  rb_fire_all_auto(GameState *g, int pl);
int  rb_fire_auto_and_pending(GameState *g, int pl);
void rb_record_event(GameState *g, int pl, const char *trig);
int  rb_fire_recorded_auto(GameState *g, int pl);
int  rb_process_pending_auto_abilities(GameState *g);
void rb_recalc_constants(GameState *g);
void rb_check_expired_effects(GameState *g, int which);
void rb_advance_phase(GameState *g);

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

void rb_calc_stage_hearts(const GameState *g, int pl, int out[8]);
void rb_stage_hearts_pipeline(const GameState *g, int pl, int out[8]);
void rb_effective_need_heart(const GameState *g, int live_cid, int out[8]);
int  rb_perform_live(GameState *g, int pl);
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
void rb_effect_reveal_until_live_card(GameState *g, int actor, AbilityEffect *e);
void rb_effect_reveal_until_chosen_card(GameState *g, int actor, AbilityEffect *e);
void rb_effect_reveal_until_target(GameState *g, int actor, AbilityEffect *e);
void rb_effect_select_cards(GameState *g, int actor, AbilityEffect *e);
int  rb_looked_at_pool(int pl, int *out_ids, int max);
void rb_gain_ability(GameState *g, int actor, AbilityEffect *e);
void rb_gain_ability_from_source(GameState *g, int actor, AbilityEffect *e, int host_cid);
void rb_invalidate_ability(GameState *g, int actor, AbilityEffect *e);
void rb_activate_ability_effect(GameState *g, int actor, AbilityEffect *e, int host_cid);
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
void rb_effect_modify_hearts(GameState *g, int actor, AbilityEffect *e);
void rb_effect_energy_placement(GameState *g, int actor, AbilityEffect *e);
void rb_effect_energy_state_change(GameState *g, int actor, AbilityEffect *e);
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

/* ── Ability tracing (engine/src/ability/debug.rs: ABILITY_DEBUG) ──
   Mirrors the Rust `ABILITY_DEBUG` atomic gate: diagnostic traces are compiled
   in permanently (they are part of the engine, per AGENTS.md) but only print
   when tracing is switched on, so a full suite run stays quiet. */
int  rb_ability_debug_enabled(void);
void rb_ability_debug_set(int enabled);

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
int rb_card_has_blade_heart(const Card *c);
int rb_card_has_score_icon(const Card *c);
int rb_card_has_all_blade(const Card *c);
int  rb_orientation_matches_state(const char *orientation, const char *state);
int  rb_card_matches_group_str(int card_id, const char *group_name);
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
/* Canonical group taxonomy — exactly the entries recognized by
    rb_card_series_matches_group / card_matches_group_str. */
extern const char *RB_KNOWN_GROUPS[5];
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
int  rb_resolution_clear(GameState *g, int *out, int max);
int  rb_resolution_len(const GameState *g);

/* ── HeartColor parsing (engine/src/core/card.rs parse_heart_color / index) ── */
/* Faithful port of `s.parse::<HeartColor>()` / `HeartColor::index()`. String
   ↁERbHeartColor; "b_"-prefixed blade hearts strip the prefix and recurse;
   "heart07"/"b_heart07" ↁEcolorless (RB_HEART_PINK / index 0); unknown ↁEpink. */
RbHeartColor rb_parse_heart_color(const char *s);
int          rb_heart_index(RbHeartColor c);

/* ── Effect execution (public for testing / harness) ── */
void rb_execute_effect(GameState *g, int actor, AbilityEffect *e);
/* Like rb_execute_effect but carries the resolving card id (Rust activating_card)
   so per-card modifiers (blade/heart) attribute correctly. */
void rb_execute_effect_ex(GameState *g, int actor, AbilityEffect *e, int host_cid);

/* ── Choice API (portable shim calls these instead of reading GameState directly) ── */
int       rb_has_pending_choice(const GameState *g);
const RbChoice *rb_get_pending_choice(const GameState *g);
int       rb_resume_with_choice(GameState *g, int selected_idx); /* 0..count-1, -1=skip */
void      rb_clear_pending_choice(GameState *g);

/* ── Ability cost payment (engine/src/ability/cost.rs) ── */
int rb_pay_cost(GameState *g, int actor, const AbilityEffect *cost);
int rb_validate_cost(const GameState *g, int actor, const AbilityEffect *cost);
int rb_pay_deferred_costs(GameState *g, int actor, const AbilityEffect *cost);
int rb_handle_optional_cost_payment(GameState *g, int actor, const AbilityEffect *cost, int pay);
int rb_handle_pay_cost_all_discard(GameState *g, int actor, const char *selected);
int rb_cost_has_skip_prompt(const AbilityEffect *cost);
int rb_compute_play_cost(const GameState *g, int actor, int card_id, int set_override);
int rb_get_change_state_candidates(const GameState *g, int actor,
                                   int *out_positions, int max);

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

/* ── Auto-trigger engine + ability use tracking (core/game_state/abilities.rs) ── */
int  rb_ability_matches_trigger(const Ability *ab, const char *trigger);
void rb_record_ability_use(GameState *g, int cid, int idx);
int  rb_collect_constant_hand(const GameState *g, int actor, AbilityEffect *out, int max);
int  rb_collect_live_modifiers(const GameState *g, int actor, AbilityEffect *out, int max);
int  rb_trigger_auto_abilities(GameState *g, int actor, const char *trigger);
int  rb_trigger_auto_abilities_for_movement(GameState *g, int pl);
int  rb_trigger_auto_abilities_for_player_with_event(GameState *g, int pl, const int *moved_cards, int n_moved, int position_change, int energy_placed);
int  rb_trigger_auto_abilities_for_player(GameState *g, int pl);
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
const AbilityEffect *rb_entry_effect(const GameState *g);
const AbilityEffect *rb_entry_cost(const GameState *g);
const char *rb_entry_destination(const GameState *g);
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

/* ── Loop detection ── */
void rb_reset_loop_detection(GameState *g);
int rb_is_loop_detected(const GameState *g);

/* ── Replacement effects (stubs) ── */
void rb_add_replacement_effect(GameState *g, int card_id, int player_id, const char *original_event, const AbilityEffect *replacement_effects, int n_replacement, int is_choice_based);
void rb_reset_replacement_effect_flags(GameState *g);
void rb_mark_replacement_effect_applied(GameState *g, int card_id);

/* ── Turn/live state reset ── */
void rb_set_opponent_live_success(GameState *g, int no_excess_heart);
void rb_reset_change_flags(GameState *g);

/* ── Choice context injection (stub) ── */
void rb_inject_choice_ability_context(GameState *g, char *json_buf, size_t buf_sz);

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
     Mirrors EffectData enum methods (C EffectData is flattened to single-card). */
int rb_effect_data_items(const RbEffectDataSingleCard *d, int card_id,
                         int *out_amount, char *out_color, size_t color_sz);
int rb_effect_data_is_p1(const RbEffectDataSingleCard *d);
int rb_effect_data_old_value(const RbEffectDataSingleCard *d);
int rb_effect_data_count(const RbEffectDataSingleCard *d);
const char *rb_effect_data_color(const RbEffectDataSingleCard *d);
int rb_effect_data_amount(const RbEffectDataSingleCard *d);

/* ── types.rs: ZoneId::equivalent / matches_source ──
     Zone aliasing for rule-purpose equivalence and zone-change condition matching. */
int rb_zone_equivalent(RbZoneId a, RbZoneId b);
int rb_zone_matches_source(RbZoneId zone, const char *source);

/* ── types.rs: ZoneId::as_str ──
     Converts a ZoneId enum to its wire string. Returns NULL for Unknown. */
const char *rb_zone_id_as_str(RbZoneId z);

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
int rb_card_get_score(int card_id);

/* ── game_modifiers.rs: ModifierEntry::total ──
     Mirrors ModifierEntry::total — returns set + additive combined. */
int rb_modifier_total_entry(const RbModifierEntry *e);

#endif /* RABUKA_H */
