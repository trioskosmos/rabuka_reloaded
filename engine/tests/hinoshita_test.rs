/// Tests for 日野下花帆 (PL!HS-bp5-001-R+) — Activation ability (ab#1):
///
/// {{kidou.png|起動}}{{turn1.png|ターン1回}}{{icon_energy.png|E}}{{icon_energy.png|E}}
/// 手札のライブカードを1枚公開する：自分の控え室から、これにより公開したカードの
/// カード名がすべて含まれるライブカードを1枚手札に加える。
///
/// Q236: Reveal "Dream Believers" → can recover "Dream Believers (104th Ver.)"
///       because the fragment "Dream Believers" is contained in the target name. ✓
/// Q237: Reveal "Dream Believers (104th Ver.)" → CANNOT recover "Dream Believers"
///       because the fragment "(104th Ver.)" is NOT contained in "Dream Believers".

mod helpers;
use helpers::*;

/// Q236: Name constraint "contains_all" — revealing a card whose name fragments
/// are ALL contained in a discard card allows recovery.
#[test]
fn hinoshita_q236_name_contains_all_matches() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hino = game.id("PL!HS-bp5-001-P");
    let reveal_live = game.id("PL!HS-bp1-019-L");     // "Dream Believers" — base name
    let target_live = game.id("PL!HS-bp5-017-L");      // "Dream Believers (104th Ver.)" — variant

    // Stage: 日野下花帆
    game.state.player1.stage.stage[1] = hino;

    // Hand: the live card to reveal
    game.state.player1.hand.cards.push(reveal_live);

    // Discard: the live card to recover
    game.state.player1.waitroom.cards.push(target_live);

    // Energy: 2 for activation cost
    game.give_energy(2);

    game.activate_ability(hino);

    // First choice: select the live card to reveal from hand
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Second choice: select the live card to recover from discard
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Target live card should be in hand now
    assert!(game.state.player1.hand.cards.contains(&target_live),
        "Q236: Should recover Dream Believers (104th Ver.) — name fragments match");
}

/// Q237: Name constraint "contains_all" — revealing a card whose name has
/// extra fragments NOT contained in a discard card prevents recovery.
#[test]
fn hinoshita_q237_name_contains_all_no_match() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hino = game.id("PL!HS-bp5-001-P");
    let reveal_live = game.id("PL!HS-bp5-017-L");      // "Dream Believers (104th Ver.)" — has extra text
    let target_live = game.id("PL!HS-bp1-019-L");       // "Dream Believers" — base name, no extra text

    game.state.player1.stage.stage[1] = hino;
    game.state.player1.hand.cards.push(reveal_live);
    game.state.player1.waitroom.cards.push(target_live);
    game.give_energy(2);

    game.activate_ability(hino);

    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // "Dream Believers" does NOT contain "(104th Ver.)", so it should NOT be recoverable
    assert!(!game.state.player1.hand.cards.contains(&target_live),
        "Q237: Should NOT recover Dream Believers — extra fragment '(104th Ver.)' not contained");
}

/// Negative: No matching live card in discard → ability fails.
#[test]
fn hinoshita_no_matching_live_in_discard_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hino = game.id("PL!HS-bp5-001-P");
    let reveal_live = game.id("PL!HS-bp1-019-L");       // "Dream Believers"
    let wrong_live = game.id("PL!-sd1-019-SD");          // different live card

    game.state.player1.stage.stage[1] = hino;
    game.state.player1.hand.cards.push(reveal_live);
    game.state.player1.waitroom.cards.push(wrong_live);
    game.give_energy(2);

    game.activate_ability(hino);

    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // After revealing, there should be no choice for discard selection
    // because the only discard live card doesn't match the name constraint
    // The card should NOT be in hand
    assert!(!game.state.player1.hand.cards.contains(&wrong_live),
        "Mismatched live card should not be recovered");
}
