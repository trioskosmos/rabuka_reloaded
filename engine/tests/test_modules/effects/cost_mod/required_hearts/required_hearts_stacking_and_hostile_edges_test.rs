/// PL!S-bp7-020-L HAPPY PARTY TRAIN — stacking + hostile edges (thin-coverage gaps).
///
/// Printed abilities (both ライブ開始時, independent, both 必要ハート heart0 −1):
///   ab#0: 自分のステージにいるすべてのメンバーがアクティブ状態の場合、…
///   ab#1: 自分のデッキの下からカードを1枚控え室に置く。それが『Aqours』の
///         メンバーカードの場合、…
///
/// Gaps closed here (docs/TEST_AUDIT_PLAN.md):
///   - STACKING: A and B both true → reduced twice (−2), not clamped to −1
///   - B with EMPTY deck → mill-shortfall + condition-on-no-card path (no panic)
///   - A with opponent waited but own all active → own-stage gate precision
///   - reduction honored at performance end-to-end (not just the modifier)
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

const HPT: &str = "PL!S-bp7-020-L";
const AQOURS_M: &str = "PL!S-sd1-001-SD"; // 千歌: heart02=3, heart04=2, heart05=2, Aqours
const FILLER: &str = "PL!-sd1-010-SD"; // μ's member (non-Aqours mill)

fn fire_trigger_nth(
    game: &mut TestGame,
    cid: i16,
    trigger: AbilityTrigger,
    trig: &str,
    nth: usize,
) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .filter(|a| a.triggers.as_deref() == Some(trig))
            .nth(nth)
            .unwrap_or_else(|| panic!("card {} lacks '{trig}' ability #{nth}", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        trigger,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

fn fire_ab0(game: &mut TestGame, hpt: i16) {
    fire_trigger_nth(game, hpt, AbilityTrigger::LiveStart, "ライブ開始時", 0);
}

fn fire_ab1(game: &mut TestGame, hpt: i16) {
    fire_trigger_nth(game, hpt, AbilityTrigger::LiveStart, "ライブ開始時", 1);
}

fn hpt_in_live_zone(game: &mut TestGame) -> i16 {
    let hpt = game.id(HPT);
    game.state.player1.live_card_zone.cards.push(hpt);
    hpt
}

fn need_mod(game: &TestGame, hpt: i16) -> i32 {
    game.state
        .mods
        .get_need_heart_modifier(hpt, HeartColor::Heart00)
}

/// Both gates true → heart0 −2 (stacking), not −1.
#[test]
fn pl_s_bp7_020_l_stacking_both_gates_reduce_twice() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hpt = hpt_in_live_zone(&mut game);

    let m = game.new_id(AQOURS_M);
    game.state.player1.stage.stage[0] = m;
    game.state.mods.add_orientation_modifier(m, "active");

    // Deck: [filler, aqours_bottom] → ab#1 mills the Aqours member from bottom.
    let filler = game.id(FILLER);
    let aq_bottom = game.new_id(AQOURS_M);
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(aq_bottom);

    fire_ab0(&mut game, hpt);
    assert_eq!(need_mod(&game, hpt), -1, "ab#0 alone: all-active → −1");
    fire_ab1(&mut game, hpt);
    assert!(
        game.state.player1.waitroom.cards.contains(&aq_bottom),
        "ab#1 milled the Aqours bottom card"
    );
    assert_eq!(
        need_mod(&game, hpt),
        -2,
        "STACKING: independent live-start abilities both apply → heart0 −2"
    );
}

/// B with EMPTY deck: mill is a no-op; condition has no preceding_moved card → no reduction, no panic.
#[test]
fn pl_s_bp7_020_l_empty_deck_mill_shortfall_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hpt = hpt_in_live_zone(&mut game);
    game.state.player1.main_deck.cards.clear();
    assert!(game.state.player1.main_deck.cards.is_empty());

    fire_ab1(&mut game, hpt);

    assert!(game.state.player1.waitroom.cards.is_empty(), "nothing to mill");
    assert_eq!(
        need_mod(&game, hpt),
        0,
        "condition-on-no-card (empty deck) → no heart reduction"
    );
    // ab#0 still independent: all-active path unaffected by the mill shortfall.
    let m = game.new_id(AQOURS_M);
    game.state.player1.stage.stage[0] = m;
    game.state.mods.add_orientation_modifier(m, "active");
    fire_ab0(&mut game, hpt);
    assert_eq!(need_mod(&game, hpt), -1, "ab#0 still applies after empty-deck mill");
}

/// Own-stage gate only: opponent waited must NOT break すべて…アクティブ.
#[test]
fn pl_s_bp7_020_l_opponent_waited_own_all_active_still_reduces() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hpt = hpt_in_live_zone(&mut game);

    let own = game.new_id(AQOURS_M);
    game.state.player1.stage.stage = [own, -1, -1];
    game.state.mods.add_orientation_modifier(own, "active");

    let opp = game.new_id(FILLER);
    game.state.player2.stage.stage = [opp, -1, -1];
    game.state.mods.add_orientation_modifier(opp, "wait");

    fire_ab0(&mut game, hpt);
    assert_eq!(
        need_mod(&game, hpt),
        -1,
        "own-stage gate: opponent orientation is irrelevant → still −1"
    );

    // Precision: flipping OWN member to wait flips the gate off (mod stays −1 from prior fire;
    // re-check on a fresh card so we assert a clean 0).
    let hpt2 = hpt_in_live_zone(&mut game);
    game.state.mods.add_orientation_modifier(own, "wait");
    fire_ab0(&mut game, hpt2);
    assert_eq!(
        need_mod(&game, hpt2),
        0,
        "own waited member breaks the gate (clean instance)"
    );
}

/// End-to-end: reduction is honored in the performance heart check, not only as a stored mod.
///
/// Base need: heart02=1, heart04=2, heart05=2, heart0=3.
/// Stage (千歌 alone, active): heart02=3, heart04=2, heart05=2 → after colored needs,
/// leftover heart02 surplus = 2. Full heart0=3 would FAIL (2 < 3); heart0 reduced by
/// stacking (−2) → heart0=1, leftover 2 ≥ 1 → PASS.
#[test]
fn pl_s_bp7_020_l_reduction_honored_at_performance_end_to_end() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hpt = game.id(HPT);
    let m = game.new_id(AQOURS_M);
    let filler = game.id(FILLER);
    game.state.player1.stage.stage = [m, -1, -1];
    game.state.mods.add_orientation_modifier(m, "active");
    game.state.player1.hand.cards.push(hpt);
    // Bottom Aqours so ab#1 also fires during real LiveStart.
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(m);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.insert(0, filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(hpt);
    for _ in 0..2 {
        game.pass();
    }
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        need_mod(&game, hpt),
        -2,
        "real LiveStart fired both abilities → heart0 −2"
    );

    for _ in 0..7 {
        game.pass();
        while game.has_pending_choice() {
            game.select_indices(&[]);
        }
    }

    let snap = game
        .state
        .performance_snapshots
        .iter()
        .find(|s| s.player_id == "p1")
        .expect("P1 performance snapshot");
    assert_eq!(
        snap.lives[0].required[0], 1,
        "performance required[heart0] must be base 3 + (−2) = 1"
    );
    assert!(
        snap.lives[0].passed,
        "reduced heart0 is honored: leftover colored surplus covers heart0=1"
    );
    assert!(snap.success, "live succeeds end-to-end under stacking reduction");
}
