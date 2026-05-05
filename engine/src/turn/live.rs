use crate::card::{BladeColor, CardDatabase, HeartColor};
use crate::game_state::GameState;
use crate::mod_map::ModMap;
use std::collections::HashMap;

impl super::TurnEngine {
    fn score_delta_since(current: &ModMap<i32>, snapshot: &ModMap<i32>, zone_cards: &[i16]) -> u32 {
        let mut total = 0i32;
        for &cid in zone_cards {
            let cur = current.get(cid).copied().unwrap_or(0);
            let prev = snapshot.get(cid).copied().unwrap_or(0);
            total += (cur - prev).max(0);
        }
        total as u32
    }

    pub fn execute_live_victory_determination(game_state: &mut GameState) {
        game_state.player1.stage_hearts = Some(game_state.player1.calculate_stage_hearts(&game_state.card_database));
        game_state.player2.stage_hearts = Some(game_state.player2.calculate_stage_hearts(&game_state.card_database));

        let player1_id = game_state.player1.id.clone();
        let player2_id = game_state.player2.id.clone();

        // Rule 11.6: LiveSuccess abilities trigger BEFORE winner determination (Q36)
        // Track whether already triggered this live to prevent re-firing on re-entry
        let p1_extra;
        let p2_extra;
        if game_state.live_success_triggered_this_turn {
            p1_extra = 0;
            p2_extra = 0;
        } else {
            game_state.live_success_triggered_this_turn = true;

            let pre_p1 = game_state.score_modifiers.clone();
            Self::trigger_live_success_abilities(game_state, &player1_id);
            game_state.process_pending_auto_abilities(&player1_id);
            if game_state.pending_choice.is_some() { return; }
            p1_extra = Self::score_delta_since(&game_state.score_modifiers, &pre_p1, &game_state.player1.live_card_zone.cards);

            let pre_p2 = game_state.score_modifiers.clone();
            Self::trigger_live_success_abilities(game_state, &player2_id);
            game_state.process_pending_auto_abilities(&player2_id);
            if game_state.pending_choice.is_some() { return; }
            p2_extra = Self::score_delta_since(&game_state.score_modifiers, &pre_p2, &game_state.player2.live_card_zone.cards);
        }

        // Determine winner (scores may have been modified by live_success abilities)
        let player1_score = game_state.player1.live_card_zone.calculate_live_score(&game_state.card_database, game_state.player1_cheer_blade_heart_count, game_state.player1.stage_hearts.as_ref(), Some(&game_state.need_heart_modifiers)) + p1_extra;
        let player2_score = game_state.player2.live_card_zone.calculate_live_score(&game_state.card_database, game_state.player2_cheer_blade_heart_count, game_state.player2.stage_hearts.as_ref(), Some(&game_state.need_heart_modifiers)) + p2_extra;
        let player1_has_cards = !game_state.player1.live_card_zone.cards.is_empty();
        let player2_has_cards = !game_state.player2.live_card_zone.cards.is_empty();

        eprintln!("LIVE_VICTORY: P1_cards={} P1_score={} P2_cards={} P2_score={}", player1_has_cards, player1_score, player2_has_cards, player2_score);

        let (player1_won, player2_won) = if !player1_has_cards && !player2_has_cards { (false, false) }
        else if player1_has_cards && !player2_has_cards { (true, false) }
        else if !player1_has_cards && player2_has_cards { (false, true) }
        else {
            (player1_score > player2_score, player2_score > player1_score)
        };

        eprintln!("LIVE_VICTORY_RESULT: P1_won={} P2_won={}", player1_won, player2_won);

        if player2_won { game_state.set_opponent_live_success(true); }

        let card_db = game_state.card_database.clone();
        Self::move_restricted_cards_to_discard(&mut game_state.player1, &card_db);
        Self::move_restricted_cards_to_discard(&mut game_state.player2, &card_db);
        Self::move_live_to_success_and_handle_wins(game_state, player1_won, player2_won);
    }

    fn move_restricted_cards_to_discard(player: &mut crate::player::Player, card_db: &CardDatabase) {
        let mut cards_to_remove = Vec::new();
        for (index, card_id) in player.live_card_zone.cards.iter().enumerate() {
            if let Some(card) = card_db.get_card(*card_id) {
                let has_restriction = card.abilities.iter().any(|ability| {
                    if let Some(ref effect) = ability.effect {
                        let restricted_dest = effect.restricted_destination.as_deref().or_else(|| effect.destination.as_deref());
                        effect.action == "restriction" && effect.restriction_type.as_deref() == Some("cannot_place")
                            && (restricted_dest == Some("success_live_zone") || restricted_dest == Some("live_card_zone"))
                    } else { false }
                });
                if has_restriction { cards_to_remove.push(index); }
            }
        }
        for &idx in cards_to_remove.iter().rev() {
            if idx < player.live_card_zone.cards.len() {
                let card_id = player.live_card_zone.cards.remove(idx);
                player.waitroom.add_card(card_id);
            }
        }
    }

    fn move_live_to_success_and_handle_wins(game_state: &mut GameState, player1_won: bool, player2_won: bool) {
        let p1_id = game_state.player1.id.clone();
        let p2_id = game_state.player2.id.clone();

        // Rule 8.4.7: Winner moves 1 card to success zone. Loser discards all.
        // Rule 8.4.7.1: If both win and a player has 2+ cards, that player moves none.
        let p1_cards = game_state.player1.live_card_zone.cards.len();
        let p2_cards = game_state.player2.live_card_zone.cards.len();
        let p1_must_skip = player1_won && player2_won && p1_cards >= 2;
        let p2_must_skip = player1_won && player2_won && p2_cards >= 2;

        if player1_won && !p1_must_skip && p1_cards > 0 {
            // Move 1 card to success zone
            let card_id = game_state.player1.live_card_zone.cards.remove(p1_cards - 1);
            if game_state.can_place_card_in_zone(card_id, "success_live_zone", &p1_id) {
                game_state.player1.success_live_card_zone.cards.push(card_id);
            } else {
                game_state.player1.waitroom.cards.push(card_id);
            }
            // Discard remaining
            while !game_state.player1.live_card_zone.cards.is_empty() {
                game_state.player1.waitroom.add_card(game_state.player1.live_card_zone.cards.remove(0));
            }
        } else {
            // Loser: discard all
            while !game_state.player1.live_card_zone.cards.is_empty() {
                game_state.player1.waitroom.add_card(game_state.player1.live_card_zone.cards.remove(0));
            }
        }

        if player2_won && !p2_must_skip && p2_cards > 0 {
            let card_id = game_state.player2.live_card_zone.cards.remove(p2_cards - 1);
            if game_state.can_place_card_in_zone(card_id, "success_live_zone", &p2_id) {
                game_state.player2.success_live_card_zone.cards.push(card_id);
            } else {
                game_state.player2.waitroom.cards.push(card_id);
            }
            while !game_state.player2.live_card_zone.cards.is_empty() {
                game_state.player2.waitroom.add_card(game_state.player2.live_card_zone.cards.remove(0));
            }
        } else {
            while !game_state.player2.live_card_zone.cards.is_empty() {
                game_state.player2.waitroom.add_card(game_state.player2.live_card_zone.cards.remove(0));
            }
        }
    }

    pub fn player_perform_live(
        player: &mut crate::player::Player,
        resolution_zone: &mut crate::zones::ResolutionZone,
        player_id: &str, card_db: &CardDatabase,
        blade_modifiers: &ModMap<i32>,
        heart_override: &HashMap<i16, (HeartColor, u32)>,
        heart_modifiers: &HashMap<i16, HashMap<HeartColor, i32>>,
        blade_type_modifiers: &ModMap<BladeColor>,
    ) -> u32 {
        // Rule 8.3.10: Sum blades from active members
        let total_blade = player.stage.total_blades(card_db, blade_modifiers);
        eprintln!("[LIVE] {}: total_blade={}", player.name, total_blade);

        // Rule 8.3.11: Draw cards from deck to resolution zone equal to blade count (yell)
        for _ in 0..total_blade {
            if let Some(card_id) = player.main_deck.draw() {
                resolution_zone.cards.push(card_id);
            }
        }

        // Rule 8.3.14: Compute owned hearts = stage member hearts + blade hearts
        let mut owned_hearts = player.stage.get_available_hearts(card_db, heart_override, heart_modifiers);

        // Resolve blade type override (e.g., all become purple)
        let blade_to_heart = |bc: BladeColor| -> HeartColor {
            match bc {
                BladeColor::Peach => HeartColor::Heart01,
                BladeColor::Red => HeartColor::Heart02,
                BladeColor::Yellow => HeartColor::Heart03,
                BladeColor::Green => HeartColor::Heart04,
                BladeColor::Blue => HeartColor::Heart05,
                BladeColor::Purple => HeartColor::Heart06,
                BladeColor::All => HeartColor::Heart00,
            }
        };
        let override_color = (0..3).filter_map(|i| {
            let cid = player.stage.stage[i];
            if cid == -1 { None } else { blade_type_modifiers.get(cid).copied().map(blade_to_heart) }
        }).next();

        // Process resolution zone card blade hearts: contribute colors to owned hearts
        // and count all colored icons for score bonus (rule 8.4.2.1)
        let mut cheer_icon_count = 0u32;
        for card_id in &resolution_zone.cards {
            if let Some(card) = card_db.get_card(*card_id) {
                if let Some(ref bh) = card.blade_heart {
                    for (color, count) in &bh.hearts {
                        let effective_color = override_color.unwrap_or(*color);
                        // BAll (ALL blade) counts as Heart00 (wildcard)
                        if effective_color == HeartColor::BAll {
                            *owned_hearts.hearts.entry(HeartColor::Heart00).or_insert(0) += count;
                        } else if effective_color == HeartColor::Draw {
                            // Draw icon: draw 1 card from deck to hand
                            for _ in 0..*count {
                                if let Some(new_card) = player.main_deck.draw() {
                                    player.hand.add_card(new_card);
                                }
                            }
                        } else if effective_color == HeartColor::Score {
                            // Score icon: adds to score bonus
                            cheer_icon_count += count;
                        } else {
                            // Colored heart: contributes to owned hearts + score bonus
                            *owned_hearts.hearts.entry(effective_color).or_insert(0) += count;
                            cheer_icon_count += count;
                        }
                    }
                }
            }
        }

        // Also check the live card's special_heart for draw/score icons
        for &lc_id in &player.live_card_zone.cards {
            if let Some(card) = card_db.get_card(lc_id) {
                if let Some(ref sh) = card.special_heart {
                    for (color, count) in &sh.hearts {
                        if *color == HeartColor::Draw {
                            for _ in 0..*count {
                                if let Some(new_card) = player.main_deck.draw() {
                                    player.hand.add_card(new_card);
                                }
                            }
                        } else if *color == HeartColor::Score {
                            cheer_icon_count += count;
                        }
                    }
                }
            }
        }

        // Rule 8.3.15-16: Check each live card's requirement. If ANY fails, discard ALL.
        let live_card_ids: Vec<i16> = player.live_card_zone.cards.iter().copied().collect();
        let any_requirement_failed = live_card_ids.iter().any(|&lc_id| {
            card_db.get_card(lc_id).map_or(false, |card| {
                card.need_heart.as_ref().map_or(false, |nh| {
                    !nh.hearts.is_empty() && !card.satisfies_heart_requirement(&owned_hearts)
                })
            })
        });
        if any_requirement_failed {
            eprintln!("[LIVE] Heart requirement not met — discarding all live cards");
            player.live_card_zone.cards.clear();
        }

        // Rule 8.4.2.1: Score bonus from each cheer blade heart icon
        eprintln!("[LIVE] {}: blades={} cheer_icons={} owned_hearts={:?} live_cards={}",
            player.name, total_blade, cheer_icon_count, owned_hearts.hearts, player.live_card_zone.len());

        // Move resolution zone cards to waitroom after processing
        for card_id in resolution_zone.cards.drain(..) {
            player.waitroom.add_card(card_id);
        }

        total_blade + cheer_icon_count
    }
}
