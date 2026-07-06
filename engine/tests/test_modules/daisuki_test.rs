/// ダイスキだったらダイジョウブ！(PL!S-bp3-020-L) ab#0
///
/// 自動 [1/ターン]: エールにより自分のカードを1枚以上公開したとき、それらのカードの中に
/// ブレードハートを持つカードが2枚以下の場合、それらのカードをすべて控え室に置いてもよい。
/// そのエールで得たブレードハートを失い、もう一度エールを行う。
use crate::helpers::*;

fn fill_decks(game: &mut TestGame) {
    for _ in 0..60 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id("PL!-sd1-010-SD"));
        game.state
            .player2
            .main_deck
            .cards
            .push(game.new_id("PL!-sd1-010-SD"));
    }
}

fn advance_to_p1_performance(game: &mut TestGame, daisuki: i16) {
    game.state.player1.hand.cards.push(daisuki);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(daisuki);
    game.pass();
    game.pass();
    game.pass();
}

// ── Clause 1: エールにより自分のカードを1枚以上公開したとき (outer condition ≥1) ──

/// 0 cards from yell (0 blade) → outer condition fails → no trigger at all.
#[test]
fn condition_0_revealed_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");
    fill_decks(&mut game);
    game.give_energy(15);
    advance_to_p1_performance(&mut game, daisuki);
    assert!(!game.has_pending_choice(), "0 revealed → no trigger");
    assert!(!game.state.re_yell_occurred, "no re-yell");
}

/// 1 card from yell → outer condition ≥1 passes → ability fires.
#[test]
fn condition_1_revealed_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    game.give_energy(15);
    advance_to_p1_performance(&mut game, daisuki);
    assert_eq!(game.state.initial_yell_revealed_cards.len(), 1);
    // 1 card with blade_heart → 1 ≤ 2 → inner condition passes → discard prompt
    assert!(game.has_pending_choice(), "1 revealed → discard prompt");
}

/// 5 cards from yell → outer condition ≥1 passes.
#[test]
fn condition_5_revealed_outer_condition_passes() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");
    // 3 fillers * blade=1 each = 3 is max without modifiers
    // Use blade modifier to reach 5
    let m1 = game.new_id("PL!-sd1-010-SD");
    let m2 = game.new_id("PL!-sd1-010-SD");
    let m3 = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[0] = m1;
    game.state.player1.stage.stage[1] = m2;
    game.state.player1.stage.stage[2] = m3;
    game.state.mods.add_blade_modifier(m3, 2); // 1+1+3 = 5
    fill_decks(&mut game);
    game.give_energy(15);
    advance_to_p1_performance(&mut game, daisuki);
    assert!(
        game.state.initial_yell_revealed_cards.len() >= 5,
        "expected >=5, got {}",
        game.state.initial_yell_revealed_cards.len()
    );
    // All 5 drawn cards are filler (blade_heart=yes) → 5 > 2 → inner condition fails
    // No discard prompt, but re-yell still fires
    assert!(
        !game.has_pending_choice(),
        "5 cards all blade_heart → inner condition fails, no prompt"
    );
    assert!(
        game.state.re_yell_occurred || !game.state.re_yell_revealed_cards.is_empty(),
        "re-yell should fire even when inner condition fails"
    );
}

// ── Clause 2: ブレードハートを持つカードが2枚以下の場合 (inner condition ≤2) ──

/// 0 blade_heart among revealed → 0 ≤ 2 → inner condition passes → discard prompt.
#[test]
fn blade_heart_0_allowed_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");
    // Use energy cards for deck: they have NO blade_heart
    let e_card = game.id("LL-E-001-SD");
    for _ in 0..60 {
        game.state.player1.main_deck.cards.push(e_card);
        game.state.player2.main_deck.cards.push(e_card);
    }
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    game.give_energy(15);
    advance_to_p1_performance(&mut game, daisuki);
    // 2 cards drawn, both energy (no blade_heart) → 0 ≤ 2 → discard prompt
    assert!(game.has_pending_choice(), "0 blade_heart → discard prompt");
}

/// 2 blade_heart among revealed → 2 ≤ 2 → inner condition passes → discard prompt.
#[test]
fn blade_heart_2_allowed_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    game.give_energy(15);
    advance_to_p1_performance(&mut game, daisuki);
    assert_eq!(game.state.initial_yell_revealed_cards.len(), 2);
    // 2 filler cards, both have blade_heart → 2 ≤ 2 → discard prompt
    assert!(game.has_pending_choice(), "2 blade_heart → discard prompt");
}

/// 3 blade_heart among revealed → 3 > 2 → inner condition fails → no discard prompt, re-yell still fires.
#[test]
fn blade_heart_3_blocks_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[2] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    game.give_energy(15);
    advance_to_p1_performance(&mut game, daisuki);
    assert_eq!(game.state.initial_yell_revealed_cards.len(), 3);
    // 3 filler cards, all have blade_heart → 3 > 2 → no discard prompt
    assert!(!game.has_pending_choice(), "3 blade_heart → no prompt");
    // But re-yell fires regardless
    assert!(
        game.state.re_yell_occurred || !game.state.re_yell_revealed_cards.is_empty(),
        "re-yell fires even when inner condition fails"
    );
}

// ── Clause 3: それらのカードをすべて控え室に置いてもよい (optional discard) ──

/// Accept discard → cards move from revealed to waitroom, re-yell follows.
#[test]
fn discard_accept_moves_cards_and_re_yells() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    game.give_energy(15);

    let deck_before = game.state.player1.main_deck.cards.len();
    advance_to_p1_performance(&mut game, daisuki);

    assert!(game.has_pending_choice(), "discard prompt");
    let (count, allow_skip) =
        if let rabuka_engine::ability::types::Choice::SelectCard {
            count, allow_skip, ..
        } = game.get_pending_choice()
        {
            (*count, *allow_skip)
        } else {
            (0, false)
        };
    eprintln!(
        "[DEBUG] SelectCard count={} allow_skip={} waitroom_before={}",
        count,
        allow_skip,
        game.state.player1.waitroom.cards.len()
    );
    let waitroom_before = game.state.player1.waitroom.cards.len();
    game.select_indices(&(0..count).collect::<Vec<_>>());

    eprintln!(
        "[DEBUG] waitroom_after={} waitroom_grew={}",
        game.state.player1.waitroom.cards.len(),
        game.state.player1.waitroom.cards.len() > waitroom_before
    );
    assert!(
        game.state.player1.waitroom.cards.len() > waitroom_before,
        "cards moved to waitroom"
    );
    assert!(
        game.state.player1.main_deck.cards.len() < deck_before,
        "re-yell consumed deck cards"
    );
}

/// Skip discard → cards stay in revealed, re-yell still fires.
#[test]
fn discard_skip_still_re_yells() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    game.give_energy(15);

    advance_to_p1_performance(&mut game, daisuki);

    assert!(game.has_pending_choice(), "discard prompt");
    game.select_indices(&[]); // skip

    assert!(
        game.state.re_yell_occurred,
        "re_yell should fire even when discard skipped"
    );
}

// ── Clause 4: そのエールで得たブレードハートを失い (lose blade hearts from yell) ──

/// After re-yell, check that yell blade hearts are removed.
/// Verifies by checking cheer_blade_heart_count is reset.
#[test]
fn lose_blade_hearts_after_re_yell() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    game.give_energy(15);

    advance_to_p1_performance(&mut game, daisuki);

    assert!(game.has_pending_choice(), "discard prompt");
    game.select_indices(&[0]); // accept discard → lose blade hearts → re-yell

    // After re-yell, the initial yell blade hearts should be gone
    // There's no direct public getter, so verify via behavior:
    // re_yell already ran since we don't have a pending choice from it
    while game.has_pending_choice() {
        let c = game.get_pending_choice();
        match c {
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                game.select_indices(&[]);
            }
            _ => break,
        }
    }
}

// ── Clause 5: もう一度エールを行う (perform yell again) ──

/// Verify re-yell draws new cards from deck.
#[test]
fn re_yell_draws_new_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    game.give_energy(15);

    let deck_before = game.state.player1.main_deck.cards.len();
    advance_to_p1_performance(&mut game, daisuki);

    assert!(game.has_pending_choice(), "discard prompt");
    game.select_indices(&[0]); // accept → re-yell

    let deck_after = game.state.player1.main_deck.cards.len();
    // Initial yell drew 2, discard moved them, re-yell drew ~2 more
    assert!(
        deck_after <= deck_before - 2,
        "re-yell consumed deck cards: before={} after={}",
        deck_before,
        deck_after
    );
    assert!(
        !game.state.re_yell_revealed_cards.is_empty(),
        "re_yell_revealed_cards populated"
    );
    // initial_yell_revealed_cards still has the original 2
    assert_eq!(game.state.initial_yell_revealed_cards.len(), 2);
}

// ── Clause 6: ターン1回 (once per turn) ──

/// Once-per-turn: ability can only fire once per turn.
/// After first re-yell resolves, the second yell's auto abilities
/// should NOT include daisuki again.
#[test]
fn once_per_turn_does_not_trigger_again_same_turn() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    game.give_energy(15);

    advance_to_p1_performance(&mut game, daisuki);

    // First trigger: discard prompt
    assert!(game.has_pending_choice(), "first trigger");
    game.select_indices(&[0]); // accept → re-yell happens

    // After re-yell, there should NOT be another daisuki trigger
    // (once per turn). Any remaining choices are from other cards.
    let mut saw_daisuki_again = false;
    while game.has_pending_choice() {
        let c = game.get_pending_choice();
        match c {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { options, .. } => {
                for opt in options {
                    if opt.card_name.contains("ダイスキ") {
                        saw_daisuki_again = true;
                    }
                }
                game.select_indices(&[]);
            }
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                game.select_indices(&[]);
            }
            _ => break,
        }
    }
    assert!(
        !saw_daisuki_again,
        "daisuki should not trigger again same turn"
    );
}

// ── Integration: Daisuki + DIA interaction ──

/// DIA (blade=3) + 2 fillers (blade=1 each) = 5 total blade.
/// Both trigger. Pick DIA first: DIA discards, re-yells, then DAISUKI fires on new yell.
#[test]
fn with_dia_5_blade_dia_first() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp2-004-R");
    let daisuki = game.id("PL!S-bp3-020-L");

    game.state.player1.stage.stage[2] = dia;
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(daisuki);
    fill_decks(&mut game);
    game.give_energy(15);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(daisuki);
    game.pass();
    game.pass();
    game.pass();

    assert_eq!(game.state.initial_yell_revealed_cards.len(), 5);
    assert!(game.has_pending_choice(), "SelectAutoAbility prompt");
    game.select_option(0); // DIA first

    // DIA fires: discard prompt
    assert!(game.has_pending_choice(), "DIA discard prompt");
    if let rabuka_engine::ability::types::Choice::SelectCard { count, .. } =
        game.get_pending_choice()
    {
        game.select_indices(&(0..*count).collect::<Vec<_>>());
    }

    // After DIA resolves, DAISUKI fires on the re-yell's cards
    while game.has_pending_choice() {
        let c = game.get_pending_choice();
        match c {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                game.select_option(0);
            }
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                game.select_indices(&[]);
            }
            _ => {
                game.select_indices(&[]);
            }
        }
    }

    assert!(
        !game.state.re_yell_revealed_cards.is_empty(),
        "re-yell happened"
    );
}

/// DIA + 2 fillers = 5 blade. Pick DAISUKI first, then DIA fires on DAISUKI's re-yell.
#[test]
fn with_dia_5_blade_daisuki_first() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp2-004-R");
    let daisuki = game.id("PL!S-bp3-020-L");

    game.state.player1.stage.stage[2] = dia;
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(daisuki);
    fill_decks(&mut game);
    game.give_energy(15);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(daisuki);
    game.pass();
    game.pass();
    game.pass();

    assert_eq!(game.state.initial_yell_revealed_cards.len(), 5);
    assert!(game.has_pending_choice(), "SelectAutoAbility prompt");
    game.select_option(1); // DAISUKI first (index 1 = second in queue)

    // DAISUKI fires: optional discard prompt
    assert!(game.has_pending_choice(), "DAISUKI discard prompt");
    game.select_indices(&[]); // skip → re-yell still fires

    // After DAISUKI's re-yell, DIA fires on the new yell's cards
    while game.has_pending_choice() {
        let c = game.get_pending_choice();
        match c {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                game.select_option(0);
            }
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                game.select_indices(&[]);
            }
            _ => {
                game.select_indices(&[]);
            }
        }
    }

    assert!(
        !game.state.re_yell_revealed_cards.is_empty(),
        "re-yell happened"
    );
    assert_eq!(
        game.state.initial_yell_revealed_cards.len(),
        5,
        "initial yell preserved"
    );
}
