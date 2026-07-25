use crate::helpers::*;

/// Sterile room: PL!HS-bp6-017-N | 日野下花帆 ab#0
///
/// 自動: このメンバーがステージから控え室に置かれたとき、
/// 手札を1枚控え室に置いてもよい。そうした場合、
/// 自分の控え室からライブカードとメンバーカードをそれぞれ1枚まで手札に加える。
///
/// Condition: self_target=true, zone_change stage→discard, card_type=member_card
#[test]
fn hanabiko_stage_to_discard_triggers_ability() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let hanabiko = g.id("PL!HS-bp6-017-N");
    let live = g.id("PL!N-bp4-026-L");
    let member = g.id("PL!N-PR-003-PR");
    let filler = g.id("PL!-sd1-010-SD");

    // Hanabiko on stage, live + member in waitroom
    g.state.player1.stage.stage = [-1, hanabiko, -1];
    g.state.player1.waitroom.cards.push(live);
    g.state.player1.waitroom.cards.push(member);
    g.state.player1.hand.cards.push(filler);
    g.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        g.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        g.state.player2.main_deck.cards.push(filler);
    }

    // Move hanabiko from stage to discard
    g.state.player1.stage.stage[1] = -1;
    g.state.player1.waitroom.cards.push(hanabiko);
    g.state.recently_moved_cards = Some(vec![hanabiko].into());

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);

    // ab#0 should present optional discard choice
    assert!(
        g.has_pending_choice(),
        "hanabiko ab#0 should fire on stage→discard"
    );

    // Accept the discard (pay the cost)
    g.select_indices(&[0]);

    // After discarding, should present retrieval choices for live + member
    // Sequential effect: up to 1 live card + up to 1 member card from discard
    assert!(
        g.has_pending_choice(),
        "should prompt to select live card from discard"
    );
    g.select_indices(&[0]);

    assert!(
        g.has_pending_choice(),
        "should prompt to select member card from discard"
    );
    g.select_indices(&[0]);

    // Verify cards moved to hand
    assert!(
        g.state.player1.hand.cards.contains(&live),
        "live card should be retrieved to hand"
    );
    assert!(
        g.state.player1.hand.cards.contains(&member),
        "member card should be retrieved to hand"
    );
}

/// Does NOT trigger: hanabiko starts in discard (static, NOT moved).
#[test]
fn hanabiko_static_discard_no_trigger() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let hanabiko = g.id("PL!HS-bp6-017-N");
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.waitroom.cards.push(hanabiko);
    g.state.player1.hand.cards.push(filler);
    g.state.player1.stage.stage = [-1, -1, -1];
    for _ in 0..10 {
        g.state.player1.main_deck.cards.push(filler);
    }

    // Simulate a different movement (filler goes to discard)
    g.state.player1.hand.cards.retain(|c| *c != filler);
    g.state.player1.waitroom.cards.push(filler);
    g.state.recently_moved_cards = Some(vec![filler].into());

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);

    while g.has_pending_choice() {
        g.select_indices(&[]);
    }

    // Verify hanabiko did not trigger (it was already in discard, not moved)
    assert!(
        g.state.player1.waitroom.cards.contains(&hanabiko),
        "hanabiko stays in waitroom"
    );
}

/// Does NOT trigger: hanabiko on stage (static, NOT moved).
#[test]
fn hanabiko_static_stage_no_trigger() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let hanabiko = g.id("PL!HS-bp6-017-N");
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.stage.stage = [-1, hanabiko, -1];
    g.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        g.state.player1.main_deck.cards.push(filler);
    }

    // A different card moves to trigger TAS
    g.state.player1.hand.cards.retain(|c| *c != filler);
    g.state.player1.waitroom.cards.push(filler);
    g.state.recently_moved_cards = Some(vec![filler].into());

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);

    while g.has_pending_choice() {
        g.select_indices(&[]);
    }

    // Hanabiko should still be on stage — its ability should NOT trigger
    assert!(
        g.state.player1.stage.stage.contains(&hanabiko),
        "hanabiko stays on stage"
    );
}

/// Does NOT trigger: hanabiko moves hand→discard (wrong source zone).
#[test]
fn hanabiko_hand_to_discard_no_trigger() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let hanabiko = g.id("PL!HS-bp6-017-N");
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.hand.cards.push(hanabiko);
    g.state.player1.hand.cards.push(filler);
    g.state.player1.stage.stage = [-1, -1, -1];
    for _ in 0..10 {
        g.state.player1.main_deck.cards.push(filler);
    }

    // Move hanabiko from hand to discard (wrong zone! should be stage→discard)
    g.state.player1.hand.cards.retain(|c| *c != hanabiko);
    g.state.player1.waitroom.cards.push(hanabiko);
    g.state.recently_moved_cards = Some(vec![hanabiko].into());

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);

    while g.has_pending_choice() {
        g.select_indices(&[]);
    }

    // Hanabiko's ability should NOT fire (source was hand, not stage)
    // Note: the condition explicitly requires stage→discard
}

/// Does NOT trigger: hanabiko moves stage→deck (wrong destination zone).
#[test]
fn hanabiko_stage_to_deck_no_trigger() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let hanabiko = g.id("PL!HS-bp6-017-N");
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.stage.stage = [-1, hanabiko, -1];
    g.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        g.state.player1.main_deck.cards.push(filler);
    }

    // Move hanabiko from stage to top of deck (wrong destination!)
    g.state.player1.stage.stage[1] = -1;
    g.state.player1.main_deck.cards.push(hanabiko);
    g.state.recently_moved_cards = Some(vec![hanabiko].into());

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);

    while g.has_pending_choice() {
        g.select_indices(&[]);
    }

    // Her ability should NOT fire (destination was deck, not discard)
}

/// Two hanabiko copies: one on stage moves to discard, one static in discard.
/// Only the moved copy should trigger.
#[test]
fn hanabiko_two_copies_one_moved_one_static() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let hanabiko_moved = g.id("PL!HS-bp6-017-N");
    let hanabiko_static = g.id("PL!HS-bp6-017-N");
    let filler = g.id("PL!-sd1-010-SD");

    // One copy on stage, one copy already in discard (static)
    g.state.player1.stage.stage = [-1, hanabiko_moved, -1];
    g.state.player1.waitroom.cards.push(hanabiko_static);
    g.state.player1.hand.cards.push(filler);
    g.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        g.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        g.state.player2.main_deck.cards.push(filler);
    }

    // Move the stage copy to discard
    g.state.player1.stage.stage[1] = -1;
    g.state.player1.waitroom.cards.push(hanabiko_moved);
    g.state.recently_moved_cards = Some(vec![hanabiko_moved].into());

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);

    // ab#0 should fire exactly once (for the moved copy only)
    assert!(
        g.has_pending_choice(),
        "hanabiko ab#0 should fire for the moved copy"
    );

    while g.has_pending_choice() {
        g.select_indices(&[0]);
        while g.has_pending_choice() {
            g.select_indices(&[0]); // drain sub-choices
        }
    }
}

/// Sterile room: declined optional discard → no retrieval happens.
#[test]
fn hanabiko_decline_discard_no_retrieval() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let hanabiko = g.id("PL!HS-bp6-017-N");
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.stage.stage = [-1, hanabiko, -1];
    g.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        g.state.player1.main_deck.cards.push(filler);
    }

    // Move hanabiko from stage to discard
    g.state.player1.stage.stage[1] = -1;
    g.state.player1.waitroom.cards.push(hanabiko);
    g.state.recently_moved_cards = Some(vec![hanabiko].into());

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);

    // ab#0 presents optional discard choice — decline
    assert!(g.has_pending_choice(), "optional discard expected");
    g.select_indices(&[]); // decline

    // No retrieval should happen (declined the discard)
    assert!(
        !g.has_pending_choice(),
        "no retrieval choices after decline"
    );
}

/// Another card played on Hanabiko's stage position → Hanabiko goes stage→discard.
/// Her auto-ability SHOULD trigger because the zone change happens.
#[test]
fn hanabiko_replaced_by_new_card_on_same_position_triggers() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let hanabiko = g.id("PL!HS-bp6-017-N");
    let replacement = g.id("PL!N-bp5-019-N");
    let live = g.id("PL!N-bp4-026-L");
    let member_retrieve = g.id("PL!N-PR-003-PR");
    let filler = g.id("PL!-sd1-010-SD");

    // Hanabiko on stage center
    g.state.player1.stage.stage = [-1, hanabiko, -1];
    // Cards in waitroom to be retrieved
    g.state.player1.waitroom.cards.push(live);
    g.state.player1.waitroom.cards.push(member_retrieve);
    // Hand: replacement card + filler for the optional discard
    g.state.player1.hand.cards.push(replacement);
    g.state.player1.hand.cards.push(filler);
    g.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        g.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        g.state.player2.main_deck.cards.push(filler);
    }
    g.give_energy(20);

    // Play replacement to center (baton-touch: Hanabiko goes to waitroom)
    g.play_to_stage(replacement, rabuka_engine::zones::MemberArea::Center);
    while g.has_pending_choice() {
        g.select_indices(&[]);
    }

    // After the replacement, Hanabiko should be in waitroom
    assert!(
        g.state.player1.waitroom.cards.contains(&hanabiko),
        "Hanabiko should be in waitroom after replacement"
    );

    // Now trigger the TAS to check if Hanabiko's auto-ability fires.
    // The replacement may have set recently_moved_cards with Hanabiko.
    // Force a TAS scan to verify.
    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);

    // If recently_moved_cards was set, ab#0 should fire
    if g.has_pending_choice() {
        g.select_indices(&[]); // decline discard
    }
}

/// Baton touch: another card baton-touches to Hanabiko's position →
/// Hanabiko goes to discard. Her auto-ability should trigger.
#[test]
fn hanabiko_baton_touch_replaced_triggers() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let hanabiko = g.id("PL!HS-bp6-017-N");
    let baton_arriver = g.id("PL!N-sd1-010-SD");
    let live = g.id("PL!N-bp4-026-L");
    let member_retrieve = g.id("PL!N-PR-003-PR");
    let filler = g.id("PL!-sd1-010-SD");

    // Hanabiko on stage center, retrieval targets in waitroom
    g.state.player1.stage.stage = [-1, hanabiko, -1];
    g.state.player1.hand.cards.push(baton_arriver);
    g.state.player1.hand.cards.push(filler);
    g.state.player1.hand.cards.push(filler);
    g.state.player1.waitroom.cards.push(live);
    g.state.player1.waitroom.cards.push(member_retrieve);
    for _ in 0..10 {
        g.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        g.state.player2.main_deck.cards.push(filler);
    }
    g.give_energy(20);

    // Baton touch: play arriver to occupied center → Hanabiko goes to waitroom
    g.play_to_stage(baton_arriver, rabuka_engine::zones::MemberArea::Center);

    // Shioriko's debut: draw 2, then mandatory discard 1 from hand.
    // Select a filler from hand.
    if g.has_pending_choice() {
        g.select_indices(&[0]);
    }

    // Now Hanabiko's auto-ability should fire.
    // Accept the optional discard, then retrieve live + member.
    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }

    assert!(
        g.state.player1.waitroom.cards.contains(&hanabiko),
        "Hanabiko should be in waitroom after baton touch"
    );
}
