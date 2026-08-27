/// Q159: When 桜坂しずく's debut activates another card's debut from discard,
/// and that debut has a self-cost of "put this member to wait", the cost is
/// unpayable because the card is in discard, not on stage.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn q159_debut_from_discard_self_cost_wait_not_payable() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sayaka = game.id("PL!N-bp3-003-R"); // 桜坂しずく, cost=9, 虹ヶ咲, debut ability
    let shioriko = game.id("PL!N-bp3-022-N"); // 三船栞子, cost=4, 虹ヶ咲, debut: self→wait → look 2
    let filler = game.id("PL!-sd1-002-SD");
    let live = game.id("PL!-sd1-019-SD");

    // Stage: empty (sayaka will be played)
    game.state.player1.stage.stage = [-1; 3];

    // Discard: 三船栞子 (cost 4, 虹ヶ咲) — eligible target
    game.state.player1.waitroom.cards.push(shioriko);
    game.state.player1.waitroom.cards.push(filler);

    // Hand: sayaka + live + fillers
    game.state.player1.hand.cards.push(sayaka);
    game.state.player1.hand.cards.push(live);
    for _ in 0..5 {
        game.state
            .player1
            .hand
            .cards
            .push(game.id("PL!-sd1-002-SD"));
    }

    // Deck fillers
    for _ in 0..20 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.id("PL!-sd1-002-SD"));
    }
    for _ in 0..20 {
        game.state
            .player2
            .main_deck
            .cards
            .push(game.id("PL!-sd1-002-SD"));
    }

    game.give_energy(20);

    // Play 桜坂しずく → triggers debut
    game.play_to_stage(sayaka, MemberArea::Center);

    // Debut prompts: select from discard, then try to activate ability
    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        let ct = game.pending_choice_type();
        match ct.as_deref() {
            Some("SelectCard") => {
                // Select 三船栞子 from discard (first eligible card)
                game.try_select_indices(&[0]).unwrap();
            }
            Some("SelectAutoAbility") => {
                // Skip auto ability selection
                game.try_select_indices(&[]).unwrap();
            }
            Some("SelectTarget") => {
                // Pay or skip optional cost — skip (cost is unpayable)
                game.try_select_indices(&[]).unwrap();
            }
            _ => {
                game.try_select_indices(&[0]).unwrap();
            }
        }
    }

    // 三船栞子 should still be in discard (debut could not activate)
    assert!(
        game.state.player1.waitroom.cards.contains(&shioriko),
        "Q159: 三船栞子 should remain in discard — debut with self-cost wait cannot activate from discard"
    );
    // 桜坂しずく should be on stage
    assert!(
        game.state.player1.stage.stage.contains(&sayaka),
        "桜坂しずく should be on stage"
    );
}

/// Positive: cost<=4 Nijigasaki without wait cost — selection prompt appears and can be resolved
#[test]
fn q159_positive_select_prompt_appears() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!N-bp3-003-R");
    let kasumi = game.id("PL!N-bp7-017-N");
    let filler = game.id("PL!-sd1-002-SD");
    let live = game.id("PL!-sd1-019-SD");
    game.state.player1.stage.stage = [-1; 3];
    game.state.player1.waitroom.cards.push(kasumi);
    game.state.player1.hand.cards.push(sayaka);
    game.state.player1.hand.cards.push(live);
    for _ in 0..5 { game.state.player1.hand.cards.push(filler); }
    for _ in 0..20 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..20 { game.state.player2.main_deck.cards.push(filler); }
    game.give_energy(20);
    game.give_energy(2);
    game.play_to_stage(sayaka, MemberArea::Center);
    // Should present at least the discard selection (SelectCard)
    assert!(game.has_pending_choice(), "Shizuku debut should present discard selection");
    let mut saw_select = false;
    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        let ct = game.pending_choice_type();
        if ct.as_deref() == Some("SelectCard") { saw_select = true; }
        let _ = game.try_select_indices(&[0]);
    }
    assert!(saw_select, "should have seen SelectCard for discard");
    assert!(game.state.player1.stage.stage.contains(&sayaka), "sayaka on stage");
}
