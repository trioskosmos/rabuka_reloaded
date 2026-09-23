/// Tests for 澁谷かのん (PL!SP-bp2-001-R＋) — Debut invalidate ability:
///
/// 登場 自分のステージにいる『Liella!』のメンバー1人のすべての
/// ライブ開始時能力を、ライブ終了時まで、無効にしてもよい。
/// これにより無効にした場合、自分の控え室から『Liella!』の
/// カードを1枚手札に加える。
///
/// Q106: Nullifying already-nullified abilities doesn't count.
use crate::helpers::*;
use rabuka_engine::game_state::{AbilityTrigger, Phase};
use rabuka_engine::zones::MemberArea;

#[test]
fn kanon_debut_invalidation_recovers_liella_from_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanon = game.id("PL!SP-bp2-001-R\u{ff0b}");
    let target = game.id("PL!SP-sd1-003-SD");
    let recovery = game.id("PL!SP-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(kanon);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(recovery);
    game.give_energy(13);
    game.state.player1.stage.stage = [target, -1, -1];

    game.play_to_stage(kanon, MemberArea::Center);
    game.drain_choices_strict(&["SelectCard"], &[0]);

    assert!(game
        .state
        .is_ability_invalidated(target, &AbilityTrigger::LiveStart));
    assert!(game.state.player1.hand.cards.contains(&recovery));
    assert!(!game.state.player1.waitroom.cards.contains(&recovery));
    assert_eq!(game.state.player1.stage.stage[0], target);
    assert_eq!(game.state.player1.stage.stage[1], kanon);
    assert_eq!(game.state.player1.hand.cards.len(), 2);
}

#[test]
fn kanon_optional_invalidation_can_be_skipped() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanon = game.id("PL!SP-bp2-001-R\u{ff0b}");
    let target = game.id("PL!SP-sd1-003-SD");
    let recovery = game.id("PL!SP-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(kanon);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(recovery);
    game.give_energy(13);
    game.state.player1.stage.stage = [target, -1, -1];

    game.play_to_stage(kanon, MemberArea::Center);
    assert_eq!(game.pending_choice_type().as_deref(), Some("SelectCard"));
    game.select_indices(&[]);

    assert!(!game.has_pending_choice());
    assert!(!game
        .state
        .is_ability_invalidated(target, &AbilityTrigger::LiveStart));
    assert!(!game.state.player1.hand.cards.contains(&recovery));
    assert!(game.state.player1.waitroom.cards.contains(&recovery));
}

#[test]
fn kanon_q106_cannot_invalidate_same_live_start_twice() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let first_kanon = game.id("PL!SP-bp2-001-R\u{ff0b}");
    let second_kanon = game.new_id("PL!SP-bp2-001-R\u{ff0b}");
    let target = game.id("PL!SP-sd1-003-SD");
    let first_recovery = game.id("PL!SP-sd1-001-SD");
    let second_recovery = game.id("PL!SP-sd1-002-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state
        .player1
        .hand
        .cards
        .extend([first_kanon, second_kanon, filler]);
    game.state
        .player1
        .waitroom
        .cards
        .extend([first_recovery, second_recovery]);
    game.give_energy(26);
    game.state.player1.stage.stage = [target, -1, -1];

    game.play_to_stage(first_kanon, MemberArea::Center);
    game.drain_choices_strict(&["SelectCard"], &[0]);
    assert!(game.state.player1.hand.cards.contains(&first_recovery));

    game.play_to_stage(second_kanon, MemberArea::RightSide);

    assert!(!game.has_pending_choice());
    assert_eq!(
        game.state
            .ability_invalidations
            .iter()
            .filter(|entry| entry.card_id == target)
            .count(),
        1
    );
    assert!(!game.state.player1.hand.cards.contains(&second_recovery));
    assert!(game.state.player1.waitroom.cards.contains(&second_recovery));
}

#[test]
fn kanon_live_start_invalidation_expires_at_live_end() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanon = game.id("PL!SP-bp2-001-R\u{ff0b}");
    let target = game.id("PL!SP-sd1-003-SD");
    let recovery = game.id("PL!SP-sd1-001-SD");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();
    game.state.player1.stage.stage = [target, filler, filler];
    game.state.player2.stage.stage = [-1, -1, -1];
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.extend([kanon, live]);
    game.state.player1.waitroom.cards.push(recovery);
    game.give_energy(13);

    game.play_to_stage(kanon, MemberArea::Center);
    game.drain_choices_strict(&["SelectCard"], &[0]);

    for _ in 0..5 {
        game.pass();
    }
    assert_eq!(game.state.current_phase, Phase::LiveCardSetFirstAttacker);
    game.set_live_card(live);
    game.pass();

    assert!(game
        .state
        .is_ability_invalidated(target, &AbilityTrigger::LiveStart));
    assert_eq!(game.state.mods.get_blade_modifier(target), 0);
    assert!(!game.has_pending_choice());

    for _ in 0..8 {
        if game.state.current_phase == Phase::Active {
            break;
        }
        game.pass();
    }

    assert_eq!(game.state.current_phase, Phase::Active);
    assert!(!game
        .state
        .is_ability_invalidated(target, &AbilityTrigger::LiveStart));
}
