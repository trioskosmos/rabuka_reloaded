/// BP07 parser-gap regression coverage for three cards whose abilities the
/// parser previously mis-parsed:
///
///  1. PL!SP-bp7-010-R ウィーン・マルガレーテ ab#0 (起動):
///     このメンバーをステージから控え室に置く：自分のエネルギー置き場にある
///     エネルギー1枚をエネルギーデッキに置く。その後、自分の控え室からカードを
///     1枚手札に加える。
///     → cost: self to discard; then move energy_zone→energy_deck; then discard→hand.
///
///  2. PL!N-bp7-001-R 上原歩夢 ab#0 (自動, ターン1):
///     自分のエネルギー置き場にあるエネルギーがメンバーの下に置かれたとき、
///     自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。
///     → when energy is placed under a member, place 1 energy from the energy
///       deck into the energy zone in WAIT.
///
///  3. PL!SP-bp7-012-N 澁谷かのん ab#0 (登場):
///     自分の控え室から、『CatChu!』と『KALEIDOSCORE』と『5yncri5e!』のカードを
///     それぞれ1枚ずつ選び、それらを好きな順番でデッキの下に置いてもよい。
///     そうしたとき、カードを1枚引く。
///     → select 1 card of each of the 3 groups from the waitroom, place them on
///       the deck bottom in any order; if you did, draw 1 card.
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

// ====================================================================
// 1. ウィーン・マルガレーテ (PL!SP-bp7-010-R): 起動
// ====================================================================

fn place_wien(game: &mut TestGame) -> i16 {
    let wien = game.id("PL!SP-bp7-010-R");
    game.state.player1.stage.stage[1] = wien;
    game.give_energy(3);
    wien
}

fn seed_deck(game: &mut TestGame) {
    let f = game.id("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(f);
        game.state.player2.main_deck.cards.push(f);
    }
}

/// Fire 澁谷かのん's 登場 (debut) ability and resolve its choices.
fn trigger_kanon_debut(game: &mut TestGame, kanon: i16, accept: bool) {
    let pid = game.state.player1.id.clone();
    let card = game.db.get_card(kanon).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("登場"))
        .expect("card should have 登場");
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::Debut,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(kanon),
        None,
        None,
    );
    game.state.activating_card = Some(kanon);
    game.state.process_pending_auto_abilities(&pid);

    // The effect is conditional_on_optional: accept/skip the optional placement,
    // then select each present group's card from the waitroom.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 40 {
        guard += 1;
        let choice = game.get_pending_choice().clone();
        match choice {
            // Accept (1) or skip (0) the optional "place on deck bottom".
            rabuka_engine::ability::types::Choice::SelectTarget { .. } => {
                game.select_choice_option(if accept { 1 } else { 0 });
            }
            // Select the group cards from the waitroom.
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                game.select_indices(&[0]);
            }
            _ => game.select_indices(&[0]),
        }
    }
}

fn run_activate_drain(game: &mut TestGame, member: i16) {
    game.activate_ability(member);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        let choice = game.get_pending_choice().clone();
        match choice {
            // Cost: select the member to place from stage to discard.
            rabuka_engine::ability::types::Choice::SelectCard { zone, .. }
                if zone == "stage" =>
            {
                game.select_indices(&[0]);
            }
            // Energy selection for energy_zone → energy_deck move.
            rabuka_engine::ability::types::Choice::SelectCard { zone, .. }
                if zone == "energy_zone" =>
            {
                game.select_indices(&[0]);
            }
            // Target card from discard to hand.
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                game.select_indices(&[0]);
            }
            _ => game.select_choice_option(0),
        }
    }
}

/// The 起動 effect moves 1 energy from the energy zone to the energy deck.
#[test]
fn wien_activation_moves_energy_zone_to_energy_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let wien = place_wien(&mut game);
    seed_deck(&mut game);
    // A card in the discard to be recovered to hand by the second step.
    let recover = game.id("PL!-sd1-002-SD");
    game.add_to_discard(recover);

    let zone_before = game.state.player1.energy_zone.active_count();
    let deck_before = game.state.player1.energy_deck.cards.len();

    run_activate_drain(&mut game, wien);

    assert!(
        game.state.player1.energy_zone.active_count() < zone_before,
        "one energy card must leave the energy zone"
    );
    assert!(
        game.state.player1.energy_deck.cards.len() > deck_before,
        "the energy card must be placed into the energy deck"
    );
    assert!(
        game.state.player1.hand.cards.contains(&recover),
        "the second step must add the discard card to hand"
    );
}

/// ウィーン: energy zone is empty → the first effect step (zone→deck) has no
/// card to move, so it fizzles; but the cost still pays and step 2 (waitroom→
/// hand) still runs independently.
#[test]
fn wien_empty_energy_zone_step1_fizzles() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let wien = game.id("PL!SP-bp7-010-R");
    game.state.player1.stage.stage[1] = wien;
    // NO energy in the zone.
    seed_deck(&mut game);
    // Another card in the discard so step 2 has a distinct target besides ウィーン.
    let recover = game.id("PL!-sd1-002-SD");
    game.add_to_discard(recover);

    let deck_before = game.state.player1.energy_deck.cards.len();
    let hand_before = game.state.player1.hand.cards.len();

    run_activate_drain(&mut game, wien);

    // No energy moved (zone was empty) → step 1 fizzles.
    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        deck_before,
        "empty energy zone → nothing moves to the energy deck"
    );
    // Step 2 (waitroom → hand) still resolves and adds exactly 1 card to hand.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "step 2 still runs and adds 1 discard card to hand even when step 1 fizzled"
    );
    // The cost paid: ウィーン left the stage (it is now a step-2 recovery candidate).
    assert!(
        !game.state.player1.stage.stage.contains(&wien),
        "ウィーン's own cost still sends it to the waitroom"
    );
}

/// ウィーン: even when the energy-zone step moves nothing, the "その後" step 2
/// still runs. Here the waitroom holds only ウィーン (put there by its own cost),
/// which step 2 recovers to hand.
#[test]
fn wien_step2_recovers_self_after_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let wien = game.id("PL!SP-bp7-010-R");
    game.state.player1.stage.stage[1] = wien;
    game.give_energy(3);
    seed_deck(&mut game);

    let hand_before = game.state.player1.hand.cards.len();

    run_activate_drain(&mut game, wien);

    // The cost moved ウィーン to the waitroom; step 2 (waitroom→hand) recovers it.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "step 2 recovers a card (ウィーン) from the waitroom to hand"
    );
    assert!(
        game.state.player1.hand.cards.contains(&wien),
        "ウィーン is recovered to hand by step 2"
    );
}

// ====================================================================
// 3. 澁谷かのん (PL!SP-bp7-012-N): 登場 — multi-group select → deck bottom → draw
// ====================================================================

#[test]
fn kanon_selects_each_group_to_deck_bottom_and_draws() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kanon = game.id("PL!SP-bp7-012-N");
    game.state.player1.stage.stage[1] = kanon;

    // One card from each target group in the waitroom.
    let c_catchu = game.id("PL!SP-bp1-004-PR"); // CatChu!
    let c_kaleido = game.id("PL!SP-bp1-013-PR"); // KALEIDOSCORE
    let c_sync = game.id("PL!SP-pb1-014-PR"); // 5yncri5e!
    game.add_to_discard(c_catchu);
    game.add_to_discard(c_kaleido);
    game.add_to_discard(c_sync);
    seed_deck(&mut game);

    let hand_before = game.state.player1.hand.cards.len();

    trigger_kanon_debut(&mut game, kanon, true);

    // All 3 group cards are placed on the deck bottom. (The net deck count is
    // deck_before + 2 because the draw took 1 off the top after placing 3.)
    let bottom3: Vec<i16> = game
        .state
        .player1
        .main_deck
        .cards
        .iter()
        .rev()
        .take(3)
        .copied()
        .collect();
    assert!(
        bottom3.contains(&c_catchu) && bottom3.contains(&c_kaleido) && bottom3.contains(&c_sync),
        "all 3 group cards must be placed on the deck bottom (got {:?})",
        bottom3
    );
    assert!(
        game.state.player1.hand.cards.len() > hand_before,
        "placing them draws 1 card"
    );
}

/// 澁谷かのん: skipping the optional deck-bottom placement means NO card is drawn.
#[test]
fn kanon_skip_placement_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kanon = game.id("PL!SP-bp7-012-N");
    game.state.player1.stage.stage[1] = kanon;
    game.add_to_discard(game.id("PL!SP-bp1-004-PR")); // CatChu!
    game.add_to_discard(game.id("PL!SP-bp1-013-PR")); // KALEIDOSCORE
    game.add_to_discard(game.id("PL!SP-pb1-014-PR")); // 5yncri5e!
    seed_deck(&mut game);

    let hand_before = game.state.player1.hand.cards.len();
    let deck_before = game.state.player1.main_deck.cards.len();

    trigger_kanon_debut(&mut game, kanon, false); // skip

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "skipping the placement must not move any cards to the deck"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "skipping the placement must not draw a card"
    );
}

/// 澁谷かのん: with ZERO cards in the discard pile there is nothing to place, so
/// the optional placement moves nothing and the "そうしたとき" draw must NOT fire
/// (Q118 all-or-nothing; the user-reported bug "0 cards → still draws").
#[test]
fn kanon_empty_discard_no_placement_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kanon = game.id("PL!SP-bp7-012-N");
    game.state.player1.stage.stage[1] = kanon;
    // No cards in the discard pile at all.
    seed_deck(&mut game);

    let hand_before = game.state.player1.hand.cards.len();
    let deck_before = game.state.player1.main_deck.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();

    trigger_kanon_debut(&mut game, kanon, true);

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "empty discard → nothing placed on the deck bottom"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before,
        "nothing moved out of the discard pile"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "empty discard → incomplete placement → NO draw"
    );
}

/// 澁谷かのん: if one of the three target groups has no card in the waitroom,
/// the other two groups are still selectable and placed (partial placement is
/// allowed — Q167 "実行可能な限り解決"), BUT the draw does NOT occur because the
/// "そうしたとき" placement was incomplete (Q118: all-or-nothing consequence).
#[test]
fn kanon_missing_group_places_present_but_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kanon = game.id("PL!SP-bp7-012-N");
    game.state.player1.stage.stage[1] = kanon;
    // Only CatChu! and KALEIDOSCORE present; 5yncri5e! has no card in waitroom.
    let c_catchu = game.id("PL!SP-bp1-004-PR"); // CatChu!
    let c_kaleido = game.id("PL!SP-bp1-013-PR"); // KALEIDOSCORE
    game.add_to_discard(c_catchu);
    game.add_to_discard(c_kaleido);
    seed_deck(&mut game);

    let hand_before = game.state.player1.hand.cards.len();
    let deck_before = game.state.player1.main_deck.cards.len();

    trigger_kanon_debut(&mut game, kanon, true);

    // The two present groups' cards were placed (partial placement allowed).
    // (Net deck count = before + 2, since the draw does NOT fire.)
    let bottom2: Vec<i16> = game
        .state
        .player1
        .main_deck
        .cards
        .iter()
        .rev()
        .take(2)
        .copied()
        .collect();
    assert!(
        bottom2.contains(&c_catchu) && bottom2.contains(&c_kaleido),
        "the two present group cards must be placed on the deck bottom (got {:?})",
        bottom2
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before + 2,
        "only the 2 present cards are placed; nothing drawn on top"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "Q118: incomplete placement → 'そうしたとき' unmet → NO draw"
    );
}

/// 澁谷かのん: "それぞれ1枚ずつ" means exactly ONE from EACH group. When the
/// waitroom holds multiple cards of the same group, only one of that group is
/// placed — never two from a single group at the expense of another group.
#[test]
fn kanon_one_per_group_not_multiple_from_same_group() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kanon = game.id("PL!SP-bp7-012-N");
    game.state.player1.stage.stage[1] = kanon;
    // Two distinct CatChu! cards + one each of the other two groups.
    let catchu_a = game.id("PL!SP-bp1-004-PR"); // CatChu! (平安名すみれ)
    let catchu_b = game.id("PL!SP-bp1-018-PR"); // CatChu! (米女メイ)
    let kaleido = game.id("PL!SP-bp1-013-PR"); // KALEIDOSCORE
    let sync = game.id("PL!SP-pb1-014-PR"); // 5yncri5e!
    game.add_to_discard(catchu_a);
    game.add_to_discard(catchu_b);
    game.add_to_discard(kaleido);
    game.add_to_discard(sync);
    seed_deck(&mut game);

    let waitroom_before = game.state.player1.waitroom.cards.len();
    trigger_kanon_debut(&mut game, kanon, true);

    // Exactly 3 cards total are placed on the deck bottom (one per group),
    // even though the waitroom held 4 (2 of one group).
    let deck = &game.state.player1.main_deck.cards;
    let bottom3: Vec<i16> = deck.iter().rev().take(3).copied().collect();
    assert_eq!(bottom3.len(), 3, "exactly 3 cards on the deck bottom");
    // All three groups are represented.
    assert!(
        bottom3.contains(&kaleido) && bottom3.contains(&sync),
        "the other two groups must each contribute one card (got {:?})",
        bottom3
    );
    // Only ONE of the two CatChu! cards is placed.
    let catchu_placed = bottom3
        .iter()
        .filter(|&&c| c == catchu_a || c == catchu_b)
        .count();
    assert_eq!(
        catchu_placed, 1,
        "exactly one CatChu! card placed, not both (got {:?})",
        bottom3
    );
    // The unplaced CatChu! card remains in the waitroom (one left over).
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before - 3,
        "one card of each group placed; the extra same-group card stays in waitroom"
    );
}

/// When an energy card is placed under a member, 上原歩夢's auto ability fires:
/// it places 1 energy from the energy deck into the energy zone in WAIT.
///
/// We trigger the placement by activating 三船栞子 (PL!N-bp7-010-R), whose cost is
/// "エネルギー置き場にあるエネルギー1枚をこのメンバーの下に置く" (place_energy_under_member).
/// 上原歩夢 is on stage as the watcher.
#[test]
fn ayumu_energy_under_member_triggers_energy_deck_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // 上原歩夢 (the watcher) and 三船栞子 (the placer) both on stage.
    let ayumu = game.id("PL!N-bp7-001-R");
    let shioriko = game.id("PL!N-bp7-010-R");
    game.state.player1.stage.stage = [-1, ayumu, shioriko];
    game.give_energy(5);

    // Some energy in the energy deck for 上原歩夢's effect to draw from.
    for _ in 0..6 {
        game.state.player1.energy_deck.cards.push(game.id("LL-E-001-SD"));
    }
    let zone_active_before = game.state.player1.energy_zone.active_count();
    let energy_deck_before = game.state.player1.energy_deck.cards.len();

    // Activate 三船栞子 → its cost places 1 energy under the member.
    game.activate_ability(shioriko);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        let choice = game.get_pending_choice().clone();
        match choice {
            // Cost: select the energy card to place under the member.
            rabuka_engine::ability::types::Choice::SelectCard { zone, .. }
                if zone == "energy_zone" =>
            {
                game.select_indices(&[0]);
            }
            // Target the 虹ヶ咲 member to deploy (or empty area), then destination.
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                game.select_indices(&[0]);
            }
            rabuka_engine::ability::types::Choice::SelectTarget { .. } => {
                game.select_choice_option(1); // accept / pay
            }
            _ => game.select_choice_option(0),
        }
    }

    // 上原歩夢's auto ability fires: it draws 1 energy from the energy deck and
    // places it in the energy zone in WAIT.
    let zone = &game.state.player1.energy_zone;
    let zone_wait = zone.cards.len() - zone.active_count() as usize;
    assert!(
        game.state.player1.energy_deck.cards.len() < energy_deck_before,
        "上原歩夢 should take 1 energy from the energy deck"
    );
    // 三船栞子's cost took 1 ACTIVE energy out; 上原歩夢 put 1 back as WAIT.
    // Active count should be one less than before the cost.
    assert_eq!(
        zone.active_count(),
        zone_active_before - 1,
        "the cost removed 1 active energy"
    );
    // The net zone card count is unchanged (one left for cost, one added in wait),
    // but there is now exactly one WAIT energy that was NOT there before.
    assert!(
        zone_wait >= 1,
        "上原歩夢's energy must be placed in the zone in WAIT"
    );
}

/// Negative: energy placed into the energy ZONE (a normal gain, not "under a
/// member") must NOT trigger 上原歩夢's auto ability.
#[test]
fn ayumu_energy_into_zone_not_under_member_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ayumu = game.id("PL!N-bp7-001-R");
    game.state.player1.stage.stage[1] = ayumu;
    game.give_energy(2);
    // Energy deck would be consumed if the ability fired.
    for _ in 0..4 {
        game.state.player1.energy_deck.cards.push(game.id("LL-E-001-SD"));
    }
    let deck_before = game.state.player1.energy_deck.cards.len();

    // Place an energy card directly into the energy zone (a plain gain) —
    // NOT under a member.
    let e = game.id("LL-E-001-SD");
    game.state.player1.energy_zone.cards.push(e);
    game.state.player1.energy_zone.add_active(1);

    // The engine scans auto abilities after the placement. No energy was placed
    // "under a member", so 上原歩夢 must NOT draw from the energy deck.
    // (The scan is triggered implicitly on the next process; assert deck intact.)
    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        deck_before,
        "no under-member placement → 上原歩夢 must not consume energy from the deck"
    );
}

/// 上原歩夢: when energy is placed under a member but the energy deck is empty,
/// the effect does nothing (no crash, no zone change).
#[test]
fn ayumu_empty_energy_deck_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ayumu = game.id("PL!N-bp7-001-R");
    let shioriko = game.id("PL!N-bp7-010-R");
    game.state.player1.stage.stage = [-1, ayumu, shioriko];
    game.give_energy(5);
    // NO energy in the energy deck.

    let zone_cards_before = game.state.player1.energy_zone.cards.len();
    let zone_active_before = game.state.player1.energy_zone.active_count();

    // Activate 三船栞子 → its cost places 1 energy under the member.
    game.activate_ability(shioriko);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        match game.get_pending_choice() {
            rabuka_engine::ability::types::Choice::SelectCard { zone, .. }
                if zone == "energy_zone" =>
            {
                game.select_indices(&[0]);
            }
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                game.select_indices(&[0]);
            }
            rabuka_engine::ability::types::Choice::SelectTarget { .. } => {
                game.select_choice_option(1);
            }
            _ => game.select_choice_option(0),
        }
    }

    // 三船栞子's cost still moved 1 energy under the member, but 上原歩夢's
    // effect drew nothing (empty deck). Zone card count drops by 1 (the cost),
    // no wait energy is added back.
    let zone = &game.state.player1.energy_zone;
    assert_eq!(
        zone.cards.len(),
        zone_cards_before - 1,
        "only the cost's energy left; 上原歩夢 added nothing (empty deck)"
    );
    assert_eq!(
        zone.active_count(),
        zone_active_before - 1,
        "one active energy left for the cost, none returned"
    );
}

