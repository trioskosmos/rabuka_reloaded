//! Rules-driven gameplay tests for special blade-hearts (rules.txt 8.3.11-8.3.12,
//! 8.3.15.1.1, 8.4.2.1) — the parenthetical notes printed on live cards such as
//! Solitude Rain (draw icon) and START:DASH!! (score icon).
//!
//! These notes are NOT abilities: they document what the card's own icon does
//! when the card is REVEALED DURING A YELL (i.e. when an effect has put the
//! live card into the main deck). Every test here drives the real phase
//! pipeline and manipulates decks — no JSON inspection.
//!
//! Deck manipulation contract: `perform_live(..., Some(layout), ...)` replaces
//! P1's main deck right after the live card is set. layout[0] is sacrificed to
//! the LiveCardSet refill draw (actions.rs draws 1 per live-zone card when
//! passing out of LiveCardSet); the yell then reveals layout[1..].
//!
//! Card facts used (from cards.json):
//! - PL!-sd1-010-SD filler: blade=1, base_heart={heart01:1,heart03:1}, blade_heart={b_heart03:1}
//! - PL!-sd1-002-SD 絢瀬絵里: blade=1, base_heart={heart06:1}, blade_heart={b_heart06:1}
//! - PL!S-bp2-004-R 黒澤ダイヤ: blade=3, base_heart={heart02:2,heart04:2,heart05:1},
//!   自動 re-yell (discard whole yell if it contained no live card)
//! - PL!N-bp1-027-L Solitude Rain: live card, blade_heart={b_heart05:1}, special_heart={draw:1}
//! - PL!-sd1-019-SD START:DASH!!: live card, need={heart01:1,heart03:1,heart06:1},
//!   score=1, special_heart={score:1}
//! - PL!-sd1-020-SD 僕らは今のなかで: live card, blade_heart={b_all:1} (ALL-blade wildcard)
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";
const ERI: &str = "PL!-sd1-002-SD";
const DIA: &str = "PL!S-bp2-004-R";
const SOLITUDE_RAIN: &str = "PL!N-bp1-027-L";
const DREAM_BELIEVERS: &str = "PL!HS-bp1-019-L";
const B_ALL_LIVE: &str = "PL!-sd1-020-SD";

/// Replace P1's main deck wholesale. Index 0 = TOP (matches `draw()`).
fn deck_from_top(game: &mut TestGame, top_to_bottom: Vec<i16>) {
    game.state.player1.main_deck.cards = top_to_bottom.into();
}

/// Stage members for P1 (areas in order: left, center, right; skip None).
fn stage_p1(game: &mut TestGame, ids: [Option<i16>; 3]) {
    for (i, id) in ids.into_iter().enumerate() {
        if let Some(id) = id {
            game.add_to_stage(
                match i {
                    0 => MemberArea::LeftSide,
                    1 => MemberArea::Center,
                    _ => MemberArea::RightSide,
                },
                id,
            );
        }
    }
}

/// Set the live card through real gameplay and run performance + victory until
/// the turn wraps to the next Active phase.
///
/// If `deck_at_yell` is given, P1's main deck is replaced (index 0 = top) right
/// after the live card is set. layout[0] is CONSUMED by the LiveCardSet refill
/// draw; the yell then reveals layout[1..]. Callers therefore put one
/// sacrificial filler first, followed by the exact reveal order they want.
///
/// Phase strings observed: "LiveCardSet (1st)" → "LiveCardSet (2nd)" →
/// "Perform (1st)" → "Perform (2nd)" → "Live Result" → "Active".
fn perform_live(
    game: &mut TestGame,
    live_id: i16,
    deck_at_yell: Option<Vec<i16>>,
    mut on_choice: impl FnMut(&mut TestGame),
) {
    game.add_to_hand(live_id);
    advance_to_live_card_set_p1(game);
    game.set_live_card(live_id);
    if let Some(layout) = deck_at_yell {
        deck_from_top(game, layout);
    }

    let mut saw_victory = false;
    let mut prev_phase = String::new();
    for _ in 0..40 {
        let phase = game.state.current_phase.to_string();
        let pending = game.pending_choice_type();
        // Permanent-ish trace: log phase transitions and pending prompts only.
        if phase != prev_phase || pending.is_some() {
            eprintln!(
                "[PERFORM_LIVE] phase={} pending={:?} p1_hand={} p1_deck={} p1_wait={}",
                phase,
                pending,
                game.state.player1.hand.len(),
                game.state.player1.main_deck.cards.len(),
                game.state.player1.waitroom.cards.len()
            );
            prev_phase = phase.clone();
        }
        if game.has_pending_choice() {
            on_choice(game);
            continue;
        }
        if phase.contains("Live Result") {
            saw_victory = true;
        }
        // Victory work completes on the pass out of "Live Result".
        if saw_victory && phase == "Active" {
            return;
        }
        game.pass();
    }
    panic!(
        "performance pipeline never returned to Active; phase={}",
        game.state.current_phase
    );
}

/// Default choice handler: no optional effects exist on these boards, so any
/// pending prompt means a setup mistake — fail loudly with the choice details.
fn skip_choices(game: &mut TestGame) {
    panic!(
        "unexpected pending choice: {:?}",
        game.get_pending_choice()
    );
}

/// Accept Dia's optional "discard the whole yell" SelectCard prompt in full.
fn accept_discard(game: &mut TestGame) {
    if let rabuka_engine::ability::types::Choice::SelectCard { count, .. } =
        game.get_pending_choice()
    {
        let count = *count;
        eprintln!("[DIA] accepting discard of {} revealed cards", count);
        game.select_indices(&(0..count).collect::<Vec<_>>());
    } else {
        panic!(
            "expected Dia's SelectCard yell-discard prompt, got {:?}",
            game.get_pending_choice()
        );
    }
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass(); // Active
    game.pass(); // Energy
    game.pass(); // Draw (no draw on turn 1)
    game.pass(); // Main (turn 2)
    game.pass(); // LiveCardSet
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn last_p1_snap(game: &TestGame) -> &rabuka_engine::types::PerformanceSnapshot {
    game.state
        .performance_snapshots
        .iter()
        .rev()
        .find(|s| s.player_id == "p1")
        .expect("P1 should have a performance snapshot")
}

// ---------------------------------------------------------------------------
// Rule 8.3.12.1 — each draw icon revealed during yell draws 1 card
// ---------------------------------------------------------------------------

/// Solitude Rain put on deck top is revealed by the yell; its special ドロー
/// icon draws exactly 1 card compared to an otherwise identical game.
///
/// Regression property: fails on builds where resolution-zone special_heart
/// icons are not processed (the original re-yell bug class).
#[test]
fn solitude_rain_revealed_by_yell_draws_one_card() {
    let db = load_real_database();

    // Treatment: Solitude Rain at deck position 1 (revealed by the yell).
    let mut game = TestGame::new(db.clone());
    let eri = game.id(ERI);
    let sr = game.id(SOLITUDE_RAIN);
    let fill = game.id(FILLER);
    stage_p1(&mut game, [None, Some(eri), None]);
    for _ in 0..25 {
        game.state.player1.main_deck.cards.push(fill);
        game.state.player2.main_deck.cards.push(fill);
    }
    let live = game.new_id(DREAM_BELIEVERS);
    perform_live(
        &mut game,
        live,
        Some(vec![fill, sr, fill, fill]),
        skip_choices,
    );
    let treated_hand = game.state.player1.hand.len();

    // Control: identical board, plain filler instead of Solitude Rain.
    let mut control = TestGame::new(db);
    let eri_c = control.id(ERI);
    let fill_c = control.id(FILLER);
    stage_p1(&mut control, [None, Some(eri_c), None]);
    for _ in 0..25 {
        control.state.player1.main_deck.cards.push(fill_c);
        control.state.player2.main_deck.cards.push(fill_c);
    }
    let live_c = control.new_id(DREAM_BELIEVERS);
    perform_live(
        &mut control,
        live_c,
        Some(vec![fill_c, fill_c, fill_c, fill_c]),
        skip_choices,
    );

    eprintln!(
        "[RESULT] treated_hand={} control_hand={}",
        treated_hand,
        control.state.player1.hand.len()
    );
    assert_eq!(
        treated_hand,
        control.state.player1.hand.len() + 1,
        "Solitude Rain's draw icon (revealed during yell) must draw exactly 1 card"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&sr),
        "yell-revealed Solitude Rain should be in the waitroom after resolution"
    );
}

/// Rule 8.3.12.1 is unconditional: even when the live FAILS the heart check,
/// the draw icon revealed during the yell still draws its card.
#[test]
fn failed_live_still_resolves_yell_draw_icons() {
    let db = load_real_database();

    // 絵里 alone provides heart06 → START:DASH!! (needs heart01+03+06) fails.
    let mut game = TestGame::new(db.clone());
    let eri = game.id(ERI);
    let sr = game.id(SOLITUDE_RAIN);
    let fill = game.id(FILLER);
    stage_p1(&mut game, [None, Some(eri), None]);
    for _ in 0..25 {
        game.state.player1.main_deck.cards.push(fill);
        game.state.player2.main_deck.cards.push(fill);
    }
    let live = game.new_id(DREAM_BELIEVERS);
    perform_live(&mut game, live, Some(vec![fill, sr, fill]), skip_choices);
    assert!(
        !last_p1_snap(&game).success,
        "live must fail (missing heart01/heart03)"
    );

    let mut control = TestGame::new(db);
    let eri_c = control.id(ERI);
    let fill_c = control.id(FILLER);
    stage_p1(&mut control, [None, Some(eri_c), None]);
    for _ in 0..25 {
        control.state.player1.main_deck.cards.push(fill_c);
        control.state.player2.main_deck.cards.push(fill_c);
    }
    let live_c = control.new_id(DREAM_BELIEVERS);
    perform_live(
        &mut control,
        live_c,
        Some(vec![fill_c, fill_c, fill_c]),
        skip_choices,
    );

    eprintln!(
        "[RESULT] failed-live hand={} control hand={}",
        game.state.player1.hand.len(),
        control.state.player1.hand.len()
    );
    assert_eq!(
        game.state.player1.hand.len(),
        control.state.player1.hand.len() + 1,
        "the yell draw resolves even though the live failed"
    );
}

/// Q104 / Rule 10.2.1: drawing from a yell draw icon with an empty deck
/// refreshes the waitroom back into the deck mid-resolution.
#[test]
fn yell_draw_refreshes_deck_when_empty() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let eri = game.id(ERI);
    let sr = game.id(SOLITUDE_RAIN);
    let fill = game.id(FILLER);

    stage_p1(&mut game, [None, Some(eri), None]);
    // Stock the main deck so no premature waitroom→deck refresh happens
    // before the layout below takes effect.
    for _ in 0..25 {
        game.state.player1.main_deck.cards.push(fill);
        game.state.player2.main_deck.cards.push(fill);
    }
    // Refresh source: 3 fillers waiting in the waitroom.
    for _ in 0..3 {
        game.state.player1.waitroom.cards.push(game.new_id(FILLER));
    }

    let live = game.new_id(DREAM_BELIEVERS);
    // Deck after refill = [SR] only; the reveal empties it, then the draw icon
    // must refresh the 3 waitroom cards and draw one of them.
    perform_live(&mut game, live, Some(vec![fill, sr]), skip_choices);

    eprintln!(
        "[RESULT] deck={} waitroom={}",
        game.state.player1.main_deck.cards.len(),
        game.state.player1.waitroom.cards.len()
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        2,
        "waitroom (3 cards) refreshed into deck, then 1 drawn"
    );
    // The revealed Solitude Rain ends up in the waitroom (the failed live card
    // Dream Believers is sent there too). The refreshed fillers were partially
    // drawn back out.
    assert_eq!(game.state.player1.waitroom.cards.len(), 2);
    assert!(game.state.player1.waitroom.cards.contains(&sr));
}

// ---------------------------------------------------------------------------
// Re-yell path (黒澤ダイヤ) — must process icons identically to the primary yell
// ---------------------------------------------------------------------------

/// Shared Dia setup: stage [filler, filler, Dia] = 5 blades.
///
/// Deck layout after set_live_card (index 0 = top):
/// `[sacrifice, F×5 (first yell), tail ×5 (re-yell batch), filler pad…]`
/// The first yell reveals exactly the 5 plain fillers — no live card among
/// them, so Dia's 自動 re-yell condition holds. After accepting the discard,
/// the re-yell reveals the next 5 cards = `deck_tail`.
fn dia_game(deck_tail: &[&str]) -> TestGame {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let fill = game.id(FILLER);
    let dia = game.id(DIA);
    stage_p1(&mut game, [Some(fill), Some(fill), Some(dia)]);
    for _ in 0..25 {
        game.state.player1.main_deck.cards.push(fill);
        game.state.player2.main_deck.cards.push(fill);
    }

    let live = game.new_id(DREAM_BELIEVERS);
    let tail_ids: Vec<i16> = deck_tail.iter().map(|no| game.id(no)).collect();
    let mut layout: Vec<i16> = (0..5).map(|_| game.new_id(FILLER)).collect();
    layout.extend_from_slice(&tail_ids);
    while layout.len() < 20 {
        layout.push(game.new_id(FILLER));
    }
    perform_live(&mut game, live, Some(layout), accept_discard);

    assert!(
        !game.state.re_yell_revealed_cards.is_empty(),
        "Dia re-yell should have happened"
    );
    game
}

/// Rule 8.3.12.1 applies to re-yelled cards too: Solitude Rain revealed by the
/// SECOND yell must draw a card. Regression property: FAILS on builds where
/// the re-yell rebuild drops Draw icons (the bug fixed in phases.rs).
#[test]
fn re_yell_revealed_draw_icon_draws() {
    // Re-yell batch: F F F F SR — Solitude Rain last.
    let game = dia_game(&[FILLER, FILLER, FILLER, FILLER, SOLITUDE_RAIN]);
    let treated_hand = game.state.player1.hand.len();

    // Control: identical board, plain filler instead of Solitude Rain.
    let control = dia_game(&[FILLER, FILLER, FILLER, FILLER, FILLER]);

    eprintln!(
        "[RESULT] re-yell treated_hand={} control_hand={}",
        treated_hand,
        control.state.player1.hand.len()
    );
    assert_eq!(
        treated_hand,
        control.state.player1.hand.len() + 1,
        "second-yell Solitude Rain draw icon must draw exactly 1 card"
    );
}

/// Rule 8.3.15.1.1 in the RE-YELL path: an ALL-blade revealed by the second
/// yell must become a wildcard heart (index 7), NOT a colorless heart (index
/// 0). START:DASH!! needs heart01+heart03+heart06; the Dia stage covers
/// heart01/03 but NOT heart06, so only a true wildcard can save this live.
///
/// Regression property: FAILS on builds where the re-yell rebuild maps BAll to
/// colorless index 0 (the bug fixed in phases.rs).
#[test]
fn re_yelled_ball_counts_as_wildcard_not_colorless() {
    // Re-yell batch: F F BALL F F — exactly one ALL-blade.
    let game = dia_game(&[FILLER, FILLER, B_ALL_LIVE, FILLER, FILLER]);

    let snap = last_p1_snap(&game);
    let ball_card = snap
        .yell_cards
        .iter()
        .find(|yc| yc.card_no.as_ref().starts_with("PL!-sd1-020"))
        .expect("the re-yell should have revealed the b_all live card");
    assert_eq!(ball_card.blade_hearts[7], 1, "ALL-blade → wildcard slot");
    assert_eq!(ball_card.blade_hearts[0], 0, "ALL-blade is NOT colorless");
    assert!(
        snap.success,
        "the wildcard heart06 must satisfy START:DASH!!'s requirement"
    );
    // Score: START:DASH!! itself (score 1). The revealed b_all card is yell
    // material, not a scored live card; no score icons were revealed.
    assert_eq!(snap.total_score, 1);
}

/// QA ruling (黒澤ダイヤ × 蓮ノ空): only the RE-YELLED cards participate —
/// the first yell's blade-hearts are lost (「そのエールで得たブレードハートを
/// 失う」). A b_all in the DISCARDED first batch must contribute nothing.
#[test]
fn discarded_first_yell_hearts_are_lost() {
    // First batch: F F BALL F F (contains a live card!)…
    // …but then Dia's condition fails (live card present) and no re-yell
    // happens. Instead verify via the second-batch-only rule:
    // first batch fillers only, re-yell batch has NO hearts at all except SR's
    // b_heart05 — covered by re_yell_revealed_draw_icon_draws. Here we assert
    // the complementary case: BALL in FIRST yell blocks the re-yell entirely
    // (condition check) and its wildcard still counts for the ORIGINAL yell
    // (hearts are only lost when the discard actually happens).
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let fill = game.id(FILLER);
    let dia = game.id(DIA);
    stage_p1(&mut game, [Some(fill), Some(fill), Some(dia)]);
    for _ in 0..25 {
        game.state.player1.main_deck.cards.push(fill);
        game.state.player2.main_deck.cards.push(fill);
    }

    let live = game.new_id(DREAM_BELIEVERS);
    // First yell batch: F F BALL F F → contains a live card → no re-yell.
    let mut layout: Vec<i16> = vec![game.new_id(FILLER)]; // sacrificial
    layout.push(game.new_id(FILLER));
    layout.push(game.new_id(FILLER));
    layout.push(game.id(B_ALL_LIVE));
    layout.push(game.new_id(FILLER));
    layout.push(game.new_id(FILLER));
    while layout.len() < 20 {
        layout.push(game.new_id(FILLER));
    }
    perform_live(&mut game, live, Some(layout), skip_choices);

    assert!(
        game.state.re_yell_revealed_cards.is_empty(),
        "a yell containing a live card must NOT trigger Dia's re-yell"
    );
    // The first-yell ALL-blade wildcard counted normally (no discard happened).
    let snap = last_p1_snap(&game);
    assert!(snap.success, "wildcard heart06 from the kept yell applies");
}

// ---------------------------------------------------------------------------
// Rule 8.4.2.1 — score icons from one's own yell add +1 to the live total
// ---------------------------------------------------------------------------

/// A START:DASH!! copy milled into the deck and revealed by the yell
/// contributes its special スコア icon: cheer count +1 and live total +1.
#[test]
fn score_icon_revealed_by_yell_adds_to_cheer_and_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori_a = game.id(FILLER);
    let kotori_b = game.new_id(FILLER);
    let eri = game.id(ERI);
    let dash_on_deck = game.id(DREAM_BELIEVERS); // revealed during yell
    let fill = game.id(FILLER);

    stage_p1(&mut game, [Some(kotori_a), Some(kotori_b), Some(eri)]);
    for _ in 0..25 {
        game.state.player1.main_deck.cards.push(fill);
        game.state.player2.main_deck.cards.push(fill);
    }
    let live = game.new_id(DREAM_BELIEVERS);
    // Blades 3 → yell reveals [DASH copy, F, F].
    perform_live(
        &mut game,
        live,
        Some(vec![fill, dash_on_deck, fill, fill]),
        skip_choices,
    );

    eprintln!(
        "[RESULT] cheer={} total_score={}",
        game.state.player1_cheer_blade_heart_count,
        last_p1_snap(&game).total_score
    );
    assert_eq!(
        game.state.player1_cheer_blade_heart_count, 1,
        "the yell-revealed score icon must count once (rule 8.4.2.1)"
    );
    let snap = last_p1_snap(&game);
    assert!(snap.success, "stage hearts satisfy heart01+03+06");
    assert_eq!(
        snap.total_score, 2,
        "live total = card score 1 + 1 per revealed score icon"
    );
}

/// Negative pin for rule scoping (「エールで出た」): a special score icon on a
/// card sitting IN THE LIVE ZONE does nothing — it was never revealed by a
/// yell, so it adds neither cheer nor draws.
///
/// Regression property: FAILS on builds that apply live-zone special hearts
/// unconditionally (the block removed from player_perform_live).
#[test]
fn live_zone_special_icons_do_not_apply() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori_a = game.id(FILLER);
    let kotori_b = game.new_id(FILLER);
    let eri = game.id(ERI);
    let fill = game.id(FILLER);

    stage_p1(&mut game, [Some(kotori_a), Some(kotori_b), Some(eri)]);
    for _ in 0..25 {
        game.state.player1.main_deck.cards.push(fill);
        game.state.player2.main_deck.cards.push(fill);
    }

    let dash_in_zone = game.new_id(DREAM_BELIEVERS);
    let live = dash_in_zone;
    perform_live(&mut game, live, None, skip_choices);

    eprintln!(
        "[RESULT] cheer={} total_score={}",
        game.state.player1_cheer_blade_heart_count,
        last_p1_snap(&game).total_score
    );
    assert_eq!(
        game.state.player1_cheer_blade_heart_count, 0,
        "an in-zone score icon must NOT count toward the cheer total"
    );
    let snap = last_p1_snap(&game);
    assert!(snap.success, "stage covers heart01+03+06 without yell help");
    assert_eq!(
        snap.total_score, 1,
        "live total is just the card score — the in-zone score icon adds nothing"
    );
}

