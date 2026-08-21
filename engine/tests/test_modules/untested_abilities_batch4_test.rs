/// Untested abilities — batch 4.
///
///   - PL!-pb1-003-R 南ことり  登場: optional self-wait cost; per-Printemps
///     member energy activation
///   - PL!HS-bp2-002-R＋ 村野さやか 登場 fetch cost≤2 members (max 2) +
///     常時 blade×3 while a higher-cost member is on stage
///   - PL!N-bp7-008-R エマ・ヴェルデ 登場: non-blade-heart members from the
///     waitroom under the deck, +1 active energy per card placed
///   - Interaction probe: exactly-energy-count constants (にこ PR-021) must
///     refresh when an activation cost is PAID mid-turn (「〜あるかぎり」).
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD"; // 高坂穂乃果 Printemps — HAS a blade heart (b_heart03)
const KOTORI_CLEAN: &str = "PL!-pb1-021-PR"; // 南ことり cost5 Printemps, ability-free, NO blade heart

fn trigger_auto(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trigger_str: &str) {
    let card = game.db.get_card(cid).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some(trigger_str))
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
// PL!-pb1-003-R 南ことり — 登場 このメンバーをウェイトにしてもよい：
// Printempsメンバー1人につき、エネルギーを1枚アクティブにする。
// ====================================================================
#[test]
fn kotori_pb1003_decline_self_wait_resolves_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id("PL!-pb1-003-R");
    let p1m = game.id(FILLER);
    let p2m = game.id(KOTORI_CLEAN);

    // Stage: 3 Printemps members (incl. kotori herself).
    game.state.player1.stage.stage = [p1m, kotori, p2m];
    // 5 energy: 2 active / 3 wait.
    game.give_energy(5);
    game.state.player1.energy_zone.set_active_count(2);

    trigger_auto(&mut game, kotori, AbilityTrigger::Debut, "登場");

    // Optional cost prompts; declining skips the whole effect.
    assert!(
        game.has_pending_choice(),
        "optional self-wait must prompt"
    );
    game.select_option(0); // decline

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "declined cost → no energy activated"
    );
    assert!(
        !game.has_pending_choice(),
        "nothing further is asked after declining"
    );
}

#[test]
fn kotori_pb1003_accept_waits_self_and_activates_per_printemps() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id("PL!-pb1-003-R");
    let p1m = game.id(FILLER);
    let p2m = game.id(KOTORI_CLEAN);

    game.state.player1.stage.stage = [p1m, kotori, p2m];
    game.give_energy(5);
    game.state.player1.energy_zone.set_active_count(2);

    trigger_auto(&mut game, kotori, AbilityTrigger::Debut, "登場");
    game.select_option(1); // accept: kotori waits

    assert_eq!(
        game.state.mods.get_orientation_modifier(kotori),
        Some("wait"),
        "accepted cost waits kotori"
    );
    // 3 Printemps on stage (counted regardless of state — no state filter in
    // the text) × 1 each = 3 wait energies flipped active.
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        5,
        "per-Printemps count includes every stage member (3) → 2+3=5 active"
    );
}

// ====================================================================
// PL!HS-bp2-002-R＋ 村野さやか ab#0 — 登場 控え室からコスト2以下のメンバーを
// 2枚まで手札に加える。
// ====================================================================
#[test]
fn sayaka_hsbp2002_fetch_is_cost_and_count_filtered() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-bp2-002-R＋");
    let cheap_a = game.id("PL!-bp3-012-PR"); // 南ことり cost 2
    let cheap_b = game.id("PL!SP-pb1-018-N"); // 米女メイ cost 2
    let pricey = game.id(KOTORI_CLEAN); // cost 5 — above the limit
    let live = game.id("PL!-sd1-019-SD"); // live card — wrong type

    game.state.player1.stage.stage[1] = sayaka;
    for cid in [pricey, cheap_a, live, cheap_b] {
        game.state.player1.waitroom.cards.push(cid);
    }
    let hand_before = game.state.player1.hand.cards.len();

    trigger_auto(&mut game, sayaka, AbilityTrigger::Debut, "登場");

    // Selection offers ONLY the two cost≤2 members.
    game.assert_pending_choice_type("SelectCard", "fetch should ask which members");
    game.select_indices(&[0, 1]);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 2,
        "two fetched members join the hand"
    );
    let waitroom = &game.state.player1.waitroom.cards;
    assert!(
        !waitroom.contains(&cheap_a) && !waitroom.contains(&cheap_b),
        "fetched cards leave the waitroom"
    );
    assert!(
        waitroom.contains(&pricey),
        "cost-5 member exceeds コスト2以下 → stays"
    );
    assert!(
        waitroom.contains(&live),
        "live card is not a メンバー → stays"
    );
}

// ====================================================================
// PL!HS-bp2-002-R＋ ab#1 — 常時: a member costing MORE than hers (13)
// present → ブレード3つ。
// ====================================================================
#[test]
fn sayaka_hsbp2002_blade_constant_tracks_higher_cost_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-bp2-002-R＋"); // cost 13
    let big = game.id("PL!S-bp5-009-R"); // 黒澤ルビィ cost 15

    game.state.player1.stage.stage[1] = sayaka;
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_blade_modifier(sayaka), 0);

    game.state.player1.stage.stage[0] = big;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(sayaka),
        3,
        "cost-15 member > her 13 → ブレード3つ"
    );

    game.state.player1.stage.stage[0] = -1;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(sayaka),
        0,
        "no bigger member → blades gone"
    );
}

// ====================================================================
// PL!N-bp7-008-R エマ — 登場: non-blade-heart members from waitroom under
// the deck (up to 4, any order); +1 ACTIVE energy per placed card.
// ====================================================================
#[test]
fn emma_bp7008_only_clean_members_place_and_pay_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let emma = game.id("PL!N-bp7-008-R");
    let clean = game.id(KOTORI_CLEAN); // no blade heart
    let bladefill = game.id(FILLER); // HAS b_heart03
    let bladefill2 = game.id("PL!-sd1-002-SD"); // 絢瀬絵里 — blade heart holder

    game.state.player1.stage.stage[1] = emma;
    for cid in [bladefill, clean, bladefill2] {
        game.state.player1.waitroom.cards.push(cid);
    }
    // Stock the deck (rule 10.2: an empty deck + non-empty waitroom triggers
    // an auto-refresh mid-effect, which would scramble assertions) — the
    // placed card lands on TOP of these at the BOTTOM end (push).
    let stock = game.id(FILLER);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(stock);
    game.state.player1.main_deck.cards.push(stock);
    // 4 energy: 1 active / 3 wait — activations draw from the wait pile.
    game.give_energy(4);
    game.state.player1.energy_zone.set_active_count(1);

    trigger_auto(&mut game, emma, AbilityTrigger::Debut, "登場");

    // Only the one clean member is selectable; skip the "place more?" re-prompt.
    game.select_indices(&[0]);
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => game.select_indices(&[]),
            _ => break,
        }
    }

    eprintln!(
        "[EMMA_DBG] deck={:?} waitroom={:?} clean={} blade={} blade2={}",
        game.state.player1.main_deck.cards,
        game.state.player1.waitroom.cards,
        clean,
        bladefill,
        bladefill2
    );
    assert_eq!(
        *game
            .state
            .player1
            .main_deck
            .cards
            .last()
            .expect("deck bottom holds the placed card"),
        clean,
        "placed card sits at the deck BOTTOM (push end)"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&clean),
        "placed card left the waitroom"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&bladefill)
            && game.state.player1.waitroom.cards.contains(&bladefill2),
        "blade-heart holders are not eligible"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "1 card placed → 1 wait energy activated (1+1)"
    );
}

// ====================================================================
// Interaction: にこ PR-021 常時 (exactly 7 energy → 2 blades). 「エネルギーが
// N枚」 counts the TOTAL energy zone (cards stay when flipped to wait), so
// the probe gains one card via 聖澤悠奈's live-start placement — and the
// constant must re-evaluate WITHOUT any manual recalc (「〜あるかぎり」).
// ====================================================================
#[test]
fn niko_pr021_exact_energy_constant_refreshes_on_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let niko = game.id("PL!-PR-021-PR");
    let yuna = game.id("PL!SP-bp5-222-R"); // ライブ開始時 E支払ってもよい: +1 energy card

    game.state.player1.stage.stage = [niko, yuna, -1];
    game.give_energy(6); // total 6 → condition off
    fill_energy_deck(&mut game, 0, 3);
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_blade_modifier(niko), 0);

    trigger_auto(
        &mut game,
        yuna,
        AbilityTrigger::LiveStart,
        "ライブ開始時",
    );
    game.select_option(1); // pay → 1 energy card placed into the zone

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        7,
        "placement brings the zone to exactly 7"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(niko),
        2,
        "exactly-7 condition must hold right after the gain (no manual recalc)"
    );
}
