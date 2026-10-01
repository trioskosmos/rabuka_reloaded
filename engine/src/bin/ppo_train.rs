//! Behaviour-cloning trainer for `PolicyNet`.
//!
//! ## Why behaviour cloning and not PPO
//!
//! The project's own data pipeline (`ppo_collect`) can already emit
//! trajectories labelled by v7's choice (`PPO_BEHAVIOR=v7`), and the net and
//! encoder already exist - but nothing in the tree could *write* weights, so
//! the whole neural seat was unreachable. Two facts decide the order of work:
//!
//! - The 2024 exploitability study of ByteRL (arXiv:2404.16689) found that
//!   plain behaviour cloning of a strong agent's policy was nearly on par with
//!   the SOTA self-play RL agent, and that the policy - not the search and not
//!   the RL fine-tuning - carried the strength. So BC is the cheap shot worth
//!   taking first, and if it cannot even clone the teacher it will not be worth
//!   an RL run either.
//! - v7 is a strong teacher here: it is the project's best Main leaf, and
//!   §13-§19 of the improvement log spent fifteen interventions failing to
//!   improve on its hand-weighted arithmetic. Cloning it is a direct test of
//!   whether those weights are learnable at all.
//!
//! ## The loss
//!
//! Softmax cross-entropy over the legal actions at each decision, which is the
//! maximum-likelihood policy gradient with reward fixed at the teacher's choice.
//! Gradients are written through the net's own forward arithmetic rather than a
//! re-derivation, so train and inference cannot drift apart:
//!
//! ```text
//!   h       = relu(W1_state . s + b1)
//!   pre_i   = h + W1_action . a_i          (per action i)
//!   n_i     = relu(pre_i)
//!   z_i     = w_policy . n_i + b_policy
//!   L       = -log softmax(z)[chosen]
//! ```
//!
//! dL/dz_i = p_i - 1[i == chosen], then backprop through ReLU.

use std::io::Read;

use rabuka_engine::bot::encoding::{
    ActionEncoding, EncodedState, ACTION_ENC_DIM, CARD_EMBED_DIM, NUM_ZONES, ZONE_EMBED_DIM,
    ACTION_TYPE_COUNT, ACTION_TYPE_EMBED_DIM, SCHEMA_VERSION,
};
use rabuka_engine::bot::neural::PolicyNet;

const HIDDEN: usize = 256;
const MAGIC: u32 = 0x5242_504B; // 'RBPK'
const VERSION: u32 = 2;

/// Per-action serialised width in `ppo_collect`'s trajectory format. Computed
/// from the field types rather than guessed:
///
/// ```text
///   action_type  u8   1   (write_all(&[act.action_type]))
///   target_card_id i16 2
///   zone,position,ability u8 x3  3
///   choice_option u16 2
///   flags,final_cost u8 x2       2
///   card_index   u8   1   (to_le_bytes() on a u8 is ONE byte)
///   count,0,0,0  u8 x4       4
///                             -- 15
/// ```
///
/// Reading this as 16 bytes desynchronises the stream on the first action and
/// then interprets a length as an allocation size (observed: a 40GB request on
/// a 1989-step file).
const ACTION_BYTES: usize = 15;

struct Step {
    state_flat: Vec<f32>,
    actions: Vec<ActionEncoding>,
    chosen: usize,
}

/// Decode one 15-byte action record.
///
/// Written with explicit offsets and a single advance at the end. An earlier
/// version read fields as a struct literal of side-effecting calls, where the
/// struct field order (not the writer's order) decided consumption, and the
/// trailing `*p += 4` double-counted the `card_indices_count` byte the writer
/// already spends inside its `count,0,0,0` group — one byte lost per action,
/// which desynchronised the stream by the third action of step 1.
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

fn load_trajectories(path: &str) -> Vec<Step> {
    let mut buf = Vec::new();
    std::fs::File::open(path)
        .expect("open trajectories")
        .read_to_end(&mut buf)
        .expect("read trajectories");
    let mut p = 0usize;
    let rd_u32 = |buf: &[u8], p: &mut usize| -> u32 {
        let b: [u8; 4] = buf[*p..*p + 4].try_into().unwrap();
        *p += 4;
        u32::from_le_bytes(b)
    };
    let magic = rd_u32(&buf, &mut p);
    assert_eq!(magic, MAGIC, "bad trajectory magic");
    let version = rd_u32(&buf, &mut p);
    assert_eq!(version, VERSION, "trajectory version mismatch");
    let state_dim = rd_u32(&buf, &mut p) as usize;
    let _action_dim = rd_u32(&buf, &mut p) as usize;

    let mut steps = Vec::new();
    let mut step_idx = 0usize;
    while p + 4 <= buf.len() {
        let n_steps = rd_u32(&buf, &mut p) as usize;
        for _ in 0..n_steps {
            step_idx += 1;
            if p + 4 > buf.len() {
                return steps;
            }
            let sd = rd_u32(&buf, &mut p) as usize;
            debug_assert_eq!(sd, state_dim);
            let mut flat = Vec::with_capacity(sd);
            for _ in 0..sd {
                let b: [u8; 4] = buf[p..p + 4].try_into().unwrap();
                p += 4;
                flat.push(f32::from_le_bytes(b));
            }
            let n_actions = rd_u32(&buf, &mut p) as usize;
            // A desynced stream turns a length field into an absurd count, and
            // the resulting Vec::with_capacity is what asked for 40GB. Fail
            // loudly on anything that cannot possibly be a real action list
            // rather than trying to allocate it.
            assert!(
                n_actions <= 512,
                "implausible n_actions={n_actions} at byte {p} - trajectory format mismatch? \
                 (expected {} bytes per action)",
                ACTION_BYTES
            );
            let mut actions = Vec::with_capacity(n_actions);
            for _ in 0..n_actions {
                assert!(
                    p + ACTION_BYTES <= buf.len(),
                    "truncated action at byte {p} of {}",
                    buf.len()
                );
                actions.push(decode_action(&buf, &mut p));
            }
            // Trailer widths, from `ppo_collect`'s writer:
            //   chosen_idx u16 (2) | old_log_prob f32 (4) | old_value f32 (4)
            //   | reward f32 (4) | done u8 (1)  = 15 bytes.
            // `chosen_idx` is a u16 field, so reading it as u32 consumed the
            // next float's bytes and yielded 65535.
            let chosen = {
                let b: [u8; 2] = buf[p..p + 2].try_into().unwrap();
                p += 2;
                u16::from_le_bytes(b) as usize
            };
            assert!(
                chosen < n_actions.max(1),
                "chosen index {chosen} out of range for {n_actions} actions \
                 (traj step {step_idx}, byte {p})"
            );
            // old_log_prob, old_value, reward - unused for BC.
            p += 4 * 3 + 1;
            steps.push(Step { state_flat: flat, actions, chosen });
        }
    }
    steps
}

/// One minibatch step of softmax cross-entropy, gradients written into `net`.
fn train_batch(net: &mut PolicyNet, batch: &[&Step], lr: f32, l2: f32) -> (f64, f64) {
    let state_dim = EncodedState::state_dim();
    let mut total_loss = 0.0;
    let mut correct = 0.0;

    for step in batch {
        // ---- forward (mirrors PolicyNet::evaluate_actions exactly) ----
        let mut h_state = vec![0.0f32; HIDDEN];
        for (i, row) in net.w1_state.chunks_exact(state_dim).enumerate() {
            let mut s = net.b1[i];
            for (w, x) in row.iter().zip(step.state_flat.iter()) {
                s += w * x;
            }
            h_state[i] = if s > 0.0 { s } else { 0.0 };
        }

        let n = step.actions.len();
        if n < 2 {
            continue;
        }
        let mut logits = Vec::with_capacity(n);
        let mut acts_enc: Vec<Vec<f32>> = Vec::with_capacity(n);
        let mut hidden = Vec::with_capacity(n);
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
            for (w, v) in net.w_policy.iter().zip(hv.iter()) {
                z += w * v;
            }
            logits.push(z);
            acts_enc.push(enc);
            hidden.push(hv);
        }

        // ---- softmax + loss ----
        let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let mut exp = Vec::with_capacity(n);
        let mut sum = 0.0f32;
        for &z in &logits {
            let e = (z - max).exp();
            exp.push(e);
            sum += e;
        }
        let probs: Vec<f32> = exp.iter().map(|e| e / sum).collect();
        let chosen = step.chosen.min(n - 1);
        total_loss += -((probs[chosen] + 1e-10).ln() as f64);
        if probs[chosen] == probs.iter().cloned().fold(f32::NEG_INFINITY, f32::max) {
            correct += 1.0;
        }

        // ---- backward ----
        let mut dz = vec![0.0f32; n];
        for (i, p) in probs.iter().enumerate() {
            dz[i] = *p;
        }
        dz[chosen] -= 1.0;

        let mut dh_state = vec![0.0f32; HIDDEN];
        for i in 0..n {
            let d = dz[i];
            if d.abs() < 1e-12 {
                continue;
            }
            for j in 0..HIDDEN {
                let g = d * net.w_policy[j];
                if g == 0.0 {
                    continue;
                }
                // d(pre_i[j]) flows both into h_state[j] and W1_action[j].
                dh_state[j] += g;
                let row = &mut net.w1_action[j * ACTION_ENC_DIM..(j + 1) * ACTION_ENC_DIM];
                let hv = hidden[i][j];
                if hv > 0.0 {
                    for k in 0..ACTION_ENC_DIM {
                        row[k] -= lr * g * acts_enc[i][k];
                    }
                }
            }
            net.b_policy -= lr * d;
        }
        // dL/dw_policy
        for j in 0..HIDDEN {
            let mut g = 0.0f32;
            for i in 0..n {
                g += dz[i] * hidden[i][j];
            }
            net.w_policy[j] -= lr * g;
        }
        // ReLU' on the trunk
        for j in 0..HIDDEN {
            if h_state[j] <= 0.0 {
                dh_state[j] = 0.0;
            }
            let d = dh_state[j];
            net.b1[j] -= lr * d;
            let row = &mut net.w1_state[j * state_dim..(j + 1) * state_dim];
            for k in 0..state_dim {
                row[k] -= lr * d * step.state_flat[k];
            }
        }
        // weight decay
        if l2 > 0.0 {
            for w in net.w_policy.iter_mut() {
                *w -= lr * l2 * *w;
            }
        }
    }

    let nb = batch.len().max(1) as f64;
    (total_loss / nb, correct / nb)
}

/// Held-out accuracy, the metric that decides whether cloning worked at all.
fn accuracy(net: &PolicyNet, steps: &[&Step]) -> (f64, f64) {
    let state_dim = EncodedState::state_dim();
    let mut correct = 0.0;
    let mut nll = 0.0;
    let mut counted = 0.0;
    for step in steps {
        if step.actions.len() < 2 {
            continue;
        }
        let mut h_state = vec![0.0f32; HIDDEN];
        for (i, row) in net.w1_state.chunks_exact(state_dim).enumerate() {
            let mut s = net.b1[i];
            for (w, x) in row.iter().zip(step.state_flat.iter()) {
                s += w * x;
            }
            h_state[i] = if s > 0.0 { s } else { 0.0 };
        }
        let mut logits = Vec::with_capacity(step.actions.len());
        for act in &step.actions {
            let enc = act.encode(&net.card_embed, &net.zone_embed, &net.action_type_embed);
            let mut z = net.b_policy;
            for (j, row) in net.w1_action.chunks_exact(ACTION_ENC_DIM).enumerate() {
                let mut s = h_state[j];
                for (w, x) in row.iter().zip(enc.iter()) {
                    s += w * x;
                }
                let hv = if s > 0.0 { s } else { 0.0 };
                z += net.w_policy[j] * hv;
            }
            logits.push(z);
        }
        let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let mut sum = 0.0f32;
        let exps: Vec<f32> = logits.iter().map(|z| (z - max).exp()).collect();
        for e in &exps {
            sum += *e;
        }
        let chosen = step.chosen.min(logits.len() - 1);
        let p = exps[chosen] / sum;
        nll += -((p + 1e-10).ln() as f64);
        if probs_argmax(&logits) == chosen {
            correct += 1.0;
        }
        counted += 1.0;
    }
    let c = (counted as f64).max(1.0);
    (correct / c, nll / c)
}

fn probs_argmax(logits: &[f32]) -> usize {
    let mut best = 0usize;
    for (i, z) in logits.iter().enumerate() {
        if *z > logits[best] {
            best = i;
        }
    }
    best
}

fn main() {
    rabuka_engine::core::runtime_flags::set_training_mode(true);
    let traj_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "../training/ppo_trajectories.bin".into());
    let out_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "../training/policy_bc.bin".into());
    let epochs: usize = std::env::var("BC_EPOCHS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    let lr: f32 = std::env::var("BC_LR")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.01);
    let l2: f32 = std::env::var("BC_L2")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1e-5);
    let holdout = std::env::var("BC_HOLDOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.1f64);

    eprintln!("loading {} ...", traj_path);
    let steps = load_trajectories(&traj_path);
    eprintln!("steps: {}", steps.len());
    if steps.is_empty() {
        eprintln!("no steps; nothing to train");
        return;
    }

    // Split by STEP INDEX is wrong here - consecutive steps are near-identical
    // states from one game, so a random split leaks the teacher nearly
    // perfectly. Split on a coarse game stride instead: every k-th 20-step
    // block goes to holdout.
    let block = 20usize;
    let holdout_every = ((1.0 / holdout).round() as usize).max(2);
    let mut train: Vec<&Step> = Vec::new();
    let mut test: Vec<&Step> = Vec::new();
    for (i, s) in steps.iter().enumerate() {
        if (i / block) % holdout_every == 0 {
            test.push(s);
        } else {
            train.push(s);
        }
    }
    eprintln!("train {} / holdout {}", train.len(), test.len());

    let mut net = PolicyNet::new();
    net.init_random(0xC0FFEE);
    let (acc0, nll0) = accuracy(&net, &test);
    eprintln!(
        "before training: holdout top-1 {:.4}  nll {:.4}",
        acc0, nll0
    );

    let batch_size = 64usize;
    let start = std::time::Instant::now();
    for epoch in 0..epochs {
        // Deterministic reshuffle each epoch.
        let mut idx: Vec<usize> = (0..train.len()).collect();
        let mut s = 0x9E37_79B9_7F4A_7C15u64 ^ (epoch as u64);
        for i in (1..idx.len()).rev() {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            idx.swap(i, (s as usize) % (i + 1));
        }
        let mut epoch_loss = 0.0;
        let mut batches = 0.0;
        for chunk in idx.chunks(batch_size) {
            let batch: Vec<&Step> = chunk.iter().map(|&i| train[i]).collect();
            let (l, _) = train_batch(&mut net, &batch, lr, l2);
            epoch_loss += l;
            batches += 1.0;
        }
        if epoch % 5 == 0 || epoch == epochs - 1 {
            let (acc, nll) = accuracy(&net, &test);
            eprintln!(
                "epoch {:3}  train_loss {:.4}  holdout top-1 {:.4}  nll {:.4}  ({:.0}s)",
                epoch,
                epoch_loss / batches,
                acc,
                nll,
                start.elapsed().as_secs_f64()
            );
        }
    }

    let (acc, nll) = accuracy(&net, &test);
    eprintln!("\nFINAL holdout top-1 {:.4}  nll {:.4}", acc, nll);
    net.save_weights(&out_path).expect("save weights");
    eprintln!("wrote {}", out_path);
    eprintln!("schema version written: {}", SCHEMA_VERSION);
    let _ = (NUM_ZONES, ZONE_EMBED_DIM, ACTION_TYPE_COUNT, ACTION_TYPE_EMBED_DIM, CARD_EMBED_DIM);
    let _ = ACTION_BYTES;
}