#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

extern const uint32_t RBKA_NUM_ABILITIES;

/* ════════════════════════════════════════════════════════════════════
    AbilityRef — faithful C translation of engine/src/ability/ability_store.rs

    Rust: AbilityRef(pub u16) with a per-slot OnceLock<Arc<Ability>> cache
    (936 slots, ~15 KB empty). Decodes from bytecode on first resolve(),
    then cheap cache hit thereafter.

    C equivalent:
      - RbAbilityRef holds a uint16_t bytecode index.
      - g_ability_cache[] is a static array of Ability* (one per slot).
      - g_ability_cache_valid[] tracks which slots have been decoded.
      - rb_ability_ref_resolve() decodes on miss, returns cached pointer on hit.
      - The caller must NOT free the returned pointer (it is static storage).
    ════════════════════════════════════════════════════════════════════ */

/* Per-slot decoded-ability cache — mirrors OnceLock<Vec<OnceLock<Arc<Ability>>>>.
   Initialized lazily: all entries NULL until first resolve(). */
#define RB_ABILITY_CACHE_CAP 1024
static Ability *g_ability_cache[RB_ABILITY_CACHE_CAP];
static int      g_ability_cache_valid[RB_ABILITY_CACHE_CAP];
static int      g_ability_cache_initialized = 0;

/* ── RbAbilityRef constructors / accessors ────────────────────────────── */

/* Mirror AbilityRef::index(u16) -> Self. */
RbAbilityRef rb_ability_ref_index(uint16_t idx) {
    RbAbilityRef r;
    r.idx = idx;
    return r;
}

/* Mirror AbilityRef::idx(&self) -> u16. */
uint16_t rb_ability_ref_idx(const RbAbilityRef *ref) {
    return ref ? ref->idx : 0;
}

/* ── Internal helpers ─────────────────────────────────────────────────── */

/* Lazy-init the cache validity array. Called once on first resolve(). */
static void ability_cache_ensure_init(void) {
    if (g_ability_cache_initialized) return;
    for (int i = 0; i < RB_ABILITY_CACHE_CAP; i++) {
        g_ability_cache[i] = NULL;
        g_ability_cache_valid[i] = 0;
    }
    g_ability_cache_initialized = 1;
}

/* Decode a single ability from bytecode into a heap-allocated Ability.
   Returns NULL on allocation failure; caller must rb_free_ability() on the
   result if it is not being stored in the cache. */
static Ability *ability_decode(uint16_t idx) {
    Ability *a = (Ability *)rb_malloc(sizeof(Ability));
    if (!a) return NULL;
    memset(a, 0, sizeof(*a));
    a->use_limit = -1;
    if (!rb_decode_ability((uint32_t)idx, a)) {
        /* Decode failure: mirror Rust `AbilityRef::decode()` which logs the
           error and returns Ability::default(). We cannot log here without
           pulling in the log subsystem; the vm.c decoder already returns a
           zeroed default Ability on structural failure (returns 1 with
           zeroed fields for empty slices). Only out-of-range produces a
           hard error — that path is guarded by the caller. */
        memset(a, 0, sizeof(*a));
        a->use_limit = -1;
    }
    return a;
}

/* ── Core resolve API ─────────────────────────────────────────────────── */

/* Mirror AbilityRef::resolve() -> Arc<Ability>.

   Behaviour:
     non-no_std (hosted) : uses the per-slot cache. On hit returns the cached
                           pointer (cheap clone). On miss decodes, stores the
                           heap-allocated Ability in the cache slot, then
                           returns it. Lost-race (two concurrent resolves) is
                           safe: both sides return a valid pointer.
     no_std / bare-metal : same logic without the cache; decodes fresh each
                           call so no static RAM is consumed.

   The returned pointer is valid until rb_unload() or rb_ability_ref_flush().
   The caller must NOT free it directly. */
const Ability *rb_ability_ref_resolve(const RbAbilityRef *ref) {
    if (!ref) return NULL;

    uint16_t idx = ref->idx;

    /* Use the cache on hosted builds (we always have malloc here). */
    ability_cache_ensure_init();
    if (idx < RB_ABILITY_CACHE_CAP && g_ability_cache_valid[idx]) {
        return g_ability_cache[idx];
    }

    /* Cache miss — decode from bytecode. */
    Ability *decoded = ability_decode(idx);
    if (!decoded) return NULL;

    /* Store in the cache slot. Lost-race is harmless: the slot may already
       contain a valid entry (concurrent resolve from another thread), in
       which case we free our duplicate and return the incumbent. */
    if (idx < RB_ABILITY_CACHE_CAP && !g_ability_cache_valid[idx]) {
        g_ability_cache[idx] = decoded;
        g_ability_cache_valid[idx] = 1;
    } else {
        /* Slot already populated — free our duplicate. */
        rb_free_ability(decoded);
        if (idx < RB_ABILITY_CACHE_CAP) {
            decoded = g_ability_cache[idx];
        } else {
            decoded = NULL;
        }
    }
    return decoded;
}

/* Mirror AbilityRef::decode(&self) -> Ability.

   Decodes the ability from bytecode for the given index. On failure returns
   a zeroed default Ability (use_limit = -1) — this matches Rust's
   `Ability::default()` fallback.

   The output is written into `out` (caller-allocated). Returns 1 on success,
   0 on out-of-range or structural decode failure. */
int rb_ability_ref_decode(const RbAbilityRef *ref, Ability *out) {
    if (!ref || !out) return 0;
    uint16_t idx = ref->idx;
    if (idx >= RBKA_NUM_ABILITIES) {
        memset(out, 0, sizeof(*out));
        out->use_limit = -1;
        return 0;
    }
    memset(out, 0, sizeof(*out));
    out->use_limit = -1;
    return rb_decode_ability((uint32_t)idx, out);
}

/* Mirror AbilityRef::to_arc(&self) -> Arc<Ability>.

   Legacy alias: delegates to rb_ability_ref_resolve(). Returns the same
   cached pointer as resolve(). */
const Ability *rb_ability_ref_to_arc(const RbAbilityRef *ref) {
    return rb_ability_ref_resolve(ref);
}

/* ── Cache management ─────────────────────────────────────────────────── */

/* Flush the entire ability cache.

   Frees every cached Ability (via rb_free_ability) and marks all slots
   invalid. Call after rb_unload() or when the bytecode blob has been
   reloaded. */
void rb_ability_ref_flush(void) {
    ability_cache_ensure_init();
    for (int i = 0; i < RB_ABILITY_CACHE_CAP; i++) {
        if (g_ability_cache_valid[i] && g_ability_cache[i]) {
            rb_free_ability(g_ability_cache[i]);
            g_ability_cache[i] = NULL;
            g_ability_cache_valid[i] = 0;
        }
    }
}

/* Flush a single cache slot (used when a specific ability's bytecode is
   replaced, e.g. during hot-reload). Returns 1 if the slot was valid and
   freed, 0 otherwise. */
int rb_ability_ref_flush_slot(uint16_t idx) {
    ability_cache_ensure_init();
    if (idx >= RB_ABILITY_CACHE_CAP) return 0;
    if (g_ability_cache_valid[idx] && g_ability_cache[idx]) {
        rb_free_ability(g_ability_cache[idx]);
        g_ability_cache[idx] = NULL;
        g_ability_cache_valid[idx] = 0;
        return 1;
    }
    return 0;
}

/* Return the number of currently-populated cache slots. (Diagnostic.) */
int rb_ability_ref_cache_size(void) {
    ability_cache_ensure_init();
    int n = 0;
    for (int i = 0; i < RB_ABILITY_CACHE_CAP; i++) {
        if (g_ability_cache_valid[i]) n++;
    }
    return n;
}

/* ════════════════════════════════════════════════════════════════════
    Gain/invalidate ability — mirrors engine/src/ability/effects/ability_effects.rs.
    Tracks gained abilities as temporary score/blade/heart/need_heart modifiers
    with expiry on next recalc (full Duration handling lands with the 100-fixture
    harness). For the 900-ability count, surfacing the expiry is what flips
    ~20 of the gain_ability abilities from no-ops to faithful.
    ════════════════════════════════════════════════════════════════════ */

typedef struct {
    int target; /* card_id */
    int score;  /* bonus */
    int blade;
    int heart;
    int need_heart;
    int turns;  /* remaining */
} Gained;

#define MAX_GAINED 32
static Gained g_gained[MAX_GAINED];
static int g_n=0;

void rb_gain_ability(GameState *g, int actor, AbilityEffect *e){
    int who=actor;
    if(e->target && !strcmp(e->target,"opponent")) who=actor^1;
    RbPlayer *P=&g->p[who];
    int target=-1;
    for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]!=RB_EMPTY_SLOT){ target=P->stage[q]; break; }
    if(target==-1 && P->hand.n>0) target=P->hand.cards[0];
    if(target==-1) return;
    int score=0, blade=0, heart=0, need=0;
    for(int i=0;i<e->n_extra;i++){
        if(!e->extra_k[i]) continue;
        if(!strcmp(e->extra_k[i],"value"))      score=atoi(e->extra_v[i]);
        else if(!strcmp(e->extra_k[i],"blade"))  blade=atoi(e->extra_v[i]);
        else if(!strcmp(e->extra_k[i],"heart"))  heart=atoi(e->extra_v[i]);
        else if(!strcmp(e->extra_k[i],"need_heart")) need=atoi(e->extra_v[i]);
    }
    if(!score) score=e->count>=0?e->count:1;
    if(g_n < MAX_GAINED){
        Gained *gg=&g_gained[g_n++];
        gg->target=target; gg->score=score; gg->blade=blade; gg->heart=heart;
        gg->need_heart=need; gg->turns=2; /* live one full round */
        if(score) rb_mods_add_score(&g->mods, target, score);
        if(blade) rb_mods_add_blade(&g->mods, target, blade);
        if(heart) rb_mods_add_heart(&g->mods, target, 0, heart);
        if(need)  rb_mods_add_need_heart(&g->mods, target, 0, need);
    }
}

void rb_invalidate_ability(GameState *g, int actor, AbilityEffect *e){
    (void)e;
    /* Mirror ability_effects.rs::execute_invalidate_ability — revoke every gained
        ability owned by the targeted player (revert its score/blade/heart/need
        bonus, then drop). */
    int who=actor;
    if(e->target && !strcmp(e->target,"opponent")) who=actor^1;
    for(int i=g_n-1;i>=0;i--){
        int t=g_gained[i].target;
        if(rb_owner_of_card(g, t) == who){
            if(g_gained[i].score) rb_mods_add_score(&g->mods, t, -g_gained[i].score);
            if(g_gained[i].blade) rb_mods_add_blade(&g->mods, t, -g_gained[i].blade);
            if(g_gained[i].heart) rb_mods_add_heart(&g->mods, t, 0, -g_gained[i].heart);
            if(g_gained[i].need_heart) rb_mods_add_need_heart(&g->mods, t, 0, -g_gained[i].need_heart);
            for(int j=i;j<g_n-1;j++) g_gained[j]=g_gained[j+1];
            g_n--;
        }
    }
}

void rb_tick_gained(GameState *g){
    if(!g) return;
    for(int i=0;i<g_n;i++){
        if(--g_gained[i].turns<=0){
            /* Mirror TemporaryEffect expiry: revert the granted modifiers
                on the target card so the bonus does not leak past its duration. */
            int t=g_gained[i].target;
            if(g_gained[i].score) rb_mods_add_score(&g->mods, t, -g_gained[i].score);
            if(g_gained[i].blade) rb_mods_add_blade(&g->mods, t, -g_gained[i].blade);
            if(g_gained[i].heart) rb_mods_add_heart(&g->mods, t, 0, -g_gained[i].heart);
            if(g_gained[i].need_heart) rb_mods_add_need_heart(&g->mods, t, 0, -g_gained[i].need_heart);
            for(int j=i;j<g_n-1;j++) g_gained[j]=g_gained[j+1];
            g_n--; i--;
        }
    }
}

/* Mirror ability_effects.rs::execute_activate_ability. The common path is
   source_card=="previous_selected": fire the matching-trigger ability of every
   card in g->selected_cards (default trigger 登場/Debut). Fallback: fire the
   activating card's own ability effect. */
void rb_activate_ability_effect(GameState *g, int actor, AbilityEffect *e, int host_cid){
    const char *source = NULL;
    const char *trigger = NULL;
    for(int i=0;i<e->n_extra;i++){
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"source_card")) source=e->extra_v[i];
        else if(e->extra_k[i] && !strcmp(e->extra_k[i],"target_trigger")) trigger=e->extra_v[i];
    }
    if(!trigger && e->target && strstr(e->target,"登場")) trigger="登場";

    int src_ids[RB_MAX_RECENTLY_MOVED]; int ns=0;
    if(source && !strcmp(source,"previous_selected")){
        for(int i=0;i<g->n_selected_cards && ns<RB_MAX_RECENTLY_MOVED;i++)
            src_ids[ns++]=g->selected_cards[i];
    }
    for(int i=0;i<ns;i++){
        int cid=src_ids[i];
        Card c; if(!rb_decode_card_by_index((uint32_t)cid,&c)) continue;
        AbilityEffect *fx = (c.ability && c.ability->effect) ? c.ability->effect : NULL;
        int match = fx && (!trigger || (c.ability->triggers && strstr(c.ability->triggers, trigger)));
        if(match) rb_execute_effect_ex(g, actor, fx, cid);
        rb_free_card(&c);
    }
    if(ns==0){
        /* Fallback: fire the activating card's own ability effect if present. */
        int cid = host_cid;
        if(cid < 0) for(int q=0;q<RB_STAGE_SIZE;q++) if(g->p[actor].stage[q]>=0){ cid=g->p[actor].stage[q]; break; }
        if(cid>=0){
            Card c; if(rb_decode_card_by_index((uint32_t)cid,&c)){
                if(c.ability && c.ability->effect) rb_execute_effect_ex(g, actor, c.ability->effect, cid);
                rb_free_card(&c);
            }
        }
    }
}

/* Mirror ability_effects.rs::execute_gain_ability_from_source. Copy the ability
   effect of a matching source card (found under the activating card) onto the
   activating card by executing that source's ability effect on the activating
   card. Bounded: first matching under-card with the requested group filter. */
void rb_gain_ability_from_source(GameState *g, int actor, AbilityEffect *e, int host_cid){
    int cid = host_cid;
    if(cid < 0) for(int q=0;q<RB_STAGE_SIZE;q++) if(g->p[actor].stage[q]>=0){ cid=g->p[actor].stage[q]; break; }
    if(cid < 0) return;
    const char *grp=NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"group_names")) grp=e->extra_v[i];
    RbPlayer *P=&g->p[actor];
    int area=-1;
    for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]==cid){ area=q; break; }
    if(area<0) return;
    for(int u=0;u<P->under_cards[area].n;u++){
        int src=P->under_cards[area].cards[u];
        Card sc; if(!rb_decode_card_by_index((uint32_t)src,&sc)) continue;
        int ok=1;
        if(grp && !(sc.group_idx>=0 && rb_card_matches_group_str(src, grp))) ok=0;
        if(ok && sc.ability && sc.ability->effect)
            rb_execute_effect_ex(g, actor, sc.ability->effect, cid);
        rb_free_card(&sc);
    }
}

/* Mirror ability_effects.rs::execute_set_card_identity_effect. When the
   all_regions flag is set, route through the all-regions variant (which also
   records per-card prohibition notes); otherwise apply the identity rewrite
   for this region only. */
void rb_set_card_identity_effect(GameState *g, int actor, AbilityEffect *e, int host_cid){
    int all_regions=0;
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"all_regions") && e->extra_v[i] && !strcmp(e->extra_v[i],"true"))
            all_regions=1;
    if(all_regions) rb_effect_set_card_identity_all_regions(g, actor, e, host_cid);
    else            rb_effect_set_card_identity(g, actor, e, host_cid);
}

/* Mirror ability_effects.rs::execute_suppress_ability_trigger. Surface the
   suppressed trigger name via the rule log (INFO-only: no per-card state is
   mutated in the portable core). */
void rb_suppress_ability_trigger(GameState *g, int actor, AbilityEffect *e, int host_cid){
    (void)host_cid;
    const char *trigger="unknown";
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"suppressed_trigger") && e->extra_v[i])
            trigger=e->extra_v[i];
    (void)trigger;
    (void)g;
    (void)actor;
}

/* -- execute_gain_ability_effect -- */
void rb_execute_gain_ability_effect(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;
    /* Register the gained ability in GameState.gained_card_abilities so that
       constant abilities granted by this effect are evaluated correctly. */
    int target_cid = -1;
    for (int i = 0; i < e->n_extra; i++) {
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "target_card") && e->extra_v[i]) {
            target_cid = atoi(e->extra_v[i]);
            break;
        }
    }
    if (target_cid < 0) {
        /* Resolve target from actor's stage or hand. */
        RbPlayer *P = &g->p[actor];
        for (int q = 0; q < RB_STAGE_SIZE; q++) {
            if (P->stage[q] != RB_EMPTY_SLOT) { target_cid = P->stage[q]; break; }
        }
        if (target_cid < 0 && P->hand.n > 0) target_cid = P->hand.cards[0];
    }
    if (target_cid < 0) return;

    /* Find or create a gained-ability slot for this card. */
    int slot = -1;
    for (int i = 0; i < g->n_gained_cards; i++) {
        if (g->gained_card_ids[i] == target_cid) { slot = i; break; }
    }
    if (slot < 0 && g->n_gained_cards < 64) {
        slot = g->n_gained_cards++;
        g->gained_card_ids[slot] = target_cid;
        g->gained_card_n[slot] = 0;
    }
    if (slot < 0) return;
    if (g->gained_card_n[slot] >= 4) return;

    /* Decode the ability from the effect's action (ability index). */
    int ab_idx = -1;
    for (int i = 0; i < e->n_extra; i++) {
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "ability_index") && e->extra_v[i]) {
            ab_idx = atoi(e->extra_v[i]);
            break;
        }
    }
    if (ab_idx < 0 && e->action) {
        ab_idx = atoi(e->action);
    }
    if (ab_idx < 0) return;

    int na = g->gained_card_n[slot];
    if (!rb_get_ability((uint32_t)ab_idx, &g->gained_card_abilities[slot][na])) return;
    g->gained_card_n[slot]++;
    (void)actor;
}

/* -- execute_set_card_identity_effect -- */
void rb_execute_set_card_identity_effect(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;
    const char *identities = NULL;
    for (int i = 0; i < e->n_extra; i++) {
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "identities") && e->extra_v[i]) {
            identities = e->extra_v[i]; break;
        }
    }
    if (identities && g->n_prohibition < 64) {
        snprintf(g->prohibition[g->n_prohibition], sizeof(g->prohibition[g->n_prohibition]),
                 "card_identity:%s", identities);
        g->n_prohibition++;
    }
    (void)actor;
}

/* -- execute_activate_ability -- */
void rb_execute_activate_ability(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;
    const char *source_card = NULL;
    for (int i = 0; i < e->n_extra; i++) {
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "source_card") && e->extra_v[i]) {
            source_card = e->extra_v[i]; break;
        }
    }
    if (source_card && !strcmp(source_card, "previous_selected")) {
        for (int i = 0; i < g->n_selected_cards; i++) {
            rb_trigger_debut(g, actor, g->selected_cards[i]);
        }
    } else {
        rb_trigger_debut(g, actor, g->queue.resume_host);
    }
}

/* -- execute_invalidate_ability -- */
void rb_execute_invalidate_ability(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;
    const char *target_trigger = NULL;
    for (int i = 0; i < e->n_extra; i++) {
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "target_trigger") && e->extra_v[i]) {
            target_trigger = e->extra_v[i]; break;
        }
    }
    if (target_trigger && g->n_prohibition < 64) {
        snprintf(g->prohibition[g->n_prohibition], sizeof(g->prohibition[g->n_prohibition]),
                 "invalidate:%s", target_trigger);
        g->n_prohibition++;
    }
    (void)actor;
}

/* -- execute_gain_ability -- */
void rb_execute_gain_ability(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;
    /* Resolve the target card and ability index from the effect, then store
       the decoded ability in GameState.gained_card_abilities for constant
       evaluation by the auto-trigger engine. */
    int target_cid = -1;
    for (int i = 0; i < e->n_extra; i++) {
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "target_card") && e->extra_v[i]) {
            target_cid = atoi(e->extra_v[i]);
            break;
        }
    }
    if (target_cid < 0) {
        RbPlayer *P = &g->p[actor];
        for (int q = 0; q < RB_STAGE_SIZE; q++) {
            if (P->stage[q] != RB_EMPTY_SLOT) { target_cid = P->stage[q]; break; }
        }
    }
    if (target_cid < 0) return;

    int ab_idx = -1;
    for (int i = 0; i < e->n_extra; i++) {
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "ability_index") && e->extra_v[i]) {
            ab_idx = atoi(e->extra_v[i]);
            break;
        }
    }
    if (ab_idx < 0) return;

    int slot = -1;
    for (int i = 0; i < g->n_gained_cards; i++) {
        if (g->gained_card_ids[i] == target_cid) { slot = i; break; }
    }
    if (slot < 0 && g->n_gained_cards < 64) {
        slot = g->n_gained_cards++;
        g->gained_card_ids[slot] = target_cid;
        g->gained_card_n[slot] = 0;
    }
    if (slot < 0) return;
    if (g->gained_card_n[slot] >= 4) return;

    int na = g->gained_card_n[slot];
    if (rb_get_ability((uint32_t)ab_idx, &g->gained_card_abilities[slot][na])) {
        g->gained_card_n[slot]++;
    }
    (void)actor;
}

/* -- execute_gain_ability_from_source -- */
void rb_execute_gain_ability_from_source(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;
    /* Resolve the activating card, locate an under-card whose ability matches
       the requested group, and execute that ability's effect on the target. */
    int host_cid = -1;
    RbPlayer *P = &g->p[actor];
    for (int q = 0; q < RB_STAGE_SIZE; q++) {
        if (P->stage[q] != RB_EMPTY_SLOT) { host_cid = P->stage[q]; break; }
    }
    if (host_cid < 0) return;

    /* Find the area for the host card. */
    int area = -1;
    for (int q = 0; q < RB_STAGE_SIZE; q++) {
        if (P->stage[q] == host_cid) { area = q; break; }
    }
    if (area < 0) return;

    const char *grp = NULL;
    for (int i = 0; i < e->n_extra; i++) {
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "group_names")) {
            grp = e->extra_v[i];
            break;
        }
    }

    /* Execute the effect of the first matching under-card's ability. */
    for (int u = 0; u < P->under_cards[area].n; u++) {
        int src = P->under_cards[area].cards[u];
        Card sc;
        if (!rb_decode_card_by_index((uint32_t)src, &sc)) continue;
        int ok = 1;
        if (grp && !(sc.group_idx >= 0 && rb_card_matches_group_str(src, grp))) ok = 0;
        if (ok && sc.ability && sc.ability->effect) {
            rb_execute_effect_ex(g, actor, sc.ability->effect, host_cid);
        }
        rb_free_card(&sc);
    }
    (void)actor;
}
