/// BP07 CLEAN-G13: PL!N-bp7-031-L Like a Treasure ab#1 (自動).
///
/// 自動：自分のライブ成功時能力によって、カードが自分のデッキから自分の控え室に置かれる
/// たび、それらのカードの中から『虹ヶ咲』のライブカードを1枚手札に加えてもよい。そうした
/// とき、このカードのスコアを+1する。
///
/// (Auto) Each time a card is placed from your deck to your discard by your
/// live-success ability, you MAY add 1 『虹ヶ咲』 live card from among those cards
/// to your hand. If you do, this card's score +1.
///
/// The defect (G13): the add-to-hand was dropped — only the score gain survived.
/// These tests pin the PARSED structure: a conditional_on_optional whose accepted
/// branch moves a 虹ヶ咲 live card to hand AND then grants +1 score.
use crate::helpers::*;
use rabuka_engine::ability::enums::ActionType;
use rabuka_engine::card::{AbilityEffect, CardType};

const LIKE_A_TREASURE: &str = "PL!N-bp7-031-L";
const NIJI_LIVE: &str = "PL!N-bp1-026-L"; // 虹ヶ咲 live card (Poppin' Up!)

fn ab1_effect(game: &mut TestGame) -> AbilityEffect {
    let id = game.id(LIKE_A_TREASURE);
    let card = game.db.get_card(id).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("自動"))
        .expect("card should have a 自動 ability");
    (**ab.effect.as_ref().expect("ability has an effect")).clone()
}

/// The whole effect is a conditional_on_optional (the "してもよい" gate).
#[test]
fn like_a_treasure_ab1_is_conditional_optional() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let eff = ab1_effect(&mut game);
    assert_eq!(
        eff.action,
        ActionType::ConditionalOnOptional,
        "ab#1 must be conditional_on_optional (optional add-to-hand), not a bare score gain"
    );
}

/// optional_action moves 1 虹ヶ咲 live card from the moved cards to hand.
#[test]
fn like_a_treasure_optional_action_moves_niji_live_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let eff = ab1_effect(&mut game);

    let opt = eff
        .compound
        .optional_action
        .as_ref()
        .expect("optional_action present");
    assert_eq!(opt.action, ActionType::MoveCards, "optional action is a move");
    assert_eq!(
        opt.source_any().as_deref(),
        Some("those_cards"),
        "moves from the just-moved cards"
    );
    assert_eq!(
        opt.destination_any().as_deref(),
        Some("hand"),
        "moves to hand"
    );
    assert_eq!(
        opt.card_type_any(),
        Some(&CardType::Live),
        "targets a live card"
    );
    assert_eq!(
        opt.count_any(),
        Some(1),
        "adds exactly 1 card"
    );
    let groups_owned: Vec<String> = opt.group_names_any().cloned().unwrap_or_default();
    assert!(
        groups_owned.iter().any(|g| g.contains("虹ヶ咲")),
        "filtered to 『虹ヶ咲』 live cards, got {:?}",
        groups_owned
    );
}

/// The accepted branch is a sequential: first move the live card to hand, then
/// grant +1 score (the "そうしたとき" consequence).
#[test]
fn like_a_treasure_accepted_branch_moves_then_scores() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let eff = ab1_effect(&mut game);

    let cond = eff
        .compound
        .conditional_action
        .as_ref()
        .expect("conditional_action present");
    assert_eq!(cond.action, ActionType::Sequential, "accepted branch is sequential");
    let steps = cond.compound.actions.as_ref().expect("sequential steps");
    assert_eq!(steps.len(), 2, "two steps: move-to-hand then score");

    let move_step = steps[0].as_ref();
    assert_eq!(move_step.action, ActionType::MoveCards, "first step moves to hand");
    assert_eq!(
        move_step.destination_any().as_deref(),
        Some("hand"),
        "first step destination is hand"
    );
    assert_eq!(
        move_step.source_any().as_deref(),
        Some("those_cards"),
        "first step moves from the just-moved cards"
    );

    let score_step = steps[1].as_ref();
    assert_eq!(score_step.action, ActionType::ModifyScore, "second step is a score gain");
    assert_eq!(score_step.value_any(), Some(1), "score +1");
    let _ = NIJI_LIVE;
}
