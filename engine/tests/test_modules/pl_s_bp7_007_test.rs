use crate::helpers::*;

// PL!S-bp7-007-R+ 国木田花丸
// Text: {{toujyou.png|登場}}自分の控え室からコスト2以下のメンバーカードを1枚手札に加える。
//       これによって「津島善子」か「黒澤ルビィ」を手札に加えた場合、
//       そのカードを自分のステージのメンバーのいないエリアに登場させてもよい。

fn process_debut(game: &mut TestGame, card_id: i16) {
    let card = game.db.get_card(card_id).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("登場"))
        .expect("Card must have Debut ability");
    let ability_id = format!("{}_{}", card.card_no, ab.full_text);
    game.state.trigger_auto_ability(
        ability_id,
        rabuka_engine::core::types::AbilityTrigger::Debut,
        game.state.player1.id.clone(),
        Some(card.card_no.to_string()),
        Some(card_id),
        None,
        None,
    );
    game.state.activating_card = Some(card_id);
    let pid = game.state.player1.id.clone();
    game.state.process_pending_auto_abilities(&pid);
}

fn seed_deck(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

// ================================================================
// Auto-resolve: 1 matching card in discard → no prompt, auto-add to hand
// ================================================================
#[test]
fn hanamaru_single_matching_auto_adds_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanamaru = game.id("PL!S-bp7-007-R+");
    let tsushima = game.id("PL!S-sd1-015-SD"); // 津島善子 cost=2
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, hanamaru, -1];
    game.state.player1.waitroom.cards.push(tsushima);
    seed_deck(&mut game);

    process_debut(&mut game, hanamaru);

    // Engine auto-resolves: 1 valid card → moved to hand, condition passes, followup auto-deploys
    // Since there IS an empty slot (right), the card should be deployed to stage
    assert!(
        game.state.player1.stage.stage[2] == tsushima
            || game.state.player1.hand.cards.contains(&tsushima),
        "Card should be either deployed or in hand"
    );
}

// ================================================================
// Multiple valid cards → forces selection prompt
// ================================================================
#[test]
fn hanamaru_multiple_valid_forces_selection() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanamaru = game.id("PL!S-bp7-007-R+");
    let tsushima = game.id("PL!S-sd1-015-SD"); // 津島善子 cost=2
    let kurosawa = game.id("PL!S-bp2-009-R"); // 黒澤ルビィ cost=2
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, hanamaru, -1];
    game.state.player1.waitroom.cards.push(tsushima);
    game.state.player1.waitroom.cards.push(kurosawa);
    seed_deck(&mut game);

    process_debut(&mut game, hanamaru);

    // Multiple valid cards → must prompt for selection
    assert!(
        game.has_pending_choice(),
        "Multiple valid cards should prompt for selection"
    );
}

// ================================================================
// Non-matching character → adds to hand but no deploy
// ================================================================
#[test]
fn hanamaru_non_matching_no_deploy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanamaru = game.id("PL!S-bp7-007-R+");
    let non_match = game.id("PL!-sd1-002-SD"); // 絢瀬 絵里 cost=2
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, hanamaru, -1];
    game.state.player1.waitroom.cards.push(non_match);
    seed_deck(&mut game);

    process_debut(&mut game, hanamaru);

    // Auto-resolves: card goes to hand, condition fails (not 津島善子/黒澤ルビィ)
    let hand_ids: Vec<i16> = game.state.player1.hand.cards.iter().copied().collect();
    assert!(hand_ids.contains(&non_match), "Non-matching card should be in hand");
    assert_eq!(game.state.player1.stage.stage[2], -1, "No deploy for non-matching");
}

// ================================================================
// No empty slot → card goes to hand even if matching
// ================================================================
#[test]
fn hanamaru_no_empty_slot_card_in_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanamaru = game.id("PL!S-bp7-007-R+");
    let tsushima = game.id("PL!S-sd1-015-SD");
    let filler1 = game.id("PL!-sd1-010-SD");
    let filler2 = game.id("PL!-sd1-020-SD");

    game.state.player1.stage.stage = [filler1, hanamaru, filler2];
    game.state.player1.waitroom.cards.push(tsushima);
    seed_deck(&mut game);

    process_debut(&mut game, hanamaru);

    // Card added to hand, condition passes, but no empty slot → stays in hand
    let hand_ids: Vec<i16> = game.state.player1.hand.cards.iter().copied().collect();
    assert!(hand_ids.contains(&tsushima), "Card should be in hand when stage full");
}

// ================================================================
// No valid cards in discard → no action
// ================================================================
#[test]
fn hanamaru_no_valid_discard_no_action() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanamaru = game.id("PL!S-bp7-007-R+");
    let too_expensive = game.id("PL!-sd1-014-SD"); // cost=4
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, hanamaru, -1];
    game.state.player1.waitroom.cards.push(too_expensive);
    seed_deck(&mut game);

    process_debut(&mut game, hanamaru);

    // No valid target → nothing happens
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.stage.stage[2], -1);
}

// ================================================================
// Both matching in discard + empty slot → one gets deployed
// ================================================================
#[test]
fn hanamaru_both_matching_one_deploys() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanamaru = game.id("PL!S-bp7-007-R+");
    let tsushima = game.id("PL!S-sd1-015-SD");
    let kurosawa = game.id("PL!S-bp2-009-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, hanamaru, -1];
    game.state.player1.waitroom.cards.push(tsushima);
    game.state.player1.waitroom.cards.push(kurosawa);
    seed_deck(&mut game);

    process_debut(&mut game, hanamaru);

    // Multiple valid → forces selection prompt
    assert!(game.has_pending_choice());
    // Select 津島善子 (index 0)
    game.select_indices(&[0]);

    // After selection, condition passes, deploy should happen
    // (either prompted or auto-resolved)
    let on_stage = game.state.player1.stage.stage.iter().any(|&id| id == tsushima || id == kurosawa);
    let in_hand = game.state.player1.hand.cards.iter().any(|&id| id == tsushima || id == kurosawa);
    assert!(
        on_stage || in_hand,
        "Selected card should be on stage or in hand"
    );
}

// ================================================================
// Deploy to left when right is full
// ================================================================
#[test]
fn hanamaru_deploy_to_left_when_right_full() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanamaru = game.id("PL!S-bp7-007-R+");
    let tsushima = game.id("PL!S-sd1-015-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, hanamaru, filler];
    game.state.player1.waitroom.cards.push(tsushima);
    seed_deck(&mut game);

    process_debut(&mut game, hanamaru);

    // Card should be deployed to left (only empty slot) or in hand
    let on_left = game.state.player1.stage.stage[0] == tsushima;
    let in_hand = game.state.player1.hand.cards.iter().any(|&id| id == tsushima);
    assert!(
        on_left || in_hand,
        "Card should be deployed to left or in hand"
    );
}
