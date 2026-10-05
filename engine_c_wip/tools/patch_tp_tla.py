import io, hashlib
P = "tests/test_tp_turn_limited_activation.c"
s = io.open(P, encoding="utf-8", newline="").read()

OLD = ''' * ENGINE GAP: the C port's action generator (rb_generate_action_candidates,
 * src/turn/phase.c) only emits Pass and PlayMemberToStage; it has no
 * UseAbility branch, so the Rust `game_setup::generate_possible_actions` offer
 * list has no C equivalent to read back. The closest real C engine behavior is
 * the same gate the Rust generator applies for a \u30bf\u30fc\u30f31\u56de activation, evaluated
 * directly against live engine state: the card sits on p1's stage, ability #0
 * is a \u8d77\u52d5 (Activate) ability, and its per-turn use limit is not yet consumed
 * (rb_ability_has_remaining_uses \u2014 abilities.rs::ability_has_remaining_uses). */'''

NEW = ''' * PREMISE, CORRECTED.  This comment used to read: "ENGINE GAP: the C port's
 * action generator (rb_generate_action_candidates, src/turn/phase.c) only emits
 * Pass and PlayMemberToStage; it has no UseAbility branch, so the Rust
 * `game_setup::generate_possible_actions` offer list has no C equivalent to read
 * back", and it substituted a hand-rolled re-implementation of the gate (on
 * stage + ability #0 is \u8d77\u52d5 + uses remaining) for the offer count.
 *
 * THAT PREMISE IS STALE.  The generator does have a UseAbility branch and the
 * offer list is readable:
 *   - `RB_SETUP_ACT_USE_ABILITY` exists (include/rabuka.h:3719) and is Rust's
 *     `ActionType::UseAbility`;
 *   - the Main-phase generator pushes one row per offered ability through
 *     `rb_setup_for_each_ability_candidate` (src/game/game_setup.c:1339), called
 *     from src/turn/phase.c:2313, and game_setup.c:1320-1324 records that bridge
 *     as added because "before this bridge existed it had no caller in the tree,
 *     so an activated ability could never be offered";
 *   - `RbGeneratedAction.parameters.card_id` is Rust's `parameters.card_id`.
 *
 * So the count below now READS the generator, exactly as Rust does.  The
 * hand-rolled gate is not merely redundant: it recomputes the per-turn use
 * limit from `rb_ability_has_remaining_uses`, which is a DIFFERENT question from
 * "did the generator emit a row", and a card whose limit is consumed can still
 * be offered while a card whose limit is free can still be withheld.  Every
 * `CHECK_EQ(count_kasumi_use_offers(...), 1)` below is therefore now a
 * measurement of the real generator and may legitimately go red where the
 * substitute was green \u2014 that red is the finding, not a regression here.
 *
 * `rb_generate_action_candidates` returns an empty list outside the mulligan /
 * Main / live-set windows (src/turn/phase.c:2185-2194), so the helper parks the
 * state in p1's Main phase first.  The returned `actions` block is malloc'd and
 * the tree exposes no free for it; these are process-lifetime test allocations. */'''

assert OLD in s, "header anchor missing"
s = s.replace(OLD, NEW, 1)

OLD_BODY = '''static int count_kasumi_use_offers(TestGame *game, int kasumi)
{
    int on_stage = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (game->state.p[0].stage[i] == kasumi) on_stage = 1;
    if (!on_stage) return 0;

    Ability ab;
    if (!rb_decode_card_ability((uint32_t)kasumi, 0, &ab)) return 0;
    int is_activate = (ab.triggers && strstr(ab.triggers, "\u8d77\u52d5") != NULL);
    rb_free_ability(&ab);
    if (!is_activate) return 0;

    if (!rb_ability_has_remaining_uses(&game->state, kasumi, 0)) return 0;
    return 1;
}'''

NEW_BODY = '''static int count_kasumi_use_offers(TestGame *game, int kasumi)
{
    /* Rust: game_setup::generate_possible_actions(&state) filtered on
     * `a.action_type == ActionType::UseAbility &&
     *  a.parameters.card_id == Some(kasumi)`; counted, not gated locally. */
    game->state.phase = RB_PHASE_MAIN;
    game->state.active = 0;
    RbGeneratedActionList list = rb_generate_action_candidates(&game->state);
    int n = 0;
    for (int i = 0; i < list.count; i++) {
        if (list.actions[i].action_type != RB_SETUP_ACT_USE_ABILITY) continue;
        if (!list.actions[i].has_parameters) continue;
        if (list.actions[i].parameters.card_id != kasumi) continue;
        n++;
    }
    return n;
}'''

assert OLD_BODY in s, "body anchor missing"
s = s.replace(OLD_BODY, NEW_BODY, 1)

with io.open(P, "r+", encoding="utf-8", newline="") as f:
    f.seek(0)
    f.write(s)
    f.truncate()
print("sha", hashlib.sha256(s.encode("utf-8")).hexdigest())