//! 僕たちはひとつの光 `PL!-pb2-039-L` (ライブ開始時)
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   ライブ開始時 — 自分の成功ライブカード置き場に『μ's』のカードが2枚以上ある場合、
//!   ライブ終了時まで、エールによって公開される自分のカードの枚数が10枚増える。
//!
//! In English: while two or more μ's cards sit in your success live zone, the cheer
//! requires 10 more cards to be revealed for the rest of the live.
//!
//! # What the number means, and a correction
//!
//! `cheer_checks_required` IS the number of cards the player must reveal:
//! `perform_cheer_check` draws one card per check and completes the cheer when
//! `cheer_checks_done >= cheer_checks_required`. So 「10枚増える」 means the
//! requirement goes UP by 10, which is what the engine does.
//!
//! I previously reported this card as applying the bonus with an inverted sign, and
//! as ignoring the 『μ's』 filter. Both were my test:
//!   * I assumed "more cards revealed" must mean FEWER required checks, and asserted
//!     the requirement should fall. It should rise.
//!   * my "non-μ's" control, `PL!-sd1-019-SD`, has `unit = None` but
//!     `series = ラブライブ！`, and `card_series_matches_group` therefore makes it a
//!     μ's card. So is my second attempt, `LL-PR-004-PR` (愛♡スクリ～ム！) — same bare
//!     `series`. 『μ's』 is read as "a series line containing ラブライブ！ with no
//!     qualifier", so only a card whose series carries one (サンシャイン!! / 虹ヶ咲 /
//!     スーパースター / 蓮ノ空) can serve as the control.
//!
//! The convention is confirmed against the existing precedent
//! `PL!SP-bp2-010-R＋` Wien: 「8枚減る」 lowers the requirement by 8, so 「N枚増える」
//! raises it by N.
//!
//! Group-name spacing is pinned separately, in
//! `tests/test_modules/characterization/group_name_spacing_normalisation_test.rs`.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const HIKARI: &str = "PL!-pb2-039-L";
/// μ's lives — both the spaced `lily white` spelling.
const MUS_A: &str = "PL!-pb1-029-SRL"; // 知らないLove
const MUS_B: &str = "PL!-bp4-025-SRL"; // 微熱からMystery
/// Not μ's: an Aqours live, `series=ラブライブ！サンシャイン!!`.
const NON_MUS: &str = "PL!S-bp2-019-L"; // WATER BLUE NEW WORLD
const FILLER: &str = "PL!-sd1-010-SD";
/// Blade high enough that the +10 is visible.
const BASE_BLADE: u8 = 30;

fn fill_decks(game: &mut TestGame) {
    let f = game.id(FILLER);
    for _ in 0..12 {
        game.state.player1.main_deck.cards.push(f);
        game.state.player2.main_deck.cards.push(f);
    }
}

/// Load `zone` into our success live zone, run ライブ開始時, then a cheer check.
/// Returns the number of cards the cheer requires the player to reveal.
fn cheer_checks_with(zone: &[&str]) -> u8 {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    fill_decks(&mut game);
    for c in zone {
        game.state
            .player1
            .success_live_card_zone
            .add_card(game.id(c));
    }

    let hikari = game.id(HIKARI);
    game.assert_card_identity(hikari, HIKARI);
    game.state.player1.hand.cards.push(hikari);
    game.give_energy(20);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(hikari);

    let ability_id = {
        let card = game.db.get_card(hikari).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap();
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(hikari).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.activating_card = Some(hikari);
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(hikari),
        None,
        None,
    );
    game.state.process_pending_auto_abilities(&pid);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        game.select_indices(&[]);
        guard += 1;
    }

    let _ = game.state.perform_cheer_check(&pid, BASE_BLADE);
    game.state.cheer_checks_required
}

/// The threshold crossing, and the exact amount.
#[test]
fn two_mus_lives_in_the_success_zone_add_ten_yell_cards() {
    assert_eq!(
        cheer_checks_with(&[MUS_A]),
        BASE_BLADE,
        "『μ's』のカードが2枚以上 — one live is below the threshold, so the cheer is \
         unmodified"
    );
    assert_eq!(
        cheer_checks_with(&[MUS_A, MUS_B]),
        BASE_BLADE + 10,
        "公開される自分のカードの枚数が10枚増える — the cheer needs ten more cards \
         revealed than the blade alone would require"
    );
}

/// The filter is 『μ's』, so a non-μ's live must not count toward the threshold.
#[test]
fn two_non_mus_lives_do_not_cross_the_threshold() {
    assert_eq!(
        cheer_checks_with(&[NON_MUS, NON_MUS]),
        BASE_BLADE,
        "『μ's』のカードが2枚以上 — two non-μ's lives must not satisfy a μ's threshold"
    );
}

/// One of each: the non-μ's card does not count, so still only one μ's.
#[test]
fn a_mixed_zone_counts_only_the_mus_cards() {
    assert_eq!(
        cheer_checks_with(&[MUS_A, NON_MUS]),
        BASE_BLADE,
        "only the μ's card counts toward 『μ's』のカードが2枚以上"
    );
}

/// An empty zone is the same below-threshold case.
#[test]
fn an_empty_zone_leaves_the_cheer_unmodified() {
    assert_eq!(
        cheer_checks_with(&[]),
        BASE_BLADE,
        "no cards in the success live zone"
    );
}

/// 自分の成功ライブカード置き場 — the opponent's zone must not count.
#[test]
fn the_opponents_success_zone_does_not_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    fill_decks(&mut game);
    let hikari = game.id(HIKARI);
    game.state.player1.hand.cards.push(hikari);
    for c in [MUS_A, MUS_B] {
        game.state
            .player2
            .success_live_card_zone
            .add_card(game.id(c));
    }
    game.give_energy(20);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(hikari);
    let pid = game.state.player1.id.clone();
    let _ = game.state.perform_cheer_check(&pid, BASE_BLADE);

    assert_eq!(
        game.state.cheer_checks_required, BASE_BLADE,
        "自分の成功ライブカード置き場 — two μ's lives on the OPPONENT's side must not \
         trigger the bonus"
    );
}

/// A CORRECTION to a test that used to live here, recorded because it is a trap.
///
/// 春情ロマンティック `PL!-pb2-041-L` carries a counting weight —
/// 「みつの成功ライブカード置き場にあるカードの数を数えるとき、このカードは
/// 2枚として扱う」 — and this card was originally used as the CONSUMER to observe it,
/// because its threshold is also 『『μ′s」のカードが2枚以上ある場合」.
///
/// That is wrong: **039's `unit` is `None`**, so it is not a lily white card and
/// 「自分の『体のカードの効果によって』 can never match it. A card with no unit
/// is not part of any group, so the weight is correctly invisible to it.
///
/// 041′s own ライブ成功時 is the right consumer and has its own test file.
#[test]
fn this_card_is_not_a_lily_white_card_and_cannot_observe_the_weight() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let card = game.db.get_card(game.id("PL!-pb2-039-L")).unwrap();
    assert_eq!(
        card.unit.as_ref().map(|s| s.to_string()),
        None,
        "039 has NO unit, so it is in no group and can never be a 『lily white』 counter. \
         An earlier version of this file used it as the observer for PL!-pb2-041-L\'s \
         weight, which could never have worked."
    );
    assert!(
        !rabuka_engine::ability::util::card_matches_group_str(
            &game.db,
            game.id("PL!-pb2-039-L"),
            Some("lily white")
        ),
        "and the group matcher agrees: 039 is not a lily white card"
    );
}
