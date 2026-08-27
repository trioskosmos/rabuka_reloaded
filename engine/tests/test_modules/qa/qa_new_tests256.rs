use crate::helpers::*;
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
    let filler_live = game.new_id("PL!SP-sd1-023-SD"); // WE WILL!! (live card)

    // Put Maki in hand, 錯覚CROSSROADS in hand (to reveal), a μ's live in waitroom
    game.add_to_hand(maki);
    game.add_to_hand(crossroads);
    game.add_to_discard(muse_live);

    // Put a filler card in success zone (will be returned to hand by step a)
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(filler_live);
    // ... add filler_live to hand for the "returned to hand" check
    game.add_to_hand(filler_live);

    // Give energy to play Maki (cost 9)
    game.give_energy(9);

    // Play Maki to stage → debut triggers
    game.play_to_stage(maki, MemberArea::Center);

    // Step 1: Optional reveal cost — observed: SelectCard zone=hand
    // count=1 allow_skip=true; crossroads is at hand index 0.
    assert!(
        game.has_pending_choice(),
        "reveal-cost prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard reveal-cost prompt"
    );
    game.select_indices(&[0]);

    // Effect step a returns the success-zone card to hand AUTOMATICALLY
    // (single candidate, can_skip=false -> no prompt), so the next thing
    // pending is already the step-b replacement choice.
    // Observed: SelectCard zone=discard count=1 allow_skip=true
    // group=μ's live_card — "Choose a live card from discard to place in
    // your success zone (or skip to place the original card)".
    assert!(
        game.has_pending_choice(),
        "replacement-choice prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard replacement prompt"
    );
    game.select_indices(&[0]);

    // Ability fully resolved after the replacement pick.
    assert!(
        !game.has_pending_choice(),
        "no further prompts after the replacement choice"
    );

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

    // 3. The filler live card that was in success zone should now be in hand
    assert!(
        game.state.player1.hand.cards.contains(&filler_live),
        "Original success zone card should be in hand"
    );

    // 4. Maki should be on stage in Center
    assert_eq!(
        game.state.player1.stage.stage[1], maki,
        "Maki should be on stage"
    );
}
