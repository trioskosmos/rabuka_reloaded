use std::fs::File;
use std::io::Read;

use super::encoding::*;

const CARD_EMBED_TABLE_SIZE: usize = 2400;
const HIDDEN: usize = 256;

pub struct PolicyNet {
    // Card embedding table [num_cards, 128]
    pub card_embed: Vec<f32>,
    // Zone embedding table [15, 16]
    pub zone_embed: Vec<f32>,
    // Action type embedding table [16, 16]
    pub action_type_embed: Vec<f32>,
    // State trunk: W1_state [HIDDEN, state_dim], b1 [HIDDEN]
    pub w1_state: Vec<f32>,
    pub b1: Vec<f32>,
    // Action projection: W1_action [HIDDEN, ACTION_ENC_DIM]
    pub w1_action: Vec<f32>,
    // Policy head: W_policy [1, HIDDEN], b_policy [1]
    pub w_policy: Vec<f32>,
    pub b_policy: f32,
    // Value head: W_value [1, HIDDEN], b_value [1]
    pub w_value: Vec<f32>,
    pub b_value: f32,
}

impl PolicyNet {
    pub fn new() -> Self {
        let state_dim = EncodedState::state_dim();
        Self {
            card_embed: vec![0.0f32; CARD_EMBED_TABLE_SIZE * CARD_EMBED_DIM],
            zone_embed: vec![0.0f32; NUM_ZONES * ZONE_EMBED_DIM],
            action_type_embed: vec![0.0f32; 16 * ACTION_TYPE_EMBED_DIM],
            w1_state: vec![0.0f32; HIDDEN * state_dim],
            b1: vec![0.0f32; HIDDEN],
            w1_action: vec![0.0f32; HIDDEN * ACTION_ENC_DIM],
            w_policy: vec![0.0f32; HIDDEN],
            b_policy: 0.0,
            w_value: vec![0.0f32; HIDDEN],
            b_value: 0.0,
        }
    }

    pub fn load_weights(&mut self, path: &str) -> std::io::Result<()> {
        let mut f = File::open(path)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        let mut pos = 0usize;

        let read_f32 = |buf: &[u8], p: &mut usize| -> f32 {
            let bytes: [u8; 4] = buf[*p..*p + 4].try_into().unwrap();
            *p += 4;
            f32::from_le_bytes(bytes)
        };

        // Version header: skipped, but must be consumed to advance the cursor.
        read_f32(&buf, &mut pos);

        // card_embed [2400, 128]
        for i in 0..CARD_EMBED_TABLE_SIZE * CARD_EMBED_DIM {
            self.card_embed[i] = read_f32(&buf, &mut pos);
        }

        // zone_embed [15, 16]
        for i in 0..NUM_ZONES * ZONE_EMBED_DIM {
            self.zone_embed[i] = read_f32(&buf, &mut pos);
        }

        // action_type_embed [16, 16]
        for i in 0..16 * ACTION_TYPE_EMBED_DIM {
            self.action_type_embed[i] = read_f32(&buf, &mut pos);
        }

        // w1_state [HIDDEN, state_dim]
        let state_dim = EncodedState::state_dim();
        for i in 0..HIDDEN * state_dim {
            self.w1_state[i] = read_f32(&buf, &mut pos);
        }

        // b1 [HIDDEN]
        for i in 0..HIDDEN {
            self.b1[i] = read_f32(&buf, &mut pos);
        }

        // w1_action [HIDDEN, ACTION_ENC_DIM]
        for i in 0..HIDDEN * ACTION_ENC_DIM {
            self.w1_action[i] = read_f32(&buf, &mut pos);
        }

        // w_policy [HIDDEN]
        for i in 0..HIDDEN {
            self.w_policy[i] = read_f32(&buf, &mut pos);
        }
        self.b_policy = read_f32(&buf, &mut pos);

        // w_value [HIDDEN]
        for i in 0..HIDDEN {
            self.w_value[i] = read_f32(&buf, &mut pos);
        }
        self.b_value = read_f32(&buf, &mut pos);

        Ok(())
    }

    /// Encode state from observation into flat vector using learned embeddings.
    pub fn encode_state(&self, obs: &super::observation::PublicObservation) -> EncodedState {
        let embed_card = |cid: i16| -> Vec<f32> {
            let idx = cid.max(0) as usize;
            let base = idx * CARD_EMBED_DIM;
            if base + CARD_EMBED_DIM <= self.card_embed.len() {
                self.card_embed[base..base + CARD_EMBED_DIM].to_vec()
            } else {
                vec![0.0f32; CARD_EMBED_DIM]
            }
        };

        let sum_embeds = |cards: &[i16]| -> Vec<f32> {
            let mut s = vec![0.0f32; CARD_EMBED_DIM];
            for &cid in cards {
                let e = embed_card(cid);
                for i in 0..CARD_EMBED_DIM {
                    s[i] += e[i];
                }
            }
            s
        };

        let stage_enc = |cards: &[i16; 3], under: &[Vec<i16>; 3]| -> [Vec<f32>; 3] {
            let mut out = [vec![], vec![], vec![]];
            for pos in 0..3 {
                let cid = cards[pos];
                if cid < 0 {
                    out[pos] = vec![0.0f32; CARD_EMBED_DIM + POSITION_FEATURES];
                    continue;
                }
                let mut v = embed_card(cid);
                // orientation: 1 for active (positive card id)
                v.push(1.0);
                // underlay count
                let ucnt = under[pos].len() as f32;
                v.push(ucnt / 5.0);
                // position index
                v.push(pos as f32 / 2.0);
                // flag
                v.push(1.0);
                out[pos] = v;
            }
            out
        };

        let my_stage_enc = stage_enc(&obs.me.stage, &obs.me.under_cards);
        let opp_stage_enc = stage_enc(&obs.opp.stage, &obs.opp.under_cards);

        // Globals: [phase_onehot(12), turn, hand, opp_hand, my_ae, opp_ae,
        //          my_deck, opp_deck, my_succ, opp_succ, is_first,
        //          my_energy_len, opp_energy_len, my_live, opp_live,
        //          my_blade, opp_blade]
        let mut globals = vec![0.0f32; GLOBAL_FEATURES];
        let pi = phase_as_u8(&obs.current_phase) as usize;
        if pi < 12 {
            globals[pi] = 1.0;
        }
        globals[12] = obs.turn_number as f32 / 30.0;
        globals[13] = obs.me.hand.len() as f32 / 10.0;
        globals[14] = obs.opp.hand_size as f32 / 10.0;
        globals[15] = obs.me.active_energy_count as f32 / 15.0;
        globals[16] = obs.opp.active_energy_count as f32 / 15.0;
        globals[17] = obs.me.main_deck_size as f32 / 60.0;
        globals[18] = obs.opp.main_deck_size as f32 / 60.0;
        globals[19] = obs.me.success_zone.len() as f32 / 3.0;
        globals[20] = obs.opp.success_zone.len() as f32 / 3.0;
        globals[21] = if obs.me.is_first_attacker { 1.0 } else { 0.0 };
        globals[22] = obs.me.energy_zone.len() as f32 / 20.0;
        globals[23] = obs.opp.energy_zone.len() as f32 / 20.0;
        globals[24] = obs.me.live_zone.len() as f32 / 3.0;
        globals[25] = obs.opp.live_zone.len() as f32 / 3.0;
        let my_blade = obs.me.stage.iter().filter(|&&c| c >= 0).count() as f32;
        let opp_blade = obs.opp.stage.iter().filter(|&&c| c >= 0).count() as f32;
        globals[26] = my_blade / 3.0;
        globals[27] = opp_blade / 3.0;

        EncodedState {
            my_hand: sum_embeds(&obs.me.hand),
            my_stage: my_stage_enc,
            my_energy: sum_embeds(&obs.me.energy_zone),
            my_waitroom: sum_embeds(&obs.me.waitroom),
            my_live: sum_embeds(&obs.me.live_zone),
            my_success: sum_embeds(&obs.me.success_zone),
            opp_stage: opp_stage_enc,
            opp_energy: sum_embeds(&obs.opp.energy_zone),
            opp_waitroom: sum_embeds(&obs.opp.waitroom),
            opp_live: sum_embeds(&obs.opp.live_zone),
            opp_success: sum_embeds(&obs.opp.success_zone),
            globals,
        }
    }

    fn forward_state(&self, state_flat: &[f32]) -> Vec<f32> {
        let state_dim = EncodedState::state_dim();
        let mut h = vec![0.0f32; HIDDEN];
        for i in 0..HIDDEN {
            let mut s = self.b1[i];
            for j in 0..state_dim {
                s += self.w1_state[i * state_dim + j] * state_flat[j];
            }
            h[i] = relu(s);
        }
        h
    }

    fn action_logit(&self, h_state: &[f32], action_enc: &[f32]) -> f32 {
        let mut h = vec![0.0f32; HIDDEN];
        for i in 0..HIDDEN {
            let mut s = h_state[i];
            for j in 0..ACTION_ENC_DIM {
                s += self.w1_action[i * ACTION_ENC_DIM + j] * action_enc[j];
            }
            h[i] = relu(s);
        }
        let mut logit = self.b_policy;
        for i in 0..HIDDEN {
            logit += self.w_policy[i] * h[i];
        }
        logit
    }

    fn state_value(&self, h_state: &[f32]) -> f32 {
        let mut v = self.b_value;
        for i in 0..HIDDEN {
            v += self.w_value[i] * h_state[i];
        }
        v.tanh()
    }

    /// Compute policy logits and values for a batch of actions.
    pub fn evaluate_actions(
        &self,
        state: &EncodedState,
        actions: &[ActionEncoding],
    ) -> (Vec<f32>, f32) {
        let state_flat = state.flatten();
        let h_state = self.forward_state(&state_flat);
        let value = self.state_value(&h_state);

        let logits: Vec<f32> = actions
            .iter()
            .map(|act| {
                let act_enc =
                    act.encode(&self.card_embed, &self.zone_embed, &self.action_type_embed);
                self.action_logit(&h_state, &act_enc)
            })
            .collect();

        (logits, value)
    }

}

fn relu(x: f32) -> f32 {
    if x > 0.0 {
        x
    } else {
        0.0
    }
}

fn phase_as_u8(p: &crate::game_state::Phase) -> u8 {
    match p {
        crate::game_state::Phase::RockPaperScissors => 0,
        crate::game_state::Phase::ChooseFirstAttacker => 1,
        crate::game_state::Phase::MulliganFirstAttacker => 2,
        crate::game_state::Phase::MulliganSecondAttacker => 3,
        crate::game_state::Phase::Active => 4,
        crate::game_state::Phase::Energy => 5,
        crate::game_state::Phase::Draw => 6,
        crate::game_state::Phase::Main => 7,
        crate::game_state::Phase::LiveCardSetFirstAttacker => 8,
        crate::game_state::Phase::LiveCardSetSecondAttacker => 9,
        crate::game_state::Phase::FirstAttackerPerformance => 10,
        crate::game_state::Phase::SecondAttackerPerformance => 11,
        crate::game_state::Phase::LiveVictoryDetermination => 12,
    }
}
