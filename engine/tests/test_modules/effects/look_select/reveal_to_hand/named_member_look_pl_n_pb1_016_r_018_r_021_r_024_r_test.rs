use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";

fn fire_pl_n_pb1_016_r_debut(game: &mut TestGame, cid: i16) {
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

fn fire_named_member_look_debut(game: &mut TestGame, cid: i16) {
    fire_trigger(game, cid, AbilityTrigger::Debut, "登場");
}

#[test]
fn pl_n_pb1_016_r_named_look_removes_karin_from_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let karin = game.id("PL!N-bp1-004-R");
    let other = game.new_id("PL!S-sd1-001-SD");
    game.state.player1.main_deck.cards.insert(0, other);
    game.state.player1.main_deck.cards.insert(0, karin);
    let wr_card = game.new_id("PL!-sd1-010-SD");
    game.state.player1.waitroom.cards.push(wr_card);

    let me = game.id("PL!N-pb1-016-R");
    game.state.player1.stage.stage[1] = me;
    fire_pl_n_pb1_016_r_debut(&mut game, me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        !game.state.player1.main_deck.cards.contains(&karin),
        "Karin left the deck"
    );
    assert!(
        game.state.player1.hand.cards.contains(&karin),
        "the looked Karin member was revealed into the hand"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&other),
        "the non-Karin looked card went to the waitroom"
    );
}

#[test]
fn pl_n_pb1_018_r_looks_two_reveals_kanata_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let kanata = game.new_id("PL!N-bp1-018-N");
    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, kanata);

    let me = game.id("PL!N-pb1-018-R");
    game.state.player1.stage.stage[1] = me;
    fire_named_member_look_debut(&mut game, me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        !game.state.player1.main_deck.cards.contains(&kanata),
        "Kanata left the deck"
    );
    assert!(
        game.state.player1.hand.cards.contains(&kanata),
        "Kanata revealed to hand"
    );
}

#[test]
fn pl_n_pb1_018_r_no_matching_kanata_adds_nothing_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, filler);

    let hand_before = game.state.player1.hand.cards.len();
    let me = game.id("PL!N-pb1-018-R");
    game.state.player1.stage.stage[1] = me;
    fire_named_member_look_debut(&mut game, me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "no Kanata in the looked two -> nothing added to hand"
    );
}

fn resolve_named_member_look_with_seed(
    game: &mut TestGame,
    holder_no: &str,
    seed_no: Option<&str>,
) -> i16 {
    let filler = game.new_id("PL!N-sd1-010-SD");
    fill_decks(game, filler);
    let me = game.id(holder_no);
    game.state.player1.stage.stage[1] = me;
    let seed = game.new_id(seed_no.unwrap_or(holder_no));
    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, seed);
    fire_named_member_look_debut(game, me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    seed
}

#[test]
fn pl_n_pb1_021_r_looks_two_reveals_rina_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let seed = resolve_named_member_look_with_seed(&mut game, "PL!N-pb1-021-R", None);
    assert!(
        game.state.player1.hand.cards.contains(&seed),
        "「天王寺璃奈」 member revealed to hand"
    );
}

#[test]
fn pl_n_pb1_021_r_wrong_character_stays_out_of_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let seed =
        resolve_named_member_look_with_seed(&mut game, "PL!N-pb1-021-R", Some("PL!N-bp7-012-R"));
    assert!(
        !game.state.player1.hand.cards.contains(&seed),
        "non-璃奈 member on top must NOT be revealed to hand"
    );
}

#[test]
fn pl_n_pb1_024_r_looks_two_reveals_lanzhu_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let seed = resolve_named_member_look_with_seed(&mut game, "PL!N-pb1-024-R", None);
    assert!(
        game.state.player1.hand.cards.contains(&seed),
        "「鐘嵐珠」 member revealed to hand"
    );
}
