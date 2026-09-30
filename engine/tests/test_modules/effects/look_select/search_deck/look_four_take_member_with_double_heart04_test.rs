use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";
const CLEAN_KOTORI: &str = "PL!-pb1-021-PR";
const DIA_H04X2: &str = "PL!S-PR-016-PR";
/// Prints EXACTLY ONE heart04 — `base_heart: {heart04: 1, heart06: 1}`, no abilities.
/// This is the only fixture class that can tell 「heart04を2つ以上持つ」 apart from
/// 「heart04を持つ」: every other fixture in this file is either 2+ heart04 (DIA) or 0
/// (KOTORI, FILLER), and on those two classes a presence test and a count test are
/// extensionally identical. See docs/ABILITY_AUDIT_2026-09-30.md §B1.
const HANA_H04X1: &str = "PL!HS-bp5-009-N";

/// The PRINTED heart04 count of `id`, read straight from `cards.json`.
///
/// `base_heart` is a flat `heartNN -> count` map; 0 for a card that prints none. This is
/// the number the printed 「ハートにheart04を2つ以上持つ」 is about — not a modifier, and not
/// the `need_heart` side of the engine's `base.max(need)`.
fn printed_heart04(game: &TestGame, id: i16) -> u8 {
    game.db
        .get_card(id)
        .and_then(|c| c.base_heart.as_ref())
        .and_then(|h| h.hearts.get(&rabuka_engine::card::parse_heart_color("heart04")).copied())
        .unwrap_or(0)
}

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

#[test]
fn pl_s_bp5_007_r_live_success_look_four_adds_one_to_hand_and_three_to_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanamaru = game.id("PL!S-bp5-007-R");

    game.state.player1.stage.stage[1] = hanamaru;
    let filler = game.id(FILLER);
    let dia = game.id(DIA_H04X2);
    let kotori_id = game.id(CLEAN_KOTORI);
    fill_decks(&mut game, filler);
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
    assert_eq!(
        game.state.player1.hand.cards.first(),
        Some(&dia),
        "the heart04×2 member (Dia) is the one fetched to hand"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 3,
        "the other three looked-at cards go to the waitroom"
    );
}

#[test]
fn pl_s_bp5_007_r_live_success_no_double_heart04_member_sends_all_four_to_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanamaru = game.id("PL!S-bp5-007-R");

    game.state.player1.stage.stage[1] = hanamaru;
    let stock = game.new_id(FILLER);
    fill_decks(&mut game, stock);
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

/// The 「2つ以上」 clause itself. `DIA_H04X2` prints two heart04 and must be offered;
/// `HANA_H04X1` prints exactly one and must NOT be.
///
/// This is the only test in the tree that distinguishes the printed COUNT from mere
/// colour PRESENCE, and it is not a restatement of the two above: those stage only
/// 2-or-more and 0, a set on which 「has heart04」 and 「has 2+ heart04」 agree.
///
/// It fails if `heart_color_count` is absent from the `select_action` parse: without it
/// `check_heart_colors` degrades to `contains_key`, HANA qualifies, and the offered count
/// becomes 2 instead of 1.
#[test]
fn pl_s_bp5_007_r_live_success_the_two_heart_threshold_excludes_a_single_heart04_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanamaru = game.id("PL!S-bp5-007-R");

    // Pin both fixtures' printed heart04 counts, so a substitution cannot quietly
    // change which side of the threshold each card sits on.
    let dia = game.id(DIA_H04X2);
    game.assert_card_identity(dia, DIA_H04X2);
    assert_eq!(
        printed_heart04(&game, dia),
        2,
        "precondition: DIA_H04X2 prints exactly 2 heart04, so it satisfies '2つ以上'"
    );

    let hana = game.id(HANA_H04X1);
    game.assert_card_identity(hana, HANA_H04X1);
    assert_eq!(
        printed_heart04(&game, hana),
        1,
        "precondition: HANA_H04X1 prints exactly 1 heart04, so it must NOT satisfy '2つ以上'. \
         Without this pin a wrong fixture could put the card on the qualifying side."
    );

    game.state.player1.stage.stage[1] = hanamaru;
    let stock = game.new_id(FILLER);
    fill_decks(&mut game, stock);
    let filler = game.id(FILLER);
    put_on_deck_top(&mut game, 0, filler);
    put_on_deck_top(&mut game, 0, hana);
    put_on_deck_top(&mut game, 0, dia);

    trigger_auto(
        &mut game,
        hanamaru,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );

    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "precondition: DIA_H04X2 qualifies, so a selection prompt must be open"
    );
    assert_eq!(
        game.pending_choice_option_count(),
        Some(1),
        "ハートにheart04を2つ以上持つ — of the four looked-at cards only DIA_H04X2 prints 2 \
         heart04. HANA_H04X1 prints one and the fillers print none, so exactly ONE candidate \
         may be offered. A count of 2 means the engine is filtering on colour PRESENCE."
    );

    game.select_indices(&[0]);
    assert!(
        game.state.player1.hand.cards.contains(&dia),
        "the one offered card is the 2-heart04 member"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&hana),
        "the 1-heart04 member must be discarded with the rest, not taken to hand"
    );
}
