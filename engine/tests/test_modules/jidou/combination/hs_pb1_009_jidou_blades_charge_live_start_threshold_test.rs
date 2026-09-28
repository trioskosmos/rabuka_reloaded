//! PL!HS-pb1-009 日野下花帆 — the deck's cleanest **L1 feeder** pair (see
//! `docs/JIDOU_COMBINATION_WORK.md` §2, and the engine-bug register there).
//!
//!   * ab#0 自動 センター ターン2回 「自分のステージに『蓮ノ空』のメンバーが
//!     登場するたび、ライブ終了時まで、ブレード2を得る。」
//!   * ab#1 ライブ開始時 「このメンバーが持つブレードの数が8つ以上の場合、
//!     カードを2枚引き、手札を1枚控え室に置く。」
//!
//! 花帆 prints ブレード 4. The jidou grants +2 per 『蓮ノ空』 appearance, twice a
//! turn, so 4 printed + 2 + 2 = 8 is EXACTLY the sibling's ブレード ≥ 8 threshold,
//! and the jidou is the only route to it. The pair is what this file exercises.
//!
//! ## What is fixed and what is blocked
//!
//! THREE engine bugs were found while working this pair. All three are now fixed
//! (see `docs/JIDOU_COMBINATION_WORK.md` §6):
//!
//! 1. `evaluate_card_blade_condition` read only `selected_cards`, which is EMPTY
//!    for an auto-triggered ライブ開始時, so 「このメンバーが持つブレード」 had no
//!    subject and returned false. It now falls back to the activating card.
//! 2. The re-scan guard `just_completed_ability_key` leaked across actions, so
//!    the ターン2回 jidou fired only ONCE per turn. The guard is now cleared at
//!    the end of the batch.
//! 3. Appearance detection was turn-lenient, so a self-scoped 自動 could fire on
//!    a *foreign* debut. It now reads the enqueue-time appearance snapshot
//!    (`AbilityQueueEntry::trigger_appeared_cards`), the mirror of the existing
//!    `entry_trigger_moved_cards` fallback.
//!
//! With all three fixed, the FULL L1 chain is exercised below: two appearances
//! charge 4 → 6 → 8, and ab#1 then draws. Before fixes 2 and 3 the second
//! appearance was swallowed and she topped out at 6, leaving ab#1 unreachable in
//! a real game.
//!
//! The two tests below assert the parts that ARE reachable and correct: the
//! jidou grants ブレード+2 to the HOST on a real 『蓮ノ空』 appearance, and ab#1
//! stays correctly silent below its ブレード ≥ 8 threshold (reading the host's
//! effective blade, not a hardcoded failure).
use crate::helpers::*;
use crate::test_modules::support::baton_swap_auto_helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const HANAMO: &str = "PL!HS-pb1-009-R";
const HASU_ALLY: &str = "PL!HS-sd1-001-SD";
const HASU_ALLY_2: &str = "PL!HS-sd1-012-SD";
const FILLER: &str = "PL!-sd1-010-SD";

/// 花帆's TOTAL blade, i.e. what the sibling's ブレード ≥ 8 condition reads: the
/// printed value plus every granted blade modifier. Reading the print from the
/// card database (rather than hard-coding 4) makes this a precondition check: if
/// the card data changes, the test says so instead of silently testing a
/// different threshold.
fn total_blade(game: &TestGame, cid: i16) -> i32 {
    let printed = game.db.get_card(cid).map(|c| c.blade as i32).unwrap_or(0);
    printed + game.state.mods.get_blade_modifier(cid)
}

fn hand_len(game: &TestGame) -> usize {
    game.state.player1.hand.cards.len()
}

fn deck_len(game: &TestGame) -> usize {
    game.state.player1.main_deck.cards.len()
}

/// Fire ab#1 (ライブ開始時) through the real queue and drain its choices.
/// `fire_trigger` enqueues the ability with the card as the activating card —
/// the same entry the live-start dispatch builds — so the condition is
/// evaluated against the real board.
fn fire_live_start(game: &mut TestGame, hanamo: i16) {
    fire_trigger(game, hanamo, AbilityTrigger::LiveStart, "ライブ開始時");
}

fn drain_live_start_choices(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 12 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectTarget { target, .. } if target == "conditional_optional" => {
                game.select_choice_option(1);
            }
            Choice::SelectTarget { .. } => game.select_option(0),
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { count, .. } => {
                if *count > 0 {
                    game.select_indices(&[0]);
                } else {
                    game.select_indices(&[]);
                }
            }
            _ => game.select_indices(&[]),
        }
    }
}

/// Stage 花帆 in センター with a deep deck, then hand her `n` Hasunosora allies.
/// Returns the ally ids in hand order. センター is asserted because ab#0 is
/// センター-gated: a side-slot fixture would make every blade expectation fail
/// for the wrong reason.
fn stage_hanamo_with_allies(game: &mut TestGame, ally_count: usize) -> (i16, Vec<i16>) {
    let hanamo = game.id(HANAMO);
    game.assert_card_identity(hanamo, HANAMO);
    game.assert_card_in_group(hanamo, "蓮ノ空", "花帆 is a 『蓮ノ空』 member");
    game.assert_card_cost(hanamo, 15);
    game.assert_blade(hanamo, 0, "no granted blades before any appearance");

    let filler = game.id(FILLER);
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.add_to_stage(MemberArea::Center, hanamo);
    game.give_energy(30);

    let mut allies = Vec::new();
    for i in 0..ally_count {
        let ally = if i % 2 == 0 {
            game.new_id(HASU_ALLY)
        } else {
            game.new_id(HASU_ALLY_2)
        };
        game.assert_card_in_group(ally, "蓮ノ空", "the appearing ally is a 『蓮ノ空』 member");
        game.add_to_hand(ally);
        allies.push(ally);
    }
    // A hand card that is NOT a live, so ab#1's 手札を1枚控え室に置く has material
    // that is not one of the two it would draw.
    game.add_to_hand(game.new_id(FILLER));

    assert_eq!(
        game.state.player1.stage.stage[1],
        hanamo,
        "precondition: 花帆 must be in センター — ab#0 is センター-gated"
    );
    (hanamo, allies)
}

/// Play one ally through the real action path; the engine's own TAS scan decides
/// whether ab#0 fires. `play_to_stage` already runs the full `process_action`
/// path (scan + resolve), so no extra scan is added.
fn play_ally(game: &mut TestGame, ally: i16) {
    let area = if game.state.player1.stage.stage[0] == -1 {
        MemberArea::LeftSide
    } else {
        MemberArea::RightSide
    };
    game.play_to_stage(ally, area);
}

// ====================================================================
// The reachable half: the jidou's grant lands on the host, and the
// sibling stays silent below its ブレード ≥ 8 threshold.
// ====================================================================

/// ONE 『蓮ノ空』 appearance grants ブレード+2 to the HOST (4 printed → 6), and
/// 6 < 8 leaves ab#1 silent — no draw, no discard.
///
/// This is the L1 link in its negative form: the jidou's grant must land on
/// 花帆 (the only member ab#1's 「このメンバーが持つブレード」 counts) and must be
/// read by ab#1's real condition. ab#1 fires as a no-op because the threshold
/// genuinely fails, which is the same observable a dropped ブレード condition
/// would produce — the positive (threshold-met) assertion is the part blocked by
/// the guard bug documented in the module header, not this one.
#[test]
fn hs_pb1_009_one_appearance_grants_blade_to_host_and_live_start_stays_below_threshold() {
    let mut game = TestGame::new(load_real_database());
    let (hanamo, allies) = stage_hanamo_with_allies(&mut game, 1);

    assert_eq!(
        total_blade(&game, hanamo),
        4,
        "precondition: 花帆 prints ブレード 4 and has granted nothing yet"
    );

    play_ally(&mut game, allies[0]);

    // The +2 is on the HOST, not on the appearing ally — ab#1 counts
    // 「このメンバーが持つブレード」, so the grant's recipient is load-bearing.
    assert_eq!(
        total_blade(&game, hanamo),
        6,
        "one 『蓮ノ空』 appearance: ab#0 granted ブレード+2 to 花帆 (ターン2回, 1 of 2)"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(allies[0]),
        0,
        "the appearing ally must NOT receive the blades; only the センター host does"
    );

    let hand_before = hand_len(&game);
    let deck_before = deck_len(&game);
    fire_live_start(&mut game, hanamo);
    drain_live_start_choices(&mut game);

    assert_eq!(
        deck_len(&game),
        deck_before,
        "6 < 8: ab#1 must not draw — カードを2枚引き is gated on the printed ブレード数"
    );
    assert_eq!(
        hand_len(&game),
        hand_before,
        "6 < 8: the whole conditional sequential stays silent, discard included"
    );
}

/// The lowest case: with NO appearance at all she is at her printed 4, and ab#1
/// is still silent. Separates "condition evaluated and failed" from a silent
/// no-op and pins the printed-blade baseline the jidou charges against.
#[test]
fn hs_pb1_009_printed_blade_alone_does_not_fire_live_start() {
    let mut game = TestGame::new(load_real_database());
    let (hanamo, _allies) = stage_hanamo_with_allies(&mut game, 0);

    assert_eq!(
        total_blade(&game, hanamo),
        4,
        "precondition: her printed ブレード 4 is the whole blade count here"
    );

    let deck_before = deck_len(&game);
    fire_live_start(&mut game, hanamo);
    drain_live_start_choices(&mut game);

    assert_eq!(
        deck_len(&game),
        deck_before,
        "printed 4 alone is not ≥ 8: ab#1 must not draw. This is the baseline the \
         jidou has to lift off for the pair to matter at all."
    );
}

// ====================================================================
// 1. THE THRESHOLD — the jidou is the only route to ブレード ≥ 8
// ====================================================================

/// Two Hasunosora appearances charge 4 printed + 4 jidou = 8, and ab#1 fires:
/// 2 drawn, 1 discarded.
///
/// This is the full L1 link, and it is the acceptance criterion for the re-scan
/// guard fix (Bug B). The ターン2回 jidou must fire on BOTH appearances; before
/// the guard was scoped to one batch, the second appearance was swallowed by the
/// leaked key and she topped out at 6.
#[test]
fn hs_pb1_009_two_appearances_reach_the_live_start_blade_threshold() {
    let mut game = TestGame::new(load_real_database());
    let (hanamo, allies) = stage_hanamo_with_allies(&mut game, 2);

    assert_eq!(
        total_blade(&game, hanamo),
        4,
        "precondition: 花帆 prints ブレード 4 and has granted nothing yet"
    );

    play_ally(&mut game, allies[0]);
    assert_eq!(
        total_blade(&game, hanamo),
        6,
        "one 『蓮ノ空』 appearance: ab#0 grants ブレード+2 (ターン2回, 1 of 2 used)"
    );

    play_ally(&mut game, allies[1]);
    assert_eq!(
        total_blade(&game, hanamo),
        8,
        "two appearances: 4 printed + 2 + 2 = 8, exactly ab#1's ブレード ≥ 8 \
         threshold. The SECOND appearance firing is the Bug B regression guard: a \
         leaked re-scan key used to swallow it and cap her at 6."
    );

    // The link observable: ab#1 only resolves because ab#0 charged her.
    let hand_before = hand_len(&game);
    let deck_before = deck_len(&game);
    fire_live_start(&mut game, hanamo);
    drain_live_start_choices(&mut game);

    assert_eq!(
        deck_before - deck_len(&game),
        2,
        "ab#1: カードを2枚引き — both cards came off the deck. This grant is only \
         reachable because the jidou charged her to the threshold."
    );
    assert_eq!(
        hand_len(&game),
        hand_before + 1,
        "ab#1: +2 drawn − 1 discarded to 控え室 = net +1 in hand"
    );
}

/// The ブレード ≥ 8 read is `このメンバーが持つ`, so the blades must be hers. A
/// jidou that granted them to a DIFFERENT member would leave her at 4 and ab#1
/// silent; this pins that the grant lands on the host.
#[test]
fn hs_pb1_009_blades_land_on_the_host_so_the_condition_reads_them() {
    let mut game = TestGame::new(load_real_database());
    let (hanamo, allies) = stage_hanamo_with_allies(&mut game, 2);
    play_ally(&mut game, allies[0]);
    play_ally(&mut game, allies[1]);

    // The grant is on 花帆, not on whichever member appeared. EXACT, not `>= 4`:
    // two appearances at ブレード+2 is precisely 4, and a non-exact bound is the
    // shape `jidou_test_audit.py --sweep d` exists to flag.
    assert_eq!(
        game.state.mods.get_blade_modifier(hanamo),
        4,
        "ab#0's 4 granted blades belong to 花帆, because ab#1 counts \
         「このメンバーが持つブレード」"
    );
    for &ally in &allies {
        assert_eq!(
            game.state.mods.get_blade_modifier(ally),
            0,
            "the appearing 『蓮ノ空』 member must not receive the blades — only the \
             センター host does"
        );
    }
    assert_eq!(total_blade(&game, hanamo), 8);
}

/// The per-turn allowance REOPENS next turn: two appearances in the second turn
/// both grant +2. The per-turn use counter is keyed by turn number, so advancing
/// the turn is the real mechanism that frees the allowance.
#[test]
fn hs_pb1_009_turn_two_reopens_the_allowance_so_two_more_appearances_grant_plus_four() {
    let mut game = TestGame::new(load_real_database());
    let (hanamo, allies) = stage_hanamo_with_allies(&mut game, 3);

    // Turn 1: one appearance uses 1 of the 2 allowances.
    play_ally(&mut game, allies[0]);
    assert_eq!(
        total_blade(&game, hanamo),
        6,
        "turn 1, one appearance: 4 printed + 2 granted"
    );

    // Roll the turn. `deployed_this_turn` is cleared by the real turn-start
    // machinery, which frees the baton-touch lock for the turn-2 swap.
    game.state.turn_number += 1;
    game.state.player1.deployed_this_turn.clear();

    // Turn 2, appearance #1: play into the free right slot.
    play_ally(&mut game, allies[1]);
    assert_eq!(
        total_blade(&game, hanamo),
        8,
        "turn 2, first appearance grants a fresh +2 — the allowance reopened"
    );

    // Turn 2, appearance #2: baton-touch the turn-1 member (now unlocked).
    let third = game.new_id(HASU_ALLY_2);
    replace_member_by_baton_touch(&mut game, allies[0], third, MemberArea::LeftSide);
    game.drain_auto_ability_choices();

    assert_eq!(
        total_blade(&game, hanamo),
        10,
        "turn 2 allowed BOTH appearances (+2 each, 4 printed + 6 granted = 10). If \
         the per-turn allowance had NOT reset, this second turn-2 appearance would \
         be the third use overall and be refused, leaving 8."
    );
}
