/// You (渡辺 曜 PL!S-bp2-005-R+) — Debut look + select from deck
///
/// Ab#0 (登場): Optional discard from hand: look at top 7, select up to 3 heart02/04/05
///   member cards, add to hand, discard rest.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn resolve_all_up_to(game: &mut TestGame, max: usize) {
    for _ in 0..max {
        if !game.has_pending_choice() {
            return;
        }
        game.select_indices(&[]);
    }
    panic!("resolve_all_up_to: exceeded {} iters", max);
}

#[test]
fn you_q124_blade_heart_excluded_base_heart_included() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let qualifying = game.id("PL!S-sd1-001-SD");
    let blade_only = game.id("PL!SP-sd1-001-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.extend(vec![
        qualifying, blade_only, filler, filler, filler, filler, filler,
    ]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    assert!(game.has_pending_choice(), "cost choice must be offered (filler in hand)");
    game.select_indices(&[0]);
    resolve_all_up_to(&mut game, 30);
    assert!(!game.has_pending_choice(), "Ability should have ended");
    assert!(
        !game.state.player1.hand.cards.contains(&blade_only),
        "Blade-only NOT in hand"
    );
}

#[test]
fn you_ability_ends_and_discard_only_grows_at_end() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.extend(vec![
        filler, filler, filler, filler, filler, filler, filler, filler,
    ]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let initial_discard = game.state.player1.waitroom.cards.len();
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    assert!(game.has_pending_choice(), "cost choice must be offered");
    game.select_indices(&[0]);
    resolve_all_up_to(&mut game, 30);
    assert_eq!(
        game.state.player1.waitroom.cards.len() - initial_discard,
        8,
        "Expected 8 in discard (1 cost + 7 looked-at)"
    );
}

#[test]
fn you_ability_select_1_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let qualifying = game.id("PL!S-sd1-001-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.extend(vec![
        qualifying, filler, filler, filler, filler, filler, filler, filler,
    ]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    assert!(game.has_pending_choice(), "Should have cost choice");
    game.select_indices(&[0]);
    assert!(
        game.has_pending_choice(),
        "Should have look_and_select choice"
    );
    game.select_indices(&[0]);
    resolve_all_up_to(&mut game, 30);
    assert!(
        game.state.player1.hand.cards.contains(&qualifying),
        "Qualifying card in hand"
    );
}

/// Taking one of seven still mills the other six — the 「残り」 clause.
///
/// 「…3枚まで公開して手札に加えてもよい。残りを控え室に置く。」 The look is 7
/// cards; only those passing `card_type: member_card` + `heart_colors:
/// [heart02, heart04, heart05]` are offerable. Everything else is 「残り」 and
/// belongs in the waitroom — including cards that WERE offerable but were not
/// taken.
///
/// This is the case that pins the 「残り」 clause for a partly-eligible look.
/// `you_ability_ends_and_discard_only_grows_at_end` takes nothing, so there
/// 「残り」 and "all of it" coincide and the clause cannot be seen; every other
/// test here asserts the hand only. This one looks at the waitroom *after*
/// taking a card.
///
/// # The defect this pins (fixed 2026-10-04)
///
/// One card reached hand, the waitroom grew by 1 (the optional cost only), and
/// the six excluded cards were left in `gs.looked_at_cards` — in no zone, yet
/// still readable as shared state by the next look-based card and by the
/// opponent.
///
/// Two engine faults, both required:
///
/// 1. `handle_select_cards_looked_at` gated its 「select up to N more」 re-prompt on
///    `!remaining_cards.is_empty()` (`effects/move/mod.rs`). A 「残り」 card is by
///    definition one the filter REJECTS, so once the eligible set ran out the
///    guard still passed and built a **zero-option** prompt.
/// 2. `handle_looked_at_selection` then fell through to `finalize_choice`, which
///    clears `awaiting.choice` unless `should_preserve` holds — and it preserves
///    only a looked_at choice with queued sequential actions. The zero-option
///    prompt was destroyed, nothing was ever milled, and the six cards stayed in
///    the looked-at buffer.
///
/// Fixing either alone leaves the bug: (1) alone still stages a prompt that (2)
/// wipes, and (2) alone still offers a prompt with nothing in it. (1) now tests
/// reachability against the ELIGIBLE set — which also bounds the offered pick
/// count honestly — and (2) returns early when the mover left a prompt standing.
#[test]
fn you_taking_one_of_seven_discards_the_remaining_six() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let qualifying = game.id("PL!S-sd1-001-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.extend(vec![
        qualifying, filler, filler, filler, filler, filler, filler,
    ]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let deck_before = game.state.player1.main_deck.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    assert!(game.has_pending_choice(), "Should have cost choice");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "Should have look_and_select choice");
    game.select_indices(&[0]); // take exactly ONE
    resolve_all_up_to(&mut game, 30);

    // The take itself works.
    assert_eq!(
        game.state.player1.hand.cards.iter().filter(|&&c| c == qualifying).count(),
        1,
        "the one qualifying card was taken to hand"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 7,
        "the look window is exactly 7, so a sentinel card below it is untouched"
    );

    // The only eligible card is gone, so the ability ends here without a second
    // prompt — there is nothing left the filter would let the player take.
    assert!(
        !game.has_pending_choice(),
        "exhausting the eligible set must end the ability, not re-prompt with an \
         empty option list"
    );

    // 「残りを控え室に置く」 covers the whole look of 7: the 6 excluded fillers
    // plus the optional cost card discarded at activation.
    assert_eq!(
        game.state.player1.waitroom.cards.len() - waitroom_before,
        7,
        "1 optional cost + 「残りを控え室に置く」 x6"
    );
    assert!(
        game.state.looked_at_cards.is_empty(),
        "no card may be left in the looked-at buffer: it is in no zone, yet still \
         readable as shared state by the next look-based card and by the opponent"
    );
}

#[test]
fn you_ability_select_multiple_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let q1 = game.id("PL!S-sd1-001-SD");
    let q2 = game.id("PL!S-sd1-002-SD");
    let q3 = game.id("PL!S-sd1-003-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state
        .player1
        .main_deck
        .cards
        .extend(vec![q1, q2, q3, filler, filler, filler, filler, filler]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    assert!(game.has_pending_choice(), "cost choice must be offered");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "select q1 must be offered");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "select q2 must be offered");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "select q3 must be offered");
    game.select_indices(&[0]);
    // Observed: picking the 3rd of 3 eligible cards reaches the max cap and
    // the ability auto-finalizes — no skip prompt is offered.
    assert!(
        !game.has_pending_choice(),
        "reaching max (3) picks must auto-finalize without prompting"
    );
    resolve_all_up_to(&mut game, 30);
    assert!(game.state.player1.hand.cards.contains(&q1), "Q1 in hand");
    assert!(game.state.player1.hand.cards.contains(&q2), "Q2 in hand");
    assert!(game.state.player1.hand.cards.contains(&q3), "Q3 in hand");
}

#[test]
fn you_ability_user_scenario_select_one_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let q1 = game.id("PL!S-sd1-001-SD");
    let q2 = game.id("PL!S-sd1-002-SD");
    let q3 = game.id("PL!S-sd1-003-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state
        .player1
        .main_deck
        .cards
        .extend(vec![q1, q2, q3, filler, filler, filler, filler, filler]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    assert!(game.has_pending_choice(), "cost choice must be offered");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "look_and_select must be offered");
    game.select_indices(&[0]);
    resolve_all_up_to(&mut game, 30);
    assert!(game.state.player1.hand.cards.contains(&q1), "Q1 in hand");
    assert!(
        !game.state.player1.hand.cards.contains(&q2),
        "Q2 NOT in hand"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&q3),
        "Q3 NOT in hand"
    );
}

#[test]
fn you_q124_two_plays_both_reject_blade_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let blade_only = game.id("PL!SP-sd1-001-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state
        .player1
        .main_deck
        .cards
        .extend(vec![filler, filler, filler, filler, filler, filler, filler]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    assert!(game.has_pending_choice(), "cost choice must be offered");
    game.select_indices(&[0]);
    resolve_all_up_to(&mut game, 30);
    assert!(
        !game.state.player1.hand.cards.contains(&blade_only),
        "Blade-only NOT in hand"
    );
}

#[test]
fn you_select_2_then_ends() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let q1 = game.id("PL!S-sd1-001-SD");
    let q2 = game.id("PL!S-sd1-002-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state
        .player1
        .main_deck
        .cards
        .extend(vec![q1, q2, filler, filler, filler, filler, filler]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    assert!(game.has_pending_choice(), "cost choice must be offered");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "select q1 must be offered");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "select q2 must be offered");
    game.select_indices(&[0]);
    // Observed: after the last eligible card is picked (2 < max 3) the
    // ability auto-finalizes when no eligible cards remain — no skip prompt.
    assert!(
        !game.has_pending_choice(),
        "exhausting eligible cards must auto-finalize without prompting"
    );
    resolve_all_up_to(&mut game, 30);
    assert!(game.state.player1.hand.cards.contains(&q1), "Q1 in hand");
    assert!(game.state.player1.hand.cards.contains(&q2), "Q2 in hand");
}

#[test]
fn you_select_3_then_ends() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let q1 = game.id("PL!S-sd1-001-SD");
    let q2 = game.id("PL!S-sd1-002-SD");
    let q3 = game.id("PL!S-sd1-003-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state
        .player1
        .main_deck
        .cards
        .extend(vec![q1, q2, q3, filler, filler, filler, filler]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    assert!(game.has_pending_choice(), "cost choice must be offered");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "select q1 must be offered");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "select q2 must be offered");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "select q3 must be offered");
    game.select_indices(&[0]);
    // Observed: picking the 3rd of 3 eligible cards reaches the max cap and
    // the ability auto-finalizes — no skip prompt is offered.
    assert!(
        !game.has_pending_choice(),
        "reaching max (3) picks must auto-finalize without prompting"
    );
    resolve_all_up_to(&mut game, 30);
    assert!(game.state.player1.hand.cards.contains(&q1), "Q1 in hand");
    assert!(game.state.player1.hand.cards.contains(&q2), "Q2 in hand");
    assert!(game.state.player1.hand.cards.contains(&q3), "Q3 in hand");
}
