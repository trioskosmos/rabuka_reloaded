/// Untested-abilities batch 42 — color coverage, CatChu! trio, live-cost look.
///
/// - PL!N-bp5-015-N 桜坂しずく (ライブ開始時): if the stage members'
///   hearts collectively include ALL of heart01..06 -> +2 blades.
/// - PL!SP-bp7-015-N 平安名すみれ (ライブ開始時, opt. {E}): exactly 3 staged
///   『CatChu!』 members -> draw 1.
/// - PL!SP-bp7-018-N 米女メイ (登場, opt. discard 1 live from hand): look at
///   top 5, add 1 to hand, rest to waitroom.
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

// ====================================================================
// PL!N-bp5-015-N 桜坂しずく — all-six-colors collective gate
// ====================================================================

fn fire_live_start(game: &mut TestGame, cid: i16) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap_or_else(|| panic!("card {} lacks a ライブ開始時 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

#[test]
fn bp5015_all_six_colors_present_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let shizuku = game.id("PL!N-bp5-015-N");
    game.state.player1.stage.stage[0] = shizuku;
    // Honoka {01,03,06} + Kanan {02,04,05} = all six colors collectively.
    let honoka = game.id("PL!-sd1-001-SD");
    let kanan = game.id("PL!S-sd1-003-SD");
    game.state.player1.stage.stage[1] = honoka;
    game.state.player1.stage.stage[2] = kanan;

    fire_live_start(&mut game, shizuku);

    assert_eq!(
        game.state.mods.get_blade_modifier(shizuku),
        2,
        "all six colors present across members -> +2 blades"
    );
}

#[test]
fn bp5015_missing_colors_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let shizuku = game.id("PL!N-bp5-015-N");
    game.state.player1.stage.stage[0] = shizuku;
    // Honoka covers only {01,03,06}.
    let honoka = game.id("PL!-sd1-001-SD");
    game.state.player1.stage.stage[1] = honoka;

    fire_live_start(&mut game, shizuku);

    assert_eq!(
        game.state.mods.get_blade_modifier(shizuku),
        0,
        "colors 02/04/05 missing -> no blades"
    );
}

// ====================================================================
// PL!SP-bp7-015-N 平安名すみれ — CatChu! x3 optional-energy draw
// ====================================================================

#[test]
fn sumire_three_catchu_draws() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let sumire = game.id("PL!SP-bp7-015-N"); // CatChu! herself
    game.state.player1.stage.stage[0] = sumire;
    let c1 = game.id("PL!SP-PR-003-PR"); // CatChu! cost 2
    let c2 = game.id("PL!SP-PR-006-PR"); // CatChu! cost 4
    game.state.player1.stage.stage[1] = c1;
    game.state.player1.stage.stage[2] = c2;

    let deck_before = game.state.player1.main_deck.cards.len();
    game.give_energy(5);
    fire_live_start(&mut game, sumire);
    assert!(game.has_pending_choice(), "optional energy cost prompted");
    game.select_option(1); // pay the energy

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        1,
        "3 CatChu! members staged + paid -> draw 1"
    );
}

#[test]
fn sumire_only_two_catchu_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let sumire = game.id("PL!SP-bp7-015-N");
    game.state.player1.stage.stage[0] = sumire;
    let c1 = game.id("PL!SP-PR-003-PR");
    game.state.player1.stage.stage[1] = c1;
    let mu_member = game.id("PL!-sd1-010-SD"); // wrong unit
    game.state.player1.stage.stage[2] = mu_member;

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_live_start(&mut game, sumire);

    assert_eq!(
        deck_before, game.state.player1.main_deck.cards.len(),
        "only 2 CatChu! members -> no draw"
    );
}

// ====================================================================
// PL!SP-bp7-018-N 米女メイ — 登場 with optional live-card discard + look 5
// ====================================================================

#[test]
fn mei_accept_live_cost_look_five_take_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.new_id("PL!SP-bp7-018-N");
    let live_cost = game.new_id("PL!-sd1-019-SD"); // LIVE card for the cost
    game.add_to_hand(me);
    game.add_to_hand(live_cost);

    let a = game.new_id("PL!S-sd1-001-SD");
    let b = game.new_id("PL!-sd1-019-SD"); // live card among the looked five
    game.give_energy(20);
    // Deck top order: [a, b, filler...].
    game.state.player1.main_deck.cards.insert(0, b);
    game.state.player1.main_deck.cards.insert(0, a);

    game.play_to_stage(me, MemberArea::LeftSide);

    // Prompt chain: optional live-discard gate -> which-card selection.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    // The live card paid as cost sits in the waitroom.
    assert!(
        game.state.player1.waitroom.cards.contains(&live_cost),
        "cost live card was discarded to the waitroom"
    );
    // The first looked card was taken into the hand.
    assert!(
        game.state.player1.hand.cards.contains(&a),
        "one looked card added to hand"
    );
}

#[test]
fn mei_decline_cost_no_look() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.new_id("PL!SP-bp7-018-N");
    let live_cost = game.new_id("PL!-sd1-019-SD");
    game.add_to_hand(me);
    game.add_to_hand(live_cost);
    game.give_energy(20);

    game.play_to_stage(me, MemberArea::LeftSide);
    // Decline the optional live-card discard.
    game.select_indices(&[]);

    assert!(
        game.state.player1.hand.cards.contains(&live_cost),
        "declined: live card stays in hand"
    );
}
