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
static int  bag_remove_at(RbBag *b, int i) {
    if (i < 0 || i >= b->n) return -1;
    int c = b->cards[i];
    for (int k = i; k < b->n - 1; k++) b->cards[k] = b->cards[k + 1];
    b->n--; return c;
}

void rb_shuffle(int *a, int n) {
    for (int i = n - 1; i > 0; i--) {
        int j = rng_range(i + 1);
        int t = a[i]; a[i] = a[j]; a[j] = t;
    }
}

/* map a wire zone name to an enum; returns 1 on success.
 *
 * RbZone (include/rabuka.h) has NO top/bottom distinction on the deck and NO
 * energy-deck value at all, so this function CANNOT name two spellings the
 * corpus actually uses:
 *
 *   deck_bottom   util.rs:2433-2436 `Zone::DeckBottom` -> main_deck.cards.push
 *   energy_deck   util.rs:2441-2444 `Zone::EnergyDeck`  -> energy_deck.cards.push
 *
 * Both used to be folded onto a representable value — deck_bottom onto
 * RB_ZONE_DECK, which silently claims "top" for a bottom insert — and that is
 * what draw.c:66-77's comment is about. They now return 0 (unresolvable by
 * this enum) rather than lie, and the two callers that genuinely must tell
 * them apart own a dedicated string-keyed arm each, which is where Rust's own
 * dispatch point lives:
 *
 *   util.c  rb_place_card_in_zone  (util.rs:2394)  — dedicated deck / deck_top /
 *                                            deck_bottom / energy_deck arms
 *   draw.c  draw_place_in_zone_g   (util.rs:2330)  — same, before this is called
 *
 * Rust draws the same distinction from its richer Zone enum (util.rs:2426-2444
 * arms on Deck | DeckTop | DeckBottom | Energy | EnergyDeck separately), so this
 * is a faithful restriction of the same rule set to a narrower enum, not a
 * behaviour change. Rust's remove_card_from_zone agrees that these spellings are
 * NOT the main deck (util.c's copy of that rule is quoted in place above).
 *
 * energy_deck is deliberately NOT mapped to RB_ZONE_ENERGY: the energy deck and
 * the energy zone are different bags, and mapping one onto the other would turn
 * a correct "unresolvable" into a wrong placement. Both corpus spellings are
 * therefore handled by their real handler before this function is consulted.
 *
 * ── The shared vocabulary now comes from ONE table ────────────────────────────
 * This used to be a second, hand-rolled `if` chain listing the spellings. Two
 * wire tables for one enum is how `deck_bottom` ended up silently folded onto
 * RB_ZONE_DECK in one of them and honestly refused in the other. The shared
 * rows are now delegated to rb_zone_id_from_str (data.c:925), which is the
 * faithful port of Rust's `Zone::from_str` (engine/src/ability/enums.rs:89-122)
 * and is the ONLY table in the tree that knows RB_ZONEID_DECK_BOTTOM and
 * RB_ZONEID_ENERGY_DECK. So the two cannot drift again: if a spelling is added
 * to Rust's Zone, adding it to data.c's table adds it here.
 *
 * The narrowing RbZoneId -> RbZone is DELIBERATE and is spelled out below
 * rather than inherited from modifiers.c's rb_zone_from_zone_id
 * (modifiers.c:400-420), because THAT one folds
 *
 *     RB_ZONEID_DECK_BOTTOM -> RB_ZONE_DECK      (claims "top" for a bottom insert)
 *     RB_ZONEID_ENERGY_DECK -> RB_ZONE_ENERGY    (moves cards between two bags)
 *
 * and both of those are wrong placements. Here they fall through to the honest
 * `return 0` instead. `RB_ZONEID_ENERGY_DECK` IS the dedicated energy-deck
 * value the deck-vs-zone distinction needs; it simply has no RbZone partner,
 * and the three callers that must tell the two apart own a dedicated
 * string-keyed arm each, which is where Rust's own dispatch point lives:
 *
 *   util.c  rb_place_card_in_zone  (util.rs:2394)  — deck / deck_top /
 *                                            deck_bottom / energy_deck arms
 *   draw.c  draw_place_in_zone_g   (util.rs:2330)  — same, before this is called
 *   util.c  zone_bag / rb_zone_cards (util.rs:2073) — deck_top/deck_bottom fold
 *                                            onto main_deck there, and
 *                                            energy_deck is `_ => &[]`
 *
 * XREQ for util.c's owner: the three dedicated string arms above are the same
 * rule set written out three times, and util.c:1022 / util.c:898 / util.c:1160
 * are where they can collapse onto rb_zone_id_from_str. util.c and draw.c are
 * not this file's, so nothing is re-pointed here.
 *
 * NOT DERIVED from rb_zone_id_from_str, kept as a C-only tail below: "live",
 * "success", "success_zone", "center", "left" and "right" are spellings the C
 * wire vocabulary carries that Rust's `Zone::from_str` does not list. They are
 * resolvable here today and dropping them would silently re-route those
 * destinations to the caller's hand fallback (draw.c:92-95). Keeping them is
 * the pre-existing behaviour and is NOT a Rust claim.
 *
 * RESIDUE (2026-10-03, checklist I7): the behaviour above is correct and both
 * arms are in place; three COMMENTS in files this task does not own still
 * describe the OLD, lying table and now contradict it:
 *   src/ability/effects/draw.c:65-74  "rb_zone_of_str (engine.c:60) collapses
 *     deck_bottom onto RB_ZONE_DECK and has no energy_deck arm at all"
 *   src/ability/effects/draw.c:66     "engine.c:60" (the table moved to :126)
 *   src/core/modifiers.c:610          "rb_zone_of_str must NOT be used here"
 * The CODE in draw.c is right and needs no change — the dedicated string arms at
 * draw.c:77 and :84 are still load-bearing, and now for the OPPOSITE reason:
 * with rb_zone_of_str returning 0 for both spellings, reaching rb_zone_of_str at
 * all would send them to the hand catch-all. Only the prose is stale.
 *
 * RESIDUE (rb_turn): confirmed a thin headless driver over rb_advance_phase with
 * no second turn machine left. What remains, all deliberate and all documented
 * at :1475-1494: the 256-step guard, the `-1` skip answer drain, and the fact
 * that `rb_game_init` now stops at RB_PHASE_RPS (so the CLI batch mode plays an
 * empty game until a host policy exists over rb_generate_action_candidates).
 * Nothing here reintroduces turn state. */
int rb_zone_of_str(const char *s, RbZone *out) {
    if (!s || !out) return 0;
    /* Shared rows — see the table note above. */
    switch (rb_zone_id_from_str(s)) {
        case RB_ZONEID_HAND:               *out = RB_ZONE_HAND; return 1;
        case RB_ZONEID_DECK:               *out = RB_ZONE_DECK; return 1;
        /* util.rs:2426-2436 — DeckTop inserts at `idx` (default 0 = top) and
           DeckBottom pushes at the end. RB_ZONE_DECK is the caller's top insert
           (draw.c:101-108), which is DeckTop's rule, so folding DeckTop onto it
           is exact. DeckBottom has NO RbZone partner and is refused below. */
        case RB_ZONEID_DECK_TOP:           *out = RB_ZONE_DECK; return 1;
        case RB_ZONEID_DECK_BOTTOM:        return 0;
        case RB_ZONEID_ENERGY_DECK:        return 0;
        case RB_ZONEID_STAGE:
        case RB_ZONEID_UNDER_MEMBER:
        case RB_ZONEID_SAME_AREA:
        case RB_ZONEID_EMPTY_AREA:         *out = RB_ZONE_STAGE; return 1;
        case RB_ZONEID_DISCARD:
        case RB_ZONEID_WAITROOM:           *out = RB_ZONE_DISCARD; return 1;
        case RB_ZONEID_ENERGY:
        case RB_ZONEID_ENERGY_ZONE:        *out = RB_ZONE_ENERGY; return 1;
        case RB_ZONEID_LIVE_CARD_ZONE:     *out = RB_ZONE_LIVE; return 1;
        case RB_ZONEID_SUCCESS_ZONE:
        case RB_ZONEID_SUCCESS_LIVE_ZONE:  *out = RB_ZONE_SUCCESS; return 1;
        case RB_ZONEID_RESOLUTION:         *out = RB_ZONE_RESOLUTION; return 1;
        default: break;
    }
    /* C-only tail — not Rust `Zone::from_str` spellings, pre-existing here. */
    if (!strcmp(s, "center") || !strcmp(s, "left") || !strcmp(s, "right"))
        *out = RB_ZONE_STAGE;
    else if (!strcmp(s, "live"))
        *out = RB_ZONE_LIVE;
    else if (!strcmp(s, "success"))
        *out = RB_ZONE_SUCCESS;
    else
        return 0;
    return 1;
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
   members and had no energy branch at all.

   Only the MEMBER test survives here. card_is_live / card_is_energy were read
   solely by the deleted live_phase / main_phase pair, and their exported twins
   rb_card_is_live / rb_card_is_energy (src/core/card.c:38/42) apply the same
   type_flags encoding to a card_id for every other caller in the tree, so there
   is one implementation of each test outside this file. */
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

/* ───────────────────────────── effect execution ───────────────────────────── */
/* rb_emit_choice is declared in include/rabuka.h; the local prototype here only
   documents that this file is a producer. rb_effect_deep_clone's local
   prototype went with handle_action, which was its only user here. */
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
    /* Park guard (see the resume_parent block below). The continuation parked by
        a DEEPER frame of this same effect tree must survive the return path here,
        so remember what resume_parent held when this frame was entered: if a
        descendant changed it, this frame is NOT the innermost one and must not
        claim the park. Comparing against the entry value (rather than testing
        `resume_parent != NULL`) is what makes this safe: rb_clear_pending_choice
        does not clear resume_parent, so a stale pointer from an ALREADY-ANSWERED
        prompt would otherwise suppress a legitimate park forever. */
    const AbilityEffect *entry_park = g->queue.resume_parent;
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
                    /* Mirror Rust pending_actions: the choice's remaining body
                        (the sibling effects that follow it in the parent, e.g. the
                        gain_resource that follows a heart-color select) runs AFTER
                        the player answers. Park the parent + the choice's child
                        index so resume executes the later siblings — but only if
                        no DEEPER frame already parked. Without that guard the
                        OUTER loop re-entered here after the grandchild's prompt
                        was raised, saw rb_has_pending_choice, and overwrote the
                        grandchild's (parent=e_inner, child=i_inner) with its own
                        (parent=e_outer, child=i_outer) — so answering the prompt
                        resumed the OUTER frame's remaining siblings and the inner
                        frame's own remaining siblings were orphaned and never ran.
                        Rust keeps a per-resolution pending_actions LIST
                        (ability_queue.rs:521), so the innermost frame's
                        continuation is the one that survives; this single-slot
                        queue must therefore let the first (innermost) writer win.
                        The `is_gate` arm had the same overwrite hazard, so both
                        arms share the one guard. */
                    if ((is_choice || is_gate) && g->queue.resume_parent == entry_park) {
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
    /* THE single effect-dispatch seam. executor_table (src/ability/effects/
       executor.c:16) is the C twin of Rust's exhaustive `match action` in
       engine/src/ability/effects/dispatch.rs, and rb_executor_execute is that
       match. This file used to carry a SECOND, parallel if/else-if chain —
       handle_action — that routed the same action strings to the same handler
       functions (rb_effect_draw_card, rb_effect_move_cards,
       rb_effect_change_state, rb_execute_misc_effect, rb_compound_*, ...),
       38 of whose 83 action literals name a string the corpus never
       produces. It is gone; the table below is the only routing.

       Why that is not a behaviour change, measured two ways:
         * STATIC. A census of every `action` the decoder can produce from
           cards/abilities.json — 47 distinct effect actions, plus the 7 cost
           `type` values — finds every one of them present in executor_table.
           Zero fall through.
         * DYNAMIC. Across the twelve gating suites the ONLY
           [EXECUTOR_MISS] ever printed is action=totally_unknown_action, a
           synthetic string built by tests/test_parity_executor_dispatch.c:1082
           and handed straight to rb_executor_execute, never through this
           function. No suite, no corpus card, and no src/ caller reached the
           else arm.

       The remaining arm is Rust's own outcome for a string no ActionType
       decoded to: "no state change" (dispatch.rs's match is exhaustive, so this
       is unreachable there; executor.c's final else already reproduces the
       outcome for a direct rb_executor_execute call). */
    if (rb_executor_has_executor(e->action))
        rb_executor_execute(g, actor, e, host_cid);
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
    /* Mirror phases.rs:1065-1066 (the double-baton completion arm of
        handle_play_member_to_stage):

            game_state.active_player_mut().debut_count_this_turn += 1;
            game_state.record_card_appearance(card_id, "hand");

        This is the SECOND of the two play-path record_card_appearance call
        sites, and it is a distinct placement from the one in rb_play_member (a
        double baton vacates two areas and the arriving card is placed into the
        vacated one), so it needs its own record. Same source token, "hand".

        (Rust's `debut_count_this_turn += 1` on the adjacent line is NOT
        reproduced here: this function never bumped the counter, and adding it is
        a separate defect outside the scope of this fix. Reported, not changed.) */
    rb_record_card_appearance(g, card, RB_ZONE_HAND);
    rb_fire_debut(g, actor, card);
    rb_fire_auto(g, actor);
    rb_drain_ability_queue(g);
    return 1;
}

/* ═══════════════ explicit PlayMemberToStage (phases.rs:1132-1250) ═══════════════
 *
 * Rust's handle_play_member_to_stage takes FOUR request parameters the C's
 * `rb_play_member(g, pl, hand_idx, stage_pos)` cannot name, and three of them
 * change the result:
 *
 *   card_indices       Vec<usize> — the TWO areas an explicit double baton
 *                     replaces (phases.rs:1182-1192). `stage_area` then picks
 *                     which of the two vacated areas the arrival lands in
 *                     (:1196, :1227-1231).
 *   use_baton_touch    Option<bool> — an EXPLICIT demand, read at
 *                     core/player.rs:394-395:
 *                         should_use_baton_touch =
 *                             use_baton_touch || self.stage.get_area(stage_area).is_some();
 *                     It can only ADD a demand, never suppress one: an occupied
 *                     area still batons whether or not the flag is set, and a
 *                     set flag onto an EMPTY area is an Err
 *                     (player.rs:431-437 — the card goes back in hand, the play
 *                     is refused). The C used to evaluate only the right-hand
 *                     side of that `||`, so the whole left-hand side was
 *                     inexpressible and a flag set onto an empty area silently
 *                     became a normal play.
 *   stage_area         Option<MemberArea> — Rust's None default is "first empty
 *                     area, else ALL[0]" (:1198-1208).
 *   card_id            Option<i16> — Rust's None default is "first member in
 *                     hand" (:1170-1177).
 *
 * The two None defaults are resolved by rb_handle_play_member_to_stage
 * (src/turn/phase.c, the file that owns the Rust function); this file owns the
 * three requests they feed.
 *
 * One latch, because Rust has a STACK FRAME and the C does not: the play-time
 * cost pause (rb_detect_alt_cost, engine/src/turn/phases.rs:1219-1222) returns
 * out of handle_play_member_to_stage and Rust re-enters it with the SAME
 * arguments (turn/actions/mod.rs:976-982 re-calls handle_play_member_to_stage
 * with Some(play.card_id) / None / Some(play.area) / Some(false)). The C's
 * equivalent re-entry is rb_complete_play_with_cost, which used to call
 * rb_play_member unconditionally — that silently dropped BOTH the explicit
 * baton flag and the explicit double-baton pair. The request is therefore
 * latched here and re-dispatched from there. */
typedef struct {
    int active;              /* 1 while a paused explicit play is outstanding */
    int hand;                /* hand index of the card being played */
    int area;                /* landing area (== pair[0] for a plain play) */
    int pair[2];             /* the two areas to replace; pair[1] < 0 = single */
    int use_baton_touch;
} RbExplicitPlay;
static RbExplicitPlay s_explicit;

int rb_play_member_ex(GameState *g, int pl, int hand_idx, int stage_pos, int use_baton_touch);
int rb_play_double_baton_ex(GameState *g, int pl, int hand_idx, int a1, int a2, int place);

/* Clear the latch once its play has completed or been refused, so a later
 * rb_play_member cannot inherit a dead request. */
static void explicit_play_clear(void) { memset(&s_explicit, 0, sizeof(s_explicit)); }

int rb_play_member(GameState *g, int pl, int hand_idx, int stage_pos) {
    /* Rust's caller for an ordinary Main-phase play passes
     * use_baton_touch: None (actions/mod.rs:312-318 forwards the action's own
     * Option), and None unwraps to false at phases.rs:1139. */
    return rb_play_member_ex(g, pl, hand_idx, stage_pos, 0);
}

int rb_play_member_ex(GameState *g, int pl, int hand_idx, int stage_pos, int use_baton_touch) {
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
    /* ── core/player.rs:394-437 — the ONE line the C was missing ──────────────
         let should_use_baton_touch =
 *             use_baton_touch || self.stage.get_area(stage_area).is_some();
       `is_some()` is the C's `P->stage[stage_pos] >= 0`; `use_baton_touch` is
       the explicit request. The C evaluated only the right-hand side, so an
       explicit flag could not demand a baton and could not be declined — a
       flag set onto an EMPTY area silently played as an ordinary placement.
       Both halves are here now. */
    int should_use_baton = use_baton_touch || (P->stage[stage_pos] >= 0);
    if (should_use_baton && P->stage[stage_pos] < 0) {
        /* player.rs:431-437 — `should_use_baton_touch` is true but the target
         * area holds no member: Rust re-inserts the card it removed at :360 and
         * returns Err("Cannot baton touch - no member in target area"). The C
         * has not removed the card yet, so the observable result is the same —
         * hand unchanged, play refused. */
        g->play_depth--;
        return 0;
    }
    int is_baton = should_use_baton;   /* player.rs:397-440 */
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
            /* Latch the EXPLICIT part of the request, which ptc_card/ptc_hand/
             * ptc_area do not carry. Without this the resume below re-enters as
             * a plain rb_play_member and the explicit baton flag is dropped
             * (phases.rs:1219-1222 returns out of the function and
             * actions/mod.rs:976-982 re-enters it WITH the flag). */
            s_explicit.active = 1;
            s_explicit.hand = hand_idx;
            s_explicit.area = stage_pos;
            s_explicit.pair[0] = stage_pos;
            s_explicit.pair[1] = -1;
            s_explicit.use_baton_touch = use_baton_touch;
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
    /* Mirror phases.rs:1293-1294 (handle_play_member_to_stage, "Phase 2: record
        appearance and debut triggers"):

            game_state.active_player_mut().debut_count_this_turn += 1;
            game_state.record_card_appearance(card_id, "hand");

        rb_record_card_appearance is the ONLY writer of
        cards_appeared_this_turn / recently_appeared / card_appearance_source
        (modifiers.rs:1401-1412) and until this call it had ZERO callers, so
        every reader of the appearance list saw it permanently empty:
        condition.c:838/928/938/989, modifiers.c:327/399, score.c:733,
        misc.c:229 — i.e. the whole 「このターン中に登場した場合」 family could
        never fire. The source token is the literal "hand" Rust passes; the C
        recorder stores the RbZone the caller already has, so RB_ZONE_HAND is
        the same value, and rb_appearance_source_matches resolves it back
        through the same wire spellings.

        Idempotence: rb_record_card_appearance already dedupes BOTH list
        structures (modifiers.c:326-339 mirrors the two `contains` guards), and
        card_appearance_source is pushed UNCONDITIONALLY exactly as Rust does
        (the reader takes the FIRST match), so a card that debuts twice records
        one list entry and two source entries — which is what Rust produces. */
    rb_record_card_appearance(g, card, RB_ZONE_HAND);
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
    explicit_play_clear();
    return 1;
}

/* ═══════════════ play_double_baton (phases.rs:936-1087) ═══════════════
 *
 * The EXPLICIT half of the double baton, i.e. the one Rust reaches when the host
 * names the two areas in `card_indices`. rb_complete_double_baton above is the
 * C's stand-in for the half Rust never had to provide: it AUTO-DETECTS that the
 * card carries a play_baton_touch with count > 1, offers every pair as a
 * SelectTarget (engine.c:653-675) and finishes here, so the host is shown the
 * pairs but never chooses one. Rust has no such auto-detect: the UI emits the
 * pair and phases.rs:1182-1192 reads it back.
 *
 * `card_indices` is a pair of AREA INDICES, and MemberArea::ALL is
 * [LeftSide, Center, RightSide] (core/zones.rs:68-72) — the same order as the C's
 * RB_STAGE_SIZE 0..2 slots, so the indices are usable directly.
 *
 * `place` is `stage_area.unwrap_or(db_areas[0])` (phases.rs:1196): which of the
 * two vacated areas the arrival lands in. */
int rb_play_double_baton_ex(GameState *g, int pl, int hand_idx, int a1, int a2, int place) {
    if (!g) return 0;
    if (hand_idx < 0 || hand_idx >= g->p[pl].hand.n) return 0;
    if (a1 < 0 || a1 >= RB_STAGE_SIZE || a2 < 0 || a2 >= RB_STAGE_SIZE || a1 == a2) return 0;
    /* phases.rs:1196 — stage_area.unwrap_or(db_areas[0]) */
    if (place != a1 && place != a2) place = a1;
    RbPlayer *P = &g->p[pl];
    int pair[2];
    pair[0] = a1;
    pair[1] = a2;
    if (g->play_depth > 4) return 0;
    g->play_depth++;
    int cid = P->hand.cards[hand_idx];

    /* phases.rs:948 card_cost — base cost, no play-time reduction yet. */
    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) { g->play_depth--; return 0; }
    int card_cost = (int)c.cost;

    /* phases.rs:949-970 — the COMBINED reduction is each replaced member's
     * floored cost, floored_cost = saturate(base + mod).max(1)
     * (core/constants.rs:116), so a net-zero modifier prices at 1, never 0. */
    int combined = 0;
    for (int k = 0; k < 2; k++) {
        int existing = P->stage[pair[k]];
        if (existing == RB_EMPTY_SLOT) continue;
        Card ec;
        int base = 0;
        if (rb_decode_card_by_index((uint32_t)existing, &ec)) {
            base = (int)ec.cost;
            rb_free_card(&ec);
        }
        int floored = base + rb_mods_get_cost(&g->mods, existing);
        if (floored < 0) floored = 0;
        if (floored < 1) floored = 1;   /* floored_cost's .max(1) */
        combined += floored;
    }
    /* phases.rs:983-993 — play-time reduction, then the combined baton credit. */
    int reduction = rb_calculate_play_cost_reduction(g, pl, P->hand.n, cid);
    int final_cost = card_cost - reduction;
    if (final_cost < 0) final_cost = 0;
    final_cost -= combined;
    if (final_cost < 0) final_cost = 0;
    /* phases.rs:994-1000 */
    if (final_cost > 0) {
        if (P->energy_active < final_cost) { rb_free_card(&c); g->play_depth--; return 0; }
        P->energy_active -= final_cost;
    }
    /* phases.rs:1001-1024 — cannot_baton_touch protection on BOTH named areas,
     * checked AFTER the payment (Rust's ordering; the Err leaves the paid
     * energy spent, which is what the C now does too). */
    for (int k = 0; k < 2; k++) {
        int existing = P->stage[pair[k]];
        if (existing == RB_EMPTY_SLOT) continue;
        if (rb_card_has_restriction(g, cid, existing, "cannot_baton_touch")) {
            rb_free_card(&c); g->play_depth--; return 0;
        }
    }
    /* phases.rs:1025-1037 — replace BOTH named members, in `db_areas` order, so
     * the LAST one to land is the last card in the waitroom. */
    int replaced[2];
    int n_replaced = 0;
    for (int k = 0; k < 2; k++) {
        int existing = P->stage[pair[k]];
        if (existing == RB_EMPTY_SLOT) continue;
        for (int u = 0; u < P->under_cards[pair[k]].n; u++)
            rb_send_to_waitroom(g, pl, P->under_cards[pair[k]].cards[u]);
        P->under_cards[pair[k]].n = 0;
        rb_send_to_waitroom(g, pl, existing);
        P->stage[pair[k]] = RB_EMPTY_SLOT;
        g->stage_arrived[pl][pair[k]] = 0;
        replaced[n_replaced++] = existing;
    }
    /* phases.rs:1049-1055 — last_vacated_stage_area is the OTHER (non-placement)
     * vacated area, which is what the empty_area / deployment effects read. */
    g->baton_last_vacated_area[pl] = (pair[0] != place) ? pair[0] : pair[1];
    /* phases.rs:1056-1063 */
    for (int i = hand_idx; i + 1 < P->hand.n; i++) P->hand.cards[i] = P->hand.cards[i + 1];
    P->hand.n--;
    P->stage[place] = cid;
    P->stage_wait[place] = 0;
    g->stage_arrived[pl][place] = 1;
    /* phases.rs:1064-1067 — TWO baton touches, both naming the arriving card. */
    rb_record_baton_touch(g, pl, cid);
    rb_record_baton_touch(g, pl, cid);
    /* phases.rs:1068-1071 — the replaced identity is the LAST waitroom card. */
    g->baton_touch_replaced_member_id = n_replaced > 0 ? replaced[n_replaced - 1] : -1;
    g->baton_touch_replaced_member_cost = combined;
    g->baton_touch_zero_cost = final_cost == 0;
    /* phases.rs:1072-1074 */
    g->debut_count_this_turn[pl]++;
    rb_record_card_appearance(g, cid, RB_ZONE_HAND);
    g->baton_touch_arriving_card_id = cid;
    g->baton_touch_used[pl] = 1;
    g->current_is_baton = 1;
    /* phases.rs:1076-1080 — trigger_debut_abilities(..., baton_touch_used = true, ..)
     * is exactly the state published above for rb_trigger_debut to read
     * (src/turn/triggers.c:223), then both auto sweeps and the pending drain. */
    g->recently_moved[0] = n_replaced > 0 ? replaced[0] : cid;
    g->n_recently_moved = 1;
    rb_fire_debut(g, pl, cid);
    rb_fire_auto(g, pl);
    rb_fire_auto(g, 1 - pl);
    rb_drain_ability_queue(g);
    /* phases.rs:1082 */
    rb_recalc_constants(g);
    g->current_is_baton = 0;
    rb_free_card(&c);
    g->play_depth--;
    explicit_play_clear();
    return 1;
}

/* Activation-tree ownership (DEFECT 4).
 *
 * g->activation_act + g->activation_keepalive + g->activation_keepalive_valid
 * are a SINGLE slot carrying one activation's AbilityEffect wrapper tree plus
 * the decoded Ability that owns its cost/effect subtrees, kept alive across the
 * choice round-trip. choice.c:4696-4703 releases the pair when the prompt
 * closes, and the inline teardown at the bottom of rb_activate_card releases it
 * when no prompt is left. g->activation_keepalive is a shallow copy of an owning
 * type, so the pair must have exactly one owner at all times.
 *
 * rb_activate_card is RE-ENTRANT: an ability effect can activate another card's
 * ability mid-resolution, and there are two entry points (rb_activate_ability
 * below and rb_execute_main_phase_action further down). The old code published
 * into the slot unconditionally, so a nested activation CLOBBERED the outer
 * pair with no free of the prior value: the outer tree leaked in full and
 * choice.c then freed only the newest one, so the outer AbilityEffect tree plus
 * its cost/effect subtrees were never reclaimed. Repeated on every nested
 * activation across a game, that is unbounded growth.
 *
 * The guard is a save/restore of the slot, NOT a free of the outer tree:
 *   - on entry, save whatever the caller had published;
 *   - publish this activation's pair;
 *   - on exit with NO prompt open, release THIS pair and restore the saved slot,
 *     so the outer frame's own teardown still finds its tree and frees it;
 *   - on exit WITH a prompt open, leave THIS pair published. The queue holds raw
 *     pointers into it (g->queue.pending / resume_parent / deferred), so it must
 *     survive and choice.c owns its release. The saved (outer) pair is
 *     deliberately NOT restored in that case: restoring it would make choice.c's
 *     tail free the outer tree while the prompt is still walking the inner one.
 *     That residual is bounded by the nesting depth of ONE activation; it is the
 *     same "known bounded leak" shape ability_queue.c:1286-1291 documents, and
 *     strictly smaller than the pre-guard behaviour, which leaked on every
 *     nested activation whether or not a prompt was involved.
 *
 * The outer's tree is never freed from the inner frame: the outer effect VM is
 * still walking it, so freeing it here would be a use-after-free.
 *
 * OWNERSHIP RULE UNCHANGED: the pair still has exactly one owner, and
 * ability_queue.c's rb_drain_keep_alive keeps its "if the slot is taken, leave
 * the tree detached" contract intact — it tests activation_keepalive_valid
 * before publishing, and this guard only ever adds a restore around that. */
static int rb_activate_act_depth = 0;

/* ONE accepted activation: publish activating_card, wrap cost+effect as a single
   resolution tree, run it, tear the keepalive down (or hand it to choice.c when
   a prompt is still open), and spend the use. `ability_index` is the index the
   resolution is keyed on — a printed slot number, or
   RB_GAINED_ABILITY_INDEX_BASE + gained_idx for a gained activation, which is
   the same encoding every other (card_id, ability_index) pair in the tree uses
   (rabuka.h:2816-2835). Both callers below go through here, so a gained
   activation cannot drift from a printed one.

   `ab` is NOT consumed: the keepalive is a by-value copy of the struct, and on
   the prompt-open path choice.c owns the release of that copy. The caller keeps
   whatever it allocated. */
static void activate_resolved_ability(GameState *g, int pl, int card_id,
                                      int ability_index, const Ability *ab) {
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
    if (ab->cost)   act->child[act->n_child++] = ab->cost;
    if (ab->effect) act->child[act->n_child++] = ab->effect;
    /* DEFECT 4 re-entrancy guard — see the ownership note above. */
    AbilityEffect *saved_act_slot = g->activation_act;
    Ability saved_keepalive = g->activation_keepalive;
    int saved_keepalive_valid = g->activation_keepalive_valid;
    rb_activate_act_depth++;
    g->activation_act = act;
    g->activation_keepalive = *ab;
    g->activation_keepalive_valid = 1;
    g->n_recently_moved = 0;
    rb_execute_effect_ex(g, pl, act, card_id);
    rb_drain_ability_queue(g);
    if (!g->queue.has_pending) {
        /* DEFECT 4 — teardown. The old body was:
             act->n_child = 0;
             g->activation_keepalive.cost = NULL;
             g->activation_keepalive.effect = NULL;
             rb_free_ability(&g->activation_keepalive);
             free(g->activation_act);
         Nulling cost/effect before rb_free_ability means rb_free_ability
         (vm.c:1180-1185, which is what actually walks
         effect_free(a->cost) / effect_free(a->effect)) never sees them:
         BOTH AbilityEffect subtrees leaked on every activation that did
         NOT leave a prompt open — the majority path, so this alone was
         unbounded growth over a game. choice.c:4696-4703 does NOT null
         them, which is why the interactive path reclaimed them and made
         the inline path look correct by comparison.

         act is a plain wrapper that is released with free(), so it never
         owned its children; ab (the keepalive) owns them. Clearing
         n_child and letting rb_free_ability walk the keepalive is the
         total, single-owner release: each of cost, effect and act is
         freed exactly once. */
        act->n_child = 0;
        rb_free_ability(&g->activation_keepalive);
        g->activation_keepalive_valid = 0;
        free(act);
        g->activation_act = NULL;
        /* Restore the slot the caller had published so an enclosing
           activation's teardown still finds its own tree. */
        g->activation_act = saved_act_slot;
        g->activation_keepalive = saved_keepalive;
        g->activation_keepalive_valid = saved_keepalive_valid;
    }
    /* Prompt still open: THIS pair stays published and choice.c owns its
       release; the saved (outer) pair is NOT restored, or choice.c's tail
       would free the outer tree while the prompt walks the inner one.
       See the ownership note above. */
    rb_activate_act_depth--;
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
    int declined = ab->cost && rb_cost_optional_skipped() && !pre_skipped;
    if (ab->use_limit > 0 && !declined)
        rb_record_use(&g->queue, card_id, ability_index, g->turn);
    g->activating_card = saved_act;
}

/* turn/actions/mod.rs:430-454 find_gained_activation.
 *
 * Rust falls back to a RUNTIME-GAINED 起動 when no printed 起動 on the card is
 * an activation candidate (actions/mod.rs:544-547). The gained list lives in
 * `game_state.gained_card_abilities` — the C twin is the gained store behind
 * rb_card_num_gained_abilities / rb_resolve_card_ability_at
 * (src/core/game_state_abilities.c:322-345), addressed as
 * RB_GAINED_ABILITY_INDEX_BASE + gained_idx.
 *
 * Two rules are ported verbatim and both matter:
 *   - the card must be ON STAGE (mod.rs:445-446 `player.stage.stage.contains`),
 *     so a gained ability cannot be activated from hand or discard;
 *   - the FIRST gained 起動 wins (mod.rs:434-440 `iter().find(...)`), and the
 *     returned index is the ENCODED one, which is what the use-limit key and
 *     the activation queue id are keyed on (mod.rs:451 activation_ability_id).
 *
 * What Rust deliberately does NOT do here, and this port therefore does not
 * either: find_gained_activation applies no use-limit gate and no
 * mandatory-cost payability check. Those run AFTER the AbilityActivation is
 * chosen (mod.rs:549-566), which is where activate_resolved_ability's
 * rb_record_use sits.
 *
 * Returns the gained slot number, or -1 for "no gained activation". */
static int find_gained_activation_index(const GameState *g, int pl, int card_id) {
    if (!g || pl < 0 || pl > 1 || card_id < 0) return -1;
    int on_stage = 0;
    for (int q = 0; q < RB_STAGE_SIZE; q++)
        if (g->p[pl].stage[q] == card_id) { on_stage = 1; break; }
    if (!on_stage) return -1;
    int n = rb_card_num_gained_abilities(g, card_id);
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_resolve_card_ability_at(g, card_id,
                                        RB_GAINED_ABILITY_INDEX_BASE + i, &ab))
            continue;
        int is_activate = ab.triggers && strstr(ab.triggers, "起動");
        rb_free_ability(&ab);   /* the deep copy is ours; this probe does not keep it */
        if (is_activate) return i;
    }
    return -1;
}

/* Activate a card's ability by card id. Mirrors Rust's `GameState::activate_ability`:
    the card may expose several abilities (RAKA_CARD_ABILITY_PAIRS); the manual
    "activate" action runs the one(s) whose trigger is 起動 (Activate). Apply each
    matched ability's cost BEFORE its effect. Falls back to the card's single
    default `ability_idx` when no 起動 ability exists (e.g. debut-only members),
    and — since this port — to a runtime-GAINED 起動 when no printed 起動 is a
    candidate (turn/actions/mod.rs:544-547 find_gained_activation). */
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
            activate_resolved_ability(g, pl, card_id, a, &ab);
            any = 1;
            matched++;
        }
        /* Don't free ab.cost/ab.effect here - the keepalive inside
           activate_resolved_ability owns them, not this decode. */
    }
    /* turn/actions/mod.rs:544-547 — no printed 起動 was accepted, so try the
       gained store before the default-ability fallback below. The gained scan
       runs on EVERY card, not only one that printed 起動 abilities: a gained
       起動 is exactly the case where the printed list has none, which is why
       this arm used to be unreachable. */
    if (matched == 0) {
        int gidx = find_gained_activation_index(g, pl, card_id);
        if (gidx >= 0) {
            Ability gab;
            if (rb_resolve_card_ability_at(g, card_id,
                                           RB_GAINED_ABILITY_INDEX_BASE + gidx, &gab)) {
                activate_resolved_ability(g, pl, card_id,
                                          RB_GAINED_ABILITY_INDEX_BASE + gidx, &gab);
                /* Deliberately NOT rb_free_ability'd: on the prompt-open path
                   choice.c has already released the keepalive copy, and on the
                   closed path the copy is released and this probe's own deep
                   clone is the same shape the printed loop above leaves. */
                any = 1;
                matched++;
            }
        }
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
 * the total cost equals the alternative value (Rust set_cost_modifier(card, base-set)),
 * then re-enter the paused play (ptc_resuming=1 so detection is skipped) to pay the
 * chosen cost and place the card. On decline, place at the base cost. Returns the
 * placement result (0 if nothing was pending / placement failed).
 *
 * The re-entry used to be a bare rb_play_member, which is only correct for the
 * ORDINARY play. Rust re-enters handle_play_member_to_stage with the same
 * arguments it was given (actions/mod.rs:976-982), so an explicit
 * use_baton_touch and an explicit double-baton pair both have to survive the
 * pause. They are latched in s_explicit (see rb_play_member_ex) and re-dispatched
 * through the same two entry points the first pass used. */
int rb_complete_play_with_cost(GameState *g, int pl, int accept) {
    if (!g->ptc_active) return 0;
    int card = g->ptc_card, hand = g->ptc_hand, area = g->ptc_area;
    int set = g->ptc_set, base = g->ptc_base;
    if (accept && set != base)
        rb_mods_set_cost(&g->mods, card, set - base);
    g->ptc_resuming = 1;
    int placed;
    if (s_explicit.active && s_explicit.pair[1] >= 0) {
        /* Explicit double baton that paused on its play-time cost. Rust returns
         * from the cost hook FIRST (phases.rs:1219-1222) and only reaches
         * play_double_baton on re-entry, so the pair is still named here. */
        placed = rb_play_double_baton_ex(g, pl, s_explicit.hand,
                                         s_explicit.pair[0], s_explicit.pair[1],
                                         s_explicit.area);
    } else {
        int use_bt = s_explicit.active ? s_explicit.use_baton_touch : 0;
        placed = rb_play_member_ex(g, pl, hand, area, use_bt);
    }
    g->ptc_resuming = 0;
    g->ptc_active = 0;
    if (!placed) explicit_play_clear();
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

/* ───────────────────────────── phases ─────────────────────────────
 *
 * THE TURN MACHINE IS rb_advance_phase (src/turn/phase.c:66), the C twin of
 * engine/src/turn/phases.rs:75 `TurnEngine::advance_phase`. Every test in the
 * tree drives it; rb_advance_phase is the only path a test reaches.
 *
 * This file used to carry a SECOND, parallel turn machine —
 * activate_wait_members / main_phase / live_phase / check_victory / rollover,
 * reachable only from rb_turn and therefore only from src/main.c:134's default
 * 200-turn batch loop. It is deleted. The five statics were not a second
 * implementation of the same rules; they were a different, much weaker set of
 * rules, and the CLI batch mode was the only thing that ever played by them:
 *
 *   activate_wait_members  4 lines, clearing stage_wait only. phase.c:45 is
 *                          the faithful twenty-line port: it also flips the
 *                          ORIENTATION modifier wait->active, honours
 *                          constant cannot_activate, respects
 *                          rb_mods_is_delayed_cannot_active, ticks the delayed
 *                          flags for this player's own cards, and recomputes
 *                          P->energy_active excluding delayed-cannot-active
 *                          energy (phases.rs:109-175). The 4-line copy did none
 *                          of that, so under the CLI a ウェイト member never
 *                          became アクティブ through the modifier and energy
 *                          activation never happened at all.
 *   main_phase             A greedy auto-player: play every affordable hand
 *                          card into the first free area, then run every staged
 *                          member's DEFAULT ability effect unconditionally
 *                          (the old lines 1513-1525). Rust has no such
 *                          sweep — the Main phase is host-driven
 *                          (game_setup.rs generate_main_phase_actions), and
 *                          firing a 登場 effect every single turn is not what
 *                          登場 means.
 *   live_phase             Auto-set live cards for BOTH seats from hand and
 *                          draw replacements, bypassing the
 *                          LiveCardSetFirstAttacker/SecondAttacker windows
 *                          (phases.rs:224-254) and rb_handle_set_live_card.
 *   check_victory          A deck-out rule with no Rust counterpart, racing the
 *                          real check. phase.c:244-249 is the faithful one and
 *                          is the branch the tests reach.
 *   rollover               Duplicates phase.c's VICTORY rollover — which
 *                          phase.c:277-279 already says in as many words
 *                          ("the other one lives in rb_end_turn ... the tests
 *                          reach THIS one"). Rust has exactly one rollover:
 *                          phases.rs:265-311.
 *
 * So rb_turn is now a THIN HEADLESS DRIVER over the tested machine, not a
 * second one: it advances rb_advance_phase until the turn counter moves, which
 * is exactly Rust's one round (first attacker's Active/Energy/Draw/Main, second
 * attacker's, then the live — phases.rs:90-314). The public name and signature
 * declared in include/rabuka.h:1709 are unchanged, so src/main.c:134 keeps
 * compiling and the CLI still plays whole rounds.
 *
 * PER-TURN RESETS, and why none of them stay here any more.
 * Rust clears its turn-scoped state in the rollover arm (phases.rs:277-311) and
 * phase.c's RB_PHASE_VICTORY arm is the C fold of that. The fields that used to
 * be re-zeroed here have ALL moved into that rollover, which is the only place
 * the turn counter moves and therefore the only place a rollover can be missed:
 *
 *   stage_arrived          Rust's deployed_this_turn (phases.rs:286-287); already
 *                          cleared by the rollover.
 *   baton_touch_used       Rust passes this per PLAY ACTION as an argument
 *                          (phases.rs:1076 -> trigger_scan.rs:73-83), so the C's
 *                          per-player latch is C-local. The rollover clears it.
 *   player_cannot_activate C twin of Rust's cannot_activate_members, which
 *   n_cannot_active_cards  phases.rs:279 DOES clear; the rollover does too.
 *   CORRECTION to the earlier claim here that tests/test_parity_restriction.c
 *   :947-948 "no longer has to zero the field by hand": it still zeroes it, at
 *   test_parity_restriction.c:465-466 / :923-924 / :1006-1007, and :947-948 is a
 *   `board_begin` call. Those are per-sub-test isolation on a fresh board, not
 *   the rollover, so the rollover clear at src/turn/phase.c:427-429 is still
 *   needed and still correct. RESIDUE: tests/ is not this task's to edit.
 *   n_prohibition          C-local prohibition notes (state.c/rules.c/misc.c
 *                          writers), no Rust analogue. Cleared by the rollover.
 *   n_selected_cards       C-local cross-effect selection buffer; choice.c clears
 *                          it on consume and the rollover now clears it too, so a
 *                          stale selection cannot leak into the next round on the
 *                          interactive or web path either.
 *
 * One round == until the turn counter moves (the rollover is the only writer, in
 * the VICTORY branch). RPS + mulligan + both attackers' normal phases + both
 * performance windows is ~16 advance steps; the cap is a runaway guard only.
 *
 * KNOWN AND DELIBERATE: this host PASSES the Main phase and the live-card-set
 * windows, so `rb_engine.exe <deck-dir>` in its default 200-turn mode now plays
 * an empty game (stage=0 live=0 score=0 at turn 201). That is not a regression:
 * the deleted main_phase/live_phase pair was ALSO reaching an empty board at the
 * same sample point (stage=0 live=0 score=0), it just got there through rules
 * that did not exist in Rust. Neither mode is a meaningful game, because neither
 * has a host with a policy — Rust's engine core has no auto-player either
 * (engine/src/bin/bot_p1.rs is where a policy lives, and it has no C twin).
 *
 * The correct follow-up is a host policy over rb_generate_action_candidates
 * (src/turn/phase.c:1351) plus a real answer for every prompt it opens; that is
 * NEW work on a surface no test covers, and an attempted first cut of it here
 * faulted inside the live-performance choice chain when it drained a mandatory
 * SelectCard with the skip answer. So it is deliberately NOT shipped here: the
 * ask was to remove the second turn machine and make the tested path the only
 * path, and that is done without adding an untested auto-player. */
void rb_turn(GameState *g) {
    if (!g || g->winner != -1) return;

    /* See the table above: C-local turn-scoped state with no Rust clear site.
     *
     * These five clears are now REDUNDANT and are gone. rb_advance_phase's
     * turn rollover (src/turn/phase.c, the RB_PHASE_VICTORY arm) clears
     * player_cannot_activate, n_cannot_active_cards, baton_touch_used,
     * n_turn_state_changes and — since the n_prohibition / n_selected_cards fix —
     * n_prohibition and n_selected_cards, and it is the only place the turn
     * counter moves. Every exit from the loop below is therefore downstream of
     * that rollover having already run (`g->turn != start_turn`), or a finished
     * game (`g->winner != -1` / `g->phase == RB_PHASE_DONE`), so re-zeroing the
     * same fields at the top of the NEXT round is a no-op.
     *
     * Verified against the one caller: src/main.c:132-138 loops rb_turn, so
     * round N+1's prologue ran immediately after round N's rollover with nothing
     * in between to write those fields. The interactive path
     * (src/main.c:29-105) never calls rb_turn at all and reaches the rollover
     * through rb_advance_phase directly, which is the path that needed the
     * clear in the first place.
     *
     * NOT removed: explicit_play_clear(). That latch is per PLAY ACTION, not per
     * turn, so a round boundary is not its lifetime — rb_play_member_ex clears
     * it on every completed or refused play. */

    const int start_turn = g->turn;
    for (int step = 0; step < 256; step++) {
        if (g->winner != -1) return;
        /* Advance, then drain. rb_advance_phase refuses to move while a choice is
           open, so the drain has to be inside the loop or a round cannot progress
           past its first prompt. -1 is the skip answer, which is exactly what
           src/main.c:135 passed to the same call after this function returned. */
        if (!rb_has_pending_choice(g)) rb_advance_phase(g);
        if (g->winner != -1) return;
        int drain = 0;
        while (rb_has_pending_choice(g) && drain++ < 32)
            rb_resume_with_choice(g, -1);
        if (g->turn != start_turn) return;   /* the round is complete */
        if (g->phase == RB_PHASE_DONE) return;
    }
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

/* ───────────────────────────── setup ───────────────────────────── */
/* CROSS-FILE REQUEST: rb_setup_game lives in src/game/game_setup.c and has no
   declaration in include/rabuka.h yet (that file is not this file's
   ownership). */
void rb_setup_game(GameState *g);

/* rb_game_init now mirrors Rust's GameState::new + setup_game and NOTHING more.

   DEFECT: the previous body collapsed the whole pre-game ceremony into the
   constructor —
     :1610-1612  drew 6 cards per seat  (Rust does NOT draw here)
     :1613-1619  decided RPS from the seeded RNG and elected the seats
     :1620-1623  jumped straight to RB_PHASE_ACTIVE
   — while :1596 had already set g->phase = RB_PHASE_RPS. RB_PHASE_RPS was set
   and immediately overwritten, so it was dead, and so were RB_PHASE_OPENING
   (Rust's ChooseFirstAttacker) and both RB_PHASE_MULLIGAN_* values. Rust's
   ordering is:
     game_setup.rs:555-561  GameState::new(p1 first-attacker, p2 not) — decks
                            shuffled, phase = Active, NO cards drawn
     game_setup.rs:562      setup_game → setup_initial_energy (Rule 6.2.1.7)
                            then current_phase = Phase::RockPaperScissors
     actions/mod.rs:291-301 the HOST then drives RockChoice/ScissorsChoice →
                            ChooseFirstAttacker (draw 6 each, mulligan) → Active
   Rust's engine core NEVER decides RPS itself; the host does. So this
   constructor now stops at RB_PHASE_RPS with the seat defaults Rust's
   Player::new uses (P1 is the first attacker until RPS says otherwise) and
   leaves the ceremony to the host — via rb_setup_execute_action, or via
   rb_advance_phase's headless auto-pass for the C harnesses.

   Consequence, called out loudly: a caller that used rb_game_init as "give me
   a playable game" no longer gets hands. tests/replay.c (rb_engine_replay,
   rb_engine_scenario_generated) is the known such caller and must now drive
   the ceremony (rb_setup_settle_auto, or three rb_advance_phase calls). The
   ~141 test suites are unaffected: they use test_game.c's test_game_new, not
   rb_game_init. */
int rb_game_init(GameState *g, const uint32_t *deck0, int n0,
                 const uint32_t *deck1, int n1) {
     memset(g, 0, sizeof(*g));
     rb_look_reset_all();
     rb_mods_init(&g->mods);
     g->winner = -1; g->turn = 1;
     g->cheer_check_base = -1;
     g->baton_touch_replaced_member_cost = -1;
     g->baton_touch_replaced_member_id = -1;
     g->baton_touch_arriving_card_id = -1;
     for (int i = 0; i < 64; i++) g->gained_card_ids[i] = -1;
    /* game_setup.rs:555-558 — Player::new("p1", …, true) / Player::new("p2",
       …, false), i.e. P1 is the first attacker until RPS elects otherwise.
       Rust's mulligan_owner_index (game_state/mod.rs:790-791) keys off exactly
       these flags, and Phase::ActivePlayer / active_player_index() read them,
       so they must hold a coherent pair from the moment the struct exists. */
    g->first_attacker = 0;
    g->second_attacker = 1;
    g->active = g->first_attacker;
    /* Rust's Option<u8> RPS choices start as None and rps_winner starts as
       None. memset leaves all three at 0, and 0 is a VALID RPS gesture (rock)
       and a VALID winner index, so every one of them is initialised to -1
       explicitly. Without this the very first rb_resolve_rps_if_both_chosen
       would see "both players already chose rock" and resolve a tie nobody
       played. */
    g->player1_rps_choice = -1;
    g->player2_rps_choice = -1;
    g->rps_winner = -1;
    g->phase = RB_PHASE_RPS;
    for (int pl = 0; pl < 2; pl++) {
        const uint32_t *d = (pl == 0) ? deck0 : deck1;
        int n = (pl == 0) ? n0 : n1;
        if (n > RB_MAX_DECK) n = RB_MAX_DECK;
        for (int i = 0; i < RB_STAGE_SIZE; i++) g->p[pl].stage[i] = -1;
        for (int i = 0; i < n; i++) g->p[pl].deck.cards[i] = (int)d[i];
        g->p[pl].deck.n = n;
        rb_shuffle(g->p[pl].deck.cards, g->p[pl].deck.n);
    }
    /* game_setup.rs:358-364 setup_game — Rule 6.2.1.7 initial energy (3 cards
       from EACH player's energy deck into their energy zone) then
       current_phase = Phase::RockPaperScissors.

       The energy draw is a no-op when the caller has not populated the energy
       decks yet, which is the normal shape here: rb_init_game_from_built_decks
       (src/game/deck_builder.c) calls rb_game_init FIRST and only then calls
       rb_player_set_energy_deck + rb_setup_initial_energy, so the cards land
       exactly once, in Rust's order. */
    rb_setup_game(g);
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

/* ── deck choice (main.rs::choose_deck) — REMOVED ────────────────────────────
 * Rust main.rs:287-293 choose_deck is a two-line "pick the first deck list"
 * placeholder called twice from main.rs:118-119, over the value
 * deck_parser::DeckParser::parse_all_decks() returns.
 *
 * The C cannot reach it, and the reason is structural rather than a missing
 * call: RbDeckList / RbDeckEntry below were C-local types that NOTHING in the
 * tree produces. src/main.c:115-122 does not parse deck lists at all — it
 * synthesises both decks straight from the card database
 * (rb_card_ability_idx / rb_num_cards) and never sees a deck-list file. So
 * there was no way to hand this function a non-empty list, and it held no
 * engine state, applied no rule, and touched no card: keeping it would have
 * been a callable that provably cannot do anything. Its caller chain in Rust
 * (deck_parser::parse_all_decks -> choose_deck -> deck_list_to_card_numbers)
 * has no C counterpart because src/game/deck_parser.c is reached through
 * rb_build_deck_from_card_numbers instead. */

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

/* main.rs:294-316, guarded by #[cfg(feature = "server")] and called ONCE at
   boot from main.rs:65, immediately before run_web_server.
 *
   KEPT, and this is a missing CALL rather than dead code. Rust's cfg(server)
   block is the SERVER binary, and the C's server binary is
   src/web_server.c (the Makefile's rb_web_server target, web_server.c linked
   over the same objects). So the Rust caller has a C counterpart binary that
   simply does not make the call yet.
   ACTION FOR web_server.c's OWNER: invoke i18n_self_check() once at startup,
   before the request loop starts, to reproduce main.rs:65. Everything the
   function needs is already here — the canonical template table
   (describe.rs:1053-1064) and the faithful
   translate_choice_prompt_en_to_ja (describe.rs:1070-1127) directly above — so
   the port is complete and only the boot call is missing. */
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

/* ── web server (main.rs::run_web_server) — REMOVED ────────────────────────────
 * Rust main.rs:318-330, #[cfg(feature = "server")], called once from
 * main.rs:66. Its body is NOT a placeholder: it builds an actix_web::rt::System
 * and blocks on web_server::run_web_server_with_ngrok(ngrok_authtoken) — the
 * real server.
 *
 * The C twin of that real server already exists and is the whole of
 * src/web_server.c (the Makefile's rb_web_server target). This function was a
 * second entry point to a server that already had one, and its body was a
 * STUB by this project's own definition (PROGRESS.md anti-pattern 1): two
 * printfs and a return, with no server, no ngrok, and no request handling. It
 * had zero callers. Deleting it removes the stub rather than leaving a callable
 * that cannot serve anything; src/web_server.c remains the single server. */
