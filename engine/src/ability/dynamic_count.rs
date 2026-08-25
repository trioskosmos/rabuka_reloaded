//! Single source of truth for resolving a `DynamicCount` reference into a count.
//!
//! Both the constant-path (`recalculate_constants`) and the ability-execution
//! path (`AbilityResolver`) call this one method, so dynamic_count semantics live
//! in exactly one place instead of being duplicated per caller.
use crate::core::constants::U8Count;
use crate::card::DynamicCount;
use crate::game_state::GameState;

impl GameState {
    /// Resolve a `DynamicCount` reference against the current game state.
    ///
    /// The transient resolver context (which cards moved / were selected / how
    /// many were drawn in the current step) is passed in because the constant
    /// path has no `AbilityResolver`. Callers that don't have that context pass
    /// empty slices / 0.
    pub(crate) fn resolve_dynamic_count(
        &self,
        dc: &DynamicCount,
        moved_cards: &[i16],
        selected_cards: &[i16],
        last_draw_count: u8,
        owner_card: Option<i16>,
    ) -> u8 {
        let reference_text = dc.reference.as_deref().or(dc.base_reference.as_deref());

        let mut count = match reference_text {
            Some("selected_card_score") => {
                if let Some(&card_id) = selected_cards.first() {
                    if let Some(card) = self.card_database.get_card(card_id) {
                        card.score.unwrap_or(0)
                    } else {
                        0
                    }
                } else {
                    0
                }
            }
            Some("previous_moved_cards") | Some("previous_move") => {
                if !moved_cards.is_empty() {
                    moved_cards.len().u8_count()
                } else if let Some(ref recently_moved) = self.recently_moved_cards {
                    recently_moved.len().u8_count()
                } else {
                    self.mods.last_cost_discard_count
                }
            }
            Some("previous_draw") => {
                if last_draw_count > 0 {
                    last_draw_count
                } else if let Some(ref recently_moved) = self.recently_moved_cards {
                    recently_moved.len().u8_count()
                } else {
                    0
                }
            }
            Some("revealed_cards") | Some("previous_reveal") => self.revealed_count(),
            Some("unit_count") => {
                let player = self.resolve_target_player("self");
                player.stage.stage.iter().filter(|&&c| c != -1).count().u8_count()
            }
            Some("energy_difference") => {
                let threshold = dc
                    .base_reference
                    .as_deref()
                    .and_then(|s| s.parse::<u8>().ok())
                    .unwrap_or(0);
                let player = self.resolve_target_player("self");
                (player.energy_zone.cards.len().u8_count()).saturating_sub(threshold)
            }
            Some("success_pile_count_difference") => {
                // 「相手の成功ライブカード置き場にあるカードの枚数が自分より多い
                // かぎり、その差に等しい数…」 — CARD COUNT difference between the
                // opponent's and the owner's success piles (owner resolved from the
                // activating card so the semantics hold for either player's copy).
                let own_is_p1 = match owner_card {
                    Some(cid) => self.player1.stage.stage.contains(&cid),
                    None => true,
                };
                let (own, other) = if own_is_p1 {
                    (&self.player1, &self.player2)
                } else {
                    (&self.player2, &self.player1)
                };
                (other.success_live_card_zone.cards.len().u8_count())
                    .saturating_sub(own.success_live_card_zone.cards.len().u8_count())
            }
            Some("these_waitroom_placed_count") => {
                if let Some(ref recently_moved) = self.recently_moved_cards {
                    recently_moved.len().u8_count()
                } else {
                    moved_cards.len().u8_count()
                }
            }
            Some("total_live_score") => {
                let player = self.resolve_target_player("self");
                player
                    .live_card_zone
                    .cards
                    .iter()
                    .filter_map(|&id| self.card_database.get_card(id).and_then(|c| c.score))
                    .sum::<u8>()
            }
            Some("stage_member_count") => {
                let player = self.resolve_target_player("self");
                player.stage.stage.iter().filter(|&&c| c != -1).count().u8_count()
            }
            Some("opponent_stage_member_count") => {
                let player = self.resolve_target_player("opponent");
                player.stage.stage.iter().filter(|&&c| c != -1).count().u8_count()
            }
            // 「相手のステージにいるウェイト状態のメンバーの数まで」 — only
            // WAITED opponents count; the old fuzzy arm counted everyone.
            Some("opponent_waited_member_count") => {
                let player = self.resolve_target_player("opponent");
                player
                    .stage
                    .stage
                    .iter()
                    .filter(|&&c| {
                        c != -1 && self.mods.get_orientation_modifier(c) == Some("wait")
                    })
                    .count().u8_count()
            }
            // 「控え室にあるカードの枚数がN枚未満の場合、その差に等しい枚数…」
            // mills exactly the shortfall (base_reference holds N).
            Some("waitroom_count_below_base") => {
                let threshold = dc
                    .base_reference
                    .as_deref()
                    .and_then(|s| s.parse::<u8>().ok())
                    .unwrap_or(0);
                let player = self.resolve_target_player("self");
                threshold.saturating_sub(player.waitroom.cards.len().u8_count())
            }
            Some("energy_cards_under_this_member") => {
                let player = self.resolve_target_player("self");
                let activating_id = self.activating_card;
                let pos = activating_id
                    .and_then(|c| player.stage.stage.iter().position(|&id| id == c))
                    .unwrap_or(1);
                let area = match pos {
                    0 => crate::zones::MemberArea::LeftSide,
                    1 => crate::zones::MemberArea::Center,
                    _ => crate::zones::MemberArea::RightSide,
                };
                player.stage.get_under_cards(area).len().u8_count()
            }
            _ => match dc.count_type.as_str() {
                "revealed_cards" => self.revealed_count(),
                _ => 0,
            },
        };
        if let Some(ref calculation) = dc.calculation {
            if &**calculation == "add" {
                count += dc.calculation_value.unwrap_or(0);
            }
        }
        count
    }

    /// Number of cards in the revealed (yell) pool belonging to "self".
    fn revealed_count(&self) -> u8 {
        let cheer = self.cheer_revealed_cards();
        if !cheer.is_empty() {
            return cheer.len().u8_count();
        }
        let player = self.resolve_target_player("self");
        self.revealed_cards
            .iter()
            .filter(|&&cid| {
                player.hand.cards.contains(&cid)
                    || player.waitroom.cards.contains(&cid)
                    || player.stage.stage.contains(&cid)
                    || player.stage.under_cards.iter().any(|v| v.contains(&cid))
                    || player.energy_zone.cards.contains(&cid)
                    || player.main_deck.cards.contains(&cid)
                    || player.energy_deck.cards.contains(&cid)
                    || player.live_card_zone.cards.contains(&cid)
                    || player.success_live_card_zone.cards.contains(&cid)
                    || self.resolution_zone.cards.contains(&cid)
            })
            .count().u8_count()
    }
}
