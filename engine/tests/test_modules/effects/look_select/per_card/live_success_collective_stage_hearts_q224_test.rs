/// Tests for Live with a smile! (LL-bp5-001-L) — LiveSuccess condition:
///
/// ライブ成功時: 2+ live cards yelled, OR 5+ heart colors on stage collectively,
/// OR a member moved areas this turn → score +1.
///
/// Q224: Hearts checked collectively across ALL stage members, not per-member.
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

#[test]
fn smile_q224_live_success_score_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let smile = game.id("LL-bp5-001-L");
    let filler = game.id("PL!-sd1-010-SD");
    let member1 = game.id("PL!S-sd1-003-SD");
    let member2 = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [member1, member2, -1];
    game.state.player1.hand.cards.push(smile);

    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(smile);
    advance_to_live_start(&mut game);

    game.pass();
    game.pass();
    game.pass();

    let l = &game.state.performance_snapshots[0].lives[0];
    assert_eq!(l.score - l.base_score, 1, "five distinct heart types add one score");
}

#[test]
fn smile_q224_five_icons_with_three_types_does_not_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let smile = game.id("LL-bp5-001-L");
    let filler = game.id("PL!-sd1-010-SD");
    let member1 = game.id("PL!S-sd1-003-SD");
    let member2 = game.id("PL!S-sd1-003-SD");

    game.state.player1.stage.stage = [member1, member2, -1];
    game.state.player1.hand.cards.push(smile);

    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(smile);
    advance_to_live_start(&mut game);

    game.pass();
    game.pass();
    game.pass();

    let l = &game.state.performance_snapshots[0].lives[0];
    assert_eq!(
        l.score - l.base_score,
        0,
        "five heart icons of only three types do not add score"
    );
}

/// The bound itself: 4 distinct types, one short of 「5種類以上」.
///
/// The two tests above stage **5** distinct types (qualifies) and **3** (does not), so
/// `>= 4`, `>= 5` and `> 4` all pass them identically — the boundary was never on the
/// board. This stages exactly 4.
///
/// Fixtures: `PL!S-PR-015-PR` 松浦果南 prints heart02+heart04, `PL!-sd1-010-SD` 高坂 穂乃果
/// prints heart01+heart03, and the two sets are disjoint, so the union is exactly four.
/// The union is asserted below rather than trusted, because a substituted printing would
/// move this card across the bound and invert the test.
#[test]
fn smile_q224_four_distinct_heart_types_is_one_short_and_does_not_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let smile = game.id("LL-bp5-001-L");
    let filler = game.id("PL!-sd1-010-SD");
    let a = game.id("PL!S-PR-015-PR"); // heart02, heart04
    let b = game.id("PL!-sd1-010-SD"); // heart01, heart03

    // Pin the premise from the card data: exactly four distinct types, so this is the
    // one-short case and not a duplicate of the 5-type test above.
    let types_of = |id: i16| -> std::collections::BTreeSet<String> {
        game.db
            .get_card(id)
            .and_then(|c| c.base_heart.as_ref())
            .map(|h| h.hearts.keys().map(|k| format!("{k:?}")).collect())
            .unwrap_or_default()
    };
    let union: std::collections::BTreeSet<String> =
        types_of(a).union(&types_of(b)).cloned().collect();
    assert_eq!(
        types_of(a).len(),
        2,
        "precondition: the first fixture prints exactly two heart types"
    );
    assert_eq!(
        types_of(b).len(),
        2,
        "precondition: the second fixture prints exactly two heart types"
    );
    assert_eq!(
        union.len(),
        4,
        "precondition: the two members must present exactly FOUR distinct heart types \
         (5種類以上 must NOT be met); got {union:?} from {a} and {b}. Two 2-element sets \
         yielding a 4-element union also proves they are disjoint."
    );

    game.state.player1.stage.stage = [a, b, -1];
    game.state.player1.hand.cards.push(smile);

    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(smile);
    advance_to_live_start(&mut game);

    game.pass();
    game.pass();
    game.pass();

    let l = &game.state.performance_snapshots[0].lives[0];
    assert_eq!(
        l.score - l.base_score,
        0,
        "5種類以上 — FOUR distinct types is one short, so no score is added. A +1 here means \
         the bound is being read as 4 or lower (score={}, base={})",
        l.score,
        l.base_score
    );
}
