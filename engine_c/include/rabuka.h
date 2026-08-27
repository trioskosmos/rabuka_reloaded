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
void rb_unload(void);
uint32_t rb_num_cards(void);
uint32_t rb_num_abilities(void);

/* ── String table (abilities) ── */
const char *rb_get_string(uint32_t idx);

/* ── Decode ── */
int  rb_decode_ability(uint32_t idx, Ability *out);     /* returns 1 on success */
void rb_free_ability(Ability *a);
void rb_free_condition(Condition *c);
int  rb_eval_condition(const GameState *g, int actor, const Condition *c); /* 1=truthy */
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
   ════════════════════════════════════════════════════════════════════ */

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

typedef struct {
    RbPlayer p[2];
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

/* ── Effect execution (public for testing / harness) ── */
void rb_execute_effect(GameState *g, int actor, AbilityEffect *e);

#endif /* RABUKA_H */
