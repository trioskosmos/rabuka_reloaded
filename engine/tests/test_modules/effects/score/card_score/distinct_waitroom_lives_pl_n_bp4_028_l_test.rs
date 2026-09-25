use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

fn fire_trigger(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trig: &str) {
    fire_trigger_nth(game, cid, trigger, trig, 0);
}

fn fire_trigger_nth(
    game: &mut TestGame,
    cid: i16,
    trigger: AbilityTrigger,
    trig: &str,
    nth: usize,
) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .filter(|a| a.triggers.as_deref() == Some(trig))
            .nth(nth)
            .unwrap_or_else(|| panic!("card {} lacks '{trig}' ability #{nth}", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        trigger,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

const NIJI_LIVES: [&str; 6] = [
    "PL!N-bp1-025-L",
    "PL!N-bp1-026-L",
    "PL!N-bp1-027-L",
    "PL!N-bp1-028-L",
    "PL!N-bp1-029-L",
    "PL!N-sd1-025-SD",
];

#[test]
fn pl_n_bp4_028_l_four_distinct_waitroom_lives_grant_one_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let swc = game.id("PL!N-bp4-028-L");
    game.state.player1.live_card_zone.cards.push(swc);
    for no in NIJI_LIVES.iter().take(4) {
        game.state.player1.waitroom.cards.push(game.id(no));
    }
    fire_trigger(&mut game, swc, AbilityTrigger::LiveStart, "ライブ開始時");
    assert_eq!(
        game.state.mods.get_score_modifier(swc),
        1,
        "4 distinct-name 虹ヶ咲 lives → +1"
    );
}

#[test]
fn pl_n_bp4_028_l_six_distinct_waitroom_lives_grant_two_score_instead() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let swc = game.id("PL!N-bp4-028-L");
    game.state.player1.live_card_zone.cards.push(swc);
    for no in NIJI_LIVES {
        game.state.player1.waitroom.cards.push(game.id(no));
    }
    fire_trigger(&mut game, swc, AbilityTrigger::LiveStart, "ライブ開始時");
    assert_eq!(
        game.state.mods.get_score_modifier(swc),
        2,
        "6 distinct-name 虹ヶ咲 lives → 代わりに +2"
    );
}

#[test]
fn pl_n_bp4_028_l_duplicate_names_or_three_distinct_lives_grant_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let swc = game.id("PL!N-bp4-028-L");
    game.state.player1.live_card_zone.cards.push(swc);
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id(NIJI_LIVES[0]));
    for _ in 1..6 {
        let copy = game.new_id(NIJI_LIVES[0]);
        game.state.player1.waitroom.cards.push(copy);
    }
    fire_trigger(&mut game, swc, AbilityTrigger::LiveStart, "ライブ開始時");
    assert_eq!(
        game.state.mods.get_score_modifier(swc),
        0,
        "6 same-name lives ≠ カード名の異なる4枚 → nothing"
    );
    game.state.player1.waitroom.cards.clear();
    for no in NIJI_LIVES.iter().take(3) {
        game.state.player1.waitroom.cards.push(game.id(no));
    }
    fire_trigger(&mut game, swc, AbilityTrigger::LiveStart, "ライブ開始時");
    assert_eq!(
        game.state.mods.get_score_modifier(swc),
        0,
        "3 distinct < 4 → nothing"
    );
}
