//! 東條 希 `PL!-pb2-007-PP` (起動)
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   起動 — このメンバーをステージから控え室に置く：自分の控え室から『μ's』のライブカードを
//!   1枚手札に加える。その後、自分の成功ライブカード置き場にある『μ's』のカード1枚につき、
//!   エネルギーを1枚アクティブにする。
//!
//! In English: as a cost, send HER from the stage to the waitroom; then recover one μ's
//! live card from your waitroom to your hand; and for each μ's card in your success live
//! zone, activate one energy.
//!
//! Two separable halves, pinned separately: the self→waitroom cost and the recovery,
//! then the per-card energy. The energy half is the one with a per-unit count, so it is
//! the one that says 「1枚につき」 — two qualifying cards must give two, not one.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const HANAYOME: &str = "PL!-pb2-007-PP"; // 東條 希, lily white
const ENERGY: &str = "LL-E-001-SD";
const FILLER: &str = "PL!-sd1-010-SD";
/// μ's lives for the waitroom and the success zone.
const MUS_LIVE: &str = "PL!-pb1-029-SRL"; // 知らないLove, lily white
const MUS_LIVE2: &str = "PL!-bp4-025-SRL"; // 微熱からMystery, lily white
/// An Aqours live — `series=ラブライブ！サンシャイン!!` carries a qualifier, so
/// `card_series_matches_group` excludes it from 『μ's』. `LL-PR-004-PR` cannot be
/// this control: its `series` is the bare `ラブライブ！`, which IS μ's.
const NON_MUS_LIVE: &str = "PL!S-bp2-019-L"; // WATER BLUE NEW WORLD

/// Stand HANAYOME with `waitroom` and `success` seeded; pay the cost by activating.
/// Returns (energy activated, whether she reached the waitroom, hand size).
fn run(waitroom: &[&str], success: &[&str]) -> (i32, bool, usize) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let me = game.id(HANAYOME);
    game.assert_card_identity(me, HANAYOME);
    for c in waitroom {
        game.state.player1.waitroom.add_card(game.id(c));
    }
    for c in success {
        game.state
            .player1
            .success_live_card_zone
            .add_card(game.id(c));
    }
    for _ in 0..3 {
        game.state.player1.energy_zone.cards.push(game.id(ENERGY));
    }
    game.state.player1.stage.stage[0] = me;
    game.give_energy(10);

    // The activation cost draws energy from the same zone, so an ABSOLUTE active
    // count is dominated by the payment. Report the DELTA across the activation.
    let before = game.state.player1.energy_zone.active_count();
    game.activate_ability(me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 5 {
        game.select_indices(&[0]);
        guard += 1;
    }

    let in_waitroom = game.state.player1.waitroom.cards.contains(&me);
    let after = game.state.player1.energy_zone.active_count();
    (
        after as i32 - before as i32,
        in_waitroom,
        game.state.player1.hand.cards.len(),
    )
}

/// The cost sends her from the stage to the waitroom.
#[test]
fn the_cost_sends_her_to_the_waitroom() {
    let (_energy, in_waitroom, _hand) = run(&[MUS_LIVE], &[]);
    assert!(
        in_waitroom,
        "このメンバーをステージから控え室に置く — the cost puts her in the waitroom"
    );
}

/// The recovery: a μ's live in the waitroom reaches hand.
#[test]
fn a_mus_live_is_recovered_from_the_waitroom() {
    let (_energy, _wait, hand) = run(&[MUS_LIVE], &[]);
    assert!(
        hand >= 1,
        "自分の控え室から『μ's』のライブカードを1枚手札に加える — the recovered card \
         is in hand"
    );
}

/// The per-card half: one μ's card in the success zone activates one energy.
#[test]
fn one_mus_card_in_the_success_zone_activates_one_energy() {
    let (energy, _wait, _hand) = run(&[], &[MUS_LIVE]);
    assert_eq!(
        energy, 1,
        "自分の成功ライブカード置き場にある『μ's』のカード1枚につき、エネルギーを1枚\
         アクティブにする"
    );
}

/// 1枚につき — two qualifying cards activate two energy, not one.
#[test]
fn two_mus_cards_activate_two_energy() {
    let (energy, _wait, _hand) = run(&[], &[MUS_LIVE, MUS_LIVE2]);
    assert_eq!(
        energy, 2,
        "1枚につき — the energy scales with the number of qualifying cards"
    );
}

/// The group filter on the success-zone count.
#[test]
fn a_non_mus_card_in_the_success_zone_activates_no_energy() {
    let (energy, _wait, _hand) = run(&[], &[NON_MUS_LIVE]);
    assert_eq!(
        energy, 0,
        "『μ's』のカード — a non-μ's live in the success zone is not counted"
    );
}

/// An empty success zone contributes nothing.
#[test]
fn an_empty_success_zone_activates_no_energy() {
    let (energy, _wait, _hand) = run(&[], &[]);
    assert_eq!(
        energy, 0,
        "自分の成功ライブカード置き場にある『μ's』のカード1枚につき — with no such card \
         there is nothing to scale by"
    );
}

/// The ability's shape, pinned so the two halves stay separable.
#[test]
fn the_cost_and_the_two_effect_halves_are_separate() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let id = game.id(HANAYOME);
    let eff = {
        let owned = game
            .db
            .get_card(id)
            .unwrap()
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("起動"));
        owned
            .and_then(|a| a.effect.as_ref().map(|e| e.clone()))
            .expect("起動 ability has an effect")
    };
    assert_eq!(
        eff.action,
        rabuka_engine::ability::enums::ActionType::Sequential,
        "…1枚手札に加える。その後… — a two-step effect"
    );
    assert!(
        eff.text.contains("その後"),
        "…1枚手札に加える。その後… — a two-step effect: recovery, then per-card energy"
    );

    let cost = {
        let owned = game
            .db
            .get_card(id)
            .unwrap()
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("起動"));
        owned.and_then(|a| a.cost.clone())
    }
    .expect("起動 ability has a cost");
    assert!(
        cost.text.contains("このメンバーをステージから控え室に置く"),
        "このメンバーをステージから控え室に置く — the cost sends HER off the stage"
    );
}
