#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

/* Look/select pools — mirrors engine/src/ability/look.rs
   For portable core we keep two pools per player:
   looked_at (cards revealed by look_at) and selected (choice result).
   The host's SELECT_CARD choice picks from looked_at; looked_at_remaining
   goes to destination (usually discard or deck). Full keep_shuffle_under
   2-phase lands with the 20-fixture harness. */

#define MAX_LOOKED 64
typedef struct {
    int cards[MAX_LOOKED];
    int n;
    int from_deck;
    int owner;
    char origin[32];
} LookPool;

static LookPool g_look[2];

static void look_pool_reset(LookPool *pool, int owner, const char *origin)
{
    pool->n = 0;
    pool->from_deck = 0;
    pool->owner = owner;
    snprintf(pool->origin, sizeof(pool->origin), "%s",
             origin && *origin ? origin : "deck_top");
}

void rb_look_clear(int pl)
{
    if (pl < 0 || pl >= 2) return;
    look_pool_reset(&g_look[pl], pl, "deck_top");
}

void rb_look_reset_all(void)
{
    rb_look_clear(0);
    rb_look_clear(1);
}

int rb_look_from_deck(int pl)
{
    return pl >= 0 && pl < 2 ? g_look[pl].from_deck : 0;
}

int rb_look_remove(int pl, int cid)
{
    if (pl < 0 || pl >= 2) return 0;
    LookPool *pool = &g_look[pl];
    for (int i = 0; i < pool->n; i++) {
        if (pool->cards[i] != cid) continue;
        for (int k = i; k < pool->n - 1; k++) pool->cards[k] = pool->cards[k + 1];
        pool->n--;
        return 1;
    }
    return 0;
}

void rb_look_add(int pl, int cid)
{
    if (pl < 0 || pl >= 2) return;
    LookPool *pool = &g_look[pl];
    if (pool->n < MAX_LOOKED) pool->cards[pool->n++] = cid;
}

int rb_looked_at_pool(int pl, int *out_ids, int max)
{
    if (pl < 0 || pl >= 2 || max < 0 || (!out_ids && max > 0)) return 0;
    LookPool *pool = &g_look[pl];
    int n = pool->n < max ? pool->n : max;
    for (int i = 0; i < n; i++) out_ids[i] = pool->cards[i];
    return n;
}

static const char *look_extra(const AbilityEffect *effect, const char *key)
{
    if (!effect || !key) return NULL;
    for (int i = 0; i < effect->n_extra; i++)
        if (effect->extra_k[i] && !strcmp(effect->extra_k[i], key))
            return effect->extra_v[i];
    return NULL;
}

static int look_bool(const AbilityEffect *effect, const char *key)
{
    const char *value = look_extra(effect, key);
    return value && (!strcmp(value, "true") || !strcmp(value, "1"));
}

static int look_target_player(const GameState *g, int actor, const AbilityEffect *effect)
{
    int who = actor;
    if (g && actor >= 0 && actor < 2 && g->queue.actor >= 0 && g->queue.actor < 2)
        who = g->queue.actor;
    if (effect && effect->target && !strcmp(effect->target, "opponent")) {
        if (who >= 0 && who < 2) who ^= 1;
    }
    if (who < 0 || who >= 2) who = 0;
    return who;
}

static int look_take_top(RbBag *bag)
{
    if (!bag || bag->n <= 0) return -1;
    int cid = bag->cards[0];
    for (int i = 1; i < bag->n; i++) bag->cards[i - 1] = bag->cards[i];
    bag->n--;
    return cid;
}

static int look_take_bottom(RbBag *bag)
{
    if (!bag || bag->n <= 0) return -1;
    return bag->cards[--bag->n];
}

static void look_append(LookPool *pool, int cid)
{
    if (pool->n < MAX_LOOKED) pool->cards[pool->n++] = cid;
}

static void look_record_revealed(GameState *g, const int *cards, int n)
{
    if (!g) return;
    for (int i = 0; i < n; i++)
        if (g->n_revealed < RB_MAX_RECENTLY_MOVED)
            g->revealed_cards[g->n_revealed++] = cards[i];
}

static int look_at_fetch(GameState *g, int actor, AbilityEffect *effect)
{
    if (!g || !effect) return 0;
    int who = look_target_player(g, actor, effect);
    RbPlayer *player = &g->p[who];
    const char *source = effect->source && *effect->source ? effect->source : "deck";
    int count = rb_effect_count(g, who, -1, effect, g->last_draw_count);
    if (count < 0) count = 0;
    if (look_bool(effect, "all")) count = RB_MAX_ZONE;
    LookPool *pool = &g_look[who];
    int deck_source = !strcmp(source, "deck") || !strcmp(source, "deck_top") ||
                      !strcmp(source, "deck_bottom");
    look_pool_reset(pool, who, source);
    pool->from_deck = deck_source;
    if (!strcmp(source, "deck") || !strcmp(source, "deck_top")) {
        int initial = player->deck.n;
        int take = initial < count ? initial : count;
        for (int i = 0; i < take; i++) look_append(pool, look_take_top(&player->deck));
        if (pool->n < count && player->deck.n == 0 && player->discard.n > 0) {
            rb_player_refresh(g, who);
            int remaining = count - pool->n;
            take = player->deck.n < remaining ? player->deck.n : remaining;
            for (int i = 0; i < take; i++) look_append(pool, look_take_top(&player->deck));
        }
    } else if (!strcmp(source, "deck_bottom")) {
        int take = player->deck.n < count ? player->deck.n : count;
        int taken[MAX_LOOKED];
        for (int i = 0; i < take; i++) taken[i] = look_take_bottom(&player->deck);
        for (int i = take - 1; i >= 0; i--) look_append(pool, taken[i]);
    } else if (!strcmp(source, "hand")) {
        for (int i = 0; i < player->hand.n && i < count; i++) look_append(pool, player->hand.cards[i]);
    } else if (!strcmp(source, "discard") || !strcmp(source, "waitroom")) {
        for (int i = 0; i < player->discard.n && i < count; i++) look_append(pool, player->discard.cards[i]);
    } else if (!strcmp(source, "stage")) {
        for (int i = 0; i < RB_STAGE_SIZE && i < count; i++)
            if (player->stage[i] != RB_EMPTY_SLOT) look_append(pool, player->stage[i]);
    }
    return pool->n;
}

void rb_effect_look_at(GameState *g, int actor, AbilityEffect *e)
{
    int who = look_target_player(g, actor, e);
    int count = look_at_fetch(g, actor, e);
    if (count <= 0) return;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "looked_at", NULL,
                   count, e->is_optional ? 1 : 0, NULL);
    g->queue.resume_look_owner = who;
    rb_queue_pause_for_choice(g, &g->queue.pending);
    g->queue.resume_mode = 2;
    g->queue.resume_eff = e;
    g->queue.resume_is_select = 0;
    g->queue.resume_actor = actor;
    g->queue.resume_host = g->queue.resume_host >= 0 ? g->queue.resume_host : actor;
}

void rb_effect_select_cards(GameState *g, int actor, AbilityEffect *e){
    /* If we have a looked_at pool, the choice is from it; otherwise from hand */
    int who = actor;
    if(e->target && !strcmp(e->target,"opponent")) who=actor^1;
    LookPool *lp=&g_look[who];
    const char *zone = lp->n>0 ? "looked_at" : (e->source ? e->source : "hand");
    const char *ctype=NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"card_type")) ctype=e->extra_v[i];
    int cnt=e->count>=0?e->count:1;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, zone, ctype, cnt, e->is_optional?1:0, NULL);
    rb_queue_pause_for_choice(g, &g->queue.pending);
    /* SelectionContext filter (ability/choice.rs): narrow the valid pool to a
        group and/or heart color so a host UI / test picks a legal card. */
    g->queue.pending.filter_group[0] = 0;
    g->queue.pending.filter_heart = -1;
    int has_heart_color = 0;
    for(int i=0;i<e->n_extra;i++){
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"group_names") && e->extra_v[i])
            strncpy(g->queue.pending.filter_group, e->extra_v[i], sizeof(g->queue.pending.filter_group)-1);
        else if(e->extra_k[i] && !strcmp(e->extra_k[i],"heart_colors") && e->extra_v[i]){
            const char *hc=e->extra_v[i];
            int col=-1;
            if(!strcmp(hc,"pink")||!strcmp(hc,"heart00")) col=0;
            else if(!strcmp(hc,"red")) col=1; else if(!strcmp(hc,"yellow")) col=2;
            else if(!strcmp(hc,"green")) col=3; else if(!strcmp(hc,"blue")) col=4;
            else if(!strcmp(hc,"purple")) col=5; else if(!strcmp(hc,"orange")) col=6;
            else if(!strcmp(hc,"all")) col=7;
            g->queue.pending.filter_heart = col;
        }
        if (e->extra_k[i] && (!strcmp(e->extra_k[i], "heart_color") ||
                              !strcmp(e->extra_k[i], "heart_colors")))
            has_heart_color = 1;
    }
    /* snapshot the filter so rb_look_resume can validate after the pending choice is cleared */
    strncpy(g->queue.resume_filter_group, g->queue.pending.filter_group, sizeof(g->queue.resume_filter_group)-1);
    g->queue.resume_filter_heart = g->queue.pending.filter_heart;
    /* Heart-color selection (mirrors Rust execute_choice → conditional_choice =
        Str(color)). A select with a heart_color extra is a "pick a heart color"
        prompt; stash the chosen color so the following gain_resource applies it.
        Route it through the default resume branch (mode 0) so the parent's later
        siblings (the gain) run after the choice resolves — NOT the card-select
        look/keep path (mode 2). */
    if (has_heart_color) {
        g->queue.selected_heart_color = -1;
        for (int i = 0; i < e->n_extra; i++) {
            if (e->extra_k[i] && (!strcmp(e->extra_k[i], "heart_color") ||
                                  !strcmp(e->extra_k[i], "heart_colors")) && e->extra_v[i]) {
                int col = -1;
                if (!strcmp(e->extra_v[i], "pink") || !strcmp(e->extra_v[i], "heart00")) col = 0;
                else if (!strcmp(e->extra_v[i], "red") || !strcmp(e->extra_v[i], "heart01")) col = 1;
                else if (!strcmp(e->extra_v[i], "yellow") || !strcmp(e->extra_v[i], "heart02")) col = 2;
                else if (!strcmp(e->extra_v[i], "green") || !strcmp(e->extra_v[i], "heart03")) col = 3;
                else if (!strcmp(e->extra_v[i], "blue") || !strcmp(e->extra_v[i], "heart04")) col = 4;
                else if (!strcmp(e->extra_v[i], "purple") || !strcmp(e->extra_v[i], "heart05")) col = 5;
                else if (!strcmp(e->extra_v[i], "orange") || !strcmp(e->extra_v[i], "heart06")) col = 6;
                else if (!strcmp(e->extra_v[i], "all") || !strcmp(e->extra_v[i], "heart07") || !strcmp(e->extra_v[i], "b_all")) col = 7;
                if (col >= 0) g->queue.selected_heart_color = col;
            }
        }
        g->queue.resume_mode = 0; g->queue.resume_is_select = 0;
        g->queue.resume_eff = e; g->queue.resume_actor = actor; g->queue.resume_host = g->queue.resume_host >= 0 ? g->queue.resume_host : actor;
    } else {
        g->queue.resume_mode = 2; g->queue.resume_eff = e; g->queue.resume_is_select = 1;
        g->queue.resume_actor = actor; g->queue.resume_host = g->queue.resume_host >= 0 ? g->queue.resume_host : actor;
    }
}

/* Called when host resumes SELECT_CARD — move chosen card to destination.
   Mirrors engine/src/ability/look.rs keep_shuffle_under: cards that came
   from the deck and were NOT kept are shuffled back into the owner's deck
   (under), not discarded; hand-sourced cards return to hand; the rest go to
   the destination / discard. */
static int look_card_allowed(const GameState *g, int cid)
{
    if (!g) return 0;
    if (g->queue.resume_filter_group[0] &&
        !rb_card_matches_group_str(cid, g->queue.resume_filter_group)) return 0;
    if (g->queue.resume_filter_heart < 0) return 1;
    Card card;
    if (!rb_decode_card_by_index((uint32_t)cid, &card)) return 0;
    int found = 0;
    for (int i = 0; i < card.n_hearts; i++)
        if (card.heart_color[i] == g->queue.resume_filter_heart) found = 1;
    rb_free_card(&card);
    return found;
}

static void look_remove_origin_card(RbPlayer *player, const char *origin, int cid)
{
    if (!player || !origin) return;
    RbBag *bag = NULL;
    if (!strcmp(origin, "hand")) bag = &player->hand;
    else if (!strcmp(origin, "discard") || !strcmp(origin, "waitroom")) bag = &player->discard;
    else if (!strcmp(origin, "live_card_zone") || !strcmp(origin, "live")) bag = &player->live;
    else if (!strcmp(origin, "success_live_zone") || !strcmp(origin, "success")) bag = &player->success;
    if (bag) {
        for (int i = 0; i < bag->n; i++) {
            if (bag->cards[i] != cid) continue;
            for (int k = i; k < bag->n - 1; k++) bag->cards[k] = bag->cards[k + 1];
            bag->n--;
            return;
        }
    } else if (!strcmp(origin, "stage")) {
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            if (player->stage[i] != cid) continue;
            player->stage[i] = RB_EMPTY_SLOT;
            player->stage_wait[i] = 0;
            return;
        }
    }
}

static void look_append_cards(RbBag *bag, const int *cards, int n)
{
    if (!bag) return;
    for (int i = 0; i < n && bag->n < RB_MAX_ZONE; i++) bag->cards[bag->n++] = cards[i];
}

static void look_prepend_cards(RbBag *bag, const int *cards, int n)
{
    if (!bag || n <= 0) return;
    int count = n;
    if (count > RB_MAX_ZONE - bag->n) count = RB_MAX_ZONE - bag->n;
    for (int i = bag->n - 1; i >= 0; i--) bag->cards[i + count] = bag->cards[i];
    for (int i = 0; i < count; i++) bag->cards[i] = cards[i];
    bag->n += count;
}

static void look_place_stage(RbPlayer *player, const int *cards, int n)
{
    if (!player) return;
    for (int i = 0; i < n; i++) {
        int slot = -1;
        for (int j = 0; j < RB_STAGE_SIZE; j++)
            if (player->stage[j] == RB_EMPTY_SLOT) { slot = j; break; }
        if (slot < 0) {
            if (player->discard.n < RB_MAX_ZONE)
                player->discard.cards[player->discard.n++] = cards[i];
            continue;
        }
        player->stage[slot] = cards[i];
        player->stage_wait[slot] = 0;
    }
}

void rb_look_resume_indices(GameState *g, int owner, const int *indices, int n_indices,
                            const char *destination, const char *remainder_destination,
                            int discard_remaining, int is_select)
{
    if (!g || owner < 0 || owner >= 2) return;
    LookPool *pool = &g_look[owner];
    RbPlayer *player = &g->p[owner];
    int selected_flags[MAX_LOOKED] = {0};
    int selected[MAX_LOOKED];
    int n_selected = 0;
    for (int i = 0; i < n_indices; i++) {
        int index = indices ? indices[i] : -1;
        if (index < 0 || index >= pool->n || selected_flags[index]) continue;
        int cid = pool->cards[index];
        if (is_select && !look_card_allowed(g, cid)) continue;
        selected_flags[index] = 1;
        selected[n_selected++] = cid;
    }
    int remainder[MAX_LOOKED];
    int n_remainder = 0;
    for (int i = 0; i < pool->n; i++)
        if (!selected_flags[i]) remainder[n_remainder++] = pool->cards[i];
    if (!pool->from_deck) {
        for (int i = 0; i < n_selected; i++) look_remove_origin_card(player, pool->origin, selected[i]);
        for (int i = 0; i < n_remainder; i++) look_remove_origin_card(player, pool->origin, remainder[i]);
    }
    if (n_selected == 0 && discard_remaining < 0 &&
        (!remainder_destination || !*remainder_destination)) {
        if (!strcmp(pool->origin, "deck_bottom")) {
            look_append_cards(&player->deck, remainder, n_remainder);
        } else if (!strcmp(pool->origin, "deck") || !strcmp(pool->origin, "deck_top")) {
            int ordered[MAX_LOOKED];
            for (int i = 0; i < n_remainder; i++) ordered[i] = remainder[n_remainder - 1 - i];
            look_prepend_cards(&player->deck, ordered, n_remainder);
        } else if (!strcmp(pool->origin, "hand")) {
            look_append_cards(&player->hand, remainder, n_remainder);
        } else if (!strcmp(pool->origin, "discard") || !strcmp(pool->origin, "waitroom")) {
            look_append_cards(&player->discard, remainder, n_remainder);
        } else {
            look_append_cards(&player->discard, remainder, n_remainder);
        }
        pool->n = 0;
        return;
    }
    const char *destination_zone = destination && *destination ? destination : "hand";
    if (!strcmp(destination_zone, "deck") || !strcmp(destination_zone, "deck_top")) {
        int ordered[MAX_LOOKED];
        for (int i = 0; i < n_selected; i++) ordered[i] = selected[n_selected - 1 - i];
        look_prepend_cards(&player->deck, ordered, n_selected);
    } else if (!strcmp(destination_zone, "deck_bottom")) {
        look_append_cards(&player->deck, selected, n_selected);
    } else if (!strcmp(destination_zone, "discard") || !strcmp(destination_zone, "waitroom")) {
        look_append_cards(&player->discard, selected, n_selected);
    } else if (!strcmp(destination_zone, "stage")) {
        look_place_stage(player, selected, n_selected);
    } else {
        look_append_cards(&player->hand, selected, n_selected);
    }
    const char *remainder_zone = remainder_destination && *remainder_destination
                                     ? remainder_destination
                                     : (discard_remaining != 0 ? "discard" : "deck_top");
    if (!strcmp(remainder_zone, "looked_at")) {
        pool->n = 0;
        for (int i = 0; i < n_remainder; i++) look_append(pool, remainder[i]);
        return;
    }
    if (!strcmp(remainder_zone, "deck") || !strcmp(remainder_zone, "deck_top")) {
        int ordered[MAX_LOOKED];
        for (int i = 0; i < n_remainder; i++) ordered[i] = remainder[n_remainder - 1 - i];
        look_prepend_cards(&player->deck, ordered, n_remainder);
    } else if (!strcmp(remainder_zone, "deck_bottom")) {
        look_append_cards(&player->deck, remainder, n_remainder);
    } else if (!strcmp(remainder_zone, "hand")) {
        look_append_cards(&player->hand, remainder, n_remainder);
    } else if (!strcmp(remainder_zone, "stage")) {
        look_place_stage(player, remainder, n_remainder);
    } else {
        look_append_cards(&player->discard, remainder, n_remainder);
    }
    if (is_select) {
        for (int i = 0; i < n_selected; i++) {
            if (g->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                g->selected_cards[g->n_selected_cards++] = selected[i];
            if (g->n_recently_moved < RB_MAX_RECENTLY_MOVED)
                g->recently_moved[g->n_recently_moved++] = selected[i];
        }
    }
    for (int i = 0; i < n_remainder; i++)
        if (g->n_recently_moved < RB_MAX_RECENTLY_MOVED)
            g->recently_moved[g->n_recently_moved++] = remainder[i];
    pool->n = 0;
}

void rb_look_resume(GameState *g, int actor, int selected_idx, const char *destination, int is_select)
{
    int owner = g && g->queue.resume_look_owner >= 0 && g->queue.resume_look_owner < 2
                    ? g->queue.resume_look_owner
                    : (actor >= 0 && actor < 2 ? actor : 0);
    int discard_remaining = -1;
    const char *remainder = NULL;
    if (g && g->queue.resume_eff) {
        const char *value = look_extra(g->queue.resume_eff, "discard_remaining");
        if (value) discard_remaining = !strcmp(value, "true") || !strcmp(value, "1");
        remainder = look_extra(g->queue.resume_eff, "remainder_destination");
    }
    int index = selected_idx;
    rb_look_resume_indices(g, owner, &index, selected_idx < 0 ? 0 : 1,
                           destination, remainder, discard_remaining, is_select);
}

/* Reveal cards from the target's deck top into the looked_at pool until
   `pred` returns true (or the deck empties). Mirrors engine/src/ability/look.rs
   reveal_until — the matched card and everything revealed above it become the
   looked_at pool (gs.looked_at_cards = all_revealed). */
static int reveal_until(GameState *g, int who, int (*pred)(const GameState*, int cid))
{
    RbPlayer *player = &g->p[who];
    LookPool *pool = &g_look[who];
    look_pool_reset(pool, who, "deck_top");
    pool->from_deck = 1;
    int refreshed = 0;
    while (pool->n < MAX_LOOKED) {
        if (player->deck.n == 0) {
            if (refreshed || player->discard.n == 0) break;
            rb_player_refresh(g, who);
            refreshed = 1;
            if (player->deck.n == 0) break;
        }
        int cid = look_take_top(&player->deck);
        look_append(pool, cid);
        look_record_revealed(g, &cid, 1);
        if (pred(g, cid)) return 1;
    }
    return 0;
}

static int card_is_live_pred(const GameState *g, int cid){
    Card c; if(!rb_decode_card_by_index((uint32_t)cid,&c)) return 0;
    int r = (c.type_flags & 0x3) == 1; rb_free_card(&c); return r;
}

/* Adapter so card_matches_card_type_filter (int,int) can be used as a
   reveal_until predicate (const GameState*,int). */
static const char *g_reveal_ctype;
static int g_reveal_cost_limit = -1;
static const char *g_reveal_cost_op;
static int card_type_pred(const GameState *g, int cid){
    (void)g;
    return card_matches_card_type_filter(cid, g_reveal_ctype);
}

/* Mirror look.rs::execute_reveal_until_target — reveal from the deck until a card
    matching card_type (and, for member_card, cost_limit op) is found. The predicate
    layers the optional cost gate on top of the card_type gate. */
static int card_type_cost_pred(const GameState *g, int cid){
    (void)g;
    if (!g_reveal_ctype) return 0;
    if (!card_matches_card_type_filter(cid, g_reveal_ctype)) return 0;
    /* cost_limit only applies when the selected card_type is member_card. */
    if (g_reveal_cost_limit >= 0 && !strcmp(g_reveal_ctype, "member_card"))
        if (!rb_card_matches_cost_limit(cid, g_reveal_cost_limit, g_reveal_cost_op)) return 0;
    return 1;
}

void rb_effect_reveal_until_live_card(GameState *g, int actor, AbilityEffect *e)
{
    int who = look_target_player(g, actor, e);
    reveal_until(g, who, card_is_live_pred);
}

void rb_effect_reveal_until_chosen_card(GameState *g, int actor, AbilityEffect *e)
{
    int who = look_target_player(g, actor, e);
    const char *card_type = look_extra(e, "card_type");
    g_reveal_ctype = card_type ? card_type : "live_card";
    reveal_until(g, who, card_type ? card_type_pred : card_is_live_pred);
}

/* Mirror look.rs::execute_reveal_until_target — reveal from the deck until a card
    matching card_type (and the member_card cost_limit gate) is found. On a match
    the matched card is moved to the FRONT of the looked_at pool; on no match the
    pool is cleared, mirroring Rust's matched_idx reordering. */
void rb_effect_reveal_until_target(GameState *g, int actor, AbilityEffect *e)
{
    int who = look_target_player(g, actor, e);
    const char *card_type = look_extra(e, "card_type");
    const char *cost_limit = look_extra(e, "cost_limit");
    const char *cost_operator = look_extra(e, "cost_limit_operator");
    g_reveal_ctype = card_type ? card_type : "live_card";
    g_reveal_cost_limit = cost_limit ? atoi(cost_limit) : -1;
    g_reveal_cost_op = cost_operator;
    LookPool *pool = &g_look[who];
    int matched = reveal_until(g, who, card_type_cost_pred);
    if (matched && pool->n > 1) {
        int matched_card = pool->cards[pool->n - 1];
        for (int i = pool->n - 1; i > 0; i--) pool->cards[i] = pool->cards[i - 1];
        pool->cards[0] = matched_card;
    } else if (!matched) {
        pool->n = 0;
    }
}

/* -- execute_reveal_per_group -- */
void rb_effect_reveal_per_group(GameState *g, int actor, AbilityEffect *e) {
    int who = look_target_player(g, actor, e);
    int count = rb_effect_count(g, who, -1, e, g->last_draw_count);
    if (count < 0) count = 0;
    RbPlayer *player = &g->p[who];
    const char *source = e->source && *e->source ? e->source : "hand";
    int card_ids[RB_MAX_ZONE];
    int n_cards = 0;
    if (!strcmp(source, "hand")) {
        for (int i = 0; i < player->hand.n && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = player->hand.cards[i];
    } else if (!strcmp(source, "deck") || !strcmp(source, "deck_top")) {
        int take = count < player->deck.n ? count : player->deck.n;
        for (int i = 0; i < take && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = player->deck.cards[i];
    } else if (!strcmp(source, "discard") || !strcmp(source, "waitroom")) {
        for (int i = 0; i < player->discard.n && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = player->discard.cards[i];
    } else if (!strcmp(source, "looked_at")) {
        LookPool *pool = &g_look[who];
        for (int i = 0; i < pool->n && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = pool->cards[i];
    }
    look_record_revealed(g, card_ids, n_cards);
}

/* -- execute_reveal -- */
void rb_effect_reveal(GameState *g, int actor, AbilityEffect *e) {
    int who = look_target_player(g, actor, e);
    int count = rb_effect_count(g, who, -1, e, g->last_draw_count);
    if (count < 0) count = 0;
    RbPlayer *player = &g->p[who];
    const char *source = e->source && *e->source ? e->source : "hand";
    int any_number = look_bool(e, "any_number");
    int is_max = look_bool(e, "max");
    int is_optional = e->is_optional ? 1 : 0;
    int available = 0;
    if (!strcmp(source, "hand")) available = player->hand.n;
    else if (!strcmp(source, "looked_at")) available = g_look[who].n;
    else if (!strcmp(source, "deck") || !strcmp(source, "deck_top")) available = player->deck.n;
    if ((!strcmp(source, "hand") || !strcmp(source, "looked_at")) && available > 0 &&
        (is_max || is_optional || count == 0 || count < available)) {
        const char *card_type = e->card_type_field[0] ? e->card_type_field : look_extra(e, "card_type");
        rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, source, card_type,
                       any_number ? available : count,
                       any_number || is_optional || is_max, NULL);
    g->queue.resume_look_owner = who;
    rb_queue_pause_for_choice(g, &g->queue.pending);
        g->queue.resume_mode = 0;
        g->queue.resume_eff = e;
        g->queue.resume_is_select = 0;
        g->queue.resume_actor = actor;
        g->queue.resume_host = g->queue.resume_host >= 0 ? g->queue.resume_host : actor;
        return;
    }
    int card_ids[RB_MAX_ZONE];
    int n_cards = 0;
    if (!strcmp(source, "hand")) {
        for (int i = 0; i < player->hand.n && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = player->hand.cards[i];
    } else if (!strcmp(source, "deck") || !strcmp(source, "deck_top")) {
        int take = count < player->deck.n ? count : player->deck.n;
        for (int i = 0; i < take && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = player->deck.cards[i];
    } else if (!strcmp(source, "looked_at")) {
        LookPool *pool = &g_look[who];
        for (int i = 0; i < pool->n && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = pool->cards[i];
    }
    if (!look_bool(e, "blind")) look_record_revealed(g, card_ids, n_cards);
}

/* -- execute_select -- */
void rb_effect_select(GameState *g, int actor, AbilityEffect *e) {
    int who = actor;
    if (e->target && !strcmp(e->target, "opponent")) who = actor ^ 1;
    RbPlayer *P = &g->p[who];
    const char *source = e->source ? e->source : "hand";
    int count = e->count >= 0 ? e->count : 1;
    int is_optional = e->is_optional ? 1 : 0;
    int card_ids[RB_MAX_ZONE];
    int n_cards = 0;
    if (!strcmp(source, "hand")) {
        for (int i = 0; i < P->hand.n && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = P->hand.cards[i];
    } else if (!strcmp(source, "deck")) {
        int take = count < P->deck.n ? count : P->deck.n;
        for (int i = 0; i < take && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = P->deck.cards[P->deck.n - 1 - i];
    } else if (!strcmp(source, "discard") || !strcmp(source, "waitroom")) {
        for (int i = 0; i < P->discard.n && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = P->discard.cards[i];
    } else if (!strcmp(source, "stage")) {
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] != RB_EMPTY_SLOT && n_cards < RB_MAX_ZONE)
                card_ids[n_cards++] = P->stage[i];
    } else if (!strcmp(source, "looked_at")) {
        LookPool *lp = &g_look[who];
        for (int i = 0; i < lp->n && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = lp->cards[i];
    } else if (!strcmp(source, "selected_cards")) {
        for (int i = 0; i < g->n_selected_cards && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = g->selected_cards[i];
    }
    LookPool *lp = &g_look[who];
    lp->n = 0; lp->from_deck = 0; lp->owner = who;
    for (int i = 0; i < n_cards && lp->n < MAX_LOOKED; i++)
        lp->cards[lp->n++] = card_ids[i];
    if (count == 0 || lp->n == 0) return;
    if (count > lp->n) count = lp->n;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, source, NULL, count, is_optional, NULL);
    rb_queue_pause_for_choice(g, &g->queue.pending);
    g->queue.resume_mode = 2; g->queue.resume_eff = e;
    g->queue.resume_is_select = 1;
    g->queue.resume_actor = actor; g->queue.resume_host = g->queue.resume_host >= 0 ? g->queue.resume_host : actor;
}

/* -- execute_look_and_select -- */
static const char *look_filter_card_type(const AbilityEffect *effect)
{
    if (!effect) return NULL;
    if (effect->card_type_field[0]) return effect->card_type_field;
    return look_extra(effect, "card_type");
}

static int look_filter_matches(GameState *g, const AbilityEffect *effect, int cid)
{
    if (!effect) return 1;
    const char *card_type = look_filter_card_type(effect);
    if (card_type && *card_type && !rb_card_matches_type(cid, card_type)) return 0;
    const char *group = look_extra(effect, "group_names");
    if (!group) group = look_extra(effect, "group");
    if (group && *group && !rb_card_matches_group_str(cid, group)) return 0;
    const char *cost_limit = look_extra(effect, "cost_limit");
    if (cost_limit) {
        const char *cost_operator = look_extra(effect, "cost_limit_operator");
        if (!cost_operator) cost_operator = look_extra(effect, "operation");
        if (!rb_card_matches_cost_limit(cid, atoi(cost_limit), cost_operator)) return 0;
    }
    const char *heart = look_extra(effect, "heart_colors");
    if (!heart) heart = look_extra(effect, "heart_color");
    if (heart && *heart) {
        const char *colors[8];
        int n_colors = 0;
        char buffer[192];
        snprintf(buffer, sizeof(buffer), "%s", heart);
        char *token = strtok(buffer, ",");
        while (token && n_colors < 8) {
            colors[n_colors++] = token;
            token = strtok(NULL, ",");
        }
        int require_all = look_bool(effect, "require_all_heart_colors");
        if (require_all ? !rb_card_matches_all_heart_colors(cid, colors, n_colors)
                        : !rb_card_matches_heart_colors(cid, colors, n_colors)) return 0;
    }
    const char *characters = look_extra(effect, "characters");
    if (characters && *characters) {
        const char *names[8];
        int n_names = 0;
        char buffer[256];
        snprintf(buffer, sizeof(buffer), "%s", characters);
        char *token = strtok(buffer, ",");
        while (token && n_names < 8) {
            names[n_names++] = token;
            token = strtok(NULL, ",");
        }
        if (!rb_card_matches_characters(cid, names, n_names)) return 0;
    }
    if (look_bool(effect, "exclude_self") && g && g->activating_card >= 0 && cid == g->activating_card)
        return 0;
    return 1;
}

static int look_select_matches(GameState *g, const AbilityEffect *select, int cid)
{
    if (!select) return 1;
    if (select->n_options > 0) {
        for (int i = 0; i < select->n_options; i++)
            if (look_filter_matches(g, select->options[i], cid)) return 1;
        return 0;
    }
    return look_filter_matches(g, select, cid);
}

static void look_discard_unmatched(GameState *g, int owner, AbilityEffect *parent)
{
    if (!g || owner < 0 || owner >= 2) return;
    LookPool *pool = &g_look[owner];
    RbPlayer *player = &g->p[owner];
    for (int i = 0; i < pool->n; i++)
        if (player->discard.n < RB_MAX_ZONE)
            player->discard.cards[player->discard.n++] = pool->cards[i];
    pool->n = 0;
    if (parent && parent->followup_action)
        rb_execute_effect_ex(g, owner, parent->followup_action,
                             g->queue.resume_host >= 0 ? g->queue.resume_host : -1);
}

void rb_effect_look_and_select(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;
    AbilityEffect *look_action = e->look_action;
    if (!look_action && e->n_child > 0) look_action = e->child[0];
    AbilityEffect *select_action = e->select_action;
    if (!select_action && e->n_child > 1) select_action = e->child[1];
    int who = look_target_player(g, actor, look_action ? look_action : e);
    if (look_action) {
        if (look_action->action && !strcmp(look_action->action, "look_at")) {
            look_at_fetch(g, actor, look_action);
            who = look_target_player(g, actor, look_action);
        } else {
            rb_execute_effect_ex(g, actor, look_action, g->activating_card);
            if (rb_has_pending_choice(g)) return;
            who = look_action->target && !strcmp(look_action->target, "opponent")
                      ? actor ^ 1
                      : actor;
        }
    }
    if (!select_action) return;
    LookPool *pool = &g_look[who];
    int filtered[MAX_LOOKED];
    int n_filtered = 0;
    for (int i = 0; i < pool->n; i++)
        if (look_select_matches(g, select_action, pool->cards[i]))
            filtered[n_filtered++] = i;
    if (n_filtered == 0) {
        look_discard_unmatched(g, who, e);
        return;
    }
    int count = select_action->count >= 0 ? select_action->count : 1;
    int any_number = look_bool(select_action, "any_number");
    int is_max = look_bool(select_action, "max");
    int is_optional = select_action->is_optional;
    int max_select = any_number ? n_filtered : (count < n_filtered ? count : n_filtered);
    const char *card_type = look_filter_card_type(select_action);
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "looked_at", card_type,
                   max_select, is_optional || is_max || any_number, NULL);
    g->queue.pending.n_filtered_indices = n_filtered;
    for (int i = 0; i < n_filtered; i++)
        g->queue.pending.filtered_indices[i] = filtered[i];
    const char *group = look_extra(select_action, "group_names");
    if (!group) group = look_extra(select_action, "group");
    g->queue.pending.filter_group[0] = 0;
    g->queue.resume_filter_group[0] = 0;
    if (group) {
        snprintf(g->queue.pending.filter_group, sizeof(g->queue.pending.filter_group), "%s", group);
        snprintf(g->queue.resume_filter_group, sizeof(g->queue.resume_filter_group), "%s", group);
    }
    const char *heart = look_extra(select_action, "heart_colors");
    if (!heart) heart = look_extra(select_action, "heart_color");
    int heart_color = heart && *heart ? rb_parse_heart_color(heart) : -1;
    g->queue.pending.filter_heart = heart_color;
    g->queue.resume_filter_heart = heart_color;
    const char *cost_limit = look_extra(select_action, "cost_limit");
    if (cost_limit) {
        g->queue.pending.cost_limit = atoi(cost_limit);
        const char *operator_text = look_extra(select_action, "cost_limit_operator");
        if (!operator_text) operator_text = look_extra(select_action, "operation");
        if (operator_text)
            snprintf(g->queue.pending.cost_limit_op,
                     sizeof(g->queue.pending.cost_limit_op), "%s", operator_text);
    }
    g->queue.pending.is_reveal = look_bool(select_action, "reveal");
    snprintf(g->queue.pending.target_player_id,
             sizeof(g->queue.pending.target_player_id), "p%d", who + 1);
    g->queue.resume_look_owner = who;
    g->queue.resume_after_look = e->followup_action;
    rb_queue_pause_for_choice(g, &g->queue.pending);
    g->queue.resume_mode = 2;
    g->queue.resume_eff = select_action;
    g->queue.resume_is_select = 1;
    g->queue.resume_actor = actor;
    g->queue.resume_host = g->queue.resume_host >= 0 ? g->queue.resume_host : -1;
}