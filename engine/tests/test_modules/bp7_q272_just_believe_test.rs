/// Q272 — Just Believe!!! PL!N-bp7-026-L.
///
/// ab#0 (ライブ開始時): 手札を2枚まで控え室に置いてもよい：自分のステージにいる『虹ヶ咲』の
///   メンバーを、これにより控え室に置いたカードの枚数に等しい数まで選ぶ。ライブ終了時まで、
///   それらはブレードを得る。
/// ab#1 (ライブ成功時): エールにより公開された自分のカードの中に、ブレードハートを持たない
///   メンバーカードが2枚以上ある場合、このカードのスコアを＋１する。
///
/// Official QA Q272: 同じメンバーを複数回選ぶことはできますか？ → いいえ。
/// The stage-member selection offers DISTINCT 虹ヶ咲 members; a member can only be
/// selected once, so with fewer members than cards discarded, no member gets extra
/// blades.
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const JB: &str = "PL!N-bp7-026-L";
// 虹ヶ咲 members for the stage (ab#0 target).
const NIJI_A: &str = "PL!N-sd1-004-SD";
const NIJI_B: &str = "PL!N-sd1-006-SD";
// non-虹ヶ咲 member (should NOT be selectable).
const NON_NIJI: &str = "PL!-sd1-010-SD";
// member WITHOUT blade-heart / WITH blade-heart (ab#1).
const NO_BLADE_MEMBER: &str = "PL!-sd1-001-SD";
const WITH_BLADE_MEMBER: &str = "PL!-sd1-010-SD";
// live card (not a member) for ab#1.
const LIVE_CARD: &str = "PL!-sd1-020-SD";
const FILLER: &str = "PL!-sd1-010-SD";

fn blade(g: &TestGame, id: i16) -> i32 {
    g.state.mods.get_blade_modifier(id)
}
fn score(g: &TestGame, id: i16) -> i32 {
    g.state.mods.get_score_modifier(id)
}

/// Trigger ab#0 (ライブ開始時) and drive the discard + stage-select choices.
/// `discard` = how many hand cards to discard (0 = skip). `select_members` = how
/// many 虹ヶ咲 members to select from the offered (distinct) pool.
fn trigger_start(
    game: &mut TestGame,
    hand: Vec<i16>,
    discard: usize,
    stage_members: &[i16],
    select_members: usize,
) -> Vec<i16> {
    let jb = game.id(JB);
    game.state.player1.live_card_zone.cards.push(jb);
    for (i, &m) in stage_members.iter().enumerate() {
        game.state.player1.stage.stage[i] = m;
    }
    for c in hand {
        game.state.player1.hand.cards.push(c);
    }

    let card = game.db.get_card(jb).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .expect("ab#0 should be ライブ開始時");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(jb),
        None,
        None,
    );
    game.state.activating_card = Some(jb);
    game.state.process_pending_auto_abilities(&pid);

    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        let choice = game.get_pending_choice().clone();
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { zone, count, allow_skip, .. } => {
                if zone == "hand" {
                    // Any-number (0..=2) selection re-prompts after each pick; the
                    // shrinking hand means each next pick is index 0 again.
                    if allow_skip && discard == 0 {
                        game.select_indices(&[]);
                    } else {
                        let idx = vec![0; discard];
                        game.select_indices_sequential(&idx);
                    }
                } else if zone == "stage" {
                    // Fixed-count selection of distinct 虹ヶ咲 members.
                    let n = select_members.min(count as usize);
                    let idx: Vec<usize> = (0..n).collect();
                    game.select_indices(&idx);
                } else {
                    game.select_indices(&[]);
                }
            }
            _ => game.select_choice_option(0),
        }
    }
    stage_members.to_vec()
}

/// Trigger ab#1 (ライブ成功時) with the given revealed cards; returns JB's id.
fn trigger_success(game: &mut TestGame, revealed: &[&str]) -> i16 {
    let jb = game.id(JB);
    game.state.player1.live_card_zone.cards.push(jb);
    for &r in revealed {
        game.state.revealed_cards.push(game.id(r));
    }
    let card = game.db.get_card(jb).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
        .expect("ab#1 should be ライブ成功時");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::LiveSuccess,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(jb),
        None,
        None,
    );
    game.state.activating_card = Some(jb);
    game.state.process_pending_auto_abilities(&pid);
    jb
}

// ====================================================================
// ab#0 — Q272: cannot select the same member multiple times.
// ====================================================================

/// Discard 1 card → select 1 虹ヶ咲 member → it gains 1 blade.
#[test]
fn q272_discard_one_grants_blade_to_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let m1 = game.id(NIJI_A);
    let hand = vec![game.id(FILLER), game.id(FILLER)];
    let _members = trigger_start(&mut game, hand, 1, &[m1], 1);

    assert_eq!(blade(&game, m1), 1, "1 discard → the selected member gains 1 blade");
    assert_eq!(game.state.player1.hand.cards.len(), 1, "1 card discarded");
}

/// Discard 2 cards → select 2 DISTINCT 虹ヶ咲 members → each gains exactly 1 blade.
#[test]
fn q272_two_distinct_members_each_gain_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let m1 = game.id(NIJI_A);
    let m2 = game.id(NIJI_B);
    let hand = vec![game.id(FILLER), game.id(FILLER)];
    let _members = trigger_start(&mut game, hand, 2, &[m1, m2], 2);

    assert_eq!(blade(&game, m1), 1, "m1 gains exactly 1 blade");
    assert_eq!(blade(&game, m2), 1, "m2 gains exactly 1 blade");
    assert_eq!(game.state.player1.hand.cards.len(), 0, "2 cards discarded");
}

/// Q272 core: only 1 虹ヶ咲 member on stage but 2 cards discarded → the member
/// cannot be selected twice, so it gains exactly 1 blade (not 2).
#[test]
fn q272_same_member_not_selectable_twice() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let m1 = game.id(NIJI_A);
    let hand = vec![game.id(FILLER), game.id(FILLER)];
    // Discard 2 cards, but only 1 虹ヶ咲 member is on stage.
    let _members = trigger_start(&mut game, hand, 2, &[m1], 2);

    assert_eq!(
        blade(&game, m1),
        1,
        "Q272: a member cannot be selected multiple times → exactly 1 blade even when 2 cards are discarded"
    );
    assert_eq!(game.state.player1.hand.cards.len(), 0, "2 cards discarded");
}

/// Non-虹ヶ咲 members on stage are NOT selectable (group filter).
#[test]
fn q272_non_niji_member_not_selectable() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let niji = game.id(NIJI_A);
    let non = game.id(NON_NIJI);
    let hand = vec![game.id(FILLER), game.id(FILLER)];
    // Stage: [虹ヶ咲, non-虹ヶ咲]. Discard 1 → only the 虹ヶ咲 member selectable.
    let _members = trigger_start(&mut game, hand, 1, &[niji, non], 1);

    assert_eq!(blade(&game, niji), 1, "虹ヶ咲 member gains blade");
    assert_eq!(blade(&game, non), 0, "non-虹ヶ咲 member is not selectable → no blade");
}

/// Skipping the discard → no members are selected, no blade is granted.
#[test]
fn q272_skip_discard_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let m1 = game.id(NIJI_A);
    let hand = vec![game.id(FILLER), game.id(FILLER)];
    let _members = trigger_start(&mut game, hand, 0, &[m1], 0);

    assert_eq!(blade(&game, m1), 0, "skipping the discard → no blade granted");
    assert_eq!(game.state.player1.hand.cards.len(), 2, "no cards discarded");
}

// ====================================================================
// ab#1 — 2+ member cards without blade-hearts among revealed → +1 score.
// ====================================================================

/// 2 member cards without blade-hearts among the revealed cards → +1 score.
#[test]
fn q271_ab1_two_no_blade_heart_members_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let jb = trigger_success(&mut game, &[NO_BLADE_MEMBER, NO_BLADE_MEMBER]);

    assert_eq!(
        score(&game, jb),
        1,
        "ab#1: 2 member cards without blade-hearts → +1 score"
    );
}

/// Only 1 member card without a blade-heart → no score.
#[test]
fn q271_ab1_one_no_blade_heart_member_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let jb = trigger_success(&mut game, &[NO_BLADE_MEMBER]);

    assert_eq!(score(&game, jb), 0, "ab#1: only 1 such member → no score");
}

/// 2 member cards WITH blade-hearts → do not count (they have blade-hearts).
#[test]
fn q271_ab1_with_blade_heart_members_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let jb = trigger_success(&mut game, &[WITH_BLADE_MEMBER, WITH_BLADE_MEMBER]);

    assert_eq!(
        score(&game, jb),
        0,
        "ab#1: members with blade-hearts do not qualify → no score"
    );
}

/// 1 no-blade-heart member + 1 with-blade-heart member → only 1 qualifies → no score.
#[test]
fn q271_ab1_mixed_members_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let jb = trigger_success(&mut game, &[NO_BLADE_MEMBER, WITH_BLADE_MEMBER]);

    assert_eq!(score(&game, jb), 0, "ab#1: only 1 no-blade-heart member → no score");
}

/// Live cards (non-member) do not count toward the 2 member-card requirement.
#[test]
fn q271_ab1_member_plus_live_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let jb = trigger_success(&mut game, &[NO_BLADE_MEMBER, LIVE_CARD]);

    assert_eq!(
        score(&game, jb),
        0,
        "ab#1: a live card is not a member → only 1 qualifying member → no score"
    );
}
