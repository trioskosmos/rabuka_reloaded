//! Full-round opponent-live-success flow (mission §both-players).
//!
//! Rules grounding: qa_data.json Q36 — ライブ成功時 abilities resolve at the
//! LIVE VICTORY DETERMINATION phase, after BOTH players' performances, before
//! the winner is decided. Therefore an ability whose condition reads
//! 「このターン、相手が…ライブを成功させていた場合」 must see the OTHER
//! seat's result relative to its OWNER — including when the owner is P2 and
//! the successful opponent is P1 (the first attacker).
//!
//! ⚠ ENGINE BUG #5 (open, diagnosed — see docs/TEST_HARDENING_PLAN_2026-08-26.md):
//! execute_live_victory_determination fires LiveSuccess triggers (source
//! ~243-355) BEFORE computing life verdicts (~403-495), the RESULT (~598-630)
//! and the per-seat success flags (~876-886). Any real-flow
//! opponent_live_success evaluation therefore sees stale state. Fixing requires
//! reordering that function (triggers must run after verdicts+flags but before
//! final scoring, because trigger bonuses feed the score comparison via
//! pX_extra). Synthetic-flag tests (strawberry_*) masked this completely.
//!
//! Organic setup (no synthetic flag injection):
//! - P1: 園田海未 PL!-sd1-013-SD staged; her base hearts EXACTLY fill
//!   僕らのLIFE 君とのLIFE (need heart01×1 + heart06×1) → success with ZERO
//!   surplus hearts.
//! - P2: owns Strawberry Trapper (needs own-stage Aqours heart05 total >= 4,
//!   provided via a pinned heart modifier — setup convenience) + stages
//!   高海千歌 PL!S-bp2-001-R whose hearts exactly fill 未来の僕らは知ってるよ.
//! - Both perform successfully; determination records per-seat results;
//!   P2's Trapper scores +2 because ITS opponent (P1) succeeded without
//!   excess hearts.

use crate::helpers::*;
use rabuka_engine::card::HeartColor;

const P1_MEMBER: &str = "PL!-sd1-007-SD"; // 東條希: exactly fills START:DASH!!
const P1_LIVE: &str = "PL!-sd1-019-SD"; // need {01:1,03:1,06:1}, score=1
const TRAPPER: &str = "PL!S-pb1-021-L";
const P2_MEMBER: &str = "PL!S-bp2-001-R"; // 千歌: {02:1,04:1,05:1} + injected 05s fill Trapper

#[test]
fn p2_owned_trapper_scores_from_p1_success_in_real_round() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let p1_member = g.id(P1_MEMBER);
    let p1_live = g.id(P1_LIVE);
    let trapper = g.id(TRAPPER);
    let p2_member = g.id(P2_MEMBER);
    let filler = g.id("PL!-sd1-010-SD");

    // Stages
    g.state.player1.stage.stage = [-1, p1_member, -1];
    g.state.player2.stage.stage = [-1, p2_member, -1];
    // P1's deck MUST be heart-less: the yell reveals deck-top cards and their
    // hearts join the performance pool — a member filler here would create
    // surplus and break the exact-fill requirement.
    let energy = g.id("LL-E-001-SD");
    for _ in 0..12 {
        g.state.player1.main_deck.cards.push(energy);
        g.state.player2.main_deck.cards.push(filler);
    }
    g.give_energy(15);
    // Lives are played from hand. P2 performs THE TRAPPER itself (its own
    // success is the trigger; the condition checks P1's result).
    g.state.player1.hand.cards.push(p1_live);
    g.state.player2.hand.cards.push(trapper);

    // Trapper gate setup: own-stage Aqours heart05 total >= 4 (pinned
    // convenience — under test here is the opponent-success wiring, not the
    // group/heart-count gate, which strawberry_test pins separately).
    g.state.mods.add_heart_modifier(
        p2_member,
        HeartColor::Heart05,
        4,
    );

    // Drive the real round: P1 Main → … → LiveCardSetP1.
    for _ in 0..5 {
        g.pass();
        while g.has_pending_choice() {
            g.select_indices(&[]);
        }
    }
    g.set_live_card(p1_live);

    // Next: LiveCardSetSecondAttacker → P2 performs the TRAPPER.
    g.pass();
    while g.has_pending_choice() {
        g.select_indices(&[]);
    }
    g.set_live_card(trapper);

    // Performances + victory determination. Drain prompts (yell reveals,
    // START:DASH!! look-3, etc.) between passes so nothing blocks phases.
    // Stop advancing the MOMENT determination records per-seat results:
    // crossing into the next turn resets them (Active-phase entry).
    for _ in 0..8 {
        while g.has_pending_choice() {
            g.select_indices(&[]);
        }
        if g.state.p1_live_success_this_turn || g.state.p2_live_success_this_turn {
            break;
        }
        g.pass();
    }
    while g.has_pending_choice() {
        g.select_indices(&[]);
    }

    // Capture at determination.
    let p1_won = g.state.p1_live_success_this_turn;
    let p1_no_excess = g.state.p1_live_success_no_excess;
    let p2_won = g.state.p2_live_success_this_turn;
    assert!(p1_won, "P1 must have succeeded (exact-fill live)");
    assert!(
        p1_no_excess,
        "P1 succeeded WITHOUT surplus hearts (exact fill)"
    );
    assert!(p2_won, "P2 must have succeeded as well");

    // ライブ成功時 abilities fire on the determination→next-phase transition
    // (Q36 timing) — keep cycling so they resolve before asserting the score.
    for _ in 0..3 {
        if !g.has_pending_choice() {
            g.pass();
        }
        while g.has_pending_choice() {
            g.select_indices(&[]);
        }
    }

    // The interaction payoff: P2's Trapper sees ITS opponent (P1) succeed
    // without excess → +2 score in P2's performance snapshot.
    let found = g
        .state
        .performance_snapshots
        .iter()
        .find_map(|s| s.lives.iter().find(|l| l.card_id == trapper))
        .unwrap_or_else(|| {
            let dump: Vec<String> = g
                .state
                .performance_snapshots
                .iter()
                .map(|s| {
                    format!(
                        "player={} lives={:?}",
                        s.player_id,
                        s.lives.iter().map(|l| l.card_id).collect::<Vec<_>>()
                    )
                })
                .collect();
            panic!("Trapper live missing from snapshots; have: {dump:?}");
        });
    assert_eq!(
        found.score - found.base_score,
        2,
        "P2-owned Trapper must gain +2 from P1's no-excess success"
    );
}
