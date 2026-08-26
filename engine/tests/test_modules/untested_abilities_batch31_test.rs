/// Untested-abilities batch 31 — ライブ成功時 retrievals + 登場 movers.
/// Idioms reused from batch18 (revealed_cards setup), batch30 (fire_trigger),
/// WRITING_TESTS §H (guarded optional-choice dispatch).
///
/// - PL!-bp6-021-L (ライブ成功時): optional cost 『μ's』メンバー1人をステージから
///   控え室に置く → score +1 AND retrieve 『μ's』ライブカード from waitroom.
/// - PL!HS-cl1-009-CL (ライブ成功時): from yell-revealed cards retrieve
///   蓮ノ空 member with cost 4–9 (range boundaries pinned).
/// - PL!HS-bp6-032-L (ライブ成功時): from yell-revealed cards retrieve
///   member with cost ≤ 4.
/// - PL!S-bp5-015-N (登場): mill exactly 10 from deck top to waitroom.
/// - PL!N-bp4-021-N (登場): optionally put 1 waitroom card on deck top.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

/// Answer one pending optional gate ("〜してもよい：…").
/// Handles both shapes: an explicit pay/skip SelectTarget, and a
/// skippable SelectCard (allow_skip=true) where declining = select nothing.
/// Returns false when nothing matched, so callers can assert the prompt.
fn answer_optional(game: &mut TestGame, accept: bool) -> bool {
    match game.get_pending_choice() {
        Choice::SelectTarget { target, .. }
            if target == "conditional_optional"
                || target.starts_with("pay_optional_cost") =>
        {
            // options[0] = skip, options[1] = do it (WRITING_TESTS §H)
            game.select_choice_option(if accept { 1 } else { 0 });
            true
        }
        Choice::SelectCard { allow_skip, .. } => {
            if !accept && *allow_skip {
                game.select_indices(&[]);
                true
            } else {
                false
            }
        }
        _ => {
            eprintln!(
                "[answer_optional] unmatched prompt: {}",
                game.pending_choice_summary()
            );
            false
        }
    }
}

// ====================================================================
// PL!-bp6-021-L — optional μ's-member cost gates score+1 AND retrieval
// ====================================================================

#[test]
fn bp6_021_accept_cost_scores_and_retrieves() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-bp6-021-L");
    let mus_member = game.new_id("PL!-sd1-010-SD"); // 『μ's』 member on stage
    let mus_live = game.id("PL!-sd1-020-SD"); // 『μ's』 live card in waitroom

    { let f = game.new_id("PL!-sd1-010-SD"); fill_decks(&mut game, f); }
    game.state.player1.live_card_zone.cards.push(live);
    game.state.player1.stage.stage[0] = mus_member;
    game.add_to_discard(mus_live);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    // Even with a single cost candidate the optional gate IS offered
    // (observed: SelectTarget pay_optional_cost:skip_optional_cost).
    assert!(
        game.has_pending_choice(),
        "optional μ's-member cost gate must be offered"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget optional-cost gate"
    );
    assert!(answer_optional(&mut game, true), "unexpected prompt shape");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        1,
        "cost accepted -> score +1"
    );
    assert!(
        game.state.player1.hand.cards.contains(&mus_live),
        "cost accepted -> μ's live card retrieved to hand"
    );
    assert!(
        !game.state.player1.stage.stage.contains(&mus_member),
        "cost accepted -> the μ's member left the stage"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&mus_member),
        "cost accepted -> the μ's member went to the waitroom"
    );
}

#[test]
fn bp6_021_skip_cost_no_score_no_retrieval() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-bp6-021-L");
    // TWO μ's members on stage: multiple candidates force a real prompt
    // instead of the single-target auto-pay.
    let m1 = game.new_id("PL!-sd1-010-SD");
    let m2 = game.new_id("PL!-sd1-010-SD");
    let mus_live = game.id("PL!-sd1-020-SD");

    { let f = game.new_id("PL!-sd1-010-SD"); fill_decks(&mut game, f); }
    game.state.player1.live_card_zone.cards.push(live);
    game.state.player1.stage.stage[0] = m1;
    game.state.player1.stage.stage[2] = m2;
    game.add_to_discard(mus_live);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(game.has_pending_choice(), "multiple candidates -> prompt expected");
    assert!(answer_optional(&mut game, false), "expected an optional-gate prompt");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "cost declined -> no score"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&mus_live),
        "cost declined -> no retrieval"
    );
    assert!(
        game.state.player1.stage.stage.contains(&m1)
            && game.state.player1.stage.stage.contains(&m2),
        "cost declined -> both members stay on stage"
    );
}

// ====================================================================
// PL!HS-cl1-009-CL — revealed-cards retrieval, 蓮ノ空 member cost 4..=9
// ====================================================================

#[test]
fn cl1_009_retrieves_cost_four_boundary() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-cl1-009-CL");
    let target = game.id("PL!HS-bp1-012-PR"); // 乙宗 梢, cost 4 (lower boundary)

    game.state.player1.live_card_zone.cards.push(live);
    game.state.revealed_cards.push(target);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    // Single valid candidate auto-resolves — no selection prompt.
    assert!(
        !game.has_pending_choice(),
        "single cost-4 candidate must auto-resolve without prompting"
    );

    assert!(
        game.state.player1.hand.cards.contains(&target),
        "cost-4 蓮ノ空 member (lower boundary) retrieved"
    );
}

#[test]
fn cl1_009_retrieves_cost_nine_boundary() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-cl1-009-CL");
    let target = game.id("PL!HS-pb1-015-R"); // セラス, cost 9 (upper boundary)

    game.state.player1.live_card_zone.cards.push(live);
    game.state.revealed_cards.push(target);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    // Single valid candidate auto-resolves — no selection prompt.
    assert!(
        !game.has_pending_choice(),
        "single cost-9 candidate must auto-resolve without prompting"
    );

    assert!(
        game.state.player1.hand.cards.contains(&target),
        "cost-9 蓮ノ空 member (upper boundary) retrieved"
    );
}

#[test]
fn cl1_009_cost_ten_outside_range_not_selectable() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-cl1-009-CL");
    let outside = game.id("PL!HS-PR-005-PR"); // 大沢瑠璃乃, cost 10 (> 9)

    game.state.player1.live_card_zone.cards.push(live);
    game.state.revealed_cards.push(outside);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(
        !game.has_pending_choice(),
        "only out-of-range candidates -> no selection prompt"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&outside),
        "cost-10 member stays in revealed pool"
    );
}

#[test]
fn cl1_009_wrong_group_excluded_even_in_range() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-cl1-009-CL");
    // Cost-4 member of the WRONG group (μ's filler).
    let mus = game.new_id("PL!-sd1-010-SD");

    game.state.player1.live_card_zone.cards.push(live);
    game.state.revealed_cards.push(mus);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(
        !game.has_pending_choice(),
        "non-蓮ノ空 candidate -> no selection prompt"
    );
}

// ====================================================================
// PL!HS-bp6-032-L — revealed-cards retrieval, any member cost <= 4
// ====================================================================

#[test]
fn hs_bp6_032_retrieves_low_cost_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-bp6-032-L");
    let target = game.id("PL!HS-bp1-009-R"); // 安養寺 姫芽, cost 4 (== boundary)

    game.state.player1.live_card_zone.cards.push(live);
    game.state.revealed_cards.push(target);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    // Single valid candidate auto-resolves — no selection prompt.
    assert!(
        !game.has_pending_choice(),
        "single cost-4 candidate must auto-resolve without prompting"
    );

    assert!(
        game.state.player1.hand.cards.contains(&target),
        "cost-4 member retrieved to hand"
    );
}

#[test]
fn hs_bp6_032_expensive_members_not_selectable() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-bp6-032-L");
    let expensive = game.id("PL!HS-bp5-001-P"); // 日野下花帆, cost 11

    game.state.player1.live_card_zone.cards.push(live);
    game.state.revealed_cards.push(expensive);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(
        !game.has_pending_choice(),
        "only cost>4 candidates -> no selection prompt"
    );
}

// ====================================================================
// PL!S-bp5-015-N — 登場: mill exactly 10 from deck top
// ====================================================================

#[test]
fn bp5_015_debut_mills_exactly_ten() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = game.id("PL!S-bp5-015-N");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.add_to_hand(nico);
    game.give_energy(12);
    // 12 fillers: 10 get milled, 2 remain so the deck never runs dry mid-effect.
    for _ in 0..12 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.play_to_stage(nico, MemberArea::Center);

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        10,
        "debut mills exactly 10 cards to the waitroom"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        2,
        "deck keeps the 2 unmilled cards"
    );
}

// ====================================================================
// PL!N-bp4-021-N — 登場: optionally put 1 waitroom card on deck top
// ====================================================================

#[test]
fn bp4_021_accept_puts_waitroom_card_on_deck_top() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-bp4-021-N");
    let recycled = game.id("PL!N-sd1-025-SD");
    let other = game.new_id("PL!-sd1-010-SD");

    { let f = game.new_id("PL!-sd1-010-SD"); fill_decks(&mut game, f); }
    game.add_to_hand(kasumi);
    game.add_to_discard(recycled);
    game.add_to_discard(other);
    game.give_energy(12);

    game.play_to_stage(kasumi, MemberArea::Center);

    assert!(game.has_pending_choice(), "optional move must be prompted");
    // Select the first offered waitroom card (= our recycled card via the
    // filtered choice list).
    game.select_indices(&[0]);

    assert_eq!(
        game.state.player1.main_deck.cards.first(),
        Some(&recycled),
        "accepted -> waitroom card sits on deck top"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&recycled),
        "accepted -> card left the waitroom"
    );
}

#[test]
fn bp4_021_skip_leaves_waitroom_intact() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-bp4-021-N");
    let recycled = game.id("PL!N-sd1-025-SD");

    { let f = game.new_id("PL!-sd1-010-SD"); fill_decks(&mut game, f); }
    game.add_to_hand(kasumi);
    game.add_to_discard(recycled);
    // A second waitroom card forces a real selection prompt (a lone
    // candidate would auto-resolve, leaving nothing to decline).
    let other = game.new_id("PL!-sd1-010-SD");
    game.add_to_discard(other);
    game.give_energy(12);

    game.play_to_stage(kasumi, MemberArea::Center);

    assert!(game.has_pending_choice(), "optional move must be prompted");
    game.select_indices(&[]); // decline / select nothing

    assert!(
        game.state.player1.waitroom.cards.contains(&recycled),
        "declined -> card stays in the waitroom"
    );
}
