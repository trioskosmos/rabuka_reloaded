/// BP07 CLEAN-G16: PL!SP-bp7-028-L 未来の音が聴こえる ab#0 (ライブ開始時).
///
/// ライブ開始時：自分の控え室にある『Liella!』のメンバーカードを9枚選び、それらを
/// シャッフルし、デッキの一番下に置いてもよい。そうしたとき、ライブ終了時まで、
/// 自分のステージにいるすべてのメンバーはブレードを得る。
///
/// (Live start) Choose 9 『Liella!』 member cards in your discard, shuffle them and
/// may place them on the BOTTOM of the deck. If you do, until live end, all members
/// on your stage gain 1 blade.
///
/// The defect (G16): the shuffle+place-under was folded into a CONDITION, so the
/// optional action never actually moved the cards. These tests pin the parsed
/// structure: a conditional_on_optional whose optional/accept branch is the
/// discard→deck-bottom shuffle.
use crate::helpers::*;
use rabuka_engine::ability::enums::ActionType;
use rabuka_engine::card::{AbilityEffect, CardType, PlacementOrder};

const MIRAI: &str = "PL!SP-bp7-028-L";

fn ab0_effect(game: &mut TestGame) -> AbilityEffect {
    let id = game.id(MIRAI);
    let card = game.db.get_card(id).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .expect("card should have a ライブ開始時 ability");
    (**ab.effect.as_ref().expect("ability has an effect")).clone()
}

/// The whole effect is a conditional_on_optional (the "してもよい" gate), NOT a
/// bare condition + gain.
#[test]
fn mirai_ab0_is_conditional_optional() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let eff = ab0_effect(&mut game);
    assert_eq!(
        eff.action,
        ActionType::ConditionalOnOptional,
        "ab#0 must be conditional_on_optional (optional shuffle-to-bottom), not a condition+gain"
    );
}

/// optional_action moves 9 Liella! discard member cards to the deck bottom, shuffled
/// in any order.
#[test]
fn mirai_optional_action_shuffles_discard_to_deck_bottom() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let eff = ab0_effect(&mut game);

    let opt = eff
        .compound
        .optional_action
        .as_ref()
        .expect("optional_action present");
    assert_eq!(opt.action, ActionType::MoveCards, "optional action is a move");
    assert_eq!(opt.source_any().as_deref(), Some("discard"), "moves from discard");
    assert_eq!(
        opt.destination_any().as_deref(),
        Some("deck_bottom"),
        "moves to deck bottom"
    );
    assert_eq!(opt.count_any(), Some(9), "selects 9 cards");
    assert_eq!(
        opt.card_type_any(),
        Some(&CardType::Member),
        "moves member cards"
    );
    assert_eq!(opt.shuffle_any(), Some(true), "shuffles the moved cards");
    assert_eq!(
        opt.placement_order_any(),
        Some(PlacementOrder::AnyOrder),
        "any order placement"
    );
    let groups_owned: Vec<String> = opt.group_names_any().cloned().unwrap_or_default();
    assert!(
        groups_owned.iter().any(|g| g.contains("Liella!")),
        "filtered to 『Liella!』 member cards, got {:?}",
        groups_owned
    );
}

/// The accepted branch first moves the 9 cards to the deck bottom, then grants
/// blade to all stage members (the "そうしたとき" consequence).
#[test]
fn mirai_accepted_branch_moves_then_grants_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let eff = ab0_effect(&mut game);

    let cond = eff
        .compound
        .conditional_action
        .as_ref()
        .expect("conditional_action present");
    assert_eq!(cond.action, ActionType::Sequential, "accepted branch is sequential");
    let steps = cond.compound.actions.as_ref().expect("sequential steps");
    assert_eq!(steps.len(), 2, "two steps: move then blade");

    let move_step = steps[0].as_ref();
    assert_eq!(move_step.action, ActionType::MoveCards, "first step moves");
    assert_eq!(
        move_step.destination_any().as_deref(),
        Some("deck_bottom"),
        "first step moves to deck bottom"
    );

    let gain_step = steps[1].as_ref();
    assert_eq!(gain_step.action, ActionType::GainResource, "second step gains");
    assert_eq!(
        gain_step.resource_any().as_deref(),
        Some("blade"),
        "second step grants blade"
    );
}
