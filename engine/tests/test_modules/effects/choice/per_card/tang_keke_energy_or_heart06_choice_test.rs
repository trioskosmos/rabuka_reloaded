/// Tests for Tang Ke Ke (PL!SP-pb2-002) activate ability choice pattern.
/// The ability has:
///   Cost: discard 1 Liella! card from hand
///   Effect: choose 1 from (energy from energy deck / heart06 on a Liella! member)
///   If the discarded card is a Liella! member WITHOUT blade heart → choose 1+ instead
///
/// Parser fix: _try_kore_niyori_result previously matched before _try_choice,
/// producing "conditional_on_result" + "select" (area_select → position prompt)
/// instead of the correct "choice" + options with alternative_condition.
use crate::helpers::*;

/// Helper: advance past the cost prompt (select card from hand to discard)
fn pay_discard_cost(game: &mut TestGame) {
    assert!(
        game.has_pending_choice(),
        "Should prompt to select card to discard"
    );
    game.select_indices(&[0]);
}

/// Helper: verify the choice prompt appears and select the energy option (index 0)
fn select_energy_option(game: &mut TestGame) -> usize {
    assert!(game.has_pending_choice(), "Choice should appear");
    let energy_before = game.state.player1.energy_zone.cards.len();
    game.select_option(0);
    energy_before
}

/// Basic setup: Tang Ke Ke on stage, some energy, energy deck has cards
fn setup_keke(game: &mut TestGame) -> i16 {
    let keke = game.id("PL!SP-pb2-002-R");
    let energy = game.id("LL-E-001-SD");

    game.state.player1.stage.stage[0] = keke;
    game.give_energy(5);
    for _ in 0..5 {
        game.state.player1.energy_deck.cards.push(energy);
    }
    keke
}

#[test]
fn tang_keke_discard_member_with_blade_heart_choose_one() {
    // Discard a Liella! member WITH blade heart → condition NOT met → count=1
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let keke = setup_keke(&mut game);
    let liella_with_bh = game.id("PL!SP-PR-003-PR"); // 澁谷かのん, b_heart03=1

    game.state.player1.hand.cards.push(liella_with_bh);
    game.activate_ability(keke);

    pay_discard_cost(&mut game);
    let energy_before = select_energy_option(&mut game);

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        energy_before + 1,
        "「colspan1.png|colspan1.png|自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く」: \
         exactly one energy card was placed"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&liella_with_bh),
        "Discarded card should be in waitroom"
    );
}

#[test]
fn tang_keke_discard_member_without_blade_heart_choose_any_number() {
    // Discard a Liella! member WITHOUT blade heart → condition MET → count=any_number
    // Player can select 1+ options. After selecting energy, re-prompt with remaining.
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let keke = setup_keke(&mut game);
    let liella_no_bh = game.id("PL!SP-PR-004-PR"); // 唐可可, no blade heart
                                                   // A Liella! member on stage for the heart06 option (not the activating member)
    let target_member = game.id("PL!SP-PR-003-PR");

    game.state.player1.hand.cards.push(liella_no_bh);
    // Place a different Liella! member on stage for the heart06 target
    game.state.player1.stage.stage[1] = target_member;
    game.activate_ability(keke);

    pay_discard_cost(&mut game);

    // Choice appears — select energy (option 0)
    assert!(game.has_pending_choice(), "Choice should appear");
    let energy_before = game.state.player1.energy_zone.cards.len();
    game.select_option(0);

    // any_number: engine should re-prompt with remaining option (heart06)
    assert!(
        game.has_pending_choice(),
        "Re-prompt should appear for remaining options (any_number)"
    );
    // Select the heart06 option — verify heart06 was gained on the target member
    game.select_option(0);

    assert_eq!(
        game.state.player1.energy_zone.cards.len() - energy_before,
        1,
        "Exactly one energy placed from the energy deck (the option selected \
         first); a band of > here would pass if the engine placed it twice"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&liella_no_bh),
        "Discarded card should be in waitroom"
    );
    // Heart06 gain is a modifier — just verify no crash / no infinite loop
    assert!(
        !game.has_pending_choice(),
        "No more choices after selecting both options"
    );
}

#[test]
fn tang_keke_discard_non_member_liella_card_choose_one() {
    // Discard a Liella! LIVE card (not a member) → card_type condition fails → count=1
    // PL!SP-pb2-002 itself is a Liella! live card — use it as discard fodder
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let keke = setup_keke(&mut game);
    // Use a second copy of the same card as discard fodder (it's a Liella! live card)
    let liella_live = game.id("PL!SP-pb2-002-R");

    game.state.player1.hand.cards.push(liella_live);
    game.activate_ability(keke);

    pay_discard_cost(&mut game);
    let energy_before = select_energy_option(&mut game);

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        energy_before + 1,
        "「colspan1.png|colspan1.png|自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く」: \
         exactly one energy card was placed"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&liella_live),
        "Discarded live card should be in waitroom"
    );
}

#[test]
fn tang_keke_use_limit_blocks_second_activation() {
    // The ability has use_limit=1. After using it once, cannot activate again this turn.
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let keke = setup_keke(&mut game);
    let liella_with_bh = game.id("PL!SP-PR-003-PR");

    // First activation
    game.state.player1.hand.cards.push(liella_with_bh);
    game.state.player1.hand.cards.push(liella_with_bh); // second copy for 2nd attempt
    game.activate_ability(keke);

    pay_discard_cost(&mut game);
    let _ = select_energy_option(&mut game);

    // The first use must actually be recorded, otherwise a broken first
    // activation would make this pass for the wrong reason.
    assert!(
        game.state
            .turn_limited_abilities_used
            .contains_key(&(keke, 0, game.state.turn_number)),
        "the first activation must have recorded its use, else the second \
         refusal proves nothing about the limit"
    );
    let energy_cards_before = game.state.player1.energy_zone.cards.len();

    // Second activation should be blocked by use_limit
    let result = game.try_activate_ability(keke);
    assert!(
        result.is_err(),
        "Second activation should fail: use_limit reached"
    );
    // The refusal must cost nothing: no second discard prompt, no energy, and
    // the use-limit table untouched by the refused attempt.
    assert!(
        !game.has_pending_choice(),
        "a use-limit refusal must not open the discard cost prompt"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        energy_cards_before,
        "a refused activation must not place another energy card"
    );
    assert!(
        game.state.player1.hand.cards.contains(&liella_with_bh),
        "the cost was paid only once; the second copy is still in hand"
    );
}

#[test]
fn tang_keke_no_liella_in_hand_cost_cannot_pay() {
    // If there are no Liella! cards in hand, the cost cannot be paid.
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let keke = setup_keke(&mut game);

    // No card in hand — cost cannot be paid.
    game.state.player1.hand.cards.clear();

    // The engine refuses the cost at resolution time (debug trace:
    // "Not enough matching cards in hand to pay cost. Needs 1, has 0")
    // and reports it via logs, not via Err — so assert the observable
    // behavior: no cost prompt, nothing discarded, no effect happens.
    let energy_before = game.state.player1.energy_zone.cards.len();
    let _ = game.try_activate_ability(keke);
    assert!(
        !game.has_pending_choice(),
        "unpayable cost must not open a discard prompt"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        energy_before,
        "no effect may resolve when the cost was not paid"
    );
    assert!(
        game.state.player1.hand.cards.is_empty(),
        "nothing to discard, hand stays empty"
    );
}

#[test]
fn tang_keke_nonmatching_hand_refuses_cost_no_prompt() {
    // Hand is non-empty but contains NO Liella! cards → filter-aware
    // validation must refuse the activation (Rule 9.4.2.3/Q56), not
    // open a discard prompt and fizzle later.
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let keke = setup_keke(&mut game);

    // Aqours member (not Liella!) — matches zone count but not cost filter.
    let aqours = game.id("PL!S-sd1-010-SD");
    game.state.player1.hand.cards.push(aqours);

    let energy_before = game.state.player1.energy_zone.cards.len();
    let _ = game.try_activate_ability(keke);
    assert!(
        !game.has_pending_choice(),
        "unpayable filtered cost must not open a discard prompt"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        energy_before,
        "no effect may resolve when the cost was not paid"
    );
    assert!(
        game.state.player1.hand.cards.contains(&aqours),
        "non-matching card must stay in hand"
    );
    assert!(
        game.state.player1.waitroom.cards.is_empty(),
        "nothing may be discarded when cost is refused"
    );
}
