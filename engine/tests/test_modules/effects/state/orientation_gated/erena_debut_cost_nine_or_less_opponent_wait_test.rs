//! 統堂英玲奈 `PL!-bp5-333-R` (cost 7) — her **登場** ability end to end.
//!
//! ```text
//! {{toujyou.png|登場}}このメンバーをウェイトにしてもよい：相手のステージにいる
//! コスト9以下のメンバー1人をウェイトにする。
//! {{jyouji.png|常時}}このメンバーがウェイト状態であるかぎり、{{heart_05.png|heart05}}を得る。
//! ```
//!
//! ## Why this file exists
//!
//! All 21 other tree references to Erena use her as a *fixture* for someone else's 常時 —
//! `waited_self_heart05_constant_test.rs`, `erena_self_wait_constant_heart05_test.rs`,
//! `wait_state_condition_constants_test.rs`, `per_other_group_member_constant_heart_test.rs`
//! and the rest. Every one of them hand-writes her orientation
//! (`add_orientation_modifier(erena, "wait")`) and calls `recalculate_constants()`. So the
//! 常時 half is well covered from both sides, and the **登場 half was executed by nothing**.
//!
//! ## How the ability actually resolves (this is the part that is easy to get wrong)
//!
//! The 登場 is a cost-then-effect pair: the cost is `change_state` with `self_cost: true`
//! and `optional: true`, and the effect is `change_state` on `target: opponent` with
//! `cost_limit: 9, cost_limit_operator: "<=", count: 1`.
//!
//! 1. `pay_cost_change_state` (`engine/src/ability/cost/handlers.rs:856`) sees
//!    `optional && !is_activation`, finds Erena herself as an active candidate, and emits
//!    the options-less `SelectTarget` pay/skip gate `pay_optional_cost:skip_optional_cost`.
//! 2. Paying runs `handle_optional_cost_payment` (`handlers.rs:1375`), which puts **Erena**
//!    to wait (`handlers.rs:1466-1489`) and then executes the entry effect.
//! 3. `execute_change_state` (`engine/src/ability/effects/change_state.rs:119`) filters the
//!    opponent's stage down to `cost <= 9`, drops members already in wait (Q137), and then
//!    decides whether to PROMPT at all:
//!
//!    ```rust
//!    let needs_prompt = !is_self_target
//!        && self.selection.cards.is_empty()
//        && ((max && !candidates.is_empty())
//!            || (!is_change_all && candidates.len() > count));   // change_state.rs:504
//!    ```
//!
//!    `candidates.len() > count` is the whole story. With **one** qualifying opponent member
//!    and `count == 1` there is nothing to choose between, so the engine applies the wait
//!    directly and **no member prompt ever opens**.
//!
//! That is why a board of `[cost 9, cost 10]` cannot be used to inspect "what was offered":
//! the cost-10 member is filtered out, the cost-9 member is the only candidate, and the
//! effect auto-resolves. The コスト9以下 bound is still fully observable there — through the
//! OUTCOME (cost 9 waited, cost 10 untouched), which is the stronger assertion anyway.
//! Reading the offered SET needs a board with at least two qualifying members so that
//! `candidates.len() > count` holds and the prompt opens; see
//! `the_offered_candidate_set_excludes_the_cost_ten_member`.
//!
//! A second trap: when `filtered_indices` is set, the JSON `selection_cards` array lists
//! **every** card in the zone, not the candidates, so the UI can grey the rest
//! (`engine/src/core/game_state/abilities.rs:2598-2601`). The candidate set is
//! `filtered_indices`, and those are STAGE POSITIONS, not card ids.
//!
//! ## The boundary
//!
//! `コスト9以下` is `<= 9`, so a member of cost **9** qualifies and one of cost **10** does
//! not. `PL!-bp5-017-N` (cost 9, no ability) is the exact-bound fixture and
//! `PL!-bp5-005-AR` (cost 10) is one above it. The cost-10 member carries an ability, which
//! is irrelevant to waiting. `PL!-sd1-010-SD` (cost 4) is the third qualifying fixture used
//! where two candidates are needed.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::game_modifiers::CardOrientation;
use rabuka_engine::zones::MemberArea;

const ERENA: &str = "PL!-bp5-333-R";
/// 小泉花陽, cost 9, no ability — the exact bound 「コスト9以下」.
const COST_9: &str = "PL!-bp5-017-N";
/// 星空凛, cost 10 — one above the bound. Carries an ability, irrelevant to waiting.
const COST_10: &str = "PL!-bp5-005-AR";
/// Cost 4, no ability — a second QUALIFYING member, for the two-candidate board.
const COST_4: &str = "PL!-sd1-010-SD";

/// Play Erena to debut with the given opponent stage, and land on the pay/skip gate.
///
/// Asserts the gate's identity rather than assuming it: the prompt is an options-less
/// `SelectTarget` on `pay_optional_cost:skip_optional_cost`, which is a different variant
/// from the member `SelectCard` that follows it, and the two are easy to confuse.
/// `select_option` panics on `options: None`, so the pay path uses
/// `select_choice_option` (the same `Pay` idiom as `bp7_q266_natsumi_blade_wait_test.rs`).
fn debuted_with_opponent_stage(opponent: [i16; 3]) -> (TestGame, i16) {
    let mut game = TestGame::new(load_real_database());
    let erena = game.id(ERENA);
    game.assert_card_identity(erena, ERENA);

    game.state.player2.stage.stage = opponent;
    game.state.player1.stage.stage = [-1, -1, -1];
    let filler = game.id(COST_4);
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);

    game.add_to_hand(erena);
    game.play_to_stage(erena, MemberArea::Center);

    game.assert_pending_choice_type(
        "SelectTarget",
        "the 登場 must open its optional 「このメンバーをウェイトにしてもよい」 pay/skip gate",
    );
    (game, erena)
}

/// The stage POSITIONS the pending member prompt actually offers, as card ids.
///
/// `filtered_indices` is the candidate set; `selection_cards` is not (it lists the whole
/// zone). Positions are resolved against the target player's stage.
fn offered_candidate_ids(game: &TestGame) -> Vec<i16> {
    let choice = game.get_pending_choice().clone();
    let (positions, target) = match &choice {
        rabuka_engine::ability::types::Choice::SelectCard {
            filtered_indices,
            target_player_id,
            ..
        } => (
            filtered_indices.clone().unwrap_or_default(),
            target_player_id.clone().unwrap_or_else(|| "self".to_string()),
        ),
        other => panic!("expected the opponent-member SelectCard prompt, got {other:?}"),
    };
    let stage = match target.as_str() {
        "opponent" => &game.state.player2.stage.stage,
        _ => &game.state.player1.stage.stage,
    };
    positions
        .iter()
        .map(|&pos| stage[pos])
        .collect::<Vec<i16>>()
}

fn is_waited(game: &TestGame, id: i16) -> bool {
    game.state.mods.get_orientation_modifier(id) == Some("wait")
}

fn heart05(game: &TestGame, id: i16) -> i16 {
    game.state.mods.get_heart_modifier(id, HeartColor::Heart05)
}

fn heart06(game: &TestGame, id: i16) -> i16 {
    game.state.mods.get_heart_modifier(id, HeartColor::Heart06)
}

// ---------------------------------------------------------------------------
// Premise: the fixtures really sit either side of the bound.
// ---------------------------------------------------------------------------

/// The bound is only meaningful if the two fixtures are exactly on it. A transposed or
/// re-numbered print would otherwise leave the whole file testing a different clause.
#[test]
fn the_two_boundary_fixtures_print_cost_nine_and_cost_ten() {
    let game = TestGame::new(load_real_database());
    let m9 = game.id(COST_9);
    let m10 = game.id(COST_10);
    let m4 = game.id(COST_4);
    game.assert_card_identity(m9, COST_9);
    game.assert_card_identity(m10, COST_10);
    game.assert_card_identity(m4, COST_4);
    game.assert_card_cost(m9, 9);
    game.assert_card_cost(m10, 10);
    game.assert_card_cost(m4, 4);
}

// ---------------------------------------------------------------------------
// Paying the optional cost.
// ---------------------------------------------------------------------------

/// The bound is INCLUSIVE: paying the self-wait waits the cost-9 member and leaves the
/// cost-10 member alone.
///
/// This is the assertion the ignored predecessor could not make. Its board was also
/// `[cost 9, cost 10]`, but it tried to read the *offered set* — and on that board the
/// engine does not prompt at all (`candidates.len() > count` is false with one
/// candidate), so it read a prompt that does not exist and got `offered=[]`. The outcome
/// is the same board and the same filter, read the way the engine actually resolves it.
#[test]
fn paying_the_cost_waits_the_cost_nine_member_and_not_the_cost_ten_member() {
    let probe = TestGame::new(load_real_database());
    let m9 = probe.id(COST_9);
    let m10 = probe.id(COST_10);

    let (mut game, erena) = debuted_with_opponent_stage([m9, m10, -1]);
    game.select_choice_option(1); // pay 「このメンバーをウェイトにしてもよい」
    game.drain_choices();

    assert!(
        is_waited(&game, erena),
        "paying the optional cost must put Erena herself to wait — the cost IS her own \
         orientation"
    );
    assert!(
        is_waited(&game, m9),
        "コスト9以下のメンバー — a cost-9 opponent member must be waited"
    );
    assert!(
        !is_waited(&game, m10),
        "コスト9以下のメンバー — a cost-10 opponent member must NOT be waited. Waiting it \
         means the bound is exclusive, or read as a minimum (>= 9)"
    );
    assert_eq!(
        game.state.player2.stage.stage,
        [m9, m10, -1],
        "neither opponent member may LEAVE the stage — ウェイト is an orientation, not a move"
    );
}

/// The offered set, read at the prompt. Needs TWO qualifying members so the prompt opens.
///
/// This is the only way to see the candidate list, and it is the assertion that pins
/// 「コスト9以下」 as a filter over a *choice*: the cost-10 member is on the board, is a
/// legal stage member, and is still not offered.
///
/// **The cost-4 member is what pins the OPERATOR, and it is easy to overlook.** I had
/// assumed `cost_limit_operator` was unfalsifiable here, on the grounds that the engine
/// defaults an unrecognised operator to `<=` (`ability/util.rs`, `card_matches_cost_limit_op`)
/// and that 9-in/10-out is satisfied by `<=`, `=` and `>=` alike. That is wrong, and the
/// cost-4 candidate is what shows it. Each mis-parse of 「コスト9以下」 predicts a
/// DIFFERENT offered set:
///
/// | parsed operator | offered                            |
/// |-----------------|------------------------------------|
/// | `<=`  (correct) | `[m9, m4]` — cost 4 and 9 qualify  |
/// | `=`             | `[m9]`    — only cost exactly 9     |
/// | `<`             | `[m4]`    — only cost below 9        |
/// | `>=`            | `[m9, m10]` — 9 and 10 qualify       |
///
/// So `offered == vec![m9, m4]` below distinguishes all four. A test that used only the
/// cost-9 and cost-10 members would pass under three of those four, and would then look like
/// coverage of the operator when it is coverage of nothing.
///
/// Parser side: `以下` is mapped to `<=` by `OPERATOR_PATTERNS` in
/// `cards/ability_extraction/parser_fields.py` (table order — `以上` is tested before
/// `以下`, so a text containing both would resolve to `>=`; this one contains only `以下`).
#[test]
fn the_offered_candidate_set_excludes_the_cost_ten_member() {
    let probe = TestGame::new(load_real_database());
    let m9 = probe.id(COST_9);
    let m4 = probe.id(COST_4);
    let m10 = probe.id(COST_10);

    // m9 at slot 0, m4 at slot 1, m10 at slot 2.
    let (mut game, erena) = debuted_with_opponent_stage([m9, m4, m10]);
    game.select_choice_option(1);

    game.assert_pending_choice_type(
        "SelectCard",
        "with two qualifying opponent members the 登場 must open a member prompt \
         (change_state.rs:504 needs candidates.len() > count)",
    );
    let offered = offered_candidate_ids(&game);
    assert_eq!(
        offered,
        vec![m9, m4],
        "the prompt must offer exactly the two コスト9以下 members"
    );
    assert!(
        !offered.contains(&m10),
        "コスト9以下のメンバー — the cost-10 member must not be offered"
    );

    // Answer it, and check the choice landed on the card that was offered.
    game.select_indices(&[0]);
    game.drain_choices();

    assert!(is_waited(&game, erena), "Erena paid her own orientation");
    assert!(
        is_waited(&game, m9),
        "selecting candidate 0 must wait the cost-9 member at stage slot 0"
    );
    assert!(
        !is_waited(&game, m4),
        "the unselected cost-4 member must stay active — the effect waits 1人, not all"
    );
    assert!(
        !is_waited(&game, m10),
        "the cost-10 member must stay active"
    );
}

/// `count: 1` is a real limit, not a formality: three qualifying members, one waited.
#[test]
fn exactly_one_member_is_waited_even_when_all_three_qualify() {
    let probe = TestGame::new(load_real_database());
    let m9 = probe.id(COST_9);
    let m4 = probe.id(COST_4);
    let m4b = probe.new_id(COST_4);

    let (mut game, _erena) = debuted_with_opponent_stage([m9, m4, m4b]);
    game.select_choice_option(1);
    game.select_indices(&[1]);
    game.drain_choices();

    let waited = [m9, m4, m4b].iter().filter(|&&c| is_waited(&game, c)).count();
    assert_eq!(
        waited, 1,
        "メンバー1人 — exactly one of three qualifying members is waited"
    );
    assert!(
        is_waited(&game, m4),
        "candidate index 1 is stage slot 1, the first cost-4 copy"
    );
}

/// Q137: an opponent member already in wait is not a legal 「ウェイトにする」 target, so it
/// must not be offered — and the single remaining candidate auto-resolves without a prompt.
///
/// This also pins the auto-resolve branch from the other side: with one candidate left the
/// engine must NOT open a prompt, and must still apply the wait.
#[test]
fn an_already_waited_member_is_not_offered_and_the_remaining_candidate_resolves() {
    let probe = TestGame::new(load_real_database());
    let m9 = probe.id(COST_9);
    let m4 = probe.id(COST_4);

    let (mut game, erena) = debuted_with_opponent_stage([m9, m4, -1]);
    // Put the cost-4 member in wait BEFORE paying, so it drops out of the candidate set.
    game.state.mods.add_orientation_modifier(m4, "wait");

    game.select_choice_option(1);
    assert!(
        !game.has_pending_choice(),
        "with the cost-4 member already waited, only one candidate remains, so \
         candidates.len() > count is false and no member prompt may open (change_state.rs:504)"
    );
    game.drain_choices();

    assert!(is_waited(&game, erena), "Erena paid her own orientation");
    assert!(
        is_waited(&game, m9),
        "the one remaining コスト9以下 member must be waited"
    );
    assert_eq!(
        game.state.mods.orientation_modifiers.get(&m4),
        Some(&CardOrientation::Wait),
        "the already-waited member keeps its state and is not re-waited"
    );
}

// ---------------------------------------------------------------------------
// Declining.
// ---------------------------------------------------------------------------

/// Declining 「このメンバーをウェイトにしてもよい」 cancels the whole chained ability.
///
/// 「…てもよい：A。」 is one optional step gating a mandatory one. Declining the self-wait
/// has to stop the opponent wait too — otherwise the player pays with their own orientation
/// for a wait they did not choose, or an opponent member is waited for free.
#[test]
fn declining_the_optional_self_wait_does_not_wait_the_opponent() {
    let probe = TestGame::new(load_real_database());
    let m9 = probe.id(COST_9);
    let m4 = probe.id(COST_4);

    let (mut game, erena) = debuted_with_opponent_stage([m9, m4, -1]);
    game.select_choice_option(0); // decline 「このメンバーをウェイトにしてもよい」
    game.drain_choices();

    assert!(
        !is_waited(&game, erena),
        "Erena herself must not be waited, since the optional step was declined"
    );
    assert!(
        !is_waited(&game, m9),
        "declining このメンバーをウェイトにしてもよい must cancel the chained opponent wait \
         too — otherwise an opponent member is waited for a choice the player refused"
    );
    assert!(
        !is_waited(&game, m4),
        "no opponent member may be waited after a decline"
    );
    assert_eq!(
        heart05(&game, erena),
        0,
        "Erena is active after a decline, so her 常時 grants no heart05 — the decline must \
         not leave her waited-but-unrecalculated"
    );
}

// ---------------------------------------------------------------------------
// No legal target.
// ---------------------------------------------------------------------------

/// Nothing on the opponent's stage is コスト9以下: the cost is still payable (it is
/// optional and the player chose it), and the effect then has no target and does nothing.
///
/// **This test found a real engine defect, and it is the only card in the pool that could.**
///
/// Paying puts Erena to wait, so her own 常時 must grant heart05 — but
/// `execute_change_state` returned early on the empty candidate list
/// (`change_state.rs:469-489`) WITHOUT calling `recalculate_constants()`, which every other
/// exit from that function does. So she was in wait and still granted nothing.
///
/// The empty-candidate path is reached only when the ability's own effect has no target,
/// which made this the single narrowest place the bug could hide: the same card on a board
/// with any qualifying member recalculated normally, and the defect was invisible.
///
/// Checking the blast radius explains why it survived at all. 33 card prints carry this exact
/// cost shape (`change_state` + `self_cost` + `optional` + `state_change: wait`, across 13
/// unique abilities) — but PL!-bp5-333-R and its P＋ are the ONLY cards in the entire pool
/// that BOTH pay an optional self-wait AND have a 常時 gated on their own wait
/// (`effect.condition` = `state_condition` / `state: wait` / `self_target: true`). Every other
/// card in the family grants nothing while waited, so for all of them a stale recalculation
/// has no observable consequence and the missing call was harmless. One card, one board.
///
/// The fix is at the symptom site (the empty-candidate exit). The invariant arguably belongs
/// in `handle_optional_cost_payment`, which writes the activating card's orientation
/// (`cost/handlers.rs:1487`) and never recalculates either — but for the other 32 prints
/// nothing can observe that, so moving the fix there would be untestable churn. Noted rather
/// than done.
#[test]
fn a_cost_ten_only_opponent_stage_waits_nobody() {
    let probe = TestGame::new(load_real_database());
    let m10 = probe.id(COST_10);
    let m10b = probe.new_id(COST_10);

    let (mut game, erena) = debuted_with_opponent_stage([m10, m10b, -1]);
    game.select_choice_option(1);
    game.drain_choices();

    assert!(
        is_waited(&game, erena),
        "the cost was paid and it is Erena herself who pays it"
    );
    assert!(
        !is_waited(&game, m10) && !is_waited(&game, m10b),
        "neither cost-10 member is コスト9以下, so neither may be waited"
    );
    assert_eq!(
        heart05(&game, erena),
        1,
        "the orientation her own debut cost wrote must still reach her 常時 even when the \
         chained effect had no target — the empty-candidate early return in \
         execute_change_state used to skip recalculate_constants() \
         (change_state.rs:469-489), leaving her in wait and granting no heart05"
    );
}

// ---------------------------------------------------------------------------
// The two abilities together — the actual payoff of the card.
// ---------------------------------------------------------------------------

/// The 登場 and the 常時 in one resolution: paying the debut cost waits Erena, and that wait
/// is what switches her 常時 on.
///
/// Every other test in the tree that touches her 常時 writes the orientation by hand and
/// calls `recalculate_constants()` itself. Nothing drives the real chain from the 登場 to the
/// heart, which is the only way the card functions in a game and the only way a regression
/// in the pay path's orientation write would be caught from the outside.
#[test]
fn paying_the_debut_cost_switches_on_her_own_constant_heart05() {
    let probe = TestGame::new(load_real_database());
    let m9 = probe.id(COST_9);
    let m10 = probe.id(COST_10);

    let (game, erena) = debuted_with_opponent_stage([m9, m10, -1]);
    assert_eq!(
        heart05(&game, erena),
        0,
        "premise: Erena is active on debut, so her 常時 grants nothing yet"
    );

    let mut game = game;
    game.select_choice_option(1);
    game.drain_choices();

    assert!(
        is_waited(&game, erena),
        "the paid cost is Erena's own orientation"
    );
    assert_eq!(
        heart05(&game, erena),
        1,
        "egines 常時 must turn on from the orientation her own debut cost just wrote — \
         not from a hand-set modifier plus a manual recalculate_constants()"
    );
}

// ---------------------------------------------------------------------------
// The wait itself is a game event: cross-ability integration.
// ---------------------------------------------------------------------------

/// The state change Erena causes re-triggers the auto-ability machinery, and another card
/// on the board can read it. 桂城 泉 `PL!HS-bp5-016-N`:
///
/// ```text
/// {{jyouji.png|常時}}相手のステージにウェイト状態のメンバーが2人以上いるかぎり、
/// {{heart_06.png|heart06}}を得る。
/// ```
///
/// This is the only test in the tree that produces a waited OPPONENT member through a real
/// ability resolution and then reads another card's 常時 off it. Every other test in
/// `two_or_more_opponent_wait_members_constant_heart06_test.rs` hand-writes the orientation
/// and calls `recalculate_constants()` itself, so the trigger from a genuine effect —
/// `change_state.rs:750-757` re-triggers auto abilities for both players — is unpinned.
///
/// Also covers the one-candidate auto-resolve: the second コスト9以下 member is waited by
/// hand beforehand, so paying the cost needs no member prompt and Izumi crosses her
/// threshold purely from the effect.
#[test]
fn the_waited_opponent_member_crosses_another_cards_constant_threshold() {
    let mut game = TestGame::new(load_real_database());
    let erena = game.id(ERENA);
    let izumi = game.id("PL!HS-bp5-016-N");
    game.assert_card_identity(izumi, "PL!HS-bp5-016-N");

    let m9 = game.id(COST_9);
    let m4 = game.id(COST_4);
    let filler = game.id(COST_4);
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Izumi is already on our stage; the centre slot is free for Erena's debut.
    game.state.player1.stage.stage = [izumi, -1, -1];
    game.state.player2.stage.stage = [m9, m4, -1];
    // One opponent member already waited, so Izumi is one short of her threshold and
    // Erena paying the cost is what completes it.
    game.state.mods.add_orientation_modifier(m4, "wait");
    game.state.recalculate_constants();
    assert_eq!(
        heart06(&game, izumi),
        0,
        "premise: with 1 of 2 needed opponent members waited, Izumi grants nothing"
    );

    game.give_energy(15);
    game.add_to_hand(erena);
    game.play_to_stage(erena, MemberArea::Center);
    game.select_choice_option(1);
    game.drain_choices();

    assert!(
        is_waited(&game, m9),
        "premise for the threshold: the effect waited the cost-9 member"
    );
    assert_eq!(
        heart06(&game, izumi),
        1,
        "the wait Erena's 登場 caused must satisfy another card's 常時 — 2人以上 is a \
         threshold, so exactly one heart06"
    );
}

// ---------------------------------------------------------------------------
// Empty board, and two copies in one turn.
// ---------------------------------------------------------------------------

/// An EMPTY opponent stage: the cost is payable, the effect has no target, nothing happens,
/// and — critically — nothing panics or leaves the game parked mid-ability.
///
/// The empty-candidate path returns before the prompt is ever built
/// (`change_state.rs:469-489`), so this is the degenerate end of the same branch the
/// cost-10-only board takes, and the one where a missing `return` would be most visible.
#[test]
fn an_empty_opponent_stage_resolves_cleanly() {
    let (mut game, erena) = debuted_with_opponent_stage([-1, -1, -1]);
    game.select_choice_option(1);
    game.drain_choices();

    assert!(
        is_waited(&game, erena),
        "the cost is optional and the player paid it, so Erena is waited regardless of \
         whether the effect had a target"
    );
    assert!(
        !game.has_pending_choice(),
        "the ability must terminate rather than leave a prompt parked — a drain that stops \
         early would otherwise satisfy every absence assertion in this file"
    );
    assert_eq!(
        game.state.player2.stage.stage,
        [-1, -1, -1],
        "an empty stage must stay empty"
    );
}

/// Two Erenas debuting in the same main phase: each opens its own gate and each pays its own
/// orientation, and the second one's candidate set reflects the first one's wait.
///
/// This is the queue-independence question. `play_to_stage` enqueues both 登場 abilities,
/// and the first Erena to resolve waits a member — which removes it from the candidate set
/// the second Erena sees. If the two shared resolver state, the second would either re-offer
/// an already-waited member or skip its own cost.
#[test]
fn two_copies_each_pay_their_own_orientation_and_see_the_updated_board() {
    let mut game = TestGame::new(load_real_database());
    let erena_a = game.id(ERENA);
    let erena_b = game.new_id(ERENA);
    game.assert_card_identity(erena_a, ERENA);
    game.assert_card_identity(erena_b, ERENA);

    let m9 = game.id(COST_9);
    let m4 = game.id(COST_4);
    let m4b = game.new_id(COST_4);
    let filler = game.id(COST_4);
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.stage.stage = [m9, m4, m4b];
    game.give_energy(15);

    game.add_to_hand(erena_a);
    game.play_to_stage(erena_a, MemberArea::LeftSide);
    game.assert_pending_choice_type(
        "SelectTarget",
        "the first copy must open its own self-wait gate",
    );
    game.select_choice_option(1);
    assert_eq!(
        offered_candidate_ids(&game),
        vec![m9, m4, m4b],
        "three qualifying members, so the member prompt opens for the first copy"
    );
    game.select_indices(&[0]);
    game.drain_choices();

    assert!(is_waited(&game, erena_a), "the first copy paid its orientation");
    assert!(is_waited(&game, m9), "the first copy waited the cost-9 member");

    // The second copy debuts onto a board where the cost-9 member is already waited.
    game.add_to_hand(erena_b);
    game.play_to_stage(erena_b, MemberArea::Center);

    // This was, before the fix, a KNOWN ENGINE DEFECT: the correct behaviour is
    // `SelectTarget` (the second copy's own optional self-wait gate) followed by a
    // `SelectCard` over `[m4, m4b]`. What the engine used to do is skip the gate entirely
    // and go straight to a `SelectCard`, because `trigger_debut_abilities` matched the
    // debut by card NUMBER, so copy B's debut re-ran COPY A's ability. With
    // `activating_card == Some(copy A)` and copy A already in wait,
    // `get_change_state_candidates` found no ACTIVE candidate (`handlers.rs:862-878`),
    // `pay_cost_change_state` returned Ok(()) with no choice, and the optional cost was
    // treated as satisfied without ever being offered. The player never got to choose and
    // copy B's orientation was never written.
    //
    // This is GENERIC, not Erena-specific: it needed only two copies of any card with a
    // 登場 on stage, where the earlier-area copy was in wait. See
    // `a_cost_free_self_wait_debut_repeats_on_the_first_copy` for the minimal repro on a
    // different card.
    game.assert_pending_choice_type(
        "SelectTarget",
        "the second copy's debut must open its OWN optional self-wait gate — the debut must \
         be attributed to the card that played, not to the lowest-area copy sharing its \
         card number",
    );
    game.select_choice_option(1);
    assert_eq!(
        offered_candidate_ids(&game),
        vec![m4, m4b],
        "the already-waited cost-9 member must have dropped out of the second copy's \
         candidate set (Q137)"
    );
    game.select_indices(&[0]);
    game.drain_choices();

    assert!(
        is_waited(&game, erena_b),
        "the second copy paid its OWN orientation — the two must not share cost state"
    );
    let waited = [m9, m4, m4b]
        .iter()
        .filter(|&&c| is_waited(&game, c))
        .count();
    assert_eq!(
        waited, 2,
        "two debuts, one member each — total 2 waited, and neither copy double-waits"
    );
}

// ---------------------------------------------------------------------------
// The re-apply path: what the prompt stashes and what actually applies the wait.
// ---------------------------------------------------------------------------

/// When the effect DOES prompt, the wait is not applied by the prompt itself.
/// `execute_change_state` stashes a re-apply action (change_state.rs:545-552) and returns;
/// the answer handler (`choice.rs:2143-2158`) dispatches on
/// `StageSelectIntent::ChangeStateWait` and, in the EFFECT phase, deliberately does NOT write
/// the orientation — the comment there says "The stored ChangeState pending-action re-applies
/// with wait-immunity filtering." The chosen ids are pushed into `selection.cards`
/// (choice.rs:2124-2128) and the re-apply picks them up through
/// `change_state.rs:350-355`.
///
/// The mechanism is worth pinning, because the stashed effect is built with
/// `..Default::default()` and so carries only `action`, `target`, `count` and `kind` —
/// **not** `state_change`, `card_type` or `cost_limit`. Read naively that would be a bug: on
/// re-apply `is_member_op` (change_state.rs:298) would be false and the wait would go to
/// `execute_energy_state_change` instead. MEASURED via instrumentation on this board, it does
/// NOT happen — the re-apply runs with `state_change="wait"` and
/// `card_type=member_card` intact, so the member branch and the wait-immunity filter are both
/// live. The full effect is reached through the answer path rather than the stripped
/// stashed copy. This test locks in the observable outcome so that if that ever changes the
/// failure names it, instead of surfacing as a stray "member not waited" elsewhere.
///
/// It is also the only place the prompted path is exercised with a selection OTHER than
/// candidate 0, and the only place a pre-existing wait on a DIFFERENT candidate coexists with
/// a prompted choice.
#[test]
fn the_prompted_path_applies_the_wait_to_the_chosen_member_only() {
    let probe = TestGame::new(load_real_database());
    let m9 = probe.id(COST_9);
    let m4 = probe.id(COST_4);
    let m4b = probe.new_id(COST_4);

    let (mut game, erena) = debuted_with_opponent_stage([m9, m4, m4b]);
    game.select_choice_option(1);
    game.assert_pending_choice_type("SelectCard", "three candidates, so the prompt opens");
    assert_eq!(
        offered_candidate_ids(&game),
        vec![m9, m4, m4b],
        "all three are コスト9以下"
    );

    // Choose the SECOND candidate (stage slot 1).
    game.select_indices(&[1]);
    game.drain_choices();

    assert!(
        is_waited(&game, m4),
        "candidate index 1 is stage slot 1, the first cost-4 copy — the re-apply must \
         apply the wait to the CHOSEN member, not to candidate 0"
    );
    assert!(
        !is_waited(&game, m9),
        "candidate 0 was not chosen, so the cost-9 member must stay active"
    );
    assert!(
        !is_waited(&game, m4b),
        "candidate 2 was not chosen either"
    );
    assert!(
        is_waited(&game, erena),
        "the paid cost is Erena's own orientation, independent of which member was chosen"
    );
    assert_eq!(
        game.state.player2.stage.stage,
        [m9, m4, m4b],
        "no member left the stage — this is an orientation, not a move, on the prompted \
         path as well as the auto-resolve one"
    );
}

// ---------------------------------------------------------------------------
// Ownership of the 常時 condition.
// ---------------------------------------------------------------------------

/// The 常時 condition is `self_target: true` on the card's OWN wait, so it must follow the
/// card, not the player who happens to be resolving something.
///
/// One Erena on each side of the board, the opponent's in wait and ours active. The
/// opponent's copy must grant the OPPONENT a heart05 and ours nothing. Getting this wrong is
/// easy and silent: a condition evaluated against `gs.activating_card`, or against
/// `ability_master_id` instead of the constant's own controller, would hand the heart to the
/// wrong side, and a single-owner test on our own copy cannot tell the difference — which is
/// why every test in `erena_self_wait_constant_heart05_test.rs` (two copies, position change,
/// leave stage, no-orientation default, P＋, other members) exercises only our side.
#[test]
fn the_constant_condition_follows_the_card_not_the_resolving_player() {
    let mut game = TestGame::new(load_real_database());
    let ours = game.id(ERENA);
    let theirs = game.new_id(ERENA);
    game.assert_card_identity(ours, ERENA);
    game.assert_card_identity(theirs, ERENA);

    game.state.player1.stage.stage = [ours, -1, -1];
    game.state.player2.stage.stage = [theirs, -1, -1];
    // The OPPONENT's copy is the one in wait; ours is active.
    game.state.mods.add_orientation_modifier(theirs, "wait");
    game.state.recalculate_constants();

    assert_eq!(
        heart05(&game, theirs),
        1,
        "the opponent's waited Erena grants the OPPONENT a heart05 — the 常時 condition is \
         `self_target` on her own wait, so it must be evaluated for her controller"
    );
    assert_eq!(
        heart05(&game, ours),
        0,
        "our copy is ACTIVE, so it must grant nothing. A heart here means the condition was \
         evaluated against the resolving player (activating_card / ability_master_id) \
         instead of the constant's own card"
    );
}

/// The same split, reached through Erena HERSELF waiting an opponent's Erena.
///
/// This is the one board where both copies' 常時 fire at once and the two sides are
/// distinguishable, and it composes the two abilities with the ownership clause: our copy is
/// waited by the cost we paid, the opponent's copy is waited by the effect we chose. Each
/// heart05 must land on its own side.
#[test]
fn waiting_an_opponent_erena_grants_the_opponent_her_own_heart() {
    let mut game = TestGame::new(load_real_database());
    let ours = game.id(ERENA);
    let theirs = game.new_id(ERENA);
    game.assert_card_identity(ours, ERENA);
    game.assert_card_identity(theirs, ERENA);

    // The opponent's Erena costs 7, so she qualifies as a コスト9以下 target.
    game.state.player2.stage.stage = [theirs, -1, -1];
    game.state.player1.stage.stage = [-1, -1, -1];
    let filler = game.id(COST_4);
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);

    assert_eq!(
        heart05(&game, theirs),
        0,
        "premise: the opponent's Erena is active, so she grants nothing yet"
    );

    game.add_to_hand(ours);
    game.play_to_stage(ours, MemberArea::Center);
    game.select_choice_option(1);
    game.drain_choices();

    assert!(
        is_waited(&game, theirs),
        "the opponent's Erena is コスト7, so she is a legal target for the effect"
    );
    assert!(
        is_waited(&game, ours),
        "our copy is waited by the optional cost we paid"
    );
    assert_eq!(
        heart05(&game, theirs),
        1,
        "the opponent's waited copy grants the OPPONENT her heart05"
    );
    assert_eq!(
        heart05(&game, ours),
        1,
        "our waited copy grants US her heart05 — each side resolves its own constant"
    );
}

// ---------------------------------------------------------------------------
// WHO is asked to choose the opponent's member.
// ---------------------------------------------------------------------------

/// The printed card says 「相手のステージにいる…メンバー1人をウェイトにする」. The cards being
/// waited belong to the opponent, but the choice of WHICH one is made by the player whose
/// ability is resolving — that is Erena, not her opponent. In LLSS target selection always
/// belongs to the resolving player; there is no rule handing the pick to the owner of the
/// targeted cards. It matters mechanically, not just formally: if the opponent picked, they
/// would choose the least damaging member for themselves, turning a cost to the card's player
/// into a cost the card's player does not control.
///
/// This is the one clause in the whole ability that no test anywhere was pinning, and the
/// engine has explicit machinery for the opposite behaviour:
///
/// ```rust
/// // engine/src/ability/queue.rs:308-343
/// let is_opponent_choice = match &choice {
///     Choice::SelectCard { target_player_id: Some(tpid), .. } => tpid == "opponent",
///     ...
/// if !is_spawn_opponent && !is_route_opponent { return None; }   // else: route to the OPPONENT
/// ```
///
/// So a `SelectCard` whose `target_player_id` is `"opponent"` is handed to the opponent
/// whenever the resolver's spawn context says the effect targets the opponent — which is
/// exactly Erena's case. `SelectCard` is also what `change_state.rs:519-546` builds, and it
/// sets `target_player_id` to the effect's `target`, i.e. `"opponent"` here.
#[test]
fn the_resolving_player_chooses_which_opponent_member_is_waited() {
    let probe = TestGame::new(load_real_database());
    let m9 = probe.id(COST_9);
    let m4 = probe.id(COST_4);

    let (mut game, _erena) = debuted_with_opponent_stage([m9, m4, -1]);
    game.select_choice_option(1);
    game.assert_pending_choice_type("SelectCard", "two candidates, so the prompt opens");

    let picker = game
        .state
        .ability_queue
        .current_entry()
        .and_then(|e| e.choice_player_id.clone());
    assert_eq!(
        picker.as_deref(),
        Some("p1"),
        "the player resolving Erena's 登場 (p1) chooses which of the opponent's members is \
         waited. Got {picker:?}. If this is p2, the OPPONENT is choosing which of their own \
         members takes the ウェイト, which is both a rules error and a mechanical one — see \
         queue.rs:308-343, which routes an opponent-targeted SelectCard to the opponent."
    );
}

// ---------------------------------------------------------------------------
// The debut trigger scan itself, isolated from Erena.
// ---------------------------------------------------------------------------

/// 近江彼方 `PL!N-bp3-006-P` (cost 4):
///
/// ```text
/// {{toujyou.png|登場}}このメンバーをウェイトにする。
/// ```
///
/// A cost-free, prompt-free, self-target debut — the simplest 登場 in the pool. Two copies
/// of it, first in LeftSide, then one in Center. The correct result is that BOTH end up
/// waited, one per debut.
///
/// This isolates the `trigger_debut_abilities` defect from everything else in Erena's
/// ability. There is no optional cost, no gate, no prompt and no opponent selection here,
/// so the ONLY thing that can go wrong is which copy the trigger scan attributes the debut
/// to. If both copies end up waited the scan is fine; if only the first is, the second
/// debut was attributed to the first copy by card number.
///
/// Unlike the Erena case this bug is SILENT — no missing prompt, just a member that never
/// receives its own 登場 — which is why nothing in the tree caught it and why this test
/// asserts the board state rather than a prompt.
#[test]
fn a_cost_free_self_wait_debut_repeats_on_the_first_copy() {
    let mut game = TestGame::new(load_real_database());
    let kanata_a = game.id("PL!N-bp3-006-P");
    let kanata_b = game.new_id("PL!N-bp3-006-P");
    game.assert_card_identity(kanata_a, "PL!N-bp3-006-P");
    game.assert_card_identity(kanata_b, "PL!N-bp3-006-P");
    // Printed cost 9. Irrelevant to this test — the ability is self-target and cost-free —
    // but pinned so the fixture is unmistakably the card the doc names.
    game.assert_card_cost(kanata_a, 9);

    let filler = game.id(COST_4);
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    // Cost 9 per copy, and up to three copies are played below.
    game.give_energy(30);

    game.add_to_hand(kanata_a);
    game.play_to_stage(kanata_a, MemberArea::LeftSide);
    game.drain_choices();
    assert!(
        is_waited(&game, kanata_a),
        "premise: the first copy's 登場 waited itself — it is unconditional, so no prompt \
         and the orientation must be written directly"
    );

    game.add_to_hand(kanata_b);
    // Take copy A out of wait so the second debut's outcome is UNAMBIGUOUS. If the trigger
    // scan attributes the debut correctly, playing copy B waits copy B and A stays active.
    // If it attributes to the first copy by card number, playing B instead re-waits A.
    //
    // The forced "active" is an EXPLICIT orientation, not a cleared one, so copy A reads
    // back as Some("active") rather than None. The assertion below accounts for that: what
    // matters is that A is not waited and B is.
    game.state.mods.add_orientation_modifier(kanata_a, "active");
    game.state.recalculate_constants();
    assert!(
        !is_waited(&game, kanata_a),
        "premise: copy A is forced ACTIVE, so any wait after B's debut must be B's own"
    );

    game.add_to_hand(kanata_b);
    game.play_to_stage(kanata_b, MemberArea::Center);
    game.drain_choices();

    // KNOWN ENGINE DEFECT, written to document rather than to pass. The correct pair is
    // (A not-waited, B waited). Before the fix the engine produced (A waited, B not-waited):
    // `TurnEngine::trigger_debut_abilities` (engine/src/turn/trigger_scan.rs) matched the
    // debuting card by CARD NUMBER —
    //
    //     if card.card_no.as_ref() != card_no { continue; }
    //
    // — and then `break` out of the area loop on the FIRST area whose occupant carried that
    // number. With two copies of one card number on stage it enqueued the LeftSide copy's
    // 登場 and never the copy that just played. Every 登場 in the pool routes through this
    // scan (265 unique 登場 abilities in cards/abilities.json), so any deck holding two
    // copies of a card with a 登場 mis-resolved the second one. Not Erena-specific.
    //
    // MEASURED on the three-copy variant of this board: a=Some("wait") b=None c=None.
    //
    // Fix: the scan now takes the debuted card's ID and matches on that, with the print
    // number only as a fallback for callers that cannot name an id, and the `break` is gone.
    let a_after = game
        .state
        .mods
        .get_orientation_modifier(kanata_a)
        .map(str::to_string);
    let b_after = game
        .state
        .mods
        .get_orientation_modifier(kanata_b)
        .map(str::to_string);
    assert!(
        !a_after.as_deref().is_some_and(|o| o == "wait"),
        "the first copy must NOT be waited by the second copy's debut — that means the \
         debut was attributed by card NUMBER to the lowest-area copy \
         (trigger_scan.rs) rather than to the copy that actually played (a_after={a_after:?})"
    );
    assert_eq!(
        b_after.as_deref(),
        Some("wait"),
        "the second copy's debut must wait the second copy. Before the trigger-scan fix the \
         debut resolved to the first copy instead, so B stayed active \
         (b_after={b_after:?}, a_after={a_after:?})"
    );
}

// ---------------------------------------------------------------------------
// The second caller of the debut scan: baton touch into an occupied area.
// ---------------------------------------------------------------------------

/// The baton-touch debut is the OTHER call site of `trigger_debut_abilities`
/// (`turn/phases.rs:1076`, the one that passes `baton_touch_used = true`). It is a
/// genuinely different path from an ordinary play and nothing in this file's tree reached it
/// through the duplicate case.
///
/// Board: Kanata copy 1 sits in LeftSide, copy 2 sits in Center, and copy 3 is played INTO
/// Center — an occupied area, so that play is a baton touch and copy 2 goes to the waitroom
/// while copy 3 arrives. Two copies of one card number remain on stage, and the LEFTMOST one
/// is the one that did not just debut.
///
/// This is why the copies must be a card whose 登場 is visible PER INSTANCE. An earlier
/// version of this idea used three identical cost-4 members whose 登場 was "draw 1, mill 1" —
/// which is worthless as a discriminator, because whichever copy fired produces the same
/// draw. 近江彼方's 登場 writes HER OWN orientation, so exactly one of the three can be
/// observed changing, and the answer is unambiguous.
#[test]
fn a_baton_touch_debut_belongs_to_the_arriving_copy() {
    let mut game = TestGame::new(load_real_database());
    let first = game.id("PL!N-bp3-006-P");
    let second = game.new_id("PL!N-bp3-006-P");
    let arriving = game.new_id("PL!N-bp3-006-P");
    game.assert_card_identity(first, "PL!N-bp3-006-P");
    game.assert_card_identity(second, "PL!N-bp3-006-P");
    game.assert_card_identity(arriving, "PL!N-bp3-006-P");

    let filler = game.id(COST_4);
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    // Cost 9 per copy.
    game.give_energy(30);

    // Two copies already on stage, as fixtures. Neither has debuted, and neither is in wait.
    game.add_to_stage(MemberArea::LeftSide, first);
    game.add_to_stage(MemberArea::Center, second);
    assert_eq!(
        game.state.player1.stage.stage,
        [first, second, -1],
        "premise: two copies staged, the leftmost is NOT the one about to debut"
    );

    game.add_to_hand(arriving);
    // `play_to_stage` deliberately panics when the target area is occupied — see the CAUTION
    // on it about 137 tests accidentally baton-touching. This test means it, so it uses the
    // dedicated helper.
    game.play_to_stage_baton(arriving, MemberArea::Center);
    game.drain_choices();

    assert_eq!(
        game.state.player1.stage.stage[MemberArea::Center as usize],
        arriving,
        "premise check: playing into an occupied area must replace the occupant, so copy 2 \
         left the stage and copy 3 took its slot"
    );
    assert_eq!(
        game.state.player1.stage.stage[MemberArea::LeftSide as usize],
        first,
        "copy 1 is untouched by the baton touch"
    );

    let waited = |g: &TestGame, id: i16| g.state.mods.get_orientation_modifier(id).map(str::to_string);
    assert_eq!(
        waited(&game, arriving),
        Some("wait".to_string()),
        "the ARRIVING copy's own 登場 must fire. The baton-touch call site must attribute the \
         debut by the played card's id, not by card number — with copy 1 in the leftmost \
         area, a number-based scan breaks on copy 1 and leaves the arriving copy active."
    );
    assert_ne!(
        waited(&game, first),
        Some("wait".to_string()),
        "the leftmost copy did not debut, so it must NOT be waited"
    );
    assert_ne!(
        waited(&game, second),
        Some("wait".to_string()),
        "copy 2 was replaced by the baton touch; its 登場 must not run"
    );
}

// ---------------------------------------------------------------------------
// 登場 is scoped to the debut event.
// ---------------------------------------------------------------------------
// 登場 fires on every ENTRY to the stage, and never otherwise.
// ---------------------------------------------------------------------------

/// Erena leaves the stage and comes back, and her 登場 offers the gate a SECOND time.
///
/// 「登場」 is not a once-per-game or once-per-turn ability — `use_limit` is null in
/// `abilities.json` — it is scoped to the ENTRY event, so a member that leaves the stage
/// and returns debuts again. This matters for this card in particular because her 登場
/// is an OPTIONAL self-wait plus a chained opponent wait: if re-entry were silently
/// ignored, the second copy of Erena to arrive in a turn would cost nothing and would
/// hand the player a free ウェイト on a コスト9以下 opponent member while she herself
/// never went to wait. Nothing in the tree covers a card debuting twice.
///
/// The removal is hand-built (`state`), but the second debut is a real play through
/// `play_to_stage`, so the whole 登場 chain runs for real the second time.
#[test]
fn a_second_entry_to_the_stage_offers_the_debut_gate_again() {
    let probe = TestGame::new(load_real_database());
    let m9 = probe.id(COST_9);
    let m4 = probe.id(COST_4);

    let (mut game, erena) = debuted_with_opponent_stage([m9, m4, -1]);
    game.select_choice_option(1);
    game.select_indices(&[0]);
    game.drain_choices();
    assert!(
        is_waited(&game, erena) && is_waited(&game, m9),
        "premise: the first debut resolved — Erena waited herself and the cost-9 member
         is waited"
    );

    // Erena leaves the stage for the waitroom, and is no longer waited. Leaving the
    // stage clears orientation, which the sibling constant tests rely on; this pins that
    // the CLEAR is what makes the next entry a real debut rather than a no-op.
    let slot = game
        .state
        .player1
        .stage
        .stage
        .iter()
        .position(|&id| id == erena)
        .expect("Erena is on our stage");
    game.state.player1.stage.stage[slot] = -1;
    game.state.player1.waitroom.cards.push(erena);
    game.state.mods.clear_all_for_card(erena);
    game.state.recalculate_constants();
    assert!(
        !is_waited(&game, erena),
        "premise: off-stage she is no longer waited, so a re-entry is a genuine debut"
    );
    assert_eq!(
        heart05(&game, erena),
        0,
        "premise: and her 常時 is off while she is off-stage"
    );

    game.give_energy(20);
    game.add_to_hand(erena);
    game.play_to_stage(erena, MemberArea::RightSide);

    game.assert_pending_choice_type(
        "SelectTarget",
        "the SECOND entry must open the 登場 self-wait gate again — 登場 is scoped to the
         entry event, not to the first time the card is seen"
    );
    game.select_choice_option(0);
    game.drain_choices();

    assert!(
        !is_waited(&game, erena),
        "declining the second time must leave Erena active and must NOT put a second
         コスト9以下 member down — otherwise a re-entering Erena gets a free wait"
    );
    assert_eq!(
        [m9, m4]
            .iter()
            .filter(|&&c| is_waited(&game, c))
            .count(),
        1,
        "still exactly one waited opponent member — the declined second debut added
         none"
    );
}

// ---------------------------------------------------------------------------
// Wait-immunity: another card's restriction can void the wait.
// ---------------------------------------------------------------------------

/// An opponent member protected by 「相手の効果によってはウェイトしない」 is still OFFERED
/// (Erena chooses freely, Q274) but the wait is suppressed at application time
/// (`change_state.rs:574-588`).
///
/// The immunity list is a `GameState` field written by `restriction.rs:191`, keyed by
/// `(member, owner)`, and the check is `controller != owner` — so it only bites when the
/// waiting effect belongs to somebody other than the member's own controller. Nothing in
/// this file's tree exercises that filter against a `target: opponent` wait driven by a
/// real debut, so a regression that dropped the filter would silently let Erena wait an
/// immune member.
#[test]
fn a_wait_immune_opponent_member_is_offered_but_not_actually_waited() {
    let probe = TestGame::new(load_real_database());
    let m9 = probe.id(COST_9);
    let m4 = probe.id(COST_4);

    let (mut game, erena) = debuted_with_opponent_stage([m9, m4, -1]);
    // m9 is protected by its own controller against effects controlled by anyone else —
    // which is exactly Erena's position. The owner id is the SHORT form (`p2`), as
    // `restriction.rs` writes it and the existing Q274/Q275 tests assert it.
    game.state.wait_immune_members.push((m9, "p2".to_string()));

    game.select_choice_option(1);
    game.assert_pending_choice_type(
        "SelectCard",
        "Q274: an immune member is still a legal CHOICE — the suppressing happens at \
         application time, so the prompt must still open",
    );
    let offered = offered_candidate_ids(&game);
    assert!(
        offered.contains(&m9),
        "the immune member must remain selectable — the player chooses freely and the wait \
         is simply suppressed (offered={offered:?})"
    );

    game.select_indices(&[0]);
    game.drain_choices();

    assert!(
        !is_waited(&game, m9),
        "the wait must be suppressed on a wait-immune member"
    );
    assert!(
        is_waited(&game, erena),
        "the cost was still paid — suppression voids the opponent wait, not Erena's own"
    );
    assert!(
        !is_waited(&game, m4),
        "the unselected member is untouched, and the suppressed target is not silently \
         swapped for it — 1人 was chosen and that 1人 was voided"
    );
}

// ---------------------------------------------------------------------------
// The reprint.
// ---------------------------------------------------------------------------

/// `PL!-bp5-333-P＋` shares the ability text, so `cards/abilities.json` registers the 登場
/// for BOTH prints. Only the 常時 was ever exercised on the P＋; the debut half was not.
#[test]
fn the_p_plus_print_has_the_same_debut() {
    let mut game = TestGame::new(load_real_database());
    let erena = game.id("PL!-bp5-333-P＋");
    game.assert_card_identity(erena, "PL!-bp5-333-P＋");
    game.assert_card_cost(erena, 7);

    let m9 = game.id(COST_9);
    let m10 = game.id(COST_10);
    game.state.player2.stage.stage = [m9, m10, -1];
    game.state.player1.stage.stage = [-1, -1, -1];
    let filler = game.id(COST_4);
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);
    game.add_to_hand(erena);
    game.play_to_stage(erena, MemberArea::Center);

    game.assert_pending_choice_type(
        "SelectTarget",
        "the P＋ print must open the same optional self-wait gate",
    );
    game.select_choice_option(1);
    game.drain_choices();

    assert!(
        is_waited(&game, erena),
        "the P＋ debut must pay its own orientation when accepted"
    );
    assert!(
        is_waited(&game, m9),
        "the P＋ debut must wait the コスト9以下 member"
    );
    assert!(
        !is_waited(&game, m10),
        "the P＋ debut must respect the same コスト9以下 bound"
    );
}
