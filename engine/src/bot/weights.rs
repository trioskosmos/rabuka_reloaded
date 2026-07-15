use std::collections::HashMap;

/// Learned scalar weight per card. After each game, weights are updated
/// based on which cards the winner had vs the loser had.
/// evaluation = (sum(my_card_weights) - sum(opp_card_weights)) / 3.0
pub struct CardWeights {
    weights: HashMap<i16, f64>,
    learning_rate: f64,
}

impl CardWeights {
    pub fn new(num_cards: usize) -> Self {
        let weights = HashMap::with_capacity(num_cards);
        // Initialize all weights to 0 — the model starts with no knowledge
        // and learns from each game's outcome.
        Self {
            weights,
            learning_rate: 0.01,
        }
    }

    pub fn set_weight(&mut self, card_id: i16, w: f64) {
        self.weights.insert(card_id, w);
    }

    pub fn get_weight(&self, card_id: i16) -> f64 {
        self.weights.get(&card_id).copied().unwrap_or(0.0)
    }

    /// Sum of weights for a set of card IDs (allows duplicates — each copy counts)
    pub fn sum_weights(&self, cards: &[i16]) -> f64 {
        cards.iter().map(|&cid| self.get_weight(cid)).sum()
    }

    /// Update weights after a game: winner's cards get positive reinforcement,
    /// loser's cards get negative reinforcement.
    /// `winner_cards` and `loser_cards` should be all non-energy cards each player
    /// had access to during the game (hand + stage + waitroom).
    /// Replaces the old update — now the current win margin is the target.
    pub fn update(
        &mut self,
        my_stage: &[i16],
        my_hand: &[i16],
        opp_stage: &[i16],
        opp_hand: &[i16],
        margin: f64,
    ) {
        let lr = self.learning_rate;
        // The predicted margin before update
        let pred = self.sum_weights(my_stage) + self.sum_weights(my_hand)
            - self.sum_weights(opp_stage)
            - self.sum_weights(opp_hand);
        let error = margin - pred; // how far off we were

        // Simple gradient descent: w += lr * error * feature
        let update = |w: &mut f64| *w += lr * error;
        for &cid in my_stage {
            self.weights.entry(cid).and_modify(update).or_insert(0.0);
        }
        for &cid in my_hand {
            self.weights.entry(cid).and_modify(update).or_insert(0.0);
        }
        for &cid in opp_stage {
            let update_neg = |w: &mut f64| *w -= lr * error;
            self.weights
                .entry(cid)
                .and_modify(update_neg)
                .or_insert(0.0);
        }
        for &cid in opp_hand {
            let update_neg = |w: &mut f64| *w -= lr * error;
            self.weights
                .entry(cid)
                .and_modify(update_neg)
                .or_insert(0.0);
        }
    }

    pub fn len(&self) -> usize {
        self.weights.len()
    }
}
