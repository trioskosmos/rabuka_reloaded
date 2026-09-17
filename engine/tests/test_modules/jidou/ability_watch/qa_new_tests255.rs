use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

/// Q255: When Live Start or Live Success resolves, if the member with that ability
/// has moved from the Center area, does Dancing stars on me!'s auto ability still trigger?
/// Answer: Yes.
///
/// Dancing stars on me! (PL!-bp6-020-L) has two auto abilities:
///   ab#0: When a μ's member in center resolves Live Start → position change that member
///   ab#1: When a μ's member in center resolves Live Success → if moved this turn → +1 score
///
/// This test drives the full chain: Honoka's LS resolves → ab#0 repositions her
/// out of center → her LSS resolves → ab#1's has_moved condition checks the MEMBER
/// card (not the live card's own movement) and scores +1.
///
/// Both member abilities are fired through the real ability queue (fire_trigger),
/// so the post-resolution each_time hook arms the watchers exactly like a live phase.
#[test]
fn q255_dancing_stars_live_success_after_position_change() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let dancing_stars = game.id("PL!-bp6-020-L");
    let honoka = game.id("PL!-bp6-001-R\u{ff0b}"); // μ's, has Live Start & Live Success

    // Stage: Honoka in center; Dancing stars on me! in the live card zone.
    game.state.player1.stage.stage = [-1, honoka, -1];
    game.state.player1.live_card_zone.cards.push(dancing_stars);
    game.give_energy(10);

    // --- Step 1: Honoka's Live Start resolves → ab#0 repositions her ---
    fire_trigger(
        &mut game,
        honoka,
        AbilityTrigger::LiveStart,
        "ライブ開始時",
    );
    // ab#0 asks for the destination; answer with the first generated option.
    assert!(
        game.has_pending_choice(),
        "ab#0 destination prompt expected after Honoka's LS resolved"
    );
    game.select_generated(0);
    while game.has_pending_choice() {
        assert_eq!(
            game.pending_choice_type().as_deref(),
            Some("SelectTarget"),
            "expected only position|destination prompts in the chain"
        );
        game.select_generated(0);
    }

    let center_card = game.state.player1.stage.stage[1];
    assert_ne!(
        center_card, honoka,
        "Honoka should no longer be in center after ab#0 position change"
    );
    assert!(
        game.state.has_card_moved_this_turn(honoka),
        "Honoka should be tracked as moved this turn"
    );

    // --- Step 2: Honoka's Live Success resolves → ab#1 scores ---
    fire_trigger(
        &mut game,
        honoka,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    assert!(
        !game.has_pending_choice(),
        "no prompt expected: ab#1 applies its score silently"
    );

    // --- Verification ---
    let score_mod = game.state.mods.score_modifiers.get(&dancing_stars);
    assert!(
        score_mod.is_some(),
        "Score modifier should exist on Dancing stars on me!"
    );
    let total = score_mod.unwrap().total();
    assert_eq!(total, 1, "Score modifier should be exactly +1, got {}", total);
}
