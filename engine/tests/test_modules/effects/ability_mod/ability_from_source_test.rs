/// Tests for gain_ability_from_source (ability copying from cards under member).
///
/// Card: PL!N-PR-026-PR | 天王寺璃奈 (ab#1)
/// 常時: このメンバーは、このメンバーの下に置かれているコスト9以下の『虹ヶ咲』の
/// メンバーカードが持つライブ成功時能力をすべて得る。
///
/// NOTE: gained_abilities stores the triggerless_text of copied abilities, but the
/// engine's auto-ability trigger pipeline does not currently read gained_abilities
/// at runtime. These tests verify the COPYING MECHANISM (filtering, storage) works.
/// Dynamic ability execution is a separate pending feature.
use crate::helpers::*;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// Basic: Rina on stage, Ayumu under her → Ayumu's live_success ability text is stored.
#[test]
fn rina_gains_ability_from_under_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rina = game.id("PL!N-PR-026-PR");
    // Ayumu (PL!N-bp4-001-R): 虹ヶ咲 member, cost=2, has live_success ability
    let ayumu = game.id("PL!N-bp4-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [rina, filler, -1];
    game.state.player1.stage.under_cards[0].push(ayumu);

    fill_decks(&mut game, filler);
    game.give_energy(5);

    game.pass();
    game.pass();

    let gained = game.state.gained_abilities.get(&rina);
    assert!(gained.is_some(), "Rina should have gained abilities");
    if let Some(list) = gained {
        assert!(!list.is_empty(), "Should copy at least one ability");
        // Verify the entry format: ability_from_source:{db_id}:{triggerless_text}
        assert!(
            list[0].starts_with("ability_from_source:"),
            "Bad entry format"
        );
    }
}

/// Trigger filter: a card with 常時 (constant) ability under Rina should NOT be copied.
#[test]
fn rina_only_copies_live_success_not_constant() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rina = game.id("PL!N-PR-026-PR");
    // Asaka Karin (PL!N-PR-027-PR): 虹ヶ咲 member, cost=4, has 常時 ability (not live_success)
    let karin = game.id("PL!N-PR-027-PR");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [rina, filler, -1];
    game.state.player1.stage.under_cards[0].push(karin);

    fill_decks(&mut game, filler);
    game.give_energy(5);

    game.pass();
    game.pass();

    let gained = game.state.gained_abilities.get(&rina);
    assert!(
        gained.is_none() || gained.unwrap().is_empty(),
        "Rina should not gain constant abilities (trigger filter mismatch)"
    );
}

/// Cost limit: card under member with cost > 9 should NOT be copied.
#[test]
fn rina_respects_cost_limit() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rina = game.id("PL!N-PR-026-PR");
    // Setsuna (PL!N-bp4-007-R+, cost=13) is 虹ヶ咲 member with live_success,
    // but cost 13 > 9 → filtered out
    let setsuna = game.id("PL!N-bp4-007-R+");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [rina, filler, -1];
    game.state.player1.stage.under_cards[0].push(setsuna);

    fill_decks(&mut game, filler);
    game.give_energy(5);

    game.pass();
    game.pass();

    let gained = game.state.gained_abilities.get(&rina);
    assert!(
        gained.is_none() || gained.unwrap().is_empty(),
        "Setsuna (cost=13) should be filtered by cost limit"
    );
}

/// No cards under member: graceful empty result.
#[test]
fn rina_no_under_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rina = game.id("PL!N-PR-026-PR");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [rina, filler, -1];
    fill_decks(&mut game, filler);
    game.give_energy(5);

    game.pass();
    game.pass();

    let gained = game.state.gained_abilities.get(&rina);
    assert!(
        gained.is_none() || gained.unwrap().is_empty(),
        "No abilities should be gained with no cards under member"
    );
}

/// Rina not on stage: graceful skip.
#[test]
fn rina_not_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rina = game.id("PL!N-PR-026-PR");
    let ayumu = game.id("PL!N-bp4-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(rina);
    game.state.player1.stage.stage = [filler, filler, -1];
    game.state.player1.stage.under_cards[0].push(ayumu);

    fill_decks(&mut game, filler);
    game.give_energy(5);

    game.pass();
    game.pass();

    let gained = game.state.gained_abilities.get(&rina);
    assert!(
        gained.is_none() || gained.unwrap().is_empty(),
        "No abilities should be gained when Rina is not on stage"
    );
}

/// Multiple matching cards: both should have their ability texts copied.
#[test]
fn rina_copies_from_multiple_under_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rina = game.id("PL!N-PR-026-PR");
    let ayumu = game.id("PL!N-bp4-001-R"); // cost=2, 虹ヶ咲, live_success
    let shizuku = game.id("PL!N-bp4-003-R"); // cost=4, 虹ヶ咲, live_success
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [rina, filler, -1];
    game.state.player1.stage.under_cards[0].push(ayumu);
    game.state.player1.stage.under_cards[0].push(shizuku);

    fill_decks(&mut game, filler);
    game.give_energy(5);

    game.pass();
    game.pass();

    let gained = game.state.gained_abilities.get(&rina);
    assert!(gained.is_some(), "Rina should have gained abilities");
    if let Some(list) = gained {
        assert!(
            list.len() >= 2,
            "Should copy from both under-cards, got {} entries",
            list.len()
        );
    }
}
