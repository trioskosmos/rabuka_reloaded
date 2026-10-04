use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// Stage the source and reveal `member_ids`.
///
/// The revealed cards are pushed ONLY into the reveal zone. They used to be pushed
/// into the waitroom as well, from the same ids — one card instance in two zones,
/// which is a board that cannot occur and leaves a reader unable to tell whether the
/// waitroom copy is load-bearing. It is not: the clause's `location` is
/// `revealed_cards`, the ability has no cost, and nothing else in it reads the
/// waitroom. (The same two-zone fixture in
/// `self_to_waitroom_recover_live_q79_q123_test.rs` was worse — there the duplicate
/// sat in the zone the final assertion checked, making that assertion unable to fail.)
fn setup_blade_heartless_member_reveals(game: &mut TestGame, member_ids: &[i16]) {
    let source = game.id("PL!-bp5-004-R＋");
    game.state.player1.stage.stage = [-1, source, -1];
    for &id in member_ids {
        game.state.revealed_cards.push(id);
    }
}

fn has_all_heart_modifier(game: &TestGame) -> bool {
    game.state
        .mods
        .heart_modifiers
        .iter()
        .any(|(_, hm)| hm.get(&HeartColor::All).is_some_and(|e| e.total() > 0))
}

#[test]
fn yell_three_members_without_blade_heart_gains_all_heart_modifier() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let m1 = game.id("PL!S-bp2-002-R");
    let m2 = game.id("PL!S-PR-013-PR");
    let m3 = game.id("PL!S-sd1-006-SD");
    setup_blade_heartless_member_reveals(&mut game, &[m1, m2, m3]);

    assert!(
        !has_all_heart_modifier(&game),
        "HeartColor::All must not be present before"
    );
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    assert!(
        has_all_heart_modifier(&game),
        "Three qualifying members in yell must set HeartColor::All"
    );
}

#[test]
fn yell_three_members_with_blade_hearts_does_not_gain_all_heart_modifier() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let m1 = game.id("PL!-pb1-014-R");
    let m2 = game.id("PL!-pb1-014-R");
    let m3 = game.id("PL!-pb1-014-R");
    setup_blade_heartless_member_reveals(&mut game, &[m1, m2, m3]);

    let before = has_all_heart_modifier(&game);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    let after = has_all_heart_modifier(&game);

    assert_eq!(
        before, after,
        "Three members with blade hearts must not set HeartColor::All"
    );
}

#[test]
fn yell_two_members_without_blade_heart_does_not_gain_all_heart_modifier() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let m1 = game.id("PL!S-bp2-002-R");
    let m2 = game.id("PL!S-PR-013-PR");
    setup_blade_heartless_member_reveals(&mut game, &[m1, m2]);

    let before = has_all_heart_modifier(&game);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    let after = has_all_heart_modifier(&game);

    assert_eq!(
        before, after,
        "Only two qualifying members must not set HeartColor::All"
    );
}

/// The blade-heart fixtures are what the clause assumes, read back from the database.
///
/// Every test above depends on three cards lacking blade hearts and one carrying them,
/// and that was asserted only in comments. It matters more here than in most clauses
/// because the filter is a NEGATION — `card_property: "has_blade_heart"` with
/// `negation: true` — so a fixture that quietly acquired a blade heart would not fail
/// loudly, it would move a card out of the qualifying set and leave
/// `yell_three_members_without_blade_heart_gains_all_heart_modifier` failing for a
/// reason that has nothing to do with the threshold.
///
/// Both accessors are checked, because the two can disagree: `has_blade_heart_strict`
/// is the `blade_heart` key alone, while `has_blade_heart` is also true when
/// `special_heart` is non-empty. A NOT-filter reads very differently depending on which
/// one the engine uses, and for these fixtures there must be no difference to reason
/// about. (See the same split documented in
/// `debut_look_five_take_blade_heart_less_mus_member_pl_pb2_032_n_test.rs`.)
#[test]
fn the_blade_heart_fixtures_are_what_the_clause_assumes() {
    let db = load_real_database();
    let game = TestGame::new(db);

    for card_no in ["PL!S-bp2-002-R", "PL!S-PR-013-PR", "PL!S-sd1-006-SD"] {
        let card = game
            .db
            .get_card(game.id(card_no))
            .unwrap_or_else(|| panic!("{card_no} not in cards.json"));
        assert_eq!(
            card.card_type,
            rabuka_engine::card::CardType::Member,
            "{card_no} must be a メンバー — the clause filters on card_type"
        );
        assert!(
            !card.has_blade_heart_strict(),
            "{card_no} must have no `blade_heart` key — it is a qualifying card"
        );
        assert!(
            !card.has_blade_heart(),
            "{card_no} answers true to has_blade_heart(), so the NOT excludes it and \
             it stops qualifying. special_heart must be empty for these fixtures."
        );
    }

    let with_blade = game
        .db
        .get_card(game.id("PL!-pb1-014-R"))
        .expect("PL!-pb1-014-R not in cards.json");
    assert_eq!(
        with_blade.card_type,
        rabuka_engine::card::CardType::Member,
        "the excluded card must still be a メンバー"
    );
    assert!(
        with_blade.has_blade_heart_strict(),
        "PL!-pb1-014-R must carry a `blade_heart` key — it is the card the negation \
         has to exclude"
    );
}
