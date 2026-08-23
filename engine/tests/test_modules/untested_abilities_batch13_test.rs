/// Untested-abilities batch 13 — depth=none gaps in PR sets:
/// - PL!HS-bp2-017-N (登場): draw when own deck has 10+ cards
/// - PL!S-PR-039-PR (常時): +2 blades while both players' success zones hold ≥4 cards
/// - PL!S-PR-042-PR (常時): heart02+heart04 while 6 total members are staged
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member, cost 4

fn fire_trigger(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trig: &str) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some(trig))
            .unwrap_or_else(|| panic!("card {} lacks a '{trig}' ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        trigger,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

// ====================================================================
// PL!HS-bp2-017-N (登場):
// 「自分の控え室にカードが10枚以上ある場合、カードを1枚引く。」
// ====================================================================

#[test]
fn bp2017_debut_draws_with_10_card_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!HS-bp2-017-N");
    game.state.player1.stage.stage[0] = me;
    for _ in 0..10 {
        let c = game.new_id(FILLER);
        game.state.player1.waitroom.cards.push(c);
    }
    let drawn = game.new_id(FILLER);
    game.state.player1.main_deck.cards.push(drawn);
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "waitroom has 10+ cards -> draw 1"
    );
}

#[test]
fn bp2017_debut_no_draw_with_9_card_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!HS-bp2-017-N");
    game.state.player1.stage.stage[0] = me;
    for _ in 0..9 {
        let c = game.new_id(FILLER);
        game.state.player1.waitroom.cards.push(c);
    }
    let drawn = game.new_id(FILLER);
    game.state.player1.main_deck.cards.push(drawn);
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "only 9 cards in waitroom -> no draw"
    );
}

// ====================================================================
// PL!S-PR-039-PR (常時):
// 「自分と相手の成功ライブカード置き場にカードが合計4枚以上あるかぎり、
//   {{blade}}{{blade}}を得る。」
// Constant — evaluated via recalculate_constants.
// ====================================================================

#[test]
fn spr039_constant_blades_with_combined_success_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-PR-039-PR");
    game.state.player1.stage.stage[0] = me;

    for _ in 0..2 {
        let a = game.new_id(FILLER);
        game.state.player1.success_live_card_zone.cards.push(a);
        let b = game.new_id(FILLER);
        game.state.player2.success_live_card_zone.cards.push(b);
    }
    // Combined = 4.

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        2,
        "4 combined success-zone cards -> +2 blades"
    );
}

#[test]
fn spr039_constant_blades_off_below_four_combined() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-PR-039-PR");
    game.state.player1.stage.stage[0] = me;

    let a = game.new_id(FILLER);
    game.state.player1.success_live_card_zone.cards.push(a);
    let b = game.new_id(FILLER);
    game.state.player2.success_live_card_zone.cards.push(b);
    // Combined = 2 < 4.

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        0,
        "only 2 combined success-zone cards -> no blades"
    );
}

// ====================================================================
// PL!S-PR-042-PR (常時):
// 「自分と相手のステージにメンバーが合計6人いるかぎり、
//   {{heart_02.png|heart02}}{{heart_04.png|heart04}}を得る。」
// ====================================================================

#[test]
fn spr042_constant_hearts_with_six_staged_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-PR-042-PR");
    let m1 = game.new_id(FILLER);
    let m2 = game.new_id(FILLER);
    game.state.player1.stage.stage = [me, m1, m2];
    // 3 more for p2 => 6 total including me.
    for i in 0..3usize {
        let m = game.new_id(FILLER);
        game.state.player2.stage.stage[i] = m;
    }

    game.state.recalculate_constants();

    const H02: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart02;
    const H04: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart04;
    let h02 = game.state.mods.get_heart_modifier(me, H02);
    let h04 = game.state.mods.get_heart_modifier(me, H04);
    if h02 == 0 || h04 == 0 {
        panic!(
            "heart02={} heart04={}\nstaged p1={:?} p2={:?}\nstatuses={:#?}",
            h02,
            h04,
            game.state.player1.stage.stage,
            game.state.player2.stage.stage,
            game.state.constant_ability_statuses
        );
    }
    assert!(h02 > 0, "6 staged members -> heart02 granted");
    assert!(h04 > 0, "6 staged members -> heart04 granted");
}
