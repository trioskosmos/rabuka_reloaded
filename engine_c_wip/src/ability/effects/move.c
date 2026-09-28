#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>

/* Forward declarations for functions added during move.rs port */
void rb_move_prompt_card_selection(GameState *g, int actor, const char *zone,
                                    int count, int can_skip, AbilityEffect *e);
int rb_move_take_cards_from_standard_zone(GameState *g, int actor,
                                           const char *zone_name,
                                           AbilityEffect *e,
                                           int count, int is_all,
                                           int can_skip, int *out_ids, int max);
int rb_move_resolve_cards_from_source(GameState *g, int actor, AbilityEffect *e,
                                        int count, int *out_ids, int max);
int rb_move_resolve_from_zone(GameState *g, int actor, const char *effective_source,
                                AbilityEffect *e, int use_p2, int count,
                                int *out_ids, int max);
int rb_move_resolve_from_recently_moved(GameState *g, int use_p2,
                                         const char *card_type_filter,
                                         const char *group_name,
                                         int *out_ids, int max);
int rb_move_resolve_from_energy_deck(GameState *g, int pl, int count, int *out_ids, int max);
int rb_move_resolve_from_stage(GameState *g, int actor, AbilityEffect *e, int use_p2,
                                int count, int *out_ids, int max);
int rb_move_resolve_from_under_member(GameState *g, int actor, AbilityEffect *e, int use_p2,
                                       int count, int *out_ids, int max);
int rb_move_resolve_from_deck_bottom(GameState *g, int actor, const AbilityEffect *e, int use_p2,
                                      int count, const char *card_type_filter,
                                      const char *group_name, int *out_ids, int max);
int rb_move_resolve_source_looked_at(GameState *g, int actor, AbilityEffect *e, int use_p2,
                                      int count, int *out_ids, int max);
int rb_move_from_revealed(GameState *g, int actor, const int *indices, int n_indices,
                           int (*validate_card)(int), const char *dst,
                           int *out_ids, int max);
void rb_move_execute_selected_cards_from_zone(GameState *g, int actor, const char *zone,
                                               const int *indices, int n_indices,
                                               const char *card_type_filter,
                                               int cost_limit, const char *cost_limit_op,
                                               int cost_total, const char *cost_total_op,
                                               const char *group,
                                               const char **characters, int n_characters,
                                               const char *target_player_id);
void rb_move_handle_select_cards_looked_at(GameState *g, int actor, const int *indices,
                                            int n_indices, const char *ctx_destination,
                                            int ctx_discard_remaining);
void rb_move_handle_energy_zone_selection(GameState *g, int actor, const int *indices,
                                           int n_indices, int count,
                                           const char *destination,
                                           int (*validate_card)(int));
void rb_move_finalize_card_movement(GameState *g, int actor,
                                    const int *moved_cards, int n_moved,
                                    const char *destination, const char *source,
                                    const char *state_change, const char *target);
void rb_move_fire_debut_side_effects(GameState *g, int actor, int card_id,
                                     const char *target, const char *source);
int rb_move_execute_stage_placement_choices(GameState *g, int actor,
                                            const int *card_ids, int n_ids,
                                            const char *src_zone,
                                            const char *dest,
                                            int vacated_area,
                                            const char *target,
                                            int *out_ids, int max);
void rb_move_handle_select_position(GameState *g, int actor, const char *position,
                                     int card_id, const char *target,
                                     const char *source_zone, const char *state_change);
int rb_move_place_card_with_stage_choice(GameState *g, int actor, int host_cid,
                                            const char *player_target, int card_id,
                                            const char *destination, int vacated_area,
                                            int is_max, int count, const char *state_change,
                                            int deck_position, const char *source_zone,
                                            int allow_occupied_stage, int under_self);
void rb_move_execute_move_cards_ported(GameState *g, int actor, AbilityEffect *e);
/* Defined in src/core/modifiers.c; not declared in rabuka.h. */
void rb_on_cards_left_zones(GameState *g, int card_id);


/* ── card_matches_filter: full effect filter (static for this TU) ── */
static const char *cmf_extra(const AbilityEffect *e, const char *k){
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],k)) return e->extra_v[i];
    return NULL;
}
static int cmf_cmp(const char *op, int a, int b){
    if(!op||!*op) return a==b;
    if(!strcmp(op,">=")) return a>=b;
    if(!strcmp(op,"<=")) return a<=b;
    if(!strcmp(op,">"))  return a>b;
    if(!strcmp(op,"<"))  return a<b;
    if(!strcmp(op,"==")||!strcmp(op,"=")) return a==b;
    if(!strcmp(op,"!=")) return a!=b;
    return a==b;
}
static int cmf_has_heart(const Card *c, int hc){
    if(hc<0||hc>7) return 0;
    for(int k=0;k<c->n_hearts;k++) if(c->heart_color[k]==(uint8_t)hc) return 1;
    return 0;
}
static int card_matches_filter(int card_idx, AbilityEffect *e){
    const char *ctype = cmf_extra(e,"card_type");
    if(!ctype && e->card_type_field[0]) ctype = e->card_type_field;
    if(ctype && !card_matches_card_type_filter(card_idx, ctype)) return 0;
    const char *gn = cmf_extra(e,"group_names");
    if(gn && !rb_card_matches_group_str(card_idx, gn)) return 0;
    const char *cnames = cmf_extra(e,"card_names");
    if(cnames){
        Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int m = c.name && strstr(c.name, cnames); rb_free_card(&c);
        if(!m) return 0;
    }
    const char *nf = cmf_extra(e,"name_fragments");
    if(nf){
        Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int m = c.name && strstr(c.name, nf); rb_free_card(&c);
        if(!m) return 0;
    }
    const char *cl = cmf_extra(e,"cost_limit");
    if(cl){
        int v=atoi(cl); const char *op=cmf_extra(e,"cost_operator");
        Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int ok=cmf_cmp(op,(int)c.cost,v); rb_free_card(&c); if(!ok) return 0;
    }
    const char *cv = cmf_extra(e,"cost_values");
    if(cv){
        int v=atoi(cv); Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int ok=(c.cost==v)||(c.score==v); rb_free_card(&c); if(!ok) return 0;
    }
    const char *clmin = cmf_extra(e,"cost_limit_min");
    if(clmin){
        int v=atoi(clmin); Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int ok=cmf_cmp(">=",(int)c.cost,v); rb_free_card(&c); if(!ok) return 0;
    }
    const char *clmax = cmf_extra(e,"cost_limit_max");
    if(clmax){
        int v=atoi(clmax); Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int ok=cmf_cmp("<=",(int)c.cost,v); rb_free_card(&c); if(!ok) return 0;
    }
    const char *ct = cmf_extra(e,"cost_total");
    if(ct){
        int v=atoi(ct); const char *op=cmf_extra(e,"cost_total_operator");
        Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int ok=cmf_cmp(op,(int)c.cost,v); rb_free_card(&c); if(!ok) return 0;
    }
    const char *ob = cmf_extra(e,"original_blade_limit");
    if(ob){
        int v=atoi(ob); const char *op=cmf_extra(e,"original_blade_operator");
        Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int ok=cmf_cmp(op,(int)c.blade,v); rb_free_card(&c); if(!ok) return 0;
    }
    const char *hc = cmf_extra(e,"heart_color");
    if(hc){
        int col = (int)rb_parse_heart_color(hc);
        Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int ok = (col==(int)RB_HEART_ANY && c.n_hearts>0) || cmf_has_heart(&c, col);
        rb_free_card(&c); if(!ok) return 0;
    }
    const char *chars = cmf_extra(e,"characters");
    if(chars && !rb_card_matches_group_str(card_idx, chars)) return 0;
    const char *exch = cmf_extra(e,"exclude_characters");
    if(exch && rb_card_matches_group_str(card_idx, exch)) return 0;
    const char *cp = cmf_extra(e,"card_property");
    if(cp){
        const char *ng = cmf_extra(e,"negation");
        int neg = ng && !strcmp(ng,"true");
        Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int has=0;
        if(!strcmp(cp,"has_blade_heart"))      has = rb_card_has_blade_heart(&c);
        else if(!strcmp(cp,"has_score_icon"))  has = rb_card_has_score_icon(&c);
        else if(!strcmp(cp,"has_all_blade"))   has = rb_card_has_all_blade(&c);
        if(neg) has=!has;
        rb_free_card(&c);
        if(!has) return 0;
    }
    return 1;
}

/* ── zone helpers ── */
static int stage_area_of(RbPlayer *P, int cid){
    for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]==cid) return q;
    return -1;
}
static int find_and_remove_card(RbPlayer *P, int cid){
    for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]==cid){ P->stage[q]=-1; P->stage_wait[q]=0; return 1; }
    for(int i=0;i<P->hand.n;i++) if(P->hand.cards[i]==cid){ for(int k=i;k<P->hand.n-1;k++) P->hand.cards[k]=P->hand.cards[k+1]; P->hand.n--; return 1; }
    for(int i=0;i<P->deck.n;i++) if(P->deck.cards[i]==cid){ for(int k=i;k<P->deck.n-1;k++) P->deck.cards[k]=P->deck.cards[k+1]; P->deck.n--; return 1; }
    for(int i=0;i<P->discard.n;i++) if(P->discard.cards[i]==cid){ for(int k=i;k<P->discard.n-1;k++) P->discard.cards[k]=P->discard.cards[k+1]; P->discard.n--; return 1; }
    for(int i=0;i<P->energy.n;i++) if(P->energy.cards[i]==cid){ for(int k=i;k<P->energy.n-1;k++) P->energy.cards[k]=P->energy.cards[k+1]; P->energy.n--; return 1; }
    for(int i=0;i<P->live.n;i++) if(P->live.cards[i]==cid){ for(int k=i;k<P->live.n-1;k++) P->live.cards[k]=P->live.cards[k+1]; P->live.n--; return 1; }
    for(int i=0;i<P->success.n;i++) if(P->success.cards[i]==cid){ for(int k=i;k<P->success.n-1;k++) P->success.cards[k]=P->success.cards[k+1]; P->success.n--; return 1; }
    return 0;
}
static void place_in_dst(RbPlayer *A, RbZone dst, int cid, int to_top, int to_bottom,
                         int dst_area, int under, int dst_stage){
    if(dst_stage || dst==RB_ZONE_STAGE){
        int area = dst_area;
        if(area<0){ for(int q=0;q<RB_STAGE_SIZE;q++) if(A->stage[q]<0){ area=q; break; } }
        if(area<0 || area>=RB_STAGE_SIZE) return;
        if(under){ if(A->under_cards[area].n < RB_MAX_ZONE) A->under_cards[area].cards[A->under_cards[area].n++]=cid; }
        else { A->stage[area]=cid; A->stage_wait[area]=0; }
        return;
    }
    RbBag *db=NULL;
    if(dst==RB_ZONE_HAND) db=&A->hand; else if(dst==RB_ZONE_DECK) db=&A->deck;
    else if(dst==RB_ZONE_DISCARD) db=&A->discard; else if(dst==RB_ZONE_ENERGY) db=&A->energy;
    else if(dst==RB_ZONE_LIVE) db=&A->live; else if(dst==RB_ZONE_SUCCESS) db=&A->success;
    if(!db) return;
    if(db->n < RB_MAX_ZONE){
        if(to_bottom && dst==RB_ZONE_DECK){ db->cards[db->n++]=cid; }
        else if(to_top && dst==RB_ZONE_DECK){ for(int k=db->n;k>0;k--) db->cards[k]=db->cards[k-1]; db->cards[0]=cid; db->n++; }
        else db->cards[db->n++]=cid;
    }
}

/* ── drain_under_cards_to_energy_zone ── */
int rb_drain_under_cards_to_energy_zone(GameState *g, const char *target, int stage_idx) {
    if (!g || stage_idx < 0 || stage_idx >= RB_STAGE_SIZE) return 0;
    int pl = 0;
    if (target && *target) { int t = rb_resolve_target_player(g, target); if (t >= 0) pl = t; }
    RbPlayer *P = &g->p[pl];
    int n = P->under_cards[stage_idx].n;
    int moved = 0;
    for (int i = 0; i < n; i++) {
        int cid = P->under_cards[stage_idx].cards[i];
        if (rb_card_is_energy(cid)) {
            if (P->energy.n < RB_MAX_ZONE) P->energy.cards[P->energy.n++] = cid;
            rb_mods_set_orientation(&g->mods, cid, "wait");
        } else {
            if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = cid;
        }
        moved++;
    }
    P->under_cards[stage_idx].n = 0;
    rb_recalc_constants(g);
    return moved;
}

/* ── remove_card_from_any_zone ── */
static void remove_card_from_any_zone(RbPlayer *P, int *last_vacated, int cid) {
    int i;
    for (i = 0; i < P->hand.n; i++) if (P->hand.cards[i] == cid) {
        for (int k = i; k < P->hand.n - 1; k++) P->hand.cards[k] = P->hand.cards[k + 1];
        P->hand.n--; return;
    }
    for (i = 0; i < P->discard.n; i++) if (P->discard.cards[i] == cid) {
        for (int k = i; k < P->discard.n - 1; k++) P->discard.cards[k] = P->discard.cards[k + 1];
        P->discard.n--; return;
    }
    for (i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] == cid) {
        P->stage[i] = -1; P->stage_wait[i] = 0;
        if (last_vacated) *last_vacated = i;
        return;
    }
    for (i = 0; i < P->energy.n; i++) if (P->energy.cards[i] == cid) {
        for (int k = i; k < P->energy.n - 1; k++) P->energy.cards[k] = P->energy.cards[k + 1];
        P->energy.n--; return;
    }
    for (i = 0; i < P->live.n; i++) if (P->live.cards[i] == cid) {
        for (int k = i; k < P->live.n - 1; k++) P->live.cards[k] = P->live.cards[k + 1];
        P->live.n--; return;
    }
}

/* ── player_mut helper ── */
static RbPlayer *mc_player_mut(GameState *g, int use_p2) {
    return use_p2 ? &g->p[1] : &g->p[0];
}

/* ── ring-buffer record movement ── */
static void mc_record_movement(GameState *g, int cid){
    if(cid < 0) return;
    if(g->n_recently_moved < RB_MAX_RECENTLY_MOVED) g->recently_moved[g->n_recently_moved++]=cid;
    else { for(int i=1;i<RB_MAX_RECENTLY_MOVED;i++) g->recently_moved[i-1]=g->recently_moved[i]; g->recently_moved[RB_MAX_RECENTLY_MOVED-1]=cid; }
}

/* ── resolve_cost_limit_reference ── */
int rb_move_resolve_cost_limit_reference(const GameState *g, const AbilityEffect *e) {
    if (!g || !e) return -1;
    const char *reference = cmf_extra(e, "cost_reference");
    if (!reference) {
        const char *cl = cmf_extra(e, "cost_limit");
        return cl ? atoi(cl) : -1;
    }
    const char *offset_str = cmf_extra(e, "cost_offset");
    int offset = offset_str ? atoi(offset_str) : 0;
    int referenced_id = -1;
    if (!strcmp(reference, "previous_moved_card")) {
        if (g->n_recently_moved > 0)
            referenced_id = g->recently_moved[g->n_recently_moved - 1];
    }
    if (referenced_id < 0) return -1;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)referenced_id, &c)) return -1;
    int base_cost = (int)c.cost;
    rb_free_card(&c);
    int resolved = base_cost + offset;
    if (resolved < 0) resolved = 0;
    if (resolved > 255) resolved = 255;
    return resolved;
}

int rb_move_looked_at_matches(GameState *g, int cid, AbilityEffect *e) {
    if (!g || !e || cid < 0) return 0;
    if (e->card_type_field[0] && !rb_card_matches_type(cid, e->card_type_field)) return 0;
    AbilityEffect filter = *e;
    filter.n_extra = 0;
    for (int i = 0; i < e->n_extra; i++) {
        const char *key = e->extra_k[i];
        if (key && (!strcmp(key, "cost_limit") || !strcmp(key, "cost_total") ||
                    !strcmp(key, "characters") || !strcmp(key, "exclude_characters"))) continue;
        filter.extra_k[filter.n_extra] = e->extra_k[i];
        filter.extra_v[filter.n_extra++] = e->extra_v[i];
    }
    if (!card_matches_filter(cid, &filter)) return 0;
    const char *chars = cmf_extra(e, "characters");
    if (chars && !rb_card_matches_characters(cid, &chars, 1)) return 0;
    const char *excluded = cmf_extra(e, "exclude_characters");
    if (excluded && rb_card_matches_characters(cid, &excluded, 1)) return 0;
    int limit = g->queue.resume_mode == 6 && g->queue.resume_eff == e ?
                g->queue.pending.cost_limit : rb_move_resolve_cost_limit_reference(g, e);
    const char *op = cmf_extra(e, "cost_limit_operator");
    if (!op) op = cmf_extra(e, "cost_operator");
    return limit < 0 || rb_card_matches_cost_limit(cid, limit, op ? op : "<=");
}

/* ── zone_label helper ── */
static const char *move_zone_label(const char *zone) {
    if (!zone) return "unknown";
    if (!strcmp(zone, "hand")) return "hand";
    if (!strcmp(zone, "discard")) return "the waiting room";
    if (!strcmp(zone, "deck")) return "deck";
    if (!strcmp(zone, "deck_top")) return "top of deck";
    if (!strcmp(zone, "deck_bottom")) return "bottom of deck";
    if (!strcmp(zone, "stage")) return "stage";
    if (!strcmp(zone, "energy")) return "energy";
    if (!strcmp(zone, "energy_deck")) return "energy deck";
    if (!strcmp(zone, "energy_zone")) return "energy zone";
    if (!strcmp(zone, "waitroom")) return "wait room";
    if (!strcmp(zone, "success_zone")) return "success zone";
    if (!strcmp(zone, "live_card_zone")) return "live card zone";
    if (!strcmp(zone, "under_member")) return "under this member";
    if (!strcmp(zone, "revealed_cards")) return "revealed cards";
    if (!strcmp(zone, "those_cards")) return "those cards";
    if (!strcmp(zone, "recently_moved")) return "recently moved";
    if (!strcmp(zone, "looked_at")) return "looked at";
    if (!strcmp(zone, "all_selected")) return "selected cards";
    return zone;
}

/* ── prompt_carry_resume_context ──
 * Port of the tail of Rust AbilityResolver::prompt_card_selection
 * (engine/src/ability/move_cards/selection.rs:31-49).
 *
 * The prompt is not self-contained: the ANSWER carries indices, and the answer
 * handler has to map them back through the SAME zone + filter the prompt was
 * built from, and then move them to the effect's destination. Rust parks all of
 * that on the Choice itself:
 *     .card_type(..) .cost_limit(..) .cost_total(..) .group(..)
 *     .characters(..) .target_player_id(..) .filtered_indices(..)
 *     .destination(effect.destination) .discard_remaining(..)       (:34-46)
 * and then `self.in_flight.execution_context =
 *      ExecutionContext::SingleEffect { effect_index: 0 }`              (:49)
 * so the resuming handler still knows WHICH effect asked.
 *
 * The C prompt carried only card_type / group_names / target, and parked a
 * resume context for `looked_at` ONLY. So a `hand` or `discard` SelectCard
 * answer arrived with:
 *   - no destination. `rb_resolver_handle_discard_selection` (choice.c:2271
 *     and :2315) resolves it as
 *     `resume_move_destination ? : pending.target ? : "hand"`, and
 *     `pending.target` holds the SEAT ("self" / "opponent"), never a zone, so
 *     the picked card was pushed into a zone literally named "self";
 *   - no cost_limit / cost_total / characters, so the answer could not re-apply
 *     the prompt's own filter;
 *   - no resume context, so `queue.resume_eff` stayed NULL and the
 *     `g->queue.resume_eff->destination` fallback those two lines already
 *     guard on could never fire, and the producing effect could not be re-read.
 * That is why only the looked_at family ever completed.
 *
 * `resume_mode` deliberately gets NO new value here. The C dispatch in
 * rb_resume_with_choice_indices_internal (choice.c:3205-3336) routes on it, and
 * every existing arm is a DIFFERENT Rust handler (look-resume, position
 * change, deferred body, optional gate, keep-shuffle, looked_at); minting an arm
 * for a zone that Rust answers in the generic SelectCard handler would be a
 * parallel architecture, not a port. `resume_eff` / `resume_actor` are the C
 * spelling of `ExecutionContext::SingleEffect { effect_index: 0 }` — the C
 * resume path already carries and reads them (choice.c:1677, :2273, :2317);
 * they simply have to be set for a hand / discard prompt as well. */
static void rb_move_prompt_carry_resume_context(GameState *g, int actor,
                                                const char *zone, AbilityEffect *e) {
    if (!g) return;
    if (e) {
        /* `.destination(effect.destination)` (selection.rs:45). RbChoice has no
           destination field, so the C slot for it is
           `queue.resume_move_destination`, which is what the answer handlers
           read (choice.c:2277, :2321). Rust's `Choice.destination` is an
           Option and is per-choice: absent means None, and
           execute_selected_cards_from_zone then falls back to "discard"
           (move_cards.rs:2573-2583). The C slot is a single sticky queue field
           shared with the live / success-replacement producers (live.c:1025,
           choice.c:2449), so it is written UNCONDITIONALLY here — leaving a
           previous prompt's value in place would give this choice a
           destination its own effect never named. */
        if (e->destination && e->destination[0])
            strncpy(g->queue.resume_move_destination, e->destination,
                    sizeof(g->queue.resume_move_destination) - 1);
        else
            g->queue.resume_move_destination[0] = '\0';
        /* `.cost_limit(filter.cost_limit, effect.cost_limit_operator_any())` and
           `.cost_total(filter.cost_total, effect.cost_total_operator_any())`
           (selection.rs:35-36). */
        const char *cl = cmf_extra(e, "cost_limit");
        if (cl) {
            g->queue.pending.cost_limit = atoi(cl);
            const char *clo = cmf_extra(e, "cost_limit_operator");
            if (!clo) clo = cmf_extra(e, "cost_operator");
            if (clo)
                strncpy(g->queue.pending.cost_limit_op, clo,
                        sizeof(g->queue.pending.cost_limit_op) - 1);
        }
        const char *ct = cmf_extra(e, "cost_total");
        if (ct) {
            g->queue.pending.cost_total = atoi(ct);
            const char *cto = cmf_extra(e, "cost_total_operator");
            if (cto)
                strncpy(g->queue.pending.cost_total_op, cto,
                        sizeof(g->queue.pending.cost_total_op) - 1);
        }
        /* `.characters(filter.characters)` (selection.rs:38) — same
           comma-splitting idiom as cost.c:660-668. */
        const char *chars = cmf_extra(e, "characters");
        if (chars && chars[0] && g->queue.pending.n_characters == 0) {
            char cbuf[256];
            strncpy(cbuf, chars, sizeof(cbuf) - 1);
            cbuf[sizeof(cbuf) - 1] = '\0';
            char *tok = strtok(cbuf, ",");
            while (tok && g->queue.pending.n_characters < 16) {
                strncpy(g->queue.pending.characters[g->queue.pending.n_characters], tok,
                        sizeof(g->queue.pending.characters[0]) - 1);
                g->queue.pending.n_characters++;
                tok = strtok(NULL, ",");
            }
        }
    }
    /* `looked_at` keeps mode 6: that arm is rb_move_handle_select_cards_looked_at
       (choice.c:3205 -> move.c), the C twin of Rust's handle_looked_at_selection
       (choice.rs:1082). Every other zone this prompt is built for is answered by
       the generic SelectCard dispatch, which reads exactly the resume_eff /
       resume_actor / destination parked just above. */
    g->queue.resume_eff = e;
    g->queue.resume_actor = actor;
    if (!strcmp(zone, "looked_at")) g->queue.resume_mode = 6;
}

/* ── prompt_card_selection ── */
void rb_move_prompt_card_selection(GameState *g, int actor, const char *zone,
                                    int count, int can_skip, AbilityEffect *e) {
    const char *zlabel = move_zone_label(zone);
    const char *any_num_str = cmf_extra(e, "any_number");
    int any_number = any_num_str && (!strcmp(any_num_str, "true") || !strcmp(any_num_str, "1"));
    char desc[128];
    if (any_number)
        snprintf(desc, sizeof(desc), "Select any number of card(s) from %s", zlabel);
    else
        snprintf(desc, sizeof(desc), "Select %d card(s) from %s", count, zlabel);
    const char *card_type = cmf_extra(e, "card_type");
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, zone, card_type, count, can_skip, NULL);
    rb_queue_pause_for_choice(g, &g->queue.pending);
    rb_choice_set_description(&g->queue.pending, desc);
    const char *group = cmf_extra(e, "group_names");
    if (group)
        strncpy(g->queue.pending.filter_group, group, sizeof(g->queue.pending.filter_group) - 1);
    if (e->target) {
        strncpy(g->queue.pending.target, e->target, sizeof(g->queue.pending.target) - 1);
        strncpy(g->queue.pending.target_player_id, e->target, sizeof(g->queue.pending.target_player_id) - 1);
    }
    rb_move_prompt_carry_resume_context(g, actor, zone, e);
    rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
}

/* ── optional_gate_source ── */
int rb_move_optional_gate_source(const char *zone_str) {
    if (!zone_str) return 0;
    return (!strcmp(zone_str, "deck") || !strcmp(zone_str, "deck_top") ||
            !strcmp(zone_str, "deck_bottom") || !strcmp(zone_str, "energy_deck") ||
            !strcmp(zone_str, "energy")) ? 1 : 0;
}

/* ── gate_optional_source ── */
int rb_move_gate_optional_source(GameState *g, int actor, const AbilityEffect *e,
                                  const char *zone_str, int count) {
    if (!g || !e || !zone_str) return 0;
    if (!e->is_optional) return 0;
    if (!rb_move_optional_gate_source(zone_str)) return 0;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, count > 0 ? count : 1, 1, "pay_optional_cost");
    rb_queue_pause_for_choice(g, &g->queue.pending);
    rb_choice_set_route(&g->queue.pending, RB_ROUTE_OPTIONAL_COST);
    return 1;
}

/* ── resolve_from_deck ── */
int rb_move_resolve_from_deck(GameState *g, int actor, const AbilityEffect *e,
                               int use_p2, int count,
                               const char *card_type_filter, const char *group_name,
                               int *out_ids, int max) {
    if (!g || count <= 0 || !out_ids) return 0;
    if (rb_move_gate_optional_source(g, actor, e, "deck", count)) return 0;
    RbPlayer *P = mc_player_mut(g, use_p2);
    int drawn = 0;
    int attempts = 0;
    int remaining = count;
    int cap = count + P->deck.n + 10;
    while (remaining > 0 && attempts < cap) {
        if (P->deck.n == 0) {
            if (P->discard.n > 0) {
                rb_shuffle(P->discard.cards, P->discard.n);
                for (int k = 0; k < P->discard.n && P->deck.n < RB_MAX_ZONE; k++)
                    P->deck.cards[P->deck.n++] = P->discard.cards[k];
                P->discard.n = 0;
                P->deck_refreshed_this_turn = 1;
            } else break;
        }
        if (P->deck.n == 0) break;
        int cid = P->deck.cards[--P->deck.n];
        attempts++;
        if (card_type_filter && !rb_card_matches_type(cid, card_type_filter)) {
            if (P->deck.n < RB_MAX_ZONE) {
                for (int k = P->deck.n; k > 0; k--) P->deck.cards[k] = P->deck.cards[k - 1];
                P->deck.cards[0] = cid;
                P->deck.n++;
            }
            continue;
        }
        if (group_name && !rb_card_matches_group_str(cid, group_name)) {
            if (P->deck.n < RB_MAX_ZONE) {
                for (int k = P->deck.n; k > 0; k--) P->deck.cards[k] = P->deck.cards[k - 1];
                P->deck.cards[0] = cid;
                P->deck.n++;
            }
            continue;
        }
        if (drawn < max) out_ids[drawn++] = cid;
        remaining--;
    }
    return drawn;
}

/* ── resolve_from_deck_bottom ── */
int rb_move_resolve_from_deck_bottom(GameState *g, int actor, const AbilityEffect *e, int use_p2,
                                      int count, const char *card_type_filter,
                                      const char *group_name, int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    int pl = use_p2 ? 1 : 0;
    RbPlayer *P = &g->p[pl];
    int n = 0;
    while (n < count && P->deck.n > 0) {
        int card = P->deck.cards[P->deck.n - 1];
        P->deck.n--;
        if (card_type_filter && !rb_card_matches_type(card, card_type_filter)) continue;
        if (group_name && !rb_card_matches_group_str(card, group_name)) continue;
        if (n < max) out_ids[n++] = card;
    }
    return n;
}

/* ── resolve_from_energy_deck ── */
int rb_move_resolve_from_energy_deck(GameState *g, int pl, int count, int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    RbPlayer *P = &g->p[pl];
    int n = 0;
    while (n < count && P->energy_deck.n > 0) {
        int card = P->energy_deck.cards[0];
        for (int i = 0; i < P->energy_deck.n - 1; i++) P->energy_deck.cards[i] = P->energy_deck.cards[i + 1];
        P->energy_deck.n--;
        if (n < max) out_ids[n++] = card;
    }
    return n;
}

/* ── resolve_from_stage ── */
int rb_move_resolve_from_stage(GameState *g, int actor, AbilityEffect *e, int use_p2,
                                int count, int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    int pl = use_p2 ? 1 : actor;
    RbPlayer *P = &g->p[pl];
    const char *ctype = cmf_extra(e, "card_type");
    const char *gn = cmf_extra(e, "group_names");
    int n = 0;
    int wanted = count < 0 ? RB_STAGE_SIZE : count;
    for (int area = 0; area < RB_STAGE_SIZE && n < wanted; area++) {
        int cid = P->stage[area];
        if (cid < 0) continue;
        if (ctype && !rb_card_matches_type(cid, ctype)) continue;
        if (gn && !rb_card_matches_group_str(cid, gn)) continue;
        out_ids[n++] = cid;
        P->stage[area] = -1;
        P->stage_wait[area] = 0;
        g->baton_last_vacated_area[pl] = area;
    }
    return n;
}

/* ── resolve_from_recently_moved ── */
int rb_move_resolve_from_recently_moved(GameState *g, int use_p2,
                                         const char *card_type_filter,
                                         const char *group_name,
                                         int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    int pl = use_p2 ? 1 : 0;
    RbPlayer *P = &g->p[pl];
    int n = 0;
    for (int i = 0; i < g->n_recently_moved && n < max; i++) {
        int cid = g->recently_moved[i];
        if (card_type_filter && !rb_card_matches_type(cid, card_type_filter)) continue;
        if (group_name && !rb_card_matches_group_str(cid, group_name)) continue;
        out_ids[n++] = cid;
    }
    for (int i = 0; i < n; i++) {
        int last_vacated = -1;
        remove_card_from_any_zone(P, &last_vacated, out_ids[i]);
        if (last_vacated >= 0)
            g->baton_last_vacated_area[pl] = last_vacated;
    }
    if (n > 0) rb_recalc_constants(g);
    return n;
}

/* ── resolve_from_looked_at ── */
int rb_move_resolve_from_looked_at(GameState *g, int use_p2, int *out_ids, int max, int *out_count) {
    if (!g || !out_ids || !out_count || max <= 0) return -1;
    *out_count = 0;
    int pl = use_p2 ? 1 : 0;
    int ids[RB_MAX_ZONE];
    int n = rb_looked_at_pool(pl, ids, RB_MAX_ZONE);
    for (int i = 0; i < n && *out_count < max; i++)
        if (rb_look_remove(pl, ids[i])) out_ids[(*out_count)++] = ids[i];
    return 0;
}

/* ── resolve_from_under_member ──
   Rust: engine/src/ability/move_cards.rs:1469-1580 (AbilityResolver::resolve_from_under_member).
   Collects the stage slots whose under-cards contain at least one energy card; drains
   the only candidate straight to the energy zone, or presents the stage SelectCard
   choice when several slots qualify (or the effect is optional, so a single candidate
   must still be skippable). */
int rb_move_resolve_from_under_member(GameState *g, int actor, AbilityEffect *e, int use_p2,
                                       int count, int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    int pl = use_p2 ? 1 : actor;
    if (pl < 0 || pl > 1) pl = actor;
    RbPlayer *P = &g->p[pl];
    const char *target = (e && e->target && e->target[0]) ? e->target : "self";
    int is_optional = e && e->is_optional;

    int candidates[RB_STAGE_SIZE];
    int n_cand = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        RbBag *under = &P->under_cards[i];
        if (under->n == 0) continue;
        int has_energy = 0;
        for (int k = 0; k < under->n; k++)
            if (rb_card_is_energy(under->cards[k])) { has_energy = 1; break; }
        if (has_energy) candidates[n_cand++] = i;
    }
    if (n_cand == 0) return 0;

    int chosen = -1;
    if (n_cand == 1 && !is_optional) {
        chosen = candidates[0];
    } else {
        /* 「Choose a member whose under energies to move」 — same Choice shape as the
           under-member host prompt in place_card_with_stage_choice. */
        rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "stage", "member_card",
                       1, is_optional, NULL);
        rb_queue_pause_for_choice(g, &g->queue.pending);
        rb_choice_set_description(&g->queue.pending,
                                  "Choose a member whose under energies to move");
        rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
        g->queue.pending.n_filtered_indices = n_cand;
        for (int i = 0; i < n_cand; i++) g->queue.pending.filtered_indices[i] = candidates[i];
        strncpy(g->queue.pending.target_player_id, target,
                sizeof(g->queue.pending.target_player_id) - 1);
        g->queue.resume_mode = 1;
        g->queue.resume_actor = actor;
        g->queue.resume_draw_target = pl;
        return 0;
    }
    if (chosen < 0) return 0;

    int moved[RB_MAX_ZONE];
    int n_moved = 0;
    RbBag *under = &P->under_cards[chosen];
    for (int k = 0; k < under->n && n_moved < RB_MAX_ZONE; k++)
        moved[n_moved++] = under->cards[k];
    /* Resolve explicitly: "self" maps to player 1 in this port, so name `pl` directly. */
    if (rb_drain_under_cards_to_energy_zone(g, pl == 0 ? "player1" : "player2", chosen) <= 0)
        return 0;
    int n_out = 0;
    for (int i = 0; i < n_moved && n_out < max; i++) {
        out_ids[n_out] = moved[i];
        mc_record_movement(g, moved[i]);
        n_out++;
    }
    return n_out;
}

/* ── prompt_card_selection_with_filtered ─────────────────────────────────────
 * Rust take_cards_from_standard_zone (move_cards/selection.rs:66-87) builds
 * `filtered_indices` from the zone and the filter ONCE, classifies with that
 * same list, and hands it to prompt_card_selection (:85
 * `.filtered_indices(filtered_indices)`). A returned index is therefore an index
 * into the candidate list the player was SHOWN, and the answer maps it back
 * through `ctx.mfi` (choice.rs:51-55, applied at :1172
 * `let mapped_indices = ctx.mfi(&ctx.indices)`) — the C equivalent is
 * map_choice_index (choice.c:944-950), which reads
 * `queue.pending.filtered_indices` / `n_filtered_indices`.
 *
 * The C prompt never received that list, so map_choice_index was the identity
 * and every answer was read as a RAW ZONE index. That is only accidentally
 * right when all of the zone's cards match the filter: under a card_type /
 * group_names filter that rejects cards, the player picks from the displayed
 * candidate list and the engine would lift the wrong card. This wrapper keeps
 * the two lists identical by construction — `idxs` is the very array the
 * classification just ran on.
 *
 * One deliberate difference from Rust, documented: Rust's `filtered_indices` is
 * `util::matching_indices` WITHOUT the self-target pinning that the same call's
 * `resolve_selection` applies (selection.rs:72 vs :74-75). The C `idxs` is
 * `rb_get_selection_indices(.., self_target_only, ..)`, which DOES pin. The C
 * list is the strictly tighter one, and it is the one the classification above
 * was decided on, so it is the list the prompt actually offered. */
static void rb_move_prompt_card_selection_with_filtered(
    GameState *g, int actor, const char *zone, int count, int can_skip,
    AbilityEffect *e, const int *idxs, int n_idxs) {
    rb_move_prompt_card_selection(g, actor, zone, count, can_skip, e);
    if (!g || !idxs || n_idxs <= 0) return;
    if (n_idxs > RB_MAX_ZONE) n_idxs = RB_MAX_ZONE;
    g->queue.pending.n_filtered_indices = n_idxs;
    for (int i = 0; i < n_idxs; i++) g->queue.pending.filtered_indices[i] = idxs[i];
}

/* ── take_cards_from_standard_zone ── */
int rb_move_take_cards_from_standard_zone(GameState *g, int actor,
                                           const char *zone_name,
                                           AbilityEffect *e,
                                           int count, int is_all,
                                           int can_skip, int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    int pl = actor;
    if (e->target && !strcmp(e->target, "opponent")) pl = actor ^ 1;
    const char *ctype = cmf_extra(e, "card_type");
    const char *gn = cmf_extra(e, "group_names");
    int cards[RB_MAX_ZONE];
    int n = rb_zone_cards(g, pl, zone_name, cards, RB_MAX_ZONE);
    int idxs[RB_MAX_ZONE];
    int mn = rb_get_selection_indices(cards, n, ctype, gn,
                                       e->self_target_field[0] && !strcmp(e->self_target_field, "true"), -1,
                                       idxs, RB_MAX_ZONE);
    int outcome = rb_classify_selection(idxs, mn, count, is_all);
    if (outcome == 1 && !strcmp(zone_name, "discard") && !is_all) {
        rb_move_prompt_card_selection_with_filtered(g, actor, zone_name, count, can_skip, e, idxs, mn);
        return -1;
    }
    if (outcome == 1 && can_skip && mn > 0) {
        if (is_all) {
            rb_zone_remove_at_indices(g, pl, zone_name, idxs, mn);
            for (int i = 0; i < mn && i < max; i++) out_ids[i] = cards[idxs[i]];
            return mn;
        }
        rb_move_prompt_card_selection_with_filtered(g, actor, zone_name, mn, can_skip, e, idxs, mn);
        return -1;
    }
    if (outcome == 1) {
        rb_zone_remove_at_indices(g, pl, zone_name, idxs, mn);
        for (int i = 0; i < mn && i < max; i++) out_ids[i] = cards[idxs[i]];
        return mn;
    }
    if (outcome == 2) {
        /* engine/src/ability/util/selection.rs:48-50 returns Prompt for
           idxs.len() > count REGARDLESS of `optional` / can_skip, and
           move_cards/selection.rs:81 consumes it as
           `SelectionOutcome::Prompt => count` — i.e. it prompts, with `count`
           cards, on every Prompt outcome. The previous C auto-resolved here
           whenever `!can_skip`, silently taking min(count, mn) cards off the
           top of the zone, so a choice Rust raises (愛♡スクリ～ム！'s
           「自分と相手は手札を1枚控え室に置く」 with 3 cards in hand is the
           shape that exposed it) never reached the player. `optional` is not
           a licence to auto-resolve: can_skip only decides whether the PROMPT
           may be declined, and it is already threaded into
           rb_move_prompt_card_selection as the choice's optional flag. */
        rb_move_prompt_card_selection_with_filtered(g, actor, zone_name, count, can_skip, e, idxs, mn);
        return -1;
    }
    if (can_skip && mn > 0) {
        rb_move_prompt_card_selection_with_filtered(g, actor, zone_name, mn, can_skip, e, idxs, mn);
        return -1;
    }
    return 0;
}

/* ── resolve_cards_from_source ── */
int rb_move_resolve_cards_from_source(GameState *g, int actor, AbilityEffect *e,
                                       int count, int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    const char *source = e->source ? e->source : "hand";
    const char *destination = e->destination ? e->destination : "discard";

    if (!strcmp(source, "recently_moved")) {
        const char *ctype = cmf_extra(e, "card_type");
        const char *gn = cmf_extra(e, "group_names");
        return rb_move_resolve_from_recently_moved(g, 0, ctype, gn, out_ids, max);
    }
    int pl = actor;
    if (e->target && !strcmp(e->target, "opponent")) pl ^= 1;
    if (!strcmp(source, "looked_at"))
        return rb_move_resolve_source_looked_at(g, actor, e, pl, count, out_ids, max);
    if (!strcmp(source, "looked_at_remaining")) {
        int n = 0;
        rb_move_resolve_from_looked_at(g, pl, out_ids, max, &n);
        return n;
    }
    if (!strcmp(source, "revealed_cards")) {
        int n = 0;
        for (int i = 0; i < g->n_revealed && n < max; i++) {
            int cid = g->revealed_cards[i];
            if (card_matches_filter(cid, e))
                out_ids[n++] = cid;
        }
        return n;
    }
    if (!strcmp(source, "under_member")) {
        int n = 0;
        int wanted = count < 0 ? RB_MAX_ZONE : count;
        const char *ctype = cmf_extra(e, "card_type");
        const char *group = cmf_extra(e, "group_names");
        for (int area = 0; area < RB_STAGE_SIZE && n < wanted; area++) {
            RbBag *under = &g->p[pl].under_cards[area];
            for (int i = 0; i < under->n && n < wanted; i++) {
                int cid = under->cards[i];
                if (ctype && !rb_card_matches_type(cid, ctype)) continue;
                if (group && !rb_card_matches_group_str(cid, group)) continue;
                out_ids[n++] = cid;
                for (int k = i; k < under->n - 1; k++) under->cards[k] = under->cards[k + 1];
                under->n--;
                i--;
            }
        }
        return n;
    }
    if (!strcmp(source, "selected_cards")) {
        int n = 0;
        for (int i = 0; i < g->n_selected_cards && n < max; i++)
            out_ids[n++] = g->selected_cards[i];
        return n;
    }
    if (!strcmp(source, "those_cards")) {
        int n = 0;
        for (int i = 0; i < g->n_those_cards && n < max; i++)
            out_ids[n++] = g->those_cards[i];
        return n;
    }
    if (!strcmp(source, "preceding_moved")) {
        int n = 0;
        for (int i = 0; i < g->n_recently_moved && n < max; i++)
            out_ids[n++] = g->recently_moved[i];
        return n;
    }
    if (!strcmp(source, "under_member")) {
        int n = 0;
        for (int i = 0; i < g->n_selected_cards && n < max; i++)
            out_ids[n++] = g->selected_cards[i];
        return n;
    }
    if (!strcmp(source, "deck") || !strcmp(source, "deck_top") ||
        !strcmp(source, "deck_bottom") || !strcmp(source, "energy_deck") ||
        !strcmp(source, "energy") || !strcmp(source, "stage") ||
        !strcmp(source, "live_card_zone")) {
        return rb_move_resolve_from_zone(g, actor, source, e, pl != actor, count, out_ids, max);
    }
    /* standard zone */
    int is_all = count < 0;
    int can_skip = e->is_optional || (cmf_extra(e, "any_number") &&
        (!strcmp(cmf_extra(e, "any_number"), "true") || !strcmp(cmf_extra(e, "any_number"), "1")));
    if (count < 0) count = RB_MAX_ZONE;
    return rb_move_take_cards_from_standard_zone(g, actor, source, e, count, is_all, can_skip, out_ids, max);
}

/* ── resolve_from_zone ── */
int rb_move_resolve_from_zone(GameState *g, int actor, const char *effective_source,
                                AbilityEffect *e, int use_p2, int count,
                                int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    int pl = use_p2 ? 1 : actor;
    RbPlayer *P = &g->p[pl];
    const char *ctype = cmf_extra(e, "card_type");
    const char *gn = cmf_extra(e, "group_names");
    const char *dst = e->destination ? e->destination : "";

    if (!strcmp(effective_source, "deck") || !strcmp(effective_source, "deck_top"))
        return rb_move_resolve_from_deck(g, actor, e, use_p2, count, ctype, gn, out_ids, max);
    if (!strcmp(effective_source, "deck_bottom"))
        return rb_move_resolve_from_deck_bottom(g, actor, e, use_p2, count, ctype, gn, out_ids, max);
    if (!strcmp(effective_source, "energy_deck"))
        return rb_move_resolve_from_energy_deck(g, pl, count, out_ids, max);
    if (!strcmp(effective_source, "stage"))
        return rb_move_resolve_from_stage(g, actor, e, use_p2, count, out_ids, max);
    if (!strcmp(effective_source, "energy")) {
        if (!strcmp(dst, "energy_deck")) {
            int n = 0;
            while (n < count && P->energy.n > 0) {
                int card = P->energy.cards[--P->energy.n];
                P->energy_active = P->energy_active > 0 ? P->energy_active - 1 : 0;
                if (n < max) out_ids[n++] = card;
            }
            return n;
        }
        return rb_move_take_cards_from_standard_zone(g, actor, "energy", e, count, 0, 1, out_ids, max);
    }
    if (!strcmp(effective_source, "hand") || !strcmp(effective_source, "discard") ||
        !strcmp(effective_source, "live_card_zone") || !strcmp(effective_source, "success_live_zone"))
        return rb_move_take_cards_from_standard_zone(g, actor, effective_source, e, count, 0, 1, out_ids, max);
    if (!strcmp(effective_source, "looked_at"))
        return rb_move_resolve_source_looked_at(g, actor, e, use_p2, count, out_ids, max);
    if (!strcmp(effective_source, "selected_cards")) {
        int n = 0;
        for (int i = 0; i < g->n_selected_cards && n < max; i++)
            out_ids[n++] = g->selected_cards[i];
        return n;
    }
    if (!strcmp(effective_source, "revealed_cards")) {
        int n = 0;
        for (int i = 0; i < g->n_revealed && n < max; i++) {
            if (ctype && !rb_card_matches_type(g->revealed_cards[i], ctype)) continue;
            if (gn && !rb_card_matches_group_str(g->revealed_cards[i], gn)) continue;
            out_ids[n++] = g->revealed_cards[i];
        }
        return n;
    }
    if (!strcmp(effective_source, "under_member"))
        return rb_move_resolve_from_under_member(g, actor, e, use_p2, count, out_ids, max);
    return 0;
}

/* ── resolve_from_standard_zone ── */
int rb_move_resolve_from_standard_zone(GameState *g, int actor, AbilityEffect *e,
                                        int use_p2, int count, int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    const char *source = e->source ? e->source : "hand";
    return rb_move_resolve_from_zone(g, actor, source, e, use_p2, count, out_ids, max);
}

/* ── resolve_from_revealed_cards ── */
int rb_move_resolve_from_revealed_cards(GameState *g, int actor, AbilityEffect *e,
                                         int count, int is_all, int is_max,
                                         int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    int take_count = is_all ? g->n_revealed : (count < g->n_revealed ? count : g->n_revealed);
    int can_skip = is_max || e->is_optional;
    RbCardFilter filter;
    memset(&filter, 0, sizeof(filter));
    const char *cl = cmf_extra(e, "cost_limit");
    if (cl) { filter.has_cost_limit = 1; filter.cost_limit = atoi(cl); }
    const char *clo = cmf_extra(e, "cost_operator");
    if (clo) strncpy(filter.cost_op, clo, sizeof(filter.cost_op) - 1);
    const char *ct = cmf_extra(e, "cost_total");
    if (ct) { filter.has_cost_total = 1; filter.cost_total = atoi(ct); }
    const char *cto = cmf_extra(e, "cost_total_operator");
    if (cto) strncpy(filter.cost_total_op, cto, sizeof(filter.cost_total_op) - 1);
    const char *gn = cmf_extra(e, "group_names");
    if (gn) { filter.has_group = 1; strncpy(filter.group, gn, sizeof(filter.group) - 1); }
    const char *ctp = cmf_extra(e, "card_type");
    if (ctp) strncpy(filter.card_type, ctp, sizeof(filter.card_type) - 1);
    const char *cp = cmf_extra(e, "card_property");
    if (cp) strncpy(filter.ability_filter, cp, sizeof(filter.ability_filter) - 1);
    const char *neg = cmf_extra(e, "negation");
    filter.negation = neg && !strcmp(neg, "true");

    int matching[RB_MAX_ZONE];
    int nm = 0;
    for (int i = 0; i < g->n_revealed && nm < max; i++) {
        int cid = g->revealed_cards[i];
        if (!rb_matching_ids(&filter, &cid, 1, &matching[nm], max - nm)) continue;
        nm++;
    }
    if (nm == 0) return 0;
    if (take_count < nm || can_skip) {
        rb_move_prompt_card_selection(g, actor, "revealed_cards", take_count, can_skip, e);
        return 0;
    }
    int actual_take = take_count < nm ? take_count : nm;
    int taken[RB_MAX_ZONE];
    int nt = 0;
    for (int i = actual_take - 1; i >= 0; i--) {
        int cid = g->revealed_cards[i];
        for (int j = i; j < g->n_revealed - 1; j++) g->revealed_cards[j] = g->revealed_cards[j + 1];
        g->n_revealed--;
        taken[nt++] = cid;
    }
    for (int i = 0; i < nt && i < max; i++) out_ids[i] = taken[i];
    return nt;
}

/* ── resolve_from_those_cards ── */
int rb_move_resolve_from_those_cards(GameState *g, int actor, AbilityEffect *e,
                                      int count, int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    int n_trigger = g->n_those_cards;
    if (n_trigger == 0) n_trigger = g->n_recently_moved;
    if (n_trigger == 0) return 0;

    const char *ctype = cmf_extra(e, "card_type");
    const char *gn = cmf_extra(e, "group_names");
    const char *dst = e->destination ? e->destination : "";

    int all_matching[RB_MAX_ZONE];
    int nm = 0;
    for (int i = 0; i < n_trigger && nm < max; i++) {
        int cid = g->those_cards[i];
        if (ctype && !rb_card_matches_type(cid, ctype)) continue;
        if (gn && !rb_card_matches_group_str(cid, gn)) continue;
        all_matching[nm++] = cid;
    }
    if (nm == 0) {
        int cur = g->queue.cur;
        if (cur >= 0 && cur < RB_QUEUE_DEPTH) g->queue.entries[cur].optional_moves_all_moved = 0;
        return 0;
    }
    if (nm <= count && strcmp(dst, "deck_top_or_bottom") != 0) {
        int found = nm < count ? nm : count;
        for (int i = 0; i < found && i < max; i++) out_ids[i] = all_matching[i];
        return found;
    }
    if (!strcmp(dst, "deck_top_or_bottom")) {
        rb_move_prompt_card_selection(g, actor, "discard", 1, 0, e);
        return 0;
    }
    rb_move_prompt_card_selection(g, actor, "discard", count, e->is_optional, e);
    return 0;
}

/* ── resolve_from_selected_cards ── */
int rb_move_resolve_from_selected_cards(GameState *g, int actor, AbilityEffect *e,
                                         int count, int is_all, int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    int n = g->n_selected_cards;
    if (n == 0) return 0;
    int take = is_all ? n : (count < n ? count : n);
    for (int i = 0; i < take && i < max; i++) out_ids[i] = g->selected_cards[i];
    for (int i = 0; i < take; i++) {
        int last_vacated = -1;
        remove_card_from_any_zone(&g->p[actor], &last_vacated, out_ids[i]);
        if (last_vacated >= 0)
            g->baton_last_vacated_area[actor] = last_vacated;
    }
    return take;
}

/* ── resolve_source_revealed_cards ── */
int rb_move_resolve_source_revealed_cards(GameState *g, int actor, AbilityEffect *e,
                                           int count, int is_all, int is_max,
                                           int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    int take_count = is_all ? g->n_revealed : (count < g->n_revealed ? count : g->n_revealed);
    int can_skip = is_max || e->is_optional;
    RbCardFilter filter;
    memset(&filter, 0, sizeof(filter));
    const char *cl = cmf_extra(e, "cost_limit");
    if (cl) { filter.has_cost_limit = 1; filter.cost_limit = atoi(cl); }
    const char *clo = cmf_extra(e, "cost_operator");
    if (clo) strncpy(filter.cost_op, clo, sizeof(filter.cost_op) - 1);
    const char *ct = cmf_extra(e, "cost_total");
    if (ct) { filter.has_cost_total = 1; filter.cost_total = atoi(ct); }
    const char *cto = cmf_extra(e, "cost_total_operator");
    if (cto) strncpy(filter.cost_total_op, cto, sizeof(filter.cost_total_op) - 1);
    const char *gn = cmf_extra(e, "group_names");
    if (gn) { filter.has_group = 1; strncpy(filter.group, gn, sizeof(filter.group) - 1); }
    const char *ctp = cmf_extra(e, "card_type");
    if (ctp) strncpy(filter.card_type, ctp, sizeof(filter.card_type) - 1);

    int matching[RB_MAX_ZONE];
    int nm = 0;
    for (int i = 0; i < g->n_revealed && nm < max; i++) {
        int cid = g->revealed_cards[i];
        if (!rb_matching_ids(&filter, &cid, 1, &matching[nm], max - nm)) continue;
        nm++;
    }
    if (nm == 0) return 0;
    if (take_count < nm || can_skip) {
        rb_move_prompt_card_selection(g, actor, "revealed_cards", take_count, can_skip, e);
        return 0;
    }
    int actual_take = take_count < nm ? take_count : nm;
    int taken[RB_MAX_ZONE];
    int nt = 0;
    for (int i = actual_take - 1; i >= 0; i--) {
        int cid = g->revealed_cards[i];
        for (int j = i; j < g->n_revealed - 1; j++) g->revealed_cards[j] = g->revealed_cards[j + 1];
        g->n_revealed--;
        taken[nt++] = cid;
    }
    for (int i = 0; i < nt && i < max; i++) out_ids[i] = taken[i];
    return nt;
}

/* ── place_card_with_stage_choice ── */
int rb_move_place_card_with_stage_choice(
    GameState *g, int actor, int host_cid, const char *player_target,
    int card_id, const char *destination, int vacated_area,
    int is_max, int count, const char *state_change,
    int deck_position, const char *source_zone,
    int allow_occupied_stage, int under_self) {
    int pl = actor;
    if (player_target && *player_target) {
        int t = rb_resolve_target_player(g, player_target);
        if (t >= 0) pl = t;
    }
    RbPlayer *P = &g->p[pl];
    /* Rust place_card_with_stage_choice (engine/src/ability/move_cards/placement.rs:57-241)
       performs NO success-zone replacement check — maybe_prompt_success_replacement is a
       separate one-shot gate owned by execute_move_cards / execute_selected_cards_from_zone
       (placement.rs:249). Calling it here too could emit a second replacement choice for a
       success-zone destination routed through the stage placement path. */
    if (!strcmp(destination, "empty_area") || !strcmp(destination, "stage")) {
        int empty_slots[RB_STAGE_SIZE], n_empty = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] == RB_EMPTY_SLOT) empty_slots[n_empty++] = i;
        if (is_max && n_empty < count) return -1;
        int avail[RB_STAGE_SIZE], n_avail = 0;
        if (allow_occupied_stage) {
            for (int i = 0; i < RB_STAGE_SIZE; i++)
                if (P->stage[i] == RB_EMPTY_SLOT || !g->stage_arrived[pl][i])
                    avail[n_avail++] = i;
        } else {
            for (int i = 0; i < n_empty; i++) avail[i] = empty_slots[i];
            n_avail = n_empty;
        }
        if (n_avail == 0) return -1;
        if (n_avail > 1) {
            if (vacated_area >= 0 && vacated_area < RB_STAGE_SIZE && P->stage[vacated_area] == RB_EMPTY_SLOT) {
                P->stage[vacated_area] = card_id;
                if (!source_zone || strcmp(source_zone, "stage") != 0) g->stage_arrived[pl][vacated_area] = 1;
                return 0;
            }
            char pos_str[128]; pos_str[0] = '\0';
            for (int i = 0; i < n_avail; i++) {
                if (i > 0) strcat(pos_str, ",");
                switch (avail[i]) {
                    case 0: strcat(pos_str, "left_side"); break;
                    case 1: strcat(pos_str, "center"); break;
                    default: strcat(pos_str, "right_side"); break;
                }
            }
            char card_name[64]; card_name[0] = '\0';
            Card c;
            if (rb_decode_card_by_index((uint32_t)card_id, &c)) {
                if (c.name) strncpy(card_name, c.name, sizeof(card_name) - 1);
                rb_free_card(&c);
            }
            if (card_name[0] == '\0') strcpy(card_name, "card");
            char desc[128];
            snprintf(desc, sizeof(desc), "Choose position for %s", card_name);
            rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, 1, 0, pos_str);
    rb_queue_pause_for_choice(g, &g->queue.pending);
            rb_choice_set_description(&g->queue.pending, desc);
            rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_TARGET);
            g->queue.resume_mode = 1;
            g->queue.resume_actor = actor;
            g->queue.resume_host = card_id;
            if (state_change && *state_change)
                strncpy(g->queue.resume_draw_dest, state_change, sizeof(g->queue.resume_draw_dest) - 1);
            if (source_zone && *source_zone)
                strncpy(g->queue.resume_draw_source, source_zone, sizeof(g->queue.resume_draw_source) - 1);
            return 1;
        } else {
            int slot = avail[0];
            if (P->stage[slot] != RB_EMPTY_SLOT) rb_waitroom_add(P, P->stage[slot]);
            P->stage[slot] = card_id;
            if (!source_zone || strcmp(source_zone, "stage") != 0) g->stage_arrived[pl][slot] = 1;
            return 0;
        }
    }
    int pos_to_use = -1;
    if (!strcmp(destination, "under_member")) {
        int from_self_displacement = (!source_zone || !strcmp(source_zone, "stage") || source_zone[0] == '\0');
        int n_members = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) n_members++;
        if (!from_self_displacement && !under_self && n_members > 1 && vacated_area < 0) {
            char card_name[64]; card_name[0] = '\0';
            Card c;
            if (rb_decode_card_by_index((uint32_t)card_id, &c)) {
                if (c.name) strncpy(card_name, c.name, sizeof(card_name) - 1);
                rb_free_card(&c);
            }
            if (card_name[0] == '\0') strcpy(card_name, "card");
            char desc[128];
            snprintf(desc, sizeof(desc), "Choose a member to place %s under", card_name);
            rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "stage", NULL, 1, 0, NULL);
    rb_queue_pause_for_choice(g, &g->queue.pending);
            rb_choice_set_description(&g->queue.pending, desc);
            rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
            g->queue.resume_mode = 1;
            g->queue.resume_actor = actor;
            g->queue.resume_host = card_id;
            if (player_target && *player_target)
                strncpy(g->queue.pending.target, player_target, sizeof(g->queue.pending.target) - 1);
            if (state_change && *state_change)
                strncpy(g->queue.resume_draw_dest, state_change, sizeof(g->queue.resume_draw_dest) - 1);
            if (source_zone && *source_zone)
                strncpy(g->queue.resume_draw_source, source_zone, sizeof(g->queue.resume_draw_source) - 1);
            return 1;
        }
        pos_to_use = -1;
        if (host_cid >= 0) {
            for (int i = 0; i < RB_STAGE_SIZE; i++)
                if (P->stage[i] == host_cid) { pos_to_use = i; break; }
        }
        if (pos_to_use < 0 && vacated_area >= 0) pos_to_use = vacated_area;
        if (pos_to_use < 0 && vacated_area < 0) {
            for (int i = 0; i < RB_STAGE_SIZE; i++)
                if (P->stage[i] != RB_EMPTY_SLOT) { pos_to_use = i; break; }
        }
        if (pos_to_use < 0) {
            for (int i = g->n_recently_moved - 1; i >= 0; i--) {
                for (int j = 0; j < RB_STAGE_SIZE; j++)
                    if (P->stage[j] == g->recently_moved[i]) { pos_to_use = j; break; }
                if (pos_to_use >= 0) break;
            }
        }
    } else if (!strcmp(destination, "deck") || !strcmp(destination, "deck_top")) {
        pos_to_use = (deck_position >= 0) ? deck_position : vacated_area;
    } else {
        pos_to_use = vacated_area;
    }
    if (!strcmp(destination, "deck") || !strcmp(destination, "deck_top")) {
        int idx = pos_to_use >= 0 ? pos_to_use : 0;
        if (idx > P->deck.n) idx = P->deck.n;
        if (P->deck.n < RB_MAX_ZONE) {
            for (int k = P->deck.n; k > idx; k--) P->deck.cards[k] = P->deck.cards[k - 1];
            P->deck.cards[idx] = card_id;
            P->deck.n++;
        }
    } else if (!strcmp(destination, "deck_bottom")) {
        if (P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = card_id;
    } else if (!strcmp(destination, "under_member")) {
        if (pos_to_use >= 0 && pos_to_use < RB_STAGE_SIZE)
            rb_stage_place_under_card(P, pos_to_use, card_id);
    } else {
        rb_place_card_in_zone(g, pl, card_id, destination, pos_to_use);
    }
    return 0;
}

/* ── execute_stage_placement_choices ── */
int rb_move_execute_stage_placement_choices(GameState *g, int actor,
                                             const int *card_ids, int n_ids,
                                             const char *src_zone,
                                             const char *dest,
                                             int vacated_area,
                                             const char *target,
                                             int *out_ids, int max) {
    if (!g || !card_ids || n_ids <= 0 || !out_ids) return 0;
    int nm = 0;
    for (int pos = 0; pos < n_ids; pos++) {
        int card_id = card_ids[pos];
        int pl = actor;
        if (target && *target) { int t = rb_resolve_target_player(g, target); if (t >= 0) pl = t; }
        rb_remove_card_from_zone(g, pl, card_id, src_zone);
        int placed = rb_move_place_card_with_stage_choice(g, actor, -1, target,
                                                           card_id, dest, vacated_area,
                                                           0, 1, NULL, -1, src_zone,
                                                           0, 0);
        if (placed == 1) {
            out_ids[nm++] = card_id;
        } else if (placed == 0) {
            out_ids[nm++] = card_id;
            rb_move_fire_debut_side_effects(g, actor, card_id, target ? target : "self", NULL);
        }
    }
    return nm;
}

/* ── maybe_prompt_success_replacement ── */
int rb_move_maybe_prompt_success_replacement(GameState *g, int actor, int card_id,
                                              const char *dest, const char *target) {
    if (!g || !dest) return 0;
    /* Rust placement.rs:256 — `Zone::from_str(dest) != Some(Zone::SuccessLiveZone)`
       gates the whole check. "success_zone" is a DIFFERENT variant
       (Zone::SuccessZone, engine/src/ability/enums.rs:48/92) and must not
       trigger a replacement prompt. "success_live_card_zone" is the same
       variant as "success_live_zone" (enums.rs:94). */
    if (strcmp(dest, "success_live_zone") != 0 &&
        strcmp(dest, "success_live_card_zone") != 0) return 0;
    int pl = actor;
    if (target && *target) {
        int resolved = rb_resolve_target_player(g, target);
        if (resolved >= 0) pl = resolved;
    }
    return rb_try_create_success_replacement_choice(g, card_id, pl);
}

/* ── prompt_deck_top_or_bottom ── */
void rb_move_prompt_deck_top_or_bottom(GameState *g, int actor, int card_id,
                                        const char *target, const char *source_zone,
                                        int allow_skip) {
    if (!g) return;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, 1, allow_skip,
                   "deck_top_or_bottom");
    rb_queue_pause_for_choice(g, &g->queue.pending);
    rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_TARGET);
    if (g->queue.cur < RB_QUEUE_DEPTH) {
        g->queue.entries[g->queue.cur].card_id = card_id;
    }
    g->queue.resume_mode = 1;
    g->queue.resume_actor = actor;
    g->queue.resume_host = card_id;
    if (target && *target)
        strncpy(g->queue.pending.target, target, sizeof(g->queue.pending.target) - 1);
    if (source_zone && *source_zone)
        strncpy(g->queue.resume_draw_source, source_zone, sizeof(g->queue.resume_draw_source) - 1);
}

/* ── finalize_card_movement ── */
void rb_move_finalize_card_movement(GameState *g, int actor,
                                     const int *moved_cards, int n_moved,
                                     const char *destination, const char *source,
                                     const char *state_change, const char *target) {
    if (!g || !moved_cards || n_moved <= 0) return;
    (void)source;
    /* Rust move_cards.rs:2278 calls `gs.on_cards_left_zones(moved_cards)`, the
       single zone-exit choke point (engine/src/core/game_state/modifiers.rs:1677)
       that clears BOTH the modifier tables AND any runtime-gained abilities.
       Calling `rb_mods_clear_card` alone left constant 「能力付与」 grants alive
       on a card that had just changed zones. `rb_on_cards_left_zones` is defined
       in src/core/modifiers.c:599 but was previously declared in no header and
       called from nowhere. */
    for (int i = 0; i < n_moved; i++)
        rb_on_cards_left_zones(g, moved_cards[i]);
    if (state_change && *state_change) {
        if (!strcmp(state_change, "wait")) {
            for (int i = 0; i < n_moved; i++)
                rb_mods_set_orientation(&g->mods, moved_cards[i], "wait");
        } else if (!strcmp(state_change, "active")) {
            for (int i = 0; i < n_moved; i++)
                rb_mods_set_orientation(&g->mods, moved_cards[i], "active");
            if (destination && !strcmp(destination, "energy")) {
                int pl = actor;
                if (target && *target) { int t = rb_resolve_target_player(g, target); if (t >= 0) pl = t; }
                int sum = (int)g->p[pl].energy_active + n_moved;
                g->p[pl].energy_active = sum > 32767 ? 32767 : sum;
            }
        }
    }
    for (int i = 0; i < n_moved; i++)
        mc_record_movement(g, moved_cards[i]);
    rb_recalc_constants(g);
    if (destination && !strcmp(destination, "stage")) {
        for (int i = 0; i < n_moved; i++)
            rb_move_fire_debut_side_effects(g, actor, moved_cards[i],
                                            target ? target : "self", NULL);
    }
}

/* ── fire_debut_side_effects ── */
void rb_move_fire_debut_side_effects(GameState *g, int actor, int card_id,
                                      const char *target, const char *source) {
    int pl = actor;
    if (target && *target) { int t = rb_resolve_target_player(g, target); if (t >= 0) pl = t; }
    if (card_id >= 0 && g->n_cards_appeared_this_turn < 64)
        g->cards_appeared_this_turn[g->n_cards_appeared_this_turn++] = card_id;
    (void)source;
    if (pl >= 0 && pl < 2)
        g->debut_count_this_turn[pl]++;
    rb_trigger_debut(g, pl, card_id);
    rb_trigger_auto_abilities(g, pl, "自動");
    rb_process_pending_auto_abilities(g);
}

/* ── handle_select_position ── */
void rb_move_handle_select_position(GameState *g, int actor, const char *position,
                                     int card_id, const char *target,
                                     const char *source_zone, const char *state_change) {
    if (!g) return;
    int pl = actor;
    if (target && *target) { int t = rb_resolve_target_player(g, target); if (t >= 0) pl = t; }
    RbPlayer *P = &g->p[pl];
    int pos_idx = -1;
    if (!strcmp(position, "left_side")) pos_idx = 0;
    else if (!strcmp(position, "center")) pos_idx = 1;
    else if (!strcmp(position, "right_side")) pos_idx = 2;
    int should_lock = (!source_zone || strcmp(source_zone, "stage") != 0);
    int placed = 0;
    if (pos_idx >= 0 && pos_idx < RB_STAGE_SIZE) {
        if (P->stage[pos_idx] == RB_EMPTY_SLOT) {
            P->stage[pos_idx] = card_id;
            if (should_lock) g->stage_arrived[pl][pos_idx] = 1;
            placed = 1;
        } else if (P->stage[pos_idx] != RB_EMPTY_SLOT) {
            rb_waitroom_add(P, P->stage[pos_idx]);
            P->stage[pos_idx] = card_id;
            if (should_lock) g->stage_arrived[pl][pos_idx] = 1;
            placed = 1;
        }
    }
    if (!placed) rb_hand_add(P, card_id);
    rb_mods_clear_card(&g->mods, card_id);
    mc_record_movement(g, card_id);
    if (state_change && !strcmp(state_change, "wait"))
        rb_mods_set_orientation(&g->mods, card_id, "wait");
    rb_move_fire_debut_side_effects(g, actor, card_id, target ? target : "self", NULL);
    rb_clear_pending_choice(g);
}

/* ── move_from_revealed ── */
int rb_move_from_revealed(GameState *g, int actor, const int *indices, int n_indices,
                           int (*validate_card)(int), const char *dst,
                           int *out_ids, int max) {
    (void)actor;
    if (!g || !indices || n_indices <= 0 || !out_ids || max <= 0) return 0;
    int sorted[RB_MAX_RECENTLY_MOVED];
    int ns = n_indices < RB_MAX_RECENTLY_MOVED ? n_indices : RB_MAX_RECENTLY_MOVED;
    for (int i = 0; i < ns; i++) sorted[i] = indices[i];
    for (int i = 0; i < ns - 1; i++)
        for (int j = i + 1; j < ns; j++)
            if (sorted[j] > sorted[i]) { int t = sorted[i]; sorted[i] = sorted[j]; sorted[j] = t; }
    int pl = g->active;
    int nm = 0;
    for (int k = 0; k < ns; k++) {
        int idx = sorted[k];
        if (idx < 0 || idx >= g->n_revealed) continue;
        int cid = g->revealed_cards[idx];
        for (int j = idx; j < g->n_revealed - 1; j++)
            g->revealed_cards[j] = g->revealed_cards[j + 1];
        g->n_revealed--;
        if (validate_card && !validate_card(cid)) continue;
        for (int p = 0; p < 2; p++) {
            RbPlayer *P = &g->p[p];
            int found = 0;
            for (int i = 0; i < P->discard.n; i++) {
                if (P->discard.cards[i] == cid) {
                    for (int j = i; j < P->discard.n - 1; j++) P->discard.cards[j] = P->discard.cards[j + 1];
                    P->discard.n--; found = 1; break;
                }
            }
            if (found) break;
            for (int i = 0; i < P->deck.n; i++) {
                if (P->deck.cards[i] == cid) {
                    for (int j = i; j < P->deck.n - 1; j++) P->deck.cards[j] = P->deck.cards[j + 1];
                    P->deck.n--; found = 1; break;
                }
            }
            if (found) break;
        }
        rb_place_card_in_zone(g, pl, cid, dst, -1);
        if (nm < max) out_ids[nm++] = cid;
    }
    return nm;
}

/* ── move_from_under_member ── */
int rb_move_from_under_member(GameState *g, int actor, const int *indices, int n_indices,
                               int (*validate)(int), const char *dst, const char *target) {
    if (!g) return -1;
    int pl = actor;
    if (target && *target) { int t = rb_resolve_target_player(g, target); if (t >= 0) pl = t; }
    RbPlayer *P = &g->p[pl];

    int host_ids[4]; int nh = 0;
    for (int k = 0; k < n_indices; k++) {
        int idx = indices ? indices[k] : -1;
        if (idx < 0) return -1;
        int global = 0, found = 0, si = -1, cid = -1;
        for (si = 0; si < RB_STAGE_SIZE; si++) {
            int len = P->under_cards[si].n;
            if (idx < global + len) {
                cid = P->under_cards[si].cards[idx - global];
                if (validate && !validate(cid)) return -1;
                found = 1;
                break;
            }
            global += len;
        }
        if (!found) return -1;
        for (int j = idx - global; j < P->under_cards[si].n - 1; j++)
            P->under_cards[si].cards[j] = P->under_cards[si].cards[j + 1];
        P->under_cards[si].n--;
        rb_place_card_in_zone(g, pl, cid, dst ? dst : "discard", -1);
        int host = P->stage[si];
        if (host >= 0) {
            int dup = 0;
            for (int h = 0; h < nh; h++) if (host_ids[h] == host) { dup = 1; break; }
            if (!dup && nh < 4) host_ids[nh++] = host;
        }
    }

    g->mods.n_last_under_move_host_ids = 0;
    for (int h = 0; h < nh; h++) {
        int dup = 0;
        for (int i = 0; i < g->mods.n_last_under_move_host_ids; i++)
            if (g->mods.last_under_move_host_ids[i] == host_ids[h]) { dup = 1; break; }
        if (!dup && g->mods.n_last_under_move_host_ids < 4)
            g->mods.last_under_move_host_ids[g->mods.n_last_under_move_host_ids++] = (int16_t)host_ids[h];
    }
    rb_recalc_constants(g);
    return n_indices;
}

/* ── place_energy_under_member_selected ── */
void rb_move_place_energy_under_member_selected(GameState *g, int actor,
                                                const int *cids, int n_cids) {
    if (!g || !cids || n_cids <= 0) return;
    int activating = g->activating_card;
    int pl = actor;
    RbPlayer *P = &g->p[pl];
    int target_index = -1;
    if (activating >= 0) {
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] == activating && P->stage[i] != RB_EMPTY_SLOT)
                { target_index = i; break; }
    }
    if (target_index < 0) {
        for (int i = g->n_recently_moved - 1; i >= 0; i--) {
            for (int j = 0; j < RB_STAGE_SIZE; j++)
                if (P->stage[j] == g->recently_moved[i] && P->stage[j] != RB_EMPTY_SLOT)
                    { target_index = j; break; }
            if (target_index >= 0) break;
        }
    }
    if (target_index < 0) {
        if (P->stage[1] != RB_EMPTY_SLOT) target_index = 1;
        else if (P->stage[0] != RB_EMPTY_SLOT) target_index = 0;
        else if (P->stage[2] != RB_EMPTY_SLOT) target_index = 2;
    }
    if (target_index < 0 || P->stage[target_index] == RB_EMPTY_SLOT) {
        for (int i = 0; i < n_cids; i++)
            if (P->energy_deck.n < RB_MAX_ZONE) P->energy_deck.cards[P->energy_deck.n++] = cids[i];
        return;
    }
    int area = target_index;
    for (int i = 0; i < n_cids; i++)
        rb_stage_place_under_card(P, area, cids[i]);
    int cause_pid = 0;
    if (g->queue.cur >= 0 && g->queue.cur < RB_QUEUE_DEPTH)
        cause_pid = g->queue.entries[g->queue.cur].player_id[0] - '0';
    int cause_cid = g->activating_card;
    for (int i = 0; i < n_cids; i++) {
        g->batch_movements[g->n_batch_movements].moved_card_id = cids[i];
        g->batch_movements[g->n_batch_movements].source_zone = RB_ZONEID_ENERGY_ZONE;
        g->batch_movements[g->n_batch_movements].dest_zone = RB_ZONEID_UNDER_MEMBER;
        g->batch_movements[g->n_batch_movements].cause_player_id = cause_pid;
        g->batch_movements[g->n_batch_movements].effect_only = 1;
        if (g->n_batch_movements < 16) g->n_batch_movements++;
    }
    rb_recalc_constants(g);
}

/* ── execute_selected_energy_zone_cards ── */
void rb_effect_selected_energy_zone_cards(GameState *g, int actor, const int *indices, int n_indices) {
    if (!g || n_indices <= 0) return;
    int pl = actor;
    RbPlayer *P = &g->p[pl];
    int to_mark[RB_MAX_ZONE]; int nm = 0;
    for (int i = 0; i < n_indices && nm < RB_MAX_ZONE; i++) {
        int idx = indices[i];
        if (idx >= 0 && idx < P->energy.n) to_mark[nm++] = P->energy.cards[idx];
    }
    int sub = nm < 32768 ? nm : 32767;
    P->energy_active = P->energy_active > sub ? P->energy_active - sub : 0;
    for (int i = 0; i < nm; i++) {
        rb_mods_clear_card(&g->mods, to_mark[i]);
        rb_mods_set_orientation(&g->mods, to_mark[i], "wait");
    }
}

/* ── handle_energy_zone_selection ── */
void rb_move_handle_energy_zone_selection(
    GameState *g, int actor, const int *indices, int n_indices,
    int count, const char *destination, int (*validate_card)(int)) {
    if (!g || !indices || n_indices <= 0) return;

    int pl = actor;
    RbPlayer *P = &g->p[pl];

    if (destination && *destination) {
        int sorted_idx[RB_MAX_RECENTLY_MOVED];
        int ns = n_indices < RB_MAX_RECENTLY_MOVED ? n_indices : RB_MAX_RECENTLY_MOVED;
        for (int i = 0; i < ns; i++) sorted_idx[i] = indices[i];
        for (int i = 0; i < ns - 1; i++)
            for (int j = i + 1; j < ns; j++)
                if (sorted_idx[j] > sorted_idx[i]) { int t = sorted_idx[i]; sorted_idx[i] = sorted_idx[j]; sorted_idx[j] = t; }

        int cids[RB_MAX_RECENTLY_MOVED];
        int nc = 0;
        for (int i = 0; i < ns; i++) {
            int idx = sorted_idx[i];
            if (idx >= 0 && idx < P->energy.n) {
                int cid = P->energy.cards[idx];
                if (validate_card && !validate_card(cid)) continue;
                cids[nc++] = cid;
                for (int k = idx; k < P->energy.n - 1; k++)
                    P->energy.cards[k] = P->energy.cards[k + 1];
                P->energy.n--;
            }
        }

        P->energy_active = P->energy_active > nc ? P->energy_active - nc : 0;

        if (!strcmp(destination, "under_member")) {
            if (nc > 0) {
                rb_move_place_energy_under_member_selected(g, actor, cids, nc);
            }
        } else {
            for (int i = 0; i < nc; i++) {
                rb_place_card_in_zone(g, pl, cids[i], destination, -1);
            }
        }

        for (int i = 0; i < nc; i++) {
            rb_mods_clear_card(&g->mods, cids[i]);
            mc_record_movement(g, cids[i]);
        }
    } else {
        rb_effect_selected_energy_zone_cards(g, actor, indices, n_indices);
    }
    rb_clear_pending_choice(g);
}

/* ── execute_move_cards (main entry) ── */
void rb_move_execute_move_cards(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;
    int count = e->count >= 0 ? e->count : 0;
    const char *source = e->source ? e->source : "hand";
    const char *destination = e->destination ? e->destination : "discard";
    int use_p2 = 0;
    if (e->target && !strcmp(e->target, "opponent")) use_p2 = 1;

    int out_ids[RB_MAX_ZONE];
    int n = rb_move_resolve_cards_from_source(g, actor, e, count, out_ids, RB_MAX_ZONE);
    if (n <= 0) return;

    RbPlayer *P = use_p2 ? &g->p[1] : &g->p[actor];
    for (int i = 0; i < n; i++) {
        int cid = out_ids[i];
        find_and_remove_card(P, cid);
        rb_place_card_in_zone(g, actor, cid, destination, -1);
        rb_mods_clear_card(&g->mods, cid);
        mc_record_movement(g, cid);
    }
    rb_recalc_constants(g);
    if (destination && !strcmp(destination, "stage")) {
        for (int i = 0; i < n; i++)
            rb_move_fire_debut_side_effects(g, actor, out_ids[i], e->target ? e->target : "self", NULL);
    }
}

/* ── execute_move_cards_both ──
 *
 * Port of AbilityResolver::execute_move_cards_both
 * (engine/src/ability/move_cards.rs:3411-3445). This is the OPPONENT-FIRST
 * path, and it is deliberately the opposite of rb_misc_handle_both_targets
 * (misc.c, Rust misc.rs:234-301), which is SELF-FIRST: the two are reached
 * through different Rust guards, and only the multi-target-to-deck shape gets
 * here, because execute_move_cards (move_cards.rs:2008-2013) hands it
 * `multiple_targets && target == "deck"`.
 *
 * The previous body just forwarded to rb_move_execute_move_cards, so the
 * opponent's half never ran and the order was never opponent-first — abilities
 * 448 and 828 each moved one card instead of two. Rust's clone rewrites only
 * `target` (no `action_by` rewrite here, unlike misc.rs:283-285), so the clone
 * below does the same.
 *
 * The arms call rb_move_execute_move_cards_ported, NOT the short
 * rb_move_execute_move_cards above: Rust's execute_move_cards_both re-enters
 * Rust's execute_move_cards (move_cards.rs:3425/:3441), and
 * rb_move_execute_move_cards_ported is the C twin of THAT function — it is what
 * move_missing.c:139-142 dispatches here from, in the same prologue position as
 * move_cards.rs:2008-2013. The short sibling is a separate, much weaker
 * implementation with no Rust counterpart on this path, and running the arms
 * through it would move the wrong cards. Each arm carries target
 * "opponent"/"self", so that prologue's `target == "deck"` test is false and the
 * pair cannot recurse. */
void rb_move_execute_move_cards_both(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;
    /* move_cards.rs:3422-3425 — OPPONENT FIRST. */
    AbilityEffect opp_eff = *e;
    opp_eff.target = (char *)"opponent";
    rb_move_execute_move_cards_ported(g, actor, &opp_eff);

    /* move_cards.rs:3427-3435 — if the opponent half raised a choice, the self
     * half waits for the answer. Rust REPLACES the pending actions with the one
     * self clone (set_pending_actions(vec![self_eff])), it does not append, so
     * rb_queue_store_pending_actions' clear-then-store is the right twin. */
    if (rb_has_pending_choice(g)) {
        AbilityEffect self_eff = *e;
        self_eff.target = (char *)"self";
        AbilityEffect *parked[1] = { &self_eff };
        rb_queue_store_pending_actions(g, parked, 1);
        return;
    }
    /* move_cards.rs:3436-3441 — no choice raised, so self runs now. */
    AbilityEffect self_eff = *e;
    self_eff.target = (char *)"self";
    rb_move_execute_move_cards_ported(g, actor, &self_eff);
}

/* ── execute_selected_cards_from_zone ── */
void rb_move_execute_selected_cards_from_zone(
    GameState *g, int actor, const char *zone, const int *indices, int n_indices,
    const char *card_type_filter, int cost_limit, const char *cost_limit_op,
    int cost_total, const char *cost_total_op,
    const char *group, const char **characters, int n_characters,
    const char *target_player_id) {
    if (!g || !zone || !indices || n_indices <= 0) return;

    const char *destination = rb_entry_destination(g);
    if (!destination) destination = "discard";

    const char *target = target_player_id ? target_player_id : "self";
    int pl = rb_resolve_target_player(g, target);
    if (pl < 0) pl = actor;

    int filtered[RB_MAX_RECENTLY_MOVED];
    int nf = 0;
    int cards[RB_MAX_ZONE];
    int nc = rb_zone_cards(g, pl, zone, cards, RB_MAX_ZONE);
    for (int i = 0; i < n_indices && nf < RB_MAX_RECENTLY_MOVED; i++) {
        int idx = indices[i];
        if (idx < 0 || idx >= nc) continue;
        int cid = cards[idx];
        if (card_type_filter && !rb_card_matches_type(cid, card_type_filter)) continue;
        if (cost_limit >= 0 && !rb_card_matches_cost_limit(cid, cost_limit, cost_limit_op ? cost_limit_op : "<=")) continue;
        if (cost_total >= 0) {
            Card selected_card;
            if (rb_decode_card_by_index((uint32_t)cid, &selected_card)) {
                int total_ok = cost_total_op ? (cost_total_op[0] == '>' ? selected_card.cost > cost_total :
                                                cost_total_op[0] == '<' ? selected_card.cost < cost_total :
                                                selected_card.cost == cost_total) : selected_card.cost == cost_total;
                rb_free_card(&selected_card);
                if (!total_ok) continue;
            }
        }
        if (group && !rb_card_matches_group_str(cid, group)) continue;
        if (characters && n_characters > 0 && !rb_card_matches_characters(cid, characters, n_characters)) continue;
        filtered[nf++] = idx;
    }

    int card_ids[RB_MAX_RECENTLY_MOVED];
    int n_ids = rb_resolve_indices_to_ids(g, pl, zone, filtered, nf, card_ids);
    if (n_ids <= 0) return;

    int dest_is_stage = !strcmp(destination, "stage") || !strcmp(destination, "empty_area") || !strcmp(destination, "same_area");
    int dest_is_deck_top_or_bottom = !strcmp(destination, "deck_top_or_bottom");
    /* Rust move_cards.rs:2607-2688 dispatches on `Zone::from_str(dest)`:
       2607-2619 Stage/EmptyArea/SameArea (guarded by `zone_enum != Some(Zone::Deck)`),
       2620-2649 Deck, 2650-2662 deck_top_or_bottom, 2663-2688 the `_` fallback
       that owns the success-zone replacement check. "deck"/"main_deck" therefore
       never reaches the `_` arm, and a `deck` SOURCE never reaches the stage arm. */
    int source_is_deck = !strcmp(zone, "deck") || !strcmp(zone, "main_deck");
    int dest_is_deck = !strcmp(destination, "deck") || !strcmp(destination, "main_deck");

    int moved[RB_MAX_RECENTLY_MOVED];
    int nm = 0;

    if (dest_is_stage && !source_is_deck) {
        int out_ids[RB_MAX_RECENTLY_MOVED];
        nm = rb_move_execute_stage_placement_choices(g, actor, card_ids, n_ids,
                                                      zone, destination, -1,
                                                      target, out_ids, RB_MAX_RECENTLY_MOVED);
    } else if (dest_is_deck_top_or_bottom) {
        if (n_ids > 0) {
            rb_move_prompt_deck_top_or_bottom(g, actor, card_ids[0], target, zone, 0);
            return;
        }
    } else {
        /* Rust move_cards.rs:2671-2681 — the success-zone replacement offer
           (錯覚CROSSROADS and friends) is the FIRST thing the `_` arm does,
           before the cards leave `zone`. This is the second of the two
           construction points named at placement.rs:246-248. */
        if (!dest_is_deck && n_ids > 0 &&
            rb_move_maybe_prompt_success_replacement(g, pl, card_ids[0], destination, target))
            return;
        rb_zone_remove_at_indices(g, pl, zone, filtered, nf);
        for (int i = 0; i < n_ids; i++) {
            rb_place_card_in_zone(g, pl, card_ids[i], destination, -1);
            moved[nm++] = card_ids[i];
        }
    }

    for (int i = 0; i < nm; i++) {
        rb_mods_clear_card(&g->mods, moved[i]);
        if (g->n_selected_cards < RB_MAX_RECENTLY_MOVED)
            g->selected_cards[g->n_selected_cards++] = moved[i];
        mc_record_movement(g, moved[i]);
    }
    rb_recalc_constants(g);
    rb_clear_pending_choice(g);
}

static int move_place_looked_at(GameState *g, int pl, int cid, const char *destination,
                                const AbilityEffect *e) {
    if (!strcmp(destination, "looked_at")) {
        rb_look_add(pl, cid);
        return 1;
    }
    if (!strcmp(destination, "deck") || !strcmp(destination, "deck_top") ||
        !strcmp(destination, "deck_bottom")) {
        RbBag *deck = &g->p[pl].deck;
        if (deck->n >= RB_MAX_ZONE) return 0;
        if (!strcmp(destination, "deck_bottom")) {
            deck->cards[deck->n++] = cid;
            return 1;
        }
        const char *position = e ? cmf_extra(e, "position") : NULL;
        int nth = position ? atoi(position) : 0;
        int idx = 0;
        if (nth > 0) {
            idx = nth - 1;
            if (idx > deck->n) idx = deck->n;
        }
        for (int i = deck->n; i > idx; i--) deck->cards[i] = deck->cards[i - 1];
        deck->cards[idx] = cid;
        deck->n++;
        return 1;
    }
    return rb_place_card_in_zone(g, pl, cid, destination, -1);
}

static void move_track_looked_at(GameState *g, int cid, const AbilityEffect *e) {
    rb_mods_clear_card(&g->mods, cid);
    const char *state = e ? cmf_extra(e, "state_change") : NULL;
    if (state) rb_mods_set_orientation(&g->mods, cid, state);
    mc_record_movement(g, cid);
    if (g->n_those_cards < RB_MAX_RECENTLY_MOVED)
        g->those_cards[g->n_those_cards++] = cid;
    if (cid >= 0 && cid < RB_MAX_CARD_IDS) g->moved_this_turn[cid] = 1;
}

/* ── handle_select_cards_looked_at ── */
void rb_move_handle_select_cards_looked_at(
    GameState *g, int actor, const int *indices, int n_indices,
    const char *ctx_destination, int ctx_discard_remaining) {
    if (!g || n_indices < 0 || (n_indices > 0 && !indices)) return;
    AbilityEffect *e = g->queue.resume_eff;
    int mode = g->queue.resume_mode;
    int pl = mode == 6 ? g->queue.resume_draw_target : actor;
    if (pl < 0 || pl > 1) return;
    const char *destination = e && e->destination ? e->destination :
                              (ctx_destination ? ctx_destination : "hand");
    const char *discard = e ? cmf_extra(e, "discard_remaining") : NULL;
    int explicit_discard = discard != NULL || ctx_discard_remaining >= 0;
    int discard_remaining = discard ? (!strcmp(discard, "true") || !strcmp(discard, "1")) :
                            (ctx_discard_remaining >= 0 ? ctx_discard_remaining : 1);
    const char *rem_dest = e ? cmf_extra(e, "remainder_destination") : NULL;
    int cards[RB_MAX_ZONE], selected[RB_MAX_ZONE], ns = 0;
    int n = rb_looked_at_pool(pl, cards, RB_MAX_ZONE);
    int limit = mode == 6 ? g->queue.resume_draw_count : g->queue.pending.count;
    if (limit < 0) limit = n;
    if (n_indices > limit || n_indices > RB_MAX_ZONE) return;
    for (int i = 0; i < n_indices; i++) {
        int idx = indices[i];
        if (idx < 0 || idx >= n || (e && !rb_move_looked_at_matches(g, cards[idx], e))) return;
        for (int j = 0; j < i; j++) if (indices[j] == idx) return;
    }
    for (int i = 0; i < n; i++)
        for (int j = 0; j < n_indices; j++)
            if (indices[j] == i) selected[ns++] = cards[i];
    for (int i = 0; i < ns; i++) {
        int cid = selected[i];
        if (!rb_look_remove(pl, cid)) continue;
        if (!move_place_looked_at(g, pl, cid, destination, e)) {
            rb_look_add(pl, cid);
            return;
        }
        move_track_looked_at(g, cid, e);
        if (g->n_selected_cards < RB_MAX_RECENTLY_MOVED)
            g->selected_cards[g->n_selected_cards++] = cid;
    }
    if (mode == 6) {
        g->queue.resume_draw_count -= ns;
        g->queue.resume_draw_self_id += ns;
    }
    n = rb_looked_at_pool(pl, cards, RB_MAX_ZONE);
    int matching = 0;
    for (int i = 0; i < n; i++)
        if (!e || rb_move_looked_at_matches(g, cards[i], e)) matching++;
    if (mode == 6 && ns > 0 && g->queue.resume_draw_count > 0 && matching > 0) {
        int remaining = g->queue.resume_draw_count;
        if (remaining > matching) remaining = matching;
        rb_move_prompt_card_selection(g, actor, "looked_at", remaining, 1, e);
        rb_recalc_constants(g);
        return;
    }
    int total_selected = mode == 6 ? g->queue.resume_draw_self_id : ns;
    if (!rem_dest) {
        if (!total_selected && !explicit_discard) {
            extern int rb_look_from_deck(int pl);
            rem_dest = rb_look_from_deck(pl) ? "deck_top" : "hand";
        } else rem_dest = discard_remaining ? "discard" : "deck_top";
    }
    if (strcmp(rem_dest, "looked_at")) {
        for (int i = 0; i < n; i++) {
            int cid = cards[i];
            if (!rb_look_remove(pl, cid)) continue;
            if (move_place_looked_at(g, pl, cid, rem_dest, NULL)) {
                if (!strcmp(rem_dest, "discard") || !strcmp(rem_dest, "waitroom"))
                    move_track_looked_at(g, cid, NULL);
            } else rb_look_add(pl, cid);
        }
    }
    rb_clear_pending_choice(g);
    g->queue.resume_mode = 0;
    g->queue.resume_eff = NULL;
    rb_recalc_constants(g);
}

/* ── rb_effect_move_cards (basic move_cards entry) ── */
void rb_effect_move_cards(GameState *g, int actor, AbilityEffect *e){
    rb_move_execute_move_cards_ported(g, actor, e);
    return;
    int drain_all = (e->count < 0);
    int cnt = drain_all ? 0x7fffffff : (e->count>=0? e->count : 1);
    const char *src_s = e->source ? e->source : "hand";
    const char *dst_s = e->destination ? e->destination : "discard";
    int relay = (!strcmp(src_s,"those_cards")||!strcmp(src_s,"recently_moved")||!strcmp(src_s,"looked_at")||!strcmp(src_s,"selected_cards"));

    RbZone dst=RB_ZONE_DISCARD;
    int dst_stage=0, dst_area=-1, dst_under=0;
    int to_top = e->destination && (!strcmp(e->destination,"deck_top")||!strcmp(e->destination,"deck_top_or_bottom"));
    int to_bottom = e->destination && !strcmp(e->destination,"deck_bottom");
    if(!strcmp(dst_s,"stage")||!strcmp(dst_s,"empty_area")){ dst_stage=1; dst_area=-1; }
    else if(!strcmp(dst_s,"same_area")){ dst_stage=1; dst_area=-2; }
    else if(!strcmp(dst_s,"under_member")){ dst_stage=1; dst_area=-3; dst_under=1; }
    else if(!strcmp(dst_s,"those_cards")||!strcmp(dst_s,"recently_moved")||!strcmp(dst_s,"looked_at")){ dst=RB_ZONE_DISCARD; }
    else rb_zone_of_str(dst_s,&dst);

    int players[2]; int np=0;
    if (e->target && !strcmp(e->target,"both")) { players[np++]=actor; players[np++]=actor^1; }
    else if (e->target && !strcmp(e->target,"opponent")) { players[np++]=actor^1; }
    else { players[np++]=actor; }

    int moved_ids[RB_MAX_ZONE]; int nm=0;
    for(int pk=0; pk<np; pk++){
        RbPlayer *A=&g->p[players[pk]];
        int is_deck = (!relay && !strcmp(src_s,"deck"));

        int src_ids[RB_MAX_ZONE]; int src_area[RB_MAX_ZONE]; int ns=0;
        if(!strcmp(src_s,"looked_at")||!strcmp(src_s,"looked_at_remaining")){
            if (!strcmp(src_s, "looked_at"))
                ns = rb_move_resolve_source_looked_at(g, actor, e, players[pk], cnt, src_ids, RB_MAX_ZONE);
            else
                rb_move_resolve_from_looked_at(g, players[pk], src_ids, RB_MAX_ZONE, &ns);
            if (g->queue.has_pending) return;
            g->n_recently_moved = 0;
            g->n_those_cards = 0;
            for (int i = 0; i < ns; i++) {
                if (move_place_looked_at(g, players[pk], src_ids[i], dst_s, e)) {
                    move_track_looked_at(g, src_ids[i], e);
                    if (nm < RB_MAX_ZONE) moved_ids[nm++] = src_ids[i];
                } else rb_look_add(players[pk], src_ids[i]);
            }
            rb_recalc_constants(g);
            continue;
        } else if(relay){
            if(!strcmp(src_s,"selected_cards")){
                for(int i=0;i<g->n_selected_cards && ns<cnt;i++){ src_ids[ns]=g->selected_cards[i]; src_area[ns]=-1; ns++; }
            } else if(!strcmp(src_s,"those_cards")){
                for(int i=0;i<g->n_those_cards && ns<cnt;i++){ src_ids[ns]=g->those_cards[i]; src_area[ns]=-1; ns++; }
            } else {
                for(int i=0;i<g->n_recently_moved && ns<cnt;i++){ src_ids[ns]=g->recently_moved[i]; src_area[ns]=-1; ns++; }
            }
        } else if(!strcmp(src_s,"energy_deck")){
            for(int i=0;i<cnt && i<A->energy_deck.n;i++){
                int cid=A->energy_deck.cards[i];
                if(rb_card_is_energy(cid)){ src_ids[ns]=cid; src_area[ns]=-1; ns++; }
            }
        } else {
            RbZone src=RB_ZONE_HAND; rb_zone_of_str(src_s,&src);
            if(src==RB_ZONE_STAGE){ for(int pos=0;pos<RB_STAGE_SIZE && ns<cnt;pos++) if(A->stage[pos]>=0 && card_matches_filter(A->stage[pos],e)){ src_ids[ns]=A->stage[pos]; src_area[ns]=pos; ns++; } }
            else { RbBag *sb=NULL;
                if(src==RB_ZONE_HAND) sb=&A->hand; else if(src==RB_ZONE_DECK) sb=&A->deck;
                else if(src==RB_ZONE_DISCARD) sb=&A->discard; else if(src==RB_ZONE_ENERGY) sb=&A->energy;
                else if(src==RB_ZONE_LIVE) sb=&A->live; else if(src==RB_ZONE_SUCCESS) sb=&A->success;
                if(sb){ for(int i=sb->n-1;i>=0 && ns<cnt;i--) if(card_matches_filter(sb->cards[i],e)){ src_ids[ns]=sb->cards[i]; src_area[ns]=-1; ns++; } }
            }
        }

        int moved=0;
        for(int i=0;i<ns;i++){
            int cid=src_ids[i];
            if(!strcmp(src_s,"energy_deck")){
                int found=0;
                for(int j=0;j<A->energy_deck.n;j++) if(A->energy_deck.cards[j]==cid){
                    for(int k=j;k<A->energy_deck.n-1;k++) A->energy_deck.cards[k]=A->energy_deck.cards[k+1];
                    A->energy_deck.n--; found=1; break;
                }
                if(!found) continue;
            } else if(!find_and_remove_card(A,cid)) continue;
            if(dst_stage){
                int area=dst_area;
                if(area==-2) area=src_area[i];
                if(area==-3){ area=src_area[i]; dst_under=1; }
                if(area<0){ for(int q=0;q<RB_STAGE_SIZE;q++) if(A->stage[q]<0){ area=q; break; } }
                place_in_dst(A,dst,cid,to_top,to_bottom,area,dst_under,1);
            } else {
                place_in_dst(A,dst,cid,to_top,to_bottom,-1,0,0);
            }
            moved_ids[nm++]=cid; moved++;
        }
        if(is_deck && moved < cnt){
            while(moved < cnt){
                if(A->deck.n==0){
                    if(A->discard.n>0){
                        rb_shuffle(A->discard.cards, A->discard.n);
                        for(int k=0;k<A->discard.n;k++) A->deck.cards[A->deck.n++]=A->discard.cards[k];
                        A->discard.n=0;
                        A->deck_refreshed_this_turn=1;
                    } else break;
                }
                int cid=A->deck.cards[--A->deck.n];
                if(dst_stage){
                    int area=dst_area;
                    if(area<0){ for(int q=0;q<RB_STAGE_SIZE;q++) if(A->stage[q]<0){ area=q; break; } }
                    place_in_dst(A,dst,cid,to_top,to_bottom,area,dst_under,1);
                } else {
                    place_in_dst(A,dst,cid,to_top,to_bottom,-1,0,0);
                }
                moved_ids[nm++]=cid; moved++;
            }
        }
    }
    g->n_recently_moved = nm < RB_MAX_RECENTLY_MOVED ? nm : RB_MAX_RECENTLY_MOVED;
    for(int i=0;i<g->n_recently_moved;i++) g->recently_moved[i]=moved_ids[i];
    g->n_those_cards = nm < RB_MAX_RECENTLY_MOVED ? nm : RB_MAX_RECENTLY_MOVED;
    for(int i=0;i<g->n_those_cards;i++) g->those_cards[i]=moved_ids[i];
    for(int i=0;i<nm;i++) g->moved_this_turn[moved_ids[i]] = 1;
    const char *state_change = cmf_extra(e, "state_change");
    if (state_change && *state_change)
        rb_move_finalize_card_movement(g, actor, moved_ids, nm, dst_s, src_s,
                                       state_change, e->target);
}
