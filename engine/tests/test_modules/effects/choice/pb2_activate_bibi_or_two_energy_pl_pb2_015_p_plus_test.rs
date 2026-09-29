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
//! The mechanism is shared with 19 other cards — all parse as `action: choice` with N
//! sub-effects under `options` (this one has 2; `PL!S-bp6-020-L` and `PL!SP-pb2-011-PP`
//! have 3). The prompt is a `SelectTarget` whose option list is read from
//! `game.state.get_pending_choice_json()` and answered with `select_option(n)`; see
//! `conditional_choice_options_execution_test.rs::bouken_three_options`.
//!
//! # What is pinned here, and what is not
//!
//! Branch 2 is pinned end to end: choosing it activates exactly two energy cards and
//! asks nothing further. Branch 1 is NOT pinned — its outcome depends on the board
//! having a BiBi member in wait, and a member written into the stage with
//! `add_orientation_modifier(..., "wait")` is read back as having no orientation
//! modifier by the time the ability fires, so the fixture could not be built
//! reliably. The 「1人をアクティブにする」 branch and its follow-up "which member?"
//! prompt are therefore untested. See docs/PBLL02_ABILITY_REVIEW.md §8.3.
//!
//! A correction to an earlier claim: I reported this card as applying BOTH branches.
//! That was wrong — it was my test, twice. Activation is recorded as the orientation
//! modifier `"active"`, not as its absence; and a blind `drain_auto_ability_choices()`
//! was answering branch 1's "which member?" prompt on the branch-2 run.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const ERI: &str = "PL!-pb2-015-P＋";
const BIBI_MEMBER: &str = "PL!-pb2-002-PP"; // 絢瀬絵里, BiBi
const ENERGY: &str = "LL-E-001-SD";
const FILLER: &str = "PL!-sd1-010-SD";
const OPPONENT: &str = "PL!S-bp5-012-N";

fn fill_decks(game: &mut TestGame) {
    let f = game.id(FILLER);
    for _ in 0..12 {
        game.state.player1.main_deck.cards.push(f);
        game.state.player2.main_deck.cards.push(f);
    }
}

/// Stage ERI, wait an opponent member (the trigger), and fire the 自動 with the
/// choice pending. Three energy cards sit in the zone and none are active, so
/// 「2枚」 is observable.
fn fired() -> TestGame {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    fill_decks(&mut game);

    let eri = game.id(ERI);
    game.assert_card_identity(eri, ERI);
    game.assert_card_in_group(game.id(BIBI_MEMBER), "BiBi", "branch-1 target");

    game.state.player1.stage.stage[MemberArea::LeftSide as usize] = eri;
    game.state.player1.stage.stage[MemberArea::Center as usize] = game.id(BIBI_MEMBER);
    game.state.player2.stage.stage[MemberArea::Center as usize] = game.id(OPPONENT);
    game.state.mods.add_orientation_modifier(game.id(OPPONENT), "wait");

    for _ in 0..3 {
        game.state.player1.energy_zone.cards.push(game.id(ENERGY));
    }

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

/// A choice is presented when the trigger fires.
#[test]
fn a_choice_is_presented_on_the_trigger() {
    let mut game = fired();
    assert!(
        game.has_pending_choice(),
        "以下から1つを選択 must present a prompt"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "the two-way fork is presented as a target-style choice"
    );
    game.select_option(0);
}

/// Branch 2: exactly two energy cards activate, and nothing else is asked.
#[test]
fn option_two_activates_two_energy() {
    let mut game = fired();
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "precondition: no energy is active before the choice"
    );
    game.select_option(1);

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "エネルギーを2枚アクティブにする — exactly two energy cards became active"
    );
    assert!(
        !game.has_pending_choice(),
        "this branch needs no follow-up prompt; got {:?}. A prompt here would mean \
         branch 1 leaked in.",
        game.pending_choice_type()
    );
}

/// Choosing branch 2 must not activate a member — that is branch 1's job.
#[test]
fn option_two_activates_no_member() {
    let mut game = fired();
    let bibi = game.id(BIBI_MEMBER);
    game.select_option(1);

    assert_ne!(
        game.state.mods.get_orientation_modifier(bibi),
        Some("active"),
        "エネルギーを2枚 branch must not also activate the BiBi member — \
         「以下から1つを選ぶ」 means exactly one branch applies"
    );
}
