/// Tests for 凪咲 (PL!-pb1-004-R) — 登場 ability with additive conditional.
///
/// Card text:
///   自分の成功ライブカード置き場にスコアを持つ『μ's』のカードが1枚いる場合、
///   ライブ終了時まで、「常時 ライブの合計スコアを+1する。」を得る。
///   2枚以上いる場合、さらに「常時 ライブの合計スコアを+2する。」を得る。
///
/// Translation:
///   If 1 μ's live card with score is in your success live card zone:
///     gain "常時 ライブの合計スコアを+1" until live end.
///   If 2+ exist: instead gain "常時 ライブの合計スコアを+2".
///
/// Parsed as: sequential[ gain_ability(+1), conditional_on_result(if ≥2, gain_ability(+2)) ]
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for p in [&mut game.state.player1, &mut game.state.player2] {
        p.main_deck.cards.clear();
        for _ in 0..50 {
            p.main_deck.cards.push(filler);
        }
    }
}

fn add_live_to_success_zone(game: &mut TestGame, card_no: &str) {
    let cid = game.id(card_no);
    game.state.player1.success_live_card_zone.cards.push(cid);
}

/// 1 μ's card in success zone → gate passes → gain_ability(+1) → score_mod = 1
#[test]
fn nagisa_one_card_gets_plus_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nagisa = game.id("PL!-pb1-004-R");

    fill_decks(&mut game);
    game.state.player1.hand.cards.push(nagisa);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.give_energy(15);

    add_live_to_success_zone(&mut game, "PL!-sd1-019-SD");

    game.play_to_stage(nagisa, MemberArea::Center);
    game.drain_auto_ability_choices();

    game.state.recalculate_constants();
    let score_mod = game.state.mods.p1_constant_total_score_bonus;
    assert_eq!(score_mod, 1, "1 card → gain +1 (live total)");
}

/// 0 μ's cards in success zone → gate fails → nothing → score_mod = 0
#[test]
fn nagisa_zero_cards_nothing_happens() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nagisa = game.id("PL!-pb1-004-R");

    fill_decks(&mut game);
    game.state.player1.hand.cards.push(nagisa);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.give_energy(15);

    game.play_to_stage(nagisa, MemberArea::Center);
    game.drain_auto_ability_choices();

    game.state.recalculate_constants();
    let score_mod = game.state.mods.p1_constant_total_score_bonus;
    assert_eq!(score_mod, 0, "0 cards → nothing");
}

/// 2 μ's cards in success zone → alternative_condition(≥2) met → gain_ability(+2) instead of +1
/// → total score_mod = 2
#[test]
fn nagisa_two_cards_gets_plus_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nagisa = game.id("PL!-pb1-004-R");

    fill_decks(&mut game);
    game.state.player1.hand.cards.push(nagisa);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.give_energy(15);

    add_live_to_success_zone(&mut game, "PL!-sd1-019-SD");
    add_live_to_success_zone(&mut game, "PL!-bp4-022-SECL");

    game.play_to_stage(nagisa, MemberArea::Center);
    game.drain_auto_ability_choices();

    game.state.recalculate_constants();
    let score_mod = game.state.mods.p1_constant_total_score_bonus;
    assert_eq!(score_mod, 2, "2 cards → gain +2 instead of +1 (live total)");
}

/// Not at center → activation_condition_parsed blocks the ability
#[test]
fn nagisa_not_at_center_does_not_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nagisa = game.id("PL!-pb1-004-R");

    fill_decks(&mut game);
    game.state.player1.hand.cards.push(nagisa);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.give_energy(15);

    add_live_to_success_zone(&mut game, "PL!-sd1-019-SD");

    game.play_to_stage(nagisa, MemberArea::LeftSide);
    game.drain_auto_ability_choices();

    game.state.recalculate_constants();
    let score_mod = game.state.mods.p1_constant_total_score_bonus;
    assert_eq!(score_mod, 0, "Left side → activation_condition blocks");
}

/// Gained ability is cleared on card leaving / live end
#[test]
fn nagisa_effect_expires_on_clear() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nagisa = game.id("PL!-pb1-004-R");

    fill_decks(&mut game);
    game.state.player1.hand.cards.push(nagisa);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.give_energy(15);

    add_live_to_success_zone(&mut game, "PL!-sd1-019-SD");
    add_live_to_success_zone(&mut game, "PL!-bp4-022-SECL");

    game.play_to_stage(nagisa, MemberArea::Center);
    game.drain_auto_ability_choices();

    game.state.recalculate_constants();
    let before = game.state.mods.p1_constant_total_score_bonus;
    assert_eq!(before, 2, "Should have +2 before clear");

    game.state.clear_gained_abilities_for_card(nagisa);

    game.state.recalculate_constants();
    let after = game.state.mods.p1_constant_total_score_bonus;
    assert_eq!(after, 0, "Gained live-total bonus cleared");
}
