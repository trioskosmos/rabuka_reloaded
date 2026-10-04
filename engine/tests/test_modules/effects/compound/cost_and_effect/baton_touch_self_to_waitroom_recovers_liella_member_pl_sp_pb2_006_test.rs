use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const KINAKO: &str = "PL!SP-bp2-006-P";
const LIELLA_MEMBER: &str = "PL!SP-pb1-001-R";
/// A second, distinguishable Liella! member, so a test can say which card went
/// where. Kanon is used only as a card sitting in the waitroom, never on the
/// stage, so her own 登場 ability cannot add a prompt of its own.
const OTHER_LIELLA_MEMBER: &str = "PL!SP-pb2-001-P";
const NON_LIELLA_MEMBER: &str = "PL!-sd1-010-SD";
const FILLER: &str = "PL!-sd1-010-SD";

fn setup(game: &mut TestGame, displaced: Option<i16>) -> (i16, i16) {
    let kinako = game.id(KINAKO);
    let filler = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(25);
    if let Some(card) = displaced {
        game.state.player1.stage.stage[1] = card;
    }
    (kinako, filler)
}

fn drain_choices(game: &mut TestGame) {
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();
}

#[test]
fn kinako_baton_touch_recovers_the_displaced_liella_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let target = game.id(LIELLA_MEMBER);
    let (kinako, _) = setup(&mut game, Some(target));

    assert_eq!(game.db.get_card(kinako).unwrap().card_no.as_ref(), KINAKO);
    assert_eq!(
        game.db.get_card(target).unwrap().card_no.as_ref(),
        LIELLA_MEMBER
    );

    game.state.player1.hand.cards.push(kinako);
    game.play_to_stage_baton(kinako, MemberArea::Center);
    drain_choices(&mut game);

    assert_eq!(game.state.player1.stage.stage[1], kinako);
    assert!(
        game.state.player1.hand.cards.contains(&target),
        "the Liella! member displaced by Kinako's baton touch must return to hand"
    );
    assert!(!game.state.player1.waitroom.cards.contains(&target));
}

#[test]
fn kinako_baton_touch_does_not_recover_a_non_liella_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let target = game.id(NON_LIELLA_MEMBER);
    let (kinako, _) = setup(&mut game, Some(target));

    game.state.player1.hand.cards.push(kinako);
    game.play_to_stage_baton(kinako, MemberArea::Center);
    drain_choices(&mut game);

    assert_eq!(game.state.player1.stage.stage[1], kinako);
    assert!(
        !game.state.player1.hand.cards.contains(&target),
        "Kinako must only recover a displaced Liella! member"
    );
    assert!(game.state.player1.waitroom.cards.contains(&target));
}

#[test]
fn kinako_normal_debut_does_not_recover_a_hand_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let target = game.id(LIELLA_MEMBER);
    let (kinako, _) = setup(&mut game, None);

    game.state.player1.hand.cards.push(kinako);
    game.state.player1.hand.cards.push(target);
    game.play_to_stage_baton(kinako, MemberArea::Center);
    drain_choices(&mut game);

    assert_eq!(game.state.player1.stage.stage[1], kinako);
    assert!(
        game.state.player1.hand.cards.contains(&target),
        "without a baton touch, a hand card is not a recovery target"
    );
}

/// 「**このバトンタッチで**控え室に置かれた」 — this baton touch, not this turn.
///
/// The parse is `source: "recently_moved"`, which is the whole point of the
/// wording: the card recovered is the one THIS baton touch displaced, not any
/// Liella! member that happens to be in the waitroom. Every test above starts from
/// a waitroom holding nothing but the displaced card, so a `recently_moved` that
/// was really "scan the waitroom for a Liella! member" would pass all of them.
///
/// The discriminator is a Liella! member already in the waitroom when the baton
/// touch lands. A waitroom scan sees two candidates and asks; `recently_moved`
/// sees one and must not. `drain_choices_strict` with an empty `allowed` list makes
/// the prompt a failure rather than answering it — a prompt here would mean the
/// player is choosing which card 「this baton touch」 displaced, which is not a
/// choice the card offers.
#[test]
fn a_preexisting_liella_member_is_not_offered_as_a_recovery_candidate() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let target = game.id(LIELLA_MEMBER);
    let bystander = game.id(OTHER_LIELLA_MEMBER);
    let (kinako, _) = setup(&mut game, Some(target));
    game.state.player1.waitroom.cards.push(bystander);

    game.state.player1.hand.cards.push(kinako);
    game.play_to_stage_baton(kinako, MemberArea::Center);
    // Any prompt is the failure: the only legal candidate is the displaced card.
    game.drain_choices_strict(&[], &[]);

    assert_eq!(game.state.player1.stage.stage[1], kinako);
    assert!(
        game.state.player1.hand.cards.contains(&target),
        "the card THIS baton touch displaced is the one that comes back"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&bystander),
        "the bystander was in the waitroom before the baton touch, so it is not \
         「このバトンタッチで控え室に置かれた」 and must stay put"
    );
}

/// The mirror image, and the sharper half: the displaced card is NOT a Liella!
/// member, and a Liella! member is sitting in the waitroom.
///
/// `kinako_baton_touch_does_not_recover_a_non_liella_member` covers this with an
/// empty waitroom. With a bystander present, a waitroom scan would find the
/// bystander and hand the player a card — a real benefit the printed text does not
/// grant, because the text filters on the card this baton touch placed, and only
/// then checks whether that card is a 『Liella!』 member. Nothing recovers.
#[test]
fn a_preexisting_liella_member_is_not_recovered_when_the_displaced_card_is_not_liella() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let target = game.id(NON_LIELLA_MEMBER);
    let bystander = game.id(OTHER_LIELLA_MEMBER);
    let (kinako, _) = setup(&mut game, Some(target));
    game.state.player1.waitroom.cards.push(bystander);

    game.state.player1.hand.cards.push(kinako);
    game.play_to_stage_baton(kinako, MemberArea::Center);
    game.drain_choices_strict(&[], &[]);

    assert_eq!(game.state.player1.stage.stage[1], kinako);
    assert!(
        !game.state.player1.hand.cards.contains(&bystander),
        "the displaced card is not a 『Liella!』 member, so the recovery does not \
         happen — and it cannot be satisfied by a bystander that was already there"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&target)
            && game.state.player1.waitroom.cards.contains(&bystander),
        "both cards stay in the waitroom; the bystander is not dragged out to \
         substitute for the card this baton touch placed"
    );
}
