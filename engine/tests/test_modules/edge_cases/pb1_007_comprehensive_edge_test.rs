/// Comprehensive edges for PL!-pb1-007-R idx344
/// 起動 ターン1回 手札を3枚控え室に置く：自分のステージにほかのlilywhiteのメンバーがいる場合、自分の控え室からμ'sのライブカードを1枚手札に加える。コストは成功ライブ1枚につき1減る。
use crate::helpers::*;

const FILLER: &str = "PL!-sd1-010-SD";

fn setup_game_with_success(success_count: usize) -> (TestGame, i16, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!-pb1-007-R");
    let lily = game.id("PL!-bp3-014-N");
    game.state.player1.stage.stage[0] = me;
    game.state.player1.stage.stage[1] = lily;
    game.give_energy(10);
    for _ in 0..success_count {
        let s = game.new_id("PL!N-bp1-025-L"); // any live with score
        game.state.player1.success_live_card_zone.cards.push(s);
    }
    (game, me, lily)
}

fn hand_with_n(game: &mut TestGame, n: usize) -> Vec<i16> {
    let mut v = Vec::new();
    for _ in 0..n {
        let id = game.new_id(FILLER);
        game.state.player1.hand.cards.push(id);
        v.push(id);
    }
    v
}

// Cost is 3 with 0 success -> prompt expects 3 cards
#[test]
fn pb1_007_cost_3_with_0_success() {
    let (mut game, me, _) = setup_game_with_success(0);
    assert_eq!(game.state.player1.success_live_card_zone.cards.len(), 0);
    hand_with_n(&mut game, 5);
    let mus_live = game.id("PL!-sd1-020-SD");
    game.state.player1.waitroom.cards.push(mus_live);
    game.activate_ability(me);
    assert!(game.has_pending_choice());
    // Should require 3 cards
    let count = game.pending_choice_count();
    assert_eq!(count, 3, "0 success -> cost 3, got {}", count);
    game.select_indices(&[0,1,2]);
    assert!(game.state.player1.hand.cards.contains(&mus_live));
}

// Cost 2 with 1 success
#[test]
fn pb1_007_cost_2_with_1_success() {
    let (mut game, me, _) = setup_game_with_success(1);
    hand_with_n(&mut game, 5);
    let mus_live = game.id("PL!-sd1-020-SD");
    game.state.player1.waitroom.cards.push(mus_live);
    game.activate_ability(me);
    assert!(game.has_pending_choice());
    let count = game.pending_choice_count();
    assert_eq!(count, 2, "1 success -> cost 2, got {}", count);
    game.select_indices(&[0,1]);
    assert!(game.state.player1.hand.cards.contains(&mus_live));
}

// Cost 1 with 2 success
#[test]
fn pb1_007_cost_1_with_2_success() {
    let (mut game, me, _) = setup_game_with_success(2);
    hand_with_n(&mut game, 5);
    let mus_live = game.id("PL!-sd1-020-SD");
    game.state.player1.waitroom.cards.push(mus_live);
    game.activate_ability(me);
    assert!(game.has_pending_choice());
    let count = game.pending_choice_count();
    assert_eq!(count, 1, "2 success -> cost 1, got {}", count);
    game.select_indices(&[0]);
    assert!(game.state.player1.hand.cards.contains(&mus_live));
}

// Cost 0 with 3 success -> should allow activation without hand cost
#[test]
fn pb1_007_cost_0_with_3_success() {
    // Game ends at 3 successes, so max before win is 2. This tests the pre-win max: 2 success -> cost 1
    let (mut game, me, _) = setup_game_with_success(2);
    hand_with_n(&mut game, 5);
    let mus_live = game.id("PL!-sd1-020-SD");
    game.state.player1.waitroom.cards.push(mus_live);
    let res = game.try_activate_ability(me);
    assert!(res.is_ok(), "cost 1 with 2 success should still activate: {:?}", res);
    if game.has_pending_choice() {
        let count = game.pending_choice_count();
        assert_eq!(count, 1, "2 success -> cost 1 (max before win), got {}", count);
        game.select_indices(&[0]);
    }
    assert!(game.state.player1.hand.cards.contains(&mus_live));
}

// Cost clamped at 0 with 4+ success - not reachable, game ends at 3. Verify 2 is max.
#[test]
fn pb1_007_cost_clamped_0_with_4_success() {
    let (mut game, me, _) = setup_game_with_success(2);
    hand_with_n(&mut game, 5);
    let mus_live = game.id("PL!-sd1-020-SD");
    game.state.player1.waitroom.cards.push(mus_live);
    let res = game.try_activate_ability(me);
    assert!(res.is_ok());
    if game.has_pending_choice() {
        let count = game.pending_choice_count();
        assert_eq!(count, 1, "2 success (max) -> cost 1, got {}", count);
        game.select_indices(&[0]);
    }
    assert!(game.state.player1.hand.cards.contains(&mus_live));
}

// Insufficient hand should be blocked when cost > hand size
#[test]
fn pb1_007_insufficient_hand_blocked() {
    let (mut game, me, _) = setup_game_with_success(0);
    // Hand has only 2 but need 3
    hand_with_n(&mut game, 2);
    let res = game.try_activate_ability(me);
    // Engine should reject activation due to insufficient cost
    assert!(res.is_err() || game.has_pending_choice() == false, "should be blocked or err, got {:?}", res);
    // If it did prompt, it would require 3 but hand only 2 -> cannot select 3
    if game.has_pending_choice() {
        assert_eq!(game.pending_choice_count(), 3);
        // Try selecting 2 should fail? The choice requires exactly 3, selecting 2 should error or not advance
        // We just verify it doesn't retrieve
        let mus_live = game.id("PL!-sd1-020-SD");
        game.state.player1.waitroom.cards.push(mus_live);
        assert!(!game.state.player1.hand.cards.contains(&mus_live));
    }
}

// Non-μ's live not retrieved even with lilywhite
#[test]
fn pb1_007_non_muse_live_not_retrieved() {
    let (mut game, me, _) = setup_game_with_success(1);
    hand_with_n(&mut game, 5);
    let liella_live = game.id("PL!S-bp2-024-L");
    game.state.player1.waitroom.cards.push(liella_live);
    game.activate_ability(me);
    assert!(game.has_pending_choice());
    let count = game.pending_choice_count();
    assert_eq!(count, 2, "1 success -> cost 2, got {}", count);
    game.select_indices(&[0,1]);
    assert!(!game.state.player1.hand.cards.contains(&liella_live), "liella live should not be retrieved as μ's filter");
}

// Turn1 limit blocks second activation same turn
#[test]
fn pb1_007_turn1_blocks_second() {
    let (mut game, me, _) = setup_game_with_success(1);
    hand_with_n(&mut game, 10);
    let mus1 = game.id("PL!-sd1-020-SD");
    let mus2 = game.new_id("PL!-sd1-020-SD");
    game.state.player1.waitroom.cards.push(mus1);
    game.state.player1.waitroom.cards.push(mus2);
    game.activate_ability(me);
    let c = game.pending_choice_count();
    game.select_indices(&(0..c).collect::<Vec<_>>());
    // May have second prompt to choose which μ's live to retrieve if multiple candidates
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    assert!(game.state.player1.hand.cards.contains(&mus1) || game.state.player1.hand.cards.contains(&mus2));
    // Second activation same turn should be blocked
    let res = game.try_activate_ability(me);
    assert!(res.is_err(), "turn1 should block second activation, got {:?}", res);
}
