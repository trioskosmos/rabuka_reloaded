/// Tests for MIRACLE WAVE (PL!S-bp3-019-L) — LiveSuccess set_score:
///
/// ライブ成功時 このターン、エールにより公開された自分のカードの中に
/// ブレードハートを持たないカードが0枚の場合か、または自分が余剰ハートを
/// 2つ以上持っている場合、このカードのスコアは４になる。
///
/// OR condition: A (no blade heart = 0) OR B (excess >= 2) → score = 4.
/// Q182: Either condition met → score = 4.
use crate::helpers::*;
use rabuka_engine::card::{BaseHeart, HeartColor, HeartMap};

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// Q182: Excess heart >= 2 (condition B) → score set to 4.
/// Stage: 2x PL!S-sd1-001-SD (千歌: h02=3, h04=2, h05=2 × 2 = 6,4,4)
/// Need: h02=4, h04=4, h05=4. Total given: 14. Excess: 2. ✓
#[test]
fn miracle_wave_q182_excess_heart_score_4() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wave = game.id("PL!S-bp3-019-L");
    let filler = game.id("PL!-sd1-010-SD");

    // Two distinct instances of the same cheer card: the same id in both slots
    // is not a board that can occur, and anything counting members would be
    // measuring that impossible state.
    game.state.player1.stage.stage = [
        game.id("PL!S-sd1-001-SD"),
        game.new_id("PL!S-sd1-001-SD"),
        -1,
    ];
    game.state.player1.hand.cards.push(wave);

    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(wave);
    advance_to_live_start(&mut game);

    game.pass(); // → SecondAttackerPerformance
    game.drain_auto_ability_choices();
    game.pass(); // → LiveVictoryDetermination (set)
    game.drain_auto_ability_choices();
    game.pass(); // → Active (processes LiveVictoryDetermination)
    game.drain_auto_ability_choices();

    assert_eq!(
        game.state.mods.get_score_modifier(wave),
        0,
        "LiveSuccess set_score cleared after live"
    );
    let l = game.state.performance_snapshots[0]
        .lives
        .iter()
        .find(|l| l.card_id == wave)
        .unwrap();
    assert_eq!(l.score, 4, "Live score should be 4");
}

/// Drive MIRACLE WAVE to a completed live whose hearts match its requirement EXACTLY, so
/// 余剰ハート = 0 and condition **B** (excess >= 2) cannot fire.
///
/// MIRACLE WAVE needs heart02/04/05 = 4/4/4 (`cards.json` `need_heart`). Each pair below
/// sums to exactly that:
///
/// | member | base (h02,h04,h05) | blade heart |
/// |---|---|---|
/// | `PL!S-bp3-001-P` 高海千歌 | 3,2,2 | **yes** |
/// | `PL!S-sd1-001-SD` 高海千歌 | 3,2,2 | no |
/// | `PL!S-bp2-011-N` 桜内梨子 | 1,2,2 | **yes** (`b_heart04`) |
/// | `PL!S-bp3-003-P` 松浦果南 | 1,2,2 | no |
///
/// So `(3,2,2)+(1,2,2) = (4,4,4)`, excess 0, and the only variable between the two tests is
/// whether the cards yelled carry a blade heart. That is what the existing test cannot do: it
/// stages two 千歌 for 6,4,4, an excess of 2, so B is true there and the score of 4 says
/// nothing about which branch produced it.
///
/// An earlier attempt to force this by assigning `player1.stage_hearts` directly did **not**
/// work — the value is recomputed from the members' base hearts, so the override was ignored
/// and the excess stayed at 2. These are real cards, so no override is needed.
fn miracle_wave_with_exact_hearts(game: &mut TestGame, blade: bool) -> i16 {
    let db = game.db.clone();
    let _ = db;
    let wave = game.id("PL!S-bp3-019-L");
    let filler = game.id("PL!-sd1-010-SD");

    let (a_no, b_no) = if blade {
        ("PL!S-bp3-001-P", "PL!S-bp2-011-N")
    } else {
        ("PL!S-sd1-001-SD", "PL!S-bp3-003-P")
    };
    let a = game.id(a_no);
    let b = game.id(b_no);

    // Pin the premise: the pair sums to exactly the requirement, and every card's blade
    // presence matches which branch this run is meant to exercise.
    let want_blade = blade;
    for (label, id) in [("A", a), ("B", b)] {
        let c = game.db.get_card(id).unwrap();
        assert_eq!(
            c.blade_heart.is_some(),
            want_blade,
            "precondition: card {label} ({id}) blade-heart presence must be {want_blade} for \
             this branch — the two tests differ ONLY in blade presence"
        );
    }
    let mut total = [0u8; 3];
    for id in [a, b] {
        let h = &game.db.get_card(id).unwrap().base_heart.as_ref().unwrap().hearts;
        total[0] += h.get(&HeartColor::Heart02).copied().unwrap_or(0);
        total[1] += h.get(&HeartColor::Heart04).copied().unwrap_or(0);
        total[2] += h.get(&HeartColor::Heart05).copied().unwrap_or(0);
    }
    assert_eq!(
        total, [4, 4, 4],
        "precondition: the stage must supply exactly MIRACLE WAVE's 4/4/4 requirement so \
         余剰ハート is 0 and condition B cannot fire"
    );

    game.state.player1.stage.stage = [a, b, -1];
    game.state.player1.hand.cards.push(wave);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(game);
    game.set_live_card(wave);
    advance_to_live_start(game);
    game.pass();
    game.drain_auto_ability_choices();
    game.pass();
    game.drain_auto_ability_choices();
    game.pass();
    game.drain_auto_ability_choices();
    wave
}

/// Condition **A** alone: every card yelled has a blade heart, so the count of
/// ブレードハートを持たないカード is 0, and the score becomes 4 with excess at 0.
///
/// The two tests below use cards with identical hearts, differing only in blade heart, so
/// whichever clause moves the score must be the blade one.
#[test]
fn miracle_wave_a_lone_no_blade_heart_count_of_zero_scores_four_with_no_excess() {
    let mut game = TestGame::new(load_real_database());
    let wave = miracle_wave_with_exact_hearts(&mut game, true);

    let l = game.state.performance_snapshots[0]
        .lives
        .iter()
        .find(|l| l.card_id == wave)
        .unwrap();
    assert_eq!(
        l.score, 4,
        "ブレードハートを持たないカードが0枚 — the only cards yelled both have blade hearts, \
         so the count is 0 and the OR's first branch alone must set the score to 4. 余剰ハート \
         is 0 here, so the second branch cannot explain it. (score={})",
        l.score
    );
}

/// Condition **A** false and condition **B** false — the OR is false, so the printed score
/// stands.
///
/// This is the branch nothing covered. The existing test stages two blade-less 千歌 for an
/// excess of 2, so B fires and the score is 4 whether or not A is ever evaluated. Here both
/// members are blade-less (A false) and the hearts match exactly (B false). An implementation
/// that treated the OR as always-true, or that read 「0枚」 as 「any number of cards」, fails
/// here and passes everything else.
#[test]
fn miracle_wave_when_both_conditions_are_false_the_printed_score_stands() {
    let mut game = TestGame::new(load_real_database());
    let wave = miracle_wave_with_exact_hearts(&mut game, false);

    let printed = game
        .db
        .get_card(wave)
        .and_then(|c| c.score)
        .expect("MIRACLE WAVE prints a score");
    let l = game.state.performance_snapshots[0]
        .lives
        .iter()
        .find(|l| l.card_id == wave)
        .unwrap();
    assert_eq!(
        l.score, printed,
        "A is false (two blade-less cards were yelled) and B is false (excess 0), so neither \
         branch of the OR applies and the card keeps its printed score of {printed}. A score \
         of 4 here means the OR fires unconditionally — score={}",
        l.score
    );
}
