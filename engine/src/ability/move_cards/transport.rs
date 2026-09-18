use super::{AbilityResolver, GameState, Player, util};
#[cfg(feature = "no_std")]
use alloc::{string::{String, ToString}, vec::Vec};
use smallvec::SmallVec;

pub(crate) fn drain_under_cards_to_energy_zone(
    gs: &mut GameState,
    target: &str,
    stage_idx: usize,
) -> Vec<i16> {
    let under = core::mem::take(
        &mut gs.resolve_target_player_mut(target).stage.under_cards[stage_idx],
    );
    let mut moved = Vec::new();
    for cid in under {
        let is_energy = gs.card_database.get_card(cid).is_some_and(|c| c.is_energy());
        if is_energy {
            gs.resolve_target_player_mut(target).energy_zone.cards.push(cid);
            gs.mods.add_orientation_modifier(cid, "wait");
        } else {
            gs.resolve_target_player_mut(target).waitroom.add_card(cid);
        }
        moved.push(cid);
        let pid = gs.ability_queue.current_entry()
            .map(|e| e.player_id.clone())
            .unwrap_or_else(|| "p1".to_string());
        gs.push_movement_event_typed(
            cid,
            crate::core::types::ZoneId::UnderMember,
            crate::core::types::ZoneId::EnergyZone,
            gs.activating_card,
            &pid,
            true,
        );
    }
    gs.recalculate_constants();
    moved
}

pub(super) fn remove_card_from_any_zone(
    player: &mut Player,
    last_vacated_stage_area: &mut Option<u8>,
    card_id: i16,
) {
    if let Some(pos) = player.hand.cards.iter().position(|&id| id == card_id) {
        player.hand.cards.remove(pos);
    } else if let Some(pos) = player.waitroom.cards.iter().position(|&id| id == card_id) {
        player.waitroom.cards.remove(pos);
    } else if let Some(pos) = player.stage.stage.iter().position(|&id| id == card_id) {
        player.stage.stage[pos] = -1;
        player.deployed_this_turn.retain(|id| *id != card_id);
        *last_vacated_stage_area = Some(pos as u8);
    } else if let Some(pos) = player.energy_zone.cards.iter().position(|&id| id == card_id) {
        player.energy_zone.cards.remove(pos);
    }
}

pub(super) fn matching_waitroom_indices(player: &Player, cards: &[i16]) -> Vec<usize> {
    let mut indices = Vec::new();
    for &cid in cards {
        for (i, &wc) in player.waitroom.cards.iter().enumerate() {
            if wc == cid && !indices.contains(&i) {
                indices.push(i);
            }
        }
    }
    indices
}

impl AbilityResolver {
    pub fn move_from_revealed(
        &mut self,
        gs: &mut GameState,
        indices: &[usize],
        validate_card: &mut impl FnMut(i16) -> bool,
        dst: &str,
    ) -> Vec<i16> {
        let cards: Vec<i16> = {
            let revealed = &mut gs.revealed_cards;
            let mut result = Vec::new();
            let mut removed: Vec<usize> = indices.to_vec();
            removed.sort_by(|a, b| b.cmp(a));
            for &i in &removed {
                if i < revealed.len() {
                    let cid = revealed.remove(i);
                    if validate_card(cid) {
                        result.push(cid);
                    }
                }
            }
            result
        };
        for &cid in &cards {
            if let Some(pos) = gs.player1.waitroom.cards.iter().position(|&c| c == cid) {
                gs.player1.waitroom.cards.remove(pos);
            } else if let Some(pos) = gs.player2.waitroom.cards.iter().position(|&c| c == cid) {
                gs.player2.waitroom.cards.remove(pos);
            } else if let Some(pos) = gs.player1.main_deck.cards.iter().position(|&c| c == cid) {
                gs.player1.main_deck.cards.remove(pos);
            } else if let Some(pos) = gs.player2.main_deck.cards.iter().position(|&c| c == cid) {
                gs.player2.main_deck.cards.remove(pos);
            }
        }
        // Remove from physical zone (waitroom for yell cards,
        // hand for cost reveals, deck for deck-peek reveals).
        for &cid in &cards {
            if let Some(pos) = gs.player1.waitroom.cards.iter().position(|&c| c == cid) {
                gs.player1.waitroom.cards.remove(pos);
            } else if let Some(pos) = gs.player2.waitroom.cards.iter().position(|&c| c == cid) {
                gs.player2.waitroom.cards.remove(pos);
            } else if let Some(pos) = gs.player1.main_deck.cards.iter().position(|&c| c == cid) {
                gs.player1.main_deck.cards.remove(pos);
            } else if let Some(pos) = gs.player2.main_deck.cards.iter().position(|&c| c == cid) {
                gs.player2.main_deck.cards.remove(pos);
            }
        }
        // Don't set self.selected_cards here — cards moved from
        // revealed_cards are effect-internal (not user-targeted
        // selections), and would bleed into downstream gain_resource
        // via the "pure sequential select→gain_resource" path.
        let player = gs.active_player_mut();
        for &cid in &cards {
            util::place_card_in_zone(player, cid, dst, None, false, 1);
        }
        cards
    }

    /// Move cards from under_member to a destination zone, using flat 3-position indexing.
    pub fn move_from_under_member(
        &mut self,
        gs: &mut GameState,
        indices: &[usize],
        validate_card: &mut impl FnMut(i16) -> bool,
        dst: &str,
        target: &str,
    ) -> Result<Vec<i16>, String> {
        let player = gs.resolve_target_player_mut(target);
        let mut cards_to_move: Vec<(usize, i16)> = Vec::new();
        for &idx in indices.iter() {
            let mut global_idx = 0;
            let mut found = false;
            for si in 0..3 {
                if idx < global_idx + player.stage.under_cards[si].len() {
                    let card_id = player.stage.under_cards[si][idx - global_idx];
                    if !validate_card(card_id) {
                        return Err(format!(
                            "Card {:?} does not match required type filter for under_member selection",
                            card_id
                        ));
                    }
                    cards_to_move.push((si, card_id));
                    found = true;
                    break;
                }
                global_idx += player.stage.under_cards[si].len();
            }
            if !found {
                return Err(format!("Card at index {} not found in under_member", idx));
            }
        }
        for (si, card_id) in &cards_to_move {
            if let Some(pos) = player.stage.under_cards[*si]
                .iter()
                .position(|&c| c == *card_id)
            {
                player.stage.under_cards[*si].remove(pos);
                util::place_card_in_zone(player, *card_id, dst, None, false, 1);
            }
        }
        // Record which stage member(s) hosted the moved cards so a following
        // 「そうした場合、そのメンバーは…」 gain step can target THEM specifically
        // (see resolve_gain_resource_targets / last_under_move_host_ids).
        let mut host_ids: SmallVec<[i16; 4]> = SmallVec::new();
        for (si, _) in &cards_to_move {
            if let Some(&host) = player.stage.stage.get(*si) {
                if host != -1 && !host_ids.contains(&host) {
                    host_ids.push(host);
                }
            }
        }
        // `player`'s borrow of `gs` ends here (last use above); the mods
        // update below re-borrows gs directly.
        for host in host_ids {
            if !gs.mods.last_under_move_host_ids.contains(&host) {
                gs.mods.last_under_move_host_ids.push(host);
            }
        }
        gs.recalculate_constants();
        // Don't save energy card IDs in selected_cards — they would leak
        // into downstream sequential actions (e.g. gain_resource heart targets).
        // moved_cards already tracks these via the caller.
        Ok(cards_to_move.iter().map(|&(_, cid)| cid).collect())
    }
}
