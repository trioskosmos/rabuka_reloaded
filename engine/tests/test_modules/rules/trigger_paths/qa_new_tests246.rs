/// Tests for Q246 — LL-bp6-001-R＋ (南ことり&黒澤ダイヤ&徒町小鈴)
///
/// Live Start ability:
///   手札の「南ことり」と「黒澤ダイヤ」と「徒町小鈴」を、好きな枚数控え室に置いてもよい：
///   ライブ終了時まで、これにより控え室に置いたそれらのカードが持つハートの色1つにつき、
///   その色のハートを1つずつ得る。
///
/// Q246: Discard 2 cards with partial overlap {Red,Green} + {Green,Blue,Purple}.
///       → Gain Red, Green, Blue, Purple (1 each). Green counted once.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
}

fn finish_live_setup(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// Q246 main: partial overlap dedup.
/// Discard Dia(H02,H05) + Kosuzu(H04,H05,H06) → shared H05 counted once.
#[test]
fn test_q246_partial_overlap_dedup() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let joint = game.id("LL-bp6-001-R\u{ff0b}");
    let dia = game.id("PL!S-bp3-004-R"); // H02, H05
    let kosuzu = game.id("PL!HS-pb1-005-R"); // H04, H05, H06
    let live = game.id("PL!-sd1-010-SD");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = joint;
    game.add_to_hand(kosuzu);
    game.add_to_hand(dia);
    game.add_to_hand(live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(10);

    advance_to_live_start(&mut game);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    assert!(
        game.has_pending_choice(),
        "bp6 should prompt for any-number discard"
    );

    // Discard both eligible cards (indices 0=kosuzu, 1=dia)
    game.try_select_indices(&[0, 1]).unwrap();
    game.select_indices(&[]); // skip re-prompt, finalize

    assert!(
        !game.has_pending_choice(),
        "bp6 should resolve after selection"
    );

    // kosuzu: H04, H05, H06
    // dia:    H02, H05
    // Shared: H05 → counted once
    // Distinct: H02, H04, H05, H06 = 4
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(joint, HeartColor::Heart02),
        1,
        "H02 from dia"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(joint, HeartColor::Heart04),
        1,
        "H04 from kosuzu"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(joint, HeartColor::Heart05),
        1,
        "H05 shared — counted once"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(joint, HeartColor::Heart06),
        1,
        "H06 from kosuzu"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(joint, HeartColor::Heart01),
        0,
        "H01 not present"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(joint, HeartColor::Heart03),
        0,
        "H03 not present"
    );
}

/// Subset selection: 3 eligible cards, discard only 2.
/// Engine must NOT count the unselected card's hearts.
#[test]
fn test_q246_subset_selection() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let joint = game.id("LL-bp6-001-R\u{ff0b}");
    let kosuzu = game.id("PL!HS-pb1-005-R"); // H04, H05, H06
    let dia = game.id("PL!S-bp3-004-R"); // H02, H05
    let kotori = game.id("PL!-bp3-003-R"); // H01, H03, H06
    let live = game.id("PL!-sd1-010-SD");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = joint;
    game.add_to_hand(kosuzu);
    game.add_to_hand(dia);
    game.add_to_hand(kotori);
    game.add_to_hand(live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(10);

    advance_to_live_start(&mut game);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    assert!(
        game.has_pending_choice(),
        "bp6 should prompt for any-number discard"
    );

    // Only select kosuzu (idx 0) and dia (idx 1). Skip kotori (idx 2).
    game.try_select_indices(&[0, 1]).unwrap();

    // any_number re-prompts after each non-empty selection; skip to finalize.
    assert!(
        game.has_pending_choice(),
        "any_number re-prompt for more cards"
    );
    game.select_indices(&[]);

    assert!(!game.has_pending_choice(), "bp6 should resolve after skip");

    // kosuzu (selected): H04, H05, H06
    // dia (selected):    H02, H05
    // kotori (NOT selected): should NOT contribute H01 or H03
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(joint, HeartColor::Heart02),
        1,
        "H02 from dia"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(joint, HeartColor::Heart04),
        1,
        "H04 from kosuzu"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(joint, HeartColor::Heart05),
        1,
        "H05 shared"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(joint, HeartColor::Heart06),
        1,
        "H06 from kosuzu"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(joint, HeartColor::Heart01),
        0,
        "H01 NOT from unselected kotori"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(joint, HeartColor::Heart03),
        0,
        "H03 NOT from unselected kotori"
    );
}

/// Single card discard: only 1 eligible card, discard it.
#[test]
fn test_q246_single_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let joint = game.id("LL-bp6-001-R\u{ff0b}");
    let kosuzu = game.id("PL!HS-pb1-005-R"); // H04, H05, H06
    let live = game.id("PL!-sd1-010-SD");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = joint;
    game.add_to_hand(kosuzu);
    game.add_to_hand(live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(10);

    advance_to_live_start(&mut game);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    assert!(
        game.has_pending_choice(),
        "bp6 should prompt for any-number discard"
    );

    // Discard the only eligible card
    game.try_select_indices(&[0]).unwrap();
    game.select_indices(&[]); // skip re-prompt, finalize

    assert!(
        !game.has_pending_choice(),
        "bp6 should resolve after selection"
    );

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(joint, HeartColor::Heart04),
        1,
        "H04 from kosuzu"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(joint, HeartColor::Heart05),
        1,
        "H05 from kosuzu"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(joint, HeartColor::Heart06),
        1,
        "H06 from kosuzu"
    );

    let total: i32 = [
        HeartColor::Heart01,
        HeartColor::Heart02,
        HeartColor::Heart03,
        HeartColor::Heart04,
        HeartColor::Heart05,
        HeartColor::Heart06,
    ]
    .iter()
    .map(|&c| game.state.mods.get_heart_modifier(joint, c))
    .sum();
    assert_eq!(total, 3, "3 distinct hearts from single kosuzu card");
}
