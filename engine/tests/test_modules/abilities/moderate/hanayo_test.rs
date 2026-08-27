/// PL!-pb1-008-R (小泉花陽) ab#0 — Q183
///
/// {{toujyou.png|登場}}メンバーを3人までウェイトにしてもよい：
/// これによりウェイト状態にしたメンバー1人につき、カードを1枚引く。
///
/// Clauses under test:
/// - Q183: cost scope is SELF stage only — opponent members are never
///   candidates and are never waited.
/// - Q137: only ACTIVE members are valid cost targets; already-waited
///   members cannot be「ウェイトにする」again.
/// - Optional cost: skipping must wait nothing and draw nothing.
/// - 「これにより」scoping: the per-unit draw counts members newly waited
///   BY THIS COST, not members that were already waited beforehand.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::zones::MemberArea;

fn setup_hanayo_game(
    game: &mut TestGame,
    hanayo: i16,
    left_member: Option<i16>,
    opp_member: Option<i16>,
) {
    let filler = game.id_ref("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let mut left = -1;
    if let Some(m) = left_member {
        left = m;
    }
    game.state.player1.stage.stage = [left, -1, -1];
    if let Some(o) = opp_member {
        game.state.player2.stage.stage = [o, -1, -1];
    }
    game.add_to_hand(hanayo);
    game.give_energy(15);
}

#[test]
fn q183_debut_offers_pay_or_skip_prompt() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo = game.id("PL!-pb1-008-R");
    let friend = game.id("PL!-sd1-001-SD");
    setup_hanayo_game(&mut game, hanayo, Some(friend), None);

    game.play_to_stage(hanayo, MemberArea::Center);

    // The optional wait cost MUST present its pay/skip prompt — an absent
    // prompt would mean the cost was silently auto-paid or silently dropped.
    assert_ability!(
        game,
        "p1",
        game.has_pending_choice(),
        "debut must prompt for the optional wait cost"
    );
    match game.get_pending_choice() {
        Choice::SelectTarget { target, allow_skip, .. } => {
            assert_eq!(
                target, "pay_optional_cost:skip_optional_cost",
                "expected the pay/skip optional-cost prompt"
            );
            assert!(*allow_skip, "optional cost must be skippable");
        }
        other => panic!("expected SelectTarget(pay/skip), got {:?}", other),
    }

    // Skipping is a legal answer: nothing waited, nothing drawn.
    game.select_option(0);
    assert!(
        !game.has_pending_choice(),
        "skip resolves without further prompts"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(hanayo),
        None,
        "skipping the cost must not wait 花陽"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(friend),
        None,
        "skipping the cost must not wait other members"
    );
    // Hand: -1 for playing 花陽, +0 drawn.
    game.assert_hand(0, "skip draws nothing");
}

#[test]
fn q183_pay_waits_self_members_draws_one_each() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo = game.id("PL!-pb1-008-R");
    let friend = game.id("PL!-sd1-001-SD");
    let opp = game.new_id("PL!-sd1-002-SD");
    setup_hanayo_game(&mut game, hanayo, Some(friend), Some(opp));

    let deck_before = game.state.player1.main_deck.cards.len();
    game.play_to_stage(hanayo, MemberArea::Center);
    assert!(game.has_pending_choice(), "pay/skip prompt must appear");

    // Pay the optional cost (option 1).
    game.select_option(1);
    assert!(
        !game.has_pending_choice(),
        "with ≤3 candidates the wait applies automatically — no further prompts"
    );

    // Both own active members were waited by the cost…
    assert_eq!(
        game.state.mods.get_orientation_modifier(hanayo),
        Some("wait"),
        "花陽 herself is a valid cost target and must be waited"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(friend),
        Some("wait"),
        "own stage member must be waited"
    );
    // …but the OPPONENT's member must be untouched (Q183 self-only scope).
    assert_eq!(
        game.state.mods.get_orientation_modifier(opp),
        None,
        "Q183: opponent member must NEVER be waited by this cost"
    );
    assert_eq!(
        game.state.player2.stage.stage[0], opp,
        "opponent member stays on stage"
    );

    // Draw 1 per newly-waited member: waited 2 → drew 2.
    // Hand: -1 (played 花陽) +2 (draw) = +1 from the pre-play baseline of 1.
    game.assert_hand(2, "waited 2 members → drew 2");
    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        2,
        "exactly 2 cards left the deck"
    );
}

#[test]
fn q183_skip_waits_nothing_draws_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo = game.id("PL!-pb1-008-R");
    let friend = game.id("PL!-sd1-001-SD");
    setup_hanayo_game(&mut game, hanayo, Some(friend), None);

    let deck_before = game.state.player1.main_deck.cards.len();
    game.play_to_stage(hanayo, MemberArea::Center);
    assert!(game.has_pending_choice(), "pay/skip prompt must appear");

    game.select_option(0); // skip

    assert_eq!(game.state.mods.get_orientation_modifier(hanayo), None);
    assert_eq!(game.state.mods.get_orientation_modifier(friend), None);
    game.assert_hand(0, "-1 played, +0 drawn");
    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        0,
        "deck untouched when the cost is skipped"
    );
}

#[test]
fn q183_already_waited_members_do_not_add_draws() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo = game.id("PL!-pb1-008-R");
    let friend = game.id("PL!-sd1-001-SD");
    setup_hanayo_game(&mut game, hanayo, Some(friend), None);
    // Friend was ALREADY waited before the debut (e.g. by an earlier effect).
    game.state.mods.add_orientation_modifier(friend, "wait");

    game.play_to_stage(hanayo, MemberArea::Center);
    assert!(
        game.has_pending_choice(),
        "花陽 is active, so a cost candidate exists — prompt must appear"
    );

    // Pay: the only ACTIVE candidate is 花陽 herself (the already-waited
    // friend cannot be「ウェイトにする」again, Q137). Exactly ONE member is
    // newly waited, so exactly ONE card is drawn — the friend's pre-existing
    // wait must not inflate the count because the text says
    // 「これによりウェイト状態にしたメンバー1人につき」(per member waited BY THIS).
    game.select_option(1);

    assert_eq!(
        game.state.mods.get_orientation_modifier(hanayo),
        Some("wait"),
        "花陽 gets waited by the cost"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(friend),
        Some("wait"),
        "friend keeps her pre-existing wait"
    );
    game.assert_hand(1, "1 newly-waited member → drew exactly 1 (-1 played +1)");
}
