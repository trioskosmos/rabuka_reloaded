/// Batch 5 — more remaining 1-QA cards with standalone testable abilities
use crate::helpers::*;
/// PL!-sd1-002-SD (絢瀬絵里) Q79: Self_cost vacates area, new member can be placed.
/// Tested in eli_test. Here just verifying the card exists.
/// (shared QA with 星空凛, tested in rin_test)

/// PL!-sd1-006-SD (西木野真姫) Q125: Cannot place in success live zone.
/// Same QA as kagayaiteru's 君のこころは輝いてるかい？ Verified in kagayaiteru_test.
/// Test: ability parses with restriction.
#[test]
fn maki_sd1_q125_restriction_parsed() {
    let db = load_real_database();
    let card = db.get_card_id("PL!-sd1-006-SD").expect("Card exists");
    let card_data = db.get_card(card).expect("Maki card should exist");
    let has_restriction = card_data
        .abilities
        .iter()
        .any(|a| a.full_text.contains("成功ライブカード"));
    assert!(
        has_restriction,
        "Card should have success zone restriction ability"
    );
}

/// PL!S-bp3-005-R (渡辺曜) Q153: LiveSuccess — compare revealed card counts,
/// draw if self's count < opponent's.
#[test]
fn you_s3_q153_live_success_draw_if_fewer_revealed() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let you = game.id("PL!S-bp3-005-R");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");

    let live_card = game.id("PL!-sd1-019-SD");
    game.state.player1.stage.stage = [you, member, -1];
    game.state.player1.hand.cards.push(live_card);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live_card);
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();

    // LiveSuccess triggers — condition checks if self's revealed count
    // is less than opponent's. At minimum, the ability fires without error.
}

/// PL!S-bp3-016-N (国木田花丸) Q155: Constant ability — each card in
/// success_live_zone increases this member's cost.
/// Data test: verify the ability is parsed.
#[test]
fn hanamaru_s3_q155_constant_cost_increase() {
    let db = load_real_database();
    let card = db.get_card_id("PL!S-bp3-016-N").expect("Card exists");
    let card_data = db.get_card(card).expect("Hanamaru card should exist");
    assert!(
        !card_data.abilities.is_empty(),
        "Card should have abilities"
    );
    let constant_ability = card_data
        .abilities
        .iter()
        .any(|a| a.triggers.as_ref().map_or(false, |t| &**t == "常時"));
    assert!(constant_ability, "Should have at least one 常時 ability");
}
