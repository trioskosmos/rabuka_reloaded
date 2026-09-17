use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const ME: &str = "PL!N-PR-032-PR";
const W1: &str = "PL!-sd1-001-SD";
const W2: &str = "PL!S-sd1-003-SD";
const W3: &str = "PL!-sd1-007-SD";
const W4: &str = "PL!HS-bp5-001-P";
const W5: &str = "PL!SP-bp5-006-R";
const W6: &str = "PL!S-bp5-009-R";
const W7: &str = "PL!N-bp3-006-R";

fn setsuna_pl_n_pr_032_pr_fire_debut(game: &mut TestGame) {
    let cid = game.id(ME);
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("登場"))
            .unwrap_or_else(|| panic!("card {} lacks a 登場 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::Debut,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

fn setsuna_pl_n_pr_032_pr_setup(
    game: &mut TestGame,
    me: i16,
    waitroom: &[i16],
    mill_cards: &[i16],
) {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    game.state.player1.stage.stage[0] = me;
    for &w in waitroom {
        game.state.player1.waitroom.cards.push(w);
    }
    for &mc in mill_cards.iter().rev() {
        game.state.player1.main_deck.cards.insert(0, mc);
    }
}

#[test]
fn setsuna_pl_n_pr_032_pr_mills_shortfall_and_recovers_live_to_deck_top() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(ME);
    let wr: Vec<i16> = [W1, W2, W3, W4, W5].iter().map(|w| game.id(w)).collect();
    let member_a = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");
    let member_b = game.id("PL!S-sd1-001-SD");

    setsuna_pl_n_pr_032_pr_setup(&mut game, me, &wr, &[member_a, live, member_b]);

    let wait_before = game.state.player1.waitroom.cards.len();
    setsuna_pl_n_pr_032_pr_fire_debut(&mut game);

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        wait_before + 3,
        "waitroom 5 of 8 -> mill exactly 3"
    );

    assert!(
        game.has_pending_choice(),
        "live-card retrieval prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the retrieval (zone=discard, allow_skip)"
    );
    game.select_indices(&[0]);

    assert_eq!(
        game.state.player1.main_deck.cards.first(),
        Some(&live),
        "live card recovered to deck TOP"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live),
        "recovered live card left the waitroom"
    );
}

#[test]
fn setsuna_pl_n_pr_032_pr_declining_keeps_milled_live_in_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(ME);
    let wr: Vec<i16> = [W1, W2, W3, W4, W5].iter().map(|w| game.id(w)).collect();
    let member_a = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");
    let member_b = game.id("PL!S-sd1-001-SD");

    setsuna_pl_n_pr_032_pr_setup(&mut game, me, &wr, &[member_a, live, member_b]);

    setsuna_pl_n_pr_032_pr_fire_debut(&mut game);

    assert!(
        game.has_pending_choice(),
        "live-card retrieval prompt expected (decline path)"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the retrieval (zone=discard, allow_skip)"
    );
    game.select_indices(&[]);

    assert!(
        !game.state.player1.main_deck.cards.contains(&live),
        "declined: live card must not enter the deck at all"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live),
        "declined: live card stays in the waitroom"
    );
}

#[test]
fn setsuna_pl_n_pr_032_pr_no_live_among_milled_no_prompt() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(ME);
    let wr: Vec<i16> = [W1, W2].iter().map(|w| game.id(w)).collect();

    let m = game.id("PL!-sd1-010-SD");
    setsuna_pl_n_pr_032_pr_setup(&mut game, me, &wr, &[m]);

    let wait_before = game.state.player1.waitroom.cards.len();
    setsuna_pl_n_pr_032_pr_fire_debut(&mut game);

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        wait_before + 6,
        "waitroom 2 of 8 -> mill exactly 6"
    );
    assert!(
        !game.has_pending_choice(),
        "no live card among the milled -> no recovery prompt"
    );
}

#[test]
fn setsuna_pl_n_pr_032_pr_waitroom_at_threshold_mills_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(ME);
    let wr: Vec<i16> = [W1, W2, W3, W4, W5, W6, W7]
        .iter()
        .map(|w| game.id(w))
        .chain(std::iter::once(game.id("PL!SP-bp4-022-N")))
        .collect();
    let m = game.id("PL!-sd1-010-SD");

    setsuna_pl_n_pr_032_pr_setup(&mut game, me, &wr, &[m]);

    let deck_before = game.state.player1.main_deck.cards.len();
    setsuna_pl_n_pr_032_pr_fire_debut(&mut game);

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "waitroom >= 8 -> gate fails, nothing mills"
    );
    assert!(
        !game.has_pending_choice(),
        "nothing milled -> no recovery prompt"
    );
}

#[test]
fn setsuna_pl_n_pr_032_pr_waitroom_seven_mills_one_and_recovers_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(ME);
    let wr: Vec<i16> = [W1, W2, W3, W4, W5, W6, W7]
        .iter()
        .map(|w| game.id(w))
        .collect();
    let live = game.id("PL!-sd1-019-SD");

    setsuna_pl_n_pr_032_pr_setup(&mut game, me, &wr, &[live]);

    let wait_before = game.state.player1.waitroom.cards.len();
    setsuna_pl_n_pr_032_pr_fire_debut(&mut game);

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        wait_before + 1,
        "waitroom 7 of 8 -> mill exactly 1"
    );

    assert!(
        game.has_pending_choice(),
        "milled live card -> recovery prompt"
    );
    game.select_indices(&[0]);
    assert_eq!(
        game.state.player1.main_deck.cards.first(),
        Some(&live),
        "accepted -> live card recovered to deck top"
    );
}
