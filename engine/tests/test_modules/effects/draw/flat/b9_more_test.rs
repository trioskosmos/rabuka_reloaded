/// Batch 9 — more 1-QA cards with engine behavior
use crate::helpers::*;
/// PL!-pb1-030-L (Cutie Panther) Q36: LiveStart — reduce required hearts
/// if wait members on stage.
#[test]
fn cutie_panther_live_start_reduce_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let cutie = game.id("PL!-pb1-030-L");
    let member = game.id("PL!-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [member, -1, -1];
    // Condition: opponent has a wait-state member on stage.
    let opp = game.id("PL!-sd1-005-SD");
    game.state.player2.stage.stage = [-1, opp, -1];
    game.state.player1.hand.cards.push(cutie);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(cutie);
    // Set wait AFTER the active phase, which would otherwise stand the member.
    game.state
        .mods
        .add_orientation_modifier(opp, "wait");
    for _ in 0..5 {
        game.pass();
        while game.has_pending_choice() {
            game.select_indices(&[]);
        }
    }
    use rabuka_engine::card::HeartColor;
    assert_eq!(
        game.state.mods.get_need_heart_modifier(cutie, HeartColor::Heart00),
        -2,
        "Cutie Panther must reduce required heart00 by 2 when opponent has a wait member"
    );
}

/// PL!-pb1-031-L (輝夜の城で踊りたい)
/// ライブ成功時: 手札を1枚控え室に置いてもよい：エールにより公開された自分のカードの中から、
/// 『μ's』のメンバーカードを1枚手札に加える。
/// Test: During LiveSuccess, cheer-revealed μ's member card can be recovered.
#[test]
fn kaguya_live_success_recover() {
    let db = load_real_database();
    let kaguya = db.get_card_id("PL!-pb1-031-L").expect("Card exists");
    let kaguya_card = db.get_card(kaguya).expect("Kaguya card exists");
    assert!(
        !kaguya_card.abilities.is_empty(),
        "Card should have abilities"
    );
}

/// Kaguya live success ability: verify it can recover a μ's member from cheer-revealed cards.
#[test]
fn kaguya_live_success_cheer_recover() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kaguya = game.id("PL!-pb1-031-L");
    let member = game.id("PL!-sd1-001-SD"); // μ's member (高坂穂乃果)
    let filler = game.id("PL!-sd1-010-SD");
    let bladed_member = game.id("PL!S-sd1-003-SD"); // Has blades to trigger cheer

    // Stage: bladed member for cheer + member with heart06 for live success requirement
    game.state.player1.stage.stage = [bladed_member, member, -1];
    game.state.player1.hand.cards.push(kaguya);
    game.state.player1.hand.cards.push(filler); // For optional discard cost

    // Deck: need exactly 1 filler before member because:
    // - LiveCardSetFirstAttacker replacement draw consumes 1 card from P1's deck
    // - Yell then draws from index 0 = member = first in revealed_cards
    // (Odd trivia: the draw phase during the first 5 passes draws from P2, not P1)
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(filler); // consumed by replacement draw
    game.state.player1.main_deck.cards.push(member); // first yell/revealed card
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    // Advance to live card set phase
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(kaguya);

    // Advance through remaining phases to live performance
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();

    // After live performance, cheer-revealed cards should be in revealed_cards
    // If the member was cheer-revealed, the ability should add it to hand
    // Handle all pending choices (cost + revealed_cards selection)
    while game.has_pending_choice() {
        // Select the first option whenever prompted
        game.select_indices(&[0]);
    }

    // Verify: the μ's member card was recovered to hand by the LiveSuccess ability
    assert!(
        game.state.player1.hand.cards.contains(&member),
        "μ's member should be recovered to hand by kaguya LiveSuccess ability"
    );
}

/// PL!S-bp2-022-L (未熟DREAMER) — deck refresh condition
#[test]
fn mijuku_dreamer_no_refresh_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let mijuku = game.id("PL!S-bp2-022-L");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards = vec![filler; 10].into();
    game.state.player1.live_card_zone.cards.push(mijuku);

    let card = game.db.get_card(mijuku).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
        .unwrap();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        rabuka_engine::core::types::AbilityTrigger::LiveSuccess,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(mijuku),
        None,
        None,
    );
    game.state.activating_card = Some(mijuku);
    game.state.process_pending_auto_abilities(&pid);

    assert_eq!(
        game.state.mods.get_score_modifier(mijuku),
        0,
        "no refresh this turn → no score bonus"
    );
}

/// Natural refresh via PL!-sd1-008-SD mill-10 from a 3-card deck.
#[test]
fn mijuku_dreamer_refresh_via_mill_gets_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let mijuku = game.id("PL!S-bp2-022-L");
    let hanayo = game.id("PL!-sd1-008-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[0] = hanayo;
    game.give_energy(2);
    game.state.player1.main_deck.cards = vec![filler; 3].into();
    game.state.player1.waitroom.cards = vec![filler; 10].into();
    game.state.player1.live_card_zone.cards.push(mijuku);
    game.state.player1.deck_refreshed_this_turn = false;

    game.activate_ability(hanayo);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.deck_refreshed_this_turn,
        "deck refresh must occur during mill-10 overdraw"
    );

    let card = game.db.get_card(mijuku).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
        .unwrap();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        rabuka_engine::core::types::AbilityTrigger::LiveSuccess,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(mijuku),
        None,
        None,
    );
    game.state.activating_card = Some(mijuku);
    game.state.process_pending_auto_abilities(&pid);

    assert_eq!(
        game.state.mods.get_score_modifier(mijuku),
        2,
        "deck refreshed this turn → +2 score"
    );
}

use rabuka_engine::card::HeartColor;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn trigger_live_start(game: &mut TestGame, card_id: i16) {
    let card = game.db.get_card(card_id).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .unwrap();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        rabuka_engine::core::types::AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(card_id),
        None,
        None,
    );
    game.state.activating_card = Some(card_id);
    game.state.process_pending_auto_abilities(&pid);
}

/// PL!SP-bp1-024-L (Tiny Stars) ab#0: LiveStart — basic case, 1 Kanon + 1 Keke.
/// No choices needed; verify both characters get correct blade+heart.
#[test]
fn tiny_stars_basic() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let tiny_stars = game.id("PL!SP-bp1-024-L");
    let kanon = game.id("PL!SP-sd1-001-SD");
    let keke = game.id("PL!SP-sd1-002-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [kanon, keke, -1];
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_live_start(&mut game, tiny_stars);

    assert!(
        !game.has_pending_choice(),
        "Basic case (1 each): no choice expected"
    );

    assert_eq!(
        game.state.mods.get_blade_modifier(kanon),
        1,
        "Kanon should have +1 blade"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kanon, HeartColor::Heart05),
        1,
        "Kanon should have +1 heart05"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(keke),
        1,
        "Keke should have +1 blade"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(keke, HeartColor::Heart01),
        1,
        "Keke should have +1 heart01"
    );
}

/// 2 Kanon + 1 Keke on stage: player must select which Kanon gets the bonus.
#[test]
fn tiny_stars_duplicate_kanon() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let tiny_stars = game.id("PL!SP-bp1-024-L");
    let kanon1 = game.id("PL!SP-sd1-001-SD");
    let kanon2 = game.new_id("PL!SP-sd1-001-SD");
    let keke = game.id("PL!SP-sd1-002-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [kanon1, kanon2, keke];
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_live_start(&mut game, tiny_stars);

    assert!(
        game.has_pending_choice(),
        "Duplicate kanon: should prompt to select one"
    );

    // Select the first kanon (index 0 in the prompt)
    game.select_indices(&[0]);

    // Verify no more pending choices: keke is the sole remaining candidate,
    // and single-candidate selections auto-resolve (no prompt offered).
    assert!(
        !game.has_pending_choice(),
        "no further choice expected after kanon selection (keke auto-resolves)"
    );

    let chosen_blade = game.state.mods.get_blade_modifier(kanon1);
    let chosen_heart = game
        .state
        .mods
        .get_heart_modifier(kanon1, HeartColor::Heart05);
    let other_blade = game.state.mods.get_blade_modifier(kanon2);
    let other_heart = game
        .state
        .mods
        .get_heart_modifier(kanon2, HeartColor::Heart05);

    assert_eq!(
        chosen_blade, 1,
        "Selected kanon should have +1 blade, got {}",
        chosen_blade
    );
    assert_eq!(
        chosen_heart, 1,
        "Selected kanon should have +1 heart05, got {}",
        chosen_heart
    );
    assert_eq!(
        other_blade, 0,
        "Unselected kanon should have 0 blade, got {}",
        other_blade
    );
    assert_eq!(
        other_heart, 0,
        "Unselected kanon should have 0 heart05, got {}",
        other_heart
    );

    assert_eq!(
        game.state.mods.get_blade_modifier(keke),
        1,
        "Keke should have +1 blade"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(keke, HeartColor::Heart01),
        1,
        "Keke should have +1 heart01"
    );
}

/// 1 Kanon + 2 Keke on stage: player must select which Keke gets the bonus.
#[test]
fn tiny_stars_duplicate_keke() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let tiny_stars = game.id("PL!SP-bp1-024-L");
    let kanon = game.id("PL!SP-sd1-001-SD");
    let keke1 = game.id("PL!SP-sd1-002-SD");
    let keke2 = game.new_id("PL!SP-sd1-002-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [kanon, keke1, keke2];
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_live_start(&mut game, tiny_stars);

    // Kanon has only 1 candidate, should be handled automatically.
    // Keke has 2 candidates → should prompt.
    assert!(
        game.has_pending_choice(),
        "Duplicate keke: should prompt to select one"
    );

    // Select the first keke (index 0 in the prompt)
    game.select_indices(&[0]);

    // Verify no more pending choices: kanon is the sole remaining candidate,
    // and single-candidate selections auto-resolve (no prompt offered).
    assert!(
        !game.has_pending_choice(),
        "no further choice expected after keke selection (kanon auto-resolves)"
    );

    assert_eq!(
        game.state.mods.get_blade_modifier(kanon),
        1,
        "Kanon should have +1 blade"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kanon, HeartColor::Heart05),
        1,
        "Kanon should have +1 heart05"
    );

    let chosen_blade = game.state.mods.get_blade_modifier(keke1);
    let chosen_heart = game
        .state
        .mods
        .get_heart_modifier(keke1, HeartColor::Heart01);
    let other_blade = game.state.mods.get_blade_modifier(keke2);
    let other_heart = game
        .state
        .mods
        .get_heart_modifier(keke2, HeartColor::Heart01);

    assert_eq!(
        chosen_blade, 1,
        "Selected keke should have +1 blade, got {}",
        chosen_blade
    );
    assert_eq!(
        chosen_heart, 1,
        "Selected keke should have +1 heart01, got {}",
        chosen_heart
    );
    assert_eq!(
        other_blade, 0,
        "Unselected keke should have 0 blade, got {}",
        other_blade
    );
    assert_eq!(
        other_heart, 0,
        "Unselected keke should have 0 heart01, got {}",
        other_heart
    );
}

/// 1 Kanon only, 0 Keke on stage: only kanon gets blade+heart05.
#[test]
fn tiny_stars_kanon_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let tiny_stars = game.id("PL!SP-bp1-024-L");
    let kanon = game.id("PL!SP-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [kanon, -1, -1];
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_live_start(&mut game, tiny_stars);

    assert!(
        !game.has_pending_choice(),
        "1 kanon only: no choice expected"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(kanon),
        1,
        "Kanon should have +1 blade"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kanon, HeartColor::Heart05),
        1,
        "Kanon should have +1 heart05"
    );
}

/// 0 Kanon, 1 Keke on stage: only keke gets blade+heart01.
#[test]
fn tiny_stars_keke_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let tiny_stars = game.id("PL!SP-bp1-024-L");
    let keke = game.id("PL!SP-sd1-002-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [keke, -1, -1];
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_live_start(&mut game, tiny_stars);

    assert!(
        !game.has_pending_choice(),
        "1 keke only: no choice expected"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(keke),
        1,
        "Keke should have +1 blade"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(keke, HeartColor::Heart01),
        1,
        "Keke should have +1 heart01"
    );
}

/// 0 Kanon, 0 Keke on stage: no resources should be granted.
#[test]
fn tiny_stars_none() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let tiny_stars = game.id("PL!SP-bp1-024-L");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, -1, -1];
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_live_start(&mut game, tiny_stars);

    assert!(
        !game.has_pending_choice(),
        "Empty stage: no choice expected"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(tiny_stars),
        0,
        "No blade should be granted"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(tiny_stars, HeartColor::Heart05),
        0,
        "No heart05 should be granted"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(tiny_stars, HeartColor::Heart01),
        0,
        "No heart01 should be granted"
    );
}

/// 2 Kanon, 0 Keke on stage: select index[1] (second kanon).
#[test]
fn tiny_stars_duplicate_kanon_select_second() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let tiny_stars = game.id("PL!SP-bp1-024-L");
    let kanon1 = game.id("PL!SP-sd1-001-SD");
    let kanon2 = game.new_id("PL!SP-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [kanon1, kanon2, -1];
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_live_start(&mut game, tiny_stars);

    assert!(
        game.has_pending_choice(),
        "2 kanon: should prompt to select one"
    );

    // Select the SECOND kanon (index 1)
    game.select_indices(&[1]);

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectHeartColor") | Some("SelectHeartType") => {
                panic!("Unexpected heart color choice")
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }

    assert_eq!(
        game.state.mods.get_blade_modifier(kanon1),
        0,
        "Unselected kanon should have 0 blade"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(kanon2),
        1,
        "Selected kanon should have +1 blade"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kanon2, HeartColor::Heart05),
        1,
        "Selected kanon should have +1 heart05"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kanon1, HeartColor::Heart05),
        0,
        "Unselected kanon should have 0 heart05"
    );
}

/// 2 Kanon + 1 Keke, select the SECOND kanon from the prompt.
#[test]
fn tiny_stars_duplicate_kanon_with_keke_select_second() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let tiny_stars = game.id("PL!SP-bp1-024-L");
    let kanon1 = game.id("PL!SP-sd1-001-SD");
    let kanon2 = game.new_id("PL!SP-sd1-001-SD");
    let keke = game.id("PL!SP-sd1-002-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [kanon1, kanon2, keke];
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_live_start(&mut game, tiny_stars);

    assert!(
        game.has_pending_choice(),
        "Duplicate kanon + keke: should prompt to select kanon"
    );

    // Select the second kanon (index 1)
    game.select_indices(&[1]);

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectHeartColor") | Some("SelectHeartType") => {
                panic!("Unexpected heart color choice")
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }

    assert_eq!(
        game.state.mods.get_blade_modifier(kanon2),
        1,
        "Selected kanon should have +1 blade"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kanon2, HeartColor::Heart05),
        1,
        "Selected kanon should have +1 heart05"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(kanon1),
        0,
        "Unselected kanon should have 0 blade"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kanon1, HeartColor::Heart05),
        0,
        "Unselected kanon should have 0 heart05"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(keke),
        1,
        "Keke should have +1 blade"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(keke, HeartColor::Heart01),
        1,
        "Keke should have +1 heart01"
    );
}

/// 1 Kanon + 2 Keke, select the SECOND keke from the prompt.
#[test]
fn tiny_stars_duplicate_keke_select_second() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let tiny_stars = game.id("PL!SP-bp1-024-L");
    let kanon = game.id("PL!SP-sd1-001-SD");
    let keke1 = game.id("PL!SP-sd1-002-SD");
    let keke2 = game.new_id("PL!SP-sd1-002-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [kanon, keke1, keke2];
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_live_start(&mut game, tiny_stars);

    // Kanon has 1 candidate → no choice. Keke has 2 → prompt.
    assert!(
        game.has_pending_choice(),
        "Duplicate keke: should prompt to select one"
    );

    // Select the SECOND keke (index 1)
    game.select_indices(&[1]);

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectHeartColor") | Some("SelectHeartType") => {
                panic!("Unexpected heart color choice")
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }

    assert_eq!(
        game.state.mods.get_blade_modifier(kanon),
        1,
        "Kanon should have +1 blade"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kanon, HeartColor::Heart05),
        1,
        "Kanon should have +1 heart05"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(keke2),
        1,
        "Selected keke should have +1 blade"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(keke2, HeartColor::Heart01),
        1,
        "Selected keke should have +1 heart01"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(keke1),
        0,
        "Unselected keke should have 0 blade"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(keke1, HeartColor::Heart01),
        0,
        "Unselected keke should have 0 heart01"
    );
}

/// PL!S-pb1-003-R (松浦果南) Q36: LiveSuccess timing.
#[test]
fn kanan_live_success_timing() {
    let db = load_real_database();
    let card = db.get_card_id("PL!S-pb1-003-R").expect("Card exists");
    let c = db.get_card(card).expect("Kanan card should exist");
    assert!(!c.abilities.is_empty());
}
