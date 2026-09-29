//! 小泉花陽 `PL!-pb2-017-P＋` (登場)
//!
//! 自分の控え室にある『Printemps』のメンバーカード4枚をこのメンバーの下に置く。
//!
//! A plain filtered move: 4 members, Printemps only, from the waitroom, under THIS
//! member. The interesting axes are the count, the group filter, the source zone and
//! 「このメンバーの下」 (under_self — another member's under-cards must not change).

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const NOZOMI: &str = "PL!-pb2-017-P＋";
/// Printemps members, cheapest first, with no abilities of their own.
const P_1: &str = "PL!-bp6-017-N"; // 小泉花陽
const P_2: &str = "PL!-bp5-012-N"; // 南 ことり
const P_3: &str = "PL!-pb1-026-N";
const P_4: &str = "PL!-sd1-010-SD"; // 高坂 穂乃果
const P_5: &str = "PL!-pb2-030-N";
/// Not Printemps.
const NON_P: &str = "PL!S-bp5-012-N"; // 松浦果南, AZALEA

/// Debut NOZOMI with `waitroom` loaded, and return her under-card count.
fn under_count_after_debut(waitroom: &[&str]) -> usize {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    for card_no in waitroom {
        game.state.player1.waitroom.add_card(game.id(card_no));
    }
    let nozomi = game.id(NOZOMI);
    game.assert_card_identity(nozomi, NOZOMI);
    game.state.player1.hand.cards.push(nozomi);
    game.give_energy(60);
    game.play_to_stage(nozomi, MemberArea::Center);
    // With more candidates than the count requires, the move asks WHICH four; answer
    // it so the assertion measures the placement and not an unanswered prompt.
    if game.has_pending_choice() {
        game.select_indices(&[0, 1, 2, 3]);
    }
    game.drain_auto_ability_choices();
    game.state.player1.stage.get_under_cards(MemberArea::Center).len()
}

#[test]
fn four_printemps_members_go_under_her() {
    assert_eq!(
        under_count_after_debut(&[P_1, P_2, P_3, P_4]),
        4,
        "自分の控え室にある『Printemps』のメンバーカード4枚をこのメンバーの下に置く"
    );
}

/// Only 4 are taken even when more are available.
#[test]
fn only_four_are_taken() {
    assert_eq!(
        under_count_after_debut(&[P_1, P_2, P_3, P_4, P_5]),
        4,
        "4枚 — the count is exact, so a deeper waitroom does not all go under"
    );
}

/// 「4枚」 is MANDATORY (not 「4枚まで」), so with fewer than four available the
/// effect cannot pay and nothing is placed. Distinct from a 「〜まで」 clause, which
/// this is not.
#[test]
fn fewer_than_four_places_nothing() {
    assert_eq!(
        under_count_after_debut(&[P_1, P_2]),
        0,
        "4枚を…置く is a requirement, not an upper bound — with only two available the \
         ability cannot resolve and no card is placed"
    );
}

#[test]
fn an_empty_waitroom_puts_nothing_under() {
    assert_eq!(
        under_count_after_debut(&[]),
        0,
        "no members in the waitroom, so nothing to place"
    );
}

/// The group filter: non-Printemps members must not be taken.
#[test]
fn non_printemps_members_are_not_taken() {
    assert_eq!(
        under_count_after_debut(&[NON_P, NON_P, NON_P, NON_P]),
        0,
        "『Printemps』のメンバーカード — AZALEA members in the waitroom are excluded"
    );
}

#[test]
fn a_mix_takes_only_the_printemps_members() {
    assert_eq!(
        under_count_after_debut(&[P_1, NON_P, P_2, NON_P, P_3, P_4]),
        4,
        "of six waitroom members, the four Printemps ones are taken and the two \
         AZALEA ones are left behind"
    );
}

/// 「このメンバーの下」 — cards under a different member are not affected, and her own
/// placement is what creates the under area.
#[test]
fn it_is_her_own_under_area() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let other = game.id(P_1);
    game.state.player1.stage.stage[MemberArea::LeftSide as usize] = other;
    for c in [P_2, P_3, P_4, P_5] {
        game.state.player1.waitroom.add_card(game.id(c));
    }
    let nozomi = game.id(NOZOMI);
    game.state.player1.hand.cards.push(nozomi);
    game.give_energy(60);
    game.play_to_stage(nozomi, MemberArea::Center);
    game.drain_auto_ability_choices();

    assert_eq!(
        game.state.player1.stage.get_under_cards(MemberArea::Center).len(),
        4,
        "precondition: the four members landed under her"
    );
    assert_eq!(
        game.state.player1.stage.get_under_cards(MemberArea::LeftSide).len(),
        0,
        "under_self — the cards belong under the member who gained the ability, not \
         under whoever else is on the stage"
    );
}
