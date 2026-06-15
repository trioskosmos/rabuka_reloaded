use crate::helpers::*;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

/// Q256: When Maki (PL!-sd1-006-SD) reveals 錯覚CROSSROADS (PL!-bp6-024-L)
/// via her debut effect and places it in the success zone, can 錯覚CROSSROADS's
/// own constant replacement ability fire to place a μ's live card from waitroom instead?
/// Answer: Yes.
#[test]
fn q256_maki_reveal_crossroads_replacement() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let maki = game.id("PL!-sd1-006-SD");
    let crossroads = game.id("PL!-bp6-024-L");
    let muse_live = game.id("PL!-bp3-019-L"); // 僕らのLIVE 君とのLIFE
    let filler_card = game.id("PL!-sd1-010-SD");

    // Put Maki in hand, 錯覚CROSSROADS in hand (to reveal), a μ's live in waitroom
    game.add_to_hand(maki);
    game.add_to_hand(crossroads);
    game.add_to_discard(muse_live);

    // Put a filler card in success zone (will be returned to hand by step a)
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(filler_card);

    // Give energy to play Maki (cost 9)
    game.give_energy(9);

    // Play Maki to stage → debut triggers
    game.play_to_stage(maki, MemberArea::Center);

    // Step 1: Optional reveal cost — choose 錯覚CROSSROADS from hand
    // The reveal prompts a choice to select a live card from hand.
    // crossroads was pushed after maki, so it's at index 0 in hand.
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Step 2: Effect step a — choose a card from success zone to return to hand
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Step 3: Effect step b — revealed card should trigger replacement choice
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Verifications:
    // 1. 錯覚CROSSROADS should be in waitroom (replaced from success zone placement)
    assert!(
        game.state.player1.waitroom.cards.contains(&crossroads),
        "錯覚CROSSROADS should be in waitroom (replacement triggered)"
    );

    // 2. The μ's live card should be in success zone
    assert!(
        game.state
            .player1
            .success_live_card_zone
            .cards
            .contains(&muse_live),
        "μ's live card should be in success zone (replacement target)"
    );

    // 3. The filler card that was in success zone should now be in hand
    assert!(
        game.state.player1.hand.cards.contains(&filler_card),
        "Original success zone card should be in hand"
    );

    // 4. Maki should be on stage in Center
    assert_eq!(
        game.state.player1.stage.stage[1], maki,
        "Maki should be on stage"
    );
}
