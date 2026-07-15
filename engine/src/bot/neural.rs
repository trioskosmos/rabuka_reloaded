/// Learned 128-dim card embeddings + 2-layer MLP evaluation network.
/// Trained from self-play: predicts final success margin from board state.
use rand::Rng;

const EMBED_DIM: usize = 128;
const HIDDEN: usize = 64;

fn relu(x: f32) -> f32 {
    if x > 0.0 {
        x
    } else {
        0.0
    }
}
fn d_relu(x: f32) -> f32 {
    if x > 0.0 {
        1.0
    } else {
        0.0
    }
}

pub struct ValueNetwork {
    embeddings: Vec<[f32; EMBED_DIM]>,
    w1: Vec<[f32; 256]>, // [HIDDEN][256]
    b1: [f32; HIDDEN],
    w2: [f32; HIDDEN], // [1][HIDDEN]
    b2: f32,
    lr: f32,
}

impl ValueNetwork {
    pub fn new(num_cards: usize) -> Self {
        let mut rng = rand::thread_rng();
        let scale = 0.02;
        let embeddings = (0..num_cards)
            .map(|_| {
                let mut e = [0.0f32; EMBED_DIM];
                for i in 0..EMBED_DIM {
                    e[i] = rng.gen::<f32>() * scale - scale * 0.5;
                }
                e
            })
            .collect();
        let w1 = (0..HIDDEN)
            .map(|_| {
                let mut row = [0.0f32; 256];
                for i in 0..256 {
                    row[i] = rng.gen::<f32>() * scale - scale * 0.5;
                }
                row
            })
            .collect();
        let mut b1 = [0.0f32; HIDDEN];
        let mut w2 = [0.0f32; HIDDEN];
        for i in 0..HIDDEN {
            w2[i] = rng.gen::<f32>() * scale - scale * 0.5;
        }
        Self {
            embeddings,
            w1,
            b1,
            w2,
            b2: 0.0,
            lr: 0.001,
        }
    }

    pub fn predict(&self, my_cards: &[i16], opp_cards: &[i16]) -> f32 {
        self.forward(my_cards, opp_cards).0
    }

    /// Forward pass. Returns (output, cached activations for backprop).
    fn forward(&self, my_cards: &[i16], opp_cards: &[i16]) -> (f32, Cached) {
        let mut my_sum = [0.0f32; EMBED_DIM];
        let mut opp_sum = [0.0f32; EMBED_DIM];
        for &cid in my_cards {
            let idx = cid.max(0) as usize;
            if let Some(e) = self.embeddings.get(idx) {
                for i in 0..EMBED_DIM {
                    my_sum[i] += e[i];
                }
            }
        }
        for &cid in opp_cards {
            let idx = cid.max(0) as usize;
            if let Some(e) = self.embeddings.get(idx) {
                for i in 0..EMBED_DIM {
                    opp_sum[i] += e[i];
                }
            }
        }

        let mut x = [0.0f32; 256];
        for i in 0..EMBED_DIM {
            x[i] = my_sum[i];
        }
        for i in 0..EMBED_DIM {
            x[EMBED_DIM + i] = opp_sum[i];
        }

        let mut h = [0.0f32; HIDDEN];
        let mut h_pre = [0.0f32; HIDDEN];
        for i in 0..HIDDEN {
            let mut s = self.b1[i];
            for j in 0..256 {
                s += self.w1[i][j] * x[j];
            }
            h_pre[i] = s;
            h[i] = relu(s);
        }

        let mut out = self.b2;
        for j in 0..HIDDEN {
            out += self.w2[j] * h[j];
        }
        let pred = out.tanh();

        (
            pred,
            Cached {
                my_sum,
                opp_sum,
                x,
                h_pre,
                h,
                pre_tanh: out,
            },
        )
    }

    /// Train on one example: update weights to reduce MSE.
    pub fn train(&mut self, my_cards: &[i16], opp_cards: &[i16], target: f32) {
        let (pred, cache) = self.forward(my_cards, opp_cards);
        let error = pred - target;
        let loss_grad = 2.0 * error; // d(MSE)/dpred = 2*(pred - target)

        // d(tanh)/d(pre_tanh) = 1 - tanh^2
        let d_pre = loss_grad * (1.0 - pred * pred);

        // Layer 2 gradients
        // w2[j] contributes d_pre * h[j]
        // b2 contributes d_pre * 1
        let mut d_h = [0.0f32; HIDDEN]; // gradient w.r.t. h[j]
        for j in 0..HIDDEN {
            let gw2 = d_pre * cache.h[j];
            d_h[j] = d_pre * self.w2[j];
            self.w2[j] -= self.lr * gw2;
        }
        self.b2 -= self.lr * d_pre;

        // Layer 1 gradients (through ReLU)
        let lr = self.lr;
        for i in 0..HIDDEN {
            let d_relu_val = d_h[i] * d_relu(cache.h_pre[i]);
            // w1[i][j] += -lr * d_relu_val * x[j]
            // b1[i] += -lr * d_relu_val
            self.b1[i] -= lr * d_relu_val;
            for j in 0..256 {
                let gw1 = d_relu_val * cache.x[j];
                self.w1[i][j] -= lr * gw1;
            }
        }

        // Embedding gradients
        let mut d_my = [0.0f32; EMBED_DIM];
        let mut d_opp = [0.0f32; EMBED_DIM];
        for i in 0..HIDDEN {
            let d_relu_val = d_h[i] * d_relu(cache.h_pre[i]);
            for j in 0..EMBED_DIM {
                d_my[j] += d_relu_val * self.w1[i][j];
                d_opp[j] += d_relu_val * self.w1[i][EMBED_DIM + j];
            }
        }

        for &cid in my_cards {
            let idx = cid.max(0) as usize;
            if let Some(e) = self.embeddings.get_mut(idx) {
                for i in 0..EMBED_DIM {
                    e[i] -= lr * d_my[i];
                }
            }
        }
        for &cid in opp_cards {
            let idx = cid.max(0) as usize;
            if let Some(e) = self.embeddings.get_mut(idx) {
                for i in 0..EMBED_DIM {
                    e[i] -= lr * d_opp[i];
                }
            }
        }
    }
}

struct Cached {
    my_sum: [f32; EMBED_DIM],
    opp_sum: [f32; EMBED_DIM],
    x: [f32; 256],
    h_pre: [f32; HIDDEN],
    h: [f32; HIDDEN],
    pre_tanh: f32,
}
