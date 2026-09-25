/// PL!N-bp7-018-N 近江彼方 ab#0 (登場).
///
/// 登場：手札を1枚控え室に置いてもよい：自分のデッキの上からカードを5枚見る。
///       その中からブレードハートを持たない『虹ヶ咲』のメンバーカードを1枚
///       公開して手札に加えてもよい。残りを控え室に置く。
///
/// (Debut) Optionally discard 1 card from hand, then look at the top 5 cards of
/// your deck. From among them, you may reveal and add to hand 1 虹ヶ咲 member card
/// that does NOT have a blade heart. Put the rest into the waitroom.
///
/// Covered:
///   - an eligible (虹ヶ咲 member, no blade heart) looked-at card can be taken to hand
///   - a looked-at card WITH a blade heart is NOT selectable (even if 虹ヶ咲 member)
///   - the activating card itself (近江彼方, which HAS a blade heart) is not eligible
///   - non-虹ヶ咲 / non-member cards are never selectable
///   - all non-selected looked-at cards go to the waitroom
///   - the optional hand-discard cost may be skipped
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;const KANATA: &str = "PL!N-bp7-018-N"; // 近江彼方 (cost 4, blade_heart b_heart03)
const ELIGIBLE: &str = "PL!N-bp7-020-N"; // エマ・ヴェルデ (虹ヶ咲 member, NO blade heart)
const WITH_BLADE: &str = "PL!N-bp7-016-N"; // 朝香果林 (虹ヶ咲 member, HAS blade heart)
const FILLER: &str = "PL!-sd1-010-SD"; // 高坂穂乃果 (μ's member, not 虹ヶ咲)
const LIVE: &str = "PL!-sd1-019-SD"; // a live card (not a member)

/// Fire 近江彼方's 登場 (debut) ability and resolve all pending choices,
/// discarding `discard_idx` from hand to pay the optional cost when prompted.
fn trigger_debut(game: &mut TestGame, card_id: i16, pay_cost: bool) {
    let card = game.db.get_card(card_id).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("登場"))
        .expect("card should have a 登場 ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::Debut,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(card_id),
        None,
        None,
    );
    game.state.activating_card = Some(card_id);
    game.state.process_pending_auto_abilities(&pid);

    // Resolve the optional hand-discard cost prompt (SelectCard in hand, allow_skip).
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => {
                if pay_cost {
                    game.select_indices(&[0]);
                } else {
                    game.select_indices(&[]);
                }
            }
            Some("SelectTarget") | Some("SelectPosition") => game.select_indices(&[0]),
            Some("SelectAutoAbility") | Some("SelectHeartColor") | Some("SelectHeartType") => {
                game.select_indices(&[])
            }
            _ => game.select_indices(&[]),
        }
    }
}

fn setup(game: &mut TestGame, top5: Vec<i16>) {
    game.state.player1.main_deck.cards.clear();
    for cid in top5 {
        game.state.player1.main_deck.cards.push(cid);
    }
    // Ensure enough cards remain if a cost card is drawn from hand (never from deck here).
    let f = game.id(FILLER);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(f);
    }
}

/// Place a fresh copy of 近江彼方 on the center of the stage directly
/// (no play cost handled here) and return its id.
fn place_kanata_on_stage(game: &mut TestGame) -> i16 {
    let kanata = game.id(KANATA);
    game.state.player1.stage.stage = [-1, kanata, -1];
    kanata
}

fn in_hand(game: &TestGame, id: i16) -> bool {
    game.state.player1.hand.cards.contains(&id)
}

fn in_waitroom(game: &TestGame, id: i16) -> bool {
    game.state.player1.waitroom.cards.contains(&id)
}

/// The single eligible (no-blade) 虹ヶ咲 member among the top 5 can be taken to hand.
#[test]
fn kanata_takes_eligible_no_blade_member_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let eligible = game.id(ELIGIBLE);
    let with_blade = game.id(WITH_BLADE);
    let filler = game.id(FILLER);
    setup(&mut game, vec![eligible, with_blade, filler, filler, filler]);

    let kanata = place_kanata_on_stage(&mut game);
    let cost_fodder = game.id(FILLER);
    game.state.player1.hand.cards.push(cost_fodder);
    trigger_debut(&mut game, kanata, true);

    // The eligible no-blade 虹ヶ咲 member is now in hand (selected via look-and-select).
    assert!(
        in_hand(&game, eligible),
        "the eligible no-blade 虹ヶ咲 member should be added to hand"
    );
    // The with-blade member was not selected and was discarded.
    assert!(
        in_waitroom(&game, with_blade),
        "the with-blade 虹ヶ咲 member must not be selectable and goes to the waitroom"
    );
    assert!(
        !in_hand(&game, with_blade),
        "the with-blade 虹ヶ咲 member must NOT be added to hand"
    );
}

/// The activating card itself has a blade heart → it is not eligible.
#[test]
fn kanata_own_card_has_blade_not_eligible() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let eligible = game.id(ELIGIBLE);
    let filler = game.id(FILLER);
    // A separate copy of 近江彼方 sits on the deck top (it has a blade heart).
    let deck_kanata = game.id(KANATA);
    setup(
        &mut game,
        vec![deck_kanata, eligible, filler, filler, filler],
    );

    let kanata = place_kanata_on_stage(&mut game);
    let cost_fodder = game.id(FILLER);
    game.state.player1.hand.cards.push(cost_fodder);
    trigger_debut(&mut game, kanata, true);

    // The copy of 近江彼方 on the deck top has a blade heart → cannot be selected.
    assert!(
        in_waitroom(&game, deck_kanata),
        "近江彼方 has a blade heart so it must NOT be selected and goes to the waitroom"
    );
    assert!(
        !in_hand(&game, deck_kanata),
        "近江彼方 (blade heart) must NOT be added to hand"
    );
    // The eligible one is still taken.
    assert!(in_hand(&game, eligible), "the eligible member is taken to hand");
}

/// Declining the optional hand-discard cost means the effect does NOT fire:
/// the looked-at cards stay in the deck (nothing is discarded, nothing added).
#[test]
fn kanata_skip_optional_cost_effect_does_not_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let eligible = game.id(ELIGIBLE);
    let filler = game.id(FILLER);
    setup(&mut game, vec![eligible, filler, filler, filler, filler]);

    let kanata = place_kanata_on_stage(&mut game);
    let cost_fodder = game.id(FILLER);
    game.state.player1.hand.cards.push(cost_fodder);
    trigger_debut(&mut game, kanata, false); // skip the discard cost

    assert!(
        !in_hand(&game, eligible),
        "declining the optional cost means the effect must NOT resolve"
    );
    assert!(
        !in_waitroom(&game, eligible),
        "the eligible card must stay in the deck, not be discarded"
    );
    assert!(
        game.state.player1.main_deck.cards.contains(&eligible),
        "the eligible card should remain on the deck"
    );
}

/// No eligible cards among the top 5 → everything is discarded, nothing added to hand.
#[test]
fn kanata_no_eligible_among_looked_discards_all() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let with_blade = game.id(WITH_BLADE); // 虹ヶ咲 but has blade heart
    let live = game.id(LIVE); // not a member
    let filler = game.id(FILLER); // not 虹ヶ咲
    setup(&mut game, vec![with_blade, live, filler, filler, filler]);

    let kanata = place_kanata_on_stage(&mut game);
    let cost_fodder = game.id(FILLER);
    game.state.player1.hand.cards.push(cost_fodder);
    trigger_debut(&mut game, kanata, true);

    // None of the looked-at cards is eligible → all go to the waitroom.
    assert!(in_waitroom(&game, with_blade), "with-blade member is discarded");
    assert!(in_waitroom(&game, live), "live card is discarded");
    assert!(in_waitroom(&game, filler), "non-虹ヶ咲 member is discarded");
    assert!(!in_hand(&game, with_blade));
    assert!(!in_hand(&game, live));
    assert!(!in_hand(&game, filler));
}
