//! PL!S-bp7-007-P 国木田花丸 ab#1 — ライブ開始時, blade per card *this step* placed.
//!
//! {{live_start.png|ライブ開始時}}
//! 自分の控え室から『Aqours』のメンバーカードを3枚まで好きな順番でデッキの下に置く。
//! ライブ終了時まで、**これによりデッキに置かれたカード1枚につき**、{{icon_blade.png|ブレード}}を得る。
//!
//! In English: at live start, place up to 3 Aqours member cards from the discard
//! into the bottom of the deck in any order. Until the end of the live, gain 1
//! blade for each card placed there BY THIS STEP.
//!
//! # The clause is about this step's cards, and nothing else
//!
//! 「これによりデッキに置かれたカード」 names the batch the first action just put
//! there, and the parse says so: step 2 carries
//! `per_unit_source: "previous_moved_cards"`. The engine never reads that field.
//! `execute_gain_resource` keys the tracked-moved pool off `per_unit_type`, and
//! only recognises `"discard"`, `"waitroom_card"` and `"energy_deck"`. This step's
//! `per_unit_type` is `"枚"` — a bare counting noun with no zone of its own — and
//! `loc_binding` is `None`, so the count falls through to
//! `resolve_per_unit_count("枚", …)`, where `per_unit_zone` maps `"枚"` to
//! `Zone::Hand` whenever the effect carries no `card_type` filter. This one does not.
//!
//! So the blade count is read from the HAND, a zone the printed clause never
//! mentions, while the cards actually placed count for nothing.
//!
//! ## Why the hand has to hold a different number of cards than the batch
//!
//! The two readings coincide at 2 unless the fixture is built to separate them, and
//! that is not hypothetical — an earlier version of this test seeded two hand
//! members, the live card left the hand when it was set, and the hand-based count
//! landed on exactly 2. The test passed against the wrong behaviour.
//!
//! So the hand holds FOUR members while the batch is TWO. A hand-based reading
//! answers 4 and this one answers 2, and the assertion below is which of them the
//! engine produced.
//!
//! Two more cards set `per_unit_source: "previous_moved_cards"` with a
//! `per_unit_type` that does not reach the tracked-moved path — `PL!N-bp7-008-P`
//! (枚) and `PL!S-bp6-021-L` (none at all). This file covers `PL!S-bp7-007-P`.

use crate::helpers::*;

/// Aqours members — the only cards step 1 may place, so the only ones that earn a
/// blade. Two of them, so the correct count is 2.
const AQOURS_A: &str = "PL!S-bp7-006-R"; // 津島善子
const AQOURS_B: &str = "PL!S-bp7-003-R＋"; // 松浦果南
/// Liella! in the 控え室: present but NOT placeable, because step 1 filters on
/// 『Aqours』. It must earn nothing.
const LIELLA_WAITROOM: &str = "PL!SP-sd1-001-SD"; // 澁谷かのん
/// Liella! in the hand — four of them, and the printed blade clause refers to none
/// of them. A hand-based count answers 4; the printed answer is 2.
const HAND_A: &str = "PL!SP-sd1-003-SD"; // 嵐 千砂都
const HAND_B: &str = "PL!HS-bp6-010-R"; // 村野さやか
const HAND_C: &str = "PL!SP-bp5-012-N"; // 澁谷かのん (second print)
const HAND_D: &str = "PL!SP-sd1-004-SD"; // 平安名すみれ

/// 5 passes: P1 sets the live card.
fn advance_to_live_card_set_p1(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
}

/// 2 passes: into ライブ開始時, where ab#1 fires.
fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn kanako_id(game: &mut TestGame) -> i16 {
    let id = game.id("PL!S-bp7-007-P");
    game.assert_card_identity(id, "PL!S-bp7-007-P");
    game.assert_card_in_group(game.id(AQOURS_A), "Aqours", "placeable fixture");
    game.assert_card_in_group(game.id(AQOURS_B), "Aqours", "placeable fixture");
    game.assert_card_in_group(game.id(LIELLA_WAITROOM), "Liella!", "unplaceable fixture");
    id
}

/// The count is the batch THIS STEP placed, and the hand is a decoy of a
/// deliberately different size.
///
/// Four members in hand against two placed: the readings cannot be confused, so
/// this fails if the engine counts the hand.
#[test]
fn blades_count_the_cards_this_step_placed_not_the_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanako = kanako_id(&mut game);
    let aqours_a = game.id(AQOURS_A);
    let aqours_b = game.id(AQOURS_B);
    let liella_in_waitroom = game.id(LIELLA_WAITROOM);

    game.state.player1.stage.stage = [-1, kanako, -1];
    game.give_energy(3);
    // The deck MUST be stocked. The live-card-set draws and the ライブ開始時
    // passes consume from the top, and an exhausted deck triggers a refresh, which
    // empties the 控え室 — the two 『Aqours』 members this test depends on would be
    // gone before step 1 ever ran. This was the first version's bug: it seeded no
    // deck at all, the refresh ate the fixture, and the placement found nothing.
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(live);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);

    // Seed AFTER the live card is set. The passes below run phase bookkeeping
    // that drains the 控え室, so a waitroom seeded before them is empty by the time
    // ライブ開始時 fires and step 1 has nothing to place.
    game.state.player1.waitroom.cards = vec![aqours_a, liella_in_waitroom, aqours_b].into();
    // Four decoys. The live card leaves the hand when it is set, so the hand holds
    // exactly 4 when the blade clause resolves.
    game.state.player1.hand.cards = vec![
        live,
        game.id(HAND_A),
        game.id(HAND_B),
        game.id(HAND_C),
        game.id(HAND_D),
    ]
    .into();

    let blades_before = game.state.mods.get_blade_modifier(kanako);
    advance_to_live_start(&mut game);
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        3,
        "precondition: the 控え室 must still hold the two 『Aqours』 members and the \
         Liella! decoy when ライブ開始時 fires, or step 1 has nothing to place"
    );
    // 「3枚まで好きな順番で」 is a two-prompt flow: choose WHICH eligible cards,
    // then choose their ORDER. Both have to be answered — the blade clause only
    // runs once the placement has actually completed.
    game.drain_choices_strict(&["SelectCard"], &[0, 1]);
    let mut safety = 0;
    while game.pending_choice_type().as_deref() == Some("SelectTarget") && safety < 5 {
        safety += 1;
        game.select_option(0);
    }

    let blades_after = game.state.mods.get_blade_modifier(kanako);

    assert_eq!(
        blades_after - blades_before,
        2,
        "「これによりデッキに置かれたカード1枚につき、ブレードを得る」 — the two 『Aqours』 \
         members this step placed earn a blade each. Not the 4 members in hand, and not \
         the Liella! member left in the 控え室: the clause names only what this step placed."
    );
}

/// With nothing placeable, the clause earns nothing even though the hand is full.
///
/// The other direction of the same distinction, and the one that would catch a fix
/// which simply special-cased 「discard」: 「3枚まで」 is an upper bound, so a 控え室
/// holding no 『Aqours』 member places no cards, and a clause about cards this step
/// placed has an empty set to count. A hand-based reading would still hand out four.
#[test]
fn nothing_placeable_earns_nothing_despite_a_full_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanako = kanako_id(&mut game);
    let liella_in_waitroom = game.id(LIELLA_WAITROOM);

    game.state.player1.stage.stage = [-1, kanako, -1];
    game.give_energy(3);
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(live);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);

    // Only a Liella! member: the step filters on 『Aqours』, so nothing is placeable.
    game.state.player1.waitroom.cards = vec![liella_in_waitroom].into();
    game.state.player1.hand.cards = vec![
        live,
        game.id(HAND_A),
        game.id(HAND_B),
        game.id(HAND_C),
        game.id(HAND_D),
    ]
    .into();

    let blades_before = game.state.mods.get_blade_modifier(kanako);
    advance_to_live_start(&mut game);
    game.drain_choices_strict(&["SelectCard"], &[0]);
    let mut safety = 0;
    while game.pending_choice_type().as_deref() == Some("SelectTarget") && safety < 5 {
        safety += 1;
        game.select_option(0);
    }

    assert_eq!(
        game.state.mods.get_blade_modifier(kanako) - blades_before,
        0,
        "no 『Aqours』 member was placeable, so 「これにより…置かれたカード」 is empty and \
         no blade is earned — the 4 members in hand are not that set."
    );
}