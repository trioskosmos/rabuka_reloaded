/// BP07 CLEAN-G16: PL!SP-bp7-028-L 未来の音が聴こえる ab#0 (ライブ開始時).
///
/// ライブ開始時：自分の控え室にある『Liella!』のメンバーカードを9枚選び、それらを
/// シャッフルし、デッキの一番下に置いてもよい。そうしたとき、ライブ終了時まで、
/// 自分のステージにいるすべてのメンバーはブレードを得る。
///
/// (Live start) Choose 9 『Liella!』 member cards in your discard, shuffle them and
/// may place them on the BOTTOM of the deck. If you do, until live end, all members
/// on your stage gain 1 blade.
///
/// Gameplay edge cases:
///   1. Accept with 9 Liella! members in discard → they move to the deck bottom and
///      all stage members gain blade.
///   2. Decline (Skip) → discard untouched, no blade.
///   3. Fewer than 9 Liella! members available → only the available ones move.
///   4. Non-『Liella!』 cards in discard are not moved.
///   5. The blade gain applies to ALL stage members (including non-Liella!).
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const MIRAI: &str = "PL!SP-bp7-028-L"; // live card, ab#0 ライブ開始時
const LIELLA: &str = "PL!SP-sd1-004-SD"; // 平安名すみれ (Liella! member)
const NON_LIELLA: &str = "PL!-sd1-010-SD"; // 高坂穂乃果 (μ's, not Liella!)
const STAGE_MEMBER: &str = "PL!N-bp1-001-R"; // a stage member to receive blade

fn trigger_live_start(game: &mut TestGame, card_id: i16) {
    let card = game.db.get_card(card_id).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .expect("card should have a ライブ開始時 ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(card_id),
        None,
        None,
    );
    game.state.activating_card = Some(card_id);
    game.state.process_pending_auto_abilities(&pid);
    game.drain_auto_ability_choices();
}

/// Put `n` Liella! members and `extra_non` non-Liella! cards in discard; put two
/// members on stage. Returns the two stage member ids.
fn setup(game: &mut TestGame, n_liella: usize, extra_non: usize) -> (i16, i16) {
    let mirai = game.id(MIRAI);
    game.state.player1.live_card_zone.cards.push(mirai);

    for _ in 0..n_liella {
        game.state.player1.waitroom.cards.push(game.id(LIELLA));
    }
    for _ in 0..extra_non {
        game.state.player1.waitroom.cards.push(game.id(NON_LIELLA));
    }

    let a = game.id(STAGE_MEMBER);
    let b = game.id(LIELLA);
    game.state.player1.stage.stage = [a, b, -1];
    (a, b)
}

fn blade_of(game: &TestGame, id: i16) -> i32 {
    game.state.mods.get_blade_modifier(id)
}

fn deck_bottom9(game: &TestGame) -> Vec<i16> {
    let d = &game.state.player1.main_deck.cards;
    let n = d.len();
    d[n.saturating_sub(9)..].to_vec()
}

/// Drain a conditional_optional (Skip/Pay) and then a SelectCard (which 9 to move).
fn resolve_accept(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 12 {
        guard += 1;
        match game.get_pending_choice() {
            rabuka_engine::ability::types::Choice::SelectTarget { .. } => {
                game.select_choice_option(1); // Pay / accept
            }
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                let cnt = game.pending_choice_count();
                let idxs: Vec<usize> = (0..cnt).collect();
                game.select_indices(&idxs);
            }
            _ => {
                game.select_indices(&[]);
            }
        }
    }
}

/// 1. Accept with 9 Liella! members in discard → they leave the discard and land on
///    the deck bottom; all stage members gain blade.
#[test]
fn mirai_accept_moves_9_to_deck_bottom_and_grants_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let (a, b) = setup(&mut game, 9, 0);
    let deck_before = game.state.player1.main_deck.cards.len();
    let discard_before = game.state.player1.waitroom.cards.len();

    let mirai = game.id_ref(MIRAI);
    trigger_live_start(&mut game, mirai);
    assert!(
        game.has_pending_choice(),
        "optional add-to-deck-bottom should be offered"
    );
    resolve_accept(&mut game);

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before - 9,
        "the 9 Liella! members should leave the discard"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before + 9,
        "9 cards added to the deck"
    );
    assert_eq!(deck_bottom9(&game).len(), 9, "9 cards on the deck bottom");
    assert_eq!(
        blade_of(&game, a),
        1,
        "stage member gains blade on accept"
    );
    assert_eq!(
        blade_of(&game, b),
        1,
        "the other stage member gains blade on accept"
    );
}

/// 2. Decline (Skip) → discard untouched, no blade.
#[test]
fn mirai_skip_leaves_discard_and_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let (a, _) = setup(&mut game, 9, 0);
    let discard_before = game.state.player1.waitroom.cards.len();
    let deck_before = game.state.player1.main_deck.cards.len();

    let mirai = game.id_ref(MIRAI);
    trigger_live_start(&mut game, mirai);
    assert!(game.has_pending_choice(), "optional should be offered");
    game.select_choice_option(0); // Skip
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before,
        "discard untouched on skip"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "deck untouched on skip"
    );
    assert_eq!(blade_of(&game, a), 0, "no blade on skip");
}

/// 3. Fewer than 9 Liella! members available (e.g. 5) → only the 5 are moved.
#[test]
fn mirai_accept_moves_available_when_fewer_than_9() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let (a, _) = setup(&mut game, 5, 0);
    let discard_before = game.state.player1.waitroom.cards.len();

    let mirai = game.id_ref(MIRAI);
    trigger_live_start(&mut game, mirai);
    assert!(game.has_pending_choice());
    resolve_accept(&mut game);

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before - 5,
        "the available 5 Liella! members are moved"
    );
    assert_eq!(
        blade_of(&game, a),
        1,
        "blade still granted for doing the optional"
    );
}

/// 4. Non-『Liella!』 cards in discard are NOT moved.
#[test]
fn mirai_non_liella_in_discard_not_moved() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // 9 Liella! + 3 non-Liella! in discard.
    let (a, _) = setup(&mut game, 9, 3);
    let discard_before = game.state.player1.waitroom.cards.len();

    let mirai = game.id_ref(MIRAI);
    trigger_live_start(&mut game, mirai);
    resolve_accept(&mut game);

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before - 9,
        "only the 9 Liella! members leave; the 3 non-Liella! cards stay"
    );
    // The non-Liella! cards remain in the discard.
    let waitroom: Vec<i16> = game.state.player1.waitroom.cards.iter().copied().collect();
    assert_eq!(waitroom.len(), 3, "3 non-Liella! cards remain in discard");
    let _ = a;
}

/// 5. The blade gain applies to ALL stage members (a non-Liella! stage member too).
#[test]
fn mirai_blade_applies_to_all_stage_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let (a, b) = setup(&mut game, 9, 0);
    // Add a non-Liella! stage member.
    let non_liella_stage = game.id(NON_LIELLA);
    game.state.player1.stage.stage[2] = non_liella_stage;

    let mirai = game.id_ref(MIRAI);
    trigger_live_start(&mut game, mirai);
    resolve_accept(&mut game);

    assert_eq!(blade_of(&game, a), 1, "member A gains blade");
    assert_eq!(blade_of(&game, b), 1, "member B gains blade");
    assert_eq!(
        blade_of(&game, non_liella_stage),
        1,
        "a non-Liella! stage member also gains blade"
    );
    let _ = MemberArea::Center;
}
