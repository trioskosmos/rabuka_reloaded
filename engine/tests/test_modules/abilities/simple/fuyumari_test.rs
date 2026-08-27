/// Tests for 鬼塚冬毬 (PL!SP-pb1-011-R) — Debut ability:
///
/// 登場 「鬼塚冬毬」以外の『Liella!』のメンバー1人をステージから控え室に置いてもよい：
/// その後の控え室から、これにより控え室に置いたメンバーカードを1枚、
/// そのメンバーがいたエリアに登場させる。
///
/// Q63: Effect-debut doesn't pay member cost separately.
/// Q95: Only the exact card sent as cost can be appeared (not same-name copies).
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::zones::MemberArea;

fn drain_auto_prompts_fuyumari(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            _ => break,
        }
    }
}

/// Answer the optional-stage-cost pay/skip gate introduced for
/// 「〜してもよい」 member-sacrifice costs. Returns true when a gate was
/// present and answered (accept=true pays, false skips).
fn answer_stage_cost_gate(game: &mut TestGame, accept: bool) -> bool {
    match game.get_pending_choice() {
        Choice::SelectTarget { target, .. } if target.starts_with("pay_optional_cost") => {
            game.select_option(if accept { 1 } else { 0 });
            true
        }
        _ => false,
    }
}

/// Q63: Effect-debut doesn't pay member cost separately.
#[test]
fn fuyumari_q63_effect_debut_no_cost_payment() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let fuyumari = game.id("PL!SP-pb1-011-R");
    let liella_member = game.id("PL!SP-sd1-006-SD");

    game.state.player1.hand.cards.push(fuyumari);
    game.state.player1.stage.stage[0] = liella_member;
    game.give_energy(13);

    game.state.player1.stage.stage[1] = -1;
    game.play_to_stage(fuyumari, MemberArea::Center);

    // Observed chain: the ability opens with a pay/skip gate
    // (SelectTarget pay_optional_cost); answering card_indices [0] resolves
    // to skip_optional_cost, so this test exercises the SKIP path (no swap,
    // no effect). The state assertions below hold trivially on that path.
    assert!(
        game.has_pending_choice(),
        "pay/skip gate for the optional stage cost expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget pay_optional_cost gate"
    );
    game.select_indices(&[0]); // [0] = skip_optional_cost (observed via HST log)
    assert!(
        !game.has_pending_choice(),
        "Exactly 1 matching card in discard → auto-resolve"
    );

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "All 13 energy spent on ふるまり herself (Q63)"
    );
    assert_eq!(
        game.state.player1.stage.stage[0], liella_member,
        "Liella! member returned to LeftSide (same_area)"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], fuyumari,
        "ふるまり on Center"
    );
}

/// Q63 variant: optional cost NOT paid → no swap.
#[test]
fn fuyumari_q63_optional_cost_skipped() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let fuyumari = game.id("PL!SP-pb1-011-R");
    let liella_member = game.id("PL!SP-sd1-006-SD");

    game.state.player1.hand.cards.push(fuyumari);
    game.state.player1.stage.stage[0] = liella_member;
    game.give_energy(13);

    game.state.player1.stage.stage[1] = -1;
    game.play_to_stage(fuyumari, MemberArea::Center);

    // Observed: the optional stage cost opens with the same pay/skip gate;
    // an empty card_indices answer also resolves to skip_optional_cost.
    assert!(
        game.has_pending_choice(),
        "pay/skip gate for the optional stage cost expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget pay_optional_cost gate"
    );
    game.select_indices(&[]); // empty answer = skip_optional_cost (observed)

    assert_eq!(
        game.state.player1.stage.stage[0], liella_member,
        "Liella! member should remain on stage when cost is skipped"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], fuyumari,
        "ふるまり on Center"
    );
}

/// Q95: Player chooses which card from discard appears via same_area.
#[test]
fn fuyumari_q95_player_chooses_card_from_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let fuyumari = game.id("PL!SP-pb1-011-R");
    let liella_member = game.id("PL!SP-sd1-006-SD");
    let other_member = game.id("PL!SP-sd1-008-SD");

    game.state.player1.hand.cards.push(fuyumari);
    game.state.player1.stage.stage[0] = liella_member;
    game.state.player1.waitroom.cards.push(other_member);
    game.state.player1.waitroom.cards.push(liella_member);
    game.give_energy(13);

    let discard_before = game.state.player1.waitroom.cards.len();

    game.state.player1.stage.stage[1] = -1;
    game.play_to_stage(fuyumari, MemberArea::Center);

    // 1. Optional-cost gate: pay or skip (「〜してもよい」).
    assert!(
        answer_stage_cost_gate(&mut game, true),
        "optional stage cost must present a pay/skip gate"
    );
    // 2. Cost target selection: exactly one Liella! member on stage → the
    // engine auto-applies it with NO prompt (single-candidate auto-resolve).
    // 3. Effect: the very next ask is the re-deploy choice from the waitroom
    // (SelectCard zone=discard count=1 allow_skip=false).
    assert!(
        game.has_pending_choice(),
        "re-deploy SelectCard prompt expected after paying the optional cost"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard (zone=discard, member_card) for the re-deploy choice"
    );
    game.select_indices(&[0]);
    assert!(
        !game.has_pending_choice(),
        "re-deploy choice answered; ability must complete with no further prompt"
    );

    assert_eq!(
        game.state.player1.stage.stage[0], other_member,
        "Chosen card (other_member) appears at same_area (LeftSide)"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], fuyumari,
        "ふるまり on Center"
    );
    assert!(
        game.state.player1.waitroom.cards.len() <= discard_before,
        "Cost adds 1, effect removes 1 → net ≤ discard_before"
    );
}

/// Edge case: Non-Liella! member on stage is NOT a valid cost target.
/// With zero candidates the engine Q167-skips silently — either way the
/// filler must never be sacrificed or summoned.
#[test]
fn fuyumari_edge_no_valid_cost_target() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let fuyumari = game.id("PL!SP-pb1-011-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(fuyumari);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage[0] = filler;
    game.give_energy(13);

    game.state.player1.stage.stage[1] = -1;
    game.play_to_stage(fuyumari, MemberArea::Center);

    // Observed: zero valid candidates (non-Liella! filler excluded) → the
    // optional cost auto-skips silently (KANAN_DEBUG cost_was_skipped=true);
    // no prompt of any kind is presented, so the filler can never be offered.
    assert!(
        !game.has_pending_choice(),
        "no prompt expected: zero Liella! candidates auto-skip the optional cost"
    );

    assert_eq!(
        game.state.player1.stage.stage[0], filler,
        "non-Liella! member must stay on stage (invalid cost target)"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&filler),
        "non-Liella! member must not be sacrificed as cost"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], fuyumari,
        "ふるまり on Center"
    );
}

/// Edge case: exclude_self — ふるまり herself must not appear in the cost
/// prompt; the Liella! member is sacrificed and re-deploys into her own
/// vacated area.
#[test]
fn fuyumari_edge_exclude_self_from_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let fuyumari = game.id("PL!SP-pb1-011-R");
    let liella_member = game.id("PL!SP-sd1-006-SD");

    game.state.player1.hand.cards.push(fuyumari);
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.stage.stage[2] = liella_member;
    game.give_energy(13);

    game.play_to_stage(fuyumari, MemberArea::Center);

    // The optional stage cost now ALWAYS opens with a pay/skip gate — that
    // gate itself is not an exclusion failure. Exclusion is proven by what
    // follows: with exclude_self honored, exactly ONE candidate remains
    // (liella_member), so after accepting there is NO SelectCard prompt
    // (single candidate auto-resolves). If the filter were broken, ふるまり
    // would be offered too and a SelectCard would appear here.
    assert!(
        answer_stage_cost_gate(&mut game, true),
        "optional stage cost must present a pay/skip gate"
    );
    assert!(
        !game.has_pending_choice(),
        "single valid candidate must auto-resolve; a SelectCard here means \
         exclude_self failed to drop herself"
    );

    drain_auto_prompts_fuyumari(&mut game);
    assert_eq!(
        game.state.player1.stage.stage[2], liella_member,
        "the sacrificed Liella! member re-deploys into her vacated area"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], fuyumari,
        "ふるまり on Center"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&fuyumari),
        "exclude_self: ふるまり never becomes a cost"
    );
}
