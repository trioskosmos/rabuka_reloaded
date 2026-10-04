/// PL!-sd1-002-SD (絢瀬絵里) ab#0 — Q79
///
/// {{kidou.png|起動}}このメンバーをステージから控え室に置く：
/// 自分の控え室からメンバーカードを1枚手札に加える。
///
/// Q79: After using this ability (self_cost removes Eli from stage),
/// can a new member card be placed in the vacated area?
/// A: Yes — the self_cost empties the area, making it available.
///
/// The two tests at the top cover the part Q79 does not: the cost puts Eli in the
/// waitroom, and the effect then looks for a member card in the waitroom.
///
/// The sacrificed member is ITSELF a legal recovery target.
///
/// 「このメンバーをステージから控え室に置く：自分の控え室からメンバーカードを1枚手札に
/// 加える」 puts the member in the waitroom and then looks for a member card in the
/// waitroom. Nothing in the text excludes the card it just put there, so the member
/// qualifies — and with an EMPTY waitroom it is the only candidate, so there is
/// nothing to choose and the ability resolves on its own.
///
/// The net result is a round trip: stage → waitroom → hand, with the stage slot
/// vacated. That is not a null effect (Q79, above: the slot becomes available) but it
/// does mean the card can be used to move a member out of the way.
///
/// The existing test in this file walks straight past this. Its comment says the
/// waitroom holds "ONLY the target member (avoids selection choice)" and then
/// immediately notes that a prompt appears anyway "because waitroom has 2 member cards
/// (target_member + Eli after self_cost)" — the sacrificed member is candidate zero,
/// and picking the other card means nothing ever asserts it is one.
#[test]
fn q79_the_sacrificed_member_is_itself_a_recovery_target() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let eli = game.id("PL!-sd1-002-SD");
    let bystander = game.id("PL!-sd1-010-SD"); // stays in hand throughout

    game.state.player1.stage.stage = [-1, eli, -1];
    game.add_to_hand(bystander);
    game.give_energy(5);

    assert!(
        game.state.player1.waitroom.cards.is_empty(),
        "fixture: the waitroom starts empty, so the sacrificed member is the only \
         possible candidate"
    );

    game.activate_ability(eli);
    // A prompt here would mean the engine filtered the just-moved card OUT of the
    // candidates. With one candidate there is nothing to choose, so anything asked
    // is a bug rather than a choice.
    game.drain_choices_strict(&[], &[]);

    assert!(
        game.state.player1.hand.cards.contains(&eli),
        "「自分の控え室からメンバーカードを1枚」 — she is a member card in the \
         waitroom now, so she comes back to hand; hand is {:?}",
        game.state.player1.hand.cards
    );
    assert_eq!(
        game.state.player1.stage.stage[1],
        -1,
        "the self_cost still vacates the slot — recovering her does not put her back"
    );
    assert!(
        game.state.player1.waitroom.cards.is_empty(),
        "she left the waitroom; waitroom is {:?}",
        game.state.player1.waitroom.cards
    );
    assert!(
        game.state.player1.hand.cards.contains(&bystander),
        "and the unrelated hand card is untouched"
    );
}

/// With a member already in the waitroom, BOTH are offered — including her.
///
/// The companion to the test above, and the one that matters: it proves she is a
/// *candidate* rather than merely present in the zone. Selecting the index she
/// occupies in the waitroom and getting her back is only possible if the engine put
/// her in the candidate list.
#[test]
fn q79_the_sacrificed_member_is_offered_beside_a_pre_existing_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let eli = game.id("PL!-sd1-002-SD");
    let target_member = game.id("PL!-sd1-001-SD");

    game.state.player1.stage.stage = [-1, eli, -1];
    game.add_to_discard(target_member);
    game.give_energy(5);

    game.activate_ability(eli);

    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "two member cards are in the waitroom, so the recovery asks which one"
    );
    let eli_idx = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .position(|&id| id == eli)
        .expect("the sacrificed member is in the waitroom and must be selectable");
    game.select_indices(&[eli_idx]);
    game.drain_choices_strict(&[], &[]);

    assert!(
        game.state.player1.hand.cards.contains(&eli),
        "picking the sacrificed member returns her to hand; hand is {:?}",
        game.state.player1.hand.cards
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&target_member),
        "the other member stays in the waitroom — only one card was recovered"
    );
}
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn q79_self_cost_vacates_area_for_new_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let eli = game.id("PL!-sd1-002-SD");
    let target_member = game.id("PL!-sd1-001-SD");
    let new_member = game.id("PL!-sd1-003-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Stage: 絢瀬絵里 at center (set directly, no need for play)
    game.state.player1.stage.stage = [-1, eli, -1];
    // Hand: filler (discard for hand count), new_member (for Q79 placement test)
    game.add_to_hand(filler);
    game.add_to_hand(new_member);

    // Waitroom: ONLY the target member (avoids selection choice)
    game.add_to_discard(target_member);

    // Enough energy for new_member's play cost (cost=13)
    game.give_energy(15);

    // Activate Eli's ability (self_cost → recover → vacates center)
    game.activate_ability(eli);

    // The ability creates a selection choice because waitroom has 2 member cards
    // (target_member + Eli after self_cost). Resolve the choice first.
    assert!(
        game.has_pending_choice(),
        "waitroom recover selection prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard (recover from waitroom)"
    );
    // Find target_member's index in waitroom to recover it
    let idx = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .position(|&id| id == target_member)
        .expect("target_member should be in waitroom");
    game.select_indices(&[idx]);

    // After activation, center should be empty (self_cost removed Eli)
    assert_eq!(
        game.state.player1.stage.stage[1], -1,
        "Self_cost vacated center area"
    );

    // Q79: A new member CAN be placed in the vacated center area
    game.play_to_stage(new_member, MemberArea::Center);
    assert_eq!(
        game.state.player1.stage.stage[1], new_member,
        "New member placed in previously vacated center area"
    );
}
