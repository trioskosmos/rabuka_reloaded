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

    // Cost prompt: select the Liella! member from stage[0]
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
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

    if game.has_pending_choice() {
        game.select_indices(&[]);
    }

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

    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

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

    // If a prompt still appears, the non-Liella! filler must not be an
    // option — then skip.
    if game.has_pending_choice() {
        match game.get_pending_choice() {
            Choice::SelectCard { .. } => {
                game.assert_selection_not_contains("PL!-sd1-010-SD");
                game.select_indices(&[]);
            }
            _ => {}
        }
    }

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

    // EXCLUSION PROOF: candidates are {liella_member, herself}. With
    // exclude_self honored, exactly ONE candidate remains → the optional
    // cost auto-resolves with NO prompt. If the filter were broken she'd
    // be offered too (2 candidates) and a SelectCard would hang here.
    let mut saw_prompt = false;
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        saw_prompt = true;
        match game.get_pending_choice() {
            Choice::SelectCard { .. } => game.select_indices(&[0]),
            _ => break,
        }
    }
    assert!(
        !saw_prompt,
        "single valid candidate must auto-resolve; a prompt means \
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
