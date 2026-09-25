use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const NIJI_CHOICE: &str = "PL!N-pb1-010-R";
const NIJI_LIVE: &str = "PL!N-bp1-026-L";
const FILLER: &str = "PL!-sd1-010-SD";

fn fill_deck(game: &mut TestGame, player: &str, count: usize) {
    let ids: Vec<i16> = (0..count).map(|_| game.id(FILLER)).collect();
    let deck = if player == "p1" {
        &mut game.state.player1.main_deck.cards
    } else {
        &mut game.state.player2.main_deck.cards
    };
    for f in ids {
        deck.push(f);
    }
}

#[test]
fn pl_n_pb1_010_r_energy_option_leaves_active_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id(NIJI_CHOICE);
    let e1 = game.id("LL-E-001-SD");
    game.state.player1.energy_zone.cards.push(e1);
    fill_deck(&mut game, "p1", 5);
    game.give_energy(10);
    game.add_to_hand(member);
    game.play_to_stage(member, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_choice_option(0);
    }
    let active = game.state.player1.energy_zone.active_count();
    assert!(
        active >= 1,
        "option 0 should activate 1 energy, got {}",
        active
    );
}

#[test]
fn pl_n_pb1_010_r_recycle_live_option_increases_deck_size() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id(NIJI_CHOICE);
    let live1 = game.id(NIJI_LIVE);
    let live2 = game.id(NIJI_LIVE);
    game.state.player1.waitroom.cards.push(live1);
    game.state.player1.waitroom.cards.push(live2);
    fill_deck(&mut game, "p1", 5);
    game.give_energy(10);
    let deck_before = game.state.player1.main_deck.cards.len();
    game.add_to_hand(member);
    game.play_to_stage(member, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_choice_option(1);
    }
    let deck_after = game.state.player1.main_deck.cards.len();
    assert!(
        deck_after > deck_before,
        "option 1 should put cards on deck top: before={}, after={}",
        deck_before,
        deck_after
    );
}
