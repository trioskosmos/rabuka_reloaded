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
void rb_unload(void);
uint32_t rb_num_cards(void);
uint32_t rb_num_abilities(void);

/* ── String table (abilities) ── */
const char *rb_get_string(uint32_t idx);

/* ── Decode ── */
int  rb_decode_ability(uint32_t idx, Ability *out);     /* returns 1 on success */
void rb_free_ability(Ability *a);
void rb_free_condition(Condition *c);
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

/* A zone is just a bag of card indices (card_no index into the database). */
typedef struct {
    int cards[RB_MAX_ZONE];
    int n;
} RbBag;

typedef struct {
    RbBag     hand;
    RbBag     deck;
    int       stage[RB_STAGE_SIZE];   /* card_no index or -1 */
    int       stage_wait[RB_STAGE_SIZE]; /* 1 if member is in "wait" state */
    RbBag     under_cards[RB_STAGE_SIZE]; /* cards placed under each stage member */
    RbBag     energy;                 /* energy cards in energy zone */
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

/* ── Choice / ability queue (engine/src/ability/choice.rs + ability_queue.rs) ── */
typedef enum {
    RB_CHOICE_NONE = 0,
    RB_CHOICE_SELECT_CARD,
    RB_CHOICE_SELECT_TARGET, /* pay_skip / position|destination / double_baton etc. */
    RB_CHOICE_SELECT_HEART_COLOR,
    RB_CHOICE_SELECT_NUMBER  /* ability/choice.rs select_number (count choice) */
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

/* QueueState  Emirrors engine/src/ability_queue.rs::QueueState FSM. */
typedef enum {
    RB_QUEUE_IDLE = 0,
    RB_QUEUE_RESOLVING,
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
} RbChoice;

typedef struct {
    int card_id;      /* activating card's deck index (0..4095) */
    int ability_idx;  /* 0..n */
    int cost_paid;    /* 1 after cost emitted */
    int effect_started;
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
} RbAbilityQueue;

int  rb_queue_push(RbAbilityQueue *q, int card_id, int ability_idx);
void rb_queue_clear(RbAbilityQueue *q);
int  rb_queue_has_pending(const RbAbilityQueue *q);
RbQueueState rb_queue_state(const RbAbilityQueue *q);
void rb_queue_set_state(RbAbilityQueue *q, RbQueueState s);
int rb_use_limit_reached(RbAbilityQueue *q, int card_id, int ability_idx, int limit, int cur_turn);
void rb_record_use(RbAbilityQueue *q, int card_id, int ability_idx, int cur_turn);
void rb_choice_set_route(RbChoice *ch, RbChoiceRoute r);

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
    int      live_success[2];   /* per player: did this player pass their live this turn */
    int      live_score[2];      /* per player: total score from the most recent live performance */
    int      p1_live_won;       /* Rule 8.4.13: P1 won the live (placed to success) this turn */
    int      p2_live_won;       /* Rule 8.4.13: P2 won the live (placed to success) this turn */
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
    int      active;
    int      first_attacker;
    int      second_attacker;
    int      turn;
    int      winner;
    RbPhase  phase;
    int      rps[2];
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
    /* ── temporal-condition tracking (mirrors GameState.has_card_moved_this_turn /
        debut_count_this_turn; position_change_occurred_this_turn already declared above) ── */
    int      moved_this_turn[RB_MAX_CARD_IDS]; /* per-card: moved during current turn */
    int      debut_count_this_turn[2];         /* members debuted this turn per player */
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
void rb_resume_position_change(GameState *g, int actor, const AbilityEffect *e, int host_cid, int selected_idx);

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
void rb_shuffle(int *a, int n);
int  rb_zone_of_str(const char *s, RbZone *out);    /* map zone wire name */

/* ── Play a card from hand ── */
int  rb_play_card(GameState *g, int pl, int hand_idx);
int  rb_play_member(GameState *g, int pl, int hand_idx, int stage_pos); /* to stage */
int  rb_activate_ability(GameState *g, int pl, int hand_idx);
int  rb_activate_card(GameState *g, int pl, int card_id); /* run the card's 起動 (Activate) ability: cost + effect */
/* Baton-touch support (replace an occupied stage member). */
int  rb_card_arrived_this_turn(const GameState *g, int pl, int card_id);
int  rb_card_has_restriction(const GameState *g, int card_id, const char *restriction);
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
int  rb_check_permanent_loop(const GameState *g);
void rb_player_refresh(GameState *g, int pl);

/* ── Card classification (mirrors Rust Card::is_live / is_energy) ── */
int rb_card_is_live(int card_id);
int rb_card_is_energy(int card_id);
void rb_calc_stage_hearts(const GameState *g, int pl, int out[8]);
void rb_stage_hearts_pipeline(const GameState *g, int pl, int out[8]);
void rb_effective_need_heart(const GameState *g, int live_cid, int out[8]);
int  rb_perform_live(GameState *g, int pl);
/* Effects  Everb handlers */
void rb_effect_move_cards(GameState *g, int actor, AbilityEffect *e);
void rb_effect_gain_surplus_heart(GameState *g, int actor, const AbilityEffect *e);
void rb_effect_look_at(GameState *g, int actor, AbilityEffect *e);
void rb_effect_reveal_until_live_card(GameState *g, int actor, AbilityEffect *e);
void rb_effect_reveal_until_chosen_card(GameState *g, int actor, AbilityEffect *e);
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
void rb_effect_change_state(GameState *g, int actor, AbilityEffect *e);
void rb_effect_position_change(GameState *g, int actor, AbilityEffect *e, int host_cid);
void rb_effect_rotation(GameState *g, int actor, AbilityEffect *e);
void rb_effect_modify_cost(GameState *g, int actor, AbilityEffect *e);
void rb_effect_modify_hearts(GameState *g, int actor, AbilityEffect *e);
void rb_effect_energy_placement(GameState *g, int actor, AbilityEffect *e);
void rb_effect_energy_state_change(GameState *g, int actor, AbilityEffect *e);
int  rb_execute_modify_score(GameState *g, int actor, AbilityEffect *e);
void rb_log_set_enabled(int enabled);
void rb_log_push_verdict(const char *text, const char *kind, int passed);
int  rb_log_buffer_len(void);
void rb_log_clear_verdicts(void);

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
int rb_card_matches_type(int card_id, const char *filter);
/* card_property predicates (card.rs::has_blade_heart/has_score_icon/has_all_blade) */
int rb_card_has_blade_heart(const Card *c);
int rb_card_has_score_icon(const Card *c);
int rb_card_has_all_blade(const Card *c);
int  rb_orientation_matches_state(const char *orientation, const char *state);
int  rb_card_matches_group_str(int card_id, const char *group_name);
void rb_set_card_identity(int cid, const char *name);
int  rb_card_matches_identity_str(int card_id, const char *group_name);
int  rb_card_at_position(const struct GameState *g, int pl, const char *pos);
int  rb_pos_to_area(const char *pos);
int  rb_zone_cards(const struct GameState *g, int pl, const char *zone,
                   int *out_ids, int max);

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
int rb_cost_has_skip_prompt(const AbilityEffect *cost);
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

/* ── Ability resolver frontend (engine/src/ability/resolver.rs) ── */
typedef struct AbilityInfo {
    int cid;            /* card id */
    int ability_idx;    /* index within card's abilities */
    const char *trigger;
} AbilityInfo;
int  rb_resolver_pending_choice(const GameState *g);
int  rb_can_activate_effect(const GameState *g, int actor, const AbilityEffect *eff);
int  rb_resolver_trigger_infos(const GameState *g, int actor, const char *trigger,
                               AbilityInfo *out, int max);
int  rb_resolve_ability(GameState *g, int actor, const AbilityEffect *eff, int *resolved);
int  rb_resolver_card_matches_type(int cid, const char *filter);

/* ── Auto-trigger engine + ability use tracking (core/game_state/abilities.rs) ── */
int  rb_ability_matches_trigger(const Ability *ab, const char *trigger);
void rb_record_ability_use(GameState *g, int cid, int idx);
int  rb_collect_constant_hand(const GameState *g, int actor, AbilityEffect *out, int max);
int  rb_collect_live_modifiers(const GameState *g, int actor, AbilityEffect *out, int max);
int  rb_trigger_auto_abilities(GameState *g, int actor, const char *trigger);
/* Mirror live.rs::determine_winners — who placed a live this turn (score-tie → both). */
void rb_determine_live_winners(const GameState *g, int *p1_won, int *p2_won);
int  rb_process_pending_auto_abilities(GameState *g);
void rb_check_expired_effects(GameState *g, int which);
int  rb_apply_ability_effects(GameState *g, int actor, const Ability *ab, int host_cid);

/* ── Misc effect handlers (engine/src/ability/effects/misc.rs) ── */
int rb_execute_misc_effect(GameState *g, int actor, const RbPlayer *self,
                           const AbilityEffect *e, int *resolved);

#endif /* RABUKA_H */
