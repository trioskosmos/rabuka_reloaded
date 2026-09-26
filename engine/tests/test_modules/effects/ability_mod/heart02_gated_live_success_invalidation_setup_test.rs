/// A ライブ開始時 that invalidates ライブ成功時 when the acting player's group
/// has at least 6 heart02 on stage — the two sides of that one gate.
///
/// Both tests this replaces walked the turn with a blind `for _ in 0..5 { pass() }`
/// and then asserted only that the live card was in the live-card zone, which
/// is true whether or not the gate fired at all. They also staged the SAME card
/// in both stage slots (`game.id("PL!S-bp2-002-R")` twice returns one id), so
/// the "two members" the threshold counts did not exist. The observable is
/// `GameState::is_ability_invalidated(live, LiveSuccess)`, which is what the
/// engine's own observation layer reads.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::game_state::Phase;

/// `heart02_total` heart02 spread over two DISTINCT 『Aqours』 members, plus the
/// live in hand, filled decks, and enough energy to play it.
fn setup(heart02_total: i16) -> (TestGame, i16, i16, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-pb1-019-L");
    let a = game.id("PL!S-bp2-002-R");
    // A separate instance: the threshold counts heart02 across the STAGE, and one
    // card cannot occupy two areas.
    let b = game.new_id("PL!S-bp2-002-R");
    game.assert_card_identity(live, "PL!S-pb1-019-L");
    game.assert_card_identity(a, "PL!S-bp2-002-R");
    game.assert_same_card_name(a, b, "two copies of one 『Aqours』 print");
    assert_ne!(a, b, "two separate card instances");

    game.state.player1.stage.stage = [a, b, -1];
    // Split the total across the two so neither alone reaches 6 when the total
    // is meant to be below it.
    let first = heart02_total / 2;
    game.state
        .mods
        .add_heart_modifier(a, HeartColor::Heart02, first);
    game.state
        .mods
        .add_heart_modifier(b, HeartColor::Heart02, heart02_total - first);

    game.state.player1.hand.cards.push(live);
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(10);
    (game, live, a, b)
}

/// Reveal the live and step to the ライブ開始時 scan, by phase name.
fn reveal_live_and_reach_live_start(game: &mut TestGame, live: i16) {
    game.advance_to_phase(Phase::LiveCardSetFirstAttacker);
    game.set_live_card(live);
    game.advance_to_phase(Phase::FirstAttackerPerformance);
}

/// 合計6以上のとき ライブ成功時 is invalidated.
#[test]
fn live_start_with_six_group_heart02_invalidates_live_success() {
    let (mut game, live, a, _b) = setup(6);

    reveal_live_and_reach_live_start(&mut game, live);

    assert!(
        game.state.is_ability_invalidated(live, &AbilityTrigger::LiveSuccess),
        "自分のグループの合計heart02が6以上 → ライブ成功時を無効にする"
    );
    assert!(
        game.state.player1.live_card_zone.cards.contains(&live),
        "invalidation does not move the live card out of the zone"
    );
    // The modifier that triggered it must still be there — the gate reads the
    // stage, it does not consume the heart02.
    assert_eq!(
        game.state.mods.get_heart_modifier(a, HeartColor::Heart02),
        3,
        "the gate reads the total; it does not spend the heart02"
    );
}

/// Below the threshold, ライブ成功時 survives.
#[test]
fn live_start_below_six_group_heart02_keeps_live_success() {
    let (mut game, live, _a, _b) = setup(4);

    reveal_live_and_reach_live_start(&mut game, live);

    assert!(
        !game.state.is_ability_invalidated(live, &AbilityTrigger::LiveSuccess),
        "合計heart02が6未満 → ライブ成功時は無効化されない"
    );
    assert!(
        game.state.player1.live_card_zone.cards.contains(&live),
        "the live stays in the zone"
    );
}
