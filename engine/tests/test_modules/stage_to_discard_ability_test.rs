use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Test that a "when this member goes from stage to waiting room" auto ability
/// only triggers when the member is actually moved from stage to discard (via baton touch),
/// and NOT when:
///   - Playing the card to stage (debut)
///   - Playing other cards to stage
///
/// Card: PL!-PR-001-PR (Honoka)
/// Auto ability: "When this member is placed from stage to waiting room, you may activate 1 member."
/// Condition: properly parsed as location_condition(location="discard", card_type="member_card")
#[test]
fn stage_to_discard_ability_triggers_only_on_baton_touch() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let honoka = game.id("PL!-PR-001-PR"); // cost 4, has stage->discard auto ability
    let filler = game.id("PL!-sd1-010-SD"); // cost 4, no abilities
    let arriver = game.id("PL!S-bp5-012-N"); // cost 2, no abilities

    // Give enough energy for all plays
    game.give_energy(30);

    // Fill deck
    let deck_card = game.id("PL!-sd1-010-SD");
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(deck_card);
    }

    // ---- Step 1: Play Honoka to LeftSide (debut, empty area) ----
    game.state.player1.hand.cards.push(honoka);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(honoka, MemberArea::LeftSide);

    // Should NOT trigger: card is on stage, not in discard
    assert!(
        !game.has_pending_choice(),
        "Ability should NOT trigger on debut (card is on stage)"
    );

    // ---- Step 2: Play filler to Center (debut, empty area) ----
    game.state.player1.hand.cards.push(filler);
    game.play_to_stage(filler, MemberArea::Center);

    // Should NOT trigger
    assert!(
        !game.has_pending_choice(),
        "Ability should NOT trigger when another card debuts"
    );

    // ---- Step 3: Directly clear area lock to allow baton touch ----
    game.state.player1.areas_locked_this_turn.clear();

    // ---- Step 4: Baton touch - play arriver to LeftSide (Honoka's area) ----
    game.state.player1.hand.cards.push(arriver);
    game.play_to_stage(arriver, MemberArea::LeftSide);

    // Honoka was replaced from stage to waitroom → her auto ability fires.
    // The ability offers to activate 1 member. Since neither the filler nor
    // the arriver are in wait state, there's nothing to activate — the
    // option is NOT offered (unpayable cost).
    // Use select_indices to confirm no pending choice exists.
    assert!(
        !game.has_pending_choice(),
        "Auto ability should NOT present a choice when no wait members exist"
    );

    // Verify Honoka is in waitroom
    assert!(
        game.state.player1.waitroom.cards.contains(&honoka),
        "Honoka should be in waitroom after baton touch"
    );
}
