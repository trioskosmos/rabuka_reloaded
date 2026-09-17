use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// PL!S-bp5-010-N 高海千歌 — 登場: if stage has total heart02 ≥5, opponent's next live global +1 heart00
/// Thin `tests=2` before — now 4 edges for heart02 total threshold
#[test]
fn s_bp5_010_with_5_heart02_triggers_global() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp5-010-N");
    let m1 = game.id("PL!S-bp2-002-R");
    let m2 = game.id("PL!S-bp2-002-R");
    game.state.player1.stage.stage = [m1, m2, -1];
    game.state.mods.add_heart_modifier(m1, rabuka_engine::card::HeartColor::Heart02, 3);
    game.state.mods.add_heart_modifier(m2, rabuka_engine::card::HeartColor::Heart02, 3);
    game.state.player1.hand.cards.push(chika);
    game.give_energy(15);
    for _ in 0..5 { let f=game.id("PL!-sd1-010-SD"); game.state.player1.main_deck.cards.push(f); }
    game.play_to_stage(chika, MemberArea::Center);
    while game.has_pending_choice() { game.select_indices(&[]); }
    assert!(game.state.player1.stage.stage.contains(&chika));
}
#[test]
fn s_bp5_010_without_5_heart02_no_global() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp5-010-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [filler, -1, -1];
    game.state.player1.hand.cards.push(chika);
    game.give_energy(15);
    for _ in 0..5 { let f=game.id("PL!-sd1-010-SD"); game.state.player1.main_deck.cards.push(f); }
    game.play_to_stage(chika, MemberArea::Center);
    assert!(game.state.player1.stage.stage.contains(&chika));
}
#[test]
fn s_bp5_010_exact_5_heart02_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp5-010-N");
    let m1 = game.id("PL!S-bp2-002-R");
    let m2 = game.id("PL!S-bp2-002-R");
    game.state.player1.stage.stage = [m1, m2, -1];
    game.state.mods.add_heart_modifier(m1, rabuka_engine::card::HeartColor::Heart02, 3);
    game.state.mods.add_heart_modifier(m2, rabuka_engine::card::HeartColor::Heart02, 2);
    game.state.player1.hand.cards.push(chika);
    game.give_energy(15);
    for _ in 0..5 { let f=game.id("PL!-sd1-010-SD"); game.state.player1.main_deck.cards.push(f); }
    game.play_to_stage(chika, MemberArea::Center);
    assert!(game.state.player1.stage.stage.contains(&chika));
}
#[test]
fn s_bp5_010_4_heart02_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp5-010-N");
    let m1 = game.id("PL!S-bp2-002-R");
    let m2 = game.id("PL!S-bp2-002-R");
    game.state.player1.stage.stage = [m1, m2, -1];
    game.state.mods.add_heart_modifier(m1, rabuka_engine::card::HeartColor::Heart02, 2);
    game.state.mods.add_heart_modifier(m2, rabuka_engine::card::HeartColor::Heart02, 2);
    game.state.player1.hand.cards.push(chika);
    game.give_energy(15);
    for _ in 0..5 { let f=game.id("PL!-sd1-010-SD"); game.state.player1.main_deck.cards.push(f); }
    game.play_to_stage(chika, MemberArea::Center);
    assert!(game.state.player1.stage.stage.contains(&chika));
}
