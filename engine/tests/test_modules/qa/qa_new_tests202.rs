/// Tests for Q202/Q201/Q200 — deploy from hand via debut ability, then
/// the deployed card's own debut ability fires separately.
///
/// All three deployers share the pattern:
///   登場 E2支払ってもよい：手札からコスト4以下の自分と同名のメンバーを1枚登場させる。
///
/// The ruling under test: the DEPLOYED card's own 登場 ability resolves
/// normally. Each test therefore PAYS the deployed debut's cost and asserts
/// its observable effect — target-on-stage alone proves nothing.
///
/// Deployed cards:
///   Q202 ミア・テイラー PL!N-pb1-023-R deploys PL!N-PR-013-PR
///        （登場 手札1枚控え室に置いてもよい：デッキ上から3枚見て1枚加え残り控え室）
///   Q201 宮下愛     PL!N-pb1-017-R deploys PL!N-bp4-005-R
///        （登場 手札1枚控え室に置いてもよい：相手のコスト4以下のメンバーを2人までウェイトにする）
///   Q200 上原歩夢   PL!N-pb1-013-R deploys PL!N-sd1-013-SD
///        （登場 カードを1枚引き、手札を1枚控え室に置く。← mandatory）
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

/// Play the deployer, pay its optional 2E, deploy the SAME-NAME low-cost
/// target from hand, and place it into the first offered area.
/// Hard-asserts every prompt along the way.
fn deploy_target(
    game: &mut TestGame,
    deployer_no: &str,
    target_no: &str,
) -> (i16, i16) {
    let deployer = game.id(deployer_no);
    let target = game.id(target_no);
    let fodder = game.new_id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(deployer);
    game.state.player1.hand.cards.push(target);
    // A SPARE card that survives deployment: the deployed debut's own
    // costs (optional or mandatory discards) need an eligible hand card,
    // otherwise the engine correctly skips the whole chain (Q167).
    let spare = game.new_id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(spare);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(fodder);
    }
    game.give_energy(30);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(deployer),
        None,
        Some(MemberArea::Center),
        Some(false),
    )
    .expect("play deployer to stage");

    // Deployer's debut: optional 2E → PAY.
    // Observed: SelectTarget pay_optional_cost gate is offered.
    match game.get_pending_choice() {
        Choice::SelectTarget { target: t, .. }
            if t == "pay_optional_cost:skip_optional_cost"
                || t == "conditional_optional" =>
        {
            game.select_option(1);
        }
        other => panic!(
            "deployer's optional 2E debut must offer the SelectTarget payment gate, got {:?}",
            other
        ),
    }

    // Select the SAME-NAME target from hand: OBSERVED — with exactly one
    // same-name candidate the engine auto-selects; NO hand prompt appears.
    // The next pending choice is already the placement SelectPosition.

    // Place into the first offered area.
    // Observed: exactly one SelectPosition prompt appears after deployment.
    assert!(
        game.has_pending_choice(),
        "position choice for the deployed member expected"
    );
    assert!(
        matches!(
            game.get_pending_choice(),
            Choice::SelectPosition { .. }
        ),
        "expected SelectPosition prompt"
    );
    game.select_generated(0);
    // Whatever is pending NOW belongs to the DEPLOYED card's own debut
    // (its cost prompt etc.) — the caller drives and verifies it. Eating
    // it here would hide the very behavior these tests exist to check.
    //
    // The debut was ENQUEUED (see [DEBUT_CHAIN] logs) but surfaces as a
    // prompt only after the queue is pumped again — exactly what the real
    // game loop does between player actions.
    if !game.has_pending_choice() {
        let pid = game.state.player1.id.clone();
        game.state.trigger_auto_abilities_for_player(&pid);
        game.state.process_pending_auto_abilities(&pid);
    }
    if !game.has_pending_choice() {
        game.dump_queue();
    }
    assert!(
        game.has_pending_choice(),
        "deployed card's own debut should be waiting for input: {}",
        game.pending_choice_summary()
    );
    (target, spare)
}

// ── Q200: 上原歩夢 deploys 歩夢 whose MANDATORY debut draws 1 / discards 1 ──
#[test]
fn q200_uehara_deploy_triggers_mandatory_draw_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Deterministic deck so the drawn card is known: top = marker.
    let marker = game.id("PL!S-bp2-001-R");
    crate::helpers::put_on_deck_top(&mut game, 0, marker);

    // Deploy. The deployed 歩夢's MANDATORY debut (draw 1, discard 1)
    // fires inside deploy_target; its discard is driven by identity below
    // via the leftover prompt (or auto-resolves on a single candidate).
    let (target, spare) = deploy_target(&mut game, "PL!N-pb1-013-R", "PL!N-sd1-013-SD");
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        if matches!(game.get_pending_choice(), Choice::SelectCard { .. }) {
            // Mandatory discard — sacrifice the spare deterministically.
            let spos = game
                .state
                .player1
                .hand
                .cards
                .iter()
                .position(|&c| c == spare);
            match spos {
                Some(p) => game.select_indices(&[p]),
                None => game.select_indices(&[0]),
            }
        } else {
            break;
        }
    }

    // Ruling: the deployed card's mandatory debut DID run.
    assert!(
        !game.state.player1.main_deck.cards.contains(&marker),
        "the marker left the deck (mandatory draw)"
    );
    assert!(
        game.state.player1.hand.cards.contains(&marker),
        "the drawn marker stays in hand"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&spare),
        "the mandatory discard was paid from hand"
    );
    assert!(game.state.player1.stage.stage.contains(&target));
}

// ── Q201: 宮下愛 deploys 愛 whose OPTIONAL debut waits 2 opponent ≤4s ──
#[test]
fn q201_miyashita_deploy_triggers_opponent_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Two opponent members at cost ≤ 4 waiting to be locked down.
    let opp_a = game.id("PL!-sd1-010-SD"); // cost 4
    let opp_b = game.new_id("PL!-sd1-010-SD"); // cost 4, distinct copy
    game.state.player2.stage.stage = [opp_a, opp_b, -1];

    let (target, spare) = deploy_target(&mut game, "PL!N-pb1-017-R", "PL!N-bp4-005-R");

    // Deployed 愛's OPTIONAL debut: pay 1 hand discard → wait up to 2
    // opponent cost≤4 members. It presents as a direct skippable
    // SelectCard over the hand — PAY with the spare.
    match game.get_pending_choice() {
        Choice::SelectCard { zone, allow_skip, .. } if zone == "hand" => {
            assert!(*allow_skip, "cost is optional");
            let spos = game
                .state
                .player1
                .hand
                .cards
                .iter()
                .position(|&c| c == spare)
                .expect("spare still in hand");
            game.select_indices(&[spos]);
        }
        other => panic!("expected skippable hand SelectCard, got {:?}", other),
    }

    // After paying, the wait-2-members effect runs. Pump the queue (the
    // real loop does this between actions) and answer any member-selection
    // prompt by taking BOTH opponents.
    if !game.has_pending_choice() {
        let pid = game.state.player1.id.clone();
        game.state.trigger_auto_abilities_for_player(&pid);
        game.state.process_pending_auto_abilities(&pid);
    }
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectCard { zone, .. } if zone == "stage" => {
                let n = game.state.player1.stage.stage.iter().filter(|&&c| c != -1).count();
                let idxs: Vec<usize> = (0..n).collect();
                game.select_indices(&idxs);
            }
            _ => break,
        }
    }

    // Ruling: BOTH opponent members (≤2, cost≤4) end up WAITED.
    assert_eq!(
        game.state.mods.get_orientation_modifier(opp_a),
        Some("wait"),
        "opponent member A waited by the deployed debut"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(opp_b),
        Some("wait"),
        "opponent member B waited by the deployed debut"
    );
    assert!(game.state.player1.stage.stage.contains(&target));
}

/// Negative control for Q201: DECLINING the deployed debut's optional cost
/// leaves the opponent board untouched.
#[test]
fn q201_miyashita_decline_leaves_opponents_active() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let opp_a = game.id("PL!-sd1-010-SD"); // cost 4
    let opp_b = game.new_id("PL!-sd1-010-SD"); // cost 4, distinct copy
    game.state.player2.stage.stage = [opp_a, opp_b, -1];

    let (target, _spare) = deploy_target(&mut game, "PL!N-pb1-017-R", "PL!N-bp4-005-R");

    // DECLINE the deployed debut's optional cost (empty selection = skip).
    match game.get_pending_choice() {
        Choice::SelectCard { zone, allow_skip, .. } if zone == "hand" => {
            assert!(*allow_skip, "cost is optional");
            game.select_indices(&[]);
        }
        other => panic!("expected skippable hand SelectCard, got {:?}", other),
    }

    assert_eq!(game.state.mods.get_orientation_modifier(opp_a), None);
    assert_eq!(game.state.mods.get_orientation_modifier(opp_b), None);
    assert!(game.state.player1.stage.stage.contains(&target));
}

// ── Q202: ミア・テイラー deploys ミア(PR) whose OPTIONAL debut looks at top 3 ──
#[test]
fn q202_mia_deploy_look_top3_adds_one_rest_to_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Seed the deck: after deployment the top three cards are L1, L2, L3.
    let l1 = game.id("PL!S-bp2-001-R");
    let l2 = game.id("PL!S-bp2-002-R");
    let l3 = game.id("PL!S-bp2-004-R");
    // insert(0) puts on TOP — push in reverse so L1 ends on top.
    crate::helpers::put_on_deck_top(&mut game, 0, l3);
    crate::helpers::put_on_deck_top(&mut game, 0, l2);
    crate::helpers::put_on_deck_top(&mut game, 0, l1);

    let (target, spare) = deploy_target(&mut game, "PL!N-pb1-023-R", "PL!N-PR-013-PR");

    // Deployed ミア's OPTIONAL debut: pay 1 discard → look top 3 →
    // add 1, rest to waitroom. Direct skippable hand SelectCard — PAY
    // with the spare.
    match game.get_pending_choice() {
        Choice::SelectCard { zone, allow_skip, .. } if zone == "hand" => {
            assert!(*allow_skip, "cost is optional");
            let spos = game
                .state
                .player1
                .hand
                .cards
                .iter()
                .position(|&c| c == spare)
                .expect("spare still in hand");
            game.select_indices(&[spos]);
        }
        other => panic!("expected skippable hand SelectCard, got {:?}", other),
    }
    // Look-and-select: add ONE of the three looked cards (take L1).
    let mut added = false;
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectCard { count, allow_skip, .. } => {
                if *count >= 1 || !*allow_skip {
                    game.select_indices(&[0]); // top of looked set
                    added = true;
                } else {
                    game.select_indices(&[]); // finalize any-number picker
                }
            }
            _ => break,
        }
    }
    assert!(added, "look-and-select must have added exactly one card");

    // Ruling effects, all observable (by identity — the helper's fodder
    // makes absolute deck lengths meaningless):
    assert!(
        !game.state.player1.main_deck.cards.contains(&l1)
            && !game.state.player1.main_deck.cards.contains(&l2)
            && !game.state.player1.main_deck.cards.contains(&l3),
        "all three looked-at cards left the deck"
    );
    assert!(
        game.state.player1.hand.cards.contains(&l1),
        "the chosen looked-at card was added to hand"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&l2)
            && game.state.player1.waitroom.cards.contains(&l3),
        "the remaining looked-at cards went to the waitroom"
    );
    assert!(game.state.player1.stage.stage.contains(&target));
}
