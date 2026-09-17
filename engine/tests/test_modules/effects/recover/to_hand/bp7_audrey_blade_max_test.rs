/// BP07 CLEAN-G11: PL!N-bp7-027-L オードリー ab#0.
///
/// ライブ成功時：自分のステージにいる『虹ヶ咲』のメンバー1人を選ぶ。そのメンバーが、
/// 自分と相手のステージにいるほかのすべてのメンバーより多くのブレードを持つ場合、
/// このカードのスコアを＋１する。
///
/// (LiveSuccess: choose 1 『虹ヶ咲』 member on your stage. If that member has MORE
/// blade than ALL other members on BOTH your stage and the opponent's stage,
/// this card's score +1.)
///
/// The parser defect (documented in _bp07_ability_gaps_hand_analysis.md CLEAN-G11):
/// the condition was `location_condition{stage, exclude_self, scope:both, all}` with
/// NO blade comparison — the "has more blade than all others" predicate was dropped.
/// These tests pin the correct behavior through the real live-success ability queue
/// (select the member → evaluate the max-blade condition → apply the score modifier).
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const AUDREY: &str = "PL!N-bp7-027-L";
/// 虹ヶ咲 member target (blade 4, 虹ヶ咲 group).
const NIJI_HIGH: &str = "PL!N-bp1-001-R";
/// Low-blade non-虹ヶ咲 members.
const LOW1: &str = "PL!-sd1-010-SD"; // 高坂穂乃果, blade 1
const LOW2: &str = "PL!N-sd1-006-SD"; // 近江彼方, blade 1
/// Higher-blade member (blade 5).
const HIGH5: &str = "PL!N-sd2-001-SD2"; // 上原歩夢, blade 5

/// Fire オードリー's ライブ成功時 ability, answering the member-selection choice
/// with the FIRST candidate. Drives the real ability queue / resolver / select.
fn trigger_live_success(game: &mut TestGame, audrey: i16) {
    fire_trigger(game, audrey, AbilityTrigger::LiveSuccess, "ライブ成功時");

    // オードリー ab#0 begins with a SelectCard (choose 1 虹ヶ咲 member).
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => {
                game.select_indices(&[0]);
            }
            Some("SelectAutoAbility") => {
                game.select_indices(&[]);
            }
            _ => break,
        }
    }
}

/// Target (blade 4) is higher than every other stage member on both sides → +1.
#[test]
fn audrey_high_target_scores_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let audrey = game.id(AUDREY);
    let target = game.id(NIJI_HIGH); // blade 4, 虹ヶ咲
    let low1 = game.id(LOW1); // blade 1
    let low2 = game.id(LOW2); // blade 1

    game.state.player1.stage.stage = [target, -1, -1];
    game.state.player2.stage.stage = [low1, low2, -1];
    // オードリー must be in the live-card zone for the score modifier to target it.
    game.state.player1.live_card_zone.cards.push(audrey);

    trigger_live_success(&mut game, audrey);

    assert_eq!(
        game.state.mods.get_score_modifier(audrey),
        1,
        "blade-4 target beats blade-1/blade-1 opponents → +1 score"
    );
}

/// An opponent member (blade 5) out-blades the target (blade 4) → no +1.
#[test]
fn audrey_lower_than_opponent_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let audrey = game.id(AUDREY);
    let target = game.id(NIJI_HIGH); // blade 4
    let high = game.id(HIGH5); // blade 5

    game.state.player1.stage.stage = [target, -1, -1];
    game.state.player2.stage.stage = [high, -1, -1];
    game.state.player1.live_card_zone.cards.push(audrey);

    trigger_live_success(&mut game, audrey);

    assert_eq!(
        game.state.mods.get_score_modifier(audrey),
        0,
        "blade-4 target is NOT strictly higher than blade-5 opponent → no score"
    );
}

/// An opponent member with EQUAL blade (4 == 4) → "より多くの" is strict, no +1.
#[test]
fn audrey_tied_with_opponent_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let audrey = game.id(AUDREY);
    let target = game.id(NIJI_HIGH); // blade 4
    let tied = game.id(NIJI_HIGH); // blade 4 (opponent copy)

    game.state.player1.stage.stage = [target, -1, -1];
    game.state.player2.stage.stage = [tied, -1, -1];
    game.state.player1.live_card_zone.cards.push(audrey);

    trigger_live_success(&mut game, audrey);

    assert_eq!(
        game.state.mods.get_score_modifier(audrey),
        0,
        "a tie does not count as 'more blade than all others'"
    );
}

/// A lower-blade member on the target's OWN stage also counts — still +1.
#[test]
fn audrey_own_stage_other_member_lower_scores_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let audrey = game.id(AUDREY);
    let target = game.id(NIJI_HIGH); // blade 4
    let own_low = game.id(LOW1); // blade 1, non-虹ヶ咲 (not selectable)
    let low = game.id(LOW2); // blade 1

    game.state.player1.stage.stage = [target, own_low, -1];
    game.state.player2.stage.stage = [low, -1, -1];
    game.state.player1.live_card_zone.cards.push(audrey);

    trigger_live_success(&mut game, audrey);

    assert_eq!(
        game.state.mods.get_score_modifier(audrey),
        1,
        "other members on both stages (incl. own side) must be compared"
    );
}

/// No other stage member at all → "ほかのすべてのメンバー" is vacuously true → +1.
#[test]
fn audrey_alone_on_stage_scores_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let audrey = game.id(AUDREY);
    let target = game.id(NIJI_HIGH); // blade 4

    game.state.player1.stage.stage = [target, -1, -1];
    game.state.player2.stage.stage = [-1, -1, -1];
    game.state.player1.live_card_zone.cards.push(audrey);

    trigger_live_success(&mut game, audrey);

    assert_eq!(
        game.state.mods.get_score_modifier(audrey),
        1,
        "target is the only member → vacuously has more blade than all others"
    );
}
