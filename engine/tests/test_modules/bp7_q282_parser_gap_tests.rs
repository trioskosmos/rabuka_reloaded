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

    let deck_before = game.state.player1.main_deck.cards.len();
    let hand_before = game.state.player1.hand.cards.len();

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

    // The effect is conditional_on_optional: accept the optional move, then
    // select each group's card, then resolve the draw.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 40 {
        guard += 1;
        let choice = game.get_pending_choice().clone();
        match choice {
            // Accept the optional "place on deck bottom" (SelectTarget pay = index 1).
            rabuka_engine::ability::types::Choice::SelectTarget { .. } => {
                game.select_choice_option(1);
            }
            // Select the group cards from the waitroom.
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                game.select_indices(&[0]);
            }
            _ => game.select_indices(&[0]),
        }
    }

    assert!(
        game.state.player1.main_deck.cards.len() > deck_before,
        "the selected group cards should be placed onto the deck"
    );
    assert!(
        game.state.player1.hand.cards.len() > hand_before,
        "placing them draws 1 card"
    );
}

// ====================================================================
// 2. 上原歩夢 (PL!N-bp7-001-R): 自動 — energy placed under a member
// ====================================================================

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
    // places it (in WAIT). The under-member trigger mechanism is proven by the
    // fact that the energy left the deck after an energy was placed under a member.
    assert!(
        game.state.player1.energy_deck.cards.len() < energy_deck_before,
        "上原歩夢 should take 1 energy from the energy deck"
    );
}

