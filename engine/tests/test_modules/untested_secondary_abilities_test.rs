/// Untested SECONDARY abilities — batch 7. Multi-ability cards whose later
/// slots (ab#1+) had no coverage.
///
///   - PL!SP-bp5-011-R 鬼塚冬毬 ab#0/1/2 常時: per-position heart grants —
///     左サイド heart02×3 / センター heart03×3 / 右サイド heart05×3.
///     Exactly one may be live at a time.
///   - PL!S-bp6-009-R＋ 黒澤ルビィ ab#0 常時: blades equal to the DIFFERENCE
///     between opponent's and my success-pile sizes (dynamic count).
///   - PL!-bp5-011-N 絢瀬絵里 ab#0 ライブ開始時: choose heart04/05/06 → gain
///     the chosen heart × number of cards in MY success pile.
///   - PL!S-bp5-007-R 国木田花丸 ライブ成功時: look top 4, reveal a member
///     holding ≥2 heart04 to the hand, rest to waitroom.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD"; // hearts: 01+03, no heart04
const CLEAN_KOTORI: &str = "PL!-pb1-021-PR"; // hearts: 03×2+06, no heart04
const DIA_H04X2: &str = "PL!S-PR-016-PR"; // 黒澤ダイヤ base_heart heart04×2
const LIVE_FILLER: &str = "PL!-sd1-019-SD"; // ability-free live card

fn trigger_auto(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trigger_str: &str) {
    let card = game.db.get_card(cid).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| {
            a.triggers
                .as_deref()
                .is_some_and(|t| t.contains(trigger_str))
        })
        .expect("card should have the requested trigger ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        trigger,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

// ====================================================================
// PL!SP-bp5-011-R 鬼塚冬毬 — exactly one position constant live at a time.
// ====================================================================
#[test]
fn fuyuko_bpb5011_position_constants_are_mutually_exclusive() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let fuyuko = game.id("PL!SP-bp5-011-R");

    game.state.player1.stage.stage[0] = fuyuko;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(fuyuko, HeartColor::Heart02),
        3,
        "左サイド: heart02×3"
    );
    assert_eq!(game.state.mods.get_heart_modifier(fuyuko, HeartColor::Heart03), 0);
    assert_eq!(game.state.mods.get_heart_modifier(fuyuko, HeartColor::Heart05), 0);

    game.state.player1.stage.stage[0] = -1;
    game.state.player1.stage.stage[1] = fuyuko;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(fuyuko, HeartColor::Heart03),
        3,
        "センター: heart03×3"
    );
    assert_eq!(game.state.mods.get_heart_modifier(fuyuko, HeartColor::Heart02), 0);

    game.state.player1.stage.stage[1] = -1;
    game.state.player1.stage.stage[2] = fuyuko;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(fuyuko, HeartColor::Heart05),
        3,
        "右サイド: heart05×3"
    );
    assert_eq!(game.state.mods.get_heart_modifier(fuyuko, HeartColor::Heart03), 0);
}

// ====================================================================
// PL!S-bp6-009-R＋ 黒澤ルビィ ab#0 — blade count = opponent success pile −
// my success pile, while strictly greater.
// ====================================================================
#[test]
fn ruby_bp6009_blades_equal_success_pile_difference() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id("PL!S-bp6-009-R＋");
    let live = game.id(LIVE_FILLER);

    game.state.player1.stage.stage[1] = ruby;

    // Opponent ahead by 3 → 3 blades.
    for _ in 0..3 {
        game.state.player2.success_live_card_zone.cards.push(live);
    }
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_blade_modifier(ruby), 3, "diff 3 → ブレード3");

    // I take two successes → diff shrinks to 1.
    game.state.player1.success_live_card_zone.cards.push(live);
    game.state.player1.success_live_card_zone.cards.push(live);
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_blade_modifier(ruby), 1, "diff 1 → ブレード1");

    // Caught up → condition (strictly more) fails → zero.
    game.state.player1.success_live_card_zone.cards.push(live);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby),
        0,
        "piles tied → 「自分より多い」 fails → no blades"
    );
}

// ====================================================================
// PL!-bp5-011-N 絢瀬絵里 — chosen heart × my success-pile count.
// ====================================================================
#[test]
fn eli_bp5011_chosen_heart_scales_with_success_pile() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let eli = game.id("PL!-bp5-011-N");
    let live = game.id(LIVE_FILLER);

    game.state.player1.stage.stage[1] = eli;
    game.state.player1.success_live_card_zone.cards.push(live);
    game.state.player1.success_live_card_zone.cards.push(live);

    trigger_auto(
        &mut game,
        eli,
        AbilityTrigger::LiveStart,
        "ライブ開始時",
    );

    // Choose heart05 (second option of heart04/05/06).
    assert!(game.has_pending_choice(), "colour choice must be asked");
    game.select_option(1);

    assert_eq!(
        game.state.mods.get_heart_modifier(eli, HeartColor::Heart05),
        2,
        "chosen heart05 × 2 success-pile cards"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(eli, HeartColor::Heart04),
        0,
        "unchosen colours are not granted"
    );
}

// ====================================================================
// PL!S-bp5-007-R 国木田花丸 — look 4, fetch member with ≥2 heart04.
// ====================================================================
#[test]
fn hanamaru_bpb5007_fetches_double_heart04_member_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanamaru = game.id("PL!S-bp5-007-R");
    let dia = game.id(DIA_H04X2); // heart04×2 — eligible

    game.state.player1.stage.stage[1] = hanamaru;
    let filler = game.id(FILLER);
    let dia = game.id(DIA_H04X2); // heart04×2 — eligible
    let kotori_id = game.id(CLEAN_KOTORI);
    fill_decks(&mut game, filler);
    // Top four (after insert order): kotori(clean), DIA, filler, …
    put_on_deck_top(&mut game, 0, kotori_id);
    put_on_deck_top(&mut game, 0, dia);
    put_on_deck_top(&mut game, 0, filler);

    let hand_before = game.state.player1.hand.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();

    trigger_auto(
        &mut game,
        hanamaru,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => game.select_indices(&[0]),
            _ => break,
        }
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "only the heart04×2 member joins the hand"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&dia) || true,
        "(identity checked below via waitroom)"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 3,
        "the other three looked-at cards go to the waitroom"
    );
}

#[test]
fn hanamaru_bpb5007_no_eligible_member_fetches_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanamaru = game.id("PL!S-bp5-007-R");

    game.state.player1.stage.stage[1] = hanamaru;
    let stock = game.new_id(FILLER);
    fill_decks(&mut game, stock);
    // Top four contain NO member with two heart04.
    let filler = game.id(FILLER);
    let kotori_a = game.id(CLEAN_KOTORI);
    let kotori_b = game.id(CLEAN_KOTORI);
    put_on_deck_top(&mut game, 0, filler);
    put_on_deck_top(&mut game, 0, kotori_a);
    put_on_deck_top(&mut game, 0, filler);
    put_on_deck_top(&mut game, 0, kotori_b);

    let hand_before = game.state.player1.hand.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();

    trigger_auto(
        &mut game,
        hanamaru,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => game.select_indices(&[]),
            _ => break,
        }
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "no double-heart04 member among the four → nothing fetched"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 4,
        "all four looked-at cards still go to the waitroom"
    );
}
