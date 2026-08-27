/// BP07 ミア・テイラー PL!N-bp7-011-R＋ ab#0 (自動): deck→discard zone-source.
///
/// このカードがデッキから控え室に置かれたとき、手札を1枚控え室に置いてもよい。
/// そうしたとき、控え室からこのカードを手札に加える。
///
/// The trigger is constrained to source=deck. The REAL mill abilities (登場
/// 黒澤ダイヤ PL!S-sd1-013-SD: 自分のデッキの上から5枚を控え室に置く) move
/// cards deck_top→discard. These tests drive the REAL mill through the real
/// engine (MoveCards::finalize records the true source zone), so they prove the
/// fix "works as written" against the real ability path — not a hand-injected
/// push_movement_event.
///
/// Key question under test: does a real deck_top→discard mill record a source
/// that ミア's condition (source="deck") accepts? If the engine records
/// "deck_top" and compares strictly against "deck", a real mill will NOT
/// trigger ミア, and that would be a real integration gap.
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::ability::types::Choice;

const MIA: &str = "PL!N-bp7-011-R\u{ff0b}"; // ミア・テイラー, ab#0 deck→discard auto
const DIAYA: &str = "PL!S-sd1-013-SD"; // 黒澤ダイヤ, ab#0 登場: mill 5 deck top → discard
const KASUMI: &str = "PL!N-bp1-014-PRproteinbar"; // 中須かすみ, ab#0 登場: draw 1, discard 1 from hand
const FILLER: &str = "PL!-sd1-010-SD"; // ability-free filler

/// Fire a card's real 登場 ability. Drains nothing — the caller inspects the
/// resulting choices (e.g. a pending ミア optional).
fn trigger_debut(game: &mut TestGame, card_id: i16) {
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
    // Do NOT drain here: the pending ミア choice must be observed by the caller.
}

/// Fire 黒澤ダイヤ's real 登場 ability (mill 5 from deck top to discard).
fn trigger_dia_mill(game: &mut TestGame, dia: i16) {
    trigger_debut(game, dia);
}

/// Answer ミア's ab#0 chain. `accept`: true → do the optional discard+recover.
/// Returns true if ミア's ab#0 actually presented its conditional_optional choice.
fn answer_mia_chain(game: &mut TestGame, accept: bool) -> bool {
    let mut saw_mia_optional = false;
    let mut guard = 0;
    while game.has_pending_choice() && guard < 40 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectTarget { target, options, .. } if target == "conditional_optional" => {
                saw_mia_optional = true;
                let pick = if accept { 1 } else { 0 };
                if let Some(ref opts) = options {
                    if pick < opts.len() {
                        game.select_choice_option(pick);
                    } else {
                        game.select_choice_option(0);
                    }
                } else {
                    game.select_choice_option(0);
                }
            }
            Choice::SelectCard { count, .. } => {
                if *count > 0 {
                    game.select_indices(&[0]);
                } else {
                    game.select_indices(&[]);
                }
            }
            _ => break,
        }
    }
    saw_mia_optional
}

#[test]
fn mia_real_dia_mill_triggers_ab0() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let dia = game.id(DIAYA);
    let mia = game.id(MIA);
    game.state.player1.stage.stage[1] = dia;

    // Put ミア on deck top so ダイヤ's mill hits her first.
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(mia);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }

    // Hand cards so the optional discard has something to throw away.
    let h1 = game.id(FILLER);
    let h2 = game.id(FILLER);
    game.state.player1.hand.cards.push(h1);
    game.state.player1.hand.cards.push(h2);

    trigger_dia_mill(&mut game, dia);

    // Report the real movement the engine recorded.
    for mv in game.state.turn_movements.iter() {
        eprintln!(
            "[MV] card={} src={:?} dst={:?}",
            mv.moved_card_id, mv.source_zone, mv.dest_zone
        );
    }

    assert!(
        game.state.turn_movements
            .iter()
            .any(|m| m.moved_card_id == mia),
        "ダイヤ's real mill must move ミア into the discard"
    );

    let saw = answer_mia_chain(&mut game, true);
    assert!(
        saw,
        "ミア ab#0 must fire when a REAL deck_top→discard mill puts her in the discard"
    );
    assert!(
        game.state.player1.hand.cards.contains(&mia),
        "ミア should be recovered to hand after accepting the optional"
    );
}

#[test]
fn mia_real_dia_mill_decline_no_recover() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let dia = game.id(DIAYA);
    let mia = game.id(MIA);
    game.state.player1.stage.stage[1] = dia;

    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(mia);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }
    let h1 = game.id(FILLER);
    let h2 = game.id(FILLER);
    game.state.player1.hand.cards.push(h1);
    game.state.player1.hand.cards.push(h2);

    trigger_dia_mill(&mut game, dia);
    let saw = answer_mia_chain(&mut game, false);

    assert!(saw, "ミア ab#0 should present its optional even when declined");
    assert!(
        !game.state.player1.hand.cards.contains(&mia),
        "declining must leave ミア in the discard, not recovered"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&mia),
        "ミア stays in the discard after decline"
    );
}

#[test]
fn mia_real_hand_discard_does_not_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // 中須かすみ's real 登場: draw 1, then discard 1 card from HAND.
    let kasumi = game.id(KASUMI);
    game.state.player1.stage.stage[1] = kasumi;

    // ミア in hand; the draw will hit it, then the discard discards from hand.
    let mia = game.id(MIA);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(mia); // draw target
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(game.id(FILLER)); // the discard target

    trigger_debut(&mut game, kasumi);

    // The draw moved ミア deck→hand, then the discard moved 1 hand card→discard.
    // The discarded card is a FILLER, so ミア never enters the discard here.
    let saw = answer_mia_chain(&mut game, false);
    assert!(
        !saw,
        "ミア ab#0 must NOT fire when a card is discarded from HAND (not deck)"
    );

    // Now the decisive variant: ミア HERSELF is the card discarded from hand.
    // Restore a fresh game so the hand→discard is clean.
    let mut game = TestGame::new(db.clone());
    let kasumi = game.id(KASUMI);
    let mia = game.id(MIA);
    game.state.player1.stage.stage[1] = kasumi;
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(game.id(FILLER)); // draw target
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(mia); // ミア in hand → will be discarded

    trigger_debut(&mut game, kasumi);

    // ミア was discarded hand→discard. Source is HAND, not deck → no trigger.
    // Drive the かすみ discard to pick ミア specifically.
    let mut mia_discarded = false;
    let mut guard = 0;
    while game.has_pending_choice() && guard < 40 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { count, .. } => {
                // かすみ's discard: pick ミア so she is the hand→discard card.
                let idx = game.state.player1.hand.cards.iter().position(|&c| c == mia);
                if let Some(i) = idx {
                    game.select_indices(&[i]);
                    mia_discarded = true;
                } else if *count > 0 {
                    game.select_indices(&[0]);
                } else {
                    game.select_indices(&[]);
                }
            }
            Choice::SelectTarget { .. } => game.select_choice_option(0),
            _ => break,
        }
    }
    assert!(
        mia_discarded,
        "かすみ's discard should have selected ミア (hand→discard)"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&mia),
        "ミア was discarded from hand into the waitroom"
    );
    assert!(
        game.state.player1.hand.cards.iter().filter(|&&c| c == mia).count() == 0,
        "no ミア copy remains in hand"
    );
    let saw2 = answer_mia_chain(&mut game, false);
    assert!(
        !saw2,
        "discarding ミア from HAND must NOT trigger her deck→discard ab#0"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&mia),
        "ミア stays discarded (hand→discard), not recovered"
    );
}