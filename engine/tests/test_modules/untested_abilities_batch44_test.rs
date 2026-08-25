/// Untested-abilities batch 44 — area-tagged debuts, self-wait draw engine,
/// KALEIDOSCORE trio constant.
///
/// - PL!SP-pb2-036-N 嵐千砂都 (登場・右サイド): draw 2, discard 2 from hand.
/// - PL!SP-pb2-037-N 平安名すみれ (登場・左サイド): mirror twin.
/// - PL!N-bp7-023-N ミア・テイラー (起動 ターン1回, self-wait): draw 2,
///   discard 2.
/// - PL!SP-bp7-013-N 唐可可 (常時): while 3 『KALEIDOSCORE』 members are
///   staged -> heart06 + blade.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

fn fire_live_start(game: &mut TestGame, cid: i16) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap_or_else(|| panic!("card {} lacks a ライブ開始時 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

// ====================================================================
// PL!SP-pb2-036-N / pb2-037-N — right/left-side debut draw2+discard2
// ====================================================================

#[test]
fn chisato_right_side_debut_draws_two_discards_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.new_id("PL!SP-pb2-036-N");
    let keep_a = game.new_id("PL!S-sd1-001-SD");
    let keep_b = game.new_id("PL!N-bp3-006-R");
    game.add_to_hand(me);
    game.add_to_hand(keep_a);
    game.add_to_hand(keep_b);
    // Deck top: the two cards that will be drawn.
    let d1 = game.new_id("PL!-sd1-007-SD");
    let d2 = game.new_id("PL!-sd1-001-SD");
    game.state.player1.main_deck.cards.insert(0, d2);
    game.state.player1.main_deck.cards.insert(0, d1);
    game.give_energy(20);

    game.play_to_stage(me, MemberArea::RightSide);

    // Answer hand-discard selection(s).
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        0,
        "sanity: no stray modifiers"
    );
    assert!(
        game.state.player1.hand.cards.contains(&d1)
            && game.state.player1.hand.cards.contains(&d2),
        "both drawn cards reached the hand"
    );
}

#[test]
fn sumire_left_side_debut_draws_two_discards_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.new_id("PL!SP-pb2-037-N");
    game.add_to_hand(me);
    let keep = game.new_id("PL!-sd1-007-SD");
    game.add_to_hand(keep);
    let d1 = game.new_id("PL!-sd1-001-SD");
    let d2 = game.new_id("PL!-sd1-004-SD");
    game.state.player1.main_deck.cards.insert(0, d2);
    game.state.player1.main_deck.cards.insert(0, d1);
    game.give_energy(20);
    let filler_deck_len = game.state.player1.main_deck.cards.len();

    game.play_to_stage(me, MemberArea::LeftSide);

    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    // Net effect: deck -2, and exactly two of the three known cards
    // (d1, d2, keep) were discarded to the waitroom.
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        filler_deck_len - 2,
        "drew exactly 2"
    );
    let survivors = [&keep, &d1, &d2]
        .iter()
        .filter(|c| game.state.player1.hand.cards.contains(c))
        .count();
    assert_eq!(survivors, 1, "exactly one of the three remains in hand");
}

#[test]
fn mia_activation_self_wait_draws_two_discards_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let mia = game.id("PL!N-bp7-023-N");
    game.state.player1.stage.stage[1] = mia;

    let d1 = game.new_id("PL!-sd1-001-SD");
    let d2 = game.new_id("PL!S-sd1-001-SD");
    game.state.player1.main_deck.cards.insert(0, d2);
    game.state.player1.main_deck.cards.insert(0, d1);
    let h1 = game.new_id("PL!-sd1-007-SD");
    let h2 = game.new_id("PL!-sd1-004-SD");
    game.add_to_hand(h1);
    game.add_to_hand(h2);

    game.activate_ability(mia);
    assert_eq!(
        game.state.mods.get_orientation_modifier(mia),
        Some("wait"),
        "activation cost waits this member"
    );

    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&d1)
            && game.state.player1.hand.cards.contains(&d2),
        "drawn cards reached the hand"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&h1)
            && game.state.player1.waitroom.cards.contains(&h2),
        "two hand cards discarded to the waitroom"
    );
}

// ====================================================================
// PL!SP-bp7-013-N 唐可可 — KALEIDOSCORE trio constant
// ====================================================================

fn koko_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let koko = game.id("PL!SP-bp7-013-N");
    game.state.player1.stage.stage[0] = koko;
    koko
}

#[test]
fn koko_three_kaleidoscore_grants_heart06_and_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let koko = koko_setup(&mut game);
    let k2 = game.id("PL!SP-bp1-013-PR"); // KALEIDOSCORE 澄香? cost 9
    let k3 = game.id("PL!SP-PR-012-PR"); // KALEIDOSCOPE ウィーン・マルガレーテ
    game.state.player1.stage.stage[1] = k2;
    game.state.player1.stage.stage[2] = k3;

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_heart_modifier(koko, HeartColor::Heart06),
        1,
        "3 KALEIDOSCORE members -> heart06 granted"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(koko),
        1,
        "3 KALEIDOSCORE members -> blade granted"
    );
}

#[test]
fn koko_only_two_kaleidoscope_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let koko = koko_setup(&mut game);
    let k2 = game.id("PL!SP-bp1-013-PR");
    game.state.player1.stage.stage[1] = k2;
    let outsider = game.id("PL!-sd1-010-SD"); // μ's
    game.state.player1.stage.stage[2] = outsider;

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_heart_modifier(koko, HeartColor::Heart06),
        0,
        "only 2 KALEIDOSCORE -> no heart06"
    );
    assert_eq!(game.state.mods.get_blade_modifier(koko), 0);
}
