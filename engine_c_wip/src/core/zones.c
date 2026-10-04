#include "rabuka.h"
#include <string.h>
#include <stdio.h>

int rb_member_area_to_index(const char *area){
    if(!area) return -1;
    if(!strcmp(area,"left")||!strcmp(area,"left_side")||!strcmp(area,"LeftSide")) return 0;
    if(!strcmp(area,"center")||!strcmp(area,"Center")) return 1;
    if(!strcmp(area,"right")||!strcmp(area,"right_side")||!strcmp(area,"RightSide")) return 2;
    return -1;
}
const char *rb_member_area_to_str(int idx){
    if(idx==0) return "left";
    if(idx==1) return "center";
    if(idx==2) return "right";
    return "?";
}
int rb_member_area_front(int area){
    if(area==0) return 2;
    if(area==1) return 1;
    if(area==2) return 0;
    return -1;
}

/* Mirror zones.rs: total_blades, draw, draw_bottom, draw_multiple, refresh.
   The old group banner also claimed "invariant"; see rb_zone_invariant below
   for why there is nothing in zones.rs to mirror there. */
/* rb_zone_invariant -- Rust has no Stage::invariant to mirror, so this is the
   whole-board shape check and its contract is written out rather than implied:
   every stage slot holds either RB_EMPTY_SLOT or a real card_no, and a
   non-empty under-card bag has a HOST (Rule 4.5.5: a member under-card exists
   only beneath a member). The previous body advertised "under_cards mirror" in
   the comment and then checked neither -- a stage emptied by a bare-array
   rb_stage_position_change call while under-cards were still parked there
   reported OK. Under-cards with no host are invisible to every consumer in the tree
   (rb_player_all_card_ids walks the bags unconditionally but
   rb_stage_under_cards_with_hosts, the only reader that knows a host, skips an
   empty area), so this is the one place that can see them. */
int rb_zone_invariant(const GameState *g) {
    if (!g) return 0;
    for (int pl = 0; pl < 2; pl++) {
        const RbPlayer *P = &g->p[pl];
        for (int s = 0; s < RB_STAGE_SIZE; s++) {
            if (P->stage[s] != RB_EMPTY_SLOT && P->stage[s] < 0) return 0;
            if (P->under_cards[s].n > 0 && P->stage[s] == RB_EMPTY_SLOT) return 0;
            if (P->under_cards[s].n < 0) return 0;
        }
    }
    return 1;
}
int rb_zone_total_blades(const GameState *g, int pl, int include_waited) {
    if (!g) return 0;
    const RbPlayer *P = &g->p[pl];
    int total = 0;
    for (int s = 0; s < RB_STAGE_SIZE; s++) {
        int cid = P->stage[s];
        if (cid == RB_EMPTY_SLOT) continue;
        if (!include_waited) {
            const char *orientation = rb_mods_get_orientation((RbMods *)&g->mods, cid);
            if (P->stage_wait[s] || (orientation && !strcmp(orientation, "wait"))) continue;
        }
        Card c; if (!rb_decode_card_by_index((uint32_t)cid, &c)) continue;
        int eff = (int)c.blade + rb_mods_get_blade((RbMods *)&g->mods, cid);
        rb_free_card(&c);
        if (eff < 0) eff = 0; if (eff > 255) eff = 255;
        total += eff;
    }
    return total;
}
int rb_zone_draw(GameState *g, int pl) {
    if (!g) return -1;
    RbBag *deck = &g->p[pl].deck;
    if (deck->n == 0) return -1;
    int cid = deck->cards[0];
    for (int i = 0; i < deck->n - 1; i++) deck->cards[i] = deck->cards[i+1];
    deck->n--;
    return cid;
}
int rb_zone_draw_bottom(GameState *g, int pl) {
    if (!g) return -1;
    RbBag *deck = &g->p[pl].deck;
    if (deck->n == 0) return -1;
    return deck->cards[--deck->n];
}
int rb_zone_draw_multiple(GameState *g, int pl, int n) {
    if (!g) return 0;
    int drawn = 0;
    for (int i = 0; i < n; i++) {
        if (rb_zone_draw(g, pl) < 0) break;
        drawn++;
    }
    return drawn;
}
/* Mirror Player::refresh (player.rs:634-648) -- shuffle the waitroom back into the
    main deck. Rust's main_deck.cards is a SmallVec with no bound, so the whole
    waitroom always goes across; the C RbBag is capped at RB_MAX_ZONE, so the
    surplus has to STAY in the waitroom rather than be dropped. The old body
    cleared `discard.n` unconditionally after copying only as many cards as
    fitted, which is the same "card leaves one zone and arrives in none" shape
    as the three defects fixed today, reached whenever deck.n + discard.n
    exceeded the bag.

    RULE 10.2.2.1 PRECONDITION, restored 2026-10-03. Rust's guard is a
    conjunction (player.rs:636):

        if self.main_deck.is_empty() && !self.waitroom.cards.is_empty() { ... }

    and BOTH halves are load-bearing. This body tested only the second, so it
    spliced the waitroom into a deck that was NOT empty -- and because
    Rule 10.2.3 appends ("refreshed cards go to the BOTTOM"), a mid-effect
    refresh that fires while any card remains above the splice point silently
    changes the order of a live deck. Every current production caller already
    tests `deck.n == 0` first (engine.c:187 rb_draw, phase.c:530's
    rb_player_refresh, effects/look.c:366 and :856), so the corrected guard is
    behaviour-neutral on the production path; it is the twin-without-the-rule
    that was wrong. `tests/test_audit_zones.c` test_refresh_conserves_cards
    pinned the OLD answer and now pins both Rust arms. */
int rb_zone_refresh(GameState *g, int pl) {
    if (!g) return 0;
    RbPlayer *P = &g->p[pl];
    if (P->deck.n != 0) return 0;      /* Rule 10.2.2.1: main deck empty */
    if (P->discard.n == 0) return 0;
    rb_shuffle(P->discard.cards, P->discard.n);
    int moved = 0;
    while (moved < P->discard.n && P->deck.n < RB_MAX_ZONE)
        P->deck.cards[P->deck.n++] = P->discard.cards[moved++];
    if (moved < P->discard.n) {
        for (int i = moved; i < P->discard.n; i++)
            P->discard.cards[i - moved] = P->discard.cards[i];
        P->discard.n -= moved;
    } else {
        P->discard.n = 0;
    }
    P->deck_refreshed_this_turn = 1;
    return 1;
}
/* Mirror MemberArea::to_tag (Rule 4.5.7 wire protocol: 1=left, 2=center, 3=right). */
uint8_t rb_member_area_to_tag(int idx){
    if(idx==0) return 1;
    if(idx==1) return 2;
    if(idx==2) return 3;
    return 0;
}

/* Mirror MemberArea::from_tag — inverse of rb_member_area_to_tag. */
int rb_member_area_from_tag(uint8_t tag){
    if(tag==1) return 0;
    if(tag==2) return 1;
    if(tag==3) return 2;
    return -1;
}

int rb_check_trigger_position(const char *triggers, int card_position){
    if(!triggers) return 1;
    if(strstr(triggers,"左サイド") && card_position!=0) return 0;
    if(strstr(triggers,"右サイド") && card_position!=2) return 0;
    if(strstr(triggers,"センター") && card_position!=1) return 0;
    return 1;
}

int rb_check_effect_position(const char *effect_pos, int card_position){
    if(!effect_pos) return 1;
    if(strchr(effect_pos,',')){
        char buf[64]; strncpy(buf, effect_pos, 63); buf[63]=0;
        char *tok = strtok(buf, ",");
        while(tok){
            while(*tok==' ') tok++;
            char *end = tok+strlen(tok)-1; while(end>tok && *end==' ') *end--=0;
            if((!strcmp(tok,"center")||!strcmp(tok,"中央")) && card_position==1) return 1;
            if((!strcmp(tok,"left")||!strcmp(tok,"左")||!strcmp(tok,"左側")||!strcmp(tok,"left_side")) && card_position==0) return 1;
            if((!strcmp(tok,"right")||!strcmp(tok,"右")||!strcmp(tok,"右側")||!strcmp(tok,"right_side")) && card_position==2) return 1;
            tok = strtok(NULL, ",");
        }
        return 0;
    }
    if((!strcmp(effect_pos,"center")||!strcmp(effect_pos,"中央")) && card_position==1) return 1;
    if((!strcmp(effect_pos,"left")||!strcmp(effect_pos,"左")||!strcmp(effect_pos,"左側")||!strcmp(effect_pos,"left_side")) && card_position==0) return 1;
    if((!strcmp(effect_pos,"right")||!strcmp(effect_pos,"右")||!strcmp(effect_pos,"右側")||!strcmp(effect_pos,"right_side")) && card_position==2) return 1;
    if(!strcmp(effect_pos,"center")||!strcmp(effect_pos,"left")||!strcmp(effect_pos,"right")||
       !strcmp(effect_pos,"左")||!strcmp(effect_pos,"右")||!strcmp(effect_pos,"中央")||
       !strcmp(effect_pos,"左側")||!strcmp(effect_pos,"右側")||!strcmp(effect_pos,"left_side")||!strcmp(effect_pos,"right_side"))
        return 0;
    return 1;
}

int rb_stage_get_area(const int stage[RB_STAGE_SIZE], int area){
    if(area<0||area>=RB_STAGE_SIZE) return RB_EMPTY_SLOT;
    return stage[area];
}
void rb_stage_set_area(int stage[RB_STAGE_SIZE], int area, int card_id){
    if(area<0||area>=RB_STAGE_SIZE) return;
    stage[area]=card_id;
}
/* ── Stage::position_change (zones.rs:368-415), Rule 11.10 / 4.5.5.3 ─────────
   Rust's `Stage` is ONE struct holding BOTH `stage[3]` and `under_cards[3]`,
   so `position_change` can and does carry the under-card stacks across with
   their members -- zones.rs:396-400, "Rule 4.5.5.3: Swap under-cards along with
   the members", an UNCONDITIONAL crossed `mem::take` pair executed AFTER the
   `card_id == -1` source check (:391-394) and BEFORE the stage swap (:402-412).
   It is unconditional, so the stack travels even when the destination area is
   EMPTY (the plain-move arm at :408-412) -- a move is a transfer, not a copy.

   In the C model `stage` and `under_cards` are two SEPARATE arrays of RbPlayer,
   so the under-card half of the Rust struct is reachable only from an entry
   point that is HANDED THE PLAYER. Hence TWO entry points over one core:

     rb_stage_position_change(stage, from, to)        -- the `stage` field only
     rb_stage_position_change_with_bags(P, from, to)  -- the whole Stage

   Why the split rather than one signature: tests/test_parity_zones_position.c:301
   passes a BARE `int stage[3]` stack array to the first form, and no runtime
   test can tell that apart from a real `P->stage` -- the port has no global
   GameState to range-check the pointer against, and a `container_of`-style
   `(RbPlayer*)((char*)stage - offsetof(RbPlayer, stage))` recovery is unsafe by
   construction (offsetting a caller's OWN array yields a wild pointer and the
   crossed take would smash ~4 KB of the caller's stack). A separate entry point
   makes the bare-array case STRUCTURALLY unable to reach the under-card write:
   the write exists in exactly one function and that function's only parameter
   is an RbPlayer*.

   The bare-array form is retained (rather than the 3-arg name being widened,
   which would be a hard compile error in the two test files that hand-declare
   the current prototype: tests/test_parity_under_member.c:584 and
   tests/test_parity_zones_position.c:59) because it still mirrors the stage
   half exactly, for callers that hold a stage array with no under-card state.

   The ability-driven routes were already correct and are NOT what this fixes:
   src/ability/effects/state.c rb_resume_position_change and the two
   rb_pos_change_for_player branches each performed their own crossed take.
   They now delegate to rb_stage_position_change_with_bags so the Rule 4.5.5.3
   take has ONE implementation instead of three copies. */
static int rb_stage_position_change_core(int *stage, RbBag *under, int *stage_wait,
                                          int from_area, int to_area){
    if(from_area==to_area) return -1;
    if(from_area<0||from_area>=RB_STAGE_SIZE||to_area<0||to_area>=RB_STAGE_SIZE) return -1;
    int card_id = stage[from_area];
    if(card_id==RB_EMPTY_SLOT) return -1;
    /* Rule 4.5.5.3 -- zones.rs:396-400. UNCONDITIONAL and BEFORE the stage
       swap, so the stack travels on the plain-move arm as well. */
    if(under){
        RbBag from_under = under[from_area];
        RbBag to_under   = under[to_area];
        under[from_area] = to_under;
        under[to_area]   = from_under;
    }
    /* Rust keys the waited flag PER CARD (`orientation_modifiers: HashMap<i16,
       CardOrientation>`, read at zones.rs:427-428), so in Rust it follows the
       member across a swap for free -- Q136, "a weighed member moving areas
       remains weighed". The C parallel array is per AREA, so it has to be
       swapped explicitly or the wait re-attaches to whichever member lands in
       the slot. Same cross-write the three ability-driven sites avoided by hand. */
    if(stage_wait){
        int wait_from = stage_wait[from_area];
        int wait_to   = stage_wait[to_area];
        stage_wait[from_area] = wait_to;
        stage_wait[to_area]   = wait_from;
    }
    int dest = stage[to_area];
    if(dest!=RB_EMPTY_SLOT){
        stage[from_area]=dest;
        stage[to_area]=card_id;
    } else {
        stage[to_area]=card_id;
        stage[from_area]=RB_EMPTY_SLOT;
    }
    return card_id;
}

/* Mirror Stage::position_change over a bare stage array (zones.rs:376-394 and
   :402-412). No under-card state is reachable from here, by construction. */
int rb_stage_position_change(int stage[RB_STAGE_SIZE], int from_area, int to_area){
    if(!stage) return -1;
    return rb_stage_position_change_core(stage, NULL, NULL, from_area, to_area);
}

/* Mirror Stage::position_change over a whole player (zones.rs:368-415), so the
   Rule 4.5.5.3 crossed under-card take at :396-400 runs too. Returns the moved
   card id, or -1 on any of Rust's two refusals (:377 same area, :393 empty
   source). */
int rb_stage_position_change_with_bags(RbPlayer *P, int from_area, int to_area){
    if(!P) return -1;
    return rb_stage_position_change_core(P->stage, P->under_cards, P->stage_wait,
                                         from_area, to_area);
}

/* Mirror util::stage_first_empty (ability/util.rs:2541-2551). The probe order is
   CENTRE, then LEFT, then RIGHT -- NOT ascending slot index. The previous body
   scanned 0,1,2 and therefore answered LEFT whenever both the left and the centre
   were empty, which is a different SLOT, not a different order of the same slot.

   Why it matters (measured 2026-10-03, 985-ability corpus): this is the slot
   chooser for every placement that names `stage` / `empty_area` (10 + 8
   abilities) and for every `same_area` destination that reaches the same arm
   (7 more), plus `turn/phase.c`'s single-baton area default. `src/ability/util.c`
   had already noticed the disagreement and worked around it with a private
   `rb_util_stage_first_empty_rust` (util.c:1111-1116), refusing to fix the root
   because "zones.c is not this file". It is now, so the root is fixed here and
   the workaround is redundant (it stays until its owner collapses it; it is
   byte-identical to this body).

   Rust's `Option<usize>` -> `-1` is the `None` case, exactly as the old body's
   fallthrough. */
int rb_stage_first_empty(const int stage[RB_STAGE_SIZE]){
    if(!stage) return -1;
    if(stage[1]==RB_EMPTY_SLOT) return 1;   /* Centre first (util.rs:2542) */
    if(stage[0]==RB_EMPTY_SLOT) return 0;
    if(stage[2]==RB_EMPTY_SLOT) return 2;
    return -1;                              /* None (util.rs:2549) */
}

/* ── Stage under-card helpers (mirror Stage::place_under_card /
   get_under_cards / under_cards_with_hosts / recycle_under_cards) ── */

void rb_stage_place_under_card(RbPlayer *player, int area, int card_id){
    if(!player || area<0 || area>=RB_STAGE_SIZE) return;
    RbBag *b = &player->under_cards[area];
    if(b->n < RB_MAX_ZONE) b->cards[b->n++] = card_id;
}

int rb_stage_get_under_cards(const RbPlayer *player, int area, int *out, int max){
    if(!player || area<0 || area>=RB_STAGE_SIZE || !out) return 0;
    const RbBag *b = &player->under_cards[area];
    int n = 0;
    for(int i=0;i<b->n && n<max;i++) out[n++] = b->cards[i];
    return n;
}

/* Mirror Stage::under_cards_with_hosts — emit (under_card, host_member) pairs.
   out_under[k] = tucked card id, out_host[k] = the stage member above it. */
int rb_stage_under_cards_with_hosts(const RbPlayer *player, int *out_under, int *out_host, int max){
    if(!player || !out_under || !out_host) return 0;
    int n = 0;
    for(int a=0;a<RB_STAGE_SIZE && n<max;a++){
        int host = player->stage[a];
        if(host==RB_EMPTY_SLOT) continue;
        const RbBag *b = &player->under_cards[a];
        for(int i=0;i<b->n && n<max;i++){
            out_under[n] = b->cards[i];
            out_host[n] = host;
            n++;
        }
    }
    return n;
}

/* Mirror Stage::recycle_under_cards (Rule 10.5.3-10.5.4): energy under-cards
   route to the energy deck, member under-cards to the waitroom. out_wait /
   out_energy are filled (each capped at max); *n_wait / *n_energy receive the
   counts. Returns total moved (-1 on bad args). */
int rb_stage_recycle_under_cards(GameState *g, int pl, int area,
                                 int *out_wait, int *n_wait,
                                 int *out_energy, int *n_energy, int max){
    if(!g || pl<0 || pl>1 || area<0 || area>=RB_STAGE_SIZE || !out_wait ||
       !n_wait || !out_energy || !n_energy) return -1;
    RbBag *b = &g->p[pl].under_cards[area];
    *n_wait = 0; *n_energy = 0;
    for(int i=0;i<b->n;i++){
        int cid = b->cards[i];
        if(rb_card_is_energy(cid)){
            if(*n_energy < max) out_energy[(*n_energy)++] = cid;
        } else {
            if(*n_wait < max) out_wait[(*n_wait)++] = cid;
        }
    }
    b->n = 0; /* mem::take */
    return *n_wait + *n_energy;
}

/* Mirror Stage::can_place_card (Rule 8.2.2): only non-live cards may occupy the
   stage. Unknown card ids place nowhere. */
int rb_stage_can_place_card(const GameState *g, int pl, int card_id){
    (void)g; (void)pl;
    if(!rb_card_record((uint32_t)card_id)) return 0;
    return rb_card_is_live(card_id) ? 0 : 1;
}

/* ── Formation change (Rule 11.11) ──────────────────────────────────────
   THE RUST TWIN IS NOT IN zones.rs. `Stage` has no `formation_change`
   method (zones.rs:368-415 goes straight from position_change to
   total_blades); the whole-formation reorder lives in
   engine/src/ability/effects/position.rs:778-925
   `AbilityResolver::finalize_formation_change`, called from
   ability/choice.rs:3244 and :3276 once every planned member has been
   assigned a destination.

   WHY THE OLD BODY WAS WRONG IN BOTH HALVES. It walked the (from,to) list
   and applied ONE `position_change`-shaped move at a time, swapping
   `stage[f]`/`stage[t]` and `uc[f]`/`uc[t]`. A formation change is a
   PERMUTATION, not a sequence of pairwise swaps, so any cycle of length >= 2
   was applied twice and returned the board to where it started: for the
   two-member swap a->b, b->a both the members and the under-card stacks came
   back to their original areas. (Red proof:
   tests/test_parity_zones_position.c:427-438 — the swap is accepted and the
   two members do exchange, but LeftSide kept n=2 under-cards when the right
   member's single stack should have travelled with it.)

   Rust does not walk the moves at all. It takes the whole under-card array
   ONCE (`let old_under = core::mem::take(&mut player.stage.under_cards);`
   position.rs:797), computes each member's DESTINATION INDEX, and writes
   `new_under[dest_idx] = old_under[from_idx].clone()` (position.rs:833) in a
   single pass. A 2-cycle therefore exchanges the stacks, because each stack
   is read from its source slot and written straight to its final slot.

   So the member movement was ALSO per-move-wrong, not just the under-card
   half, and both halves are replaced here by the same direct-permutation
   shape. Rust's three phases are reproduced one-for-one; the only additions
   are the validations this entry point owns (see PREFLIGHT below).

   Rust's Phase 1 also maintains an `occupant` array, but its only reader is
   the `let _evicted = ...` binding at position.rs:827-831, which is prefixed
   with `_` and never used (Phase 2 recomputes the evicted occupant from
   `old_stage[dest_idx]` at :871). It is provably dead, so it is not ported. */
int rb_stage_formation_change(GameState *g, int pl,
                              const int *from_areas, const int *to_areas, int n){
    if(!g || pl<0 || pl>1 || !from_areas || !to_areas) return -1;
    /* position.rs:779-781 -- an empty plan is a successful no-op. */
    if(n<=0) return 0;
    RbPlayer *P = &g->p[pl];
    int   *stage = P->stage;
    int   *swait = P->stage_wait;
    RbBag *uc    = P->under_cards;

    /* PREFLIGHT -- every refusal happens before any mutation, so a rejected
       formation leaves the board EXACTLY as it was. Two of these are C-side
       validations with no Rust counterpart, kept because they are the entry
       point's contract:
         * two members targeting one area -- Rust makes this unreachable
           upstream (compute_valid_position_destinations skips a zone another
           planned member already claimed, position.rs:702-707), and the C
           signature is a flat array with no such guard;
         * a plan in which NO member changes area. Rust accepts it and
           changes nothing, but the C surface is a mutation call: reporting
           success for a plan that cannot move anything would let a caller
           record a formation change that never happened. A stay-in-place
           entry INSIDE a real formation is legitimate and is honoured --
           Rust's own formation_change_test::stay_in_place_no_new_move and
           the three-member rotation
           (live_start_subunit_threshold_formation_test.rs:74-86, which
           answers "right, center, left" and asserts the result [c, b, a],
           i.e. the center member keeping its own area) both rely on it, so
           the old blanket `if(f==t) return -1` also rejected two cases Rust
           performs. The empty-source check is hoisted here out of the apply
           loop, where it could refuse AFTER earlier moves had already
           mutated the board. */
    int changed = 0;
    for(int i=0;i<n;i++){
        if(from_areas[i]<0 || from_areas[i]>=RB_STAGE_SIZE) return -1;
        if(to_areas[i]<0   || to_areas[i]  >=RB_STAGE_SIZE) return -1;
        for(int j=i+1;j<n;j++) if(to_areas[i]==to_areas[j]) return -1;
        if(stage[from_areas[i]]==RB_EMPTY_SLOT) return -1;
        if(from_areas[i]!=to_areas[i]) changed++;
    }
    if(changed==0) return -1;

    int   old_stage[RB_STAGE_SIZE];
    int   old_wait[RB_STAGE_SIZE];
    RbBag old_under[RB_STAGE_SIZE];
    int   new_stage[RB_STAGE_SIZE];
    int   new_wait[RB_STAGE_SIZE];
    RbBag new_under[RB_STAGE_SIZE];
    for(int a=0;a<RB_STAGE_SIZE;a++){
        old_stage[a] = stage[a];
        old_wait[a]  = swait[a];
        old_under[a] = uc[a];
        new_stage[a] = RB_EMPTY_SLOT;
        new_wait[a]  = 0;
        new_under[a].n = 0;
    }

    /* Phase 1 (position.rs:806-840): place every planned member at its
       destination, carrying its under-cards across in the SAME write. */
    for(int i=0;i<n;i++){
        int from = from_areas[i], to = to_areas[i];
        if(from==to) continue;            /* stay-in-place is Phase 2's job */
        new_stage[to] = old_stage[from];
        new_under[to] = old_under[from];  /* position.rs:833 */
        new_wait[to]  = old_wait[from];
    }

    /* Phase 2 (position.rs:842-890): the evicted occupant goes to the
       mover's vacated position; an occupant that is itself a planned member
       was already relocated in Phase 1 and must not be re-inserted (that
       would duplicate one member across two areas). */
    for(int i=0;i<n;i++){
        int from = from_areas[i], to = to_areas[i];
        if(new_stage[from]!=RB_EMPTY_SLOT) continue;   /* slot already filled */
        if(from==to){                                   /* stayed in place */
            new_stage[from] = old_stage[from];
            new_under[from] = old_under[from];
            new_wait[from]  = old_wait[from];
            continue;
        }
        int evicted = old_stage[to];
        int evicted_is_planned = 0;
        for(int j=0;j<n;j++)
            if(old_stage[from_areas[j]]==evicted){ evicted_is_planned = 1; break; }
        if(evicted!=RB_EMPTY_SLOT && evicted!=old_stage[from] && !evicted_is_planned){
            new_stage[from] = evicted;
            new_under[from] = old_under[to];  /* position.rs:882 */
            new_wait[from]  = old_wait[to];
        }
    }

    /* Phase 3 (position.rs:892-901): unplanned members keep their own slot if
       it is still free, with their own under-cards. */
    for(int a=0;a<RB_STAGE_SIZE;a++){
        if(new_stage[a]!=RB_EMPTY_SLOT || old_stage[a]==RB_EMPTY_SLOT) continue;
        int is_planned = 0;
        for(int j=0;j<n;j++)
            if(old_stage[from_areas[j]]==old_stage[a]){ is_planned = 1; break; }
        if(!is_planned){
            new_stage[a] = old_stage[a];
            new_under[a] = old_under[a];
            new_wait[a]  = old_wait[a];
        }
    }

    /* Atomic update (position.rs:903-905). `stage_wait` rides along for the
       same reason the under-cards do: Rust keys the waited flag PER CARD in
       `orientation_modifiers`, so it follows the member for free, while the
       C parallel array is per AREA. Same reasoning as the crossed
       `stage_wait` swap in rb_stage_position_change_core (zones.c:229-240);
       without it a rotation leaves each member weighed by whatever area it
       used to occupy (Q136). */
    for(int a=0;a<RB_STAGE_SIZE;a++){
        stage[a] = new_stage[a];
        swait[a]  = new_wait[a];
        uc[a]     = new_under[a];
    }
    return 0;
}

/* ── EnergyZone helpers (mirror EnergyZone; backed by RbPlayer::energy /
   energy_active) ── */

int rb_energy_can_place_card(const RbPlayer *player, int card_id){
    (void)player;
    if(!rb_card_record((uint32_t)card_id)) return 0;
    return rb_card_is_energy(card_id) ? 1 : 0;
}

int rb_energy_add_card(RbPlayer *player, int card_id){
    if(!player) return -1;
    if(player->energy.n >= RB_MAX_ZONE) return -1;
    player->energy.cards[player->energy.n++] = card_id;
    player->energy_active++; /* new energy starts Active (Rule 7.4) */
    return 0;
}

int rb_energy_pay(RbPlayer *player, int amount){
    if(!player || amount<0) return -1;
    if(player->energy_active >= amount){
        player->energy_active -= amount; /* Rule 5.9 */
        return 0;
    }
    return -1;
}

void rb_energy_activate_all(RbPlayer *player){
    if(!player) return;
    player->energy_active = player->energy.n;
}

int rb_energy_active_count(const RbPlayer *player){
    return player ? player->energy_active : 0;
}

/* Mirror EnergyZone::set_active_count (Rule 7.4 energy activation state). */
void rb_energy_set_active_count(RbPlayer *player, int count){
    if(!player) return;
    if(count < 0) count = 0;
    player->energy_active = count;
}

/* Mirror EnergyZone::add_active — saturating increment of active energy.
 *
 * core/zones.rs:682-684:
 *     pub fn add_active(&mut self, delta: u8) {
 *         self.active_energy_count = self.active_energy_count.saturating_add(delta);
 *     }
 *
 * WAS DEAD-WRONG ON THE UPPER BOUND. Rust saturates on a `u8`, so
 * `saturating_add` clamps at 255. The C clamped only the lower bound (a
 * `delta <= 0` guard plus a post-hoc `< 0` fixup, and `delta` is a signed
 * `int` here where Rust's is a `u8`), and `RbPlayer.energy_active` is `int`
 * (include/rabuka.h:800) — so a large `delta` ran the count straight past 255
 * and every later reader that compared it against a Rust-shaped `u8` total
 * would disagree. Upper clamp added; both bounds now match.
 */
void rb_energy_add_active(RbPlayer *player, int delta){
    if(!player || delta <= 0) return;
    player->energy_active += delta;
    if(player->energy_active < 0) player->energy_active = 0;
    if(player->energy_active > 255) player->energy_active = 255;
}

/* Mirror EnergyZone::sub_active — saturating decrement of active energy. */
void rb_energy_sub_active(RbPlayer *player, int delta){
    if(!player || delta <= 0) return;
    player->energy_active -= delta;
    if(player->energy_active < 0) player->energy_active = 0;
}

/* ── LiveCardZone helpers (mirror LiveCardZone; backed by RbPlayer::live) ── */

/* Mirror LiveCardZone::can_place_card (zones.rs:518-521), which is
 * `true` unconditionally -- it ignores both the card database and the card id
 * and applies NO capacity rule. The C body invented one ("subject to the
 * live-zone capacity, RB_MAX_LIVE_CARDS") that Rust does not have, so the two
 * engines disagreed about whether a 4th card could enter the live zone; the
 * limit that does exist in Rust lives in the Live Card Set phase
 * (phases.rs:834 `if live_cards.len() >= MAX_LIVE_CARDS`), which the C port
 * reproduces in phase.c at rb_handle_set_live_card and
 * rb_handle_live_card_confirmation -- not here. */
int rb_live_can_place_card(const RbPlayer *player, int card_id){
    (void)player; (void)card_id;
    return 1;
}

/* Mirror LiveCardZone::add_card (zones.rs:523-535): it calls
 * can_place_card, which is unconditionally true, and then pushes -- so the
 * function cannot fail on a full zone and there is no capacity ceiling short
 * of the container itself. The C body refused at RB_MAX_LIVE_CARDS and
 * returned -1; every caller in the tree has already lifted the card out of
 * its source, so that -1 was a deletion. RB_MAX_ZONE is the RbBag bound and is
 * the only limit that can be expressed here. */
int rb_live_add_card(RbPlayer *player, int card_id){
    if(!player) return -1;
    if(player->live.n >= RB_MAX_ZONE) return -1;
    player->live.cards[player->live.n++] = card_id;
    return 0;
}

int rb_live_clear(RbPlayer *player, int *out, int max){
    if(!player || !out) return 0;
    int n = 0;
    for(int i=0;i<player->live.n && n<max;i++) out[n++] = player->live.cards[i];
    player->live.n = 0;
    return n;
}

int rb_live_len(const RbPlayer *player){
    return player ? player->live.n : 0;
}

/* ── Hand helpers (mirror Hand; backed by RbPlayer::hand) ── */

void rb_hand_add(RbPlayer *player, int card_id){
    if(!player) return;
    if(player->hand.n < RB_MAX_HAND) player->hand.cards[player->hand.n++] = card_id;
}

int rb_hand_remove_card(RbPlayer *player, int index){
    if(!player || index<0 || index>=player->hand.n) return -1;
    int c = player->hand.cards[index];
    for(int k=index;k<player->hand.n-1;k++) player->hand.cards[k] = player->hand.cards[k+1];
    player->hand.n--;
    return c;
}

int rb_hand_len(const RbPlayer *player){
    return player ? player->hand.n : 0;
}

int rb_hand_is_empty(const RbPlayer *player){
    return player ? player->hand.n==0 : 1;
}

/* ── Waitroom helpers (mirror Waitroom; backed by RbPlayer::discard) ── */

void rb_waitroom_add(RbPlayer *player, int card_id){
    if(!player) return;
    if(player->discard.n < RB_MAX_ZONE) player->discard.cards[player->discard.n++] = card_id;
}

int rb_waitroom_take_all(RbPlayer *player, int *out, int max){
    if(!player || !out) return 0;
    int n = 0;
    for(int i=0;i<player->discard.n && n<max;i++) out[n++] = player->discard.cards[i];
    player->discard.n = 0;
    return n;
}

void rb_waitroom_shuffle(GameState *g, int pl){
    if(!g || pl<0 || pl>1) return;
    rb_shuffle(g->p[pl].discard.cards, g->p[pl].discard.n);
}

int rb_waitroom_len(const RbPlayer *player){
    return player ? player->discard.n : 0;
}

void rb_waitroom_remove_card(RbPlayer *player, int card_id){
    if(!player) return;
    int w = 0;
    for(int i=0;i<player->discard.n;i++)
        if(player->discard.cards[i]!=card_id) player->discard.cards[w++] = player->discard.cards[i];
    player->discard.n = w;
}

/* ── SuccessLiveCardZone helpers (mirror SuccessLiveCardZone; RbPlayer::success) ── */

/* Mirror SuccessLiveCardZone::add_card (zones.rs:1002-1004), an unconditional
   `cards.push(card_id)`. The C body capped at RB_MAX_LIVE_CARDS (3) and
   SILENTLY DROPPED the overflow -- and the 4th success card is not
   hypothetical: live.c already guards its four call sites with
   `success.n < RB_MAX_ZONE` (live.c:1067, :1070, :1437, :1632), so the guard
   passes at success.n == 3 and this cap then eats the card. Rust has no such
   ceiling, and the two C placement routes disagreed with each other: the
   generic rb_place_card_in_zone() writes into P->success up to RB_MAX_ZONE.
   RB_MAX_ZONE is the RbBag bound, which is the only limit expressible here. */
void rb_success_add(RbPlayer *player, int card_id){
    if(!player) return;
    if(player->success.n < RB_MAX_ZONE) player->success.cards[player->success.n++] = card_id;
}

int rb_success_len(const RbPlayer *player){
    return player ? player->success.n : 0;
}

/* ── ResolutionZone helpers (mirror ResolutionZone; GameState::resolution) ────

   OWNERS. Rust's ResolutionZone is TWO parallel vectors (engine/src/core/zones.rs:
   1049-1053): `cards: SmallVec<[i16;10]>` and `owners: SmallVec<[u8;10]>`, both
   appended by add_card_for_owner (zones.rs:1067-1070), both SWAPPED by
   swap_slots (zones.rs:1089-1090), and owners CLEARED by clear (zones.rs:1095).
   The C had only the card bag, so the owner of a resolved card was structurally
   absent and the single drain (rb_check_invalid_resolution_zone, phase.c) had
   to assume the ACTIVE seat -- which is why a card yelled by the non-active
   performer came back to the wrong waitroom. include/rabuka.h:1617-1618 now
   carries `resolution_owners[RB_MAX_ZONE]` + `n_resolution_owners` as the
   parallel array, and rb_resolution_add_for_owner is the twin of
   add_card_for_owner.

   THE SENTINEL. RB_RESOLUTION_OWNER_UNKNOWN (0xFF) is stored for a card pushed
   by rb_resolution_add, i.e. the C's no-owner form. It is the exact observable
   equivalent of Rust's short `owners` vector: the reader falls back to the
   active seat (`owners.get(index).copied().unwrap_or(active)`,
   turn/actions/mod.rs:1867-1868) and cannot distinguish "no owner recorded"
   from "owner out of range", which is what Rust's Option<i16> = None does.
   RB_RESOLUTION_OWNER_UNKNOWN is referenced by that reader in src/turn/phase.c;
   keep the two in step. */
#define RB_RESOLUTION_OWNER_UNKNOWN 0xFF

void rb_resolution_add_for_owner(GameState *g, int card_id, int owner) {
    if (!g) return;
    if (g->resolution.n >= RB_MAX_ZONE) return;   /* saturate, never wrap */
    g->resolution_owners[g->resolution.n] = (uint8_t)((owner >= 0 && owner <= 1)
                                                     ? owner
                                                     : RB_RESOLUTION_OWNER_UNKNOWN);
    g->resolution.cards[g->resolution.n++] = card_id;
    /* The two vectors are the same length by construction here, so the count is
       the parallel-array invariant, not a second index: it is what lets the
       reader tell "this entry has an owner stamp" from "this entry was poked
       into g->resolution directly" (several suites do exactly that). */
    g->n_resolution_owners = g->resolution.n;
}

/* Rust's add_card (zones.rs:1063-1065) is add_card_for_owner(card_id, 0) -- a
   literal seat 0, NOT the active seat. The C keeps the ACTIVE-seat behaviour it
   had before the owners field existed, which is Rust's `unwrap_or(active)`
   fallback rather than its `add_card`, because the C's only producer
   (rb_perform_cheer_check, src/core/tracking.c) knows the yeller and passes it
   only once the call site below is swapped. That call site is reported as
   BLOCKED-ON tracking.c; until it lands the two forms are behaviourally
   identical, which is why this is not a silent divergence. */
void rb_resolution_add(GameState *g, int card_id){
    rb_resolution_add_for_owner(g, card_id, RB_RESOLUTION_OWNER_UNKNOWN);
}

int rb_resolution_clear(GameState *g, int *out, int max){
    if(!g || !out) return 0;
    int n = 0;
    for(int i=0;i<g->resolution.n && n<max;i++) out[n++] = g->resolution.cards[i];
    g->resolution.n = 0;
    g->n_resolution_owners = 0;   /* zones.rs:1095 `self.owners.clear()` */
    return n;
}

int rb_resolution_len(const GameState *g){
    return g ? g->resolution.n : 0;
}

/* Mirror ResolutionZone::swap_slots (engine/src/core/zones.rs:1072-1092) --
   Rule 5.8, the two-card exchange inside the resolution zone. Returns 0 on
   success, -1 on any of Rust's four refusals.

   Rust's preflight, in order, and the C collapse of each:
     1. left == right            -> Err "Cannot swap the same ... slot"
     2. cards[left]  out of range-> Err "Left ... out of bounds"
     3. cards[right] out of range-> Err "Right ... out of bounds"
     4. owners[left]/[right] absent -> Err "... has no owner"
   Checks 1-3 map one-for-one onto the RbBag. Check 4 now has a counterpart too:
   the owners vector is parallel by construction (rb_resolution_add_for_owner
   extends both, rb_resolution_clear clears both), so `owners[left]` is absent
   exactly when `left >= g->n_resolution_owners` -- an entry written straight
   into g->resolution.cards[] rather than through the zone API. That refusal is
   reachable, so it is enforced rather than described.

   The MUTATION is no longer just the ids: Rust swaps cards AND owners
   (zones.rs:1089-1090), and with a real owners vector the second swap is
   load-bearing -- without it a Rule 5.8 exchange would hand each card to the
   other seat's waitroom. */
int rb_resolution_swap_slots(GameState *g, int left_slot, int right_slot){
    if(!g) return -1;
    if(left_slot == right_slot) return -1;
    if(left_slot < 0 || left_slot >= g->resolution.n) return -1;
    if(right_slot < 0 || right_slot >= g->resolution.n) return -1;
    if(left_slot >= g->n_resolution_owners || right_slot >= g->n_resolution_owners) return -1;
    int tmp = g->resolution.cards[left_slot];
    g->resolution.cards[left_slot] = g->resolution.cards[right_slot];
    g->resolution.cards[right_slot] = tmp;
    uint8_t otmp = g->resolution_owners[left_slot];
    g->resolution_owners[left_slot] = g->resolution_owners[right_slot];
    g->resolution_owners[right_slot] = otmp;
    return 0;
}

/* Mirror MemberArea::from_index — inverse of rb_member_area_to_index. */
int rb_member_area_from_index(int idx){
    if(idx==0) return 0;
    if(idx==1) return 1;
    if(idx==2) return 2;
    return -1;
}

/* Mirror MainDeck::shuffle (Rule: index 0 = top). */
void rb_deck_shuffle(GameState *g, int pl){
    if(!g || pl<0 || pl>1) return;
    rb_shuffle(g->p[pl].deck.cards, g->p[pl].deck.n);
}

/* Mirror MainDeck::is_empty. */
int rb_deck_is_empty(const GameState *g, int pl){
    if(!g || pl<0 || pl>1) return 1;
    return g->p[pl].deck.n == 0;
}

/* Mirror MainDeck::len. */
int rb_deck_len(const GameState *g, int pl){
    if(!g || pl<0 || pl>1) return 0;
    return g->p[pl].deck.n;
}

/* Mirror EnergyDeck::draw — draw the top energy card. */
int rb_energy_deck_draw(GameState *g, int pl){
    if(!g || pl<0 || pl>1) return -1;
    RbBag *d = &g->p[pl].energy_deck;
    if(d->n == 0) return -1;
    int cid = d->cards[0];
    for(int i=0;i<d->n-1;i++) d->cards[i] = d->cards[i+1];
    d->n--;
    return cid;
}

/* Mirror EnergyDeck::is_empty. */
int rb_energy_deck_is_empty(const GameState *g, int pl){
    if(!g || pl<0 || pl>1) return 1;
    return g->p[pl].energy_deck.n == 0;
}

/* Mirror LiveCardZone::calculate_live_score's heart gate (zones.rs:577-597)
 * ON TOP OF the one real port of `check_heart_requirement`
 * (src/ability/effects/state.c:1991 `rb_check_heart_requirement`, twin of
 * core/card.rs:4360-4403). There is deliberately NO second implementation of
 * the wildcard/leftover rule here any more.
 *
 * The old body was a private `local_check_heart_req` and it DISAGREED with Rust
 * in three measurable ways:
 *
 *  1. Rust iterates `&need.hearts`, a HeartMap whose keys INCLUDE
 *     `HeartColor::All` (index 7), so a live card whose need is
 *     「N枚以上のハート」/`heart_type:"all"` is checked against bucket 7.
 *     The old loop ran `for(c=1;c<7;c++)` and NEVER LOOKED AT INDEX 7, so such
 *     a requirement was silently treated as absent and the card scored.
 *  2. Rust tracks a `remaining` copy of the provided table and, after the loop,
 *     checks `leftover_sum + max(wildcard_remaining,0) >= need[Heart00]`
 *     (card.rs:4392-4401). The old body had no `remaining` at all, so a
 *     Heart00 ("heartless") requirement could never be evaluated.
 *  3. Rust's guard is `if need.hearts.is_empty() { return true }` BEFORE
 *     anything touches `provided`; the old body reached into `provided` while
 *     summing `total_prov`.
 *
 * The empty-need early-true is reproduced here (it is Rust's line 4361, and it
 * is what lets a `stage_hearts == NULL` call answer "satisfied" for a card with
 * no printed requirement). `rb_check_heart_requirement(need, NULL)` returns 0,
 * which is exactly Rust's `else { false }` for a NON-empty need with no stage
 * hearts (zones.rs:592-594). */
static int live_score_hearts_satisfied(const int *need, const int *provided){
    int any = 0;
    for(int c=0;c<8;c++) if(need[c] > 0){ any = 1; break; }
    if(!any) return 1;               /* card.rs:4361-4363 */
    return rb_check_heart_requirement(need, provided);
}

/* Mirror LiveCardZone::calculate_live_score (Rule 5.2/8.2). Iterates the live
 * card zone, sums each card's (base_score + score_modifier) when its need_heart
 * is satisfied by stage_hearts, then adds cheer_blade_heart_count and the
 * constant_total_score_bonus — all saturated to u8. */
int rb_live_calculate_score(const GameState *g, int pl, int cheer_blade_heart_count,
                            const int *stage_hearts, int constant_total_score_bonus){
    if(!g || pl<0 || pl>1) return 0;
    const RbPlayer *P = &g->p[pl];
    int total_score = 0;
    for(int i=0;i<P->live.n;i++){
        int cid = P->live.cards[i];
        Card c;
        if(!rb_decode_card_by_index((uint32_t)cid, &c)) continue;
        int base_score = (int)c.score;
        int modifier = rb_mods_get_score((RbMods *)&g->mods, cid);
        int card_score = base_score + modifier;
        if(card_score < 0) card_score = 0;
        if(card_score > 255) card_score = 255;
        int need[8] = {0};
        rb_effective_need_heart(g, cid, need);
        int satisfied = live_score_hearts_satisfied(need, stage_hearts);
        rb_free_card(&c);
        if(satisfied) total_score += card_score;
    }
    total_score += cheer_blade_heart_count;
    /* saturate_u8(constant_total_score_bonus): negative contributions clamp to 0
       (zones.rs:565-566). Callers that pass the live mods field get correct
       behaviour; legacy 0 callers are unchanged. */
    if (constant_total_score_bonus > 0) total_score += constant_total_score_bonus;
    if(total_score < 0) total_score = 0;
    if(total_score > 255) total_score = 255;
    return total_score;
}

/* ── Ported from zones.rs (Stage) ── */

/* Mirror Stage::get_available_hearts — delegates to stats_pipeline::stage_hearts.
 * out must point to an 8-element int array (one per heart color). */
void rb_stage_get_available_hearts(const GameState *g, int pl, int out[8]){
    if(!g || pl<0 || pl>1 || !out) return;
    rb_stage_hearts_pipeline(g, pl, out);
}

/* Mirror Stage::get_available_hearts_i32 — legacy adapter that converts i32
 * heart modifiers to ModifierEntry and delegates. The C engine stores heart
 * modifiers as RbModifierEntry, so this applies the i32 deltas additively on
 * top of the existing mods, computes stage hearts, then restores them. */
void rb_stage_get_available_hearts_i32(const GameState *g, int pl, int out[8],
                                       const int *card_ids, const int *deltas,
                                       int n_mods){
    if(!g || pl<0 || pl>1 || !out) return;
    RbMods *m = (RbMods *)&g->mods;
    /* apply i32 deltas additively */
    for(int i=0;i<n_mods && card_ids && deltas;i++){
        int cid = card_ids[i];
        if(cid<0||cid>=RB_MAX_CARD_IDS) continue;
        for(int c=0;c<8;c++){
            m->heart[cid][c].add += (int16_t)deltas[i];
        }
    }
    rb_stage_hearts_pipeline(g, pl, out);
    /* restore: remove the deltas we added */
    for(int i=0;i<n_mods && card_ids && deltas;i++){
        int cid = card_ids[i];
        if(cid<0||cid>=RB_MAX_CARD_IDS) continue;
        for(int c=0;c<8;c++){
            m->heart[cid][c].add -= (int16_t)deltas[i];
        }
    }
}

/* Mirror ExclusionZone::add_card (zones.rs:1038-1041) -- a bare
   `cards.push(card_id)`. The previous body opened with a membership scan and
   returned early if the id was already present; Rust has no such guard, so the
   C silently refused a second push of the same id. Nothing depends on that
   (0 external callers tree-wide) and `exclusion_zone` is 0 of the 985 corpus
   zone strings, so removing it cannot move a verdict -- it just makes the twin
   the one Rust wrote. */
void rb_exclusion_add(GameState *g, int card_id){
    if(!g || card_id < 0) return;
    if(g->exclusion_zone.n < RB_MAX_ZONE)
        g->exclusion_zone.cards[g->exclusion_zone.n++] = card_id;
}

/* ── Ported from player.rs (Player) ── */

/* Mirror Player::set_main_deck — replaces the main deck with the given cards. */
void rb_player_set_main_deck(GameState *g, int pl, const int *cards, int n){
    if(!g || pl<0 || pl>1) return;
    RbBag *deck = &g->p[pl].deck;
    deck->n = 0;
    for(int i=0;i<n && deck->n<RB_MAX_ZONE;i++){
        deck->cards[deck->n++] = cards[i];
    }
}

/* Mirror Player::set_energy_deck — replaces the energy deck with the given cards. */
void rb_player_set_energy_deck(GameState *g, int pl, const int *cards, int n){
    if(!g || pl<0 || pl>1) return;
    RbBag *deck = &g->p[pl].energy_deck;
    deck->n = 0;
    for(int i=0;i<n && deck->n<RB_MAX_ZONE;i++){
        deck->cards[deck->n++] = cards[i];
    }
}

/* Mirror Player::get_card_index_by_id — returns the index of card_id in hand,
 * or -1 if not found. */
int rb_player_get_card_index_by_id(const RbPlayer *player, int card_id){
    if(!player) return -1;
    for(int i=0;i<player->hand.n;i++){
        if(player->hand.cards[i]==card_id) return i;
    }
    return -1;
}

/* Mirror Player::is_area_locked — checks if the given stage area currently
 * holds a member deployed this turn (Rule 9.6.2.1.2.1). The C engine tracks
 * this via stage_arrived[pl][area]. */
int rb_player_is_area_locked(const GameState *g, int pl, int area){
    if(!g || pl<0 || pl>1 || area<0 || area>=RB_STAGE_SIZE) return 0;
    return g->stage_arrived[pl][area];
}

/* Mirror Player::remove_member_from_stage_with_recycling — removes the member
 * at the given stage index, recycles its under-cards (member under-cards to
 * waitroom, energy under-cards to energy deck), and clears the deployment
 * tracking. Returns the removed member card ID, or -1 on failure. */
int rb_player_remove_member_from_stage_with_recycling(GameState *g, int pl, int index){
    if(!g || pl<0 || pl>1 || index<0 || index>=RB_STAGE_SIZE) return -1;
    RbPlayer *P = &g->p[pl];
    int card_id = P->stage[index];
    if(card_id==RB_EMPTY_SLOT) return -1;
    P->stage[index] = RB_EMPTY_SLOT;
    P->stage_wait[index] = 0;
    /* recycle under-cards */
    int wait[RB_MAX_ZONE], energy[RB_MAX_ZONE];
    int n_wait=0, n_energy=0;
    int moved = rb_stage_recycle_under_cards(g, pl, index,
                                              wait, &n_wait,
                                              energy, &n_energy, RB_MAX_ZONE);
    if(moved < 0) return -1;
    for(int i=0;i<n_wait;i++) rb_waitroom_add(P, wait[i]);
    for(int i=0;i<n_energy;i++){
        if(P->energy_deck.n < RB_MAX_ZONE)
            P->energy_deck.cards[P->energy_deck.n++] = energy[i];
    }
    /* clear deployment tracking for this area */
    g->stage_arrived[pl][index] = 0;
    return card_id;
}

/* Mirror Player::move_card_from_hand_to_stage — plays a member card from hand
 * to the given stage area. Handles baton touch (swapping with existing member),
 * energy cost payment, and deployment tracking.
 * Returns 0 on success, -1 on failure. */
int rb_player_move_card_from_hand_to_stage(GameState *g, int pl, int hand_index,
                                            int stage_area, int use_baton_touch){
    if(!g || pl<0 || pl>1) return -1;
    RbPlayer *P = &g->p[pl];
    if(hand_index<0 || hand_index>=P->hand.n) return -1;
    int card_id = P->hand.cards[hand_index];
    if(!rb_card_is_member(card_id)){
        return -1;
    }
    /* compute cost via the ability util (mirrors compute_play_cost) */
    int cost_to_pay = rb_compute_play_cost(g, pl, card_id, 0);
    if(cost_to_pay < 0) cost_to_pay = 0;
    /* check for existing member in target area (baton touch scenario) */
    int existing = P->stage[stage_area];
    int replaced_member_cost = 0;
    int replaced_id = -1;
    if(existing != RB_EMPTY_SLOT){
        /* baton touch: check if area is locked */
        if(g->stage_arrived[pl][stage_area]) return -1;
        /* check baton touch protection */
        if(rb_has_cannot_baton_touch_protection(card_id, existing)) return -1;
        /* get replaced member's cost for reduction */
        Card ec;
        if(rb_decode_card_by_index((uint32_t)existing, &ec)){
            int base_cost = ec.cost;
            int cost_mod = rb_mods_get_cost((RbMods*)&g->mods, existing);
            replaced_member_cost = (int)base_cost + cost_mod;
            if(replaced_member_cost < 1) replaced_member_cost = 1;
            rb_free_card(&ec);
        }
        cost_to_pay = cost_to_pay - replaced_member_cost;
        if(cost_to_pay < 0) cost_to_pay = 0;
    } else if(use_baton_touch){
        /* no member to baton touch */
        return -1;
    }
    /* check energy */
    if(cost_to_pay > 0 && P->energy_active < cost_to_pay) return -1;
    /* pay energy */
    if(cost_to_pay > 0){
        if(rb_energy_pay(P, cost_to_pay) != 0) return -1;
    }
    /* remove card from hand */
    rb_hand_remove_card(P, hand_index);
    /* handle existing member (send to waitroom) */
    if(existing != RB_EMPTY_SLOT){
        replaced_id = rb_player_remove_member_from_stage_with_recycling(g, pl, stage_area);
        rb_waitroom_add(P, replaced_id);
    }
    /* place new member on stage */
    P->stage[stage_area] = card_id;
    P->stage_wait[stage_area] = 0;
    /* track deployment */
    g->stage_arrived[pl][stage_area] = 1;
    return 0;
}

/* Mirror Player::calculate_stage_hearts — delegates to
 * stats_pipeline::stage_hearts (same as Stage::get_available_hearts). */
void rb_player_calculate_stage_hearts(const GameState *g, int pl, int out[8]){
    if(!g || pl<0 || pl>1 || !out) return;
    rb_stage_hearts_pipeline(g, pl, out);
}

/* Mirror Player::activate_all_energy — activates all energy cards. */
void rb_player_activate_all_energy(RbPlayer *player){
    if(!player) return;
    rb_energy_activate_all(player);
}

/* Mirror Player::activate_all_energy_exclude — activates all energy cards
 * except `excluded` cards that carry a do-not-activate flag. The C engine
 * tracks this as an aggregate count, so we subtract the excluded count. */
void rb_player_activate_all_energy_exclude(RbPlayer *player, int excluded){
    if(!player) return;
    int total = player->energy.n;
    int active = total - excluded;
    if(active < 0) active = 0;
    player->energy_active = active;
}

/* Mirror Player::all_card_ids — collects every card ID this player owns
 * across all zones into out[]. Returns the count written. */
int rb_player_all_card_ids(const GameState *g, int pl, int *out, int max){
    if(!g || pl<0 || pl>1 || !out || max<=0) return 0;
    const RbPlayer *P = &g->p[pl];
    int n = 0;
    for(int i=0;i<P->deck.n && n<max;i++) out[n++] = P->deck.cards[i];
    for(int i=0;i<P->hand.n && n<max;i++) out[n++] = P->hand.cards[i];
    for(int i=0;i<P->energy.n && n<max;i++) out[n++] = P->energy.cards[i];
    for(int i=0;i<P->energy_deck.n && n<max;i++) out[n++] = P->energy_deck.cards[i];
    for(int i=0;i<P->discard.n && n<max;i++) out[n++] = P->discard.cards[i];
    for(int i=0;i<P->live.n && n<max;i++) out[n++] = P->live.cards[i];
    for(int i=0;i<P->success.n && n<max;i++) out[n++] = P->success.cards[i];
    for(int s=0;s<RB_STAGE_SIZE;s++){
        if(P->stage[s]!=RB_EMPTY_SLOT && n<max) out[n++] = P->stage[s];
    }
    for(int s=0;s<RB_STAGE_SIZE;s++){
        const RbBag *uc = &P->under_cards[s];
        for(int i=0;i<uc->n && n<max;i++) out[n++] = uc->cards[i];
    }
    return n;
}

/* Mirror Player::draw_card -- draws the top card from main deck to hand.
 * Returns the card ID, or -1 if deck is empty.
 *
 * Rust's Hand is a SmallVec with no bound (zones.rs:890-892, add_card at
 * :908-910 is an unconditional push), so a draw into a 41-card hand succeeds
 * there and fails here only because RbPlayer::hand is a fixed RB_MAX_HAND
 * array. The bound is checked BEFORE rb_zone_draw(), not after: rb_zone_draw()
 * has already lifted the card out of the deck by the time it returns, and the
 * old rb_hand_add() then discarded it on the `hand.n < RB_MAX_HAND` test, so a
 * full hand turned one draw into one destroyed card -- invisible to any zone
 * count, visible only to a whole-board census. Refusing up front keeps the card
 * in the deck, which is the state Rust's caller would have to reconcile too.
 * (The capacity gap itself is pinned red by test_audit_zones.c.) */
int rb_player_draw_card(GameState *g, int pl){
    if (!g || pl<0 || pl>1) return -1;
    RbPlayer *P = &g->p[pl];
    if (P->hand.n >= RB_MAX_HAND) return -1;
    int cid = rb_zone_draw(g, pl);
    if(cid < 0) return -1;
    rb_hand_add(P, cid);
    return cid;
}

/* Mirror Player::draw_energy (core/player.rs:585-591):
 *
 *     pub fn draw_energy(&mut self) -> Option<i16> {
 *         self.energy_deck.draw().inspect(|&card_id| {
 *             // Insert at the active boundary (not append): the drawn card is
 *             // active, and appending past waited cards would corrupt the
 *             // active-prefix convention.
 *             self.energy_zone.push_active(card_id);
 *         })
 *     }
 *
 * and EnergyZone::push_active (core/zones.rs:718-722):
 *
 *     let at = (self.active_energy_count as usize).min(self.cards.len());
 *     self.cards.insert(at, card_id);
 *     self.active_energy_count = self.active_energy_count.saturating_add(1);
 *
 * WAS DEAD-WRONG ON THREE COUNTS, and this is the class that matters: a dead
 * twin whose body diverges fires the moment somebody wires it.
 *   1. POSITION. The old body appended at `energy.n`. With waited energy in
 *      the zone (`energy_active < energy.n`) that lands the freshly drawn
 *      card AFTER a waited card, so it becomes waited — precisely what Rust's
 *      own comment above forbids. Now inserted at
 *      `min(energy_active, energy.n)`.
 *   2. CAPACITY. The old body drew from the energy deck FIRST and only then
 *      tested `energy.n < RB_MAX_ZONE`; on a full zone the card was already
 *      out of the deck and was then DROPPED ON THE FLOOR. Rust's `Vec::insert`
 *      always succeeds. The capacity test now precedes the draw, so the card
 *      stays in the energy deck instead of ceasing to exist.
 *   3. SATURATION. `saturating_add` is on a `u8`; the C's `energy_active` is
 *      `int` (include/rabuka.h:800) and would run past 255. Clamped.
 *
 * The capacity branch is unreachable on the real corpus (an energy zone never
 * approaches RB_MAX_ZONE == 512), which is exactly why it is stated here
 * rather than left to chance.
 */
int rb_player_draw_energy(GameState *g, int pl){
    if(!g || pl<0 || pl>1) return -1;
    RbPlayer *P = &g->p[pl];
    if(P->energy.n >= RB_MAX_ZONE) return -1;
    int cid = rb_energy_deck_draw(g, pl);
    if(cid < 0) return -1;
    {
        int at = P->energy_active;
        if(at < 0) at = 0;
        if(at > P->energy.n) at = P->energy.n;
        for(int i = P->energy.n; i > at; i--) P->energy.cards[i] = P->energy.cards[i-1];
        P->energy.cards[at] = cid;
        P->energy.n++;
        P->energy_active++;
        if(P->energy_active > 255) P->energy_active = 255;
    }
    return cid;
}

/* Mirror Player::contains_card — returns 1 if the given card ID exists in
 * any of this player's zones. */
int rb_player_contains_card(const GameState *g, int pl, int cid){
    if(!g || pl<0 || pl>1) return 0;
    const RbPlayer *P = &g->p[pl];
    for(int i=0;i<P->deck.n;i++) if(P->deck.cards[i]==cid) return 1;
    for(int i=0;i<P->hand.n;i++) if(P->hand.cards[i]==cid) return 1;
    for(int i=0;i<P->energy.n;i++) if(P->energy.cards[i]==cid) return 1;
    for(int i=0;i<P->energy_deck.n;i++) if(P->energy_deck.cards[i]==cid) return 1;
    for(int i=0;i<P->discard.n;i++) if(P->discard.cards[i]==cid) return 1;
    for(int i=0;i<P->live.n;i++) if(P->live.cards[i]==cid) return 1;
    for(int i=0;i<P->success.n;i++) if(P->success.cards[i]==cid) return 1;
    for(int s=0;s<RB_STAGE_SIZE;s++) if(P->stage[s]==cid) return 1;
    for(int s=0;s<RB_STAGE_SIZE;s++){
        const RbBag *uc = &P->under_cards[s];
        for(int i=0;i<uc->n;i++) if(uc->cards[i]==cid) return 1;
    }
    return 0;
}

/* Mirror Player::track_deployment — marks a card as deployed this turn so
 * its area cannot be targeted for baton touch. The C engine tracks this via
 * stage_arrived[pl][area]. */
int rb_zone_track_deployment(GameState *g, int pl, int card_id){
    if(!g || pl<0 || pl>1) return 0;
    RbPlayer *P = &g->p[pl];
    for(int s=0;s<RB_STAGE_SIZE;s++){
        if(P->stage[s]==card_id){
            g->stage_arrived[pl][s] = 1;
            return 1;
        }
    }
    return 0;
}
