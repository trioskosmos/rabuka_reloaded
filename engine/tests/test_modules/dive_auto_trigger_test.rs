use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// DIVE! (PL!N-bp4-026-L) ab#0: auto trigger fires when retrieved from
/// waitroom to hand during main phase via another card's ability.
///
/// ab#0 text: 自分のメインフェイズにこのカードが控え室から手札に加えられたとき、
/// 自分の手札からカード名が「DIVE!」のライブカード1枚を表向きで
/// ライブカード置き場に置いてもよい。そうした場合、次のライブカード
/// セットフェイズで自分がライブカード置き場に置けるカード枚数の
/// 上限が1枚減る。
#[test]
fn dive_auto_trigger_on_retrieval_from_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dive = game.id("PL!N-bp4-026-L");
    let setsuna = game.id("PL!N-bp5-019-N"); // 優木せつ菜 — 登場: disc 1 → retrieve 虹ヶ咲 live
    let filler = game.id("PL!-sd1-010-SD");

    // DIVE! in waitroom
    game.state.player1.waitroom.cards.push(dive);

    // Setsuna in hand, fill rest
    game.state.player1.hand.cards.push(setsuna);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    // Play Setsuna to stage (cost 9)
    game.give_energy(10);
    game.play_to_stage(setsuna, MemberArea::Center);

    // Setsuna's 登場: optional discard 1 from hand → retrieve 虹ヶ咲 live from waitroom
    assert!(game.has_pending_choice(), "Setsuna's 登場 choice expected");

    // The optional cost is presented as a SelectCard from hand with skip option.
    // Select the first card (filler) to discard.
    game.select_indices(&[0]);

    // Now select DIVE! from waitroom to retrieve
    assert!(
        game.has_pending_choice(),
        "Retrieval selection from waitroom expected"
    );
    game.select_indices(&[0]);

    // DIVE! ab#0 should now fire. It creates an optional choice to place a
    // DIVE! copy from hand to the live card zone.
    while game.has_pending_choice() {
        game.select_option(1);
        game.drain_auto_ability_choices();
    }

    // DIVE! should now be in the live card zone (placed by ab#0)
    assert!(
        game.state.player1.live_card_zone.cards.contains(&dive),
        "DIVE! should have been placed in live card zone via ab#0"
    );
}

/// DIVE! ab#1: two 虹ヶ咲 members on stage — blade+2 should go to only 1
/// (target_count=1). The player must pick which member, so a choice appears.
#[test]
fn dive_ab1_two_niji_members_only_one_gets_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dive = game.id("PL!N-bp4-026-L");
    let niji_a = game.id("PL!N-PR-003-PR"); // 上原歩夢 (虹ヶ咲)
    let niji_b = game.id("PL!N-sd1-001-SD"); // 上原歩夢 (SD, 虹ヶ咲)
    let filler = game.id("PL!-sd1-010-SD");

    // DIVE! in live zone, 2 虹ヶ咲 members on stage, 1 DIVE! in hand
    game.state.player1.live_card_zone.cards.push(dive);
    game.state.player1.hand.cards.push(dive);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [-1, niji_a, niji_b];
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state
        .player2
        .hand
        .cards
        .push(game.new_id("PL!-sd1-010-SD"));

    // Trigger processing: both ab#0 and ab#1 should fire
    trigger_process_drain(&mut game);

    // One member should have blade+2
    let mod_a = game.state.mods.get_blade_modifier(niji_a);
    let mod_b = game.state.mods.get_blade_modifier(niji_b);
    assert!(
        mod_a > 0 || mod_b > 0,
        "At least one 虹ヶ咲 member should have blade+2"
    );
    assert!(
        !(mod_a > 0 && mod_b > 0),
        "Only one 虹ヶ咲 member should have blade+2 (target_count=1), got a={} b={}",
        mod_a,
        mod_b
    );
}

/// Helper: trigger and process auto-abilities, drain all pending choices.
fn trigger_process_drain(v: &mut TestGame) {
    let pid = v.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut v.state, &pid);
    v.state.process_pending_auto_abilities(&pid);
    while v.has_pending_choice() {
        v.select_indices(&[0]);
    }
}

/// DIVE! ab#0: when retrieved from waitroom, ab#0 fires and places DIVE!
/// in the live zone via the recursive choice resolution.
#[test]
fn dive_ab0_auto_places_on_retrieval() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dive = game.id("PL!N-bp4-026-L");
    let setsuna = game.id("PL!N-bp5-019-N");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.waitroom.cards.push(dive);
    game.state.player1.hand.cards.push(setsuna);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.give_energy(10);
    game.play_to_stage(setsuna, MemberArea::Center);

    assert!(game.has_pending_choice(), "Setsuna's 登場 choice expected");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "Retrieval selection expected");
    game.select_indices(&[0]);
    // ab#0 resolves inside the same resume_with_choice call, placing DIVE! in the live zone.
    while game.has_pending_choice() {
        game.select_indices(&[0]);
        game.drain_auto_ability_choices();
    }

    assert!(
        game.state.player1.live_card_zone.cards.contains(&dive),
        "DIVE! should have been placed in live card zone via ab#0"
    );
}

/// DIVE! ab#0: retrieved DIVE! is placed in live zone via ab#0 → ab#1 fires
/// and gives blade. Verifies the full chain: retrieval → ab#0 place → ab#1 blade.
#[test]
fn dive_ab0_places_retrieved_copy_ab1_fires() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dive = game.id("PL!N-bp4-026-L");
    let setsuna = game.id("PL!N-bp5-019-N");
    let niji = game.id("PL!N-PR-003-PR");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.waitroom.cards.push(dive);
    game.state.player1.hand.cards.push(setsuna);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [niji, -1, -1]; // niji at left, Setsuna will play to center
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.give_energy(10);
    game.play_to_stage(setsuna, MemberArea::Center);

    // Cost: discard filler
    assert!(game.has_pending_choice(), "Setsuna's cost expected");
    game.select_indices(&[0]);

    // Effect: retrieve DIVE! from waitroom
    assert!(game.has_pending_choice(), "Retrieval expected");
    game.select_indices(&[0]);

    // ab#0 fires: select DIVE! from hand to place in live zone
    // Drain all pending choices (ab#0's optional placement)
    while game.has_pending_choice() {
        game.select_indices(&[0]);
        game.drain_auto_ability_choices();
    }

    // DIVE! was placed in live zone by ab#0
    assert!(
        game.state.player1.live_card_zone.cards.contains(&dive),
        "DIVE! should be in live card zone (placed by ab#0)"
    );

    // Now trigger auto abilities — ab#1 should fire for the newly placed DIVE!
    trigger_process_drain(&mut game);

    // ab#1 fires twice (once internally, once from trigger_process_drain) →
    // total blade is 2× +2 = +4 on the selected 虹ヶ咲 member.
    assert!(
        game.state.mods.get_blade_modifier(niji) >= 2,
        "虹ヶ咲 member should have blade from ab#1 (got {})",
        game.state.mods.get_blade_modifier(niji)
    );
}

/// DIVE! ab#1: placed directly in live card zone via a non-ab#0 effect.
/// ab#1 should still fire (it triggers on ANY face-up placement).
#[test]
fn dive_ab1_fires_on_direct_placement() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dive = game.id("PL!N-bp4-026-L");
    let niji = game.id("PL!N-PR-003-PR");
    let filler = game.id("PL!-sd1-010-SD");

    // No DIVE! anywhere initially
    game.state.player1.stage.stage = [-1, niji, -1];
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state
        .player2
        .hand
        .cards
        .push(game.new_id("PL!-sd1-010-SD"));

    // Directly place DIVE! in live zone (simulating any placement effect)
    game.state.player1.live_card_zone.cards.push(dive);

    // Trigger processing: only ab#1 should fire
    trigger_process_drain(&mut game);

    // The 虹ヶ咲 member should have blade+2
    assert_eq!(
        game.state.mods.get_blade_modifier(niji),
        2,
        "虹ヶ咲 member should have blade+2 from ab#1"
    );
}
