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
    int16_t         heart_copy[RB_MAX_CARD_IDS];   /* target→source */
    int8_t          heart_multiplier[RB_MAX_CARD_IDS]; /* -1 none, else colour 0..7 */
    int8_t          blade_type[RB_MAX_CARD_IDS];   /* -1 none, else BladeColor idx */
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
uint16_t rb_card_ability_idx(uint32_t i);   /* 0xFFFF if none */
const unsigned char *rb_card_record(uint32_t i);
const unsigned char *rb_bc_slice(uint32_t idx, uint32_t *out_len);
const char *rb_card_string(uint16_t idx);

/* ════════════════════════════════════════════════════════════════════
    Engine — game state + turn loop + faithful effect execution.
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
    RbBag     energy;                 /* energy cards in energy zone */
    int       energy_active;          /* count of active energy */
    RbBag     live;                   /* live card zone */
    RbBag     success;                /* success live card zone */
    RbBag     discard;                /* waitroom */
    int       score;
    int       hearts[RB_MAX_HEARTS]; /* hearts-by-color on this player */
    int       yell_note_icons;        /* hearts produced during performance */
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

/* ── Choice / ability queue (engine/src/ability/choice.rs + ability_queue.rs) ── */
typedef enum {
    RB_CHOICE_NONE = 0,
    RB_CHOICE_SELECT_CARD,
    RB_CHOICE_SELECT_TARGET, /* pay_skip / position|destination / double_baton etc. */
    RB_CHOICE_SELECT_HEART_COLOR
} RbChoiceKind;

typedef struct {
    RbChoiceKind kind;
    char zone[32];        /* e.g. "hand", "looked_at" */
    char card_type[32];   /* member_card / live_card / energy_card */
    int  count;           /* how many to pick */
    int  allow_skip;      /* 1 = may skip */
    char target[64];      /* for SELECT_TARGET: "pay_optional_cost:skip..." etc. */
    char description[128];
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
} RbAbilityQueue;

int  rb_queue_push(RbAbilityQueue *q, int card_id, int ability_idx);
void rb_queue_clear(RbAbilityQueue *q);
int  rb_queue_has_pending(const RbAbilityQueue *q);
int  rb_use_limit_reached(RbAbilityQueue *q, int card_id, int ability_idx, int limit, int cur_turn);
void rb_record_use(RbAbilityQueue *q, int card_id, int ability_idx, int cur_turn);

typedef struct GameState {
    RbPlayer p[2];
    RbMods   mods;            /* global modifiers (blades/hearts/scores/costs) */
    RbAbilityQueue queue;     /* pending choice / deferred sequential */
    int      live_set_limit_reduction[2]; /* per-player reduce_live_card_set_limit */
    int      active;          /* player taking the normal-phase turn */
    int      first_attacker;  /* 0/1 winner of RPS for first turn */
    int      second_attacker; /* the other player */
    int      turn;            /* turn number (starts at 1) */
    int      winner;          /* -1 none, 0/1 winner, 2 draw */
    RbPhase  phase;
    int      rps[2];          /* 0=rock 1=paper 2=scissors */
    int      live_set_player; /* which player is setting live cards (first/second) */
} GameState;

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
void rb_shuffle(int *a, int n);
int  rb_zone_of_str(const char *s, RbZone *out);    /* map zone wire name */

/* ── Play a card from hand ── */
int  rb_play_card(GameState *g, int pl, int hand_idx);
int  rb_play_member(GameState *g, int pl, int hand_idx, int stage_pos); /* to stage */
int  rb_activate_ability(GameState *g, int pl, int hand_idx);

/* ── Triggers / phase / live / stats_pipeline ── */
int  rb_trigger_is(const char *triggers, const char *needle);
int  rb_trigger_debut(GameState *g, int pl, int card_id);
void rb_recalc_constants(GameState *g);
void rb_advance_phase(GameState *g);
void rb_calc_stage_hearts(const GameState *g, int pl, int out[8]);
void rb_stage_hearts_pipeline(const GameState *g, int pl, int out[8]);
void rb_effective_need_heart(const GameState *g, int live_cid, int out[8]);
int  rb_perform_live(GameState *g, int pl);
/* Effects — verb handlers */
void rb_effect_move_cards(GameState *g, int actor, AbilityEffect *e);
void rb_effect_look_at(GameState *g, int actor, AbilityEffect *e);
void rb_effect_select_cards(GameState *g, int actor, AbilityEffect *e);
void rb_gain_ability(GameState *g, int actor, AbilityEffect *e);
void rb_invalidate_ability(GameState *g, int actor, AbilityEffect *e);
void rb_look_clear(int pl);
void rb_tick_gained(void);
int  card_matches_card_type_filter(int card_idx, const char *filter);
void rb_emit_choice(GameState *g, int actor, RbChoiceKind kind,
                    const char *zone, const char *card_type,
                    int count, int allow_skip, const char *target);
void rb_effect_change_state(GameState *g, int actor, AbilityEffect *e);
void rb_effect_position_change(GameState *g, int actor, AbilityEffect *e);
void rb_effect_modify_cost(GameState *g, int actor, AbilityEffect *e);
void rb_effect_modify_hearts(GameState *g, int actor, AbilityEffect *e);

/* ── Effect execution (public for testing / harness) ── */
void rb_execute_effect(GameState *g, int actor, AbilityEffect *e);

/* ── Choice API (portable shim calls these instead of reading GameState directly) ── */
int       rb_has_pending_choice(const GameState *g);
const RbChoice *rb_get_pending_choice(const GameState *g);
int       rb_resume_with_choice(GameState *g, int selected_idx); /* 0..count-1, -1=skip */
void      rb_clear_pending_choice(GameState *g);

#endif /* RABUKA_H */
