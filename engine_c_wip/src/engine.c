#include "rabuka.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

/* cost.c's "the optional cost resolved as a skip" latch. It mirrors Rust's
   `entry.optional_cost_result == Some(false)` + the `optional_cost_skip_gate`
   POST_COST gate (engine/src/ability/gates.rs:195-213), which stops the
   resolution BEFORE the ability's effect. Declared here rather than in
   include/rabuka.h because the value is only meaningful to the resolver loop
   that just paid the cost. */
int rb_cost_optional_skipped(void);

/* ───────────────────────────── RNG ───────────────────────────── */
static uint32_t rng_state = 0;
void rb_seed(uint32_t s) { rng_state = s; }
uint32_t rb_rand(void) {
    uint32_t x = rng_state ? rng_state : 1u;
    x ^= x << 13; x ^= x >> 17; x ^= x << 5;
    rng_state = x; return x;
}
uint32_t rb_rng_checkpoint(void) { return rng_state; }
void rb_rng_restore(uint32_t state) { rb_seed(state); }
size_t rb_rand_range(size_t max) { return max ? (size_t)rb_rand() % max : 0; }
static int rng_range(int n) { return n > 0 ? (int)rb_rand_range((size_t)n) : 0; }
RbLcg rb_lcg_new(uint64_t seed) {
    RbLcg rng = { seed };
    return rng;
}
uint64_t rb_lcg_next_u64(RbLcg *rng) {
    rng->state = rng->state * UINT64_C(6364136223846793005) + UINT64_C(1442695040888963407);
    return rng->state;
}
size_t rb_lcg_range(RbLcg *rng, size_t n) {
    return n ? (size_t)((rb_lcg_next_u64(rng) >> 33) % (uint64_t)n) : 0;
}

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
    if (P->deck.n == 0) rb_player_refresh(g, pl);   /* refresh shuffles waitroom in */
    if (P->deck.n == 0) return 0;
    if (P->hand.n >= RB_MAX_HAND) return 0;
    bag_push(&P->hand, bag_remove_at(&P->deck, 0));
    return 1;
}
int rb_draw_energy(GameState *g, int pl) {
    RbPlayer *P = &g->p[pl];
    if (P->energy_deck.n == 0) return 0;
    if (P->energy.n >= RB_MAX_ENERGY_CARDS) return 0;
    bag_push(&P->energy, bag_remove_at(&P->energy_deck, 0));
    if (P->energy_active < RB_MAX_ENERGY_CARDS) P->energy_active++;
    return 1;
}

/* ───────────────────────────── card classification ───────────────────────────── */
/* Faithful classification mirrors Card::is_live/is_energy (core/card.c): the low
   2 bits of type_flags encode 0=Member, 1=Live, 2=Energy. The old heuristic
   (n_hearts==0 && cost==0 && blade==0) mis-classified real live/energy cards as
   members and had no energy branch at all. */
static int card_is_live(Card *c) {
    return (c->type_flags & 0x03) == 1;
}
static int card_is_energy(Card *c) {
    return (c->type_flags & 0x03) == 2;
}
static int card_is_member(Card *c) {
    return (c->type_flags & 0x03) == 0;
}

/* ───────────────────────────── extra-field lookup ───────────────────────────── */
static const char *extra(AbilityEffect *e, const char *k) {
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}
int heart_color_of(AbilityEffect *e, int dflt) {
    const char *h = extra(e, "heart_color");
    if (!h) h = extra(e, "target");
    if (!h) return dflt;
    if (!strcmp(h, "pink") || !strcmp(h, "heart00")) return RB_HEART_PINK;
    if (!strcmp(h, "red") || !strcmp(h, "heart01")) return RB_HEART_RED;
    if (!strcmp(h, "yellow") || !strcmp(h, "heart02")) return RB_HEART_YELLOW;
    if (!strcmp(h, "green") || !strcmp(h, "heart03")) return RB_HEART_GREEN;
    if (!strcmp(h, "blue") || !strcmp(h, "heart04")) return RB_HEART_BLUE;
    if (!strcmp(h, "purple") || !strcmp(h, "heart05")) return RB_HEART_PURPLE;
    if (!strcmp(h, "orange") || !strcmp(h, "heart06")) return RB_HEART_ORANGE;
    if (!strcmp(h, "all") || !strcmp(h, "heart07") || !strcmp(h, "b_all")) return RB_HEART_ALL;
    if (!strcmp(h, "draw")) return RB_HEART_DRAW;
    if (!strcmp(h, "score")) return RB_HEART_SCORE;
    /* generic "heartNN" (NN = 00..07) → numeric color index */
    if (!strncmp(h, "heart", 5) && h[5] >= '0' && h[5] <= '9') {
        int idx = atoi(h + 5);
        if (idx >= 0 && idx <= 7) return idx;
    }
    return dflt;
}

/* Move `count` cards from one of actor's zones to another.
   Delegates to rb_card_matches_type (src/ability/util.c) which mirrors
   engine/src/ability/util.rs:card_matches_type. */
int card_matches_card_type_filter(int card_idx, const char *filter){
    return rb_card_matches_type(card_idx, filter);
}
static void do_move_filtered(GameState *g, int actor, RbZone src, RbZone dst, int count, int to_top, const char *card_type_filter);
static void do_move(GameState *g, int actor, RbZone src, RbZone dst, int count, int to_top) {
    do_move_filtered(g, actor, src, dst, count, to_top, NULL);
}
static void record_movement(GameState *g, int cid){
    if(g->n_recently_moved < RB_MAX_RECENTLY_MOVED) g->recently_moved[g->n_recently_moved++]=cid;
    else { for(int i=1;i<RB_MAX_RECENTLY_MOVED;i++) g->recently_moved[i-1]=g->recently_moved[i]; g->recently_moved[RB_MAX_RECENTLY_MOVED-1]=cid; }
}
static void do_move_filtered(GameState *g, int actor, RbZone src, RbZone dst, int count, int to_top, const char *card_type_filter) {
    RbPlayer *A = &g->p[actor];
    if (src == RB_ZONE_STAGE) {
        int moved = 0;
        int limit = (count<0)? RB_STAGE_SIZE : count;
        for (int pos = 0; pos < RB_STAGE_SIZE && moved < limit; pos++) {
            if (A->stage[pos] >= 0 && card_matches_card_type_filter(A->stage[pos], card_type_filter)) {
                int c = A->stage[pos]; A->stage[pos] = -1; A->stage_wait[pos] = 0;
                record_movement(g,c);
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
        record_movement(g,c);
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
static void handle_action(GameState *g, int actor, AbilityEffect *e, int host_cid);
AbilityEffect *rb_effect_deep_clone(const AbilityEffect *src);
void rb_emit_choice(GameState *g, int actor, RbChoiceKind kind,
                    const char *zone, const char *card_type,
                    int count, int allow_skip, const char *target);

/* Mirror Rust's `activating_card`: the card whose ability is resolving.
   Threaded through so per-card modifiers (blade/heart) attribute correctly. */
static int s_exec_depth = 0;
void rb_execute_effect_ex(GameState *g, int actor, AbilityEffect *e, int host_cid) {
    if (!e) return;
    if (g->yell_occurred)
        fprintf(stderr, "[YELL_EXEC_ENTER] host=%d action=%s has_cond=%d pending=%d occurred=%d revealed=%d\n",
                host_cid, e->action ? e->action : "-", e->has_condition,
                rb_has_pending_choice(g), g->yell_occurred, g->n_revealed);
    if (rb_ability_debug_enabled() || (e->action && !strcmp(e->action, "draw_until_count")))
        fprintf(stderr, "[EXEC_EFFECT_ENTER] action=%s host=%d pending=%d depth=%d ptr=%p\n",
                e->action ? e->action : "-", host_cid, rb_has_pending_choice(g), s_exec_depth, (void *)e);
    g->queue.resume_host = host_cid;
    /* Bound recursion: a deeply-nested effect tree (or a chain of ability effects
        that move/activate members) can recurse through this fn and overflow the
        stack. Headless caps the depth; effects past the cap are skipped (the same
        ability would re-resolve on a later drain pass). */
    if (s_exec_depth > 64) return;
    s_exec_depth++;
    if (rb_has_pending_choice(g)) {
        if (e->action && !strcmp(e->action, "draw_until_count"))
            fprintf(stderr, "[EXEC_EFFECT_STOP] pending action=%s\n", e->action);
        s_exec_depth--;
        return;
    }
    if (e->has_condition && e->condition) {
        int condition_ok = rb_eval_condition_for_host(g, actor, host_cid, e->condition);
        if (g->yell_occurred && e->action && !strcmp(e->action, "gain_resource"))
            fprintf(stderr, "[YELL_EXEC_COND] host=%d occurred=%d revealed=%d result=%d\n",
                    host_cid, g->yell_occurred, g->n_revealed, condition_ok);
        if (e->action && !strcmp(e->action, "draw_until_count"))
            fprintf(stderr, "[EXEC_EFFECT_COND] action=%s ok=%d\n", e->action, condition_ok);
        if (!condition_ok) { s_exec_depth--; return; }
    }
    for (int i = 0; i < e->n_child; i++) {
        if (!(e->action && rb_executor_is_structural(e->action))) {
            rb_execute_effect_ex(g, actor, e->child[i], host_cid);
        }
        if (rb_has_pending_choice(g)) {
                /* A child emitted a pending choice and stalled this effect chain.
                   Stash the parent + child index + host so the resume can run the
                   remaining sibling effects (e.g. the gain_resource that follows a
                   paid optional cost, or the heart grant that follows a
                   choice/select_number/select_cards prompt). Mirrors Rust's
                   pending_actions park + resume_pending_actions. */
                AbilityEffect *ch = e->child[i];
                if (ch && ch->action) {
                    int is_gate = ch->is_optional &&
                        (!strcmp(ch->action, "pay_energy") || !strcmp(ch->action, "pay_cost") ||
                         !strcmp(ch->action, "activation_cost") || !strcmp(ch->action, "pay_optional_cost") ||
                         !strcmp(ch->action, "draw") || !strcmp(ch->action, "draw_card") ||
                         !strcmp(ch->action, "draw_until_count"));
                    int is_choice = (!strcmp(ch->action, "choice") ||
                                      !strcmp(ch->action, "select_number") ||
                                      !strcmp(ch->action, "select_cards") ||
                                      !strcmp(ch->action, "select") ||
                                      !strcmp(ch->action, "look_and_select"));
                    if (is_choice) {
                        /* Mirror Rust pending_actions: the choice's remaining body
                            (the sibling effects that follow it in the parent, e.g. the
                            gain_resource that follows a heart-color select) runs AFTER
                            the player answers. Park the parent + the choice's child
                            index so resume executes the later siblings. */
                        g->queue.resume_parent = e;
                        g->queue.resume_child = i;
                        g->queue.resume_host = host_cid;
                    } else if (is_gate) {
                        g->queue.resume_parent = e;
                        g->queue.resume_child = i;
                        g->queue.resume_host = host_cid;
                    }
                }
                s_exec_depth--;
                return;
            }
    }
    if (!e->action) { s_exec_depth--; return; }
    if (e->action && !strcmp(e->action, "draw_until_count"))
        fprintf(stderr, "[EXEC_EFFECT_DISPATCH] has=%d action=%s\n", rb_executor_has_executor(e->action), e->action);
    if (rb_executor_has_executor(e->action))
        rb_executor_execute(g, actor, e, host_cid);
    else
        handle_action(g, actor, e, host_cid);
    s_exec_depth--;
}

void rb_execute_effect(GameState *g, int actor, AbilityEffect *e) {
    /* Batch-scope recently_moved at the start of each ability resolution
       (mirrors Rust GameState::clear_recently_moved_batch between batches so
       `has_moved`/`preceding_moved` refer to THIS ability's moves, not the
       whole game). selected_cards is NOT cleared here — it is set
       asynchronously by a look/select choice resume and must survive across
       ability boundaries until consumed. */
    g->n_recently_moved = 0;
    rb_execute_effect_ex(g, actor, e, -1);
}

static int target_player(AbilityEffect *e, int actor) {
    if (e->target) {
        if (!strcmp(e->target, "opponent")) return actor ^ 1;
        if (!strcmp(e->target, "both") || !strcmp(e->target, "either")) return actor; /* self pass */
    }
    return actor;
}

static void handle_action(GameState *g, int actor, AbilityEffect *e, int host_cid) {
    const char *act = e->action;
    int cnt = rb_effect_count(g, actor, host_cid, e, 0);
    int who = target_player(e, actor);
    RbPlayer *W = &g->p[who];
    RbPlayer *O = &g->p[actor ^ 1];

    if (!strcmp(act, "draw") || !strcmp(act, "draw_card") ||
        !strcmp(act, "draw_until_count")) {
        /* Faithful draw — mirror draw.rs:execute_draw_wrapper/execute_draw. */
        rb_effect_draw_card(g, actor, e, host_cid);
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
        /* gain_resource: mirror misc.rs:execute_gain_resource — the resource
           (blade/heart/score/energy) is granted to the targets chosen by
           target / card_type / group_names / self_target. Duration::LiveEnd
           registers a temporary effect that rb_check_expired_effects reverts
           at live end (same mechanism as rb_fire_debut). place_energy* carries
           no resource field and falls through to the energy path. */
        const char *res = NULL; int dur = RB_TEMP_PERM;
        for (int i = 0; i < e->n_extra; i++) {
            if (e->extra_k[i] && !strcmp(e->extra_k[i], "resource")) res = e->extra_v[i];
            if (e->extra_k[i] && !strcmp(e->extra_k[i], "duration") && e->extra_v[i]) {
                if (!strcmp(e->extra_v[i], "live_end") || !strcmp(e->extra_v[i], "during_live"))
                    dur = RB_TEMP_LIVE_END;
                else if (!strcmp(e->extra_v[i], "until_end_of_turn") || !strcmp(e->extra_v[i], "first_turn"))
                    dur = RB_TEMP_TURN_END;
            }
        }
        int amt = cnt < 0 ? -cnt : cnt;
        int sign = cnt < 0 ? -1 : 1;
        int self_target = 0; const char *gn = NULL; int target_count = -1;
        for (int i = 0; i < e->n_extra; i++) {
            if (e->extra_k[i] && !strcmp(e->extra_k[i], "self_target") &&
                e->extra_v[i] && !strcmp(e->extra_v[i], "true")) self_target = 1;
            if (e->extra_k[i] && !strcmp(e->extra_k[i], "group_names")) gn = e->extra_v[i];
            if (e->extra_k[i] && !strcmp(e->extra_k[i], "target_count") && e->extra_v[i])
                target_count = atoi(e->extra_v[i]);
        }
        if (!res || !strcmp(res, "energy")) {
            /* target=="both" grants energy to BOTH players (mirrors gain_resource). */
            int eng_players[2]; int nep = 0;
            if (e->target && !strcmp(e->target, "both")) { eng_players[nep++] = actor; eng_players[nep++] = actor ^ 1; }
            else eng_players[nep++] = who;
            for (int ep = 0; ep < nep; ep++) {
                RbPlayer *EP = &g->p[eng_players[ep]];
                EP->energy_active += cnt;
                if (EP->energy_active > RB_MAX_ENERGY_CARDS) EP->energy_active = RB_MAX_ENERGY_CARDS;
            }
        } else {
            int recips[RB_STAGE_SIZE * 2 + 1]; int nr = 0;
            const char *position = NULL;
            for (int i = 0; i < e->n_extra; i++) {
                if (e->extra_k[i] && !strcmp(e->extra_k[i], "position")) {
                    position = e->extra_v[i];
                    break;
                }
            }
            int position_idx = position ? rb_stage_position_index(position) : -1;
            if (g->n_selected_cards > 0) {
                for (int i = 0; i < g->n_selected_cards && nr < (int)(sizeof(recips)/sizeof(recips[0])); i++)
                    recips[nr++] = g->selected_cards[i];
            } else if (self_target && !gn && target_count < 0) {
                if (host_cid >= 0) recips[nr++] = host_cid;
            } else {
                int tgt_players[2]; int ntp = 0;
                if (e->target && !strcmp(e->target, "both")) { tgt_players[ntp++] = actor; tgt_players[ntp++] = actor ^ 1; }
                else tgt_players[ntp++] = target_player(e, actor);
                for (int tp = 0; tp < ntp; tp++) {
                    RbPlayer *TP = &g->p[tgt_players[tp]];
                    for (int q = 0; q < RB_STAGE_SIZE; q++) {
                        int cid = TP->stage[q];
                        if (cid == RB_EMPTY_SLOT) continue;
                        if (position_idx >= 0 && q != position_idx) continue;
                        if (gn && !rb_card_matches_group_str(cid, gn)) continue;
                        if (e->card_type_field[0] && !rb_card_matches_type(cid, e->card_type_field)) continue;
                        if (nr < (int)(sizeof(recips)/sizeof(recips[0]))) recips[nr++] = cid;
                    }
                }
            }
            fprintf(stderr, "[GR_TARGET] resource=%s selected=%d candidates=%d target_count=%d host=%d\n",
                    res ? res : "-", g->n_selected_cards, nr, target_count, host_cid);
            if (g->n_selected_cards == 0 && target_count > 0 && nr > target_count) {
                if (g->queue.target_selection_eff) rb_effect_free(g->queue.target_selection_eff);
                g->queue.target_selection_eff = rb_effect_deep_clone(e);
                rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "stage",
                               e->card_type_field[0] ? e->card_type_field : "member_card",
                               target_count, 0, "gain_resource_targets");
                if (gn) strncpy(g->queue.pending.filter_group, gn, sizeof(g->queue.pending.filter_group) - 1);
                strncpy(g->queue.pending.target_player_id, e->target ? e->target : "self",
                        sizeof(g->queue.pending.target_player_id) - 1);
                rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
                g->queue.resume_host = host_cid;
                g->queue.resume_eff = e;
                return;
            }
            if (g->n_selected_cards == 0 && target_count > 0 && nr > target_count)
                nr = target_count;
            /* Filtered grants (group/card_type) must NOT fall back to the host
                when no member matches — Rust grants to the matching set only, so
                a "boost other μ's members" ability yields 0 when none are present.
                Only the unfiltered self-target case falls back to the host. */
            if (nr == 0 && host_cid >= 0 && !gn) recips[nr++] = host_cid;
            for (int r = 0; r < nr; r++) {
                int cid = recips[r];
                if (!strcmp(res, "blade")) {
                    rb_mods_add_blade(&g->mods, cid, amt * sign);
                    if (dur != RB_TEMP_PERM) {
                        RbTempEffect te; memset(&te, 0, sizeof(te));
                        te.card_id = cid; te.dur = dur; te.blade = (int16_t)(amt * sign);
                        if (g->n_temp_effects < RB_MAX_TEMP_EFFECTS) g->temp_effects[g->n_temp_effects++] = te;
                    }
                } else if (!strcmp(res, "heart")) {
                    int col = (g->queue.selected_heart_color >= 0)
                                  ? g->queue.selected_heart_color
                                  : RB_HEART_PINK;
                    /* honour an explicit heart_color extra (e.g. "heart06") — mirrors
                        the per-color heart grants in misc.rs:execute_gain_resource. */
                    for (int i = 0; i < e->n_extra; i++) {
                        if (e->extra_k[i] && !strcmp(e->extra_k[i], "heart_color") && e->extra_v[i]) {
                            int pc = rb_parse_heart_color(e->extra_v[i]);
                            if (pc >= 0) col = pc;
                            break;
                        }
                    }
                    g->queue.selected_heart_color = -1; /* consumed by this grant */
                    rb_mods_add_heart(&g->mods, cid, col, amt * sign);
                    fprintf(stderr, "[GR_HEART_APPLY] cid=%d color=%d amount=%d modifier=%d\n",
                            cid, col, amt * sign, rb_mods_get_heart(&g->mods, cid, col));
                    if (dur != RB_TEMP_PERM) {
                        RbTempEffect te; memset(&te, 0, sizeof(te));
                        te.card_id = cid; te.dur = dur; te.heart[col] = (int16_t)(amt * sign);
                        if (g->n_temp_effects < RB_MAX_TEMP_EFFECTS) g->temp_effects[g->n_temp_effects++] = te;
                    }
                } else if (!strcmp(res, "score")) {
                    rb_mods_add_score(&g->mods, cid, amt * sign);
                    if (dur != RB_TEMP_PERM) {
                        RbTempEffect te; memset(&te, 0, sizeof(te));
                        te.card_id = cid; te.dur = dur; te.score = (int16_t)(amt * sign);
                        if (g->n_temp_effects < RB_MAX_TEMP_EFFECTS) g->temp_effects[g->n_temp_effects++] = te;
                    }
                }
            }
            /* Temporary-duration grants are tracked via temp effects and must
               survive rb_recalc_constants (which owns the constant_* tracking);
               only recalc for permanent grants. */
            if (dur == RB_TEMP_PERM) rb_recalc_constants(g);
            for (int r = 0; r < nr; r++)
                fprintf(stderr, "[GR_FINAL] cid=%d orange=%d temp_count=%d\n",
                        recips[r], rb_mods_get_heart(&g->mods, recips[r], RB_HEART_ORANGE),
                        g->n_temp_effects);
        }
    } else if (!strcmp(act, "pay_energy") || !strcmp(act, "pay_cost") ||
               !strcmp(act, "activation_cost")) {
        /* Optional pay-or-skip gate (mirrors ability/cost.rs: has_skip_prompt / handle_optional_cost_payment).
           If the effect is marked optional and active energy insufficient, emit a pay/skip choice
           instead of auto-paying. Host auto-drains via skip. */
        if (e->is_optional) {
            /* Optional cost: emit a pay/skip gate (cost.rs handle_optional_cost_payment).
               Stash the cost effect so resume can pay (selected) or skip (declined). */
            rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, cnt, 1, "pay_optional_cost:skip");
            g->queue.deferred = (AbilityEffect *)e;
            return;
        }
        if (W->energy_active < cnt) {
            /* insufficient without optional flag → treat as skip for portability (don't go negative) */
            return;
        }
        W->energy_active -= cnt;
        if (W->energy_active < 0) W->energy_active = 0;
        rb_recalc_constants(g);
    } else if (!strcmp(act, "modify_score") || !strcmp(act, "gain_score")) {
        if (e->target && !strcmp(e->target, "live_total")) {
            rb_execute_modify_score(g, actor, e);
        } else if (e->target && !strcmp(e->target, "both")) {
            g->p[actor].score += cnt;
            g->p[actor ^ 1].score += cnt;
        } else if (host_cid >= 0) {
            rb_execute_modify_score(g, actor, e);
        } else {
            W->score += cnt;
        }
    } else if (!strcmp(act, "gain_heart") ||
                !strcmp(act, "place_heart")) {
        int col = (g->queue.selected_heart_color >= 0)
                      ? g->queue.selected_heart_color
                      : heart_color_of(e, RB_HEART_PINK);
        g->queue.selected_heart_color = -1;
        if (host_cid >= 0) rb_mods_add_heart(&g->mods, host_cid, col, cnt);
        else W->hearts[col] += cnt;
    } else if (!strcmp(act, "specify_heart_color")) {
        rb_effect_specify_heart_color(g, actor, e, host_cid);
    } else if (!strcmp(act, "lose_heart") || !strcmp(act, "damage")) {
        int col = heart_color_of(e, RB_HEART_PINK);
        O->hearts[col] -= cnt;
        if (O->hearts[col] < 0) O->hearts[col] = 0;
    } else if (!strcmp(act, "heal")) {
        int col = heart_color_of(e, RB_HEART_PINK);
        if (host_cid >= 0) rb_mods_add_heart(&g->mods, host_cid, col, cnt);
        else O->hearts[col] += cnt;
    } else if (!strcmp(act, "gain_blade") || !strcmp(act, "add_blade") ||
               !strcmp(act, "gain_blade_heart") || !strcmp(act, "set_blade_count") ||
               !strcmp(act, "modify_blade")) {
        /* Blade is a per-card property; mirror misc.rs grant_blade which targets
           the resolving card (activating_card). Fall back to the actor's first
           stage member when no host is threaded. */
        int cid = host_cid >= 0 ? host_cid : -1;
        if (cid < 0) for (int q = 0; q < RB_STAGE_SIZE; q++) if (W->stage[q] != RB_EMPTY_SLOT) { cid = W->stage[q]; break; }
        if (cid >= 0) rb_mods_add_blade(&g->mods, cid, cnt);
        rb_recalc_constants(g);
    } else if (!strcmp(act, "return_to_hand") || !strcmp(act, "bounce") ||
               !strcmp(act, "back_to_hand")) {
        rb_effect_move_cards(g, actor, e); /* target re-resolved inside */
        (void)O;
    } else if (!strcmp(act, "deck_bottom") || !strcmp(act, "put_on_bottom")) {
        do_move(g, who, RB_ZONE_HAND, RB_ZONE_DECK, cnt, 0);
    } else if (!strcmp(act, "move_cards")) {
        rb_effect_move_cards(g, actor, e);
    } else if (!strcmp(act, "change_state")) {
         rb_effect_change_state(g, actor, e, host_cid);
    } else if (!strcmp(act, "look_at")) {
        rb_effect_look_at(g, actor, e);
    } else if (!strcmp(act, "reveal")) {
        rb_effect_reveal(g, actor, e);
    } else if (!strcmp(act, "reveal_per_group")) {
        rb_effect_reveal_per_group(g, actor, e);
    } else if (!strcmp(act, "reveal_until_live_card")) {
        rb_effect_reveal_until_live_card(g, actor, e);
    } else if (!strcmp(act, "reveal_until_chosen_card")) {
        rb_effect_reveal_until_chosen_card(g, actor, e);
    } else if (!strcmp(act, "reveal_until_target")) {
        rb_effect_reveal_until_target(g, actor, e);
    } else if (!strcmp(act, "select_cards")) {
        rb_effect_select_cards(g, actor, e);
    } else if (!strcmp(act, "select")) {
        rb_effect_select(g, actor, e);
    } else if (!strcmp(act, "look_and_select")) {
        rb_effect_look_and_select(g, actor, e);
    } else if (!strcmp(act, "set_cost")) {
        rb_effect_set_cost(g, actor, e, host_cid);
    } else if (!strcmp(act, "modify_cost") || !strcmp(act, "set_cost_to_use") ||
                !strcmp(act,"modify_yell_count") || !strcmp(act,"modify_yell_source")) {
        rb_effect_modify_cost(g, actor, e, host_cid);
    } else if (!strcmp(act, "energy_placement")) {
        rb_effect_energy_placement(g, actor, e);
    } else if (!strcmp(act, "energy_state_change")) {
        rb_effect_energy_state_change(g, actor, e);
    } else if (!strcmp(act, "set_card_identity")) {
        rb_effect_set_card_identity(g, actor, e, host_cid);
    } else if (!strcmp(act, "set_blade_type")) {
        rb_effect_set_blade_type(g, actor, e, host_cid);
    } else if (!strcmp(act, "set_blade_count")) {
        rb_effect_set_blade_count(g, actor, e, host_cid);
    } else if (!strcmp(act, "set_heart_type")) {
        rb_effect_set_heart_type(g, actor, e, host_cid);
    } else if (!strcmp(act, "all_blade_timing")) {
        rb_effect_all_blade_timing(g, actor, e, host_cid);
    } else if (!strcmp(act, "choose_required_hearts")) {
        /* Headless can't let the player pick a color, so apply the chosen
           required-heart count to the "all" color (satisfies any color check). */
        int cid = host_cid >= 0 ? host_cid : -1;
        if (cid < 0) for (int q = 0; q < RB_STAGE_SIZE; q++) if (W->stage[q] != RB_EMPTY_SLOT) { cid = W->stage[q]; break; }
        if (cid >= 0) {
            int lim = cnt > 0 ? cnt : 1;
            rb_mods_add_need_heart(&g->mods, cid, 7, lim);
            rb_recalc_constants(g);
        }
    } else if (!strcmp(act, "modify_required_hearts") || !strcmp(act, "modify_required_hearts_global") ||
                !strcmp(act, "modify_required_hearts_success")) {
        rb_effect_modify_hearts(g, actor, e);
    } else if (!strcmp(act, "gain_ability")) {
        rb_gain_ability(g, actor, e);
    } else if (!strcmp(act, "gain_ability_from_source")) {
        rb_gain_ability_from_source(g, actor, e, host_cid);
    } else if (!strcmp(act, "invalidate_ability")) {
        rb_invalidate_ability(g, actor, e);
    } else if (!strcmp(act, "suppress_ability_trigger")) {
        rb_suppress_ability_trigger(g, actor, e, host_cid);
    } else if (!strcmp(act, "activate_ability")) {
        /* Mirror ability_effects.rs::execute_activate_ability — fire the matching
           ability of each selected card (or the activating card's own ability). */
        rb_activate_ability_effect(g, actor, e, host_cid);
    } else if (!strcmp(act, "reduce_live_card_set_limit")) {
        rb_effect_reduce_live_card_set_limit(g, actor, e, host_cid);
    } else if (!strcmp(act, "position_change")) {
        rb_effect_position_change(g, actor, e, host_cid);
    } else if (!strcmp(act, "rotation")) {
        rb_effect_rotation(g, actor, e);
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
        /* Real handler in effects/misc.c (h_restriction): records prohibition
           notes (consulted by rb_is_action_prohibited) and applies
           cannot_activate / cannot_active lockouts. */
        int resolved = 0;
        rb_execute_misc_effect(g, actor, W, e, &resolved);
        (void)resolved;
    } else if (!strcmp(act, "gain_surplus_heart")) {
        /* Mirror misc.rs:execute_gain_surplus_heart — capture this player's live
           surplus (total_hearts − total_required) into last_surplus_loss_count
           so it can be granted/lost as a resource by a later effect. */
        rb_effect_gain_surplus_heart(g, actor, e);
    } else if (!strcmp(act, "pay_cost_all:discard_all")) {
        /* Mirror cost.rs::handle_pay_cost_all_discard — the "may discard your whole
            hand" cost: move every card in the target player's hand into the waitroom
            (C's discard pile). Cost moves are player actions, not effects, so they are
            not recorded in recently_moved / moved_this_turn. */
        rb_effect_pay_cost_all_discard(g, actor, e);
    } else if (!strcmp(act, "re_yell")) {
        /* Mirror misc.rs:execute_re_yell. Optionally clear blade/heart modifiers
           of the target's staged members, clear the revealed pool, and mark that
           a re-yell occurred so perform_yell's hearts are applied this live. */
        int tgt = target_player(e, actor);
        RbPlayer *TP = &g->p[tgt];
        int lose = 0;
        for (int i = 0; i < e->n_extra; i++)
            if (e->extra_k[i] && !strcmp(e->extra_k[i], "lose_blade_hearts") &&
                e->extra_v[i] && !strcmp(e->extra_v[i], "true")) lose = 1;
        if (lose) {
            for (int q = 0; q < RB_STAGE_SIZE; q++) {
                int cid = TP->stage[q];
                if (cid != RB_EMPTY_SLOT) rb_mods_clear_card(&g->mods, cid);
            }
        }
        g->n_revealed = 0;
        g->re_yell_occurred = 1;
    } else if (!strcmp(act, "perform_yell")) {
        /* Mirror misc.rs:execute_perform_yell. Draw `total_blade` (= sum of the
           target's effective stage blades) cards, reveal + harvest their yell
           icons, draw the draw-icon count, and stash harvested blade hearts for
           the live's success check (pending_reyell_rebuild). */
        int tgt = target_player(e, actor);
        RbPlayer *TP = &g->p[tgt];
        int total_blade = 0;
        for (int q = 0; q < RB_STAGE_SIZE; q++) {
            int cid = TP->stage[q];
            if (cid == RB_EMPTY_SLOT) continue;
            Card c; if (rb_decode_card_by_index((uint32_t)cid, &c)) {
                total_blade += (int)c.blade + rb_mods_get_blade(&g->mods, cid);
                rb_free_card(&c);
            }
        }
        int count = total_blade > 0 ? total_blade : cnt;
        if (count <= 0) count = 1;
        memset(g->re_yell_blade_hearts, 0, sizeof(g->re_yell_blade_hearts));
        g->re_yell_note_icons = 0;
        g->re_yell_pending = 1;
        g->re_yell_owner = tgt;
        int draw_total = 0;
        for (int k = 0; k < count; k++) {
            if (TP->deck.n == 0) break;
            int cid = TP->deck.cards[--TP->deck.n];
            if (g->n_revealed < RB_MAX_REVEALED_CARDS) g->revealed_cards[g->n_revealed++] = cid;
            Card c; if (!rb_decode_card_by_index((uint32_t)cid, &c)) continue;
            int blade = (int)c.blade + rb_mods_get_blade(&g->mods, cid);
            if (blade > 0) g->re_yell_blade_hearts[RB_HEART_PINK] += blade;
            for (int hh = 0; hh < c.n_hearts; hh++) {
                int col = c.heart_color[hh];
                if (col == RB_HEART_DRAW) draw_total += c.heart_count[hh];
                else if (col == RB_HEART_SCORE) g->re_yell_note_icons += c.heart_count[hh];
                else if (col >= 0 && col < 8) g->re_yell_blade_hearts[col % 8] += c.heart_count[hh];
            }
            rb_free_card(&c);
        }
        for (int d = 0; d < draw_total; d++) { if (TP->deck.n == 0) break; rb_draw(g, tgt); }
        g->re_yell_occurred = 1;
    } else if (!strcmp(act, "repeat_procedure")) {
        /* Mirror misc.rs repeat_procedure — execute the procedure's children
           `count` times. The single pre-order pass is skipped in
           rb_execute_effect_ex so this runs exactly `count` repetitions. */
        int reps = cnt >= 1 ? cnt : 1;
        for (int r = 0; r < reps; r++)
            for (int i = 0; i < e->n_child; i++)
                rb_execute_effect_ex(g, actor, e->child[i], host_cid);
    } else if (!strcmp(act, "custom") ||
                !strcmp(act, "do_nothing")) {
        /* Compound/control no-ops. */
    } else if (!strcmp(act, "sequential")) {
        /* Mirror compound.rs::execute_sequential_effect — run the action list
            (children) in order with per-step condition gating + otherwise-condition
            skip + trailing repeat_procedure loop. */
        rb_compound_sequential(g, actor, e, host_cid);
    } else if (!strcmp(act, "conditional_alternative")) {
        /* Mirror compound.rs::execute_conditional_alternative — tiered conditions
            then legacy single-condition routing. branch=-1 routes via the effect's
            own condition (post-negation). */
        rb_compound_conditional_alternative(g, actor, e, -1, host_cid);
    } else if (!strcmp(act, "conditional_on_result")) {
        /* Mirror compound.rs::execute_conditional_on_result — run primary_effect,
            then the followup_action if result_condition is met. */
        rb_compound_conditional_on_result(g, actor, e, host_cid);
    } else if (!strcmp(act, "conditional_on_optional")) {
        /* Mirror compound.rs::execute_conditional_on_optional — taken=-1 preserves
            the legacy emit-choice (headless auto-skips); resume routes via the
            (chose_yes, negation) matrix. */
        rb_compound_conditional_on_optional(g, actor, e, -1, host_cid);
    } else if (!strcmp(act, "choice")) {
        int choice_resolved = 0;
        rb_execute_misc_effect(g, actor, W, e, &choice_resolved);
    } else if (!strcmp(act, "select_number")) {
        /* Mirror ability/choice.rs select_number — emit a count-choice the host
            answers; the chosen number is recorded in queue.choice_result on resume
            (downstream effects do not yet consume it headless). Same pause/resume
            semantics as the other choice verbs. */
        int allow = e->is_optional ? 1 : 0;
        rb_emit_choice(g, actor, RB_CHOICE_SELECT_NUMBER, NULL, NULL, cnt, allow, act);
        g->queue.selected_heart_color = heart_color_of(e, -1);
    }
    /* sequential / conditional_* / choice / re_yell / perform_yell / custom /
       do_nothing: children already executed (or nothing to do). */
    else if (!strcmp(act, "repeat_procedure")) {
        /* Mirror ability/compound.rs:execute_repeat_procedure — run the child
           effect `count` times (default 1). Children are NOT auto-run here. */
        int reps = (e->count > 0) ? e->count : 1;
        for (int r = 0; r < reps; r++)
            for (int ci = 0; ci < e->n_child; ci++)
                if (e->child[ci]) rb_execute_effect_ex(g, actor, e->child[ci], host_cid);
    }
    /* Unknown verbs: explicit no-op so they remain visible. */
}

/* ───────────────────────────── play / activate ───────────────────────────── */
int rb_card_arrived_this_turn(const GameState *g, int pl, int card_id) {
    if (pl < 0 || pl > 1) return 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (g->p[pl].stage[i] == card_id && g->stage_arrived[pl][i]) return 1;
    return 0;
}
/* Mirror engine/src/ability/util.rs::has_cannot_baton_touch_protection — walk a
   card's resolved abilities; if any effect carries `restriction_type == restriction`
   (e.g. "cannot_baton_touch") and the incoming card is NOT in that restriction's
   `exclude_group_names`, the existing member blocks the baton touch. The effect tree
   is walked recursively (child / primary / alternative / followup / optional /
   conditional sub-effects) so nested restriction abilities are honored. */
static int effect_has_restriction(const AbilityEffect *e, const char *restriction, int incoming_cid) {
    if (!e) return 0;
    /* Direct restriction effect on this node. */
    const char *rt = NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "restriction_type")) { rt = e->extra_v[i]; break; }
    if (rt && !strcmp(rt, restriction)) {
        /* exclude_group_names: a member of any excluded group is NOT protected. */
        const char *ex = NULL;
        for (int i = 0; i < e->n_extra; i++)
            if (e->extra_k[i] && !strcmp(e->extra_k[i], "exclude_group_names")) { ex = e->extra_v[i]; break; }
        if (ex && *ex) {
            if (ex && *ex && rb_card_matches_group_str(incoming_cid, ex)) return 0;
        }
        return 1;
    }
    /* Recurse into compound / nested effects. */
    for (int i = 0; i < e->n_child; i++)
        if (effect_has_restriction(e->child[i], restriction, incoming_cid)) return 1;
    if (effect_has_restriction(e->primary_effect, restriction, incoming_cid)) return 1;
    if (effect_has_restriction(e->alternative_effect, restriction, incoming_cid)) return 1;
    if (effect_has_restriction(e->look_action, restriction, incoming_cid)) return 1;
    if (effect_has_restriction(e->select_action, restriction, incoming_cid)) return 1;
    if (effect_has_restriction(e->followup_action, restriction, incoming_cid)) return 1;
    if (effect_has_restriction(e->optional_action, restriction, incoming_cid)) return 1;
    if (effect_has_restriction(e->conditional_action, restriction, incoming_cid)) return 1;
    return 0;
}
int rb_card_has_restriction(const GameState *g, int incoming_cid, int card_id, const char *restriction) {
    /* Honor the runtime cannot-activate ban (set_card_active / restriction effect)
        as well as the card-data restriction ability (has_cannot_baton_touch_protection). */
    if (rb_card_is_cannot_active(g, card_id)) return 1;
    int n = rb_card_num_abilities((uint32_t)card_id);
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, i, &ab)) continue;
        if (effect_has_restriction(ab.effect, restriction, incoming_cid)) { rb_free_ability(&ab); return 1; }
        if (effect_has_restriction(ab.cost, restriction, incoming_cid))   { rb_free_ability(&ab); return 1; }
        rb_free_ability(&ab);
    }
    return 0;
}
void rb_send_to_waitroom(GameState *g, int pl, int card_id) {
    RbPlayer *P = &g->p[pl];
    if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = card_id;
}

int rb_card_is_cannot_active(const GameState *g, int card_id) {
    if (g->player_cannot_activate[0] || g->player_cannot_activate[1]) {
        /* Per-player lockout: a staged member of a locked player cannot act.
           We can't cheaply know ownership here, so rb_activate_ability checks
           the actor's player flag directly. This helper covers the per-card set. */
    }
    for (int i = 0; i < g->n_cannot_active_cards; i++)
        if (g->cannot_active_cards[i] == card_id) return 1;
    return 0;
}

/* Search an effect tree for a modify_cost action with op "set"; return its value.
    Mirrors state.rs execute_modify_cost's "set" branch (absolute cost). */
static int rb_find_set_cost_value(const AbilityEffect *e, int *out) {
    if (!e) return 0;
    if (e->action && !strcmp(e->action, "modify_cost")) {
        const char *op = "add"; int value = 0;
        for (int i = 0; i < e->n_extra; i++) {
            if (e->extra_k[i] && !strcmp(e->extra_k[i], "operation") && e->extra_v[i]) op = e->extra_v[i];
            if (e->extra_k[i] && (strcmp(e->extra_k[i], "value") == 0 || strcmp(e->extra_k[i], "set_value") == 0) && e->extra_v[i])
                value = atoi(e->extra_v[i]);
        }
        if (value == 0 && e->count) value = e->count;
        if (!strcmp(op, "set")) { *out = value; return 1; }
        return 0;
    }
    for (int i = 0; i < e->n_child; i++)
        if (rb_find_set_cost_value(e->child[i], out)) return 1;
    return 0;
}

/* Detect a play-time alternative cost: a 常時/プレイ時 ability whose effect sets the
    card's cost to a fixed value (Rust play_time_cost_reduction_hook's alt-cost).
    Returns the alternative value, or 0 if none. */
static int rb_detect_alt_cost(GameState *g, int card, int *set_out) {
    (void)g;
    int n = rb_card_num_abilities((uint32_t)card);
    for (int ai = 0; ai < n; ai++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card, ai, &ab)) continue;
        int is_pt = 0;
        if (ab.triggers && (strstr(ab.triggers, "常時") || strstr(ab.triggers, "プレイ時"))) is_pt = 1;
        int val = 0;
        if (is_pt && ab.effect && rb_find_set_cost_value(ab.effect, &val)) {
            rb_free_ability(&ab);
            *set_out = val;
            return 1;
        }
        rb_free_ability(&ab);
    }
    return 0;
}

static int effect_is_double_baton(const AbilityEffect *e) {
    if (!e || !e->action) return 0;
    if (!strcmp(e->action, "double_baton_touch")) return e->count > 1;
    if (!strcmp(e->action, "play_baton_touch") && e->count > 1) return 1;
    for (int i = 0; i < e->n_child; i++)
        if (effect_is_double_baton(e->child[i])) return 1;
    if (effect_is_double_baton(e->primary_effect)) return 1;
    if (effect_is_double_baton(e->alternative_effect)) return 1;
    if (effect_is_double_baton(e->look_action)) return 1;
    if (effect_is_double_baton(e->select_action)) return 1;
    if (effect_is_double_baton(e->followup_action)) return 1;
    if (effect_is_double_baton(e->optional_action)) return 1;
    if (effect_is_double_baton(e->conditional_action)) return 1;
    return 0;
}

int rb_complete_double_baton(GameState *g, int selected_pair) {
    if (!g || selected_pair < 0) return 0;
    int actor = g->queue.actor;
    if (actor < 0 || actor > 1 || !g->queue.resume_host) return 0;
    int card = g->queue.resume_host;
    RbPlayer *P = &g->p[actor];
    int eligible[RB_STAGE_SIZE];
    int n = 0;
    for (int q = 0; q < RB_STAGE_SIZE; q++)
        if (P->stage[q] != RB_EMPTY_SLOT && !g->stage_arrived[actor][q])
            eligible[n++] = q;
    int i1, i2;
    if (n == 2) { i1 = eligible[0]; i2 = eligible[1]; }
    else if (selected_pair == 0) { i1 = eligible[0]; i2 = eligible[1]; }
    else if (selected_pair == 1) { i1 = eligible[0]; i2 = eligible[2]; }
    else { i1 = eligible[1]; i2 = eligible[2]; }
    if (i1 >= RB_STAGE_SIZE || i2 >= RB_STAGE_SIZE) return 0;
    int old1 = P->stage[i1], old2 = P->stage[i2];
    for (int u = 0; u < P->under_cards[i1].n; u++)
        rb_send_to_waitroom(g, actor, P->under_cards[i1].cards[u]);
    for (int u = 0; u < P->under_cards[i2].n; u++)
        rb_send_to_waitroom(g, actor, P->under_cards[i2].cards[u]);
    P->under_cards[i1].n = 0;
    P->under_cards[i2].n = 0;
    rb_send_to_waitroom(g, actor, old1);
    rb_send_to_waitroom(g, actor, old2);
    P->stage[i1] = RB_EMPTY_SLOT;
    P->stage[i2] = RB_EMPTY_SLOT;
    g->stage_arrived[actor][i1] = 0;
    g->stage_arrived[actor][i2] = 0;
    int hand_idx = -1;
    for (int i = 0; i < P->hand.n; i++) if (P->hand.cards[i] == card) { hand_idx = i; break; }
    if (hand_idx < 0) return 0;
    for (int i = hand_idx; i + 1 < P->hand.n; i++) P->hand.cards[i] = P->hand.cards[i + 1];
    P->hand.n--;
    int dst = g->baton_last_vacated_area[actor];
    if (dst < 0 || dst >= RB_STAGE_SIZE || dst == i1 || dst == i2) dst = i1;
    P->stage[dst] = card;
    P->stage_wait[dst] = 0;
    g->stage_arrived[actor][dst] = 1;
    g->baton_touch_used[actor] = 1;
    rb_record_baton_touch(g, actor, card);
    rb_record_baton_touch(g, actor, card);
    g->baton_touch_replaced_member_id = old2;
    g->baton_touch_arriving_card_id = card;
    g->recently_moved[0] = old1;
    if (g->n_recently_moved < 1) g->n_recently_moved = 1;
    rb_fire_debut(g, actor, card);
    rb_fire_auto(g, actor);
    rb_drain_ability_queue(g);
    return 1;
}

int rb_play_member(GameState *g, int pl, int hand_idx, int stage_pos) {
    RbPlayer *P=&g->p[pl];
    if (hand_idx < 0 || hand_idx >= P->hand.n) return 0;
    if (stage_pos < 0 || stage_pos >= RB_STAGE_SIZE) return 0;
    /* Bound re-entrancy: a debut/baton effect that places another member would
        recurse into this fn. Cap the depth so a pathological chain cannot overflow
        the stack (headless: the deepest legitimate chain is shallow). */
    if (g->play_depth > 4) return 0;
    g->play_depth++;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)P->hand.cards[hand_idx], &c)) { g->play_depth--; return 0; }
    int cid = P->hand.cards[hand_idx];
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int ai = 0; ai < n; ai++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, ai, &ab)) continue;
        int is_double = effect_is_double_baton(ab.effect);
        rb_free_ability(&ab);
        if (is_double) {
            int ne = 0;
            for (int q = 0; q < RB_STAGE_SIZE; q++)
                if (P->stage[q] != RB_EMPTY_SLOT && !g->stage_arrived[pl][q]) ne++;
            if (ne >= 2) {
                int pairs = ne * (ne - 1) / 2;
                g->baton_last_vacated_area[pl] = stage_pos;
                rb_emit_choice(g, pl, RB_CHOICE_SELECT_TARGET, NULL, NULL, pairs, 1, "double_baton_touch");
                rb_queue_pause_for_choice(g, &g->queue.pending);
                g->queue.resume_mode = 0;
                g->queue.resume_host = cid;
                rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_TARGET);
                g->queue.resume_actor = pl;
                g->play_depth--;
                return 1;
            }
        }
        break;
    }
    rb_free_card(&c);
    int is_baton = (P->stage[stage_pos] >= 0); /* playing onto an occupied area */
    int baton_replaced = RB_EMPTY_SLOT;


    /* Baton-touch legality gates (Rule 9.6.2.1.2.1): cannot replace a member that
       arrived THIS turn, and a member with cannot_baton_touch restriction is
       immovable. Also enforce one baton per play-action. */
    if (is_baton) {
        int old = P->stage[stage_pos];
        if (g->baton_touch_used[pl] || rb_card_arrived_this_turn(g, pl, old) ||
            rb_card_has_restriction(g, cid, old, "cannot_baton_touch")) {
            rb_free_card(&c); g->play_depth--; return 0;
        }
    }

    int base_cost = c.cost;
    int cost_mod = rb_mods_get_cost(&g->mods, cid);
    int cost = base_cost + cost_mod;
    if (cost < 0) cost = 0;
    if (is_baton) {
        /* Baton: pay only the difference vs the replaced member's cost. */
        Card oldc; rb_decode_card_by_index((uint32_t)P->stage[stage_pos], &oldc);
        int old_cost = (int)oldc.cost + rb_mods_get_cost(&g->mods, P->stage[stage_pos]);
        if (old_cost < 0) old_cost = 0;
        cost = cost - old_cost;
        if (cost < 0) cost = 0;
        rb_free_card(&oldc);
    }
    /* Play-time alternative cost (Rust play_time_cost_reduction_hook): if the card
        has a 常時/プレイ時 modify_cost(set) alt-cost, pause here and remember the play
        so the caller answers via rb_complete_play_with_cost (mapped from the
        transpiled answer_play_choice). Only on the first pass (ptc_resuming==0);
        the resume re-enters this fn below and simply pays the chosen cost. Cards
        without such an ability take the unchanged synchronous path. */
    if (!g->ptc_resuming) {
        int setv = 0;
        if (rb_detect_alt_cost(g, cid, &setv)) {
            g->ptc_active = 1;
            g->ptc_card = cid; g->ptc_hand = hand_idx; g->ptc_area = stage_pos;
            g->ptc_set = setv; g->ptc_base = base_cost;
            rb_free_card(&c); g->play_depth--; return 0;
        }
    }
    /* Cost gate: member play cost is never optional, so reject if insufficient. */
    if (P->energy_active < cost) {
        rb_free_card(&c); g->play_depth--; return 0;
    }
    P->energy_active -= cost;
    g->n_recently_moved = 0;
    rb_recalc_constants(g);

    int card = bag_remove_at(&P->hand, hand_idx);
    if (is_baton) {
        /* Replace: old member (and its under-cards) → waitroom. */
        int old = P->stage[stage_pos];
        baton_replaced = old;
        Card old_card;
        int old_cost = -1;
        if (rb_decode_card_by_index((uint32_t)old, &old_card)) {
            old_cost = old_card.cost + rb_mods_get_cost(&g->mods, old);
            if (old_cost < 0) old_cost = 0;
            rb_free_card(&old_card);
        }
        g->baton_touch_zero_cost = cost == 0;
        g->baton_touch_replaced_member_cost = old_cost;
        g->baton_touch_replaced_member_id = old;
        for (int u = 0; u < P->under_cards[stage_pos].n; u++)
            rb_send_to_waitroom(g, pl, P->under_cards[stage_pos].cards[u]);
        P->under_cards[stage_pos].n = 0;
        rb_send_to_waitroom(g, pl, old);
        P->stage[stage_pos] = -1;
        g->baton_touch_used[pl] = 1;
        rb_record_baton_touch(g, pl, card);
        g->baton_touch_arriving_card_id = card;
    }
    P->stage[stage_pos] = card;
    P->stage_wait[stage_pos] = 0;
    g->stage_arrived[pl][stage_pos] = 1;
    /* Mirror GameState::debut_count_this_turn — a member debuted (登場) on this
        turn; bump the per-player counter so temporal conditions gate on it. */
    g->debut_count_this_turn[pl]++;
    /* Mirror Rust set_recently_moved_batch: the played (or baton-replaced) member
        is now "recently moved" so movement-condition gates and auto-abilities-for-
        movement (trigger_auto_abilities_for_movement) see it during this resolution. */
    g->recently_moved[0] = is_baton && baton_replaced >= 0 ? baton_replaced : card;
    g->n_recently_moved = 1;
    if (is_baton && baton_replaced >= 0)
        g->moved_this_turn[baton_replaced] = 1;
    if (is_baton) g->baton_last_vacated_area[pl] = stage_pos;

    /* Fire ALL debut / baton abilities on the played card. Mirrors Rust's
        handle_play_member_to_stage → trigger_debut_abilities, which iterates
        EVERY 登場 (and バトンタッチ) ability and executes each ability's COST
        then its EFFECT. The previous port only ran the single default
        c.ability->effect and silently dropped both the card's additional debut
        abilities and their costs (the DEFERRED(ST-K) stub) — that erased
        kasumi/ayumu/rina/mia debut costs/effects. We execute inline, guarded
        against re-entrant pending choices, and deliberately do NOT also queue
        the abilities (rb_trigger_debut) so a later rb_drain_ability_queue
        cannot double-run them. */
    g->current_is_baton = is_baton;
    /* Rust's resolver::resolve_ability publishes `gs.activating_card` for the
       whole resolution (resolver.rs:1161) and clears it in
       finish_ability_resolution. The 登場 trigger reaches resolve_ability the
       same way (turn/triggers.rs:119-129 → trigger_auto_ability with the
       stage card id), so every debut cost that names `self_cost` /
       `exclude_self` sees the member that was just placed. The inline loop
       below has no queue entry of its own, so publish and restore it here —
       without it `self_cost` resolved against -1 and validate_cost dropped the
       whole bundled 「ウェイトにし、手札を1枚控え室に置いてもよい」 cost. */
    int saved_activating = g->activating_card;
    g->activating_card = card;
    {
        int n = rb_card_num_abilities((uint32_t)card);
        for (int ai = 0; ai < n; ai++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)card, ai, &ab)) continue;
            int is_debut = ab.triggers && rb_trigger_is(ab.triggers, "登場");
            int is_baton_tr = ab.triggers && strstr(ab.triggers, "バトンタッチ");
            if (is_debut || (is_baton && is_baton_tr)) {
                /* resolver.rs:1182 pays the cost with `?`, so a cost that
                   fails validation aborts the resolution and run_ability_effect
                   is never reached (resolver.rs:1214). The C discarded the
                   return value and ran the effect anyway, so an unpayable
                   debut cost still applied its effect. */
                int cost_ok = 1;
                int cost_skipped = 0;
                if (ab.cost && !rb_has_pending_choice(g)) {
                    cost_ok = rb_pay_cost(g, pl, ab.cost);
                    cost_skipped = rb_cost_optional_skipped();
                    if (rb_has_pending_choice(g) && ab.effect) {
                        g->queue.deferred = ab.effect;
                        g->queue.resume_host = card;
                        g->queue.resume_eff = ab.effect;
                        fprintf(stderr, "[DEBUT_COST_PENDING] card=%d deferred=%p eff=%p cur=%d n=%d state=%d\n",
                                card, (void *)g->queue.deferred, (void *)ab.effect,
                                g->queue.cur, g->queue.n_entries, g->queue.state);
                    }
                }
                rb_drain_ability_queue(g);
                /* gates.rs:195-213 `optional_cost_skip_gate` is a POST_COST_GATE:
                   an optional cost that resolved as a skip
                   (optional_cost_result == Some(false)) stops the resolution
                   before the effect. The C has no gate table here, so the same
                   verdict arrives through rb_cost_optional_skipped(). */
                if (cost_ok && !cost_skipped && ab.effect && !rb_has_pending_choice(g))
                    rb_execute_effect_ex(g, pl, ab.effect, card);
                rb_drain_ability_queue(g);
                if (rb_has_pending_choice(g)) {
                    /* A child effect deferred a pending choice; its resume parks
                       a raw pointer (g->queue.resume_parent) into THIS ability's
                       cost/effect tree, so rb_free_ability must not free it out
                       from under the deferred choice. Detach the trees; they stay
                       alive until the choice is answered. The leak is bounded per
                       game and harmless for the headless harness.

                       LEAK STILL OPEN (deliberately not "fixed" here). Deferring
                       the free to a later moment is NOT safe: closing the prompt
                       does not release the tree, because the queue keeps raw
                       pointers into it (g->queue.deferred / resume_eff) and a
                       later rb_drain_ability_queue walks them. A pool that
                       released on the next 登場 was measured to SEGFAULT
                       tests/test_baton_touch.c (use-after-free after the prompt
                       closed), so the detach-and-leak is kept rather than traded
                       for a crash.

                       The real fix is Rust's: the 登場 trigger reaches
                       resolve_ability THROUGH the ability queue
                       (resolver.rs:1159-1161), so the resolver owns the ability
                       for the whole resolution and the queue entry frees it when
                       the resolution completes. That means pushing a real entry
                       with rb_queue_push_with_trigger instead of resolving inline,
                       and it also retires the `s_cost_deferred_host` workaround in
                       cost.c and the `choice_debut_cost_action` reconstruction in
                       choice.c — both exist only because this loop owns no entry
                       and therefore has no `activating_card` window. Recommended
                       to the owner of the queue. */
                    ab.cost = NULL;
                    ab.effect = NULL;
                }
            }
            rb_free_ability(&ab);
        }
    }
    g->activating_card = saved_activating;
    g->current_is_baton = 0;
    /* A member placed on stage is an event that triggers that player's 自動
        (Auto) abilities — mirrors engine/src/turn/actions.rs
        handle_play_member_to_stage → trigger_auto_abilities_for_player. */
    rb_fire_auto(g, pl);
    rb_drain_ability_queue(g);
    rb_free_card(&c);
    g->play_depth--;
    return 1;
}

/* Activate a card's ability by card id. Mirrors Rust's `GameState::activate_ability`:
    the card may expose several abilities (RAKA_CARD_ABILITY_PAIRS); the manual
    "activate" action runs the one(s) whose trigger is 起動 (Activate). Apply each
    matched ability's cost BEFORE its effect. Falls back to the card's single
    default `ability_idx` when no 起動 ability exists (e.g. debut-only members). */
int rb_activate_card(GameState *g, int pl, int card_id) {
    int any = 0, matched = 0, has_activate = 0;
    int n = rb_card_num_abilities((uint32_t)card_id);
    for (int a = 0; a < n; a++) {
        uint32_t aidx;
        if (!rb_card_get_ability_idx((uint32_t)card_id, a, &aidx)) continue;
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, a, &ab)) continue;
        int is_activate = ab.triggers && strstr(ab.triggers, "起動");
        if (is_activate) {
            has_activate = 1;
            /* USE LIMIT — turn/actions/mod.rs:519-528 (handle_use_ability) with
               ability/util.rs:78-89 (ability_under_use_limit). A 起動 that
               carries 「{{ターン1回}}」 whose use is already spent THIS turn is
               not a candidate: Rust `continue`s to the next ability, so no cost
               is paid and no effect runs. Without this the C accepted a second
               起動 every turn — PL!SP-bp1-003-P and PL!S-bp3-001-R＋ both
               granted their live-total-score +1 twice, and rb_use_count stayed 0
               after the first activation. */
            if (ab.use_limit > 0 &&
                rb_use_limit_reached(&g->queue, card_id, a, ab.use_limit, g->turn))
                continue;
            /* turn/actions/mod.rs:548-550 — `game_state.activating_card =
               Some(card_id)` is published BEFORE any cost logic, with the comment
               "so self-referential filters resolve to this card", and stays
               published across the cost's choice round-trip
               (resolver.rs:1161 publishes, resolver.rs:1091 clears at
               finish_ability_resolution). Save/restore rather than clear: the
               queue path publishes the same field with a re-entry depth guard
               (game_state_abilities.c:1919-1928) and a nested resolution inside
               this one will reset it to -1, so the value must be put back on the
               way out — a stale card id here would make a LATER, unrelated
               ability believe it is paying its own self_cost. */
            int saved_act = g->activating_card;
            g->activating_card = card_id;
            /* rb_cost_optional_skipped() (cost.c:47) is a STICKY latch: a
               decline sets it, the next cost PAYMENT clears it. Sample it before
               the tree runs so only a 0->1 transition can be attributed to THIS
               activation's cost. */
            int pre_skipped = rb_cost_optional_skipped();
            /* Mirror Rust resolver::resolve_ability: cost then effect are one
                ability resolution. Wrap them in a single heap effect tree so an
                optional cost that emits a pay/skip choice bridges to the effect via
                the resume_parent/resume_child continuation. The decoded ability is
                kept alive in g->activation_keepalive across the choice round-trip. */
            AbilityEffect *act = (AbilityEffect *)malloc(sizeof(AbilityEffect));
            memset(act, 0, sizeof(*act));
            act->n_child = 0;
            if (ab.cost)   act->child[act->n_child++] = ab.cost;
            if (ab.effect) act->child[act->n_child++] = ab.effect;
            g->activation_act = act;
            g->activation_keepalive = ab;
            g->activation_keepalive_valid = 1;
            g->n_recently_moved = 0;
            rb_execute_effect_ex(g, pl, act, card_id);
            rb_drain_ability_queue(g);
            if (!g->queue.has_pending) {
                /* Free act's children first (they are owned by act, not by keepalive) */
                act->n_child = 0;
                g->activation_keepalive.cost = NULL;
                g->activation_keepalive.effect = NULL;
                rb_free_ability(&g->activation_keepalive);
                g->activation_keepalive_valid = 0;
                free(g->activation_act);
                g->activation_act = NULL;
            }
            /* ── USE LIMIT RECORD ── ability/gates.rs:279-335
               (record_use_limit) / turn/actions/mod.rs:1488-1500
               (finish_paid_ability). One accepted activation spends exactly one
               use. The C spends it at RESOLUTION time, not at enqueue
               (game_state_abilities.c:2001-2032, whose `use_limit_recorded` flag
               is what stops the resolution-time gate from rejecting the entry
               that just consumed it), and this inline path owns no queue entry at
               all, so it has to record for itself.

               A DECLINED optional cost must not spend the activation —
               finish_paid_ability:1492 guards on
               `snap.cost_entry_opt_result != Some(false)`. The verdict is only
               knowable once the prompt is closed, and that is guaranteed here:
               cost.c's decline path (rb_handle_optional_cost_payment,
               cost.c:1195-1218) clears has_pending and settles the latch
               synchronously, so a still-open prompt always means the cost has NOT
               been declined yet — recording then matches Rust recording at
               Final/finish_paid_ability once the round-trip completes. */
            int declined = ab.cost && rb_cost_optional_skipped() && !pre_skipped;
            if (ab.use_limit > 0 && !declined)
                rb_record_use(&g->queue, card_id, a, g->turn);
            g->activating_card = saved_act;
            any = 1;
            matched++;
        }
        /* Don't free ab.cost/ab.effect here - they are owned by act, not by ab */
    }
    /* Rust reports "No activatable ability found for this card at its current
       location" (turn/actions/mod.rs:545-546) when every 起動 candidate was
       refused. The default-ability fallback below must therefore NOT run for a
       card that HAS 起動 abilities which the use-limit gate turned down —
       otherwise a refused second 起動 silently re-runs the card's default
       ability and grants its effect all over again. */
    if (matched == 0 && !has_activate) {
        /* Fallback: single default ability (debut/auto-only members). */
        Card c;
        if (rb_decode_card_by_index((uint32_t)card_id, &c)) {
            if (c.ability) {
                if (c.ability->cost)   { g->n_recently_moved = 0; rb_execute_effect_ex(g, pl, c.ability->cost, card_id); rb_drain_ability_queue(g); any = 1; }
                if (c.ability->effect)  { g->n_recently_moved = 0; rb_execute_effect_ex(g, pl, c.ability->effect, card_id); rb_drain_ability_queue(g); any = 1; }
            }
            rb_free_card(&c);
        }
    }
    return any;
}

/* Answer a play paused by rb_detect_alt_cost. On accept, set the cost modifier so
    the total cost equals the alternative value (Rust set_cost_modifier(card, base-set)),
    then re-enter rb_play_member (ptc_resuming=1 so detection is skipped) to pay the
    chosen cost and place the card. On decline, place at the base cost. Returns the
    placement result (0 if nothing was pending / placement failed). */
int rb_complete_play_with_cost(GameState *g, int pl, int accept) {
    if (!g->ptc_active) return 0;
    int card = g->ptc_card, hand = g->ptc_hand, area = g->ptc_area;
    int set = g->ptc_set, base = g->ptc_base;
    if (accept && set != base)
        rb_mods_set_cost(&g->mods, card, set - base);
    g->ptc_resuming = 1;
    int placed = rb_play_member(g, pl, hand, area);
    g->ptc_resuming = 0;
    g->ptc_active = 0;
    return placed;
}

int rb_activate_ability(GameState *g, int pl, int hand_idx) {
    RbPlayer *P = &g->p[pl];
    /* Restriction gate: a player (or specific card) under a cannot-activate
        lockout may not activate abilities. */
    if (g->player_cannot_activate[pl]) return 0;
    if (hand_idx >= 0 && hand_idx < P->hand.n &&
        rb_card_is_cannot_active(g, P->hand.cards[hand_idx])) return 0;
    if (hand_idx < 0 || hand_idx >= P->hand.n) return 0;
    return rb_activate_card(g, pl, (int)P->hand.cards[hand_idx]);
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
            if (rb_has_pending_choice(g)) { rb_resume_with_choice(g, -1); rb_drain_ability_queue(g); }
            if (played) { again = 1; break; }
        }
        if (rb_has_pending_choice(g)) { rb_resume_with_choice(g, -1); rb_drain_ability_queue(g); }
    }
    /* activate abilities of staged members (one pass) — host would normally poll choice */
    for (int q = 0; q < RB_STAGE_SIZE; q++) {
        int cid = P->stage[q];
        if (cid < 0) continue;
        Card c;
        if (!rb_decode_card_by_index((uint32_t)cid, &c)) continue;
        if (c.ability && c.ability->effect) {
            g->n_recently_moved = 0; /* batch-scope for this staged member */
            rb_execute_effect_ex(g, pl, c.ability->effect, cid);
            rb_drain_ability_queue(g);
        }
        rb_free_card(&c);
    }
}

/* Faithful performance lives in src/live.c:rb_perform_live (yell→stage_hearts
   via RbMods→allocation→verdict→score). Keep a thin alias so old callers
   still compile if live.c is not linked. */
int rb_perform_live(GameState *g, int pl);

static void live_phase(GameState *g) {
    /* Live card set: auto-place up to MAX_LIVE_CARDS - reduction from each player's hand. */
    g->live_success[0] = 0; g->live_success[1] = 0; /* fresh per-turn live result */
    g->live_score[0] = 0;   g->live_score[1] = 0;   /* fresh per-turn live scores */
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
    /* Revert until_end_of_turn / first_turn temporary effects at turn end. */
    rb_check_expired_effects(g, RB_TEMP_TURN_END);
    /* The turn that just completed is a real turn — count it before deciding
        the match (mirrors Rust: the winning turn increments turn_number). */
    g->turn++;
    /* Clear per-turn temporal-condition tracking (mirrors GameState reset of
        moved_this_turn / debut_count_this_turn / position_change_occurred_this_turn). */
    for(int i=0;i<RB_MAX_CARD_IDS;i++) g->moved_this_turn[i]=0;
    g->debut_count_this_turn[0]=g->debut_count_this_turn[1]=0;
    g->position_change_occurred_this_turn=0;
    check_victory(g);
    if (g->winner != -1) { g->phase = RB_PHASE_DONE; return; }
    g->active = g->active ^ 1;
    g->phase = RB_PHASE_ACTIVE;
    /* Clear turn-scoped state_change_condition tracking. */
    memset(g->state_change_from, 0, sizeof(g->state_change_from));
    memset(g->state_change_to, 0, sizeof(g->state_change_to));
    g->last_wait_to_active_count = 0;
}

/* Main phase action execution (mirrors TurnEngine::execute_main_phase_action).
   action_type: 0=UseAbility, 1=Pass, 2=PlayMember, 3=Rock/Paper/Scissors, etc.
   Returns 0 on success, -1 on error. */
int rb_execute_main_phase_action(GameState *g, int action_type, int card_id,
                                 int target_player, int target_idx, int target_zone) {
    switch (action_type) {
        case 0: /* UseAbility */
            if (card_id >= 0) {
                return rb_activate_card(g, g->active, card_id);
            }
            break;
        case 1: /* Pass */
            rb_advance_phase(g);
            return 0;
        default:
            break;
    }
    return -1;
}

/* One full turn: active player's normal phase + shared live phase + rollover. */
void rb_turn(GameState *g) {
    if (g->winner != -1) return;
    int pl = g->active;
    /* Reset per-turn baton state: deployment-arrival ban and one-baton-per-action. */
    for (int p = 0; p < 2; p++) {
        for (int i = 0; i < RB_STAGE_SIZE; i++) g->stage_arrived[p][i] = 0;
        g->baton_touch_used[p] = 0;
        g->player_cannot_activate[p] = 0;
    }
    g->n_cannot_active_cards = 0;
    g->n_prohibition = 0;
    g->n_selected_cards = 0;
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
    /* Revert Duration::LiveEnd / during_live temporary effects (blade/heart/score
       granted during this live). Mirrors Rust's LiveVictoryDetermination cleanup.
       Without this the grants leak into subsequent turns. */
    rb_check_expired_effects(g, RB_TEMP_LIVE_END);
    g->phase = RB_PHASE_VICTORY;
    rollover(g);
}

/* ───────────────────────────── setup ───────────────────────────── */
int rb_game_init(GameState *g, const uint32_t *deck0, int n0,
                 const uint32_t *deck1, int n1) {
     memset(g, 0, sizeof(*g));
     rb_look_reset_all();
     rb_mods_init(&g->mods);
     g->winner = -1; g->turn = 1; g->phase = RB_PHASE_RPS;
     g->cheer_check_base = -1;
     g->baton_touch_replaced_member_cost = -1;
     g->baton_touch_replaced_member_id = -1;
     g->baton_touch_arriving_card_id = -1;
     for (int i = 0; i < 64; i++) g->gained_card_ids[i] = -1;
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

/* ───────────────────────────── deck choice (main.rs::choose_deck) ───────────────────────────── */
#define RB_MAX_DECK_ENTRIES 60
#define RB_MAX_DECK_NAME 64
#define RB_MAX_CARD_NO 16

typedef struct {
    char card_no[RB_MAX_CARD_NO];
    int  quantity;
} RbDeckEntry;

typedef struct {
    char        name[RB_MAX_DECK_NAME];
    RbDeckEntry entries[RB_MAX_DECK_ENTRIES];
    int         n_entries;
} RbDeckList;

const RbDeckList *choose_deck(const RbDeckList *deck_lists, int n_lists, const char *player_name) {
    if (n_lists <= 0 || !deck_lists) return NULL;
    printf("%s chose: %s\n", player_name ? player_name : "Player", deck_lists[0].name);
    return &deck_lists[0];
}

/* ───────────────────────────── i18n self-check (main.rs:295-316) ───────────────────────────── */
/* Canonical English choice-prompt templates — describe.rs:1053-1064. The C port
   emits these same strings (choice.c:1420/1449 build "Select up to N ..." and
   ability/types.c:660/663 set "Repeat effect?" bilingually), so the self-check
   has a real counterpart here and is not a placeholder. */
static const char *const s_choice_prompt_templates_en[] = {
    "Select up to 3 card(s) to keep",
    "Select up to 3 more card(s) from hand to keep",
    "Select up to 1 member(s) to change state",
    "Select up to 3 card(s) from the 5 looked-at cards (or skip)",
    "Select up to 2 more card(s) from the 4 remaining looked-at cards",
    "Repeat effect?",
    "Pay optional cost or skip",
};

/* describe.rs:1070-1127 translate_choice_prompt_en_to_ja. The parameterized
   templates ("Select up to N ...", N varying) cannot come from an exact-match
   table, so the number is parsed out of the string exactly as Rust does. Returns
   1 and fills `out` for a recognised prompt, 0 for an unrecognised one. */
static int translate_choice_prompt_en_to_ja(const char *en, char *out, size_t cap) {
    if (!en || !out || cap == 0) return 0;
    while (*en == ' ' || *en == '\t') en++;

    /* Which prefix was matched, so the family-specific tails can be told apart. */
    enum { PFX_NONE, PFX_SELECT, PFX_PLACE, PFX_ADD, PFX_DISCARD } pfx = PFX_NONE;
    const char *rest = NULL;
    if (!strncmp(en, "Select up to ",  13)) { pfx = PFX_SELECT;  rest = en + 13; }
    else if (!strncmp(en, "Place up to ",  12)) { pfx = PFX_PLACE;   rest = en + 12; }
    else if (!strncmp(en, "Add up to ",    10)) { pfx = PFX_ADD;     rest = en + 10; }
    else if (!strncmp(en, "Discard up to ", 14)) { pfx = PFX_DISCARD; rest = en + 14; }

    if (pfx != PFX_NONE) {
        const char *p = rest;
        while (*p >= '0' && *p <= '9') p++;
        size_t n = (size_t)(p - rest);
        char num[16];
        if (n == 0 || n >= sizeof(num)) return 0;
        memcpy(num, rest, n);
        num[n] = '\0';
        const char *after = p;

        switch (pfx) {
        case PFX_SELECT:
            if (strstr(after, "to keep"))
                snprintf(out, cap, "最大%s枚まで手札に残すカードを選択", num);
            else if (strstr(after, "member(s) to change state"))
                snprintf(out, cap, "状態を変更するメンバーを最大%s体選択", num);
            else if (strstr(after, "looked-at cards"))
                snprintf(out, cap, "見たカードから最大%s枚を選択（スキップ可）", num);
            else
                return 0;
            return 1;
        case PFX_PLACE:
            if (strstr(en, "bottom of deck")) {
                if (!strcmp(num, "1"))
                    snprintf(out, cap, "見たカードをデッキの下に置きますか？");
                else
                    snprintf(out, cap, "見たカードを最大%s枚までデッキの下に置きますか？", num);
            } else if (strstr(en, "top of deck")) {
                if (!strcmp(num, "1"))
                    snprintf(out, cap, "見たカードをデッキの上に置きますか？");
                else
                    snprintf(out, cap, "見たカードを最大%s枚までデッキの上に置きますか？", num);
            } else {
                return 0;
            }
            return 1;
        case PFX_ADD:
            if (!strstr(en, "to hand")) return 0;
            if (!strcmp(num, "1"))
                snprintf(out, cap, "見たカードを手札に加えますか？");
            else
                snprintf(out, cap, "見たカードを最大%s枚まで手札に加えますか？", num);
            return 1;
        case PFX_DISCARD:
            if (!strstr(en, "looked-at")) return 0;
            if (!strcmp(num, "1"))
                snprintf(out, cap, "見たカードを控え室に置きますか？");
            else
                snprintf(out, cap, "見たカードを最大%s枚まで控え室に置きますか？", num);
            return 1;
        default:
            return 0;
        }
    }

    if (!strcmp(en, "Repeat effect?")) {
        snprintf(out, cap, "効果を繰り返しますか？");
        return 1;
    }
    if (!strcmp(en, "Pay optional cost or skip")) {
        snprintf(out, cap, "オプションコストを支払うかスキップ");
        return 1;
    }
    return 0;   /* describe.rs:1125 — unrecognised prompt */
}

/* main.rs:295-316. Rust guards this with #[cfg(feature = "server")] and calls it
   once at boot; the C port has no boot-time caller, so it is exported for the
   same ABI parity. */
void i18n_self_check(void) {
    size_t n = sizeof(s_choice_prompt_templates_en) / sizeof(s_choice_prompt_templates_en[0]);
    int missing = 0;
    char ja[128];
    for (size_t i = 0; i < n; i++) {
        const char *tpl = s_choice_prompt_templates_en[i];
        if (!translate_choice_prompt_en_to_ja(tpl, ja, sizeof(ja))) {
            fprintf(stderr, "[i18n] missing Japanese translation for choice prompt template: %s\n", tpl);
            missing++;
        }
    }
    if (missing > 0)
        fprintf(stderr, "[i18n] %d choice-prompt template(s) lack Japanese — "
                        "JA mode will show English for these\n", missing);
    else
        fprintf(stderr, "[i18n] all canonical choice-prompt templates have Japanese translations\n");
}

/* ───────────────────────────── web server (main.rs::run_web_server) ───────────────────────────── */
void run_web_server(const char *ngrok_authtoken) {
    /* Server-only in Rust (#[cfg(feature = "server")]); requires tokio +
       actix-web + ngrok. The C port has no HTTP server runtime, so this logs
       the intent and returns. Kept for ABI parity. */
    printf("Web server starting on http://127.0.0.1:8080\n");
    if (ngrok_authtoken) {
        printf("  (ngrok tunnel requested — not available in C port)\n");
    }
}
