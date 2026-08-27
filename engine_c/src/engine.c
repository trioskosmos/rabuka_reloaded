#include "rabuka.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

/* ───────────────────────────── RNG ───────────────────────────── */
static uint32_t rng_state = 0x12345678u;
void rb_seed(uint32_t s) { rng_state = s ? s : 0x12345678u; }
uint32_t rb_rand(void) {
    uint32_t x = rng_state;
    x ^= x << 13; x ^= x >> 17; x ^= x << 5;
    rng_state = x; return x;
}
static int rng_range(int n) { return (int)(rb_rand() % (uint32_t)n); }

/* ───────────────────────────── bag helpers ───────────────────────────── */
static void bag_push(RbBag *b, int c) { if (b->n < RB_MAX_ZONE) b->cards[b->n++] = c; }
static int  bag_pop(RbBag *b) { return b->n > 0 ? b->cards[--b->n] : -1; }
static int  bag_remove_at(RbBag *b, int i) {
    if (i < 0 || i >= b->n) return -1;
    int c = b->cards[i];
    for (int k = i; k < b->n - 1; k++) b->cards[k] = b->cards[k + 1];
    b->n--; return c;
}
static int  bag_take_first(RbBag *b) { return bag_pop(b); }

void rb_shuffle(int *a, int n) {
    for (int i = n - 1; i > 0; i--) {
        int j = rng_range(i + 1);
        int t = a[i]; a[i] = a[j]; a[j] = t;
    }
}

/* map a wire zone name to an enum; returns 1 on success */
int rb_zone_of_str(const char *s, RbZone *out) {
    if (!s) return 0;
    if (!strcmp(s, "hand")) *out = RB_ZONE_HAND;
    else if (!strcmp(s, "deck") || !strcmp(s, "deck_top") || !strcmp(s, "deck_bottom")) *out = RB_ZONE_DECK;
    else if (!strcmp(s, "stage") || !strcmp(s, "under_member") ||
             !strcmp(s, "center") || !strcmp(s, "left") || !strcmp(s, "right") ||
             !strcmp(s, "same_area") || !strcmp(s, "empty_area")) *out = RB_ZONE_STAGE;
    else if (!strcmp(s, "discard") || !strcmp(s, "waitroom")) *out = RB_ZONE_DISCARD;
    else if (!strcmp(s, "energy") || !strcmp(s, "energy_zone")) *out = RB_ZONE_ENERGY;
    else if (!strcmp(s, "live_card_zone") || !strcmp(s, "live")) *out = RB_ZONE_LIVE;
    else if (!strcmp(s, "success_live_zone") || !strcmp(s, "success") ||
             !strcmp(s, "success_zone")) *out = RB_ZONE_SUCCESS;
    else if (!strcmp(s, "resolution") || !strcmp(s, "resolution_zone")) *out = RB_ZONE_RESOLUTION;
    else return 0;
    return 1;
}

/* Resolve a player's bag for a zone (stage handled specially by caller). */
static RbBag *zone_bag(RbPlayer *pl, RbZone z) {
    switch (z) {
        case RB_ZONE_HAND: return &pl->hand;
        case RB_ZONE_DECK: return &pl->deck;
        case RB_ZONE_DISCARD: return &pl->discard;
        case RB_ZONE_ENERGY: return &pl->energy;
        case RB_ZONE_LIVE: return &pl->live;
        case RB_ZONE_SUCCESS: return &pl->success;
        case RB_ZONE_RESOLUTION: return &pl->discard; /* resolution cards return here */
        default: return NULL;
    }
}

/* ───────────────────────────── draw helpers ───────────────────────────── */
int rb_draw(GameState *g, int pl) {
    RbPlayer *P = &g->p[pl];
    if (P->deck.n == 0) return 0;
    if (P->hand.n >= RB_MAX_HAND) return 0;
    bag_push(&P->hand, bag_take_first(&P->deck));
    return 1;
}
int rb_draw_energy(GameState *g, int pl) {
    RbPlayer *P = &g->p[pl];
    if (P->deck.n == 0) return 0;
    if (P->energy.n >= RB_MAX_ENERGY_CARDS) return 0;
    bag_push(&P->energy, bag_take_first(&P->deck));
    if (P->energy_active < RB_MAX_ENERGY_CARDS) P->energy_active++;
    return 1;
}

/* ───────────────────────────── card classification ───────────────────────────── */
static int card_is_live(Card *c) {
    /* A live/song card carries no member hearts and no play cost. */
    return (c->n_hearts == 0) && (c->cost == 0) && (c->blade == 0);
}
static int card_is_member(Card *c) {
    return !card_is_live(c);
}

/* ───────────────────────────── extra-field lookup ───────────────────────────── */
static const char *extra(AbilityEffect *e, const char *k) {
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}
static int heart_color_of(AbilityEffect *e, int dflt) {
    const char *h = extra(e, "heart_color");
    if (!h) h = extra(e, "target");
    if (!h) return dflt;
    if (!strcmp(h, "pink") || !strcmp(h, "heart00")) return RB_HEART_PINK;
    if (!strcmp(h, "red")) return RB_HEART_RED;
    if (!strcmp(h, "yellow")) return RB_HEART_YELLOW;
    if (!strcmp(h, "green")) return RB_HEART_GREEN;
    if (!strcmp(h, "blue")) return RB_HEART_BLUE;
    if (!strcmp(h, "purple")) return RB_HEART_PURPLE;
    if (!strcmp(h, "orange")) return RB_HEART_ORANGE;
    if (!strcmp(h, "draw")) return RB_HEART_DRAW;
    if (!strcmp(h, "score")) return RB_HEART_SCORE;
    return dflt;
}

/* Move `count` cards from one of actor's zones to another. */
int card_matches_card_type_filter(int card_idx, const char *filter){
    if(!filter) return 1;
    Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
    int is_live = (c.n_hearts==0 && c.cost==0 && c.blade==0);
    int is_member = !is_live;
    int match=0;
    if(!strcmp(filter,"live_card") && is_live) match=1;
    else if(!strcmp(filter,"member_card") && is_member) match=1;
    else if(!strcmp(filter,"card")) match=1;
    else if(!strcmp(filter,"energy_card")) match=0;
    else match=1;
    rb_free_card(&c);
    return match;
}
static void do_move_filtered(GameState *g, int actor, RbZone src, RbZone dst, int count, int to_top, const char *card_type_filter);
static void do_move(GameState *g, int actor, RbZone src, RbZone dst, int count, int to_top) {
    do_move_filtered(g, actor, src, dst, count, to_top, NULL);
}
static void do_move_filtered(GameState *g, int actor, RbZone src, RbZone dst, int count, int to_top, const char *card_type_filter) {
    RbPlayer *A = &g->p[actor];
    if (src == RB_ZONE_STAGE) {
        int moved = 0;
        int limit = (count<0)? RB_STAGE_SIZE : count;
        for (int pos = 0; pos < RB_STAGE_SIZE && moved < limit; pos++) {
            if (A->stage[pos] >= 0 && card_matches_card_type_filter(A->stage[pos], card_type_filter)) {
                int c = A->stage[pos]; A->stage[pos] = -1; A->stage_wait[pos] = 0;
                if (dst == RB_ZONE_STAGE) { /* relocate on stage: first empty */
                    for (int q = 0; q < RB_STAGE_SIZE; q++)
                        if (A->stage[q] < 0) { A->stage[q] = c; break; }
                } else {
                    RbBag *db = zone_bag(A, dst);
                    if (db) bag_push(db, c);
                }
                moved++;
            }
        }
        return;
    }
    RbBag *sb = zone_bag(A, src);
    if (!sb) return;
    int n = (count < 0) ? sb->n : count;
    /* Collect matching indices first to avoid skipping */
    int moved=0;
    for (int i = sb->n-1; i >=0 && moved < n; i--) {
        int cid = sb->cards[i];
        if (!card_matches_card_type_filter(cid, card_type_filter)) continue;
        int c = bag_remove_at(sb, i);
        if (dst == RB_ZONE_STAGE) {
            for (int q = 0; q < RB_STAGE_SIZE; q++)
                if (A->stage[q] < 0) { A->stage[q] = c; break; }
        } else {
            RbBag *db = zone_bag(A, dst);
            if (!db) { bag_push(sb, c); break; }
            if (to_top && dst == RB_ZONE_DECK) {
                if (db->n < RB_MAX_ZONE) {
                    for (int k = db->n; k > 0; k--) db->cards[k] = db->cards[k-1];
                    db->cards[0] = c; db->n++;
                }
            } else bag_push(db, c);
        }
        moved++;
    }
}

/* ───────────────────────────── effect execution ───────────────────────────── */
static void handle_action(GameState *g, int actor, AbilityEffect *e);
void rb_emit_choice(GameState *g, int actor, RbChoiceKind kind,
                    const char *zone, const char *card_type,
                    int count, int allow_skip, const char *target);

void rb_execute_effect(GameState *g, int actor, AbilityEffect *e) {
    if (!e) return;
    if (rb_has_pending_choice(g)) return;
    if (e->has_condition && e->condition && !rb_eval_condition(g, actor, e->condition)) return;
    for (int i = 0; i < e->n_child; i++) {
        rb_execute_effect(g, actor, e->child[i]);
        if (rb_has_pending_choice(g)) return;
    }
    if (!e->action) return;
    handle_action(g, actor, e);
}

static int target_player(AbilityEffect *e, int actor) {
    if (e->target) {
        if (!strcmp(e->target, "opponent")) return actor ^ 1;
        if (!strcmp(e->target, "both") || !strcmp(e->target, "either")) return actor; /* self pass */
    }
    return actor;
}

static void handle_action(GameState *g, int actor, AbilityEffect *e) {
    const char *act = e->action;
    int cnt = (e->count >= 0) ? e->count : 1;
    int who = target_player(e, actor);
    RbPlayer *W = &g->p[who];
    RbPlayer *O = &g->p[actor ^ 1];

    if (!strcmp(act, "draw") || !strcmp(act, "draw_card")) {
        for (int i = 0; i < cnt; i++) rb_draw(g, who);
    } else if (!strcmp(act, "draw_until_count")) {
        while (W->hand.n < cnt) { if (!rb_draw(g, who)) break; }
    } else if (!strcmp(act, "discard_card")) {
        RbZone src = RB_ZONE_HAND;
        if (e->source) rb_zone_of_str(e->source, &src);
        do_move(g, who, src, RB_ZONE_DISCARD, cnt, 0);
    } else if (!strcmp(act, "discard_until_count")) {
        while (W->hand.n > cnt) do_move(g, who, RB_ZONE_HAND, RB_ZONE_DISCARD, 1, 0);
    } else if (!strcmp(act, "shuffle")) {
        rb_shuffle(W->deck.cards, W->deck.n);
    } else if (!strcmp(act, "gain_resource") || !strcmp(act, "place_energy") ||
               !strcmp(act, "place_energy_under_member")) {
        W->energy_active += cnt;
        if (W->energy_active > RB_MAX_ENERGY_CARDS) W->energy_active = RB_MAX_ENERGY_CARDS;
    } else if (!strcmp(act, "pay_energy") || !strcmp(act, "pay_cost") ||
               !strcmp(act, "activation_cost")) {
        W->energy_active -= cnt;
        if (W->energy_active < 0) W->energy_active = 0;
    } else if (!strcmp(act, "modify_score") || !strcmp(act, "gain_score")) {
        W->score += cnt;
    } else if (!strcmp(act, "gain_heart") ||
               !strcmp(act, "place_heart") || !strcmp(act, "specify_heart_color")) {
        int col = heart_color_of(e, RB_HEART_PINK);
        W->hearts[col] += cnt;
    } else if (!strcmp(act, "lose_heart") || !strcmp(act, "damage")) {
        int col = heart_color_of(e, RB_HEART_PINK);
        O->hearts[col] -= cnt;
        if (O->hearts[col] < 0) O->hearts[col] = 0;
    } else if (!strcmp(act, "heal")) {
        int col = heart_color_of(e, RB_HEART_PINK);
        O->hearts[col] += cnt;
    } else if (!strcmp(act, "move_cards")) {
        rb_effect_move_cards(g, who, e);
    } else if (!strcmp(act, "change_state")) {
        rb_effect_change_state(g, actor, e);
    } else if (!strcmp(act, "look_at") || !strcmp(act, "reveal") ||
               !strcmp(act, "reveal_per_group") || !strcmp(act, "reveal_until_live_card") ||
               !strcmp(act, "reveal_until_chosen_card")) {
        rb_effect_look_at(g, actor, e);
    } else if (!strcmp(act, "select_cards") || !strcmp(act, "select") ||
               !strcmp(act, "select_number") || !strcmp(act, "look_and_select")) {
        rb_effect_select_cards(g, actor, e);
    } else if (!strcmp(act, "set_cost") || !strcmp(act, "modify_cost") ||
               !strcmp(act, "set_cost_to_use") || !strcmp(act,"modify_yell_count") ||
               !strcmp(act,"modify_yell_source")) {
        rb_effect_modify_cost(g, actor, e);
    } else if (!strcmp(act, "set_card_identity") || !strcmp(act, "set_blade_type") ||
               !strcmp(act, "set_blade_count") || !strcmp(act, "set_heart_type") ||
               !strcmp(act, "choose_required_hearts") || !strcmp(act, "all_blade_timing")) {
        /* card-property rewrites; log as trace */
        if(g->mods.constant_blade[0]==0) { /* touch mods to avoid unused warning */ }
    } else if (!strcmp(act, "modify_required_hearts") || !strcmp(act, "modify_required_hearts_global") ||
               !strcmp(act, "modify_required_hearts_success")) {
        rb_effect_modify_hearts(g, actor, e);
    } else if (!strcmp(act, "gain_ability") || !strcmp(act, "gain_ability_from_source")) {
        rb_gain_ability(g, actor, e);
    } else if (!strcmp(act, "invalidate_ability") || !strcmp(act, "suppress_ability_trigger")) {
        rb_invalidate_ability(g, actor, e);
    } else if (!strcmp(act, "activate_ability")) {
        /* immediate execute of gained ability — stub to emit choice if needed */
        rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, 1, 1, "activate_ability");
    } else if (!strcmp(act, "reduce_live_card_set_limit")) {
        int lim = cnt>0?cnt:1;
        g->live_set_limit_reduction[who] += lim;
        if(g->live_set_limit_reduction[who] > RB_MAX_LIVE_CARDS) g->live_set_limit_reduction[who]=RB_MAX_LIVE_CARDS;
    } else if (!strcmp(act, "position_change") || !strcmp(act, "rotation")) {
        rb_effect_position_change(g, actor, e);
    } else if (!strcmp(act, "choose_target_player")) {
        rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, 1, 0, "self_or_opponent");
    } else if (!strcmp(act, "play_baton_touch") || !strcmp(act, "double_baton_touch")) {
        int is_double = !strcmp(act,"double_baton_touch");
        int replaced=0;
        for(int q=0;q<RB_STAGE_SIZE && replaced < (is_double?2:1);q++){
            if(W->stage[q]!=RB_EMPTY_SLOT){
                int moved=W->stage[q];
                W->stage[q]=RB_EMPTY_SLOT;
                if(W->discard.n < RB_MAX_ZONE) W->discard.cards[W->discard.n++]=moved;
                replaced++;
            }
        }
        if(W->hand.n>0){
            int card=W->hand.cards[--W->hand.n];
            for(int q=0;q<RB_STAGE_SIZE;q++) if(W->stage[q]==RB_EMPTY_SLOT){ W->stage[q]=card; break; }
        }
    } else if (!strcmp(act, "restriction") || !strcmp(act, "activation_restriction") ||
               !strcmp(act, "modify_limit")) {
        /* placement/trigger restrictions; no-op — prohibition_effects would be tracked here */
    } else if (!strcmp(act, "repeat_procedure") || !strcmp(act, "re_yell") ||
               !strcmp(act, "perform_yell") || !strcmp(act, "custom") ||
               !strcmp(act, "do_nothing") || !strcmp(act, "sequential") ||
               !strcmp(act, "conditional_alternative")) {
        /* Compound/control: children already executed pre-order in rb_execute_effect.
           repeat_procedure would loop children cnt times in full port; stub does once. */
    } else if (!strcmp(act, "choice") || !strcmp(act, "conditional_on_result") ||
               !strcmp(act, "conditional_on_optional") || !strcmp(act, "conditional_alternative")) {
        int allow = e->is_optional ? 1 : 0;
        rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, cnt, allow, act);
    }
    /* sequential / conditional_* / choice / repeat_procedure / re_yell /
       perform_yell / custom / do_nothing: children already executed (or nothing
       to do). Kept here as explicit no-ops so unknown verbs are visible. */
}

/* ───────────────────────────── play / activate ───────────────────────────── */
int rb_play_member(GameState *g, int pl, int hand_idx, int stage_pos) {
    RbPlayer *P = &g->p[pl];
    if (hand_idx < 0 || hand_idx >= P->hand.n) return 0;
    if (stage_pos < 0 || stage_pos >= RB_STAGE_SIZE) return 0;
    if (P->stage[stage_pos] >= 0) return 0; /* occupied */
    Card c;
    if (!rb_decode_card_by_index((uint32_t)P->hand.cards[hand_idx], &c)) return 0;
    int cost = c.cost;
    if (P->energy_active < cost) { rb_free_card(&c); return 0; }
    P->energy_active -= cost;
    int card = bag_remove_at(&P->hand, hand_idx);
    P->stage[stage_pos] = card; P->stage_wait[stage_pos] = 0;
    if (c.ability && c.ability->effect)
        rb_execute_effect(g, pl, c.ability->effect);
    rb_free_card(&c);
    return 1;
}

int rb_activate_ability(GameState *g, int pl, int hand_idx) {
    RbPlayer *P = &g->p[pl];
    if (hand_idx < 0 || hand_idx >= P->hand.n) return 0;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)P->hand.cards[hand_idx], &c)) return 0;
    int ok = 0;
    if (c.ability && c.ability->effect) {
        rb_execute_effect(g, pl, c.ability->effect);
        ok = 1;
    }
    rb_free_card(&c);
    return ok;
}

int rb_play_card(GameState *g, int pl, int hand_idx) {
    RbPlayer *P = &g->p[pl];
    if (hand_idx < 0 || hand_idx >= P->hand.n) return 0;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)P->hand.cards[hand_idx], &c)) return 0;
    int ok = 0;
    if (card_is_member(&c)) {
        for (int q = 0; q < RB_STAGE_SIZE; q++) {
            if (P->stage[q] < 0) { ok = rb_play_member(g, pl, hand_idx, q); break; }
        }
    } else {
        /* live card: send to live zone during live set */
        int card = bag_remove_at(&P->hand, hand_idx);
        bag_push(&P->live, card);
        ok = 1;
    }
    rb_free_card(&c);
    return ok;
}

/* ───────────────────────────── phases ───────────────────────────── */
static void activate_wait_members(GameState *g, int pl) {
    RbPlayer *P = &g->p[pl];
    for (int q = 0; q < RB_STAGE_SIZE; q++)
        if (P->stage[q] >= 0 && P->stage_wait[q]) P->stage_wait[q] = 0;
}

static void main_phase(GameState *g, int pl) {
    RbPlayer *P = &g->p[pl];
    int guard = 0;
    int again = 1;
    while (again && guard++ < 64) {
        again = 0;
        for (int i = 0; i < P->hand.n; i++) {
            Card c;
            if (!rb_decode_card_by_index((uint32_t)P->hand.cards[i], &c)) continue;
            int played = 0;
            if (card_is_member(&c)) {
                if (P->energy_active >= c.cost) {
                    for (int q = 0; q < RB_STAGE_SIZE; q++)
                        if (P->stage[q] < 0) { played = rb_play_member(g, pl, i, q); break; }
                }
            } else if (P->live.n < RB_MAX_LIVE_CARDS) {
                played = rb_play_card(g, pl, i);
            }
            rb_free_card(&c);
            if (rb_has_pending_choice(g)) rb_resume_with_choice(g, -1);
            if (played) { again = 1; break; }
        }
        if (rb_has_pending_choice(g)) rb_resume_with_choice(g, -1);
    }
    /* activate abilities of staged members (one pass) — host would normally poll choice */
    for (int q = 0; q < RB_STAGE_SIZE; q++) {
        int cid = P->stage[q];
        if (cid < 0) continue;
        Card c;
        if (!rb_decode_card_by_index((uint32_t)cid, &c)) continue;
        if (c.ability && c.ability->effect)
            rb_execute_effect(g, pl, c.ability->effect);
        rb_free_card(&c);
    }
}

/* Faithful performance lives in src/live.c:rb_perform_live (yell→stage_hearts
   via RbMods→allocation→verdict→score). Keep a thin alias so old callers
   still compile if live.c is not linked. */
int rb_perform_live(GameState *g, int pl);

static void live_phase(GameState *g) {
    /* Live card set: auto-place up to MAX_LIVE_CARDS - reduction from each player's hand. */
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &g->p[pl];
        int limit = RB_MAX_LIVE_CARDS - g->live_set_limit_reduction[pl];
        if(limit<0) limit=0;
        if(limit>RB_MAX_LIVE_CARDS) limit=RB_MAX_LIVE_CARDS;
        int placed = 0;
        for (int i = 0; i < P->hand.n && placed < limit; ) {
            Card c;
            if (rb_decode_card_by_index((uint32_t)P->hand.cards[i], &c)) {
                if (card_is_live(&c)) {
                    int card = bag_remove_at(&P->hand, i);
                    bag_push(&P->live, card);
                    placed++;
                    rb_free_card(&c);
                    continue;
                }
                rb_free_card(&c);
            }
            i++;
        }
        for (int k = 0; k < placed; k++) rb_draw(g, pl);
    }
    /* Performance: first attacker then second attacker (faithful via live.c). */
    rb_perform_live(g, g->first_attacker);
    rb_perform_live(g, g->second_attacker);
    for(int pl=0;pl<2;pl++) g->live_set_limit_reduction[pl]=0;
}

static void check_victory(GameState *g) {
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &g->p[pl];
        if (P->success.n >= RB_VICTORY_CARD_COUNT) g->winner = pl;
        else if (P->score >= RB_SCORE_WIN) g->winner = pl;
        /* deck-out: no resources left to act */
        int alive = (P->deck.n > 0) || (P->hand.n > 0) ||
                    (P->stage[0] >= 0) || (P->stage[1] >= 0) || (P->stage[2] >= 0) ||
                    (P->live.n > 0) || (P->success.n > 0);
        if (!alive) g->winner = (pl ^ 1);
    }
    if (g->p[0].success.n >= RB_VICTORY_CARD_COUNT && g->p[1].success.n >= RB_VICTORY_CARD_COUNT)
        g->winner = 2; /* draw */
}

static void rollover(GameState *g) {
    check_victory(g);
    if (g->winner != -1) { g->phase = RB_PHASE_DONE; return; }
    g->turn++;
    g->active = g->active ^ 1;
    g->phase = RB_PHASE_ACTIVE;
}

/* One full turn: active player's normal phase + shared live phase + rollover. */
void rb_turn(GameState *g) {
    if (g->winner != -1) return;
    int pl = g->active;
    g->phase = RB_PHASE_ACTIVE;
    activate_wait_members(g, pl);
    g->phase = RB_PHASE_ENERGY;
    rb_draw_energy(g, pl);
    g->phase = RB_PHASE_DRAW;
    rb_draw(g, pl);
    g->phase = RB_PHASE_MAIN;
    main_phase(g, pl);
    g->phase = RB_PHASE_LIVE_SET;
    live_phase(g);
    g->phase = RB_PHASE_VICTORY;
    rollover(g);
}

/* ───────────────────────────── setup ───────────────────────────── */
int rb_game_init(GameState *g, const uint32_t *deck0, int n0,
                 const uint32_t *deck1, int n1) {
    memset(g, 0, sizeof(*g));
    rb_mods_init(&g->mods);
    g->winner = -1; g->turn = 1; g->phase = RB_PHASE_RPS;
    for (int pl = 0; pl < 2; pl++) {
        const uint32_t *d = (pl == 0) ? deck0 : deck1;
        int n = (pl == 0) ? n0 : n1;
        if (n > RB_MAX_DECK) n = RB_MAX_DECK;
        for (int i = 0; i < RB_STAGE_SIZE; i++) g->p[pl].stage[i] = -1;
        for (int i = 0; i < n; i++) g->p[pl].deck.cards[i] = (int)d[i];
        g->p[pl].deck.n = n;
        rb_shuffle(g->p[pl].deck.cards, g->p[pl].deck.n);
        /* opening hand: 6 each (matches Rust's post-RPS draw) */
        for (int k = 0; k < 6; k++) rb_draw(g, pl);
    }
    /* RPS decided deterministically from the seeded RNG. */
    g->rps[0] = rng_range(3);
    g->rps[1] = rng_range(3);
    while (g->rps[0] == g->rps[1]) g->rps[1] = rng_range(3);
    /* 0 rock beats 2 scissors, 1 paper beats 0 rock, 2 scissors beats 1 paper */
    int w = (g->rps[0] == g->rps[1]) ? 0
          : ((g->rps[0] + 1) % 3 == g->rps[1]) ? 1 : 0;
    g->first_attacker = w;
    g->second_attacker = w ^ 1;
    g->active = g->first_attacker;
    g->phase = RB_PHASE_ACTIVE;
    return 0;
}

void rb_print_state(const GameState *g) {
    for (int pl = 0; pl < 2; pl++) {
        const RbPlayer *P = &g->p[pl];
        int onstage = (P->stage[0] >= 0) + (P->stage[1] >= 0) + (P->stage[2] >= 0);
        printf("P%d  energy=%d score=%d hand=%d deck=%d stage=%d live=%d success=%d hearts[pink]=%d\n",
               pl, P->energy_active, P->score, P->hand.n,
               P->deck.n, onstage, P->live.n, P->success.n, P->hearts[RB_HEART_PINK]);
    }
    printf("turn=%d active=%d first=%d winner=%d\n",
           g->turn, g->active, g->first_attacker, g->winner);
}
