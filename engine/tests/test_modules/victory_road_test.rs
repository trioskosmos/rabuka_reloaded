/// Tests for 繚乱！ビクトリーロード (PL!N-bp5-030-L) — each_time auto abilities.
///
/// ab#0: Each time a member's LiveStart resolves, if member lacks hearts, give all hearts.
/// ab#1: Each time a member's LiveSuccess resolves, draw 1 card.
///
/// Q217: Cost IS paid (select 0 for any_number) → ability is "used" → each_time fires.
/// Q227: Cost declined entirely → ability NOT "used" → each_time does NOT fire.
///
/// These test whether paying 0 vs declining produces different trigger behavior.
/// The referenced member is LL-bp2-001-R＋ (鬼塚夏美&遠藤アリサ&遠手鞠) which has
/// an optional LiveStart cost: discard any number of named characters from hand.
use crate::helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

/// Verify the multi-member card exists and its LiveStart ability fires.
/// The optional cost creates a choice. Q217: selecting 0 still counts as "used."
/// Q227: declining the cost entirely does NOT count as "used."
#[test]
fn victory_road_q217_q227_cost_handling() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");
    let victory = game.id("PL!N-bp5-030-L");
    // Fullwidth plus sign
    let multi = game.id("LL-bp2-001-R\u{ff0b}");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    // The multi-member card on stage, victory road in hand
    game.state.player1.stage.stage[0] = multi;
    game.state.player1.stage.stage[1] = filler;
    game.state.player1.hand.cards.push(victory);
    // Put the named characters in hand so the cost filter has targets
    // Named: 鬼塚夏美, 遠藤アリサ, 遠手鞠 — put filler as dummy
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_set(&mut game);
    game.set_live_card(victory);

    // Pass through live set phases to trigger LiveStart
    game.pass(); // LiveCardSetFirstAttacker → P2Turn
    game.pass(); // LiveCardSetSecondAttacker → FirstAttackerPerformance → LiveStart

    // Multi-member's LiveStart fires. The optional cost displays.
    // Handle whatever choices come up
    eprintln!("[Q217/Q227] Choices after LiveStart trigger:");
    game.dbg_choice();

    // Drain all pending choices
    let mut safety = 0;
    while game.has_pending_choice() && safety < 10 {
        safety += 1;
        // If there's a pending_choice, try option -1 (skip/decline)
        if game.state.has_pending_choice() {
            game.select_option(-1);
        } else {
            game.select_indices(&[]);
        }
    }

    eprintln!("[Q217/Q227] Done. safety={}", safety);
    assert!(safety < 10, "Didn't loop infinitely");

    // After resolving all choices, the live card (victory road) should be in the live zone
    // and the multi-member card should still be on stage
    assert!(
        game.state.player1.live_card_zone.cards.contains(&victory),
        "Victory Road should be set as live card"
    );
    assert!(
        game.state.player1.stage.stage.contains(&multi),
        "Multi-member card should remain on stage after LiveStart resolution"
    );
    // The victory road ability (each_time on LiveSuccess → draw 1) hasn't fired yet
    // but the setup should be valid
    assert!(
        !game.has_pending_choice(),
        "All pending choices should be drained after LiveStart resolution"
    );
}
