use crate::Arc;
use crate::HashMap;

use crate::card::CardDatabase;
use crate::game_state::GameState;
use crate::player::Player;

use super::observation::{PlayerView, PublicObservation};

pub struct DeterminizationSampler {
    card_database: Arc<CardDatabase>,
    our_pool: Vec<String>,
    /// `Some(list)` = tournament open-lists mode (opponent's actual deck).
    /// `None` = fair mode: opponent hidden cards are sampled from an
    /// anonymous pool of all Member/Live cards in the database, minus what
    /// their public zones reveal.
    opp_pool: Option<Vec<String>>,
    energy_ids: Vec<i16>,
}

impl DeterminizationSampler {
    pub fn new(
        card_database: Arc<CardDatabase>,
        our_card_numbers: &[String],
        opp_card_numbers: &[String],
    ) -> Self {
        Self::with_policy(
            card_database,
            our_card_numbers,
            Some(opp_card_numbers),
        )
    }

    /// Fair sampler: does not use the opponent's deck list.
    pub fn new_fair(card_database: Arc<CardDatabase>, our_card_numbers: &[String]) -> Self {
        Self::with_policy(card_database, our_card_numbers, None)
    }

    pub fn with_policy(
        card_database: Arc<CardDatabase>,
        our_card_numbers: &[String],
        opp_card_numbers: Option<&[String]>,
    ) -> Self {
        let our_pool = our_card_numbers.to_vec();
        let opp_pool = opp_card_numbers.map(|c| c.to_vec());

        let mut energy_ids: Vec<i16> = card_database
            .cards
            .iter()
            .filter(|(_, c)| c.is_energy())
            .map(|(id, _)| *id)
            .collect();
        crate::rng::shuffle_slice(&mut energy_ids);

        Self {
            card_database,
            our_pool,
            opp_pool,
            energy_ids,
        }
    }

    pub fn sample(&self, obs: &PublicObservation) -> GameState {
        let my_player = self.build_player(&obs.me, Some(&self.our_pool));
        let opp_player = self
            .opp_pool
            .as_ref()
            .map(|v| &v[..])
            .map(|pool| self.build_player(&obs.opp, Some(pool)))
            .unwrap_or_else(|| self.build_player(&obs.opp, None));

        let mut gs = GameState::new(my_player, opp_player, Arc::clone(&self.card_database));
        gs.current_phase = obs.current_phase.clone();
        gs.turn_number = obs.turn_number;
        gs.game_result = obs.game_result.clone();
        for &cid in &obs.resolution_zone {
            gs.resolution_zone.add_card(cid);
        }
        gs
    }

    fn build_player(&self, view: &PlayerView, starting_pool: Option<&[String]>) -> Player {
        let mut player = Player::new(String::new(), String::new(), view.is_first_attacker);

        for &cid in &view.hand {
            player.hand.cards.push(cid);
        }

        player.stage.stage = view.stage;
        for i in 0..3 {
            for &cid in &view.under_cards[i] {
                player.stage.under_cards[i].push(cid);
            }
        }

        for &cid in &view.energy_zone {
            player.energy_zone.cards.push(cid);
        }
        player
            .energy_zone
            .set_active_count(view.active_energy_count as u8);

        for &cid in &view.waitroom {
            player.waitroom.cards.push(cid);
        }
        for &cid in &view.success_zone {
            player.success_live_card_zone.cards.push(cid);
        }
        for &cid in &view.live_zone {
            player.live_card_zone.cards.push(cid);
        }

        // Track HOW MANY of each card_no we've seen (not just whether seen).
        // The starting_pool may have multiple copies of the same card_no.
        let mut seen_counts: HashMap<String, usize> = HashMap::default();
        let mut count_seen = |cid: i16| {
            if let Some(card) = self.card_database.get_card(cid) {
                *seen_counts.entry(card.card_no.to_string()).or_insert(0) += 1;
            }
        };
        for &cid in &view.hand {
            count_seen(cid);
        }
        for &cid in &view.stage {
            if cid >= 0 {
                count_seen(cid);
            }
        }
        for uc in &view.under_cards {
            for &cid in uc {
                count_seen(cid);
            }
        }
        for &cid in &view.energy_zone {
            count_seen(cid);
        }
        for &cid in &view.waitroom {
            count_seen(cid);
        }
        for &cid in &view.success_zone {
            count_seen(cid);
        }
        for &cid in &view.live_zone {
            count_seen(cid);
        }

        // Compute remaining pool: starting_pool minus seen_counts.
        // In fair mode (no deck list) assume up to MAX_COPIES of every
        // non-energy card in the database, minus what public zones reveal.
        const MAX_COPIES: usize = 4;
        let mut starting_counts: HashMap<String, usize> = HashMap::default();
        match starting_pool {
            Some(list) => {
                for cn in list {
                    *starting_counts.entry(cn.clone()).or_insert(0) += 1;
                }
            }
            None => {
                for card in self.card_database.cards.values() {
                    if matches!(card.card_type, crate::card::CardType::Energy) {
                        continue;
                    }
                    let entry = starting_counts.entry(card.card_no.to_string()).or_insert(0);
                    if *entry < MAX_COPIES {
                        *entry += 1;
                    }
                }
            }
        }
        let mut remaining: Vec<String> = Vec::new();
        for (cn, total) in starting_counts {
            let seen = seen_counts.get(&cn).copied().unwrap_or(0);
            let avail = total.saturating_sub(seen);
            for _ in 0..avail {
                remaining.push(cn.clone());
            }
        }
        crate::rng::shuffle_slice(&mut remaining);

        // Sample opponent's hand from remaining pool
        let hand_known = view.hand.len();
        let hand_to_sample = view.hand_size.saturating_sub(hand_known);
        let mut sampled = 0usize;
        for i in 0..hand_to_sample {
            if i < remaining.len() {
                if let Some(&cid) = self.card_database.card_no_to_id.get(&remaining[i]) {
                    player.hand.cards.push(cid);
                    sampled += 1;
                }
            }
        }
        remaining.drain(0..sampled);

        // Fill main deck with remaining cards
        for cn in &remaining {
            if let Some(&cid) = self.card_database.card_no_to_id.get(cn) {
                player.main_deck.cards.push(cid);
            }
        }
        // Pad if deck is still too small (extra copies of available cards)
        while player.main_deck.cards.len() < view.main_deck_size {
            let pad_cn = starting_pool
                .and_then(|l| l.first().cloned())
                .or_else(|| self.fallback_card_no());
            match pad_cn {
                Some(cn) => {
                    if let Some(&cid) = self.card_database.card_no_to_id.get(&cn) {
                        player.main_deck.cards.push(cid);
                    } else {
                        break;
                    }
                }
                None => break,
            }
        }

        // Energy deck — fill with any energy card ID
        for _ in 0..view.energy_deck_size {
            let eid = if !self.energy_ids.is_empty() {
                self.energy_ids[0]
            } else {
                0i16
            };
            player.energy_deck.cards.push(eid);
        }

        player
    }

    /// Any non-energy card_no, used to pad undersized decks in fair mode.
    fn fallback_card_no(&self) -> Option<String> {
        self.card_database
            .cards
            .values()
            .find(|c| !matches!(c.card_type, crate::card::CardType::Energy))
            .map(|c| c.card_no.to_string())
    }
}
