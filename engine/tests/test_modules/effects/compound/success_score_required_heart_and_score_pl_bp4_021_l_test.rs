use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

fn fire_trigger(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trig: &str) {
    fire_trigger_nth(game, cid, trigger, trig, 0);
}

fn fire_trigger_nth(
    game: &mut TestGame,
    cid: i16,
    trigger: AbilityTrigger,
    trig: &str,
    nth: usize,
) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .filter(|a| a.triggers.as_deref() == Some(trig))
            .nth(nth)
            .unwrap_or_else(|| panic!("card {} lacks '{trig}' ability #{nth}", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        trigger,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

#[test]
fn pl_bp4_021_l_success_score_thresholds_reduce_required_heart_then_add_score() {
    let db = load_real_database();
    let run = |success_scores: &[i16]| -> (i32, i32) {
        let mut game = TestGame::new(db.clone());
        let hb = game.id("PL!-bp4-021-L");
        game.state.player1.live_card_zone.cards.push(hb);
        for (k, &score) in success_scores.iter().enumerate() {
            let (no, _) = match score {
                1 => ("PL!-sd1-019-SD", 1),
                5 => ("PL!S-PR-024-PR", 5),
                _ => ("PL!SP-bp1-027-L", 6),
            };
            let id = if k == 0 { game.id(no) } else { game.new_id(no) };
            game.state.player1.success_live_card_zone.add_card(id);
        }
        fire_trigger(&mut game, hb, AbilityTrigger::LiveStart, "ライブ開始時");
        (
            game.state
                .mods
                .get_need_heart_modifier(hb, HeartColor::Heart00),
            game.state.mods.get_score_modifier(hb),
        )
    };
    let (need, score) = run(&[1, 1, 1]);
    assert_eq!(need, 0, "total 3 < 6 → nothing");
    assert_eq!(score, 0);
    let (need, score) = run(&[6]);
    assert_eq!(need, -1, "total 6 ≥ 6 → 必要ハート heart0 −1");
    assert_eq!(score, 0, "total 6 < 9 → NO score bonus yet");
    let (need, score) = run(&[6, 5]);
    assert_eq!(need, -1);
    assert_eq!(score, 1, "total 11 ≥ 9 → スコア +1");
}
