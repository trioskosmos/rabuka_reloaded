use crate::helpers::*;
use rabuka_engine::card::{BaseHeart, HeartColor, HeartMap};
use rabuka_engine::game_state::Phase;
use rabuka_engine::turn::TurnEngine;

fn trigger_and_drain(game: &mut TestGame) {
    game.state.current_phase = Phase::LiveVictoryDetermination;
    TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");
    // Drain any remaining choices (e.g. from the live card's own LiveSuccess)
    loop {
        if !game.state.has_pending_choice() {
            break;
        }
        let choice = game.state.get_pending_choice().unwrap().clone();
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { allow_skip, .. } => {
                if allow_skip {
                    TurnEngine::resume_with_choice(&mut game.state, None, Some(vec![]))
                        .expect("skip select card");
                } else {
                    break;
                }
            }
            rabuka_engine::ability::types::Choice::SelectTarget { options, .. } => {
                if let Some(opts) = options {
                    if !opts.is_empty() {
                        TurnEngine::resume_with_choice(&mut game.state, Some(0), None)
                            .expect("select option 0");
                    }
                }
            }
            _ => break,
        }
        game.state.process_pending_auto_abilities("p1");
    }
}

/// 若菜四季 (PL!SP-pb2-008-R) — LiveSuccess: per-unit score +1 per 2 Liella! member
/// cards among yell-revealed. Capped at +2 total.
///
/// Per_unit counting uses UnderMember zone (per_unit_type="枚" + member_card).
/// CardFilter::from_effect() hardcodes negation=false, so matching cards
/// are Liella! members WITH blade_heart.
///
/// Setup: Shiki (trigger) + Kanon (target, has blade_heart) on stage.
/// Live card in live_card_zone (required for should_trigger_live_success).
/// Under-member: 6 Liella! member cards with blade_heart.
/// Expected: 6/2*1 = raw 3, capped at max_repeats=2 → target gets +2.
#[test]
fn shiki_per_unit_score_capped_at_2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let shiki = game.id("PL!SP-pb2-008-R");
    let kanon = game.id("PL!SP-sd1-001-SD"); // Liella!, has blade_heart
    let under_card = game.id("PL!SP-sd1-003-SD"); // Liella! member, has blade_heart
    let live = game.id("PL!-sd1-020-SD"); // filler live card
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [kanon, shiki, -1];
    game.state.player1.live_card_zone.cards.push(live);

    for _ in 0..6 {
        game.state.player1.stage.under_cards[1].push(under_card);
    }

    let mut h = BaseHeart {
        hearts: HeartMap::new(),
    };
    h.hearts.insert(HeartColor::Heart00, 20);
    game.state.player1.stage_hearts = Some(h);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    trigger_and_drain(&mut game);

    let score_mod = game.state.mods.get_score_modifier(kanon);
    assert_eq!(
        score_mod, 2,
        "Score should be capped at 2 (raw 6/2*1=3, max_repeats=2), got {}",
        score_mod
    );
}

#[test]
fn shiki_per_unit_score_no_cap_needed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let shiki = game.id("PL!SP-pb2-008-R");
    let kanon = game.id("PL!SP-sd1-001-SD");
    let under_card = game.id("PL!SP-sd1-003-SD");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [kanon, shiki, -1];
    game.state.player1.live_card_zone.cards.push(live);

    for _ in 0..2 {
        game.state.player1.stage.under_cards[1].push(under_card);
    }

    let mut h = BaseHeart {
        hearts: HeartMap::new(),
    };
    h.hearts.insert(HeartColor::Heart00, 20);
    game.state.player1.stage_hearts = Some(h);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    trigger_and_drain(&mut game);

    let score_mod = game.state.mods.get_score_modifier(kanon);
    assert_eq!(
        score_mod, 1,
        "2 matching /2 * 1 = 1 (under cap of 2), got {}",
        score_mod
    );
}

#[test]
fn shiki_per_unit_score_zero_matching() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let shiki = game.id("PL!SP-pb2-008-R");
    let kanon = game.id("PL!SP-sd1-001-SD");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [kanon, shiki, -1];
    game.state.player1.live_card_zone.cards.push(live);

    let mut h = BaseHeart {
        hearts: HeartMap::new(),
    };
    h.hearts.insert(HeartColor::Heart00, 20);
    game.state.player1.stage_hearts = Some(h);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    trigger_and_drain(&mut game);

    let score_mod = game.state.mods.get_score_modifier(kanon);
    assert_eq!(
        score_mod, 0,
        "0 matching cards → 0 score, got {}",
        score_mod
    );
}
