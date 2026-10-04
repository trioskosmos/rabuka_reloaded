/// Q231: 三船栞子 `PL!N-bp5-010-R` — LiveSuccess: no excess hearts → total score +1;
/// 2+ excess hearts → total score −1, never below 0.
///
/// # These two tests are VACUOUS — read this before trusting a green run
///
/// Both assert `mods.get_score_modifier(mifune) == 0` after driving to
/// `FirstAttackerPerformance`. Three things are wrong with that, and together they make
/// the assertion unfalsifiable:
///
///   * **The ability never fires.** 三船栞子's ability is ライブ成功時, which is processed at
///     `LiveVictoryDetermination`. The drive stops at `FirstAttackerPerformance`. The
///     second test's own comment admits the window "only happens to be that window today"
///     — i.e. the reachability depends on where five `pass()` calls happen to land.
///   * **Zero is what an unfired ability leaves behind.** So `0` is equally the value for
///     "the +1 and the −1 cancelled" and for "nothing ran at all". The assertion cannot
///     tell the behaviour it names from the absence of behaviour.
///   * **A ライブ成功時 `live_total` change is not readable through
///     `mods.get_score_modifier` anyway.** The live total is settled by the time the
///     modifier table is consulted; it has to be read off `performance_snapshots`, which
///     is what the MIRACLE WAVE tests (`…/jidou/yell/live_success_from_yell_reveal/`)
///     do. `set_live_card` is also handed `mifune`, who is a メンバー.
///
/// The 2+ excess board those tests stage is itself not what they claim: 高海千歌
/// `PL!S-sd1-001-SD` carries base (3,2,2) and no blade heart, so two of them give
/// (6,4,4) — against whatever live is played, that is not a controlled excess of 2.
///
/// # What the parse fix actually was
///
/// 「余剰ハートを持たない場合」 parses to
/// `comparison_condition { resource_type: "surplus_heart", negation: true }` with **no count
/// and no operator**. `compare_counts` reads a missing operator as `>=`, so the base is
/// `surplus >= 0` — true on every board — and the 「ない」 that sets `negation` does **not**
/// get applied on the surplus comparison path, so the branch is decided by a bound it does
/// not have. At surplus 0 that happens to be right (and is pinned in
/// `live_success_surplus_score_zero_floor_test`); at surplus >= 1 it is wrong and untested.
///
/// Adding `count: 1` + `operator: ">="` in the parser was tried and reverted — with
/// `negation` still ignored it only moves the error to surplus >= 1 and breaks the
/// surplus-0 case. **The fix is engine-side (honour `negation` for `surplus_heart`) and
/// must land with the parser half, not before it.** `the_no_excess_branch_is_negated_surplus_heart`
/// below records the shape and the gap.
///
/// That fix is pinned by `the_no_excess_branch_is_not_a_tautology` below, which asserts
/// the parse. It is NOT yet pinned behaviourally, and that gap is deliberate rather than
/// hidden: a behavioural test needs an exact-heart board (余剰ハート = 0) with 栞子 present
/// for her own trigger, and her printed hearts are (heart04 4, heart05 1, heart06 2) while
/// **no live card in the pool requires heart06 >= 2** — so every board she contributes to
/// has excess and lands on the −1 branch. Waiting her removes her hearts, and whether a
/// waited member's ライブ成功時 still fires has not been established. Until that is, a
/// score-based test would be a guess, and a guess that happens to pass is worse than no
/// test.
use crate::helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

#[test]
fn mifune_q231_excess_heart_2_score_cancels_to_0() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mifune = game.id("PL!N-bp5-010-R");
    let filler = game.id("PL!-sd1-010-SD");
    let filler_n = game.id("PL!N-sd1-001-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler_n);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(filler);
    }

    // Stage: 虹ヶ咲 members with hearts → excess hearts will exist
    let filler_n_copy2 = game.new_id("PL!N-sd1-001-SD");
    let filler_n_copy3 = game.new_id("PL!N-sd1-001-SD");
    game.state.player1.stage.stage = [filler_n, filler_n_copy2, filler_n_copy3];
    game.state.player1.hand.cards.push(mifune);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_set(&mut game);
    game.set_live_card(mifune);
    // Pass through to LiveVictoryDetermination where LiveSuccess triggers fire
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();

    game.drain_choices();

    let mod_val = game.state.mods.get_score_modifier(mifune);
    assert_eq!(
        mod_val, 0,
        "Score modifier should be 0 (+1 add -1 remove) with 2+ surplus hearts (got {})",
        mod_val
    );
}

#[test]
fn mifune_q231_excess_heart_2_score_cancels_to_0_all_rarities() {
    for suffix in ["AR", "P", "R"] {
        let db = load_real_database();
        let mut game = TestGame::new(db);
        let card_no = format!("PL!N-bp5-010-{suffix}");
        let mifune = game.id(&card_no);
        let filler = game.id("PL!-sd1-010-SD");
        let filler_n = game.id("PL!N-sd1-001-SD");

        game.state.player1.main_deck.cards.clear();
        for _ in 0..40 {
            game.state.player1.main_deck.cards.push(filler_n);
        }
        game.state.player2.main_deck.cards.clear();
        for _ in 0..40 {
            game.state.player2.main_deck.cards.push(filler);
        }
        let filler_n_copy2 = game.new_id("PL!N-sd1-001-SD");
        let filler_n_copy3 = game.new_id("PL!N-sd1-001-SD");
        game.state.player1.stage.stage = [filler_n, filler_n_copy2, filler_n_copy3];
        game.state.player1.hand.cards.push(mifune);
        game.state.player1.hand.cards.push(filler);

        advance_to_live_set(&mut game);
        game.set_live_card(mifune);
        game.advance_to_phase(rabuka_engine::game_state::Phase::FirstAttackerPerformance);
        game.drain_choices();

        assert_eq!(
            game.state.mods.get_score_modifier(mifune),
            0,
            "{card_no}: +1 and -1 must cancel"
        );
    }
}

/// What the 「余剰ハートを持たない」 branch parses to, and where its reading breaks.
///
/// **The correct half.** 「〜を持たない」 is the negated form, and the resource is right:
///
///     comparison_condition { resource_type: "surplus_heart", negation: true }
///
/// **The broken half.** It carries **no `count` and no `operator`**, and
/// `compare_counts(None, surplus, 0)` reads the missing operator as `>=` — `surplus >= 0`,
/// true on every board. That alone is a tautology.
///
/// But `negation` is NOT applied on the surplus comparison path, so the branch is in fact
/// decided by the bound alone, and this clause has no bound. The observable result is
/// "`surplus >= 0`": the `+1` fires at surplus 0, which is **correct** for that board and
/// is what `live_success_surplus_score_zero_floor_test` pins — and it also fires at
/// surplus >= 1, which is wrong (neither branch should apply there) and which no test
/// covers.
///
/// Adding `count: 1` + `operator: ">="` in the parser was tried and reverted: with
/// `negation` still ignored it just moves the error to surplus >= 1 and breaks the
/// surplus-0 case that is right. **The fix is engine-side — honour `negation` for
/// `surplus_heart` — and it has to land together with the parser half, not before it.**
///
/// So this test pins the two things that are correct, and records the shape of what is not.
/// It deliberately does not assert `count == None`: that would pin the defect as if it were
/// the intended contract, and the next person to read it would preserve it.
#[test]
fn the_no_excess_branch_is_negated_surplus_heart() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let shizuku = game.id("PL!N-bp5-010-R");
    let eff = game
        .db
        .get_card(shizuku)
        .unwrap()
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
        .and_then(|a| a.effect.as_ref().map(|e| e.clone()))
        .expect("ライブ成功時 ability has an effect");

    let acts = eff
        .compound
        .actions
        .as_ref()
        .expect("a sequential's steps live in compound.actions");
    let no_excess = acts
        .iter()
        .find(|s| {
            s.condition
                .as_ref()
                .and_then(|c| c.get_text())
                .is_some_and(|t| t.contains("余剰ハートを持たない"))
        })
        .expect("the 「余剰ハートを持たない」 branch");
    let cond = no_excess.condition.as_deref().expect("it is conditional");

    assert_eq!(
        cond.get_resource_type(),
        Some("surplus_heart"),
        "the compared quantity is 余剰ハート"
    );
    assert_eq!(
        cond.get_negation(),
        Some(true),
        "「〜を持たない」 is the negated reading. The engine does not apply it here, which \
         is the open defect this test documents — it must still be emitted, because the \
         engine-side fix is what makes it mean something"
    );

    // The sibling branch is the control: it carries its own bound, so it needs neither
    // negation nor an engine fix.
    let two_plus = acts
        .iter()
        .find(|s| {
            s.condition
                .as_ref()
                .and_then(|c| c.get_text())
                .is_some_and(|t| t.contains("2つ以上"))
        })
        .expect("the 「2つ以上持つ」 branch");
    let cond2 = two_plus.condition.as_deref().expect("it is conditional");
    assert_eq!(
        (cond2.get_count(), cond2.get_operator(), cond2.get_negation()),
        (Some(2), Some(">="), None),
        "「2つ以上持つ」 is a plain >= 2 with no negation — the shape that makes the other \
         branch's missing bound visible by comparison"
    );
}