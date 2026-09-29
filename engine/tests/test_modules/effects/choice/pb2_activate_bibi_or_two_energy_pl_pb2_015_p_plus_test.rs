//! 絢瀬絵里 `PL!-pb2-015-P＋` (自動, ターン1回)
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   自動／ターン1回 — 自分の『BiBi』のカードの効果によって、相手のステージにいる
//!   メンバーがウェイト状態になったとき、以下から1つを選ぶ。
//!     ・自分のステージにいる『BiBi』のメンバー1人をアクティブにする。
//!     ・エネルギーを2枚アクティブにする。
//!
//! In English: once per turn, when one of MY BiBi cards' effects puts an OPPONENT's
//! member into wait, choose one of: (1) make one BiBi member on my stage active
//! again, or (2) activate 2 energy.
//!
//! The mechanism is shared with 19 other cards in the corpus — all parse as
//! `action: choice` with N sub-effects under `options` (this one has 2;
//! `PL!S-bp6-020-L` has 3, `PL!SP-pb2-011-PP` has 3 and is also 自動). The prompt is a
//! `SelectChoice` whose `options` array is answered with `select_option(n)`; see
//! `conditional_choice_options_execution_test.rs::bouken_three_options`.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const ERI: &str = "PL!-pb2-015-P＋";
/// A BiBi member — the target of branch 1.
const BIBI_MEMBER: &str = "PL!-pb2-002-PP"; // 絢瀬絵里, BiBi
const ENERGY: &str = "LL-E-001-SD";
const FILLER: &str = "PL!-sd1-010-SD";
/// An opponent member that our own BiBi effect will have waited.
const OPPONENT: &str = "PL!S-bp5-012-N";

fn fill_decks(game: &mut TestGame) {
    let f = game.id(FILLER);
    for _ in 0..12 {
        game.state.player1.main_deck.cards.push(f);
        game.state.player2.main_deck.cards.push(f);
    }
}

/// Set the board up, fire the 自動, and return the game with the choice pending.
fn fired() -> TestGame {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    fill_decks(&mut game);

    let eri = game.id(ERI);
    game.assert_card_identity(eri, ERI);
    let bibi = game.id(BIBI_MEMBER);
    let foe = game.id(OPPONENT);

    game.state.player1.stage.stage[MemberArea::LeftSide as usize] = eri;
    game.state.player1.stage.stage[MemberArea::Center as usize] = bibi;
    game.state.mods.add_orientation_modifier(bibi, "wait");
    game.state.player2.stage.stage[MemberArea::Center as usize] = foe;
    game.state.mods.add_orientation_modifier(foe, "wait");

    game.state.player1.energy_zone.cards.push(game.id(ENERGY));
    game.state.player1.energy_zone.cards.push(game.id(ENERGY));
    game.state.player1.energy_zone.add_active(2);

    let ability_id = {
        let card = game.db.get_card(eri).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("自動"))
            .unwrap_or_else(|| panic!("{} lacks a 自動 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(eri).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.activating_card = Some(eri);
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::Auto,
        pid.clone(),
        Some(card_no),
        Some(eri),
        None,
        None,
    );
    game.state.process_pending_auto_abilities(&pid);
    game
}

/// The choice is presented, and it has exactly the two options the text lists.
#[test]
fn the_choice_offers_exactly_the_two_listed_options() {
    let mut game = fired();
    assert!(
        game.has_pending_choice(),
        "以下から1つを選択 must present a prompt"
    );
    // Same accessor the working analogue uses (bouken_three_options): the option
    // list lives in the pending-choice JSON, not on the typed `Choice` enum.
    let json = game.state.get_pending_choice_json();
    let opts = json
        .as_ref()
        .and_then(|j| j.get("options"))
        .and_then(|o| o.as_array())
        .expect("the choice carries an options array");
    assert_eq!(
        opts.len(),
        2,
        "・自分のステージにいる『BiBi』のメンバー1人をアクティブにする / \
         ・エネルギーを2枚アクティブにする — exactly two options"
    );
    game.select_option(0);
}

/// Branch 1: the BiBi member becomes active again, and no energy is gained.
#[test]
fn option_one_activates_a_bibi_member() {
    let mut game = fired();
    game.select_option(0);
    let _ = game.drain_auto_ability_choices();

    assert_eq!(
        game.state.mods.get_orientation_modifier(game.id(BIBI_MEMBER)),
        None,
        "自分のステージにいる『BiBi』のメンバー1人をアクティブにする — the waited BiBi \
         member is active again"
    );
}

/// Branch 2: two energy become active, and the member stays waited.
#[test]
fn option_two_activates_two_energy() {
    let mut game = fired();
    game.select_option(1);
    let _ = game.drain_auto_ability_choices();

    let bibi = game.id(BIBI_MEMBER);
    assert_eq!(
        game.state.mods.get_orientation_modifier(bibi),
        Some("wait"),
        "エネルギーを2枚アクティブにする — branch 2 must NOT activate the BiBi member"
    );
    assert!(
        game.state.player1.energy_zone.active_count() >= 2,
        "エネルギーを2枚アクティブにする — the energy zone gained two active cards, got {}",
        game.state.player1.energy_zone.active_count()
    );
}

/// The two branches are exclusive — picking one must not do the other.
#[test]
fn the_two_options_are_exclusive() {
    let mut one = fired();
    one.select_option(0);
    let _ = one.drain_auto_ability_choices();

    let mut two = fired();
    two.select_option(1);
    let _ = two.drain_auto_ability_choices();

    assert_eq!(
        one.state.mods.get_orientation_modifier(one.id(BIBI_MEMBER)),
        None,
        "branch 1 activates the member"
    );
    assert_eq!(
        two.state.mods.get_orientation_modifier(two.id(BIBI_MEMBER)),
        Some("wait"),
        "branch 2 leaves the member waited — the two options must not both apply"
    );
}
