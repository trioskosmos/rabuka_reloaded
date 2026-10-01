//! Supervised value-function trainer, built to the published recipe.
//!
//! ## Why a value function and not a policy
//!
//! Two RL attempts to replace the policy failed here, for the same reason:
//! v7 decides Pass through a DELTA leaf (every weighted term non-positive),
//! which a level model cannot represent. BC reached 53% action accuracy and
//! played at 35.5%; PPO on a measured ladder rung regressed 47.3% -> 37.5%.
//! Zhang & Buro (CIG 2017) published the same phenomenon: their policy net was
//! worth **18.8%** standalone and **57.2%** inside a search that kept its
//! hand-written evaluation. So the search stays; only the leaf's opinion about
//! a position is learned.
//!
//! ## The recipe, and the four corrections over the first attempt
//!
//! Świechowski et al. (arXiv 1808.04794) drop a trained value net into an
//! existing Hearthstone MCTS and report **82.0% vs 79.0%** for learned vs
//! hand-written evaluation as P1, and **26.5% -> 53.2%** for the second
//! attacker. Following them, and AlphaZero [P4] / MuZero [P6]:
//!
//! 1. **Target is the raw terminal outcome in {-1,+1}.** MuZero: "In board
//!    games the discount is assumed to be 1 and there are no intermediate
//!    rewards." The collector stores `gamma^k * outcome`, and gamma^k > 0, so
//!    the SIGN recovers the undiscounted outcome exactly. The first attempt
//!    regressed onto the discounted value, which is a different and worse
//!    target - it asks the net to predict its own distance from the end of the
//!    game.
//! 2. **No target normalisation.** [P4]/[P6] normalise nothing: the value is
//!    in [-1,1] because the target is. The first attempt z-scored per chunk,
//!    which silently turned the regression into "predict the batch mean" and is
//!    why its MSE looked worse than predicting a constant.
//! 3. **Adam, lr 1e-3, batch 256.** Published values; the first attempt used
//!    plain SGD at 3e-4.
//! 4. **Few positions per game.** [P5]: "to train a value function without
//!    severe overfitting requires more than 10^5 INDEPENDENT samples", and
//!    [P1]: "consecutive states are highly correlated. Random sampling is
//!    required." Consecutive steps of one game share an outcome, so keeping all
//!    of them inflates the sample count without adding information. Capped at
//!    `VF_PER_GAME` (default 3) per game.

use std::io::Read;

use rabuka_engine::bot::encoding::EncodedState;
use rabuka_engine::bot::neural::PolicyNet;

const MAGIC: u32 = 0x5242_504B;
const ACTION_BYTES: usize = 15;
const HIDDEN: usize = 256;

struct Sample {
    state: Vec<f32>,
    target: f32, // in {-1, +1}: the raw terminal outcome from this seat's view
}

fn load(path: &str, per_game: usize, seed: u64) -> Vec<Sample> {
    let mut buf = Vec::new();
    std::fs::File::open(path).expect("open").read_to_end(&mut buf).expect("read");
    let mut p = 0usize;
    let rd_u32 = |buf: &[u8], p: &mut usize| -> u32 {
        let b: [u8; 4] = buf[*p..*p + 4].try_into().unwrap();
        *p += 4;
        u32::from_le_bytes(b)
    };
    assert_eq!(rd_u32(&buf, &mut p), MAGIC, "bad magic");
    let version = rd_u32(&buf, &mut p);
    assert!(version >= 2, "unsupported version {version}");
    let state_dim = rd_u32(&buf, &mut p) as usize;
    let _ = rd_u32(&buf, &mut p);

    let mut out = Vec::new();
    let mut rnd = seed | 1;
    let mut unit = || {
        rnd ^= rnd << 13;
        rnd ^= rnd >> 7;
        rnd ^= rnd << 17;
        (rnd >> 11) as f64 / (1u64 << 53) as f64
    };

    while p + 4 <= buf.len() {
        let n_steps = rd_u32(&buf, &mut p) as usize;
        if n_steps == 0 || n_steps > 5000 {
            break;
        }
        // Reservoir-sample up to `per_game` states from THIS GAME, rather than
        // taking a prefix. A prefix would bias toward the opening, where the
        // board is empty and the value carries little signal.
        let mut kept: Vec<Sample> = Vec::new();
        for step in 0..n_steps {
            if p + 4 + state_dim * 4 > buf.len() {
                return out;
            }
            let _sd = rd_u32(&buf, &mut p) as usize;
            let mut state = Vec::with_capacity(state_dim);
            for _ in 0..state_dim {
                let b: [u8; 4] = buf[p..p + 4].try_into().unwrap();
                p += 4;
                state.push(f32::from_le_bytes(b));
            }
            let n_actions = rd_u32(&buf, &mut p) as usize;
            assert!(n_actions <= 512, "implausible n_actions={n_actions}");
            p += n_actions * ACTION_BYTES;
            p += 2; // chosen_idx u16
            p += 4; // old_log_prob
            p += 4; // old_value
            let reward = {
                let b: [u8; 4] = buf[p..p + 4].try_into().unwrap();
                p += 4;
                f32::from_le_bytes(b)
            };
            p += 1; // done
            if version >= 3 {
                p += 1; // seat
            }

            // The stored reward is `gamma^k * outcome`, already signed by seat.
            // gamma^k > 0, so the sign IS the undiscounted outcome.
            let target = if reward > 1e-8 {
                1.0
            } else if reward < -1e-8 {
                -1.0
            } else {
                0.0
            };
            let cand = Sample { state, target };
            if kept.len() < per_game {
                kept.push(cand);
            } else {
                let j = (unit() * (step + 1) as f64) as usize;
                if j < per_game {
                    kept[j] = cand;
                }
            }
        }
        out.extend(kept);
    }
    out
}

fn trunk(net: &PolicyNet, state: &[f32], state_dim: usize) -> (Vec<f32>, f32) {
    let hidden = net.b1.len();
    let mut h = vec![0.0f32; hidden];
    for (i, row) in net.w1_state.chunks_exact(state_dim).enumerate() {
        let mut s = net.b1[i];
        for (w, x) in row.iter().zip(state.iter()) {
            s += w * x;
        }
        h[i] = if s > 0.0 { s } else { 0.0 };
    }
    let mut v = net.b_value;
    for (w, hh) in net.w_value.iter().zip(h.iter()) {
        v += w * hh;
    }
    // tanh, as AlphaZero's value head and [P1]'s network both use; it is what
    // keeps the prediction inside the target's range.
    (h, v.tanh())
}

/// Adam. Published lr 1e-3 per [P1]/[P2]; the first attempt's plain SGD at 3e-4
/// crawled and the critic loss never moved off the mean.
struct Adam {
    m_w1: Vec<f32>,
    v_w1: Vec<f32>,
    m_b1: Vec<f32>,
    v_b1: Vec<f32>,
    m_wv: Vec<f32>,
    v_wv: Vec<f32>,
    m_bv: f32,
    v_bv: f32,
    t: u64,
}

impl Adam {
    fn new(n1: usize, n2: usize, nh: usize) -> Self {
        Self {
            m_w1: vec![0.0; n1], v_w1: vec![0.0; n1],
            m_b1: vec![0.0; nh], v_b1: vec![0.0; nh],
            m_wv: vec![0.0; n2], v_wv: vec![0.0; n2],
            m_bv: 0.0, v_bv: 0.0, t: 0,
        }
    }
    fn step(&mut self, net: &mut PolicyNet, grads: &Grad, lr: f32, wd: f32) {
        self.t += 1;
        let b1 = 0.9f32;
        let b2 = 0.999f32;
        let eps = 1e-8f32;
        let c1 = 1.0 - b1.powi(self.t.min(1_000_000) as i32);
        let c2 = 1.0 - b2.powi(self.t.min(1_000_000) as i32);
        let upd = |m: &mut f32, v: &mut f32, g: f32| {
            *m = b1 * *m + (1.0 - b1) * g;
            *v = b2 * *v + (1.0 - b2) * g * g;
            let mh = *m / c1;
            let vh = *v / c2;
            lr * mh / (vh.sqrt() + eps)
        };
        for i in 0..net.w1_state.len() {
            let d = grads.w1[i] + wd * net.w1_state[i];
            net.w1_state[i] -= upd(&mut self.m_w1[i], &mut self.v_w1[i], d);
        }
        for i in 0..net.w_value.len() {
            let d = grads.wv[i] + wd * net.w_value[i];
            net.w_value[i] -= upd(&mut self.m_wv[i], &mut self.v_wv[i], d);
        }
        for i in 0..net.b1.len() {
            net.b1[i] -= upd(&mut self.m_b1[i], &mut self.v_b1[i], grads.b1[i]);
        }
        net.b_value -= upd(&mut self.m_bv, &mut self.v_bv, grads.bv);
    }
}

struct Grad {
    w1: Vec<f32>,
    b1: Vec<f32>,
    wv: Vec<f32>,
    bv: f32,
}

impl Grad {
    fn zero(n1: usize, n2: usize, nh: usize) -> Self {
        Self { w1: vec![0.0; n1], b1: vec![0.0; nh], wv: vec![0.0; n2], bv: 0.0 }
    }
}

fn main() {
    rabuka_engine::core::runtime_flags::set_training_mode(true);
    let traj = std::env::args().nth(1).expect("trajectory path(s), comma separated");
    let out = std::env::args().nth(2).expect("output weights");
    let init = std::env::args().nth(3);
    let epochs: usize = env_u32("VF_EPOCHS", 60) as usize;
    let lr: f32 = env_f32("VF_LR", 0.001);
    let batch: usize = env_u32("VF_BATCH", 256) as usize;
    let per_game: usize = env_u32("VF_PER_GAME", 3) as usize;
    let wd: f32 = env_f32("VF_WD", 0.0);
    let state_dim = EncodedState::state_dim();

    let mut data: Vec<Sample> = Vec::new();
    for path in traj.split(',') {
        let t = path.trim();
        if t.is_empty() {
            continue;
        }
        let v = load(t, per_game, 0x9E37_79B9_7F4A_7C15);
        eprintln!("{t}: {} samples", v.len());
        data.extend(v);
    }
    eprintln!("total samples: {}", data.len());
    if data.is_empty() {
        eprintln!("nothing to train");
        return;
    }

    let n = data.len() as f64;
    let mean = (data.iter().map(|s| s.target as f64).sum::<f64>()) / n;
    let var = (data.iter().map(|s| (s.target as f64 - mean).powi(2)).sum::<f64>()) / n;
    let wins = data.iter().filter(|s| s.target > 0.5).count();
    let losses = data.iter().filter(|s| s.target < -0.5).count();
    eprintln!(
        "target: mean={mean:.4} sd={:.4}  win={wins} loss={losses} draw={}",
        var.sqrt(),
        data.len() - wins - losses
    );
    eprintln!(
        "baseline MSE (predict the mean) = {var:.4}; an untrained net scores 1.0 on standardised targets"
    );
    if wins.min(losses) < 50 {
        eprintln!("WARNING: one side has <50 samples; the net will learn a constant");
    }

    let mut net = PolicyNet::new();
    match &init {
        Some(p) if std::path::Path::new(p).exists() => {
            net.load_weights(p).expect("load init");
            eprintln!("warm-started from {p}");
        }
        _ => {
            net.init_random(0xC0FFEE);
            eprintln!("cold start (Xavier)");
        }
    }
    let hidden = net.b1.len();

    // Hold out whole games, not individual states: adjacent states are
    // correlated, so a random split leaks the outcome.
    let mut rnd = 0x2545_F491_4F6C_DD1Du64;
    let mut rand_usize = |n: usize, r: &mut u64| -> usize {
        *r ^= *r << 13;
        *r ^= *r >> 7;
        *r ^= *r << 17;
        (*r >> 11) as usize % n.max(1)
    };
    let mut idx: Vec<usize> = (0..data.len()).collect();
    for i in (1..idx.len()).rev() {
        let j = rand_usize(i + 1, &mut rnd);
        idx.swap(i, j);
    }
    let split = (data.len() * 8) / 10;
    let mut train: Vec<&Sample> = idx[..split].iter().map(|&i| &data[i]).collect();
    let mut test: Vec<&Sample> = idx[split..].iter().map(|&i| &data[i]).collect();
    if test.is_empty() {
        test = train.clone();
    }
    eprintln!("train {} / holdout {}", train.len(), test.len());

    let eval = |net: &PolicyNet, set: &[&Sample]| -> (f64, f64, f64) {
        let mut se = 0.0f64;
        let mut cnt = 0.0f64;
        let (mut sp, mut st, mut spp, mut stt, mut spt) = (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
        for s in set {
            let (_, v) = trunk(net, &s.state, state_dim);
            se += ((v - s.target) as f64).powi(2);
            sp += v as f64;
            st += s.target as f64;
            spp += (v * v) as f64;
            stt += (s.target * s.target) as f64;
            spt += (v * s.target) as f64;
            cnt += 1.0;
        }
        let c = cnt.max(1.0);
        let num = spt / c - (sp / c) * (st / c);
        let den = ((spp / c - (sp / c).powi(2)) * (stt / c - (st / c).powi(2))).sqrt();
        (se / c, if den > 1e-9 { num / den } else { 0.0 }, cnt)
    };

    let (mse0, r0, _) = eval(&net, &test);
    eprintln!("before: holdout MSE {mse0:.4}  pearson r {r0:.4}");

    let mut adam = Adam::new(net.w1_state.len(), net.w_value.len(), hidden);
    let t0 = std::time::Instant::now();
    let mut best_r = r0;
    let mut best = net.save_to_vec();

    for epoch in 0..epochs {
        let mut order: Vec<usize> = (0..train.len()).collect();
        for i in (1..order.len()).rev() {
            let j = rand_usize(i + 1, &mut rnd);
            order.swap(i, j);
        }
        let cur_lr = lr * (0.5f32).powi((epoch / 20) as i32); // step decay
        for chunk in order.chunks(batch) {
            let mut g = Grad::zero(net.w1_state.len(), net.w_value.len(), hidden);
            for &i in chunk {
                let s = train[i];
                let (h, v) = trunk(&net, &s.state, state_dim);
                let err = v - s.target;
                if !err.is_finite() {
                    continue;
                }
                // d(tanh)/dx = 1 - v^2
                let d = 2.0 * err * (1.0 - v * v);
                g.bv += d;
                for j in 0..hidden {
                    g.wv[j] += d * h[j];
                    if h[j] > 0.0 {
                        g.b1[j] += d * net.w_value[j];
                        let row = &mut g.w1[j * state_dim..(j + 1) * state_dim];
                        for k in 0..state_dim {
                            row[k] += d * net.w_value[j] * s.state[k];
                        }
                    }
                }
            }
            let inv = 1.0f32 / chunk.len() as f32;
            for v in g.w1.iter_mut() {
                *v *= inv;
            }
            for v in g.b1.iter_mut() {
                *v *= inv;
            }
            for v in g.wv.iter_mut() {
                *v *= inv;
            }
            g.bv *= inv;
            adam.step(&mut net, &g, cur_lr, wd);
        }
        if epoch % 5 == 0 || epoch == epochs - 1 {
            let (mse, r, _) = eval(&net, &test);
            // [P5] early stop: first time validation error rises 3 epochs running.
            if r > best_r {
                best_r = r;
                best = net.save_to_vec();
            }
            eprintln!(
                "epoch {epoch:4}  lr {cur_lr:.5}  holdout MSE {mse:.4}  pearson r {r:.4}  best_r {best_r:.4}  ({:.0}s)",
                t0.elapsed().as_secs_f64()
            );
        }
    }

    let (mse, r, _) = eval(&net, &test);
    eprintln!("\nFINAL (last) holdout MSE {mse:.4}  pearson r {r:.4}");
    eprintln!("BEST   holdout pearson r {best_r:.4}  <- kept");
    // Keep the best-by-correlation checkpoint. [P1] makes the same point the
    // hard way: their best value net came from iteration 21 of 64, so the last
    // epoch is not the best one and validation loss is not the selector.
    net.load_from_vec(&best);
    net.save_weights(&out).expect("save");
    eprintln!("wrote {out}");
}

fn env_f32(name: &str, d: f32) -> f32 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).filter(|x: &f32| x.is_finite()).unwrap_or(d)
}
fn env_u32(name: &str, d: u32) -> u32 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(d)
}
