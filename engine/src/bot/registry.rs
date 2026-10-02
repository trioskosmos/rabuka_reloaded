//! Central bot registry: version dispatch without renames.
//!
//! Adding a new bot version is now three steps and NO renames:
//!   1. add `strategy_v8.rs` with the generic entry points
//!      `choose_action` / `choose_live_set` / `choose_mulligan`
//!      (keep any versioned names inside the module if you like);
//!   2. add a `V8` variant below;
//!   3. add three dispatch lines in the `impl` block.
//!
//! `bot_arena.rs` (and any future harness) calls only this registry, so it
//! never names a versioned function directly.

use crate::card::CardDatabase;
use crate::game_setup::{Action, ActionType};
use crate::game_state::GameState;

use super::{conductor, strategy, strategy_v2, strategy_v3, strategy_v4, strategy_v5, strategy_v6, strategy_v7, strategy_v8};

/// Every bot the arena can field. The string form is the CLI name.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BotKind {
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
    /// v7 with the ISMCTS refinement forced off — the ablation seat for a
    /// mirror `v7 vs v7plain`. Same build, same heuristics, one variable.
    V7Plain,
    /// The ISMCTS rollout policy played as a real bot. This is how you grade
    /// the thing a leaf is measured with, instead of assuming it is good.
    V7Rollout,
    V8,
    /// v7's Main phase with v8's live-card set. CLI name `v9`.
    ///
    /// This exists because a crossover measurement said so, not because a
    /// design said so. Swapping one half of v8 for v7's, paired, at n=1200:
    ///
    ///   v8 Main  + v8 live   43.7%   (v8 as shipped)
    ///   v7 Main  + v8 live   53.9%   <- recovers ~10 points
    ///   v8 Main  + v7 live   40.7%   <- v7's live set is worse than v8's
    ///
    /// So the entire deficit is in the Main phase and v8's live set is an asset,
    /// not a liability — which is the opposite of what §6 of the handoff doc
    /// assumed, and it explains why ten leaf-term experiments all failed: they
    /// were tuning a live set that was already better than the thing it was
    /// meant to replace.
    ///
    /// As a first-class kind rather than an env var because `--env` is
    /// process-global: a composite needs to be playable on ONE seat against a
    /// pure opponent, and a switch cannot express that.
    V9,
    /// v8's Main phase **with the v7 feature-delta term vector enabled**,
    /// plus v8's live-card set. CLI name `v10`.
    ///
    /// This exists because the two best components in the project are in
    /// different bots. The term vector took v8's Main from 42.5% to **47.9%**
    /// (seed 11) and **47.5%** (holdout seed 23), both p=0.00000 — the largest
    /// confirmed change in the file, and it closed about half of v8's ~10-point
    /// deficit to v7. Separately, the crossover showed v8's live set beats v7's
    /// by roughly three points.
    ///
    /// `v9` is v7's Main plus v8's live set, and it is the strongest bot so far
    /// at 57.3% / 57.6%. `v10` is the other combination: v8's improved Main plus
    /// v8's live set. Neither dominates a priori — v7's Main is still ahead of
    /// v8's by a few points, and v8's live set is ahead of v7's by a few — so
    /// this is the cell that says which half of v8 is worth keeping.
    ///
    /// The terms are **on by default here** and that is the whole point of the
    /// kind: they are thirteen switches whose combined effect is confirmed on
    /// two seeds, and turning them on by default in a bot whose Main phase is
    /// the point of the exercise is not the same risk as turning them on for
    /// stock `v8`, which is a different bot with a different baseline.
    V10,
    /// v9 with v8's live-card set allowed to set TWO lives instead of one.
    /// CLI name `v11`. **A measured null - nothing selects it.**
    ///
    /// Kept because it is the cleanest measurement in the file of why the
    /// headline metric cannot be moved. v9 places ~0.43 cards per live phase,
    /// 86% of what rule 8.4.7 allows a single side, and the backlog has been
    /// ranked against that number for weeks. Setting two lives raises it - both
    /// seats, by 0.002 to 0.004 per phase, two cells significant - and changes
    /// the win rate by nothing:
    ///
    /// ```text
    ///    v11 vs v9   2004 - 1996   n=4000    v11 = 50.1%
    ///    v9 vs v11   2041 - 1959   n=4000    v11 = 47.6%
    ///    v11 mirror vs v9 base, seeds 101 / 31337:  -0.67 pp p=0.20 | +0.10 pp p=0.90
    /// ```
    ///
    /// Placements are a zero-sum race to three (1.2.1.1), so a policy that
    /// raises both seats' placement rate lengthens the race and leaves the win
    /// rate alone. A metric both seats can raise by the same amount is not the
    /// objective. `v8_live` carries the full write-up.
    ///
    /// A kind rather than an env var for the reason `V9` gives: `--env` is
    /// process-global, so a switch can never be played on ONE seat against a
    /// pure opponent - and that head-to-head is what refuted this, where the
    /// mirror had it at +1.10 pp and p=0.036.
    V11,
    Neural,
    Conductor,
    Random,
}

impl BotKind {
    /// All CLI names, for help text.
    pub const ALL: &'static [&'static str] = &[
        "v1",
        "v2",
        "v3",
        "v4",
        "v5",
        "v6",
        "v7",
        "v7plain",
        "v7rollout",
        "v8",
        "v9",
        "v10",
        "v11",
        "neural",
        "conductor",
        "random",
    ];

    pub fn parse(s: &str) -> Self {
        match s {
            "v1" => BotKind::V1,
            "v2" => BotKind::V2,
            "v3" => BotKind::V3,
            "v4" => BotKind::V4,
            "v5" => BotKind::V5,
            "v6" => BotKind::V6,
            "v7" => BotKind::V7,
            "v7plain" => BotKind::V7Plain,
            "v7rollout" => BotKind::V7Rollout,
            "v8" => BotKind::V8,
            "v9" => BotKind::V9,
            "v10" => BotKind::V10,
            "v11" => BotKind::V11,
            "neural" => BotKind::Neural,
            "conductor" => BotKind::Conductor,
            _ => BotKind::Random,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            BotKind::V1 => "v1",
            BotKind::V2 => "v2",
            BotKind::V3 => "v3",
            BotKind::V4 => "v4",
            BotKind::V5 => "v5",
            BotKind::V6 => "v6",
            BotKind::V7 => "v7",
            BotKind::V7Plain => "v7plain",
            BotKind::V7Rollout => "v7rollout",
            BotKind::V8 => "v8",
            BotKind::V9 => "v9",
            BotKind::V10 => "v10",
            BotKind::V11 => "v11",
            BotKind::Neural => "neural",
            BotKind::Conductor => "conductor",
            BotKind::Random => "random",
        }
    }

    /// Main-phase (and any other action-phase) decision.
    /// `v2_policy` / `plan` are only read by the generations that need them;
    /// every other bot ignores them, so the arena can pass shared values.
    pub fn choose_action(
        self,
        gs: &GameState,
        actions: &[Action],
        me: u8,
        v2_policy: &strategy_v2::V2Policy,
        plan: &strategy_v3::V3Plan,
    ) -> Action {
        let _ = v2_policy;
        // `PROBE=<bot>` runs this bot's Main-phase policy and v8's on the same
        // state and logs every disagreement. It lives here rather than inside a
        // strategy so it works for **any** bot — the first version sat in
        // `strategy_v8`'s entry, so it never fired for `v9`, the bot that
        // actually gets played, because v9's Main phase *is* v7's. An
        // instrument that cannot see the shipping bot points at a research
        // branch.
        //
        // Comparing v9 against v8 is the useful comparison at the current
        // state: v9 borrows v7's Main, so it inherits v7's decisions wholesale
        // and any divergence is in the LIVE SET, which is the one component v9
        // still owns. Against v7 the Main phases agree by construction and only
        // the reference's own second opinion is visible.
        //
        // UNTRACED by construction — an `eprintln!` in the policy path, and
        // `BOT_STRATEGY.md` section 8.4 records that logging changes outcomes.
        // It finds defects; it never reports numbers.
        if std::env::var("PROBE").as_deref() == Ok(self.name()) {
            if gs.current_phase == crate::types::Phase::Main && !gs.has_pending_choice() {
                let db = &gs.card_database;
                let ours = strategy_v7::choose_action_v7(gs, actions, me);
                let theirs = super::v8_main::choose_action_v8(gs, actions, me);
                if ours.action_type != theirs.action_type
                    || ours.description != theirs.description
                {
                    eprintln!(
                        "PROBE t={} me={} bl={} cost={} h={} lives={} hand={} deck={} | \
                         opp: bl={} h={} | {}(main)={:?} v8={:?}",
                        gs.turn_number,
                        me,
                        super::v8_model::active_blades(gs, me, db),
                        super::v8_model::stage_cost(gs, me, db),
                        super::v8_model::supply_hearts(gs, me, db),
                        super::v8_model::lives_in_hand(gs.seat_player(me), db),
                        gs.seat_player(me).hand.cards.len(),
                        gs.seat_player(me).main_deck.cards.len(),
                        super::v8_model::active_blades(gs, 1 - me, db),
                        super::v8_model::supply_hearts(gs, 1 - me, db),
                        self.name(),
                        ours.action_type,
                        theirs.action_type,
                    );
                }
            }
        }
        match self {
            BotKind::V1 => strategy::choose_action_heuristic(gs, actions, me),
            BotKind::V2 => strategy_v2::choose_action_heuristic_v2(gs, actions, me),
            BotKind::V3 => strategy_v3::choose_action_heuristic_v3(gs, actions, me, plan),
            BotKind::V4 => strategy_v4::choose_action(gs, actions, me),
            BotKind::V5 => strategy_v5::choose_action(gs, actions, me),
            BotKind::V6 => strategy_v6::choose_action(gs, actions, me),
            BotKind::V7 => strategy_v7::choose_action(gs, actions, me),
            BotKind::V7Plain => super::v7_main::choose_action_plain(gs, actions, me),
            BotKind::V7Rollout => super::v7_ismcts::choose_rollout_policy(gs, actions, me),
            // v9: v7's Main, v8's live set. See the enum for the measurement
            // that decided this. The mulligan is v7's too — the crossover
            // localised the deficit to the Main phase, and borrowing v8's
            // forward-walk mulligan here would re-import the thing being
            // removed.
            BotKind::V9 => strategy_v7::choose_action(gs, actions, me),
            // v11 is v9 with a wider live set, so its Main phase and mulligan are
            // v9's unchanged. Isolating the live-set width is the whole point: a
            // candidate that also moved the Main phase could not be read.
            BotKind::V11 => strategy_v7::choose_action(gs, actions, me),
            // v10 is v8's Main, which becomes the confirmed configuration when
            // `V8_ALL_TERMS` is set. It is not forced on here: a bot's identity
            // should not depend on whether the caller remembered an env var, and
            // a silently-different bot under a name that promises otherwise is
            // the exact class of bug this file's log is full of. The run line
            // echoes the switch, so a `v10` without it is visible.
            BotKind::V10 => strategy_v8::choose_action(gs, actions, me),
            BotKind::V8 => {
                // `V8_BORROW_MAIN=v7` hands the Main phase to v7 and leaves
                // v8's live set and mulligan in place.
                if std::env::var("V8_BORROW_MAIN").as_deref() == Ok("v7") {
                    strategy_v7::choose_action(gs, actions, me)
                } else {
                    strategy_v8::choose_action(gs, actions, me)
                }
            }
            BotKind::Neural => actions.first().cloned().unwrap_or(Action {
                description: "pass".into(),
                description_ja: None,
                action_type: ActionType::Pass,
                parameters: None,
                selected: None,
            }),
            BotKind::Conductor => conductor::choose_main_conductor(gs, actions, me),
            BotKind::Random => {
                // Deterministic harness random is handled by the caller; fall
                // back to the first action so this stays total.
                actions.first().cloned().unwrap_or_else(|| Action {
                    description: "pass".into(),
                    description_ja: None,
                    action_type: crate::game_setup::ActionType::Pass,
                    parameters: None,
                    selected: None,
                })
            }
        }
    }

    /// Live-card-set decision.
    pub fn choose_live_set(
        self,
        gs: &GameState,
        actions: &[Action],
        db: &CardDatabase,
        v2_policy: &strategy_v2::V2Policy,
        plan: &strategy_v3::V3Plan,
    ) -> Action {
        match self {
            BotKind::V1 => strategy::choose_live_set_action(gs, actions, db),
            BotKind::V2 => strategy_v2::choose_live_set_action_v2(gs, actions, db, v2_policy),
            BotKind::V3 => strategy_v3::choose_live_set_action_v3(gs, actions, db, v2_policy, plan),
            BotKind::V4 => strategy_v4::choose_live_set(gs, actions, db),
            BotKind::V5 => strategy_v5::choose_live_set(gs, actions, db),
            BotKind::V6 => strategy_v6::choose_live_set(gs, actions, db),
            BotKind::V7 => {
                // The reciprocal crossover: v7's proven Main with v8's live
                // set. If this loses, the live set is not merely a v8 liability,
                // it is worse than the thing it replaced, and the honest
                // conclusion is that v8's live set is not ready to ship at all.
                if std::env::var("V7_BORROW_LIVE").as_deref() == Ok("v8") {
                    strategy_v8::choose_live_set(gs, actions, db)
                } else {
                    strategy_v7::choose_live_set(gs, actions, db)
                }
            }
            BotKind::V7Plain => strategy_v7::choose_live_set(gs, actions, db),
            BotKind::V7Rollout => strategy_v7::choose_live_set(gs, actions, db),
            BotKind::V8 => {
                // The other half of the crossover. See `V8_BORROW_MAIN` for
                // why this is worth two runs.
                if std::env::var("V8_BORROW_LIVE").as_deref() == Ok("v7") {
                    strategy_v7::choose_live_set(gs, actions, db)
                } else {
                    strategy_v8::choose_live_set(gs, actions, db)
                }
            }
            // The v8 half of v9, and the whole reason v9 exists: the crossover
            // showed v8's live set beating v7's (v8 Main + v7 live = 40.7%
            // versus v8 Main + v8 live = 43.7%).
            //
            // `V9_LIVE_ROLLOUT=v7` swaps v8's *model* of the live set for v7's
            // *engine-executing* one, which clones the state, commits the
            // candidate set and lets the rules decide. That is the only
            // remaining way to stop modelling the check and start computing it,
            // and the live set is where the model is most load-bearing: v8's
            // leaf prices "will I place later" with one function and the live
            // set prices "should I set this" with a different one, and they
            // disagree on 21.4% of live phases.
            BotKind::V9 => {
                if std::env::var("V9_LIVE_ROLLOUT").as_deref() == Ok("v7") {
                    strategy_v7::choose_live_set_rollout(gs, actions, db)
                } else {
                    strategy_v8::choose_live_set(gs, actions, db)
                }
            }
            // v10 keeps the v8 live set: the crossover showed it is worth about
            // three points over the v7 one, and it is the other half of why this
            // bot exists.
            BotKind::V10 => strategy_v8::choose_live_set(gs, actions, db),
            // The one difference from `V9`. See the enum.
            BotKind::V11 => strategy_v8::choose_live_set_wide(gs, actions, db),
            BotKind::Neural => actions.first().cloned().unwrap_or(Action {
                description: "pass".into(),
                description_ja: None,
                action_type: ActionType::Pass,
                parameters: None,
                selected: None,
            }),
            BotKind::Conductor => conductor::choose_live_set_conductor(gs, actions, db),
            BotKind::Random => actions
                .first()
                .cloned()
                .unwrap_or_else(|| strategy::choose_live_set_action(gs, actions, db)),
        }
    }

    /// Mulligan decision.
    pub fn choose_mulligan(self, gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
        match self {
            BotKind::V1 | BotKind::Random => actions
                .iter()
                .find(|a| {
                    matches!(
                        a.action_type,
                        crate::game_setup::ActionType::ConfirmMulligan
                            | crate::game_setup::ActionType::SkipMulligan
                    )
                })
                .or_else(|| actions.first())
                .cloned()
                .unwrap_or_else(|| Action {
                    description: "skip".into(),
                    description_ja: None,
                    action_type: crate::game_setup::ActionType::SkipMulligan,
                    parameters: None,
                    selected: None,
                }),
            BotKind::V2 => strategy_v2::choose_mulligan_action_v2(gs, actions, db),
            BotKind::V3 => strategy_v3::choose_mulligan_action_v3(gs, actions, db),
            BotKind::V4 => strategy_v4::choose_mulligan(gs, actions, db),
            BotKind::V5 => strategy_v5::choose_mulligan(gs, actions, db),
            BotKind::V6 => strategy_v6::choose_mulligan(gs, actions, db),
            BotKind::V7 => strategy_v7::choose_mulligan(gs, actions, db),
            BotKind::V7Plain => strategy_v7::choose_mulligan(gs, actions, db),
            BotKind::V7Rollout => strategy_v7::choose_mulligan(gs, actions, db),
            BotKind::V8 => strategy_v8::choose_mulligan(gs, actions, db),
            // The crossover localised the deficit to the Main phase and never
            // crossed the mulligan, so it is untested rather than exonerated.
            // v7's is a static curve rule; v8's enumerates every legal discard
            // set and scores it with a five-snapshot forward walk. One of those
            // is right and the difference is worth one cell.
            BotKind::V9 => {
                if std::env::var("V9_MULLIGAN").as_deref() == Ok("v8") {
                    strategy_v8::choose_mulligan(gs, actions, db)
                } else {
                    strategy_v7::choose_mulligan(gs, actions, db)
                }
            }
            BotKind::Neural => actions.first().cloned().unwrap_or(Action {
                description: "skip".into(),
                description_ja: None,
                action_type: ActionType::SkipMulligan,
                parameters: None,
                selected: None,
            }),
            BotKind::Conductor => strategy_v4::choose_mulligan(gs, actions, db),
            // v10 keeps the v7 mulligan. The forward-walk one measured -3.4 from
            // v8 and -5.8 from v9, so it is worse on two different base bots and
            // is not a context problem.
            BotKind::V10 => strategy_v7::choose_mulligan(gs, actions, db),
            BotKind::V11 => strategy_v7::choose_mulligan(gs, actions, db),
        }
    }
}
