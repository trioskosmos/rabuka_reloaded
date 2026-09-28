//! PL!SP-bp5-005-R＋ 葉月 恋 — a strong **L1/L2 shared-batch** pair (see
//! `docs/JIDOU_COMBINATION_WORK.md` §2). The 起動 and the 自動 read the *same*
//! batch of freshly-milled cards, so the pair is a single event seen twice.
//!
//!   * ab#0 起動 ターン1回 「デッキの上からカードを3枚控え室に置く：ライブ終了時
//!     まで、これにより控え室に置いた『Liella!』のメンバーカード1枚につき、
//!     ブレードを得る。」
//!   * ab#1 自動 ターン1回 「自分のメインフェイズの間、自分のカードが1枚以上
//!     いずれかの区域から控え室に置かれるたび、E支払ってもよい。そうした場合、
//!     それらのカードの中から1枚手札に加える。」
//!
//! ## The link
//!
//! The 起動 mills 3 cards into 控え室. That single placement is what arms ab#1:
//! 「自分のカードが…控え室に置かれる」. ab#1's conditional_action then recovers
//! one of 「それらのカード」 — and 「それ」 is *exactly* the 3 the 起動 milled,
//! the same batch ab#0 counts for ブレード. One event, two readings:
//!
//!   * ab#0 reads the batch as a **count** (Liella! members among the 3).
//!   * ab#1 reads the batch as a **pool** (choose 1 of the 3 to recover).
//!
//! ## What the single-ability tests miss
//!
//! `effects/gain/blades/per_card/hazuki_activate_discard_per_liella_member_blade_test.rs`
//! drives ab#0 and asserts only the ブレード count. It never arms ab#1, so it
//! cannot catch ab#1 recovering a card from the WRONG batch (a different mill, a
//! different zone, or a card that was not actually milled) — the defining failure
//! of this pair. These tests assert the recovered card is a member of the exact
//! 3-card set the 起動 milled.
//!
//! One mill = one jidou firing, so this file is independent of the ターン2回
//! re-scan-guard bug (Bug B, §6).

use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

const REN: &str = "PL!SP-bp5-005-R＋";
/// Two DISTINCT Liella! members so the milled set is identifiable by id and the
/// ブレード count is 2 (both milled members are Liella!). If these were the same
/// card id the "recovered card ∈ milled set" assertion could not distinguish the
/// members from each other.
/// 唐 可可. Was `"PL!SP-bp1-002-R"`, which drops the fullwidth `＋` — the real
/// print is `PL!SP-bp1-002-R＋`, and the other prints of this card are `-P` and
/// `-SEC`. `get_card_id`'s lenient fallback silently substituted a different
/// print, so this staged a bystander member by accident. The assertions still
/// held (it is only used to make the milled set identifiable by id), which is
/// why the typo survived.
const LIELLA_A: &str = "PL!SP-bp1-002-R＋";
const LIELLA_B: &str = "PL!SP-bp1-004-R";
/// A non-Liella card, so the mill's 3 = 2 Liella + 1 other and ab#0's count (2)
/// and ab#1's pool (3) genuinely differ.
const OTHER: &str = "PL!-sd1-010-SD";

/// Drain ab#1's pay-E + recover choices. `pay` = true pays the E and recovers;
/// false declines. Returns true if a choice was actually present.
fn resolve_jidou(game: &mut TestGame, pay: bool, pick: usize) -> bool {
    if !game.has_pending_choice() {
        return false;
    }
    // ab#1's optional gate: SelectTarget, option 1 = pay E, option 0 = skip.
    match game.get_pending_choice() {
        rabuka_engine::ability::types::Choice::SelectTarget { .. } => {
            game.select_choice_option(if pay { 1 } else { 0 });
        }
        _ => return false,
    }
    if !pay {
        return true;
    }
    // Then the "choose 1 of those cards" selection.
    if game.has_pending_choice() {
        match game.get_pending_choice() {
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                game.select_indices(&[pick]);
            }
            _ => game.select_indices(&[]),
        }
    }
    // Any trailing prompts.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 8 {
        guard += 1;
        match game.get_pending_choice() {
            rabuka_engine::ability::types::Choice::SelectCard { .. } => game.select_indices(&[0]),
            _ => game.select_indices(&[]),
        }
    }
    true
}

/// Fixture: 恋 on stage, deck top-3 = [LiellaA, LiellaB, OTHER] so the milled
/// set is exactly these three ids and ab#0's blade count is 2.
fn fixture(game: &mut TestGame) -> (i16, i16, i16, i16) {
    let ren = game.id(REN);
    game.assert_card_identity(ren, REN);
    let a = game.id(LIELLA_A);
    let b = game.id(LIELLA_B);
    let o = game.id(OTHER);
    game.assert_card_in_group(a, "Liella!", "milled A is a 『Liella!』 member");
    game.assert_card_in_group(b, "Liella!", "milled B is a 『Liella!』 member");

    game.add_to_stage(MemberArea::Center, ren);
    // Main deck: top three are the milled cards; pad below so the draw from the
    // jidou (if any) and the 起動 don't run off the end.
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(a);
    game.state.player1.main_deck.cards.push(b);
    game.state.player1.main_deck.cards.push(o);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(o);
    }
    game.give_energy(13);
    (ren, a, b, o)
}

// ====================================================================
// The shared-batch link: ab#0 counts the 3, ab#1 recovers one of the 3.
// ====================================================================

/// Activate the 起動 → 3 cards milled to 控え室 → ab#0 gains 2 ブレード (2 Liella
/// members) AND ab#1 (the 自動) fires on that same placement; paying E recovers
/// one of exactly those 3 milled cards to hand.
///
/// The two assertions are the link: the ブレード count (ab#0) and the recovered
/// card (ab#1) are computed from the SAME milled batch. A jidou that recovered a
/// card outside the mill (or a test that ignored it) would fail the "recovered ∈
/// {A,B,O}" check.
#[test]
fn sp_bp5_005_mill_arms_jidou_recover_and_its_blade_counts_the_same_batch() {
    let mut game = TestGame::new(load_real_database());
    let (ren, a, b, o) = fixture(&mut game);

    let hand_before = game.state.player1.hand.cards.len();

    // Activate the 起動 (mills a, b, o into waitroom) via the real action path.
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(ren),
        None,
        None,
        None,
    )
    .expect("activate 恋's 起動");

    // ab#0: 2 Liella! members among the 3 milled → 2 blade on 恋.
    let blade = game
        .state
        .mods
        .blade_modifiers
        .get(&ren)
        .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
    assert_eq!(
        blade, 2,
        "ab#0: the 起動 milled 2 『Liella!』 members → 2 ブレード. This reads the \
         SAME batch ab#1 will recover from."
    );

    // The 3 milled are now in 控え室 (not hand, not deck).
    for (card, label) in [(a, "A"), (b, "B"), (o, "O")] {
        assert!(
            game.state.player1.waitroom.cards.contains(&card),
            "milled {label} must be in 控え室 before ab#1 can recover it"
        );
    }

    // ab#1: the mill is 「自分のカードが…控え室に置かれる」, so the 自動 fires and
    // offers to pay E. Pay it, then recover the FIRST milled card (A).
    let offered = resolve_jidou(&mut game, true, 0);
    assert!(
        offered,
        "ab#1 must fire on the 起動's own mill of 3 cards into 控え室. If this is \
         absent, the 自動 and the 起動 are not actually coupled — the pair's whole \
         point is that the mill arms the recover."
    );

    // The recovered card is one of EXACTLY the 3 the 起動 milled.
    let hand: Vec<i16> = game.state.player1.hand.cards.to_vec();
    let recovered: Vec<i16> = hand.iter().copied().filter(|c| [a, b, o].contains(c)).collect();
    assert_eq!(
        recovered.len(),
        1,
        "ab#1: paying E adds 1 of 「それらのカード」 to hand. Exactly one of the 3 \
         milled cards must be recovered — hand now holds {:?}, milled set was \
         {{{}, {}, {}}}. A card from outside the mill would mean ab#1 read the \
         wrong batch.",
        hand, a, b, o
    );
    let got = recovered[0];
    assert!(
        !game.state.player1.waitroom.cards.contains(&got),
        "the recovered card must have left 控え室 for 手札"
    );
    assert_eq!(
        hand.len(),
        hand_before + 1,
        "ab#1 recovers exactly 1 card to hand"
    );
}

/// DECLINE ab#1's E payment: ab#0's ブレード is unaffected (it is unconditional and
/// reads the same mill), but nothing is recovered to hand.
///
/// This is the independence check the two halves must satisfy: the 起動's ブレード
/// is earned by the mill regardless of whether the player spends E on ab#1, so a
/// jidou bug that suppressed or double-counted the blade would show up here.
#[test]
fn sp_bp5_005_declining_jidou_recover_keeps_the_mills_blade_but_recovers_nothing() {
    let mut game = TestGame::new(load_real_database());
    let (ren, a, b, o) = fixture(&mut game);

    let hand_before = game.state.player1.hand.cards.len();

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(ren),
        None,
        None,
        None,
    )
    .expect("activate 恋's 起動");

    // ab#1 is offered (the mill happened); decline it.
    let offered = resolve_jidou(&mut game, false, 0);
    assert!(
        offered,
        "ab#1 must be offered after the 起動 mills into 控え室, even though the \
         player will decline the E"
    );

    let blade = game
        .state
        .mods
        .blade_modifiers
        .get(&ren)
        .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
    assert_eq!(
        blade, 2,
        "declining ab#1 must not change ab#0's ブレード — the 起動's blade is earned \
         by the mill itself and is independent of the recover choice."
    );

    let hand: Vec<i16> = game.state.player1.hand.cards.to_vec();
    assert!(
        !hand.contains(&a) && !hand.contains(&b) && !hand.contains(&o),
        "declining the E recovers nothing: hand {:?} must contain none of the milled \
         set {{{}, {}, {}}}",
        hand, a, b, o
    );
    assert_eq!(
        hand.len(),
        hand_before,
        "hand unchanged when ab#1 is declined"
    );
}
