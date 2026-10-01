//! PPO with GAE for `PolicyNet`.
//!
//! ## Why this and not more behaviour cloning
//!
//! Cloning v7 reached 53% holdout action accuracy and then played at 35.5%,
//! because a cloned policy is capped at its teacher by construction - and its
//! teacher is a *delta-leaf searcher*, so the cap is low. RL instead optimises
//! win rate directly, which is the objective that actually matters and is not
//! bounded by anything v7 does.
//!
//! ## The algorithm
//!
//! Standard PPO-clip (Schulman et al. 2017) with Generalised Advantage
//! Estimation (GAE, Schulman et al. 2016):
//!
//! ```text
//!   delta_t   = r_t + gamma * V(s_{t+1}) - V(s_t)
//!   A_t       = delta_t + gamma * lambda * A_{t+1}
//!   ratio     = pi(a_t | s_t) / pi_old(a_t | s_t)
//!   L_policy  = -min(ratio * A_t, clip(ratio, 1-e, 1+e) * A_t)
//!   L_value   = (V(s_t) - V_target_t)^2
//!   L_total   = L_policy - c_v * L_value + c_e * H(pi)
//! ```
//!
//! Two details this game forces:
//!
//! - **Seat sign.** `ppo_collect` records both seats in one trajectory and
//!   signs each step's reward by its own seat, so advantages are already from
//!   the acting player's perspective and need no further negation here.
//! - **Trajectories are not sequential after the collector.** A recorded
//!   trajectory interleaves both seats, so `V(s_{t+1})` is the NEXT RECORDED
//!   STEP, not the next state in a Markov chain. GAE is therefore computed
//!   within each seat's contiguous run and the bootstrap is suppressed at a seat
//!   boundary. Using the raw next step's value across a seat boundary would
//!   bootstrap one player's state into the other's advantage.
//!
//! The entropy bonus matters more than usual here: §20.4 showed the cloned
//! policy collapses onto "never pass", so the fix for that class of failure is a
//! policy that keeps exploring the whole legal set.

use std::io::Read;

use rabuka_engine::bot::encoding::{ActionEncoding, EncodedState, ACTION_ENC_DIM};
use rabuka_engine::bot::neural::PolicyNet;

const HIDDEN: usize = 256;
const MAGIC: u32 = 0x5242_504B;
const ACTION_BYTES: usize = 15;

struct Step {
    state_flat: Vec<f32>,
    actions: Vec<ActionEncoding>,
    chosen: usize,
    old_log_prob: f32,
    old_value: f32,
    reward: f32,
    seat: u8,
}

fn decode_action(buf: &[u8], p: &mut usize) -> ActionEncoding {
    let base = *p;
    let u16_at = |off: usize| -> u16 { u16::from_le_bytes([buf[base + off], buf[base + off + 1]]) };
    let act = ActionEncoding {
        action_type: buf[base],
        target_card_id: u16_at(1) as i16,
        target_zone: buf[base + 3],
        position: buf[base + 4],
        ability_index: buf[base + 5],
        choice_option: u16_at(6),
        flags: buf[base + 8],
        final_cost: buf[base + 9],
        card_index: buf[base + 10],
        card_indices_count: buf[base + 11],
    };
    *p = base + ACTION_BYTES;
    act
}

fn load(path: &str) -> Vec<Step> {
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
    assert!(version >= 2, "unsupported trajectory version {version}");
    let has_seat = version >= 3;
    let state_dim = rd_u32(&buf, &mut p) as usize;
    let _ = rd_u32(&buf, &mut p);

    let mut steps = Vec::new();
    while p + 4 <= buf.len() {
        let n_steps = rd_u32(&buf, &mut p) as usize;
        if n_steps == 0 || n_steps > 5000 {
            break;
        }
        for _ in 0..n_steps {
            if p + 4 + state_dim * 4 > buf.len() {
                return steps;
            }
            let _sd = rd_u32(&buf, &mut p) as usize;
            let mut flat = Vec::with_capacity(state_dim);
            for _ in 0..state_dim {
                let b: [u8; 4] = buf[p..p + 4].try_into().unwrap();
                p += 4;
                flat.push(f32::from_le_bytes(b));
            }
            let n_actions = rd_u32(&buf, &mut p) as usize;
            assert!(n_actions <= 512, "implausible n_actions={n_actions} - format mismatch");
            let mut actions = Vec::with_capacity(n_actions);
            for _ in 0..n_actions {
                assert!(p + ACTION_BYTES <= buf.len(), "truncated action");
                actions.push(decode_action(&buf, &mut p));
            }
            let chosen = {
                let b: [u8; 2] = buf[p..p + 2].try_into().unwrap();
                p += 2;
                u16::from_le_bytes(b) as usize
            };
            let rd_f32 = |buf: &[u8], p: &mut usize| -> f32 {
                let b: [u8; 4] = buf[*p..*p + 4].try_into().unwrap();
                *p += 4;
                f32::from_le_bytes(b)
            };
            let old_log_prob = rd_f32(&buf, &mut p);
            let old_value = rd_f32(&buf, &mut p);
            let reward = rd_f32(&buf, &mut p);
            let _done = buf[p];
            p += 1;
            let seat = if has_seat {
                let s = buf[p];
                p += 1;
                s
            } else {
                0
            };
            steps.push(Step { state_flat: flat, actions, chosen, old_log_prob, old_value, reward, seat });
        }
    }
    steps
}

/// Forward pass, returning per-action logits, the value, and the entropy of the
/// policy over the legal set.
struct Fwd {
    logits: Vec<f32>,
    value: f32,
    entropy: f32,
    h_state: Vec<f32>,
    acts_enc: Vec<Vec<f32>>,
    hidden: Vec<Vec<f32>>,
    /// The flat state vector, needed for the `W1_state` gradient.
    state: Vec<f32>,
}

impl Fwd {
    #[inline]
    fn scalar_state(&self, k: usize) -> f32 {
        self.state[k]
    }
}

fn forward(net: &PolicyNet, step: &Step) -> Option<Fwd> {
    if step.actions.len() < 2 {
        return None;
    }
    let state_dim = EncodedState::state_dim();
    let mut h_state = vec![0.0f32; HIDDEN];
    for (i, row) in net.w1_state.chunks_exact(state_dim).enumerate() {
        let mut s = net.b1[i];
        for (w, x) in row.iter().zip(step.state_flat.iter()) {
            s += w * x;
        }
        h_state[i] = if s > 0.0 { s } else { 0.0 };
    }
    let value = {
        let mut v = net.b_value;
        for (w, h) in net.w_value.iter().zip(h_state.iter()) {
            v += w * h;
        }
        v
    };

    let mut logits = Vec::with_capacity(step.actions.len());
    let mut acts_enc = Vec::with_capacity(step.actions.len());
    let mut hidden = Vec::with_capacity(step.actions.len());
    for act in &step.actions {
        let enc = act.encode(&net.card_embed, &net.zone_embed, &net.action_type_embed);
        let mut hv = vec![0.0f32; HIDDEN];
        for (i, row) in net.w1_action.chunks_exact(ACTION_ENC_DIM).enumerate() {
            let mut s = h_state[i];
            for (w, x) in row.iter().zip(enc.iter()) {
                s += w * x;
            }
            hv[i] = if s > 0.0 { s } else { 0.0 };
        }
        let mut z = net.b_policy;
        for (w, h) in net.w_policy.iter().zip(hv.iter()) {
            z += w * h;
        }
        logits.push(z);
        acts_enc.push(enc);
        hidden.push(hv);
    }

    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0f32;
    let exps: Vec<f32> = logits.iter().map(|z| (z - max).exp()).collect();
    for e in &exps {
        sum += *e;
    }
    let mut entropy = 0.0f32;
    for e in &exps {
        let p = e / sum;
        if p > 1e-9 {
            entropy -= p * p.ln();
        }
    }

    Some(Fwd {
        logits,
        value,
        entropy,
        h_state,
        acts_enc,
        hidden,
        state: step.state_flat.clone(),
    })
}

/// GAE within seat-contiguous runs.
fn compute_gae(steps: &[Step], gamma: f32, lam: f32) -> (Vec<f32>, Vec<f32>) {
    let n = steps.len();
    let mut adv = vec![0.0f32; n];
    let mut ret = vec![0.0f32; n];
    let mut running = 0.0f32;
    let mut prev_seat = u8::MAX;
    for t in (0..n).rev() {
        let seat = steps[t].seat;
        if seat != prev_seat {
            // Seat boundary: no bootstrap across players, GAE restarts.
            running = 0.0;
            prev_seat = seat;
        }
        let next_v = if t + 1 < n && steps[t + 1].seat == seat {
            steps[t + 1].old_value
        } else {
            0.0
        };
        let delta = steps[t].reward + gamma * next_v - steps[t].old_value;
        running = delta + gamma * lam * running;
        adv[t] = running;
        ret[t] = running + steps[t].old_value;
    }
    (adv, ret)
}

fn main() {
    rabuka_engine::core::runtime_flags::set_training_mode(true);
    let traj = std::env::args().nth(1).unwrap_or_else(|| "../training/ppo_trajectories.bin".into());
    let init = std::env::args().nth(2);
    let out = std::env::args().nth(3).unwrap_or_else(|| "../training/policy_ppo.bin".into());
    let epochs: usize = env_u32("PPO_EPOCHS", 4) as usize;
    let iters: usize = env_u32("PPO_ITERS", 4) as usize;
    let lr: f32 = env_f32("PPO_LR", 0.0003);
    let gamma: f32 = env_f32("PPO_GAMMA", 0.99);
    let lam: f32 = env_f32("PPO_LAMBDA", 0.95);
    let clip_eps: f32 = env_f32("PPO_CLIP", 0.2);
    let v_coef: f32 = env_f32("PPO_VCOEF", 0.5);
    let ent_coef: f32 = env_f32("PPO_ENTCOEF", 0.01);
    let batch: usize = env_u32("PPO_BATCH", 1024) as usize;

    let steps = load(&traj);
    eprintln!("steps: {}", steps.len());
    if steps.is_empty() {
        eprintln!("nothing to train");
        return;
    }

    let mut net = PolicyNet::new();
    match &init {
        Some(p) if std::path::Path::new(p).exists() => {
            net.load_weights(p).expect("load init");
            eprintln!("warm-started from {p}");
        }
        _ => {
            net.init_random(0xC0FFEE);
            eprintln!("cold start (random init)");
        }
    }

    let mut rnd = 0x2545_F491_4F6C_DD1Du64;
    let mut rand_usize = |n: usize, r: &mut u64| -> usize {
        *r ^= *r << 13;
        *r ^= *r >> 7;
        *r ^= *r << 17;
        (*r >> 11) as usize % n.max(1)
    };

    let t0 = std::time::Instant::now();
    for iter in 0..iters {
        // Advantages are normalised ONCE per iteration, over the whole batch.
        //
        // Per-chunk normalisation is a common shortcut and it is wrong here: it
        // re-centres every minibatch on its own mean, so a chunk whose mean
        // advantage happens to sit below zero gets every sample pushed the same
        // way regardless of its own sign. Over an epoch the policy walks away
        // from the behaviour distribution it was supposed to stay near, and the
        // diagnostic is the ratio: |dlog ratio| climbed 4.5 -> 41.3 across five
        // iterations, which is a clipped PPO update that is not respecting its
        // own trust region.
        let (adv_raw, ret) = compute_gae(&steps, gamma, lam);
        let mut amean_g = 0.0f32;
        for &a in &adv_raw {
            amean_g += a;
        }
        amean_g /= adv_raw.len().max(1) as f32;
        let mut avar_g = 0.0f32;
        for &a in &adv_raw {
            let d = a - amean_g;
            avar_g += d * d;
        }
        let astd_g = (avar_g / adv_raw.len().max(1) as f32).sqrt().max(1e-6);
        let adv: Vec<f32> = adv_raw.iter().map(|a| (a - amean_g) / astd_g).collect();
        let mut idx: Vec<usize> = (0..steps.len()).collect();
        for i in (1..idx.len()).rev() {
            let j = rand_usize(i + 1, &mut rnd);
            idx.swap(i, j);
        }

        let mut pl_sum = 0.0;
        let mut vl_sum = 0.0;
        let mut ent_sum = 0.0;
        let mut kl_sum = 0.0;
        let mut counted = 0.0f64;

        for _epoch in 0..epochs {
            for chunk in idx.chunks(batch) {
                // `adv` is already globally normalised above, so there is no
                // per-chunk mean/std here any more - that was the drift.

                // The critic regresses onto the SAME normalised scale as the
                // policy. `ret` is in game-reward units ([-1, 1]) while `adv`
                // carries accumulated GAE magnitude; with a mean reward of -0.69
                // on a losing ladder rung, `ret` spread over +/-12 and the value
                // loss sat at 160+ for every iteration, swamping the policy term
                // (-0.004) by four orders of magnitude.
                let mut rmean = 0.0f32;
                for &i in chunk {
                    rmean += ret[i];
                }
                rmean /= chunk.len().max(1) as f32;
                let mut rvar = 0.0f32;
                for &i in chunk {
                    let d = ret[i] - rmean;
                    rvar += d * d;
                }
                let rstd = (rvar / chunk.len().max(1) as f32).sqrt().max(1e-6);

                // Gradients are accumulated across the whole chunk and applied
                // once. Applying them per sample means 88k sequential SGD steps
                // per epoch at this learning rate, which is a divergent
                // optimisation, not a batched one.
                let step_lr = lr / chunk.len().max(1) as f32;
                let mut touched_state = false;

                for &i in chunk {
                    let step = &steps[i];
                    let Some(f) = forward(&net, step) else { continue };
                    let c = step.chosen.min(f.logits.len() - 1);
                    // new log-prob of the taken action
                    let max = f.logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                    let mut sum = 0.0f32;
                    for z in &f.logits {
                        sum += (z - max).exp();
                    }
                    let logp = f.logits[c] - max - sum.ln();
                    let old = step.old_log_prob;
                    // PPO requires pi_new / pi_old on the SAME policy the data
                    // came from. If the stored behaviour log-prob is missing the
                    // ratio is meaningless, so the batch is treated as a no-op
                    // rather than silently trusting a fresh ratio of 1.
                    let log_old = old;
                    let ratio = if log_old.is_finite() {
                        (logp - log_old).clamp(-20.0, 20.0).exp()
                    } else {
                        1.0
                    };
                    // Clamp the normalised advantage: a win/loss terminal reward
                    // produces a few enormous advantages, and one of them
                    // dominating a gradient step is what NaN'd the value loss.
                    let a = adv[i].clamp(-10.0, 10.0);
                    let unclipped = ratio * a;
                    let clipped = ratio.clamp(1.0 - clip_eps, 1.0 + clip_eps) * a;
                    pl_sum += unclipped.min(clipped) as f64;
                    kl_sum += (logp - log_old).abs() as f64;

                    let v_target = (ret[i] - rmean) / rstd;
                    let vloss = f.value - v_target;
                    vl_sum += (vloss * vloss) as f64;
                    ent_sum += f.entropy as f64;
                    counted += 1.0;

                    // ---- policy backward (accumulated, applied per chunk) ----
                    let d = a;
                    let dclip = if unclipped <= clipped { d } else { 0.0 };
                    let scale = -dclip;
                    apply_policy_grad(&mut net, &f, c, scale, step_lr);
                    touched_state = true;
                    // ---- value backward ----
                    apply_value_grad(&mut net, &f.h_state, f.value - v_target, v_coef * step_lr);
                    if !f.value.is_finite() {
                        eprintln!("non-finite value encountered; skipping step");
                        return;
                    }
                }
                let _ = touched_state;
                // Entropy is a nudge away from the current policy; a full
                // backward pass on it is the same cost as the policy pass, so
                // it is applied per batch rather than per step.
            }
        }

        let c = counted.max(1.0);
        eprintln!(
            "iter {:2}  policy {:.4}  value {:.4}  entropy {:.4}  |dlogratio| {:.4}  ({:.0}s)",
            iter,
            pl_sum / c,
            vl_sum / c,
            ent_sum / c,
            kl_sum / c,
            t0.elapsed().as_secs_f64()
        );
    }

    net.save_weights(&out).expect("save");
    eprintln!("wrote {out}");
}

fn apply_policy_grad(net: &mut PolicyNet, f: &Fwd, chosen: usize, scale: f32, lr: f32) {
    let n = f.logits.len();
    let max = f.logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0f32;
    for z in &f.logits {
        sum += (z - max).exp();
    }
    let logsumexp = max + sum.ln();
    let mut probs = Vec::with_capacity(n);
    for z in &f.logits {
        probs.push((*z - logsumexp).exp());
    }
    // dL/dz_i = scale * (p_i - 1[i == chosen])
    let mut dz = vec![0.0f32; n];
    for i in 0..n {
        dz[i] = probs[i];
    }
    dz[chosen] -= 1.0;

    let mut dh_state = vec![0.0f32; HIDDEN];
    for i in 0..n {
        let d = scale * dz[i];
        if d.abs() < 1e-12 {
            continue;
        }
        for j in 0..HIDDEN {
            let g = d * net.w_policy[j];
            dh_state[j] += g;
            if f.hidden[i][j] > 0.0 {
                let row = &mut net.w1_action[j * ACTION_ENC_DIM..(j + 1) * ACTION_ENC_DIM];
                for k in 0..ACTION_ENC_DIM {
                    row[k] -= lr * g * f.acts_enc[i][k];
                }
            }
        }
        net.b_policy -= lr * d;
    }
    for j in 0..HIDDEN {
        let mut g = 0.0f32;
        for i in 0..n {
            g += dz[i] * f.hidden[i][j];
        }
        net.w_policy[j] -= lr * scale * g;
    }
    let state_dim = EncodedState::state_dim();
    // h_state passed in is post-ReLU; recompute the pre-activation sign from the
    // stored hidden vector is not possible, so use the value > 0 test.
    for j in 0..HIDDEN {
        if f.h_state[j] <= 0.0 {
            dh_state[j] = 0.0;
        }
        let d = dh_state[j];
        if d == 0.0 {
            continue;
        }
        let row = &mut net.w1_state[j * state_dim..(j + 1) * state_dim];
        for k in 0..state_dim {
            row[k] -= lr * d * f.scalar_state(k);
        }
    }
}

fn apply_value_grad(net: &mut PolicyNet, h_state: &[f32], err: f32, lr: f32) {
    for j in 0..HIDDEN {
        net.w_value[j] -= lr * 2.0 * err * h_state[j];
    }
    net.b_value -= lr * 2.0 * err;
}

fn env_f32(name: &str, default: f32) -> f32 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|x: &f32| x.is_finite())
        .unwrap_or(default)
}

fn env_u32(name: &str, default: u32) -> u32 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}