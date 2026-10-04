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
void rb_remove_revealed_card(GameState *g, int card_id);
/* Defined in src/ability/util.c; not declared in rabuka.h. */
int  rb_dedupe_by_normalized_name(const int *items, int n, int *out, int max);
/* Defined in src/core/zones.c; not declared in rabuka.h. Mirrors
 * Player::remove_member_from_stage_with_recycling (player.rs) — the ONLY twin
 * that recycles the departing member's under-cards (energy -> energy deck,
 * members -> waitroom, Rule 10.5.3-10.5.4) and clears its deployment lock. */
int  rb_player_remove_member_from_stage_with_recycling(GameState *g, int pl, int index);

/* ── shared helpers (Rust twins named in the comments) ─────────────────── */

/* `effect.foo_any()` — Rust reads the wire key anywhere on the effect tree's
 * own fields; the C bytecode decoder flattens them into extra_k/extra_v, so
 * cmf_extra IS the lookup. `optional` / `max` / `all` / `self_cost` /
 * `exclude_self` are named fields in Rust and `is_optional` here, so they need
 * their own accessors for the optional-family keys that ARE flattened. */
/* Forward declaration: cmf_extra is defined at ~line 151 but first used at the
 * line below. Without this the compiler saw an implicit declaration and then a
 * conflicting-type error, which blocked the entire tree from building. */
static const char *cmf_extra(const AbilityEffect *e, const char *k);

/* Defined in src/ability/util.c and used in two places in this file. Declared
 * here at file scope because the first use precedes any visible declaration -
 * a block-scope declaration inside the calling function is NOT visible to the
 * second call site, which is what produced the remaining build error. Note
 * util.c's own forward declaration must NOT be static; that mismatch broke
 * tree-wide linking once already. */
int rb_get_selection_indices_filter(const int *cards, int n, const RbCardFilter *filter,
                                    int self_target_only, int activating_card,
                                    int *out_idx, int max);

static int cmf_extra_true(const AbilityEffect *e, const char *k) {
    const char *v = cmf_extra(e, k);
    return v && (!strcmp(v, "true") || !strcmp(v, "1"));
}
/* selection.rs:71 `effect.is_self_target()` — the ONLY writer of the prompt's
 * target_player_id besides action_by (selection.rs:40-43). */
static int cmf_is_self_target(const AbilityEffect *e) {
    return e && e->self_target_field[0] && !strcmp(e->self_target_field, "true");
}

/* The C home of Rust `self.selection.moved_cards` (the resolver-local list of
 * cards THIS activation moved). `g->those_cards` is documented in rabuka.h as
 * "cards moved by the immediately preceding move_cards action", which is the
 * same lifetime, and every reader below (the under-member host fallback,
 * `preceding_moved`, `those_cards`) wants that list. See CROSS-FILE REQUESTS
 * for the field-addition request. */
static int mc_sel_moved_count(const GameState *g) { return g->n_those_cards; }
static const int *mc_sel_moved(const GameState *g) { return g->those_cards; }
static void mc_sel_moved_push(GameState *g, int cid) {
    if (cid < 0) return;
    for (int i = 0; i < g->n_those_cards; i++) if (g->those_cards[i] == cid) return;
    if (g->n_those_cards < RB_MAX_RECENTLY_MOVED)
        g->those_cards[g->n_those_cards++] = cid;
}
/* ── TOMBSTONE (2026-10-04). `mc_set_recently_moved_batch` REMOVED.
 *
 * A census classified this DEAD-WRONG: "comments claim it REPLACES the batch;
 * being unreferenced, no live path performs that replacement; g->those_cards is
 * therefore only ever APPENDED to". Checked, and BOTH halves of that are wrong:
 *
 *  (1) The replace idiom is not missing. `g->n_recently_moved = 0` followed by
 *      a bounded append is implemented on ~20 LIVE paths in this tree --
 *      effects/look.c:822+827, effects/misc.c:172, effects/move_missing.c:416,
 *      effects/state.c:1613, ability/ability_queue.c:1420+1569,
 *      ability/choice.c:2299, core/game_state_core.c:485
 *      (rb_gsc_set_recently_moved_cards), turn/live.c:1875-1876, engine.c
 *      (several). This function was a dead DUPLICATE of that idiom, not its
 *      only implementation.
 *  (2) `those_cards` is NOT only ever appended. It is reset to 0 at
 *      move.c:2525 and move_missing.c:417. The census compared two different
 *      fields: `recently_moved` (what this function wrote) and `those_cards`
 *      (the resolver-local accumulator that mc_sel_moved_push above appends to
 *      BY DESIGN, with an intentional dedupe). mc_sel_moved_push is not Rust's
 *      set_recently_moved_batch and is not meant to be.
 *
 * The body was faithful to Rust mod.rs:1683; it was simply the 21st copy of an
 * idiom the tree already had 20 live copies of, with no callers. Deleted so the
 * next reader does not wire it "to activate the code" and get a 21st
 * implementation of something already correct.
 *
 * Deadness was proved by the compiler, not by grep: gcc -Wunused-function on
 * this file reports "'mc_set_recently_moved_batch' defined but not used". It
 * was `static`, so no other translation unit could have been calling it. */
/* mod.rs:2527-2533 — the CURRENT entry's player id ("p1"/"p2") as an index. */
static int mc_cause_player(const GameState *g) {
    if (!g) return 0;
    if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
        const char *pid = g->queue.entries[g->queue.cur].player_id;
        if (pid && pid[0] == 'p' && pid[1] >= '1' && pid[1] <= '2')
            return pid[1] - '1';
    }
    return 0;
}
/* mod.rs:2499-2547 `finalize_card_movement`'s and mod.rs:2947-2952
 * `execute_selected_cards_from_zone`'s `gs.push_movement_event(...)`.
 * rb_record_card_movement is the folded twin (modifiers.c:496-529): it writes
 * batch_movements, turn_movements, recently_moved_cards AND cards_moved_this_turn,
 * so it REPLACES the local mc_record_movement rather than following it. */
static void mc_push_movement_event(GameState *g, const int *cards, int n,
                                   const char *source, const char *dest) {
    if (!g || !cards || n <= 0) return;
    int src = (int)rb_zone_id_from_str(source);
    int dst = (int)rb_zone_id_from_str(dest);
    int pid = mc_cause_player(g);
    for (int i = 0; i < n; i++)
        rb_record_card_movement(g, cards[i], src, dst, pid, 1);
}


/* ── card_matches_filter: full effect filter (static for this TU) ── */
/* The `!e || !k` guard is the tree's strictest extra-accessor policy and the
 * one this function's own sibling uses (move_missing.c:432 `cmf_extra`, the
 * same name and the same port). It was MISSING here, and every one of this
 * file's ~80 call sites dereferences `e` through this accessor, so a NULL
 * effect crashed on the first one it reached.
 *
 * It is not hypothetical. move.c:1380 read
 *     if (is_discard) can_skip = cmf_extra_true(e, "max") || (e && e->is_optional);
 * -- the only reference to `e` in that whole block written WITHOUT the `e &&`
 * that its immediate neighbours 1379/1381/1382/1384 all carry. So the one
 * reachable-with-NULL line in rb_move_take_cards_from_standard_zone (a
 * non-static function whose entire body is otherwise written to tolerate a
 * NULL effect: `if (e && ...)`, `e ? cmf_extra(e,...) : NULL`, and a forward
 * declaration in a second TU) faulted. Guarding the accessor rather than that
 * one line fixes all ~80 sites, not one. */
static const char *cmf_extra(const AbilityEffect *e, const char *k){
    if(!e || !k) return NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],k)) return e->extra_v[i];
    return NULL;
}
/* `card_type` is one of the few keys the bytecode decoder does NOT fold into
   the extras table: vm.c:996-1003 diverts it into the struct's `card_type_field`
   and `continue`s, so it never reaches effect_set_extra. Every reader here must
   therefore consult the field FIRST and only then the extras, exactly as
   draw.c:420 and compound.c's eff_card_type_any do. Reading only cmf_extra made
   every 「メンバーカード」 / 「ライブカード」 / 「エネルギーカード」 source filter a
   silent no-op: q242 (PL!HS-pb1-012-R) moved the energy card and the live card
   out of the waitroom alongside the members. */
static const char *cmf_card_type(const AbilityEffect *e){
    if(!e) return NULL;
    if(e->card_type_field[0]) return e->card_type_field;
    return cmf_extra(e, "card_type");
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

/* ── Zone::from_str alias table ─────────────────────────────────────────
 *
 * placement.rs and mod.rs gate on `Zone::from_str(x) == Some(Zone::V)`, and
 * enums.rs:92-122 is a WIRE-NAME table in which several names fold onto one
 * variant: "energy" | "energy_zone" -> Energy, "waitroom" is its OWN variant
 * (not an alias of Discard), "success_live_zone" | "success_live_card_zone" ->
 * SuccessLiveZone, "under_member" | "under" -> UnderMember. The C has no Zone
 * enum, and the previous code compared raw strings, so an alias took the wrong
 * branch (most visibly: destination="energy_zone" never reached the
 * state_change=active counter bump, and dest="success_live_card_zone" was
 * special-cased by hand at each site instead of once, here). */
static int move_zone_is(const char *z, const char *variant) {
    if (!z || !variant) return 0;
    struct { const char *v; const char *names[5]; } T[] = {
        {"hand",          {"hand", NULL}},
        {"stage",         {"stage", NULL}},
        {"discard",       {"discard", NULL}},
        {"waitroom",      {"waitroom", NULL}},
        {"energy",        {"energy", "energy_zone", NULL}},
        {"deck",          {"deck", NULL}},
        {"deck_top",      {"deck_top", NULL}},
        {"deck_bottom",   {"deck_bottom", NULL}},
        {"energy_deck",   {"energy_deck", NULL}},
        {"live_card_zone",{"live_card_zone", NULL}},
        {"success_live_zone", {"success_live_zone", "success_live_card_zone", NULL}},
        {"success_zone",  {"success_zone", NULL}},
        {"empty_area",    {"empty_area", NULL}},
        {"same_area",     {"same_area", NULL}},
        {"under_member",  {"under_member", "under", NULL}},
        {"looked_at",     {"looked_at", NULL}},
        {"revealed_cards",{"revealed_cards", NULL}},
        {"selected_cards",{"selected_cards", NULL}},
        {NULL, {NULL}}
    };
    for (int i = 0; T[i].v; i++) {
        if (strcmp(T[i].v, variant)) continue;
        for (int k = 0; k < 5 && T[i].names[k]; k++)
            if (!strcmp(T[i].names[k], z)) return 1;
        return 0;
    }
    return 0;
}

/* ── CSV group lists (engine/src/core/card.rs:2007 `group_names_any`) ─────
 *
 * Rust models `group_names` as a Vec<String> and `group_name()` (card.rs:2446)
 * takes one entry. The C bytecode decoder flattens the whole list into ONE
 * comma-joined extra value (e.g. "Aqours,SaintSnow" — 24 decoded effects carry
 * such a value, see PL!S-bp5-004-R ab#0 and PL!HS-bp2-020-L ab#0), and
 * rb_card_matches_group_str (util.c:339) compares that whole string as ONE
 * exact token against the card's unit / group / name. No card is called
 * "Aqours,SaintSnow", so every multi-group filter matched NOTHING: the
 * candidate set came back empty, no prompt was raised, and the effect
 * silently no-opped.
 *
 * cost.c:163 already has the working split_csv twin for exactly this and
 * util.c:403 rb_card_matches_any_group is the working matcher. This is the
 * same split feeding the same matcher, applied on the move.c paths. */
static int move_split_csv(const char *s, char out[][64], int max){
    if(!s||!*s) return 0;
    int n=0; const char *p=s;
    while(*p && n<max){
        const char *comma=strchr(p,',');
        size_t len=comma?(size_t)(comma-p):strlen(p);
        if(len>=64) len=63;
        memcpy(out[n],p,len); out[n][len]='\0'; n++;
        if(!comma) break;
        p=comma+1;
    }
    return n;
}
/* Pass if the CSV is empty or the card matches ANY of its entries. */
static int move_card_matches_group_csv(int card_idx, const char *group_csv){
    if(!group_csv||!*group_csv) return 1;
    char groups[16][64];
    int ng=move_split_csv(group_csv,groups,16);
    const char *names[16];
    for(int i=0;i<ng;i++) names[i]=groups[i];
    return rb_card_matches_any_group(card_idx,names,ng);
}
/* ── card_matches_filter: the ONE builder + the ONE matcher ───────────────
 * Rust has no function of this name. It is this port's own "does the printed
 * filter on this effect accept this card" predicate, used by
 * rb_move_looked_at_matches and by the revealed_cards / stage / waitroom source
 * scans below — a FOURTH hand-rolled walk over the same extras that
 * rb_effect_filter_subset (core/card.c, the twin of util.rs:1819-1884
 * CardFilter::from_effect) and rb_matching_ids (util.c local_filter_matches,
 * the twin of util.rs:1676-1688 CardFilter::matches) already own.
 *
 * It now READS every field it can from that one builder and runs that one
 * matcher. Each term the central pair still cannot carry is listed below with
 * the Rust line that says so; none of them re-implements a central term. */
static int card_matches_filter(int card_idx, AbilityEffect *e){
    RbCardFilter f;
    rb_effect_filter_subset(e, &f);

    /* util.rs:1327-1332 — `current_blade_limit` is deliberately NOT a
       matches() term: Rust applies it after matches() via filter_current_blade,
       which needs the modifier table matches() has no access to. card.c:937
       fills has_blade_limit from the plain `blade_limit` key regardless, so it
       is switched off here instead of narrowing every looked-at card. */
    f.has_blade_limit = 0;
    /* util.rs:1833-1834 `characters` / `exclude_characters` are Vec<String> and
       matches() tests them with card_matches_characters (util.rs:1723-1731).
       RbCardFilter carries each as ONE comma-joined string and util.c:1356-1369
       wraps it as a single name, so a multi-name card could never match it;
       this path's own group-CSV reading is kept below instead. */
    f.has_characters = 0;
    f.has_exclude_characters = 0;
    /* util.rs:1821-1825 + check_group (util.rs:1484-1499) keep BOTH `group`
       (group_names.first()) and `groups` (the whole Vec) and accept a card
       matching ANY entry. RbCardFilter has no list member and util.c:1322 keeps
       only the first, so a paired list like 「μ's,μ's'」 would reject every card
       here. The list is decided below, over the whole CSV. */
    f.group[0] = '\0';
    f.has_group = 0;

    int one[1]; one[0] = card_idx;
    int out[1];
    if (rb_matching_ids(&f, one, 1, out, 1) != 1) return 0;

    const char *gn = cmf_extra(e, "group_names");
    if (gn && !move_card_matches_group_csv(card_idx, gn)) return 0;
    /* `name_fragments` is a different wire key from `card_names`: card.c:928-930
       builds name_fragments out of card_names only, so this spelling still has
       to be read straight off the effect. */
    const char *nf = cmf_extra(e,"name_fragments");
    if(nf){
        Card c; if(!rb_decode_card_by_index((uint32_t)card_idx,&c)) return 0;
        int m = c.name && strstr(c.name, nf); rb_free_card(&c);
        if(!m) return 0;
    }
    /* util.rs:1713-1722 — cost_limit_min / cost_limit_max are separate
       CardFilter fields (util.rs:1312-1314) that RbCardFilter does not carry
       and card.c never reads. */
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
/* card.c:912-921 reads the blade bound from `blade_limit` plus
       `original_blade_operator`/`original_value`, NOT from an
       `original_blade_limit` key, so this spelling still has to be read here. */
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
    if(chars && !move_card_matches_group_csv(card_idx, chars)) return 0;
    const char *exch = cmf_extra(e,"exclude_characters");
    if(exch && move_card_matches_group_csv(card_idx, exch)) return 0;
/* util.rs:1818 / check_card_property (util.rs:1654-1673). card.c:908-911
       populates the member and util.c:42-46 reads it, both behind
       RB_CARD_FILTER_CARD_PROPERTY, which no build defines — so neither the
       builder nor the matcher carries it yet and the test stays here. It now
       goes through rb_check_card_property, the central helper, instead of a
       private three-case decode, which restores the `has_draw_icon` arm
       util.rs:1660 has and this copy was missing. */
    const char *cp = cmf_extra(e, "card_property");
    if(cp){
        const char *ng = cmf_extra(e, "negation");
        int neg = ng && !strcmp(ng,"true");
        Card c;
        const Card *card = rb_decode_card_by_index((uint32_t)card_idx,&c) ? &c : NULL;
        int passes = rb_check_card_property(cp, neg, card);
        if (card) rb_free_card(&c);
        if (!passes) return 0;
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

/* ── under_member placement (Rule 4.5.5) ───────────────────────────────
 *
 * Two Rust steps that the C had collapsed into "the card disappears":
 *
 *   placement.rs:208-219  pos_to_use = the area of the ACTIVATING member,
 *                         else the vacated area, else the last
 *                         self.selection.moved_cards entry still on stage.
 *   util.rs:2341-2358     place_card_in_zone(Zone::UnderMember, pos_to_use):
 *                         that area if pos_to_use is Some, else
 *                         center -> left -> right, and — if the stage is
 *                         completely empty — the WAITROOM.
 *
 * rb_place_card_in_zone (util.c:836) has no under_member arm, and the
 * zone_bag() it dispatches through (util.c:797) returns NULL for the zone,
 * so every move_cards(destination="under_member") removed the card from its
 * source and then dropped it: under_cards stayed [0 0 0] for all three areas
 * and the card was in no zone at all. That is the confirmed "card vanishes
 * into no zone" class; this is the landing that was missing. */

int rb_move_under_member_pos(GameState *g, int pl) {
    if (!g) return -1;
    RbPlayer *P = &g->p[pl];
    int act = g->activating_card;
    if (act < 0) act = g->queue.resume_host;
    /* placement.rs:210-215 — the activating card's own area wins. */
    if (act >= 0)
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] == act) return i;
    /* placement.rs:216 — else the last recently-moved card still on stage. */
    for (int m = g->n_recently_moved - 1; m >= 0; m--)
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] == g->recently_moved[m]) return i;
    return -1;
}

int rb_move_place_card_under_member(GameState *g, int pl, int cid, int pos_to_use) {
    if (!g) return 0;
    RbPlayer *P = &g->p[pl];
    int target_idx = -1;
    if (pos_to_use >= 0 && pos_to_use < RB_STAGE_SIZE) target_idx = pos_to_use;
    /* util.rs:2344-2355 — vacated_stage_area, else center, left, right. */
    if (target_idx < 0 && P->stage[1] != RB_EMPTY_SLOT) target_idx = 1;
    if (target_idx < 0 && P->stage[0] != RB_EMPTY_SLOT) target_idx = 0;
    if (target_idx < 0 && P->stage[2] != RB_EMPTY_SLOT) target_idx = 2;
    /* util.rs:2352-2354 — an entirely empty stage takes the card to the
       waitroom; Rule 10.5.4's energy-deck case is place_energy_under_member_
       selected's own fallback, not this one. */
    if (target_idx < 0) { rb_waitroom_add(P, cid); return 1; }
    rb_stage_place_under_card(P, target_idx, cid);
    return 1;
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

/* ── move_build_standard_zone_filter ───────────────────────────────────────
 * The filter assembly Rust's resolve_from_standard_zone does per SOURCE zone
 * (move_cards/mod.rs:1209-1274), factored out of
 * rb_move_take_cards_from_standard_zone so that the prompt itself can rebuild
 * the identical filter. Rust passes BOTH `filter` and `filtered_indices` into
 * `prompt_card_selection` (selection.rs:6-14), so the two are built from one
 * description of the source; before this helper existed only the caller's copy
 * existed, which is why the prompt could not fill filtered_indices itself. */
static void move_build_standard_zone_filter(GameState *g, const char *zone_name,
                                            const AbilityEffect *e, RbCardFilter *f) {
    memset(f, 0, sizeof *f);
    int is_discard = move_zone_is(zone_name, "discard");
    int is_hand    = move_zone_is(zone_name, "hand");
    int is_lcz     = move_zone_is(zone_name, "live_card_zone");
    int is_success = move_zone_is(zone_name, "success_live_zone");
    int is_energy  = move_zone_is(zone_name, "energy");
    /* mod.rs:1209-1274 — which optional filters are meaningful for THIS source.
     * A live-card-zone move is always a live card; an energy-zone move carries
     * no group or cost filter at all; only hand/discard can be narrowed by
     * name fragments or a total cost. */
    const char *ctype_raw = e ? cmf_card_type(e) : NULL;
    const char *gn_raw    = e ? cmf_extra(e, "group_names") : NULL;
    const char *frag_raw  = e ? cmf_extra(e, "name_fragments") : NULL;
    const char *ct_raw    = e ? cmf_extra(e, "cost_total") : NULL;
    const char *cto_raw   = e ? cmf_extra(e, "cost_total_operator") : NULL;
    const char *clo_raw   = e ? cmf_extra(e, "cost_limit_operator") : NULL;
    if (!clo_raw) clo_raw = e ? cmf_extra(e, "cost_operator") : NULL;
    const char *exch_raw  = e ? cmf_extra(e, "exclude_characters") : NULL;
    const char *cp_raw    = e ? cmf_extra(e, "card_property") : NULL;
    int from_hand_or_discard = is_hand || is_discard;
    int is_live_zone_move = is_lcz || (is_success && ctype_raw != NULL);
    (void)gn_raw;

    if (is_live_zone_move)       strncpy(f->card_type, "live_card", sizeof f->card_type - 1);
    else if (is_success)         f->card_type[0] = '\0';
    else if (ctype_raw && ctype_raw[0]) strncpy(f->card_type, ctype_raw, sizeof f->card_type - 1);
    /* mod.rs:1225-1229 — `group` is None whenever the effect carries its own
     * group_names, so nothing is written here; those names ride the `groups`
     * list instead (mod.rs:1266-1268) and are applied by
     * move_apply_effect_groups. Setting f->group from gn_raw under this guard
     * was dead code and the group filter ended up dropped entirely. */
    /* mod.rs:1266-1268 — effect.group_names_any() ALSO rides the `groups`
     * list when present, so a multi-group name list filters correctly. */
    int limit = rb_move_resolve_cost_limit_reference(g, e);
    if (!is_energy && limit >= 0) {
        f->has_cost_limit = 1; f->cost_limit = limit;
        if (clo_raw) strncpy(f->cost_op, clo_raw, sizeof f->cost_op - 1);
    }
    if (from_hand_or_discard && ct_raw) {
        f->has_cost_total = 1; f->cost_total = atoi(ct_raw);
        if (cto_raw) strncpy(f->cost_total_op, cto_raw, sizeof f->cost_total_op - 1);
    }
    if (from_hand_or_discard && frag_raw && frag_raw[0]) {
        /* CSV split into the fixed name_fragments array. */
        char buf[256];
        strncpy(buf, frag_raw, sizeof buf - 1);
        buf[sizeof buf - 1] = '\0';
        char *tok = strtok(buf, ",");
        while (tok && f->n_name_fragments < 8) {
            strncpy(f->name_fragments[f->n_name_fragments], tok,
                    sizeof f->name_fragments[0] - 1);
            f->n_name_fragments++;
            tok = strtok(NULL, ",");
        }
    }
    { const char *ch = e ? cmf_extra(e, "characters") : NULL;
      if (ch && ch[0]) { strncpy(f->characters, ch, sizeof f->characters - 1); f->has_characters = 1; } }
    if (exch_raw && exch_raw[0]) {
        strncpy(f->exclude_characters, exch_raw, sizeof f->exclude_characters - 1);
        f->has_exclude_characters = 1;
    }
    if (cp_raw && cp_raw[0]) strncpy(f->ability_filter, cp_raw, sizeof f->ability_filter - 1);
    f->negation = e && cmf_extra_true(e, "negation");
    const char *nht = e ? cmf_extra(e, "need_heart_total") : NULL;
    if (nht) { f->has_need_heart_total = 1; f->need_heart_total = atoi(nht); }
    const char *nho = e ? cmf_extra(e, "need_heart_operator") : NULL;
    if (nho) strncpy(f->need_heart_operator, nho, sizeof f->need_heart_operator - 1);
    const char *nhc = e ? cmf_extra(e, "need_heart_color") : NULL;
    if (nhc) strncpy(f->need_heart_color, nhc, sizeof f->need_heart_color - 1);
    f->has_filter = f->card_type[0] || f->has_group || f->has_cost_limit || f->has_cost_total ||
                    f->has_characters || f->has_exclude_characters || f->ability_filter[0] ||
                    f->n_name_fragments || f->has_need_heart_total || f->need_heart_color[0];
}

/* ── move_exclude_stage_groups ─────────────────────────────────────────────
 * Rust mod.rs:1290-1333 — `group_reference == "different_group_names"` on a
 * DISCARD source excludes every discard card whose group matches ANY stage
 * member's group (Q89/Q105/Q208). Rust puts that in the FILTER
 * (`filter.exclude_group_names`), so it is part of `matching_indices` and
 * therefore part of the `filtered_indices` Rust hands the prompt
 * (selection.rs:72 / :85). RbCardFilter has no exclude_group_names field, so
 * the C applies it as a post-pass over the candidate list — which means BOTH
 * the classifying caller and the prompt have to run it, or the two disagree
 * about what was offered. Skipped when the CONDITION already carries the same
 * reference (Mia option 1 — the condition does the counting, so excluding again
 * over-filters). Writes the distinct stage group names, returns their count. */
static int move_collect_stage_group_exclusions(GameState *g, int pl,
                                                const char *zone_name,
                                                const AbilityEffect *e,
                                                char out[][64], int max) {
    if (!g || !out || max <= 0) return 0;
    if (!e || !move_zone_is(zone_name, "discard")) return 0;
    const char *gref = cmf_extra(e, "group_reference");
    if (!gref || strcmp(gref, "different_group_names")) return 0;
    RbPlayer *P = &g->p[pl];
    int nsg = 0;
    for (int i = 0; i < RB_STAGE_SIZE && nsg < max; i++) {
        int cid = P->stage[i];
        if (cid < 0) continue;
        char cand[64];
        cand[0] = '\0';
        const char *grp = rb_card_group_name(cid);
        if (grp && grp[0]) {
            snprintf(cand, sizeof cand, "%s", grp);
        } else {
            static const char *known[] = {
                "Aqours", "Liella!", "Nijigasaki", "A-RISE",
                "Saint Snow", "Hasunosora", NULL};
            for (int k = 0; known[k]; k++)
                if (rb_card_matches_group_str(cid, known[k])) {
                    snprintf(cand, sizeof cand, "%s", known[k]);
                    break;
                }
        }
        if (!cand[0]) continue;
        int dup = 0;
        for (int k = 0; k < nsg; k++)
            if (!strcmp(out[k], cand)) { dup = 1; break; }
        if (!dup) {
            strncpy(out[nsg], cand, sizeof out[0] - 1);
            out[nsg][sizeof out[0] - 1] = '\0';
            nsg++;
        }
    }
    return nsg;
}

/* ── move_apply_effect_groups ──────────────────────────────────────────────
 * Rust mod.rs:1266-1268 — when the EFFECT carries group_names, they do NOT go
 * into `filter.group` (mod.rs:1225-1229 leaves that None when
 * has_effect_groups); they go into `filter.groups`, the LIST, and
 * `check_group` (util.rs:1495-1499) then requires the card to match ANY entry.
 *
 * RbCardFilter has no list field, and the old `has_group` assignment was
 * unreachable anyway: `has_effect_groups` is literally `gn_raw[0]`, so the
 * `!(has_effect_groups || ...) && gn_raw[0]` guard could only be true when
 * gn_raw was empty. The group filter was therefore DROPPED, not relocated —
 * while rb_move_prompt_card_selection still stamped `filter_group` on the
 * choice. The prompt therefore advertised group candidates while its candidate
 * list was unfiltered, and choice_discard_pick_legal (choice.c) then rejected
 * every offered card that did not belong to the group: measured as
 * [DISCARD_PICK_REJECTED] in tests/parity_mill_deck.log.
 *
 * Applied as a post-pass over the candidate list, exactly like Rust applies it
 * inside matching_indices, so the classifying producer and the prompt agree.
 * The CSV split + any-match matcher are the same ones card_matches_filter uses
 * (mod.rs:1267 passes the whole Vec, not just the first entry). */
static int move_apply_effect_groups(const int *cards, int *idxs, int n,
                                     const char *group_csv) {
    if (!group_csv || !group_csv[0] || n <= 0) return n;
    char groups[16][64];
    int ng = move_split_csv(group_csv, groups, 16);
    if (ng <= 0) return n;
    const char *names[16];
    for (int i = 0; i < ng; i++) names[i] = groups[i];
    int w = 0;
    for (int i = 0; i < n; i++) {
        int idx = idxs[i];
        if (idx < 0 || idx >= RB_MAX_ZONE) continue;
        if (!rb_card_matches_any_group(cards[idx], names, ng)) continue;
        idxs[w++] = idx;
    }
    return w;
}

/* Drop every candidate whose card belongs to one of the excluded stage groups.
 * Returns the surviving count, preserving the input order. */
static int move_exclude_stage_groups(const int *cards, int *idxs, int n,
                                     char groups[][64], int nsg) {
    if (nsg <= 0) return n;
    int w = 0;
    for (int i = 0; i < n; i++) {
        int idx = idxs[i];
        int blocked = 0;
        for (int k = 0; k < nsg && !blocked; k++)
            if (idx >= 0 && rb_card_matches_group_str(cards[idx], groups[k])) blocked = 1;
        if (blocked) continue;
        idxs[w++] = idx;
    }
    return w;
}

/* ── move_build_stage_filter ───────────────────────────────────────────────
 * Rust mod.rs:1044-1071 assembles the filter for a STAGE source. It is a
 * different assembly from move_build_standard_zone_filter (cost_total is not
 * restricted to hand/discard here, and `exclude_self` exists), so it is its own
 * helper rather than a shared one — but both the classifying producer and the
 * prompt must use the SAME copy, exactly as selection.rs:6-14 hands Rust's
 * prompt the one filter the classification ran on. */
static void move_build_stage_filter(GameState *g, const AbilityEffect *e, RbCardFilter *f) {
    memset(f, 0, sizeof *f);
    const char *ctype = cmf_card_type(e);
    const char *gn = cmf_extra(e, "group_names");
    if (ctype) strncpy(f->card_type, ctype, sizeof f->card_type - 1);
    if (gn && gn[0]) { strncpy(f->group, gn, sizeof f->group - 1); f->has_group = 1; }
    const char *clo = cmf_extra(e, "cost_limit_operator");
    if (!clo) clo = cmf_extra(e, "cost_operator");
    int limit = rb_move_resolve_cost_limit_reference(g, e);
    if (limit >= 0) { f->has_cost_limit = 1; f->cost_limit = limit;
                      if (clo) strncpy(f->cost_op, clo, sizeof f->cost_op - 1); }
    const char *cto = cmf_extra(e, "cost_total");
    if (cto) { f->has_cost_total = 1; f->cost_total = atoi(cto); }
    const char *ctoo = cmf_extra(e, "cost_total_operator");
    if (ctoo) strncpy(f->cost_total_op, ctoo, sizeof f->cost_total_op - 1);
    const char *chars = cmf_extra(e, "characters");
    if (chars && chars[0]) { strncpy(f->characters, chars, sizeof f->characters - 1); f->has_characters = 1; }
    const char *exch = cmf_extra(e, "exclude_characters");
    if (exch && exch[0]) { strncpy(f->exclude_characters, exch, sizeof f->exclude_characters - 1); f->has_exclude_characters = 1; }
    const char *cp = cmf_extra(e, "card_property");
    if (cp) { strncpy(f->ability_filter, cp, sizeof f->ability_filter - 1); }
    f->negation = cmf_extra_true(e, "negation");
    if (e && cmf_extra_true(e, "exclude_self")) {
        f->has_exclude_self = 1; f->exclude_self_id = g->activating_card;
    }
    f->has_filter = f->has_group || f->has_cost_limit || f->has_cost_total ||
                   f->has_characters || f->has_exclude_characters ||
                   f->ability_filter[0] || f->has_exclude_self || (ctype && ctype[0]);
}

/* ── rb_move_prompt_fill_filtered ──────────────────────────────────────────
 * selection.rs:44 `.filtered_indices(filtered_indices)` — the candidate list
 * the prompt ADVERTISES, in OFFER order, so display index i addresses
 * filtered_indices[i]. map_choice_index (choice.c) reads exactly these two
 * fields; with them empty it is the IDENTITY, which means a pick the player
 * made against a filtered candidate list is read as a RAW ZONE position and
 * lands on an INELIGIBLE card.
 *
 * The list is rebuilt here from the same zone + the same filter assembly the
 * classifying producer used, so the prompt and the classification cannot drift.
 * Producers that already know their own candidate array (rb_move_take_cards_
 * standard_zone, rb_move_resolve_from_stage, rb_move_resolve_from_under_member)
 * overwrite this with their list, which is the same list by construction.
 *
 * Zones that are NOT player zones (looked_at, revealed_cards, those_cards,
 * selected_cards, recently_moved) are left untouched: rb_zone_cards returns 0
 * for them and their answers index the pool directly. */
static void rb_move_prompt_fill_filtered(GameState *g, int actor, const char *zone,
                                         const AbilityEffect *e) {
    if (!g || !zone) return;
    /* mod.rs:1046-1048 — a STAGE candidate index is a STAGE POSITION, and
     * rb_zone_cards compacts empty slots away, so the generic zone walk cannot
     * produce them. resolve_from_stage supplies that list itself. */
    if (move_zone_is(zone, "stage")) return;
    int pl = actor;
    if (e && e->target && !strcmp(e->target, "opponent")) pl = actor ^ 1;
    if (pl < 0 || pl > 1) pl = 0;
    int cards[RB_MAX_ZONE];
    int n = rb_zone_cards(g, pl, zone, cards, RB_MAX_ZONE);
    if (n <= 0) return;
    RbCardFilter f;
    move_build_standard_zone_filter(g, zone, e, &f);
    int idxs[RB_MAX_ZONE];
    int nf = rb_get_selection_indices_filter(cards, n, &f, cmf_is_self_target(e),
                                             g->activating_card, idxs, RB_MAX_ZONE);
    if (nf > RB_MAX_ZONE) nf = RB_MAX_ZONE;
    char groups[8][64];
    int nsg = move_collect_stage_group_exclusions(g, pl, zone, e, groups, 8);
    if (nsg > 0) nf = move_exclude_stage_groups(cards, idxs, nf, groups, nsg);
    { const char *gn = cmf_extra(e, "group_names");
      if (gn && gn[0]) nf = move_apply_effect_groups(cards, idxs, nf, gn); }
    if (nf <= 0) return;
    g->queue.pending.n_filtered_indices = nf;
    for (int i = 0; i < nf; i++) g->queue.pending.filtered_indices[i] = idxs[i];
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
    const char *card_type = cmf_card_type(e);
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, zone, card_type, count, can_skip, NULL);
    rb_queue_pause_for_choice(g, &g->queue.pending);
    rb_choice_set_description(&g->queue.pending, desc);
    const char *group = cmf_extra(e, "group_names");
    if (group)
        strncpy(g->queue.pending.filter_group, group, sizeof(g->queue.pending.filter_group) - 1);
    /* The `e &&` here is the same NULL contract as cmf_extra's, and this is the
     * SECOND way a NULL effect faulted on this path. `rb_move_take_cards_from_
     * standard_zone` is written to tolerate a NULL effect throughout, and when
     * the source zone holds more cards than were asked for it ends up here
     * (outcome "Prompt"); this bare `e->target` then dereferenced NULL for the
     * hand / energy / live_card_zone cases, where line 1380's discard branch is
     * not taken. cmf_extra above is now guarded, so this line was the only
     * remaining fault. The two prompt helpers called below
     * (rb_move_prompt_carry_resume_context, rb_move_prompt_fill_filtered)
     * already guard, which is why the defect sits here and not there. */
    if (e && e->target) {
        strncpy(g->queue.pending.target, e->target, sizeof(g->queue.pending.target) - 1);
        strncpy(g->queue.pending.target_player_id, e->target, sizeof(g->queue.pending.target_player_id) - 1);
    }
    rb_move_prompt_carry_resume_context(g, actor, zone, e);
    rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
    rb_move_prompt_fill_filtered(g, actor, zone, e);
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
    /* The C main deck is TOP-INDEX-0 (it mirrors Rust `main_deck.cards`, where
       `insert(0, x)` is put_on_deck_top), so Rust's `MainDeck::draw()` —
       `self.cards.remove(0)`, zones.rs:811-820 — is a removal from INDEX 0, not
       from the end.  This loop used to take `cards[--deck.n]`, i.e. the BOTTOM,
       which silently turned every "from the top of the deck" effect into a
       bottom mill.  Rejects go back with `cards.push(card)`, which
       is likewise the BOTTOM (append), not index 0. */
    while (remaining > 0 && attempts < count + P->deck.n + 10) {
        /* Q104 / Rule 10.2.1: deck empty mid-draw -> flush the already-taken
           cards into the waitroom, refresh, and continue (mod.rs:902-906). */
        if (P->deck.n == 0 && P->discard.n > 0) {
            for (int k = 0; k < drawn && P->discard.n < RB_MAX_ZONE; k++)
                P->discard.cards[P->discard.n++] = out_ids[k];
            drawn = 0;
            rb_shuffle(P->discard.cards, P->discard.n);
            for (int k = 0; k < P->discard.n && P->deck.n < RB_MAX_ZONE; k++)
                P->deck.cards[P->deck.n++] = P->discard.cards[k];
            P->discard.n = 0;
            P->deck_refreshed_this_turn = 1;
        }
        if (P->deck.n == 0) break;
        int cid = P->deck.cards[0];
        for (int k = 1; k < P->deck.n; k++) P->deck.cards[k - 1] = P->deck.cards[k];
        P->deck.n--;
        attempts++;
        if (card_type_filter && !rb_card_matches_type(cid, card_type_filter)) {
            if (P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = cid;
            continue;
        }
        if (group_name && !rb_card_matches_group_str(cid, group_name)) {
            if (P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = cid;
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

/* ── resolve_from_stage ────────────────────────────────────────────────────
 * Rust mod.rs:1007-1140. The previous C body walked the three stage slots,
 * nulled the matching ones and returned: it never recycled the departing
 * member's under-cards, never set last_vacated_stage_area, had no is_self_cost
 * arm at all, and could never raise a prompt. The missing recycling is the
 * measured "Q139: 2 under-cards follow kasumi to LeftSide (got 0 expected 2)"
 * / "Center keeps no under-cards afterwards (got 2 expected 0)"
 * (parity_under_member.log:25-26) and "the displaced member's under-energy
 * returns to the energy deck (got 0 expected 2)" (:141); the missing prompt is
 * "PROMPT CONTROL: exactly one member should be selected (got -1 expected 1)"
 * (:444). */
int rb_move_resolve_from_stage(GameState *g, int actor, AbilityEffect *e, int use_p2,
                               int count, int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    /* mod.rs:1014-1019 — the player is p2 iff use_p2, else p1. The C used
     * `use_p2 ? 1 : actor`, which read p1's stage whenever a p2-led
     * activation resolved against its own stage. */
    int pl = use_p2 ? 1 : 0;
    if (pl < 0 || pl > 1) pl = 0;
    RbPlayer *P = &g->p[pl];
    const char *destination = e && e->destination ? e->destination : "";

    /* mod.rs:1021-1042 — the is_self_cost arm: the ACTIVATING member itself is
     * the card being paid, so its area is published as last_vacated_stage_area
     * (which every later placement step reads) and the member leaves with its
     * under-cards recycled. `same_area` does NOT remove it (mod.rs:1035-1038)
     * — the destination repositions it in place instead. */
    if (e && cmf_extra_true(e, "self_cost")) {
        int act = g->activating_card;
        int idx = -1;
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] == act) { idx = i; break; }
        if (idx < 0) return -1;  /* Err("Activating card not found at stage") */
        g->baton_last_vacated_area[pl] = idx;
        if (strcmp(destination, "same_area") != 0) {
            int cid = rb_player_remove_member_from_stage_with_recycling(g, pl, idx);
            if (cid >= 0 && max > 0) out_ids[0] = cid;
            return cid >= 0 ? 1 : 0;
        }
        int cid = P->stage[idx];
        if (max > 0) out_ids[0] = cid;
        return 1;
    }

    /* mod.rs:1044-1071 — build the filter, then resolve the selection over the
     * RAW stage array (Rust indexes `&player.stage.stage`, so a returned index
     * IS a stage position, which is what the answer handler reads back). */
    RbCardFilter f;
    move_build_stage_filter(g, e, &f);

    int stage_ids[RB_STAGE_SIZE];
    for (int i = 0; i < RB_STAGE_SIZE; i++) stage_ids[i] = P->stage[i];
    int idxs[RB_STAGE_SIZE];
    int mn = rb_get_selection_indices_filter(stage_ids, RB_STAGE_SIZE, &f,
                                             cmf_is_self_target(e),
                                             g->activating_card, idxs, RB_STAGE_SIZE);
    int is_all = e && cmf_extra_true(e, "all");
    int outcome = rb_classify_selection(idxs, mn, count < 0 ? 0 : count, is_all);

    if (outcome == 1) {
        /* mod.rs:1073-1102 — take in REVERSE index order and publish the area
         * of the LAST removal as last_vacated_stage_area (`vacated = Some(...)`
         * inside the loop over `.rev()`). */
        int n = 0;
        int vacated = -1;
        for (int i = mn - 1; i >= 0; i--) {
            int si = idxs[i];
            if (si < 0 || si >= RB_STAGE_SIZE) continue;
            int cid = rb_player_remove_member_from_stage_with_recycling(g, pl, si);
            if (cid >= 0) {
                vacated = si;
                if (n < max) out_ids[n++] = cid;
            }
        }
        g->baton_last_vacated_area[pl] = vacated;
        return n;
    }
    if (outcome == 2) {
        /* mod.rs:1104-1136 — 「最大で」 optional stage moves MUST be declinable
         * (mod.rs:1128-1130), and the offered list is the stage POSITIONS that
         * pass the filter. */
        rb_move_prompt_card_selection(g, actor, "stage", count,
                                      e && e->is_optional, e);
        /* The prompt deliberately does NOT fill filtered_indices for a STAGE
         * zone (its candidate indices are stage POSITIONS, which rb_zone_cards
         * cannot produce), so this list is written here. The guard used to be
         * `!rb_has_pending_choice(g)`, which is inverted: the line above HAS
         * just raised the prompt, so the condition was always false and the
         * stage prompt advertised no candidates at all. */
        if (rb_has_pending_choice(g) && mn > 0) {
            g->queue.pending.n_filtered_indices = mn > RB_STAGE_SIZE ? RB_STAGE_SIZE : mn;
            for (int i = 0; i < g->queue.pending.n_filtered_indices; i++)
                g->queue.pending.filtered_indices[i] = idxs[i];
        }
        return 0;
    }
    /* mod.rs:1137 — SelectionOutcome::Skip */
    return 0;
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
    /* mod.rs:497-498 — "Card left any zone → full zone-exit cleanup (rule
     * 4.1.4)": gs.on_cards_left_zones(&cards). Without it a baton-touched /
     * cost-paid card kept its runtime-gained abilities into the new zone. */
    for (int i = 0; i < n; i++) rb_on_cards_left_zones(g, out_ids[i]);
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
 * candidate list and the engine would lift the wrong card. rb_move_prompt_
 * fill_filtered, called from inside rb_move_prompt_card_selection itself, now
 * rebuilds that same list for every prompt in this file, from the same zone and
 * the same filter assembly; the `rb_move_prompt_card_selection_with_filtered`
 * wrapper that used to do this for one caller was never called and is gone.
 *
 * One deliberate difference from Rust, documented: Rust's `filtered_indices` is
 * `util::matching_indices` WITHOUT the self-target pinning that the same call's
 * `resolve_selection` applies (selection.rs:72 vs :74-75). The C `idxs` is
 * `rb_get_selection_indices(.., self_target_only, ..)`, which DOES pin. The C
 * list is the strictly tighter one, and it is the one the classification above
 * was decided on, so it is the list the prompt actually offered. */
/* ── take_cards_from_standard_zone ─────────────────────────────────────────
 * Rust move/selection.rs:53-87, consuming mod.rs:1142-1367
 * `resolve_from_standard_zone`'s per-source-zone decisions (can_skip,
 * pass_is_all, which optional filters are meaningful, the is_self_cost hand
 * shortcut, and the InsufficientBehaviour for the live-card zone).
 *
 * The previous C body hard-coded `can_skip = 1` for every zone and dropped the
 * whole per-zone filter assembly, so a 「手札から〜枚」 move that Rust refuses to
 * auto-resolve was auto-resolved, and a live-card-zone move that Rust reports
 * as an error resolved silently. Returns the number of cards written, or -1
 * when a prompt was raised (the caller must stop). */
int rb_move_take_cards_from_standard_zone(GameState *g, int actor,
                                           const char *zone_name,
                                           AbilityEffect *e,
                                           int count, int is_all,
                                           int can_skip, int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    /* mod.rs:1147 / 1183-1197 — resolve against the ability master (p1 unless
     * use_p2), and derive can_skip / pass_is_all from the SOURCE zone. */
    int pl = actor;
    if (e && e->target && !strcmp(e->target, "opponent")) pl = actor ^ 1;
    if (pl < 0 || pl > 1) pl = 0;
    RbPlayer *P = &g->p[pl];
    int is_discard = move_zone_is(zone_name, "discard");
    int is_hand    = move_zone_is(zone_name, "hand");
    int is_success = move_zone_is(zone_name, "success_live_zone");

    /* mod.rs:1153-1177 — 「このカード自体をコストに」 out of hand: the
     * activating card leaves hand (or the waitroom, where a revealed cost
     * card already sits) and nothing is filtered. */
    if (e && cmf_extra_true(e, "self_cost") && is_hand) {
        int act = g->activating_card;
        if (act < 0) return 0;  /* Err("Self hand cost card is not in hand or waitroom") */
        for (int i = 0; i < P->hand.n; i++) if (P->hand.cards[i] == act) {
            for (int k = i; k < P->hand.n - 1; k++) P->hand.cards[k] = P->hand.cards[k + 1];
            P->hand.n--;
            if (max > 0) out_ids[0] = act;
            return 1;
        }
        for (int i = 0; i < P->discard.n; i++) if (P->discard.cards[i] == act) {
            for (int k = i; k < P->discard.n - 1; k++) P->discard.cards[k] = P->discard.cards[k + 1];
            P->discard.n--;
            if (max > 0) out_ids[0] = act;
            return 1;
        }
        return 0;
    }

    /* mod.rs:1190-1197 */
    int any_number = e && cmf_extra_true(e, "any_number");
    if (is_discard)       can_skip = (e && cmf_extra_true(e, "max")) || (e && e->is_optional);
    else if (is_hand)     can_skip = (e && e->is_optional) || any_number;
    else if (is_success)  can_skip = (e && e->is_optional);
    else                  can_skip = 0;
    if (e && cmf_extra_true(e, "max")) can_skip = 1;
    /* mod.rs:1199-1202 — only hand/discard/energy honour 「すべて」. */
    if (!(is_hand || is_discard || move_zone_is(zone_name, "energy"))) is_all = 0;

    /* mod.rs:1209-1274 — the per-source filter assembly. Factored into
     * move_build_standard_zone_filter so the prompt this function raises at the
     * bottom rebuilds the IDENTICAL filter instead of leaving
     * filtered_indices empty (selection.rs:6-14 hands the prompt both the
     * filter and the candidate list, so both come from one description). */
    RbCardFilter f;
    move_build_standard_zone_filter(g, zone_name, e, &f);

    /* mod.rs:1301-1333 — group_reference "different_group_names" on a discard
     * move excludes every discard card whose group matches ANY stage member's
     * group (Q89/Q105/Q208). RbCardFilter has no exclude_group_names field, so
     * the exclusion is applied to the candidate list below — and to the
     * PROMPT's candidate list too (rb_move_prompt_fill_filtered runs the same
     * two helpers), because Rust puts it in the filter and therefore in the
     * `filtered_indices` the prompt advertises. */
    char stage_groups[8][64];
    int nsg = move_collect_stage_group_exclusions(g, pl, zone_name, e, stage_groups, 8);

    /* mod.rs:66-84 — the body proper. */
    int cards[RB_MAX_ZONE];
    int n = rb_zone_cards(g, pl, zone_name, cards, RB_MAX_ZONE);
    int filtered[RB_MAX_ZONE];
    int nf = rb_get_selection_indices_filter(cards, n, &f, cmf_is_self_target(e),
                                              g->activating_card, filtered, RB_MAX_ZONE);
    if (nsg > 0)
        nf = move_exclude_stage_groups(cards, filtered, nf, stage_groups, nsg);
    /* mod.rs:1266-1268 — the effect's own group_names ride the `groups` list
     * (ANY-match), not `filter.group`. RbCardFilter has no list field, so it is
     * the same post-pass over the candidate list. */
    { const char *gn = e ? cmf_extra(e, "group_names") : NULL;
      if (gn && gn[0]) nf = move_apply_effect_groups(cards, filtered, nf, gn); }
    int outcome = rb_classify_selection(filtered, nf, count, is_all);
    int prompt_count = 0;
    switch (outcome) {
    case 1:  /* Exact */
        if (can_skip && !is_all && nf > 0) {
            /* mod.rs:77 — 「〜均可」 with EXACTLY `count` available: the choice
             * is still raised, offering exactly that many. */
            prompt_count = nf;
            break;
        }
        rb_zone_remove_at_indices(g, pl, zone_name, filtered, nf);
        for (int i = 0; i < nf && i < max; i++) out_ids[i] = cards[filtered[i]];
        return nf;
    case 2:  /* Prompt */
        prompt_count = count;
        break;
    default: /* Skip */
        /* mod.rs:82-83 */
        if (can_skip && nf > 0) {
            prompt_count = nf;
            break;
        }
        return 0;   /* Ok(Some(vec![])) */
    }
    /* mod.rs:85 — prompt_card_selection(zone_name, prompt_count, can_skip,
     * effect, filter, Some(filtered_indices)) and Ok(None). */
    rb_move_prompt_card_selection(g, actor, zone_name, prompt_count, can_skip, e);
    if (nf > 0) {
        g->queue.pending.n_filtered_indices = nf > RB_MAX_ZONE ? RB_MAX_ZONE : nf;
        for (int i = 0; i < g->queue.pending.n_filtered_indices; i++)
            g->queue.pending.filtered_indices[i] = filtered[i];
    }
    return -1;
}

/* ── resolve_cards_from_source ─────────────────────────────────────────────────
 *
 * THE VACATED-AREA RESET. Rust models "which stage area did the last member
 * leave" as `gs.last_vacated_stage_area: Option<u8>`
 * (core/game_state/mod.rs:277) — it is None until a card is actually removed
 * FROM a stage slot, and `execute_move_cards` clears it again on the way out
 * (move/mod.rs:2189-2191, `if !consumes_vacated_area { gs.last_vacated_stage_area
 * = None; }`) so a move whose source is not the stage never sees a stale value.
 * The ONLY writer is `util::remove_card_from_any_zone`
 * (move/transport.rs:43-53), which move.c reaches through
 * rb_move_resolve_from_stage / rb_move_resolve_from_recently_moved /
 * rb_move_resolve_from_selected_cards.
 *
 * The C's stand-in, `GameState.baton_last_vacated_area[2]`, is a plain `int`
 * pair that the game-state memset leaves at 0, and 0 is a LEGAL area index, so
 * it cannot express Rust's None. Nothing ever wrote -1 and nothing ever cleared
 * it, so every placement in a fresh game saw `vacated_area == 0`.
 *
 * That is invisible for the seat whose area 0 is occupied, and fatal for the
 * other one: `rb_move_place_card_with_stage_choice` takes Rust's vacated-area
 * shortcut (placement.rs:97-109) whenever `available_slots.len() > 1`, and the
 * shortcut is checked BEFORE the prompt is built. With a bogus Some(0) and an
 * empty area 0 the placement therefore never reached the prompt and the card
 * landed in the lowest free slot instead. Measured on parity_full_round2
 * (probe in the isolated tree):
 *     place_card_with_stage_choice pl=1 card=2641 dest=empty_area
 *         vacated=0 n_avail=3 bvl0=0 bvl1=0 -> VACATED SHORTCUT slot=0
 * i.e. the `target="both"` OPPONENT arm ran, took the shortcut and raised NO
 * SelectPosition at all, which is the bulk of that suite's "no prompt at all"
 * failures ("P2 left is still empty", "the opponent prompt is a SelectPosition",
 * "the opponent prompt is P2's").
 *
 * The reset below is Rust's clear, moved to the one point in move.c that owns
 * the whole source resolution. It runs BEFORE any source dispatch, so a source
 * that genuinely vacates a stage slot re-establishes the value immediately
 * afterwards (resolve_from_stage's self_cost and selection arms,
 * resolve_from_recently_moved, resolve_from_selected_cards) and the caller's
 * read still sees it; every other source leaves -1, which is Rust's None.
 * Only engine.c's two baton writes (engine.c:1007 double-baton arrival,
 * engine.c:1132 plain baton) stop feeding the placement path — and both of
 * those values are consumed by engine.c itself for the arriving member's area
 * (engine.c:950), never by an ability's placement step.
 */
int rb_move_resolve_cards_from_source(GameState *g, int actor, AbilityEffect *e,
                                       int count, int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    g->baton_last_vacated_area[0] = -1;
    g->baton_last_vacated_area[1] = -1;
    /* move/mod.rs:358-362 — Rust's `resolve_cards_from_source` opens with
     *
     *     let source_str = if source.is_empty() {
     *         // No source specified in the parsed ability (e.g. auto abilities
     *         // that trigger on zone transitions like "when X goes to waitroom").
     *         // The card is now in the discard/waitroom -- search there.
     *         "discard"
     *     } else { source };
     *
     * and `source` itself is `effect.source_any().map(|s| s.to_string())
     * .unwrap_or_default()` (mod.rs:2105-2108) — "" for BOTH a missing wire key
     * and an empty string, which is why the guard tests `[0]` and not just the
     * pointer. An ABSENT source is therefore the WAITROOM; it is never the hand.
     *
     * The old `? : "hand"` invented a HAND source the effect never asked for, and
     * it was the last place on this path that still did: a zone-less
     * `discard_card` searched the hand, found a card and raised a SelectCard
     * prompt, where Rust reads the (empty) waitroom, finds nothing and raises
     * nothing — resolve_from_standard_zone's Skip arm, mod.rs:1436-1441. That is
     * the difference tests/parity_executor_dispatch.c:354 measures.
     *
     * `destination` is deliberately left defaulting to "discard": Rust passes the
     * raw "" to place_taken_cards, and util.rs:2499-2504 turns an empty
     * destination into the waitroom too, so the two spellings agree. */
    const char *source = (e->source && e->source[0]) ? e->source : "discard";
    const char *destination = e->destination ? e->destination : "discard";

    if (!strcmp(source, "recently_moved")) {
        const char *ctype = cmf_card_type(e);
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
        const char *ctype = cmf_card_type(e);
        const char *group = cmf_extra(e, "group_names");
        for (int area = 0; area < RB_STAGE_SIZE && n < wanted; area++) {
            RbBag *under = &g->p[pl].under_cards[area];
            for (int i = 0; i < under->n && n < wanted; i++) {
                int cid = under->cards[i];
                if (ctype && !rb_card_matches_type(cid, ctype)) continue;
                if (group && !move_card_matches_group_csv(cid, group)) continue;
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
    const char *ctype = cmf_card_type(e);
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
    const char *ctp = cmf_card_type(e);
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

    const char *ctype = cmf_card_type(e);
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
    const char *ctp = cmf_card_type(e);
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
            /* Rust placement.rs:122-149 emits `Choice::SelectPosition { position:
               pos_str }` and parks `ExecutionContext::MoveCardsPosition { card_id,
               state_change, target: pos_target, source_zone }` beside it
               (placement.rs:141-147). RB_CHOICE_SELECT_TARGET was the wrong
               Choice twin: it made test_pending_choice_type() report
               "SelectTarget" and left the `case RB_CHOICE_SELECT_POSITION:` arm of
               the answer dispatch unreachable. The handler this prompt belongs to
               is rb_move_handle_select_position (below).

               The context's `target` has no field of its own on RbChoice and
               `pending.target` is already carrying `position`, so it rides in
               target_player_id — the same field the discard SelectCard parks its
               seat in (rb_move_execute_selected_cards_from_zone) and the one
               choice_effect_target_seat / rb_resolver_handle_select_card read.
               state_change rides in resume_move_state, the queue's own copy of it
               (rabuka.h:1214, read by choice_effect_state_change); resume_draw_dest
               is the DRAW destination and must never carry it. Rust's
               SelectPosition variant has no route (types.rs:1005-1023), and
               RB_ROUTE_NONE resolves to the same RB_ROUTEK_CHOICE
               (types.c:651). */
            rb_emit_choice(g, actor, RB_CHOICE_SELECT_POSITION, NULL, NULL, 1, 0, pos_str);
            if (player_target && *player_target)
                strncpy(g->queue.pending.target_player_id, player_target,
                        sizeof(g->queue.pending.target_player_id) - 1);
            rb_choice_set_description(&g->queue.pending, desc);
            rb_choice_set_route(&g->queue.pending, RB_ROUTE_NONE);
            g->queue.resume_mode = 1;
            g->queue.resume_actor = actor;
            g->queue.resume_host = card_id;
            if (state_change && *state_change)
                strncpy(g->queue.resume_move_state, state_change, sizeof(g->queue.resume_move_state) - 1);
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
            /* The same field the SelectPosition prompt above uses, and for the
             * same reason (choice.c:4523, `choice_effect_state_change`, reads
             * ONLY resume_move_state). resume_draw_dest is the DRAW destination
             * — choice.c:4696-4703 hands it straight to the draw resume as the
             * zone a card is placed into — so parking "wait"/"active" there
             * teaches a later draw resume to place a card in a zone named
             * "wait". Rust carries the same value in the SAME field on both
             * prompts: `ExecutionContext::MoveCardsPosition { card_id,
             * state_change, .. }` at placement.rs:200-205 and :144-149. */
            if (state_change && *state_change)
                strncpy(g->queue.resume_move_state, state_change, sizeof(g->queue.resume_move_state) - 1);
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
    /* Rust move/mod.rs:2490-2572 `finalize_card_movement`, in order:
     *   2499  gs.on_cards_left_zones(moved_cards)   — the single zone-exit
     *        choke point (modifiers.rs:1677) that clears BOTH the modifier
     *        tables AND any runtime-gained abilities. rb_mods_clear_card alone
     *        left constant 「能力付与」 grants alive on a card that changed zones.
     *   2501-2519  state_change wait / active, plus the Zone::Energy
     *        active_energy_count bump for "active".
     *   2521-2523  gs.record_card_movement per card.
     *   2525       self.selection.moved_cards.extend(moved_cards)
     *   2535-2546  gs.push_movement_event per card, SKIPPED for "same_area"
     *        ("same_area is an internal repositioning mechanic, not a zone
     *        transition" — mod.rs:2533-2534).
     *   2558-2560  gs.recalculate_constants()
     *   2566-2571  fire_debut_side_effects per card, Zone::Stage only.
     *
     * The MISSING piece was 2521-2546: `source` was `(void)`-cast and neither
     * the per-card movement record nor the movement EVENT was pushed, so
     * `g->moved_this_turn` stayed clear and `g->batch_movements` stayed empty
     * for every mill / deck->waitroom / hand->discard move. That is the
     * measured failure "a real mill must record at least one movement event
     * (n_batch_movements > 0)" (parity_mill_deck.log:1, :20) and the reason
     * every 「〜が控え室に置かれたとき」 watcher on a milled card stayed silent.
     */
    for (int i = 0; i < n_moved; i++)
        rb_on_cards_left_zones(g, moved_cards[i]);
    if (state_change && *state_change) {
        if (!strcmp(state_change, "wait")) {
            for (int i = 0; i < n_moved; i++)
                rb_mods_set_orientation(&g->mods, moved_cards[i], "wait");
        } else if (!strcmp(state_change, "active")) {
            for (int i = 0; i < n_moved; i++)
                rb_mods_set_orientation(&g->mods, moved_cards[i], "active");
            if (move_zone_is(destination, "energy")) {
                int pl = actor;
                if (target && *target) { int t = rb_resolve_target_player(g, target); if (t >= 0) pl = t; }
                int sum = (int)g->p[pl].energy_active + n_moved;
                g->p[pl].energy_active = sum > 32767 ? 32767 : sum;
            }
        }
    }
    /* mod.rs:2525 — the resolver's accumulated move list, which the next
     * sequential step's `preceding_moved` / `those_cards` source reads. */
    for (int i = 0; i < n_moved; i++)
        mc_sel_moved_push(g, moved_cards[i]);
    if (move_zone_is(destination, "same_area")) {
        /* mod.rs:2533-2535 — same_area records the card as moved but emits NO
         * movement event, so watcher triggers do not fire for a reposition. */
        for (int i = 0; i < n_moved; i++)
            mc_record_movement(g, moved_cards[i]);
    } else {
        mc_push_movement_event(g, moved_cards, n_moved, source, destination);
    }
    rb_recalc_constants(g);
    if (move_zone_is(destination, "stage")) {
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

/* ── handle_select_position ──────────────────────────────────────────────────
   Rust handle_select_position, ExecutionContext::MoveCardsPosition arm
   (engine/src/ability/effects/move/mod.rs:2372-2426), reached from
   provide_choice_result's `(Choice::SelectPosition{..}, PositionSelected{..})`
   match (result_handlers.rs:233-238). The C twin of that whole function used to
   be duplicated: choice.c's rb_resolver_handle_select_position, reachable only
   from the kind switch (which the SelectTarget mis-emission made unreachable),
   and this one, which had no caller at all. This is the one that survives. */
void rb_move_handle_select_position(GameState *g, int actor, const char *position,
                                     int card_id, const char *target,
                                     const char *source_zone, const char *state_change) {
    if (!g) return;
    int pl = actor;
    if (target && *target) { int t = rb_resolve_target_player(g, target); if (t >= 0) pl = t; }
    RbPlayer *P = &g->p[pl];
    /* mod.rs:2396 — `util::stage_position_index(position)`, which accepts
       "left"/"left_side"/"center"/"right"/"right_side". The answer index is
       mapped to Rust's own spellings ("left"/"center"/"right", mod.rs:2390)
       before it gets here, so the C has to accept them too. */
    int pos_idx = position ? rb_stage_position_index(position) : -1;
    /* mod.rs:2377 — `should_lock = source_zone != Zone::Stage.to_str()`, and
       track_deployment is the Rust spelling of stage_arrived. */
    int should_lock = (!source_zone || strcmp(source_zone, "stage") != 0);
    int placed = 0;
    if (pos_idx >= 0 && pos_idx < RB_STAGE_SIZE) {
        /* mod.rs:2394-2400 — an occupied slot displaces its member to the
           waitroom. Rust leaves `under_cards[idx]` alone on this path (the
           recycling twin, Stage::recycle_under_cards, belongs to the Rule 4.5.5
           stage swap at zones.rs:396-400), so the stack is NOT carried here. */
        if (P->stage[pos_idx] != RB_EMPTY_SLOT) rb_waitroom_add(P, P->stage[pos_idx]);
        P->stage[pos_idx] = card_id;
        if (should_lock) g->stage_arrived[pl][pos_idx] = 1;
        placed = 1;
    }
    if (!placed) rb_hand_add(P, card_id);   /* mod.rs:2407-2412 */
    rb_mods_clear_card(&g->mods, card_id);
    /* mod.rs:2415 — `gs.record_card_movement(card_id)` records the card as
       moved this turn but pushes NO MovementEvent (Rust logs the pair only,
       log_move_result at mod.rs:2425). The C's choke point fuses the two, so
       the from-zone is RB_ZONEID_UNKNOWN — Rust's own ZoneId::Unknown, and the
       only spelling that does NOT read as a stage->stage pair and trip the
       area-move arm at modifiers.c:525-526. */
    rb_record_card_movement(g, card_id, RB_ZONEID_UNKNOWN,
                            placed ? RB_ZONEID_STAGE : RB_ZONEID_HAND, 0, 0);
    /* mod.rs:2417-2419 — selection.moved_cards, which `preceding_moved` /
       `those_cards` sources read. */
    mc_sel_moved_push(g, card_id);
    if (state_change && !strcmp(state_change, "wait"))
        rb_mods_set_orientation(&g->mods, card_id, "wait");
    /* mod.rs:2420-2422 — the orientation modifier is the ONE place Rust records
       the appeared-state change. The C mirrors an orientation in TWO places
       (condition.c:1692-1699 and rb_move_finalize_card_movement,
       choice.c:3001-3008), and `stage_wait[]` is the C-only second channel the
       placement emitters read back, so it is stamped here too. rb_place_card_in_zone
       hard-codes stage_wait[area]=0 (util.c:1019), which is why this path cannot
       go through it. */
    if (placed && state_change && !strcmp(state_change, "wait"))
        P->stage_wait[pos_idx] = 1;
    rb_move_fire_debut_side_effects(g, actor, card_id, target ? target : "self", source_zone);
    /* mod.rs:2429-2438 — Rust does NOT clear the choice here, on purpose:
       fire_debut_side_effects may have opened the placed card's OWN 登場 prompt,
       and a blanket clear silently swallowed it. The C resume path already
       cleared the position choice before dispatching here
       (rb_clear_pending_choice at the top of
       rb_resume_with_choice_indices_internal), so there is nothing left to
       clear and the call that used to stand here was pure data loss. */
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

/* ── place_energy_under_member_selected ────────────────────────────────────
 * Rust mod.rs:3412-3489 (「このメンバーの下」 energy placement, Rule 10.5.3,
 * with the Rule 10.5.4 energy-deck fallback). */
void rb_move_place_energy_under_member_selected(GameState *g, int actor,
                                                const int *cids, int n_cids) {
    if (!g || !cids || n_cids <= 0) return;
    /* mod.rs:3420-3422 — Rust resolves against "self" (the ability master),
     * NOT against the effect target, so the host member is always the acting
     * player's own stage. The C previously used the raw `actor` index, which
     * is p2 whenever a p2-led activation ran the effect. */
    int pl = rb_resolve_target_player(g, "self");
    if (pl < 0 || pl > 1) pl = (actor >= 0 && actor <= 1) ? actor : 0;
    RbPlayer *P = &g->p[pl];
    int activating = g->activating_card;
    int activating_pos = -1;
    if (activating >= 0) {
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] == activating) { activating_pos = i; break; }
    }
    int target_index = -1;
    if (activating_pos >= 0) {
        /* mod.rs:3424-3431 — the activating member vacated its slot between the
         * prompt and the answer, so Rule 10.5.4 sends the energy to the energy
         * deck (NOT to the vacated slot, which now holds someone else). */
        if (P->stage[activating_pos] == RB_EMPTY_SLOT) {
            for (int i = 0; i < n_cids; i++)
                if (P->energy_deck.n < RB_MAX_ZONE)
                    P->energy_deck.cards[P->energy_deck.n++] = cids[i];
            return;
        }
        target_index = activating_pos;
    } else {
        /* mod.rs:3432-3438 — `self.selection.moved_cards` searched in reverse
         * (the most recently moved card still on stage wins). */
        const int *mv = mc_sel_moved(g);
        for (int i = mc_sel_moved_count(g) - 1; i >= 0; i--) {
            for (int j = 0; j < RB_STAGE_SIZE; j++)
                if (P->stage[j] == mv[i]) { target_index = j; break; }
            if (target_index >= 0) break;
        }
    }
    if (target_index < 0 && P->stage[1] != RB_EMPTY_SLOT) target_index = 1;
    if (target_index < 0 && P->stage[0] != RB_EMPTY_SLOT) target_index = 0;
    if (target_index < 0 && P->stage[2] != RB_EMPTY_SLOT) target_index = 2;
    /* mod.rs:3445-3450 — no member anywhere on stage: Rule 10.5.4 energy deck. */
    if (target_index < 0) {
        for (int i = 0; i < n_cids; i++)
            if (P->energy_deck.n < RB_MAX_ZONE)
                P->energy_deck.cards[P->energy_deck.n++] = cids[i];
        return;
    }
    /* mod.rs:3451-3456 — re-check the resolved slot. */
    if (target_index < 0 || target_index >= RB_STAGE_SIZE ||
        P->stage[target_index] == RB_EMPTY_SLOT) {
        for (int i = 0; i < n_cids; i++)
            if (P->energy_deck.n < RB_MAX_ZONE)
                P->energy_deck.cards[P->energy_deck.n++] = cids[i];
        return;
    }
    int area = target_index;
    for (int i = 0; i < n_cids; i++)
        rb_stage_place_under_card(P, area, cids[i]);
    /* mod.rs:3461-3478 — record the placement as a MOVEMENT EVENT so
     * 「エネルギーがメンバーの下に置かれたとき」 watchers fire. The C built a
     * RbBatchMovement by hand here, which (a) computed `cause_cid` and threw it
     * away — the single field the event exists to carry — and (b) never set
     * `moved_this_turn` or appended to `recently_moved`, so a following
     * `preceding_moved` / `those_cards` step saw an empty batch. That is the
     * measured "the displaced member's under-energy returns to the energy deck
     * (got 0 expected 2)" (parity_under_member.log:141). */
    mc_push_movement_event(g, cids, n_cids, "energy_zone", "under_member");
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

/* ── handle_energy_zone_selection ──────────────────────────────────────────
 * Rust mod.rs:3344-3405. */
void rb_move_handle_energy_zone_selection(
    GameState *g, int actor, const int *indices, int n_indices,
    int count, const char *destination, int (*validate_card)(int)) {
    if (!g || !indices || n_indices <= 0) return;

    /* mod.rs:3360-3362 — Rust resolves the energy zone against "self" (the
     * ability master), never the effect target. */
    int pl = rb_resolve_target_player(g, "self");
    if (pl < 0 || pl > 1) pl = (actor >= 0 && actor <= 1) ? actor : 0;
    RbPlayer *P = &g->p[pl];

    if (destination && *destination) {
        /* mod.rs:3363-3376 — High→low so earlier removals do not shift later
         * indices, then reverse back into selection order. */
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
        /* REVERSE back to the order the indices were given in (mod.rs:3375),
         * so the batch lands in the order the player saw. */
        for (int i = 0, j = nc - 1; i < j; i++, j--) {
            int t = cids[i]; cids[i] = cids[j]; cids[j] = t;
        }

        /* Rust EnergyZone::remove_at (core/zones.rs:346-355) decrements
           `active_energy_count` only for a card that occupied an ACTIVE SLOT,
           and that positional rule is not expressible here: the C energy zone
           is an UNORDERED bag (rb_energy_add_card, zones.c:271) plus a
           separate count. For the under_member placement the C model keeps the
           count — Q184, an energy card tucked under a member still counts as
           your energy — and that is pinned strictly by
           test_parity_cce_gaps.c:2720 (h5: "the active energy count is
           UNCHANGED"). Every other destination is an energy SPEND and does
           consume the count. */
        int is_under_member = move_zone_is(destination, "under_member");
        if (!is_under_member)
            P->energy_active = P->energy_active > nc ? P->energy_active - nc : 0;

        if (is_under_member) {
            /* mod.rs:3378-3389 — an empty selection drops the pending
               「そうした場合」 actions and records the decline; the landing
               itself is place_energy_under_member_selected (:3388), whose
               Rule 10.5.3/10.5.4 host resolution and energy-deck fallback
               live in rb_move_place_energy_under_member_selected. */
            if (nc == 0) {
                rb_queue_take_pending_actions(g);
                if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
                    g->queue.entries[g->queue.cur].optional_cost_result = 0;
            } else {
                rb_move_place_energy_under_member_selected(g, actor, cids, nc);
            }
        } else {
            /* mod.rs:3390-3395 */
            for (int i = 0; i < nc; i++)
                rb_place_card_in_zone(g, pl, cids[i], destination, -1);
        }

        /* mod.rs:3396-3400 — clear modifiers, record the movement, then
         * REPLACE the resolver's moved-card list with this batch
         * (`self.selection.moved_cards = cids.into()` is an assignment, not
         * an extend). Rust calls `gs.record_card_movement`, which is ONLY
         * `cards_moved_this_turn.push` (modifiers.rs:1932-1934) — it does NOT
         * touch recently_moved_cards and does NOT emit an event. The C was
         * appending the whole turn's history into recently_moved here, so a
         * following `preceding_moved` / `those_cards` step read every card the
         * activation had moved instead of this batch. (For the under_member
         * arm the movement EVENT is emitted by place_energy_under_member_selected,
         * mod.rs:3461-3478, exactly as Rust does.) */
        for (int i = 0; i < nc; i++) {
            rb_mods_clear_card(&g->mods, cids[i]);
            if (cids[i] >= 0 && cids[i] < RB_MAX_CARD_IDS) g->moved_this_turn[cids[i]] = 1;
        }
        g->n_those_cards = 0;
        for (int i = 0; i < nc && i < RB_MAX_RECENTLY_MOVED; i++)
            g->those_cards[g->n_those_cards++] = cids[i];
    } else {
        /* mod.rs:3401-3403 */
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
    /* move_cards.rs:2023 resolves pos_to_use ONCE and place_taken_cards
       (:2335-2343) then hands the same area to every card of the batch. */
    int dest_pl = use_p2 ? 1 : actor;
    int dest_is_under_member = destination && !strcmp(destination, "under_member");
    int um_pos = dest_is_under_member ? rb_move_under_member_pos(g, dest_pl) : -1;
    for (int i = 0; i < n; i++) {
        int cid = out_ids[i];
        find_and_remove_card(P, cid);
        /* util.rs:2341-2359 has a real Zone::UnderMember arm; the C twin
           (util.c:836) does not, so route the placement here instead of
           through rb_place_card_in_zone, which would drop the card. Note
           um_pos == -1 is still a valid under_member landing: the helper
           then falls back to center -> left -> right -> waitroom. */
        if (dest_is_under_member)
            rb_move_place_card_under_member(g, dest_pl, cid, um_pos);
        else
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

    /* mod.rs:2666-2676 — the destination is a THREE-link chain:
     *   1. gs.entry_destination()            -> the C twin is rb_entry_destination
     *   2. self.in_flight.spawn_context.destination
     *   3. gs.entry_effect().destination     -> already folded into (1)
     * and only then the zone-dependent default of mod.rs:2718-2724.
     *
     * Link 2 was MISSING here, and it is the link that carries a SEQUENTIAL's
     * sub-action destination. `entry_destination()` reads the CURRENT queue
     * entry's ability effect; for a sequential that effect is the `sequential`
     * node, which names no destination, so link 1 is None and the chain fell
     * straight through to the default. Rust instead re-reads the destination
     * `execute_move_cards` stamped on spawn_context (mod.rs:2196) — the
     * destination of the very move_cards step that raised this prompt.
     *
     * The C slot for it is `queue.resume_move_destination`: rb_move_prompt_
     * carry_resume_context (move.c:662-666) writes it from `e->destination`
     * UNCONDITIONALLY at every card-selection prompt, and it is the SAME value
     * choice.c's own `choice_effect_destination` (choice.c:3126-3130) reads on
     * the discard/live answers. Without it the put-back half of a
     * 「1枚 ℃引き、1枚を山札の下に戻す」 chain answered its hand SelectCard and
     * put the chosen card in the WAITROOM: measured on parity_draw_flat, every
     * deck_bottom chain lost the card's destination
     *   [PROBE_DEST] zone=hand entry_dest=(null) resume_dest=deck_bottom
     * and left the deck one card short (19 where the twin expects 20).
     *
     * mod.rs:2718-2724 — with no destination named anywhere, a selection out
     * of the DISCARD defaults to the HAND and every other source to the
     * DISCARD. The C used "discard" for both, so a discard-sourced prompt with
     * no destination put the cards straight back where they came from. */
    const char *destination = rb_entry_destination(g);
    if (!destination && g->queue.resume_move_destination[0])
        destination = g->queue.resume_move_destination;
    if (!destination)
        destination = move_zone_is(zone, "discard") ? "hand" : "discard";

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
    int dest_is_under_member = !strcmp(destination, "under_member");
    /* move_cards.rs:2023 — pos_to_use is resolved once for the batch. */
    int um_pos = dest_is_under_member ? rb_move_under_member_pos(g, pl) : -1;

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
            /* util.rs:2341-2359 — placement.rs:307-309, used in place of
               rb_place_card_in_zone, which has no under_member arm and would
               drop the card on the floor between the source and under_cards. */
            if (dest_is_under_member)
                rb_move_place_card_under_member(g, pl, card_ids[i], um_pos);
            else
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

/* ── rb_effect_move_cards (basic move_cards entry) ──
 *
 * The whole body is rb_move_execute_move_cards_ported (move_missing.c), the C
 * twin of Rust's AbilityResolver::execute_move_cards; it forwards the
 * multiple-target-to-deck shape to rb_move_execute_move_cards_both (move.c)
 * from the same prologue position Rust does (move_missing.c:139-142 vs
 * Rust's mod.rs:2056-2060).
 *
 * What used to sit after this call was a ~120-line PRE-PORT implementation of
 * the same effect: its own `drain_all`/`cnt` encoding, its own target=both /
 * opponent fan-out, its own deck-refresh loop, and its own destination
 * classification (`rb_zone_of_str` + `place_in_dst`). It was unreachable —
 * a bare `return;` preceded it in the same compound statement, so no branch,
 * fallthrough or label could ever reach it, and gcc had already dropped it
 * from the object:
 *   $ objdump -d --disassemble=rb_effect_move_cards src/ability/effects/move.o
 *     0000..acc0 <rb_effect_move_cards>:
 *       acc0:  jmp  acc5 <rb_effect_move_cards+0x5>     <- the one call
 *       acc5:  nop ... (padding)                        <- nothing else
 * Keeping it was the hazard, not the size: it is a SECOND implementation of
 * `move_cards` in the same TU, so a future fix to the ported one can be read
 * past in favour of the copy that reads like the pre-port file and does
 * nothing. `place_in_dst`, its only file-local helper, went with it (it had no
 * caller outside that body). */
void rb_effect_move_cards(GameState *g, int actor, AbilityEffect *e){
    rb_move_execute_move_cards_ported(g, actor, e);
}
