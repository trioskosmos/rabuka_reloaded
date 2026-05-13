/// Tests for 鬼塚夏美 (PL!SP-bp2-009-R+) — LiveStart ability:
///
/// {{live_start.png|ライブ開始時}}ライブ終了時まで、自分の手札2枚につき、
/// {{icon_blade.png|ブレード}}を得る。
///
/// For each 2 cards in hand, gain 1 blade until live end.

mod helpers;
use helpers::*;

/// Ab#0 (LiveStart): For each 2 cards in hand, gain 1 blade.
/// 4 cards in hand → 2 blades (4 ÷ 2 = 2)
#[test]
fn natsumi_live_start_blade_per_2_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let natsumi = game.id("PL!SP-bp2-009-P");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-019-SD");

    // Stage: 鬼塚夏美
    game.state.player1.stage.stage[1] = natsumi;

    // Hand: 5 fillers + live_card. After set_live_card removes live_card, 5 remain.
    // After LiveCardSetP1 pass draws 1 replacement, 6 remain.
    // At LiveStart: 6 hand cards → 6 ÷ 2 = 3 blade
    for _ in 0..5 { game.state.player1.hand.cards.push(filler); }
    game.state.player1.hand.cards.push(live_card);

    // Seed decks
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Advance to LiveStart
    for _ in 0..5 { game.pass(); }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));

    game.set_live_card(live_card);
    game.pass(); // LiveCardSetP2
    game.pass(); // FirstAttackerPerformance (LiveStart fires here)

    // LiveStart fired: per_unit(hand, 2) → 6 hand cards ÷ 2 = 3 blade
    let blade_mod = game.state.mods.get_blade_modifier(natsumi);
    assert_eq!(blade_mod, 3,
        "6 hand cards should give 3 blade (per 2 cards): 6/2*1=3");
}
