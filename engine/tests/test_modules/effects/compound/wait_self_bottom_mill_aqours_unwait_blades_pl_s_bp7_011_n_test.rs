use crate::helpers::*;
use rabuka_engine::game_setup::{self, ActionType};

// ====================================================================
// PL!S-bp7-011-N 桜内梨子 (was ZERO behavior tests — only mill fodder)
// {{起動}}{{ターン1回}}このメンバーをウェイトにする：自分のデッキの下から
// カードを2枚控え室に置く。それらがすべて『Aqours』のメンバーカードの場合、
// このメンバーをアクティブにし、ライブ終了時まで、ブレード2を得る。
//
// Turn-1, wait-self cost: mill 2 from deck BOTTOM. If ALL milled are Aqours
// members: unwait self + 2 blades until live end.
// ====================================================================

const AQOURS_A: &str = "PL!S-bp7-015-N"; // 津島善子 (Aqours)
const AQOURS_B: &str = "PL!S-bp7-017-N"; // 小原鞠莉 (Aqours)
const NON_AQOURS: &str = "PL!-sd1-010-SD"; // 高坂穂乃果 (μ's member)

/// Stage active Riko; deck = 10 filler + `bottom_two` with the LAST element
/// at the deck bottom (index 0 = top).
fn riko_setup(game: &mut TestGame, bottom_two: Vec<i16>) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let riko = game.id("PL!S-bp7-011-N");
    assert_eq!(
        game.db.get_card(riko).unwrap().card_no,
        "PL!S-bp7-011-N",
        "staged Riko N print"
    );
    game.state.player1.stage.stage[1] = riko;
    let mut deck: Vec<i16> = (0..10).map(|_| game.new_id("PL!-sd1-010-SD")).collect();
    deck.extend(bottom_two);
    game.state.player1.main_deck.cards = deck.into();
    riko
}

fn use_offers(game: &TestGame, cid: i16) -> usize {
    game_setup::generate_possible_actions(&game.state)
        .iter()
        .filter(|a| {
            a.action_type == ActionType::UseAbility
                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(cid)
        })
        .count()
}

#[test]
fn riko_all_aqours_mill_unwaits_and_grants_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let a = game.new_id(AQOURS_A);
    let b = game.new_id(AQOURS_B);
    let riko = riko_setup(&mut game, vec![a, b]);

    assert_eq!(use_offers(&game, riko), 1, "active Riko offered");
    game.activate_ability(riko);
    assert!(
        !game.has_pending_choice(),
        "no prompt expected in the all-Aqours line"
    );

    assert!(
        game.state.player1.waitroom.cards.contains(&a)
            && game.state.player1.waitroom.cards.contains(&b),
        "exactly the bottom 2 milled to the waitroom"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(riko),
        Some("active"),
        "all Aqours: cost-wait is undone"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(riko),
        2,
        "all Aqours: exactly 2 blades"
    );
    assert!(
        game.state
            .turn_limited_abilities_used
            .contains_key(&(riko, 0, game.state.turn_number)),
        "ターン1回 use recorded"
    );
    assert_eq!(use_offers(&game, riko), 0, "no re-offer after consuming");
}

#[test]
fn riko_mixed_mill_stays_wait_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let a = game.new_id(AQOURS_A);
    let other = game.new_id(NON_AQOURS);
    let riko = riko_setup(&mut game, vec![a, other]);

    game.activate_ability(riko);
    assert!(
        !game.has_pending_choice(),
        "no prompt expected in the mixed line"
    );

    assert!(
        game.state.player1.waitroom.cards.contains(&a)
            && game.state.player1.waitroom.cards.contains(&other),
        "both bottom cards milled"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(riko),
        Some("wait"),
        "mixed mill: cost-wait stands"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(riko),
        0,
        "mixed mill: no blades"
    );
}

#[test]
fn riko_already_wait_refused() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let a = game.new_id(AQOURS_A);
    let b = game.new_id(AQOURS_B);
    let riko = riko_setup(&mut game, vec![a, b]);
    game.state.mods.add_orientation_modifier(riko, "wait");

    assert_eq!(use_offers(&game, riko), 0, "waited Riko not offered");
    let result = game.try_activate_ability(riko);
    assert!(
        result.is_err(),
        "wait-self cost on an already-wait member must be refused, got {:?}",
        result
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&a),
        "refused activation mills nothing"
    );
}

#[test]
fn riko_empty_deck_mills_nothing_grants_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp7-011-N");
    game.state.player1.stage.stage[1] = riko;
    game.state.player1.main_deck.cards.clear();

    game.activate_ability(riko);
    assert!(
        !game.has_pending_choice(),
        "no prompt expected with empty deck"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(riko),
        0,
        "milling nothing must not satisfy 'all Aqours' (no vacuous bonus)"
    );
}
