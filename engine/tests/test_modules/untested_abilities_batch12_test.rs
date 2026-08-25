/// Untested-abilities batch 12 — depth=none gaps in pb1/bp6 sets:
/// - PL!HS-pb1-021-N (ライブ成功時): draw when a DOLLCHESTRA card is in own live zone
/// - PL!S-bp6-022-L (ライブ成功時): score +1 when own energy > opponent's
/// - PL!HS-bp6-012-R (登場): activate 1 energy when another スリーズブーケ is on stage
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member, cost 4

// ====================================================================
// PL!HS-pb1-021-N (ライブ成功時):
// 「自分のライブカード置き場に『DOLLCHESTRA』のカードがある場合、カードを1枚引く。」
// ====================================================================

#[test]
fn pb1021_dollchestra_in_live_zone_draws() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!HS-pb1-021-N");
    game.state.player1.stage.stage[0] = me;
    // A DOLLCHESTRA live card in the live card zone.
    let doll_live = game.id("PL!HS-bp2-020-L");
    game.state.player1.live_card_zone.cards.push(doll_live);
    let drawn = game.new_id(FILLER);
    game.state.player1.main_deck.cards.push(drawn);
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, me, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "DOLLCHESTRA in live zone -> draw 1"
    );
}

#[test]
fn pb1021_no_dollchestra_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!HS-pb1-021-N");
    game.state.player1.stage.stage[0] = me;
    // Non-DOLLCHESTRA live card: Link to the FUTURE is DOLLCHESTRA, so use a
    // μ's live card instead.
    let other_live = game.id("PL!-sd1-020-SD");
    game.state.player1.live_card_zone.cards.push(other_live);
    let drawn = game.new_id(FILLER);
    game.state.player1.main_deck.cards.push(drawn);
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, me, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "no DOLLCHESTRA in live zone -> no draw"
    );
    let _ = drawn;
}

// ====================================================================
// PL!S-bp6-022-L (ライブ成功時):
// 「相手のエネルギーが自分より多い場合、このカードのスコアを＋１する。」
// (opponent's energy strictly greater than mine)
// ====================================================================

#[test]
fn bp6022_more_opp_energy_scores() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-bp6-022-L");
    game.state.player1.live_card_zone.cards.push(live);

    game.give_energy(1);
    // Opponent gets 3 active energies.
    for _ in 0..3 {
        let e2 = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(e2);
    }
    game.state.player2.energy_zone.add_active(3);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        1,
        "opponent 3 > self 1 -> score +1"
    );
}

#[test]
fn bp6022_equal_or_fewer_opp_energy_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-bp6-022-L");
    game.state.player1.live_card_zone.cards.push(live);

    game.give_energy(2);
    for _ in 0..2 {
        let e2 = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(e2);
    }
    game.state.player2.energy_zone.add_active(2);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "opponent 2 vs self 2 is not more -> no score"
    );
}

// ====================================================================
// PL!HS-bp6-012-R (登場):
// 「自分のステージにほかの『スリーズブーケ』のメンバーがいる場合、エネルギーを1枚アクティブにする。」
// ====================================================================

#[test]
fn bp6012_debut_activates_energy_with_sbuuke_teammate() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!HS-bp6-012-R");
    let mate = game.id("PL!HS-bp1-012-PR"); // スリーズブーケ member
    game.state.player1.stage.stage[0] = me;
    game.state.player1.stage.stage[1] = mate;

    // One WAIT energy (card pushed without add_active).
    let energy = game.id("LL-E-001-SD");
    game.state.player1.energy_zone.cards.push(energy);
    assert_eq!(game.state.player1.energy_zone.active_count(), 0);

    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        1,
        "teammate present -> 1 energy activated"
    );
}
