//! 絢瀬絵里 `PL!-pb2-011-P＋` / `-R` (常時)
//!
//! このメンバーの下に置かれている『BiBi』のメンバーカード1枚につき、{{icon_blade.png|ブレード}}を得る。
//!
//! Two constraints stack: the card must be physically under THIS member, and it must
//! be a BiBi member. Each is isolated below, plus the "under a different member" case,
//! which distinguishes このメンバーの下 from any under-card.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const ERI: &str = "PL!-pb2-011-P＋";
/// BiBi members to place underneath.
const BIBI_ERI: &str = "PL!-bp5-011-N"; // 絢瀬絵里, BiBi
const BIBI_ERI2: &str = "PL!-bp6-002-P"; // 絢瀬絵里, BiBi, different print
const BIBI_MAKI: &str = "PL!-bp5-006-P"; // 西木野真姫, BiBi
/// Not BiBi.
const PRINTEMPS: &str = "PL!-bp6-017-N"; // 小泉花陽, Printemps, no ability
const AZALEA: &str = "PL!S-bp5-012-N"; // 松浦果南, AZALEA, no ability
/// A second PBLL02 member to hang a decoy under.
const OTHER_ERI: &str = "PL!-pb2-025-N"; // 東條 希, lily white

/// Stage 011 in the centre, then place each `under` card underneath it.
/// Returns 011's blade modifier.
fn blade_with_under(under: &[&str]) -> i16 {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let eri = game.id(ERI);
    game.assert_card_identity(eri, ERI);
    game.state.player1.stage.stage = [-1, eri, -1];
    for card_no in under {
        let id = game.id(card_no);
        game.state.player1.stage.place_under_card(MemberArea::Center, id);
    }
    game.state.recalculate_constants();
    game.state.mods.get_blade_modifier(eri)
}

#[test]
fn one_bibi_under_grants_one_blade() {
    assert_eq!(
        blade_with_under(&[BIBI_ERI]),
        1,
        "『BiBi』のメンバーカード1枚につき — one BiBi member under is +1 blade"
    );
}

#[test]
fn two_bibi_under_grant_two_blades() {
    assert_eq!(
        blade_with_under(&[BIBI_ERI, BIBI_MAKI]),
        2,
        "1枚につき scales — two BiBi members under are +2"
    );
}

#[test]
fn three_bibi_under_grant_three_blades() {
    assert_eq!(
        blade_with_under(&[BIBI_ERI, BIBI_MAKI, BIBI_ERI2]),
        3,
        "1枚につき keeps scaling to 3, including two prints of one character"
    );
}

#[test]
fn nothing_under_grants_nothing() {
    assert_eq!(
        blade_with_under(&[]),
        0,
        "このメンバーの下に置かれているカードが0枚 is +0"
    );
}

#[test]
fn non_bibi_member_under_is_not_counted() {
    assert_eq!(
        blade_with_under(&[PRINTEMPS]),
        0,
        "小泉花陽 is a Printemps member, not 『BiBi』, so it is excluded"
    );
}

#[test]
fn mixed_under_counts_only_bibi_members() {
    assert_eq!(
        blade_with_under(&[PRINTEMPS, BIBI_ERI, AZALEA, BIBI_MAKI]),
        2,
        "of four cards under her, only the two BiBi members count"
    );
}

/// A BiBi member under a DIFFERENT member is not under HER.
#[test]
fn bibi_under_a_different_member_is_not_counted() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let eri = game.id(ERI);
    let other = game.id(OTHER_ERI);
    game.state.player1.stage.stage = [other, eri, -1];

    // Hang the BiBi member under the LEFT slot, not under 011 in the centre.
    let bibi = game.id(BIBI_ERI);
    game.state
        .player1
        .stage
        .place_under_card(MemberArea::LeftSide, bibi);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(eri),
        0,
        "このメンバーの下 — a BiBi member under a different stage member does not count"
    );
}

/// BiBi members in hand / discard are not "under" her.
#[test]
fn bibi_members_elsewhere_are_not_counted() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let eri = game.id(ERI);
    game.state.player1.stage.stage = [-1, eri, -1];

    let hand_eri = game.id(BIBI_ERI);
    let discard_maki = game.id(BIBI_MAKI);
    game.state.player1.hand.cards.push(hand_eri);
    game.state.player1.waitroom.cards.push(discard_maki);
    // …and one genuinely underneath, to prove the fixture is not simply dead.
    game.state
        .player1
        .stage
        .place_under_card(MemberArea::Center, game.id(BIBI_ERI2));
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(eri),
        1,
        "only the card physically underneath counts; hand and discard BiBi members do not"
    );
}

#[test]
fn off_stage_grants_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let eri = game.id(ERI);
    game.state
        .player1
        .stage
        .place_under_card(MemberArea::Center, game.id(BIBI_ERI));
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(eri),
        0,
        "常時 — off stage she contributes no blade"
    );
}
