/// Policy network loaded from Python training.
/// Fast inference: score all actions in one forward pass.
use std::fs::File;
use std::io::Read;

const EMBED_DIM: usize = 128;
const HIDDEN: usize = 64;
const NUM_ACTIONS: usize = 25;
const TRUNK_IN: usize = EMBED_DIM * 2 + 16 + EMBED_DIM; // 400

pub struct PolicyNet {
    card_embed: Vec<[f32; EMBED_DIM]>,      // [num_cards, 128]
    action_embed: [[f32; 16]; NUM_ACTIONS], // [16, 16]
    trunk_w1: [[f32; TRUNK_IN]; HIDDEN],    // [64, 400]
    trunk_b1: [f32; HIDDEN],                // [64]
    trunk_w2: [[f32; HIDDEN]; HIDDEN],      // [64, 64]
    trunk_b2: [f32; HIDDEN],                // [64]
    vw: [f32; HIDDEN],                      // value_head weight [64]
    vb: f32,                                // value_head bias
    pw: [f32; HIDDEN],                      // policy_head weight [64]
    pb: f32,                                // policy_head bias
}

impl PolicyNet {
    pub fn new(num_cards: usize) -> Self {
        let table_size = num_cards.max(2400);
        Self {
            card_embed: vec![[0.0f32; EMBED_DIM]; table_size],
            action_embed: [[0.0f32; 16]; NUM_ACTIONS],
            trunk_w1: [[0.0f32; TRUNK_IN]; HIDDEN],
            trunk_b1: [0.0f32; HIDDEN],
            trunk_w2: [[0.0f32; HIDDEN]; HIDDEN],
            trunk_b2: [0.0f32; HIDDEN],
            vw: [0.0f32; HIDDEN],
            vb: 0.0,
            pw: [0.0f32; HIDDEN],
            pb: 0.0,
        }
    }

    pub fn load_weights(&mut self, path: &str) -> std::io::Result<()> {
        let mut f = File::open(path)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        let mut pos = 0usize;

        macro_rules! read {
            ($dst:expr) => {{
                let n = std::mem::size_of_val(&$dst);
                if pos + 4 * ($dst.len() * if $dst.is_empty() { 0 } else { 1 }) > buf.len() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "weights truncated",
                    ));
                }
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        buf[pos..].as_ptr(),
                        &mut $dst as *mut _ as *mut u8,
                        n,
                    );
                }
                pos += n;
            }};
        }

        // card_embed: up to 2400×128 f32
        let max_emb = self.card_embed.len().min(2400);
        for i in 0..max_emb {
            for j in 0..EMBED_DIM {
                let bytes: [u8; 4] = buf[pos..pos + 4].try_into().unwrap();
                self.card_embed[i][j] = f32::from_le_bytes(bytes);
                pos += 4;
            }
        }
        // skip any extra embeddings in file
        pos += (2400usize.saturating_sub(max_emb)) * EMBED_DIM * 4;

        // action_embed: 16×16 f32
        for i in 0..NUM_ACTIONS {
            for j in 0..16 {
                let bytes: [u8; 4] = buf[pos..pos + 4].try_into().unwrap();
                self.action_embed[i][j] = f32::from_le_bytes(bytes);
                pos += 4;
            }
        }

        // trunk.0.weight: [64, 400]
        for i in 0..HIDDEN {
            for j in 0..TRUNK_IN {
                let bytes: [u8; 4] = buf[pos..pos + 4].try_into().unwrap();
                self.trunk_w1[i][j] = f32::from_le_bytes(bytes);
                pos += 4;
            }
        }
        // trunk.0.bias: [64]
        for i in 0..HIDDEN {
            let bytes: [u8; 4] = buf[pos..pos + 4].try_into().unwrap();
            self.trunk_b1[i] = f32::from_le_bytes(bytes);
            pos += 4;
        }
        // trunk.2.weight: [64, 64]
        for i in 0..HIDDEN {
            for j in 0..HIDDEN {
                let bytes: [u8; 4] = buf[pos..pos + 4].try_into().unwrap();
                self.trunk_w2[i][j] = f32::from_le_bytes(bytes);
                pos += 4;
            }
        }
        // trunk.2.bias: [64]
        for i in 0..HIDDEN {
            let bytes: [u8; 4] = buf[pos..pos + 4].try_into().unwrap();
            self.trunk_b2[i] = f32::from_le_bytes(bytes);
            pos += 4;
        }
        // value_head.weight: [1, 64]
        for i in 0..HIDDEN {
            let bytes: [u8; 4] = buf[pos..pos + 4].try_into().unwrap();
            self.vw[i] = f32::from_le_bytes(bytes);
            pos += 4;
        }
        // value_head.bias: [1]
        let bytes: [u8; 4] = buf[pos..pos + 4].try_into().unwrap();
        self.vb = f32::from_le_bytes(bytes);
        pos += 4;

        // policy_head.weight: [1, 64]
        for i in 0..HIDDEN {
            let bytes: [u8; 4] = buf[pos..pos + 4].try_into().unwrap();
            self.pw[i] = f32::from_le_bytes(bytes);
            pos += 4;
        }
        // policy_head.bias: [1]
        let bytes: [u8; 4] = buf[pos..pos + 4].try_into().unwrap();
        self.pb = f32::from_le_bytes(bytes);

        Ok(())
    }

    fn embed(&self, ids: &[i16]) -> [f32; EMBED_DIM] {
        let mut s = [0.0f32; EMBED_DIM];
        for &cid in ids {
            let idx = cid.max(0) as usize;
            if let Some(e) = self.card_embed.get(idx) {
                for i in 0..EMBED_DIM {
                    s[i] += e[i];
                }
            }
        }
        s
    }

    /// State embedding for action scoring: sum of my card embeddings + sum of opp card embeddings.
    fn state_embed(&self, my_ids: &[i16], opp_ids: &[i16]) -> [f32; 256] {
        let mut x = [0.0f32; 256];
        let my = self.embed(my_ids);
        let opp = self.embed(opp_ids);
        for i in 0..128 {
            x[i] = my[i];
        }
        for i in 0..128 {
            x[128 + i] = opp[i];
        }
        x
    }

    /// Forward pass: produce (logit, value) for a single (state, action).
    fn forward(&self, state: &[f32; 256], act_type: u8, act_card: i16) -> (f32, f32) {
        let at = (act_type as usize).min(NUM_ACTIONS - 1);
        let act_card_id = act_card.max(0) as usize;
        // Build input: state[256] || action_type_embed[16] || card_embed[128] = [400]
        let mut x = [0.0f32; TRUNK_IN];
        for i in 0..256 {
            x[i] = state[i];
        }
        let ae = &self.action_embed[at];
        for i in 0..16 {
            x[256 + i] = ae[i];
        }
        if let Some(ce) = self.card_embed.get(act_card_id) {
            for i in 0..128 {
                x[272 + i] = ce[i];
            }
        }
        // Trunk
        let mut h = [0.0f32; HIDDEN];
        for i in 0..HIDDEN {
            let mut s = self.trunk_b1[i];
            for j in 0..TRUNK_IN {
                s += self.trunk_w1[i][j] * x[j];
            }
            h[i] = if s > 0.0 { s } else { 0.0 };
        }
        let mut h2 = [0.0f32; HIDDEN];
        for i in 0..HIDDEN {
            let mut s = self.trunk_b2[i];
            for j in 0..HIDDEN {
                s += self.trunk_w2[i][j] * h[j];
            }
            h2[i] = if s > 0.0 { s } else { 0.0 };
        }
        // Value head
        let mut v = self.vb;
        for j in 0..HIDDEN {
            v += self.vw[j] * h2[j];
        }
        let value = v.tanh();
        // Policy head
        let mut logit = self.pb;
        for j in 0..HIDDEN {
            logit += self.pw[j] * h2[j];
        }
        (logit, value)
    }

    /// Score a single (state, action): return policy logit.
    pub fn score_action(
        &self,
        my_ids: &[i16],
        opp_ids: &[i16],
        act_type: u8,
        act_card: i16,
    ) -> f32 {
        let state = self.state_embed(my_ids, opp_ids);
        let (logit, _) = self.forward(&state, act_type, act_card);
        logit
    }

    /// State value without any action.
    pub fn state_value(&self, my_ids: &[i16], opp_ids: &[i16]) -> f32 {
        let state = self.state_embed(my_ids, opp_ids);
        let mut x = [0.0f32; TRUNK_IN];
        for i in 0..256 {
            x[i] = state[i];
        }
        let mut h = [0.0f32; HIDDEN];
        for i in 0..HIDDEN {
            let mut s = self.trunk_b1[i];
            for j in 0..TRUNK_IN {
                s += self.trunk_w1[i][j] * x[j];
            }
            h[i] = if s > 0.0 { s } else { 0.0 };
        }
        let mut h2 = [0.0f32; HIDDEN];
        for i in 0..HIDDEN {
            let mut s = self.trunk_b2[i];
            for j in 0..HIDDEN {
                s += self.trunk_w2[i][j] * h[j];
            }
            h2[i] = if s > 0.0 { s } else { 0.0 };
        }
        let mut v = self.vb;
        for j in 0..HIDDEN {
            v += self.vw[j] * h2[j];
        }
        v.tanh()
    }
}
