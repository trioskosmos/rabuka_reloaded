#include "test_game.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

void test_game_new(TestGame *tg){
    memset(tg,0,sizeof(*tg));
    rb_look_reset_all();
    rb_mods_init(&tg->state.mods);
    tg->state.winner=-1; tg->state.turn=1;
    tg->state.phase=RB_PHASE_MAIN;
    tg->state.active=0; tg->state.first_attacker=0; tg->state.second_attacker=1;
    for(int p=0;p<2;p++) for(int i=0;i<RB_STAGE_SIZE;i++) tg->state.p[p].stage[i]=RB_EMPTY_SLOT;
    tg->state.cheer_check_base = -1;
    tg->state.baton_touch_replaced_member_cost = -1;
    tg->state.baton_touch_replaced_member_id = -1;
     tg->state.baton_touch_arriving_card_id = -1;
     for (int i = 0; i < 64; i++) tg->state.gained_card_ids[i] = -1;
}


int test_id(TestGame *tg, const char *card_no){
    (void)tg;
    return rb_find_card_by_no(card_no);
}
int test_new_id(TestGame *tg, const char *card_no){
    (void)tg;
    int template_id = rb_find_card_by_no(card_no);
    if (template_id < 0) return -1;
    int copy = rb_create_card_copy(template_id);
    if (copy < 0) {
        /* LOUD, on purpose. rb_create_card_copy draws from a process-global
           monotonic pool (data.c:285 g_next_copy_id) that NOTHING in the
           corpus resets between cases, so it runs dry after
           RB_MAX_CARD_IDS - num_cards (= 4096 - 2639 = 1457) allocations --
           and rb_build_deck_from_card_numbers (deck_builder.c:63,71) eats
           from the SAME pool. A caller that ignored this -1 and went on to
           place the card got a silently-wrong fixture: several exhausted
           calls all yield -1, so two different prints resolve to the same id
           and every "these are distinct instances" / "this is that print"
           assertion after it becomes vacuously true. Say so on stderr. */
        fprintf(stderr,
                "[TEST_POOL_EXHAUSTED] test_new_id(\"%s\"): the card-copy pool "
                "is full (%d ids). Call test_card_copy_pool_reset() between "
                "cases, or this id is invalid and identity assertions on it "
                "prove nothing.\n",
                card_no ? card_no : "(null)", RB_MAX_CARD_IDS);
    }
    return copy;
}
/* Drop every card-copy id back into the pool (data.c:279). MUST be called
   between independent cases by any suite that leans on test_new_id /
   test_id_exact / rb_build_deck_from_card_numbers: the pool is process
   global and monotonic, so without this a long file runs dry and every
   later test_new_id returns -1. Only call it when no live card copy from
   before the reset is still in play -- the ids themselves are invalidated,
   the underlying card records are not. */
void test_card_copy_pool_reset(void){ rb_card_copy_pool_reset(); }

/* Mirror Rust TestGame::id_ref (engine/tests/helpers/mod.rs:245-253):
 *
 *     pub fn id_ref(&self, card_no: &str) -> i16 {
 *         let template_id = card_id(&self.db, card_no);
 *         let pos = self.pool_positions.borrow().get(&template_id).copied().unwrap_or(0);
 *         pool.get(&template_id).and_then(|v| v.get(pos).copied()).unwrap_or(template_id)
 *     }
 *
 * A PEEK, not a consume: it answers "what id would the NEXT id()/new_id() call
 * for this card_no hand out?" WITHOUT advancing the per-card counter. That is
 * the whole point of the helper — a test can look at the instance a fixture is
 * about to create and assert on it without spending it, which is what lets
 * `let a = game.id(X); assert_eq!(game.id_ref(X), a + 1);` work.
 *
 * MISSING FROM THE C until the Q4 audit. Nothing in tests/ referenced it (0 call
 * sites), which is why the gap survived: an unused helper is invisible to a
 * ratchet that counts call sites.
 *
 * The C cannot reproduce the Rust counter, because the C's `test_id` has NO
 * counter at all (it is `rb_find_card_by_no`, the shared template, every call —
 * see test_harness_self.c §C). So `test_id_ref` returns the TEMPLATE id, which is
 * exactly what the C's `test_id` will hand out. It is therefore an honest
 * implementation of "peek at what test_id would return" and a documented
 * divergence from the Rust's "peek at what the NEXT id() would return". Stated
 * here rather than left to be discovered: in the C the two are the same number
 * for the same card_no, forever. */
int test_id_ref(TestGame *tg, const char *card_no){
    (void)tg;
    return rb_find_card_by_no(card_no);
}

/* Strict card_no lookup: EXACT match only, or -1.
   rb_find_card_by_no (data.c:529) is deliberately forgiving -- after the
   exact hit it falls through to a base+rarity rebuild, then to
   card_no_folded_prefix_lowest, and finally to a *substring* search
   (card_no_contains_lowest, data.c:446). That chain is what the engine's
   own UI needs, but in a test it means a typo'd or truncated card number
   resolves to SOME real card instead of failing: "PL!SP-PR-004-R" has no
   exact match, base "PL!SP-PR-004" + rarity "R" does not match, the prefix
   arm picks the lowest "PL!SP-PR-004-*" key, and the fixture is quietly the
   -PR print. test_id keeps that forgiving behaviour because most of the
   corpus relies on it; use test_id_exact when the print itself is the thing
   under test. */
int test_id_exact(TestGame *tg, const char *card_no){
    (void)tg;
    if(!card_no) return -1;
    uint32_t n = rb_num_cards();
    for(uint32_t i=0;i<n;i++){
        Card c;
        if(!rb_decode_card_by_index(i,&c)) continue;
        const char *cn = rb_card_string(c.card_no_idx);
        int eq = (cn && strcmp(cn, card_no)==0);
        rb_free_card(&c);
        if(eq) return (int)i;
    }
    return -1;
}
void test_add_to_hand_for(TestGame *tg, int pl, int card_id){
    if(!tg || pl<0 || pl>1) return;
    RbPlayer *P=&tg->state.p[pl];
    if(P->hand.n < RB_MAX_ZONE) P->hand.cards[P->hand.n++]=card_id;
}
void test_add_to_hand(TestGame *tg, int card_id){ test_add_to_hand_for(tg, 0, card_id); }
void test_add_to_discard(TestGame *tg, int card_id){
    RbPlayer *P=&tg->state.p[0];
    if(P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++]=card_id;
}
void test_add_to_stage(TestGame *tg, int area, int card_id){
    if(area<0||area>=RB_STAGE_SIZE) return;
    tg->state.p[0].stage[area]=card_id;
    tg->state.p[0].stage_wait[area]=0;
}
void test_place_under(TestGame *tg, int pl, int area, int card_id){
    /* Mirror stage.place_under_card(area, card): tuck `card_id` under the
        member occupying `area` of player `pl` (0=p1, 1=p2). */
    if(pl<0||pl>1||area<0||area>=RB_STAGE_SIZE) return;
    RbPlayer *P=&tg->state.p[pl];
    RbBag *u=&P->under_cards[area];
    if(u->n < RB_MAX_ZONE) u->cards[u->n++]=card_id;
}
void test_add_to_success(TestGame *tg, int card_id){
    RbPlayer *P=&tg->state.p[0];
    if(P->success.n < RB_MAX_ZONE) P->success.cards[P->success.n++]=card_id;
}
void test_add_to_live(TestGame *tg, int card_id){ test_add_to_live_for(tg, 0, card_id); }
/* Seat-aware raw live placement (Rust live_card_zone.cards.push with no hand
   removal). Like test_set_live_card_for this APPENDS, and it appends to `pl`'s
   OWN live zone. The cap stays RB_MAX_ZONE (the bag capacity test_add_to_live
   has always used) so this shim's reachability does not change. */
void test_add_to_live_for(TestGame *tg, int pl, int card_id){
    if(!tg || pl<0 || pl>1 || card_id < 0) return;
    RbPlayer *P=&tg->state.p[pl];
    if(P->live.n < RB_MAX_ZONE) P->live.cards[P->live.n++]=card_id;
}
void test_add_to_deck(TestGame *tg, int card_id){
    RbPlayer *P=&tg->state.p[0];
    if(P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++]=card_id;
}
void test_add_to_deck_pl(TestGame *tg, int pl, int card_id){
    if(pl<0||pl>1) return;
    RbPlayer *P=&tg->state.p[pl];
    if(P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++]=card_id;
}
/* Add a card to the energy deck (mirrors Rust energy_deck.cards.push) */
void test_add_to_energy_deck(TestGame *tg, int pl, int card_id){
    if(pl<0||pl>1) return;
    RbPlayer *P=&tg->state.p[pl];
    if(P->energy_deck.n < RB_MAX_ZONE) P->energy_deck.cards[P->energy_deck.n++]=card_id;
}
/* Prepend a card to the top of player pl's deck (mirrors Rust
    main_deck.cards.insert(0, card)). Shifts existing cards down by one; if the
    deck is full the new card is dropped (matching a saturated RbBag). */
void test_insert_deck_top(TestGame *tg, int pl, int card_id){
    if(pl<0||pl>1) return;
    RbPlayer *P=&tg->state.p[pl];
    if(P->deck.n >= RB_MAX_ZONE) return;
    for (int i = P->deck.n; i > 0; i--) P->deck.cards[i] = P->deck.cards[i-1];
    P->deck.cards[0] = card_id;
    P->deck.n++;
}
void test_add_to_energy(TestGame *tg, int pl, int card_id){
    if(pl<0||pl>1) return;
    RbPlayer *P=&tg->state.p[pl];
    /* Mirror rb_energy_add_card (zones.c): the card is appended and the
       ACTIVE count rises with it. Only bump active when the card really
       landed, so energy_active <= energy.n stays true (the invariant
       rb_energy_activate_all relies on). */
    if(P->energy.n >= RB_MAX_ZONE) return;
    P->energy.cards[P->energy.n++]=card_id;
    if(P->energy_active < P->energy.n) P->energy_active++;
}
void test_set_energy_active(TestGame *tg, int pl, int n){
    if(pl<0||pl>1) return;
    tg->state.p[pl].energy_active = n;
}
void test_add_to_revealed(TestGame *tg, int card_id){
    if(tg->state.n_revealed < RB_MAX_REVEALED_CARDS)
        tg->state.revealed_cards[tg->state.n_revealed++]=card_id;
}
void test_give_opp_energy(TestGame *tg, int count){
    test_give_energy_for(tg, 1, count);
}
/* Find a live card with a specific score — mirrors Rust's db lookup.
   Returns the card index, or -1 when no live card carries that score. */
int test_find_live_by_score(TestGame *tg, int score){
    (void)tg;
    uint32_t n = rb_num_cards();
    for(uint32_t i = 0; i < n; i++){
        if(!rb_card_is_live((int)i)) continue;
        Card c;
        if(!rb_decode_card_by_index(i, &c)) continue;
        int match = (int)c.score == score;
        rb_free_card(&c);
        if(match) return (int)i;
    }
    return -1;
}

void test_give_energy(TestGame *tg, int count){ test_give_energy_for(tg, 0, count); }

/* Give `count` energy cards to seat `pl`, all Active.
   Mirror Rust TestGame::give_energy_for -> energy_zone.push_active. The C
   energy zone is an unordered bag plus a SEPARATE `energy_active` count (see
   rb_energy_add_card / rb_energy_activate_all in zones.c), so "push active"
   is: append the card, then raise the active count with it. A card that does
   not land never raises the count, so energy_active <= energy.n — the
   invariant rb_energy_activate_all relies on — always holds. */
void test_give_energy_for(TestGame *tg, int pl, int count){
    if(!tg || pl<0 || pl>1) return;
    int eid = rb_find_card_by_no("LL-E-001-SD");
    if(eid<0) eid=0;
    RbPlayer *P=&tg->state.p[pl];
    for(int i=0;i<count;i++){
        if(P->energy.n >= RB_MAX_ZONE) return;  /* bypasses the 7/12 cap, not the bag */
        P->energy.cards[P->energy.n++]=eid;
        P->energy_active++;
    }
}
/* Mirror Rust TestGame::play_to_stage(card_id, area) — which PANICS when the
   target area already holds a DIFFERENT card (helpers/mod.rs:390-400), because
   the three-slot stage makes that a baton touch: the occupant is replaced and
   sent to the waitroom. The Rust header calls this out as "a passing but wrong
   test", and records that 137 tests were found doing it by accident (the §8.3
   `PL!-pb2-029-N` vacuous negative is the named example).

   C cannot panic, so the guard is a LOUD line on stderr instead of a silent
   substitution — the same tag the pool-exhaustion guard above uses. It sits on
   `test_play_to_stage` ONLY, because Rust's guard sits only there:
   `try_play_to_stage` (:446) / `play_to_stage_baton` (:404) do NOT check, and
   the C's `test_try_play_to_stage` / `test_play_to_stage_for` are those twins.
   A stderr line that does not begin with `FAIL` cannot move the `^FAIL` verdict
   count, so a suite that legitimately bats on stays green and still gets told.
   Pinned by §G (G15/G16) of tests/test_harness_self.c. */
int test_play_to_stage(TestGame *tg, int card_id, int area){
    if(tg && area>=0 && area<RB_STAGE_SIZE){
        int occ = tg->state.p[0].stage[area];
        if(occ != RB_EMPTY_SLOT && occ != card_id)
            fprintf(stderr,
                    "[TEST_BATON_TOUCH] test_play_to_stage(%d, area %d): area %d "
                    "already holds card %d, so this play BATON-TOUCHES it (the "
                    "occupant is replaced and goes to the waitroom). Rust's "
                    "TestGame::play_to_stage panics here (helpers/mod.rs:391); "
                    "use test_play_to_stage_for / test_try_play_to_stage if the "
                    "swap is intended.\n",
                    card_id, area, area, occ);
    }
    return test_play_to_stage_for(tg, 0, card_id, area);
}
/* Mirror TestGame::try_play_to_stage_for(Side, card, area): make `pl` the
   attacking seat, then run the real main-phase play action. Returns 1 on
   success, 0 when the card is not in that seat's hand.

   THE ATTACKING SEAT IS SET FIRST, and that is the fix. Rust
   (engine/tests/helpers/mod.rs:290) calls `set_active_side(side)` before the
   action, and `set_active_side` (:255-262) writes BOTH `player1
   .is_first_attacker` and `player2.is_first_attacker`. The C models that pair
   as the seat indices `first_attacker` / `second_attacker` (`first_attacker ==
   0` means p1 leads — game_setup.c:534), so the faithful two-line write is
   `first_attacker = pl; second_attacker = pl ^ 1;`.

   This shim used to pass `pl` to the engine and touch neither, so every
   `test_play_to_stage_for(tg, 1, ..)` ran with seat 0 still marked as the
   attacker. Measured 8 such call sites in the corpus
   (test_parity_compound_per_card.c:2097, test_parity_condition_and_effect.c:791,
   test_parity_integration_extra.c:1740, test_parity_jidou_state.c:827,
   test_parity_jidou_watch_clusters.c:2135, test_parity_support.c:682,
   test_parity_trigger_paths.c:893 and :1116), and every 「自分のアタッカーの
   ターン」 / attacker's-turn condition, and every owner-scoped drain keyed off
   the attacker, read the wrong seat in all eight. Pinned by §H of
   tests/test_harness_self.c. */
static void test_set_active_seat(TestGame *tg, int pl){
    if(!tg || pl<0 || pl>1) return;
    tg->state.first_attacker  = pl;
    tg->state.second_attacker = pl ^ 1;
}
int test_play_to_stage_for(TestGame *tg, int pl, int card_id, int area){
    if(!tg || pl<0 || pl>1) return 0;
    test_set_active_seat(tg, pl);
    RbPlayer *P=&tg->state.p[pl];
    int idx=-1;
    for(int i=0;i<P->hand.n;i++) if(P->hand.cards[i]==card_id){ idx=i; break; }
    if(idx<0) return 0;
    return rb_play_member(&tg->state, pl, idx, area);
}
/* Rust's `try_play_to_stage` (helpers/mod.rs:446) carries NO baton guard, so it
   is routed to `test_play_to_stage_for` directly rather than through
   `test_play_to_stage`. Return value is identical either way — `test_play_to_
   to_stage` only adds the stderr diagnostic — this keeps the diagnostic on the
   one helper that is Rust's panicking twin. */
int test_try_play_to_stage(TestGame *tg, int card_id, int area){
    return test_play_to_stage_for(tg, 0, card_id, area);
}

/* The two explicit-baton engine entry points are NOT declared in
   include/rabuka.h — engine.c forward-declares them for itself (engine.c:645-646)
   and nothing else has ever named them. That omission is WHY the baton-touch
   test helpers were missing: the seam existed and was unreachable. Local externs
   here, following the convention already used below (test_max_distinct_names,
   hb_arena_token_crc). REPORTED as a rabuka.h gap, not fixed here. */
extern int rb_play_member_ex(GameState *g, int pl, int hand_idx, int stage_pos, int use_baton_touch);
extern int rb_play_double_baton_ex(GameState *g, int pl, int hand_idx, int a1, int a2, int place);

/* Mirror Rust TestGame::play_to_stage_baton(card_id, area) (mod.rs:404-407) —
   "play a member into an area that MAY already be occupied, for tests that
   deliberately exercise a baton touch".

   This was MISSING ENTIRELY until 2026-10-03. Rust splits the two cases into
   two functions precisely because the automatic swap is a trap: `play_to_stage`
   panics on an occupied area and `play_to_stage_baton` is the opt-in. The C had
   only the panicking twin's guard and no way to say "the swap is intended", so
   a Rust test written against `play_to_stage_baton` had no C spelling at all.

   The engine already carries the flag — rb_play_member_ex's 5th parameter, the
   direct mirror of actions/mod.rs:312-318 forwarding the action's
   `use_baton_touch: Option<bool>` (engine.c:645) — so this is the caller half
   of an existing seam, not a new engine entry point. */
int test_play_to_stage_baton(TestGame *tg, int card_id, int area){
    if(!tg) return 0;
    RbPlayer *P=&tg->state.p[0];
    int idx=-1;
    for(int i=0;i<P->hand.n;i++) if(P->hand.cards[i]==card_id){ idx=i; break; }
    if(idx<0) return 0;
    return rb_play_member_ex(&tg->state, 0, idx, area, 1);
}

/* Mirror Rust TestGame::play_to_stage_double_baton(card, area_a, area_b, land_in)
   (mod.rs:418-435): replace the members in TWO areas and land the played card
   in a third.

   Also missing until 2026-10-03, and it is not a cosmetic gap. Rust's doc on it
   is the reason: "The engine takes the double-baton route only when the action
   carries two area INDICES (card_indices); with one index, or none, it takes the
   single-play path and a 'baton touch' against an empty area just places the
   card." A single-slot helper therefore CANNOT build the two-member debut that
   PL!-pb2-000-DUO is gated on — the gate needs trigger_event.min_count = 2
   recorded touches, and only this path records two (engine.c:672-693).

   use_baton_touch is `Some(false)` in the Rust (mod.rs:432), matching the C's
   5th argument of 0: the double-baton branch is taken from the card_indices,
   not from a baton request. */
int test_play_to_stage_double_baton(TestGame *tg, int card_id, int area_a, int area_b, int land_in){
    if(!tg) return 0;
    RbPlayer *P=&tg->state.p[0];
    int idx=-1;
    for(int i=0;i<P->hand.n;i++) if(P->hand.cards[i]==card_id){ idx=i; break; }
    if(idx<0) return 0;
    return rb_play_double_baton_ex(&tg->state, 0, idx, area_a, area_b, land_in);
}
void test_recalc(TestGame *tg){ rb_recalc_constants(&tg->state); }
void test_clear_mods_for_card(TestGame *tg, int card_id){ rb_mods_clear_card(&tg->state.mods, card_id); }
void test_set_opp_stage(TestGame *tg, int area, int card_id){
    if(area<0||area>=RB_STAGE_SIZE) return;
    tg->state.p[1].stage[area]=card_id;
    tg->state.p[1].stage_wait[area]=0;
}
void test_add_to_opp_live(TestGame *tg, int card_id){ test_add_to_live_for(tg, 1, card_id); }
void test_add_to_opp_success(TestGame *tg, int card_id){
    RbPlayer *P=&tg->state.p[1];
    if(P->success.n < RB_MAX_ZONE) P->success.cards[P->success.n++]=card_id;
}
void test_fire_debut(TestGame *tg, int card_id){ rb_fire_debut(&tg->state, 0, card_id); }
void test_expire_effects(TestGame *tg){ rb_check_expired_effects(&tg->state, 0); }
int test_activate_ability(TestGame *tg, int card_id){
    return test_activate_ability_for(tg, 0, card_id);
}
/* Mirror TestGame::activate_ability_for(Side, card): make `pl` the attacking
   seat, then run the real 起動 path.

   THE ATTACKING SEAT IS SET FIRST for the same reason and with the same
   citation as test_play_to_stage_for above: Rust's
   `activate_ability_for` (helpers/mod.rs:302) calls `set_active_side(side)`
   before the action, and the C had no equivalent write. */
int test_activate_ability_for(TestGame *tg, int pl, int card_id){
    if(!tg || pl<0 || pl>1) return 0;
    test_set_active_seat(tg, pl);
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < P->hand.n; i++)
        if (P->hand.cards[i] == card_id) return rb_activate_ability(&tg->state, pl, i);
    /* Rust activate_ability also fires a member already on stage — run the real
        multi-ability activate path (cost + 起動-triggered effect). */
    return rb_activate_card(&tg->state, pl, card_id);
}
void test_spend_energy(TestGame *tg, int n){
    RbPlayer *P=&tg->state.p[0];
    P->energy_active -= n;
    if(P->energy_active < 0) P->energy_active = 0;
}
void test_drain_auto_choices(TestGame *tg){
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 1000) {
        if (tg->state.queue.resume_mode == 3 || tg->state.queue.auto_ability)
            rb_resume_with_choice(&tg->state, 0);  /* proceed with auto ability */
        else
            break;  /* only auto-ability prompts are drainable here */
    }
}
/* Answer a single interactive (position/target) pending choice by index — mirrors
   the test's select_position_option / accept_position_swap calls. */
void test_resume_choice(TestGame *tg, int idx){
    if (rb_has_pending_choice(&tg->state)) rb_resume_with_choice(&tg->state, idx);
}

/* Select indices from a pending choice — mirrors game.select_indices(&[..]).
   n == 0 is Rust's DECLINE (select_indices(&[])) and must go through the
   engine's skip path (rb_resume_with_choice(g, -1)); it is NOT a no-op.
   n > 1 is a genuine multi-index answer and must reach the engine intact
   (rb_resume_with_choice_indices). The previous revision used only indices[0]
   and skipped the resume entirely when n == 0, so "decline" left the prompt
   pending (turning the ubiquitous
       while (test_has_pending_choice(g)) test_select_indices(g, NULL, 0);
   drain loops into no-ops) and a multi-card pick could never complete. */
void test_select_indices(TestGame *tg, const int *indices, int n){
    if (!tg || !rb_has_pending_choice(&tg->state)) return;
    if (n > 0 && indices) rb_resume_with_choice_indices(&tg->state, indices, n);
    else                     rb_resume_with_choice(&tg->state, -1);
}
int test_has_pending_choice(TestGame *tg){ return rb_has_pending_choice(&tg->state); }
/* Mirror Rust TestGame::pending_choice_count (engine/tests/helpers/choices.rs:421-428):
 *
 *     match self.pending_choice_view() {
 *         Live(Choice::SelectCard { count, .. }) => *count,
 *         Live(_)                               => 0,
 *         Json(pc) => pc["count"].as_u64().map_or(0, |c| c as usize),
 *         None                                  => 0,
 *     }
 *
 * So the answer is HOW MANY CARDS THE PROMPT ASKS FOR, not "is one standing".
 *
 * DIVERGENCE, found by the Q4 audit and FIXED here. This used to be
 *     return rb_has_pending_choice(&tg->state) ? 1 : 0;
 * i.e. a CONSTANT 1 for every prompt that exists. For a `SelectCard { count: 1 }`
 * — the common case — that is right by luck. For every other SelectCard it is
 * wrong: a "look at five, take three" prompt reads 1, and so does a mandatory
 * 5-pick. Three call sites already assert the real number and were therefore
 * measuring the harness rather than the engine:
 *     tests/test_parity_characterization_extra.c:1484  expects 2
 *     tests/test_parity_characterization_extra.c:1495  expects 1
 *     tests/test_parity_characterization_extra.c:1505  expects 3
 * A `Json` pending choice has no analogue in the C's flat RbChoice, and the C
 * models every pending prompt as an RbChoice, so the Json arm is unreachable
 * here rather than unimplemented.
 *
 * The non-SelectCard arm returning 0 (not 1) is Rust's behaviour and is
 * load-bearing: `SelectHeartColor`, `SelectAutoAbility` and `SelectLiveSuccess`
 * ask for no cards, so a suite using this to decide "how many picks do I owe"
 * must read 0 for them.
 */
int test_pending_choice_count(TestGame *tg){
    if(!tg || !rb_has_pending_choice(&tg->state)) return 0;
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    if(!c) return 0;
    if(c->kind == RB_CHOICE_SELECT_CARD) return c->count < 0 ? 0 : c->count;
    return 0;
}
void test_set_live_card(TestGame *tg, int slot, int card_id){
    /* Live-card placement — mirror Rust TestGame::set_live_card(card_id).
       The live zone is an ORDERED bag: the card leaves the hand and is
       APPENDED to live.cards[live.n++], which is what live_card_zone.cards
       .push does in Rust.

       The `slot` parameter is IGNORED. It exists only because
       tools/port_one.py emits test_set_live_card(&game, 0, id) for the
       one-argument Rust call, and every call site in the transpiled corpus
       passes 0. The previous revision wrote live.cards[slot] at a FIXED index
       and only grew n to slot+1, so three consecutive calls put ONE card in
       the zone and silently invalidated every transpiled live test (canary
       live_cards_stuck_in_live_zone_instead_of_discard: "got 1 expected 3").
       Use test_insert_live_card_at for genuine positional placement. */
    (void)slot;
    test_set_live_card_for(tg, 0, card_id);
}
/* Seat-aware live placement — mirror TestGame::set_live_card_for(Side, card).
   The live zone it writes is `pl`'s OWN, so a P2-owned live card really lands
   in p2's live_card_zone (which is what a P2-triggered ability scans) instead
   of silently landing in P1's zone, where the fixture still looks plausible
   and measures the wrong thing. Still the APPEND path; never a fixed index. */
void test_set_live_card_for(TestGame *tg, int pl, int card_id){
    if(!tg || pl<0 || pl>1 || card_id < 0) return;
    RbPlayer *P=&tg->state.p[pl];
    for (int i = 0; i < P->hand.n; i++) {
        if (P->hand.cards[i] == card_id) {
            rb_hand_remove_card(P, i);
            break;
        }
    }
    if (P->live.n >= RB_MAX_LIVE_CARDS) return;   /* the live zone holds at most 3 */
    P->live.cards[P->live.n++] = card_id;
}
void test_insert_live_card_at(TestGame *tg, int slot, int card_id){
    test_insert_live_card_at_for(tg, 0, slot, card_id);
}
/* Explicit positional live-zone placement: shift the tail right and place the
   card at `slot`. Test-only; prefer the append path (test_set_live_card) so a
   test never has to reason about slot reuse.

   Contract, chosen so a live zone can never contain a hole:
     slot == n          append at the tail;
     slot  < n          ordered insert, tail shifts right, and if the 3-card
                        zone is already full the LAST card falls off the end;
     slot  > n          refused -- the live zone is an ordered bag, and a gap
                        there would be read either as a phantom card (the bare
                        slot still holds whatever the zone had, 0 on a fresh
                        TestGame, which IS a real card index) or as end-of-zone
                        by the zone accessors. Neither is a card state the
                        engine can produce, so the placement is refused
                        instead of fabricating one. */
void test_insert_live_card_at_for(TestGame *tg, int pl, int slot, int card_id){
    if(!tg || pl<0 || pl>1 || card_id < 0) return;
    if(slot<0 || slot>=RB_MAX_LIVE_CARDS) return;
    RbPlayer *P=&tg->state.p[pl];
    for (int i = 0; i < P->hand.n; i++) {
        if (P->hand.cards[i] == card_id) {
            rb_hand_remove_card(P, i);
            break;
        }
    }
    if (slot > P->live.n) return;            /* would leave a gap: refuse */
    if (slot == P->live.n) {
        P->live.n = slot + 1;
    } else {
        int n = P->live.n;
        if (n >= RB_MAX_LIVE_CARDS) {
            /* Full: shift the tail right and let the last card fall off. The
               previous revision clamped to CAP-1 and then wrote cards[slot]
               over a live card, silently destroying it. */
            for (int i = RB_MAX_LIVE_CARDS - 1; i > slot; i--)
                P->live.cards[i] = P->live.cards[i-1];
        } else {
            for (int i = n; i > slot; i--) P->live.cards[i] = P->live.cards[i-1];
            P->live.n = n + 1;
        }
    }
    P->live.cards[slot] = card_id;
}
/* Debug-only card-name accessor. rb_decode_card_by_index allocates a Card
   whose `name` is heap-owned, so the previous revision either leaked the whole
   Card on every call (it never freed) or, once freed, handed back a dangling
   pointer. Copy the name into a small rotating set of static buffers: no
   leak, and the returned pointer stays valid for the last 8 calls. */
#define TEST_NAME_SLOTS 8
#define TEST_NAME_LEN   256
static char g_name_buf[TEST_NAME_SLOTS][TEST_NAME_LEN];
static int  g_name_next;
const char *test_card_name(int card_id){
    if(card_id < 0) return "?";
    Card c;
    if(!rb_decode_card_by_index((uint32_t)card_id,&c)) return "?";
    char *slot = g_name_buf[g_name_next];
    g_name_next = (g_name_next + 1) % TEST_NAME_SLOTS;
    if(c.name){
        strncpy(slot, c.name, TEST_NAME_LEN - 1);
        slot[TEST_NAME_LEN - 1] = '\0';
    } else {
        slot[0] = '?'; slot[1] = '\0';
    }
    rb_free_card(&c);
    return slot;
}
int test_stage_has(TestGame *tg, int area, int card_id){
    if(!tg || area<0 || area>=RB_STAGE_SIZE) return 0;
    return tg->state.p[0].stage[area]==card_id;
}
int test_hand_has(TestGame *tg, int card_id){
    for(int i=0;i<tg->state.p[0].hand.n;i++) if(tg->state.p[0].hand.cards[i]==card_id) return 1;
    return 0;
}
int test_success_count(TestGame *tg){ return tg->state.p[0].success.n; }
void test_print_board(TestGame *tg){ rb_print_state(&tg->state); }

/* Advance the current player's phase, mirroring Rust TestGame::pass() which
   feeds execute_main_phase_action(ActionType::Pass). */
void test_pass(TestGame *tg){ rb_advance_phase(&tg->state); }

const char *test_pending_choice_type(TestGame *tg){
    if(!rb_has_pending_choice(&tg->state)) return "";
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    switch(c->kind){
        case RB_CHOICE_SELECT_CARD:        return "SelectCard";
        case RB_CHOICE_SELECT_TARGET:      return "SelectTarget";
        case RB_CHOICE_SELECT_HEART_COLOR: return "SelectHeartColor";
        case RB_CHOICE_SELECT_NUMBER:      return "SelectNumber";
        case RB_CHOICE_SELECT_POSITION:    return "SelectPosition";
        case RB_CHOICE_SELECT_AUTO_ABILITY: return "SelectAutoAbility";
        default:                           return "Unknown";
    }
}

int test_get_blade_modifier(TestGame *tg, int cid){ return rb_mods_get_blade(&tg->state.mods, cid); }
int test_get_score_modifier(TestGame *tg, int cid){ return rb_mods_get_score(&tg->state.mods, cid); }
int test_get_cost_modifier(TestGame *tg, int cid){ return rb_mods_get_cost(&tg->state.mods, cid); }
/* Mirrors Rust GameModifiers::get_heart_modifier(card, color): the colour
   index goes straight into rb_mods_get_heart (modifiers.c:64), which uses it
   as the raw slot 0..7. The engine's own colour map is
 *     0=heart00 1=heart01 2=heart02 3=heart03 4=heart04 5=heart05 6=heart06 7=all
   (state.c:2002 rb_heart_color_index, rb_heart_color_short_label), so in the
   RbHeartColor enum RB_HEART_PURPLE == 5 is heart05 and RB_HEART_ORANGE == 6
   is heart06.

   This shim used to carry `if (color == 5) color = RB_HEART_ORANGE;`. That
   made every heart05 read return the heart06 slot, so a suite asking for
   heart05 while heart06 happened to hold the value got a clean green, and a
   suite asking for heart05 while heart05 held it got a red it could not
   explain (test_target_selection.c:77, test_trigger_paths.c:449,
   test_tp_zone_change_gate.c:252 are the call sites that were silently
   measuring the wrong slot). The remap is gone: this is a passthrough. */
int test_get_heart_modifier(TestGame *tg, int cid, int color){
     int value = rb_mods_get_heart(&tg->state.mods, cid, color);
    fprintf(stderr, "[TEST_HEART_READ] cid=%d color=%d value=%d\n", cid, color, value);
    return value;
}
void test_answer_play_cost_choice(TestGame *tg, int accept){
    if (tg->state.ptc_active) {
        rb_complete_play_with_cost(&tg->state, 0, accept);
        return;
    }
    /* No paused alt-cost play: fall back to the generic choice resume. */
    rb_resume_with_choice(&tg->state, accept ? 1 : 0);
}
/* Default filler card id (mirrors per-module `fn filler_hand(game)` helpers that
   return a common SD filler). Used only where the helper call appears inline. */
int test_filler_hand(TestGame *tg){ return rb_find_card_by_no("PL!-sd1-010-SD"); }

/* ── Rust `fill_decks` (engine/tests/helpers/mod.rs:564-571) ──────────────────
 *
 *     pub fn fill_decks(game: &mut TestGame, filler: i16) {
 *         game.state.player1.main_deck.cards.clear();
 *         game.state.player2.main_deck.cards.clear();
 *         for _ in 0..30 {
 *             game.state.player1.main_deck.cards.push(filler);
 *             game.state.player2.main_deck.cards.push(filler);
 *         }
 *     }
 *
 * MISSING FROM THE C until the Q4 audit, and the gap was a NAME TRAP rather
 * than an omission: the C has `test_filler_hand`, which RETURNS a filler card id
 * and mutates nothing. Nothing else in the C was named like `fill_decks`, so a
 * transpiled `fill_decks(game, filler)` had nowhere to land and a porter reading
 * `test_filler_hand` could reasonably have read it as "fill the hand" — it does
 * not, it returns one id. 30 per seat, both seats, and both decks CLEARED first:
 * the clear is the part that matters, because a deck the fixture meant to be
 * empty otherwise keeps whatever the setup put there.
 */
void test_fill_decks(TestGame *tg, int filler){
    if(!tg) return;
    for(int p=0;p<2;p++){
        RbPlayer *P=&tg->state.p[p];
        P->deck.n=0;
        for(int i=0;i<30;i++) P->deck.cards[i]=filler;
        P->deck.n=30;
    }
}
/* ── Rust `fill_energy_deck` (mod.rs:577-583) ────────────────────────────────
 *
 *     pub fn fill_energy_deck(game: &mut TestGame, player: u8, count: usize) {
 *         let energy = card_id(&db, "LL-E-001-SD");
 *         for _ in 0..count { deck.cards.push(energy); }
 *     }
 *
 * Same MISSING-twin story as fill_decks. `test_add_to_energy_deck` exists and is
 * the per-card push, so a porter could have written the loop — but the card id
 * is resolved HERE, once, from the printed number "LL-E-001-SD", and `count` is
 * the argument. Note this does NOT raise `energy_active`: it fills the ENERGY
 * DECK (the draw pile), not the energy zone, so a suite that wants active energy
 * must still call test_give_energy_for. */
void test_fill_energy_deck(TestGame *tg, int pl, int count){
    if(!tg || pl<0 || pl>1 || count<0) return;
    int eid = rb_find_card_by_no("LL-E-001-SD");
    if(eid<0) return;
    RbPlayer *P=&tg->state.p[pl];
    for(int i=0;i<count;i++){
        if(P->energy_deck.n >= RB_MAX_ZONE) return;
        P->energy_deck.cards[P->energy_deck.n++]=eid;
    }
}
/* ── Rust `stage_occupant` (mod.rs:440-444) ──────────────────────────────────
 *
 *     pub fn stage_occupant(&self, card_id: i16, area: MemberArea) -> Option<i16> {
 *         let slot = *self.state.player1.stage.stage.get(idx)?;
 *         (slot != -1 && slot != card_id).then_some(slot)
 *     }
 *
 * The card in `area` of OUR stage, EXCLUDING the card being played. P1 only,
 * exactly as Rust.
 *
 * This is not a convenience. Rust's own doc says it "lets a test assert its own
 * setup, which is what would have caught §8.3's vacuous negative" — a test that
 * staged an AZALEA member and then played into the same area baton-touched the
 * very card its negative was written about, and passed. The C inlines the same
 * comparison inside test_play_to_stage's diagnostic, so the CHECK exists, but a
 * test could not CALL it: 137 tests had the hazard and none of them could ask.
 * Returns -1 for "no occupant that is not the card being played", matching
 * Rust's `None`.
 */
int test_stage_occupant(TestGame *tg, int card_id, int area){
    if(!tg || area<0 || area>=RB_STAGE_SIZE) return -1;
    int slot = tg->state.p[0].stage[area];
    if(slot == RB_EMPTY_SLOT || slot == card_id) return -1;
    return slot;
}

/* Collection-predicate helpers mirroring the Rust tests' ubiquitous
   `zone.cards.iter().any(|c| c.card_no == "X")` / `.contains(&id)` patterns,
   which a line-based transpiler cannot emit directly. */
static RbBag *zone_bag(TestGame *tg, int pl, const char *zone){
    /* `pl` is validated HERE so no caller can index p[] out of range. */
    if(!tg || !zone || pl<0 || pl>1) return NULL;
    RbPlayer *P=&tg->state.p[pl];
    if(!strcmp(zone,"hand")) return &P->hand;
    if(!strcmp(zone,"deck")||!strcmp(zone,"main_deck")) return &P->deck;
    if(!strcmp(zone,"discard")||!strcmp(zone,"waitroom")) return &P->discard;
    if(!strcmp(zone,"live")||!strcmp(zone,"live_card_zone")) return &P->live;
    if(!strcmp(zone,"success")||!strcmp(zone,"success_live_card_zone")) return &P->success;
    if(!strcmp(zone,"energy")||!strcmp(zone,"energy_zone")) return &P->energy;
    if(!strcmp(zone,"energy_deck")) return &P->energy_deck;
    return NULL;
}
/* The i-th card of `zone` for seat `pl`, or -1 when out of range / unknown
   zone. One accessor, so test_zone_len / test_zone_ids /
   test_zone_count_of_id / test_zone_has_id / test_zone_has_card_no can never
   disagree about what a zone contains. `stage` skips empty slots; `under<0..2>`
   addresses the per-member under-card bag. */
static int zone_card_at(TestGame *tg, int pl, const char *zone, int i){
    if(!tg || !zone || pl<0 || pl>1 || i<0) return -1;
    RbPlayer *P=&tg->state.p[pl];
    if(!strcmp(zone,"stage")){
        int seen=0;
        for(int a=0;a<RB_STAGE_SIZE;a++){
            if(P->stage[a]==RB_EMPTY_SLOT) continue;
            if(seen++==i) return P->stage[a];
        }
        return -1;
    }
    if(!strncmp(zone,"under",5) && zone[5]){
        int a=0; const char *d=zone+5;
        while(*d>='0' && *d<='9'){ a=a*10+(*d-'0'); d++; }
        if(*d || a<0 || a>=RB_STAGE_SIZE) return -1;
        RbBag *b=&P->under_cards[a];
        return i<b->n ? b->cards[i] : -1;
    }
    RbBag *b=zone_bag(tg,pl,zone);
    if(!b || i>=b->n) return -1;
    return b->cards[i];
}
int test_zone_len(TestGame *tg, int pl, const char *zone){
    int n=0;
    while(n<RB_MAX_ZONE && zone_card_at(tg,pl,zone,n)>=0) n++;
    return n;
}
int test_zone_ids(TestGame *tg, int pl, const char *zone, int *out, int max){
    if(!out || max<=0) return 0;
    int n=0;
    while(n<max){
        int c=zone_card_at(tg,pl,zone,n);
        if(c<0) break;
        out[n++]=c;
    }
    return n;
}
int test_zone_count_of_id(TestGame *tg, int pl, const char *zone, int id){
    int n=0;
    for(int i=0;i<RB_MAX_ZONE;i++){
        int c=zone_card_at(tg,pl,zone,i);
        if(c<0) break;
        if(c==id) n++;
    }
    return n;
}
int test_total_card_count(TestGame *tg){
    /* Zone-independent census: every card slot of BOTH seats. A move between
       zones must leave this unchanged; a drain that duplicates or drops a card
       changes it. */
    static const char *const zones[] = {
        "hand","main_deck","waitroom","live","success","energy","energy_deck"
    };
    if(!tg) return 0;
    int total=0;
    for(int pl=0;pl<2;pl++){
        total += test_zone_len(tg,pl,"stage");
        for(int a=0;a<RB_STAGE_SIZE;a++){
            char z[16];
            snprintf(z,sizeof(z),"under%d",a);
            total += test_zone_len(tg,pl,z);
        }
        for(size_t i=0;i<sizeof(zones)/sizeof(zones[0]);i++)
            total += test_zone_len(tg,pl,zones[i]);
    }
    return total;
}
/* EXACT card-number comparison -- never the forgiving rb_find_card_by_no
   chain. The comparison itself was always an exact strcmp; the only hole was
   the empty string. rb_card_string returns the SHARED "" literal for an
   out-of-range index (data.c:328), so rb_card_no_eq(id, "") answered "yes"
   for every card in the database -- a permanent false green for any
   "this zone holds no card numbered X" assertion written with X empty.
   Refused explicitly. */
int rb_card_no_eq(int card_id, const char *no){
    if(!no || !no[0] || card_id<0) return 0;
    Card c;
    if(!rb_decode_card_by_index((uint32_t)card_id,&c)) return 0;
    const char *cn = rb_card_string(c.card_no_idx);
    int eq = (cn && cn[0] && strcmp(cn, no)==0);
    rb_free_card(&c);   /* the decoded Card is heap-owned */
    return eq;
}
int test_zone_has_card_no(TestGame *tg, int pl, const char *zone, const char *no){
    if(!no) return 0;
    for(int i=0;i<RB_MAX_ZONE;i++){
        int c=zone_card_at(tg,pl,zone,i);
        if(c<0) return 0;
        if(rb_card_no_eq(c,no)) return 1;
    }
    return 0;
}
int test_zone_has_id(TestGame *tg, int pl, const char *zone, int id){
    for(int i=0;i<RB_MAX_ZONE;i++){
        int c=zone_card_at(tg,pl,zone,i);
        if(c<0) return 0;
        if(c==id) return 1;
    }
    return 0;
}

/* ── cost / selection zone moves (see the header for the scope note) ── */
static int bag_take_id(RbBag *b, int id){
    for(int i=0;i<b->n;i++){
        if(b->cards[i]!=id) continue;
        for(int k=i;k<b->n-1;k++) b->cards[k]=b->cards[k+1];
        b->n--;
        return 1;
    }
    return 0;
}
int test_move_hand_to_waitroom(TestGame *tg, int pl, int n){
    if(!tg || pl<0 || pl>1) return 0;
    RbPlayer *P=&tg->state.p[pl];
    int moved=0;
    for(int i=0;i<n && P->hand.n>0;i++){
        int id=P->hand.cards[0];
        rb_hand_remove_card(P,0);
        rb_waitroom_add(P,id);
        moved++;
    }
    return moved;
}
int test_move_ids_to_waitroom(TestGame *tg, int pl, const int *ids, int n){
    if(!tg || !ids || n<=0 || pl<0 || pl>1) return 0;
    RbPlayer *P=&tg->state.p[pl];
    int moved=0;
    for(int i=0;i<n;i++){
        int id=ids[i];
        if(id<0) continue;
        /* hand, then live, then success: the zones a cost / selection pick can
           legitimately come from. */
        if(bag_take_id(&P->hand,id) || bag_take_id(&P->live,id) || bag_take_id(&P->success,id)){
            rb_waitroom_add(P,id);
            moved++;
        }
    }
    return moved;
}

/* ═══════════════════════════════════════════════════════════════════════════
 * PHASE ADVANCE
 * ═══════════════════════════════════════════════════════════════════════════ */

/* Why there is more than one advance helper
 * -----------------------------------------
 * rb_advance_phase (src/turn/phase.c:37) is ONE step, and it is a no-op in
 * three situations the caller must know about:
 *   1. `if (rb_has_pending_choice(g)) return;`            phase.c:39
 *      A standing prompt FREEZES the phase machine. This is Rust-faithful
 *      (the engine is mid-question), and it is why "just pass 8 times" is
 *      not a phase advance: 3 of those 8 passes did nothing.
 *   2. ENERGY -> DRAW does NOT draw. phase.c:52-56 runs rb_draw_energy and
 *      only *sets* RB_PHASE_DRAW; the rb_draw(g, g->active) that gives the
 *      seat its card runs on the NEXT call, from the DRAW arm (phase.c:57).
 *      So "advance to Draw" and "the seat has its card" are two different
 *      numbers of passes apart, and a test that assumes otherwise measures
 *      the deck one step early.
 *   3. MAIN flips `active` to `second_attacker` and goes back to ACTIVE
 *      (phase.c:71-73). The Active/Energy/Draw triple that follows is
 *      therefore served to the OTHER seat -- the first draw of a round lands
 *      on second_attacker, not on first_attacker. Any helper that quietly
 *      "fixed" this would be lying about the engine, so it is REPORTED
 *      instead (TestPhaseAdvance.in_swap_window), never papered over.
 *
 * And the phase enum does not line up with Rust's. Rust's `Phase`
 * (engine/src/core/types.rs:137) names FOUR phases inside the live window:
 * LiveCardSetFirstAttacker, LiveCardSetSecondAttacker,
 * FirstAttackerPerformance, SecondAttackerPerformance. The C `RbPhase`
 * (include/rabuka.h) ALSO names all four, and phase.c now STORES all four
 * (phase.c:131 / :151 / :172 / :1158), so a literal compare is the faithful
 * test for three of the four. Which of the pair-resolving arms below are still
 * needed was MEASURED, not assumed:
 *
 *   - RB_PHASE_PERFORMANCE_SECOND is DEAD. RB_PHASE_PERFORMANCE is stored only
 *     on the tail of the LIVE_SET_SECOND arm (phase.c:172), which immediately
 *     precedes the first performance with `active == first_attacker`, and the
 *     second window stores RB_PHASE_PERFORMANCE_SECOND (phase.c:1158) with
 *     `active == second_attacker`. A walk of the whole live window never
 *     observes (PERFORMANCE, second_attacker). Collapsed to `default`.
 *
 *   - RB_PHASE_LIVE_SET_SECOND is STILL REACHABLE and its arm is KEPT.
 *     rb_advance_phase stores LIVE_SET_SECOND properly (phase.c:151), but the
 *     live-card SELECTION handlers do not: rb_handle_live_card_confirmation
 *     (phase.c:988-992) and rb_handle_live_card_skip (phase.c:1001-1002) both
 *     do `g->active = g->second_attacker` and leave `g->phase` on
 *     RB_PHASE_LIVE_SET. That is exactly the folded state this arm matches, and
 *     it is the state a test sits in between confirming the first attacker's
 *     live cards and advancing. Dropping the arm would make
 *     test_advance_to_phase(RB_PHASE_LIVE_SET_SECOND) — used by
 *     test_parity_restriction — stop matching there. phase.c is not this
 *     file's to fix; the arm is the thing standing in for it.
 */

/* Is the game currently standing at the Rust phase `target` names?
   RB_PHASE_PERFORMANCE_SECOND and every other stored value are a plain
   g->phase compare (see the note above for which arms survive and why);
   RB_PHASE_LIVE_SET_SECOND still needs its pair-resolving arm because the
   live-card selection handlers flip `active` without storing the phase, and
   RB_PHASE_MULLIGAN_* genuinely store either mulligan value or OPENING. */
static int phase_is(TestGame *tg, int target){
    switch (target) {
        case RB_PHASE_LIVE_SET_SECOND:
            return tg->state.phase == RB_PHASE_LIVE_SET ||
                   tg->state.phase == RB_PHASE_LIVE_SET_SECOND;
        /* The C machine folds RPS and OPENING into one arm and treats both
           as a step straight to ACTIVE (phase.c:41-44); the mulligan phases
           are documented no-ops for a headless host. Compare against the
           stored value so a caller asking for OPENING still finds it. */
        case RB_PHASE_MULLIGAN_FIRST:
        case RB_PHASE_MULLIGAN_SECOND:
            return tg->state.phase == RB_PHASE_MULLIGAN_FIRST ||
                   tg->state.phase == RB_PHASE_MULLIGAN_SECOND ||
                   tg->state.phase == RB_PHASE_OPENING;
        default:
            return tg->state.phase == target;
    }
}

/* Rust Phase Display names (types.rs:153-171), plus the C RbPhase spellings,
   so a transpiled `advance_to_phase(Phase::SecondAttackerPerformance)` and a
   hand-written one resolve to the same value. Returns -1 for unknown. */
int test_phase_from_name(const char *name){
    if(!name) return -1;
    static const struct { const char *n; int v; } tbl[] = {
        {"RPS", RB_PHASE_RPS}, {"RockPaperScissors", RB_PHASE_RPS},
        {"ChooseFirstAttacker", RB_PHASE_RPS},
        {"Opening", RB_PHASE_OPENING}, {"OPENING", RB_PHASE_OPENING},
        {"MulliganFirstAttacker", RB_PHASE_MULLIGAN_FIRST},
        {"MulliganSecondAttacker", RB_PHASE_MULLIGAN_SECOND},
        {"Active", RB_PHASE_ACTIVE}, {"ACTIVE", RB_PHASE_ACTIVE},
        {"Energy", RB_PHASE_ENERGY}, {"ENERGY", RB_PHASE_ENERGY},
        {"Draw", RB_PHASE_DRAW}, {"DRAW", RB_PHASE_DRAW},
        {"Main", RB_PHASE_MAIN}, {"MAIN", RB_PHASE_MAIN},
        {"LiveCardSetFirstAttacker", RB_PHASE_LIVE_SET},
        {"LiveCardSet", RB_PHASE_LIVE_SET}, {"LIVE_SET", RB_PHASE_LIVE_SET},
        {"LiveCardSetSecondAttacker", RB_PHASE_LIVE_SET_SECOND},
        {"FirstAttackerPerformance", RB_PHASE_PERFORMANCE},
        {"Performance", RB_PHASE_PERFORMANCE}, {"PERFORMANCE", RB_PHASE_PERFORMANCE},
        {"SecondAttackerPerformance", RB_PHASE_PERFORMANCE_SECOND},
        {"LiveVictoryDetermination", RB_PHASE_VICTORY},
        {"Victory", RB_PHASE_VICTORY}, {"VICTORY", RB_PHASE_VICTORY},
        {"Done", RB_PHASE_DONE}, {"DONE", RB_PHASE_DONE},
    };
    for(size_t i=0;i<sizeof(tbl)/sizeof(tbl[0]);i++)
        if(!strcmp(name, tbl[i].n)) return tbl[i].v;
    return -1;
}

/* Rust Display name for the phase the game is in right now, resolving the
   folded pairs the way phase_is() does. For logs and FAIL messages only. */
const char *test_phase_name_now(TestGame *tg){
    if(!tg) return "?";
    int p = tg->state.phase;
    if (p == RB_PHASE_LIVE_SET)
        return (tg->state.active == tg->state.second_attacker)
             ? "LiveCardSetSecondAttacker" : "LiveCardSetFirstAttacker";
    if (p == RB_PHASE_PERFORMANCE)
        return (tg->state.active == tg->state.second_attacker)
             ? "SecondAttackerPerformance" : "FirstAttackerPerformance";
    switch (p) {
        case RB_PHASE_RPS: return "RPS";
        case RB_PHASE_OPENING: return "Opening";
        case RB_PHASE_MULLIGAN_FIRST: return "MulliganFirstAttacker";
        case RB_PHASE_MULLIGAN_SECOND: return "MulliganSecondAttacker";
        case RB_PHASE_ACTIVE: return "Active";
        case RB_PHASE_ENERGY: return "Energy";
        case RB_PHASE_DRAW: return "Draw";
        case RB_PHASE_MAIN: return "Main";
        case RB_PHASE_VICTORY: return "LiveVictoryDetermination";
        case RB_PHASE_DONE: return "Done";
        default: return "?";
    }
}

static void phase_advance_snapshot(TestGame *tg, TestPhaseAdvance *r){
    r->active         = tg->state.active;
    r->first_attacker = tg->state.first_attacker;
    r->second_attacker= tg->state.second_attacker;
    r->winner         = tg->state.winner;
    /* The seat-swap landmine, surfaced rather than hidden: after the first
       MAIN of a round `active` has become second_attacker, so the next
       Active/Energy/Draw triple -- including the round's first draw -- is
       served to the OTHER seat. Assert on this flag if a test cares whose
       deck was drawn. */
    r->in_swap_window = (tg->state.active != tg->state.first_attacker);
}

/* THE helper. Steps the turn toward `target` ONE PHASE AT A TIME and STOPS
   THE MOMENT A CHOICE IS STANDING. It never answers a prompt.

   Why it stops instead of answering: the phase being stepped INTO is
   usually the window whose effects raise the prompt (a ライブ開始時 position
   change, a ライブ成功時 draw), so draining it here would consume the exact
   choice the test came to inspect. Rust says the same thing in as many words
   (engine/tests/helpers/mod.rs:545-551): "hand any prompt raised by this
   step to the CALLER ... draining it here would consume the very choice the
   test came to inspect."

   Returns one of the TestPhaseStop codes and fills `*out` when asked. */
int test_advance_to_phase_strict_ex(TestGame *tg, int target, int max_passes,
                                    TestPhaseAdvance *out){
    TestPhaseAdvance r;
    memset(&r,0,sizeof(r));
    if(!tg){ if(out) *out = r; return TEST_PHASE_STOPPED_BAD_TARGET; }

    if(max_passes <= 0) max_passes = 16;   /* Rust's own bound (mod.rs:539) */

    if(phase_is(tg, target)){ r.reached = 1; goto done; }

    for(int i=0;i<max_passes;i++){
        int phase_before = tg->state.phase;
        int active_before= tg->state.active;
        rb_advance_phase(&tg->state);
        r.passes++;
        if(tg->state.phase==phase_before && tg->state.active==active_before)
            r.noops++;
        if(phase_is(tg, target)){ r.reached = 1; break; }
        /* A prompt raised by the step we just took is the CALLER's. Stop. */
        if(rb_has_pending_choice(&tg->state)) break;
        /* A decided game stops advancing (phase.c:38). Anything further would
           spin to max_passes reporting a stuck machine. */
        if(tg->state.winner != -1) break;
    }
done:
    /* Both flags can be true: a single step can arrive at the target AND
       leave a prompt standing. That is the Rust "Arrived: hand any prompt
       raised by this step to the CALLER" case, not a contradiction. */
    r.prompt = rb_has_pending_choice(&tg->state) ? 1 : 0;
    phase_advance_snapshot(tg, &r);
    if(out) *out = r;
    if(r.reached)      return TEST_PHASE_STOPPED_REACHED;
    if(r.prompt)       return TEST_PHASE_STOPPED_PROMPT;
    if(r.winner != -1) return TEST_PHASE_STOPPED_GAME_OVER;
    return TEST_PHASE_STOPPED_TIMEOUT;
}

int test_advance_to_phase_strict(TestGame *tg, int target){
    return test_advance_to_phase_strict_ex(tg, target, 0, NULL) == TEST_PHASE_STOPPED_REACHED;
}

/* Same walk, addressed by Rust's phase NAME. */
int test_advance_to_phase_named(TestGame *tg, const char *phase_name){
    int target = test_phase_from_name(phase_name);
    if(target < 0){
        fprintf(stderr, "[TEST_PHASE] unknown phase name \"%s\"\n",
                phase_name ? phase_name : "(null)");
        return 0;
    }
    return test_advance_to_phase_strict(tg, target);
}

/* Value -> the canonical Rust Phase name, the inverse of
   test_phase_from_name(). "?" for a value that names no phase. */
const char *test_phase_to_name(int phase){
    switch (phase) {
        case RB_PHASE_RPS: return "RPS";
        case RB_PHASE_OPENING: return "Opening";
        case RB_PHASE_MULLIGAN_FIRST: return "MulliganFirstAttacker";
        case RB_PHASE_MULLIGAN_SECOND: return "MulliganSecondAttacker";
        case RB_PHASE_ACTIVE: return "Active";
        case RB_PHASE_ENERGY: return "Energy";
        case RB_PHASE_DRAW: return "Draw";
        case RB_PHASE_MAIN: return "Main";
        case RB_PHASE_LIVE_SET: return "LiveCardSetFirstAttacker";
        case RB_PHASE_LIVE_SET_SECOND: return "LiveCardSetSecondAttacker";
        case RB_PHASE_PERFORMANCE: return "FirstAttackerPerformance";
        case RB_PHASE_PERFORMANCE_SECOND: return "SecondAttackerPerformance";
        case RB_PHASE_VICTORY: return "LiveVictoryDetermination";
        case RB_PHASE_DONE: return "Done";
        default: return "?";
    }
}

/* Diagnostics for a failed advance. Always prints -- a caller that ignores
   the return value still gets told WHERE it stopped and whether the seat it
   is looking at is the swapped-in one. */
void test_advance_report(TestGame *tg, const char *what, int target,
                         int stop){
    if(!tg) return;
    fprintf(stderr,
            "[TEST_PHASE] %s: stop=%d (%s%s%s%s) now=%s want=%s "
            "phase=%d active=%d first=%d second=%d swap_window=%d pending=%d "
            "winner=%d\n",
            what ? what : "advance", stop,
            stop==TEST_PHASE_STOPPED_REACHED ? "reached" : "",
            stop==TEST_PHASE_STOPPED_PROMPT ? "prompt" : "",
            stop==TEST_PHASE_STOPPED_TIMEOUT ? "timeout" : "",
            stop==TEST_PHASE_STOPPED_GAME_OVER ? "game-over" : "",
            test_phase_name_now(tg), test_phase_to_name(target),
            tg->state.phase, tg->state.active, tg->state.first_attacker,
            tg->state.second_attacker,
            tg->state.active != tg->state.first_attacker,
            rb_has_pending_choice(&tg->state) ? 1 : 0, tg->state.winner);
}

/* Faithful port of Rust TestGame::advance_to_phase
   (engine/tests/helpers/mod.rs:538-561), for the ported tests whose setup
   genuinely depends on the auto-drain. It is NOT the default and it is
   genuinely destructive:

     while has_pending_choice() { select_indices(&[0]) }        mod.rs:553-555

   answers EVERY prompt raised mid-walk with option 0. Inside the live window
   that eats the YELL: rb_performance runs yell, 自動, allocation, score,
   live-success and victory inside a single pass (phase.c:112-124 ->
   rb_execute_performance_phase -> rb_perform_live), so option 0 is not "the
   obvious answer", it is an answer to a question the test never saw. Prefer
   test_advance_to_phase_strict() unless you are transcribing a Rust test
   whose assertions were written against the drained walk. */
int test_advance_to_phase_rust(TestGame *tg, int target){
    if(!tg) return 0;
    for(int i=0;i<16;i++){
        if(phase_is(tg, target)) return 1;
        test_pass(tg);
        if(phase_is(tg, target)) return 1;   /* mod.rs:544-552, prompt kept */
        int guard=0;
        while(rb_has_pending_choice(&tg->state) && guard++<64){
            int zero = 0;
            rb_resume_with_choice_indices(&tg->state, &zero, 1);
        }
    }
    return 0;
}

/* LEGACY, and the reason the strict helpers above exist. Kept
   bit-for-bit in behaviour so no shipped suite moves; DO NOT BUILD ON IT.

   test_advance_to_phase(RB_PHASE_PERFORMANCE_SECOND) can never arrive --
   RB_PHASE_PERFORMANCE_SECOND is never stored in g->phase (phase.c:1070
   reuses RB_PHASE_PERFORMANCE for both windows) -- so it fell through all
   16 iterations, auto-answering EVERY prompt it met on the way, yell
   included. That is a live false-red generator for any suite that steps to
   the second performance, and it is why ~20 ported tests were parked rather
   than written. Use test_advance_to_phase_strict(). */
int test_advance_to_phase(TestGame *tg, int target){
    if(!tg) return 0;
    for(int i=0;i<16;i++){
        if(tg->state.phase==target) return 1;
        test_pass(tg);
        if(tg->state.phase==target) return 1;
        int guard=0;
        while(rb_has_pending_choice(&tg->state) && guard++<64)
            rb_resume_with_choice(&tg->state, 0);
    }
    return 0;
}

int test_deck_len(TestGame *tg){ return tg->state.p[0].deck.n; }
int test_hand_len(TestGame *tg){ return tg->state.p[0].hand.n; }

/* ── queue-entry seat introspection ──────────────────────────────────────
   A Rust test asserts things like
       assert_eq!(entry.choice_player_id.as_deref(), Some("p2"));
   whose C stand-in is `strcmp(game.queue.entries[i].choice_player_id, "p2")`.
   Two suites hand-rolled exactly that compare, in two different shapes, and
   neither could report the EMPTY-string case cleanly (an empty token compared
   equal to "p1" only by accident of never being asserted at all). These
   accessors give the comparison one typed spelling:

     - test_queue_entry_seat / test_queue_entry_choice_seat  -> 0 (p1), 1 (p2),
       or -1 for "empty or unrecognised". -1 is a DISTINCT answer from 0, so a
       test can assert the defect an un-stamped live-start entry causes.
     - the *_owned_by_seat predicates for the plain "is it this seat?" question.
     - the *_player_id readers for a test that wants the raw string.

   The long form is normalised here exactly as Rust's
   build_ability_queue_entry does (engine/src/core/game_state/abilities.rs
   :240-246: "player1" -> "p1", "player2" -> "p2"), so a test never has to
   re-implement that mapping. Layout of RbQueueEntry is untouched. */
const char *test_seat_id(int pl){
    if (pl == 0) return "p1";
    if (pl == 1) return "p2";
    return "";
}
/* Map a queue-entry player token onto a seat index. Returns -1 for NULL, "" and
   anything unrecognised, so "unstamped" and "seat 0" are never confused. */
static int seat_of_token(const char *tok){
    if(!tok) return -1;
    if(!strcmp(tok,"p1")||!strcmp(tok,"player1")) return 0;
    if(!strcmp(tok,"p2")||!strcmp(tok,"player2")) return 1;
    return -1;
}
int test_queue_n_entries(TestGame *tg){ return tg ? tg->state.queue.n_entries : 0; }
int test_queue_entry_seat(TestGame *tg, int idx){
    if(!tg || idx<0 || idx>=tg->state.queue.n_entries) return -1;
    return seat_of_token(tg->state.queue.entries[idx].player_id);
}
int test_queue_entry_choice_seat(TestGame *tg, int idx){
    if(!tg || idx<0 || idx>=tg->state.queue.n_entries) return -1;
    return seat_of_token(tg->state.queue.entries[idx].choice_player_id);
}
/* Copy the entry's owner token into `buf`. Returns 1 when the index addresses a
   real entry, 0 otherwise; `buf` is always NUL-terminated and cleared first, so
   a failed read yields "" rather than a stale value. */
static int queue_entry_id_copy(TestGame *tg, int idx, const char *field,
                               char *buf, size_t buf_len){
    if(!buf || buf_len==0) return 0;
    buf[0]='\0';
    if(!tg || !field || idx<0 || idx>=tg->state.queue.n_entries) return 0;
    snprintf(buf, buf_len, "%s", field);
    return 1;
}
int test_queue_entry_player_id(TestGame *tg, int idx, char *buf, size_t buf_len){
    if(!tg || idx<0 || idx>=tg->state.queue.n_entries){
        if(buf && buf_len) buf[0]='\0';
        return 0;
    }
    return queue_entry_id_copy(tg, idx, tg->state.queue.entries[idx].player_id, buf, buf_len);
}
int test_queue_entry_choice_player_id(TestGame *tg, int idx, char *buf, size_t buf_len){
    if(!tg || idx<0 || idx>=tg->state.queue.n_entries){
        if(buf && buf_len) buf[0]='\0';
        return 0;
    }
    return queue_entry_id_copy(tg, idx, tg->state.queue.entries[idx].choice_player_id, buf, buf_len);
}
int test_queue_entry_owned_by_seat(TestGame *tg, int idx, int pl){
    if(pl<0 || pl>1) return 0;
    return test_queue_entry_seat(tg, idx) == pl;
}
int test_queue_entry_choice_owned_by_seat(TestGame *tg, int idx, int pl){
    if(pl<0 || pl>1) return 0;
    return test_queue_entry_choice_seat(tg, idx) == pl;
}

/* ── distinct-name count ──────────────────────────────────────────────────
   rb_max_distinct_names is defined in src/ability/util.c but is NOT declared in
   include/rabuka.h, so a test that wanted Rust's max_distinct_names had to
   settle for the declared sibling rb_count_distinct_member_name_units and
   measure something else. The header is not this file's to edit, so declare
   the symbol here and expose it through the test header instead. */
extern int rb_max_distinct_names(const int *cards, int n);
int test_max_distinct_names(const int *cards, int n){
    if(!cards || n<=0) return 0;
    return rb_max_distinct_names(cards, n);
}

/* ═══════════════════════════════════════════════════════════════════════════
 * FIRE ONE ABILITY ON ONE CARD  (Rust helpers/mod.rs:114 `fire_trigger`)
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * WHAT THIS REPLACES, AND WHY THE OLD SPELLING WAS WRONG
 * ----------------------------------------------------
 * Thirty-six C suites each carried a local `fire_trigger` with this shape:
 *
 *     static int fire_trigger(TestGame *tg, int cid, const char *trig) {
 *         if (!card_prints_trigger(cid, trig)) return 0;
 *         tg->state.activating_card = cid;
 *         rb_queue_trigger_abilities(&tg->state, 0, trig);   <-- fans out
 *         rb_process_pending_auto_abilities(&tg->state);
 *     }
 *
 * `cid` was used ONLY to set `activating_card`. The ability that actually fired
 * was chosen by `rb_queue_trigger_abilities`, which is a whole-board TAS scan
 * (src/core/game_state_abilities.c:2734): it walks P1's stage, success zone,
 * live zone, hand, energy zone and the recently-moved batch and enqueues EVERY
 * ability that prints `trig`. So whenever a second card in the fixture printed
 * the same trigger, BOTH fired, and every discard / draw / score effect ran
 * twice. Rust's helper cannot do that: it resolves ONE `ability_id` for ONE
 * card and calls `trigger_auto_ability` with an explicit card id.
 *
 * THE ENGINE API USED
 * -------------------
 * `rb_trigger_auto_ability` (src/core/game_state_abilities.c:1573), which is
 * the port of Rust `GameState::trigger_auto_ability`. It is the ONE existing
 * entry point that takes a single `ability_id` + `explicit_card_id`, and its
 * body is faithful: it re-checks the trigger match and the invalidation gate,
 * and it enqueues through the same `rb_build_ability_queue_entry` the TAS scan
 * uses, so the owner stamp, the entry trigger token and the drain all behave
 * identically. No new engine entry point was needed and none was added.
 * It is not declared in include/rabuka.h (that header is not this file's to
 * edit), so it is declared `extern` here exactly as rb_max_distinct_names is.
 *
 * `trigger_moved_cards` is passed as NULL/0 because Rust's fire_trigger passes
 * `None` there: the Rust helper snapshots NO movement batch. The old C fan-out
 * did pass `g->recently_moved`, which is one more fact than the Rust helper
 * gives the engine, so a 「それらのカード」 batch condition could satisfy here
 * and never satisfy in Rust. Suites that need an appearance event must
 * publish it themselves (see test_parity_recover_live.c's DEBUT arm).
 *
 * RETURN VALUE
 * ------------
 * 1 when exactly one queue entry was produced (so the suite's
 * `if (!fire_trigger(...)) return 0;` premise check keeps its meaning), 0 when
 * the card prints no such ability at that index or the enqueue was refused.
 */

/* Locate the nth ability of `cid` whose trigger token matches `trig`, where
 * "matches" is the ENGINE's own predicate (rb_trigger_is / abilities.rs
 * ability_matches_trigger), so a card the TAS scan would refuse is never
 * reported as printable here. Returns the absolute ability index, or -1.
 * nth < 0 is refused rather than silently treated as 0. */
int test_ability_index_for_trigger(int cid, const char *trig, int nth)
{
    if (cid < 0 || !trig || nth < 0) return -1;
    int nab = rb_card_num_abilities((uint32_t)cid);
    int seen = 0;
    for (int a = 0; a < nab; a++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        int hit = rb_trigger_is(ab.triggers, trig);
        rb_free_ability(&ab);
        if (hit && seen++ == nth) return a;
    }
    return -1;
}

/* rb_trigger_auto_ability is defined in src/core/game_state_abilities.c but is
   not declared in include/rabuka.h, which is not this file's to edit. */
extern void rb_trigger_auto_ability(GameState *g, const char *ability_id,
                                    const char *trigger_type, int player_id,
                                    const char *source_card_no,
                                    int explicit_card_id,
                                    const int *trigger_moved_cards, int n_moved,
                                    int triggering_member_id);

/* One core, three entry points. `_nth` selects the Nth PRINTED ability carrying
   the trigger (Rust fire_trigger_nth); `_at_index` addresses an ABSOLUTE
   ability index, which is what a suite isolating one leg of a multi-leg
   watcher needs and is NOT the same thing. */
int test_fire_trigger_core_seat(TestGame *tg, int cid, const char *trig,
                                int use_nth, int nth_or_index, int pl)
{
    if (!tg || cid < 0 || !trig) return 0;
    if (pl < 0 || pl > 1) return 0;
    int a = use_nth ? test_ability_index_for_trigger(cid, trig, nth_or_index)
                    : nth_or_index;
    if (a < 0) return 0;
    int nab = rb_card_num_abilities((uint32_t)cid);
    if (a >= nab) return 0;

    /* Rust: `format!("{}_{}", card.card_no, ab.full_text)`.
     *
     * BUILT ON THE HEAP, AND DELIBERATELY NOT OUT OF A FIXED BUFFER.
     * Rust's `trigger_auto_ability` takes `ability_id: String` — UNBOUNDED. It
     * never truncates, never rejects, and has no maximum. So there is no "safe"
     * size for a C stand-in: a constant is a guess about the corpus, and the
     * corpus already broke one. MEASURED over cards/abilities.json (985 unique
     * abilities, 2116 (card_no, full_text) instances): the longest key is
     * 653 bytes — PL!-bp6-006-R＋ / -P＋, full_text 635 bytes — and 38 keys
     * need more than 512. The key this helper used to build lived in
     * `char ability_id[512]`, so `snprintf` silently cut PL!-pb2-040-L
     * (card_no 13 + '_' + full_text 514 = 529 bytes) down to 511 characters,
     * `par_ability_id_matches` then compared a suffix that was not the ability's
     * text, the match FAILED, and the ability was never enqueued at all. The
     * failure was invisible because a fixed `snprintf` into a too-small buffer
     * reports success — only the length check catches it, and a check that
     * "refuses rather than firing" just converts a wrong fire into a no fire.
     * Sizing from strlen is the whole fix, and it has no upper limit to get
     * wrong.
     *
     * MEASURED max, if a future reader wants it: strlen(card_no) + 1 +
     * strlen(full_text) + 1 == 653 over the whole corpus; the longest
     * full_text alone is 635 bytes and the longest card_no is 25. */
    char *card_no = NULL;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return 0;
    const char *cn = rb_card_string(c.card_no_idx);
    if (cn) {
        size_t n_cn2 = strlen(cn);
        card_no = (char *)malloc(n_cn2 + 1);
        if (card_no) memcpy(card_no, cn, n_cn2 + 1);
    }
    rb_free_card(&c);
    if (!card_no || !card_no[0]) { free(card_no); return 0; }

    /* `ab.full_text` is already a heap string owned by the decoded Ability, so
       it is used in place rather than copied through another fixed buffer. */
    char *ability_id = NULL;
    Ability ab;
    if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) { free(card_no); return 0; }
    int print_trig = rb_trigger_is(ab.triggers, trig);
    if (ab.full_text) {
        size_t n_cn = strlen(card_no);
        size_t n_ft = strlen(ab.full_text);
        ability_id = (char *)malloc(n_cn + 1 + n_ft + 1);
        if (ability_id) {
            memcpy(ability_id, card_no, n_cn);
            ability_id[n_cn] = '_';
            memcpy(ability_id + n_cn + 1, ab.full_text, n_ft + 1);
        }
    }
    rb_free_ability(&ab);
    /* The direct-index form still refuses an ability that does not print the
       trigger: enqueuing one would fire something Rust could not have fired. */
    if (!print_trig || !ability_id) { free(ability_id); free(card_no); return 0; }

    int before = tg->state.queue.n_entries;
    rb_trigger_auto_ability(&tg->state, ability_id, trig, /*player_id*/ pl,
                            card_no, /*explicit_card_id*/ cid,
                            /*trigger_moved_cards*/ NULL, /*n_moved*/ 0,
                            /*triggering_member_id*/ -1);
    free(ability_id);
    free(card_no);
    int queued = tg->state.queue.n_entries - before;
    if (queued <= 0) return 0;
    /* NOTHING IS RECORDED HERE, AND NOW THAT IS FAITHFUL.
     *
     * Rust's `fire_trigger` (engine/tests/helpers/mod.rs:114-141) is three calls
     * and no bookkeeping: trigger_auto_ability, activating_card, then
     * process_pending_auto_abilities. It never touches
     * `turn_limited_abilities_used` and never touches the entry's
     * `use_limit_recorded` flag. `enqueue_printed_ability`
     * (abilities.rs:1282-1326) constructs the entry with
     * use_limit_recorded: false (abilities.rs:270) and records nothing either;
     * the use is spent later, from resolution, by the one and only writer of
     * the counter — `GameState::record_ability_use` (abilities.rs:216-231),
     * reached at resolver.rs:1353 (UseLimitPhase::Early, gates.rs:301-310),
     * resolver.rs:1230/:1390 and gates.rs:160/:308/:318/:331.
     *
     * This helper used to do the opposite: it pre-set `use_limit_recorded` on
     * every entry it had just produced and then called `rb_record_use`
     * unconditionally. Two measured regressions came out of that, both in the
     * same three lines.
     *
     *  1. BURNING THE USE ON A DECLINE. The record was unconditional, so a
     *     declined optional cost still consumed the turn-limited use. Rust does
     *     not: gates.rs:301-310 skips the Early record when the effect is
     *     `optional` or `conditional_on_optional`, and gates.rs:321-333's Final
     *     record then sees the same exclusions, so the ability is left with
     *     `ability_uses_used() == 0` and may fire again the same turn. MEASURED
     *     on test_parity_bp7's Q233 case (PL!SP-bp5-005-R / -S / -SEC):
     *     "a declined <effect> consumes no use of the 1-per-turn limit
     *     (got 1 expected 0)", 5 assertions across the rarity sweep.
     *
     *  2. THE ENTRY NEVER BEING GATED. The resolution-time gate in
     *     pca_resolve_entry is `if (ab->use_limit > 0 &&
     *     !entry->use_limit_recorded)` (game_state_abilities.c:2710). A
     *     pre-flagged entry skips it, so a SECOND `fire_trigger` on the same
     *     「{{ターン1回}}」 ability in the same turn was admitted and ran again;
     *     Rust rejects it in `use_limit_gate` (gates.rs:93-111), which the
     *     unflagged entry reaches. MEASURED on test_parity_cce_gaps: +2.
     *
     * Simply deleting the record is not enough on its own, and the reason is
     * worth keeping in the tree: with no record anywhere on this path the use
     * stayed unspent for the WHOLE of the effect, because pca_resolve_entry's
     * only record sat after `rb_execute_effect_ex`. The effect re-enters the
     * post-resolution scan (rb_process_player_abilities_depth_full), that scan
     * asks `ability_has_remaining_uses`, sees 0 uses and enqueues the same
     * ability a second time — the "blade 4 instead of 2" regression on
     * test_parity_state_per_card's T/010 (PL!N-sd2-010-SD2, 自動/ターン1回).
     * The fix is not a record in this harness at all: it is the Early
     * resolution-time record now sitting between the cost payment and the
     * post-cost gate in pca_resolve_entry (game_state_abilities.c), which is
     * where resolver.rs:1353 puts it. That record also reproduces Rust's
     * optional / conditional_on_optional exclusions, so the Q233 decline leaves
     * the use unspent without any special case here. */
    /* Rust order: trigger_auto_ability, THEN activating_card, THEN the drain. */
    tg->state.activating_card = cid;
    rb_process_pending_auto_abilities(&tg->state);
    return queued;
}

/* One core, several entry points. `_nth` selects the Nth PRINTED ability
   carrying the trigger (Rust fire_trigger_nth); `_at_index` addresses an
   ABSOLUTE ability index, which is what a suite isolating one leg of a
   multi-leg watcher needs and is NOT the same thing; `_seat` is for the suites
   whose Rust twin fires the trigger on the OPPONENT (test_parity_reveal_extra's
   fire_trigger_p2 and its neighbours), which pass pl == 1 to
   rb_queue_trigger_abilities. */
int test_fire_trigger_on_card_seat(TestGame *tg, int cid, const char *trig, int pl)
{
    return test_fire_trigger_core_seat(tg, cid, trig, /*use_nth*/ 1, /*nth*/ 0, pl);
}

int test_fire_trigger_on_card(TestGame *tg, int cid, const char *trig)
{
    return test_fire_trigger_core_seat(tg, cid, trig, /*use_nth*/ 1, /*nth*/ 0, /*pl*/ 0);
}

int test_fire_trigger_on_card_nth(TestGame *tg, int cid, const char *trig, int nth)
{
    return test_fire_trigger_core_seat(tg, cid, trig, /*use_nth*/ 1, nth, /*pl*/ 0);
}

int test_fire_trigger_at_index(TestGame *tg, int cid, int ability_idx)
{
    /* The trigger token is read back off the ability itself so the entry is
       stamped with the printed trigger (rb_queue_enqueue does that for us). */
    if (!tg || cid < 0 || ability_idx < 0) return 0;
    int nab = rb_card_num_abilities((uint32_t)cid);
    if (ability_idx >= nab) return 0;
    Ability ab;
    memset(&ab, 0, sizeof ab);
    if (!rb_decode_card_ability((uint32_t)cid, ability_idx, &ab)) return 0;
    char trig[64];
    snprintf(trig, sizeof(trig), "%s", ab.triggers ? ab.triggers : "");
    rb_free_ability(&ab);
    if (!trig[0]) return 0;
    /* Address the index directly, but keep the "does it print the trigger"
       gate the nth path applies, by asking for the FIRST match first. */
    if (test_ability_index_for_trigger(cid, trig, 0) < 0) return 0;
    return test_fire_trigger_core_seat(tg, cid, trig, /*use_nth*/ 0, ability_idx, /*pl*/ 0);
}

/* ── FLAT-suite per-test verdict layer ─────────────────────────────────────
 *
 * A FLAT suite runs every test body in ONE process out of ONE main(), so its
 * `FAIL:` lines name an ASSERTION and nothing else. Without this layer a
 * failing test in a FLAT suite cannot be attributed to any test at all, which
 * is why "how many tests failed" was ill-defined across this corpus: hundreds
 * of FAIL lines sat in suites that named no failing test.
 *
 * `__func__` (C11 6.4.2.2) names the enclosing function, and it is the SAME
 * array object at every expansion inside one function, so pointer identity is
 * the dedupe key -- no string compare, and no duplicate verdict lines for a
 * function that fails twenty assertions.
 *
 * WHY IT LIVES HERE AND NOT PER-SUITE. An earlier harness pass pasted a private
 * copy of this function into 12 suites. That is 12 chances to spell the dedupe
 * or the flush differently, and the copies had already drifted. One definition,
 * linked into every suite, cannot drift.
 *
 * THE TRAP THIS EXISTS TO AVOID: patching `CHECK` but not `CHECK_EQ` silently
 * loses the majority of the tests, because most FLAT suites fail through
 * CHECK_EQ. Both macros in every suite that adopts this MUST call FLAT_VERDICT()
 * in their failure branch. tests/test_harness_self.c cannot police that -- it
 * has no view of a suite's macros -- so the check is the measurement: per
 * suite, the number of distinct FAILED verdicts must be non-zero whenever the
 * suite emits FAIL lines. That is a number, not a promise.
 */
static const char *g_flat_failed[2048];
static int g_flat_failed_n;

void test_flat_verdict_failed(const char *fn)
{
    if (!fn) return;
    for (int i = 0; i < g_flat_failed_n; i++)
        if (g_flat_failed[i] == fn) return;   /* __func__ is one array per function */
    if (g_flat_failed_n >= (int)(sizeof g_flat_failed / sizeof *g_flat_failed))
        return;
    g_flat_failed[g_flat_failed_n++] = fn;
    printf("FAILED: %s\n", fn);
    fflush(stdout);
}

/* How many distinct test functions have been reported failed so far. Exposed
 * so a suite's own summary can be checked against its verdict lines instead of
 * being taken on trust. */
int test_flat_failed_count(void)
{
    return g_flat_failed_n;
}
