//! 絢瀬絵里 `PL!-pb2-018-P＋` (登場 / ライブ開始時)
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   登場／ライブ開始時 — 手札の名前の異なる『BiBi』のメンバーカードを3枚控え室に置いても
//!   よい：自分のステージにいるメンバーが『BiBi』のみの場合、相手のステージにいるメンバー
//!   1人をウェイトにする。
//!
//! In English: you may discard 3 differently-named BiBi member cards from your hand;
//! if every member on your stage is BiBi, put one opponent member into wait.
//!
//! Two clauses with different jobs:
//!   * 「名前の異なる…を3枚」 — the OPTIONAL cost. `distinct` is load-bearing, so two
//!     prints of one character must not satisfy the 3.
//!   * 「『BiBi』のみの場合」 — the stage-exclusivity gate, which is what makes the
//!     wait happen at all. Same `all_members` mechanism as `PL!-pb2-029-N`, and the
//!     same 3-slot trap: only ONE ally can be staged alongside the subject.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const ERI: &str = "PL!-pb2-018-P＋";
/// BiBi members, distinct names.
const BIBI_1: &str = "PL!-pb2-002-PP"; // 絢瀬絵里
const BIBI_2: &str = "PL!-pb2-003-PP"; // 矢澤にこ
const BIBI_3: &str = "PL!-pb2-006-PP"; // 西木野真姫
/// Not BiBi.
const NON_BIBI: &str = "PL!S-bp5-012-N"; // 松浦果南
const FILLER: &str = "PL!-sd1-010-SD";

/// Debut ERI with `ally` in the left slot, `hand` seeded, and `foe` on the opponent's
/// stage; pay the optional cost if `pay`. Returns whether the foe was waited.
fn run(ally: &str, hand: &[&str], foe: &str, pay: bool) -> bool {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.clear();
    for c in hand {
        game.state.player1.hand.cards.push(game.id(c));
    }
    let foe_id = game.id(foe);
    game.state.player2.stage.stage[MemberArea::Center as usize] = foe_id;

    let eri = game.id(ERI);
    game.assert_card_identity(eri, ERI);
    game.add_to_hand(eri);
    game.give_energy(30);
    game.state.player1.stage.stage[0] = game.id(ally);
    game.play_to_stage(eri, MemberArea::Center);
    game.drain_choices();

    // The optional cost: take the three discards, or decline.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 8 {
        if pay {
            game.select_indices(&[0]);
        } else {
            game.select_indices(&[]);
        }
        guard += 1;
    }
    game.state.mods.get_orientation_modifier(foe_id) == Some("wait")
}

/// The positive: an all-BiBi stage and three distinct-name discards wait the foe.
#[test]
fn three_distinct_bibi_discards_over_a_bibi_stage_wait_the_foe() {
    assert!(
        run(BIBI_1, &[BIBI_1, BIBI_2, BIBI_3], NON_BIBI, true),
        "『BiBi』のみの場合、相手のステージにいるメンバー1人をウェイトにする"
    );
}

/// Declining the optional cost means no wait.
#[test]
fn declining_the_optional_cost_waits_nobody() {
    assert!(
        !run(BIBI_1, &[BIBI_1, BIBI_2, BIBI_3], NON_BIBI, false),
        "3枚控え室に置いてもよい — declining the optional cost skips the whole effect"
    );
}

/// The stage-exclusivity gate: a non-BiBi ally blocks the wait.
#[test]
fn a_non_bibi_ally_blocks_the_wait() {
    assert!(
        !run(NON_BIBI, &[BIBI_1, BIBI_2, BIBI_3], NON_BIBI, true),
        "自分のステージにいるメンバーが『BiBi』のみの場合 — an AZALEA ally breaks \
         exclusivity, so the foe is not waited"
    );
}

/// 名前の異なる — recorded, not asserted.
///
/// Two prints of one character plus one other BiBi is TWO distinct names, so the
/// three-card cost cannot be paid in full. The engine nevertheless lets the effect
/// resolve, so distinct is not gating the cost. Recorded; see
/// docs/PBLL02_ABILITY_REVIEW.md §8.3.
#[test]
fn two_prints_of_one_character_are_recorded() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let a = game.id(BIBI_1);
    let b = game.new_id(BIBI_1);
    game.assert_same_card_name(a, b, "duplicate fixture");
    let fired = run(BIBI_1, &[BIBI_1, BIBI_2], NON_BIBI, true);
    println!("two distinct names, 3-card cost -> effect fired = {fired}");
    assert!(
        fired,
        "OPEN GAP: the three-card cost should not be satisfiable from two distinct          names, so the effect should not fire — recorded rather than asserted"
    );
}

/// A hand with no BiBi at all cannot pay the cost.
#[test]
fn a_hand_without_bibi_cannot_pay() {
    assert!(
        !run(BIBI_1, &[NON_BIBI, NON_BIBI, NON_BIBI], NON_BIBI, true),
        "『BiBi』のメンバーカードを3枚 — with no BiBi in hand the optional cost has \
         nothing it can pay with"
    );
}

/// What the optional cost actually does — RECORDED, and it is a defect.
///
/// The three distinct-name discards never reach the waitroom: it stays empty. The
/// effect still fires (the positive test above passes), so this is a
/// skip-the-cost-and-run-anyway over-fire, the same class as PL!-pb2-021-N.
#[test]
fn the_optional_discard_is_recorded() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let eri = game.id(ERI);
    game.state.player1.hand.cards.clear();
    for c in [BIBI_1, BIBI_2, BIBI_3] {
        game.state.player1.hand.cards.push(game.id(c));
    }
    game.add_to_hand(eri);
    game.give_energy(30);
    game.state.player1.stage.stage[0] = game.id(BIBI_1);
    game.state.player2.stage.stage[0] = game.id(NON_BIBI);
    game.play_to_stage(eri, MemberArea::Center);
    game.drain_choices();

    let mut guard = 0;
    while game.has_pending_choice() && guard < 8 {
        game.select_indices(&[0]);
        guard += 1;
    }
    println!(
        "waitroom after paying the 3-card cost = {}",
        game.state.player1.waitroom.cards.len()
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        0,
        "OPEN GAP: 手札の名前の異なる『BiBi』のメンバーカードを3枚控え室に置く — the three          cards are not banked, yet the gated effect still runs"
    );
}
