#!/usr/bin/env python3
"""PPO training for Rabuka Reloaded bot.

Pipeline:
  1. cargo run --bin ppo_collect -- <n_games> trajectories.bin [weights.bin]
  2. python train_ppo.py trajectories.bin policy_weights.bin --epochs 100
  3. cargo run --bin bot_demo -- policy_weights.bin

Architecture mirrors Rust PolicyNet (zone-aware encoder + 256-h units).
"""

import struct, math, sys, os, time, argparse
from dataclasses import dataclass
from typing import List
import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F
from torch.distributions import Categorical

# ─── Constants (match encoding.rs) ──────────────────────────────────────
CARD_EMBED_DIM = 128
NUM_CARDS = 2400
ZONE_EMBED_DIM = 16
NUM_ZONES = 15
ACTION_TYPE_EMBED_DIM = 16
NUM_ACTION_TYPES = 25
POSITION_FEATURES = 4
STAGE_ABILITY_FEATURES = 13
ACTION_EXTRA_FEATURES = 9
GLOBAL_FEATURES = 66
SCHEMA_VERSION = 4.0
TRAJECTORY_MAGIC = 0x5242504B
TRAJECTORY_VERSION = 2
ACTION_STRUCT = struct.Struct("<BhBBBHBBHBB2x")
ACTION_ENC_DIM = (
    ACTION_TYPE_EMBED_DIM
    + CARD_EMBED_DIM
    + ZONE_EMBED_DIM
    + POSITION_FEATURES
    + ACTION_EXTRA_FEATURES
)
HIDDEN = 256
STATE_DIM = (
    10 * CARD_EMBED_DIM
    + 6 * (CARD_EMBED_DIM + POSITION_FEATURES + STAGE_ABILITY_FEATURES)
    + GLOBAL_FEATURES
)


# ─── PyTorch Model ──────────────────────────────────────────────────────
class PolicyNetTorch(nn.Module):
    def __init__(self):
        super().__init__()
        self.card_embed = nn.Embedding(NUM_CARDS, CARD_EMBED_DIM)
        self.zone_embed = nn.Embedding(NUM_ZONES, ZONE_EMBED_DIM)
        self.action_type_embed = nn.Embedding(NUM_ACTION_TYPES, ACTION_TYPE_EMBED_DIM)
        self.fc_state = nn.Linear(STATE_DIM, HIDDEN)
        self.fc_action = nn.Linear(ACTION_ENC_DIM, HIDDEN, bias=False)
        self.fc_policy = nn.Linear(HIDDEN, 1)
        self.fc_value = nn.Linear(HIDDEN, 1)
        self._init_weights()

    def _init_weights(self):
        for m in [self.fc_state, self.fc_action, self.fc_policy, self.fc_value]:
            if hasattr(m, "weight"):
                nn.init.orthogonal_(
                    m.weight, gain=math.sqrt(2) if m is not self.fc_policy else 0.01
                )
            if hasattr(m, "bias") and m.bias is not None:
                nn.init.zeros_(m.bias)

    def encode_state(self, s: torch.Tensor) -> torch.Tensor:
        return F.relu(self.fc_state(s))

    def forward_value(self, h: torch.Tensor) -> torch.Tensor:
        return torch.tanh(self.fc_value(h)).squeeze(-1)

    def encode_actions(self, at, cid, zon, pos, extra):
        at_ = at.clamp(0, NUM_ACTION_TYPES - 1)
        c_ = self.card_embed(cid.clamp(0, NUM_CARDS - 1))
        z_ = self.zone_embed(zon.clamp(0, NUM_ZONES - 1))
        p_ = pos.float().unsqueeze(-1) / 3.0
        position_extra = torch.zeros(*p_.shape[:-1], POSITION_FEATURES - 1, device=p_.device)
        return torch.cat([self.action_type_embed(at_), c_, z_, p_, position_extra, extra], dim=-1)

    def forward(self, s, all_at, all_cid, all_zon, all_pos, all_extra, chosen, action_mask):
        h = self.encode_state(s)
        v = self.forward_value(h)
        ae = self.encode_actions(all_at, all_cid, all_zon, all_pos, all_extra)
        logits = h.unsqueeze(-2) + self.fc_action(ae)
        logits = self.fc_policy(F.relu(logits)).squeeze(-1)
        logits = logits.masked_fill(~action_mask, -1e9)
        dist = Categorical(F.softmax(logits, dim=-1))
        return dist.log_prob(chosen), v, dist.entropy()

    def save_weights(self, path: str):
        with open(path, "wb") as f:
            f.write(struct.pack("<f", SCHEMA_VERSION))
            for t in [
                self.card_embed.weight,
                self.zone_embed.weight,
                self.action_type_embed.weight,
                self.fc_state.weight,
                self.fc_state.bias,
                self.fc_action.weight,
                self.fc_policy.weight,
                self.fc_policy.bias,
                self.fc_value.weight,
                self.fc_value.bias,
            ]:
                for v in t.contiguous().view(-1).tolist():
                    f.write(struct.pack("<f", v))
        print(f"Saved weights to {path}")

    def load_weights(self, path: str):
        with open(path, "rb") as f:
            data = f.read()
        p = [0]

        def r(n):
            vals = struct.unpack_from(f"<{n}f", data, p[0])
            p[0] += n * 4
            return vals

        ver = r(1)[0]
        if ver != SCHEMA_VERSION:
            raise ValueError(f"unsupported policy schema version {ver}")
        print(f"Loading v{ver:g} from {path}")
        shapes = [
            ("card_embed", (NUM_CARDS, CARD_EMBED_DIM)),
            ("zone_embed", (NUM_ZONES, ZONE_EMBED_DIM)),
            ("action_type_embed", (NUM_ACTION_TYPES, ACTION_TYPE_EMBED_DIM)),
            ("fc_state.weight", (HIDDEN, STATE_DIM)),
            ("fc_state.bias", (HIDDEN,)),
            ("fc_action.weight", (HIDDEN, ACTION_ENC_DIM)),
            ("fc_policy.weight", (1, HIDDEN)),
            ("fc_policy.bias", (1,)),
            ("fc_value.weight", (1, HIDDEN)),
            ("fc_value.bias", (1,)),
        ]
        for name, sh in shapes:
            vals = torch.tensor(r(math.prod(sh)), dtype=torch.float32).reshape(sh)
            parts = name.split(".")
            if len(parts) == 2:
                getattr(self, parts[0]).__getattribute__(parts[1]).data = vals
            else:
                getattr(self, name).weight.data = vals


# ─── Data ───────────────────────────────────────────────────────────────
@dataclass
class Step:
    state: np.ndarray
    actions: np.ndarray
    chosen_idx: int
    old_log_prob: float
    old_value: float
    reward: float
    done: bool


def load_trajs(path: str) -> List[List[Step]]:
    with open(path, "rb") as f:
        data = f.read()
    if len(data) < 16:
        raise ValueError("trajectory file is shorter than its header")
    magic, version, state_dim, action_dim = struct.unpack_from("<4I", data, 0)
    if magic != TRAJECTORY_MAGIC or version != TRAJECTORY_VERSION:
        raise ValueError("unsupported trajectory format")
    if state_dim != STATE_DIM or action_dim != ACTION_ENC_DIM:
        raise ValueError(
            f"trajectory schema mismatch: state={state_dim}, action={action_dim}; "
            f"expected state={STATE_DIM}, action={ACTION_ENC_DIM}"
        )
    pos = [16]

    def take(fmt):
        size = struct.calcsize(fmt)
        if pos[0] + size > len(data):
            raise ValueError("truncated trajectory")
        value = struct.unpack_from(fmt, data, pos[0])[0]
        pos[0] += size
        return value

    trajs = []
    while pos[0] < len(data):
        n = take("<I")
        steps = []
        for _ in range(n):
            sd = take("<I")
            if sd != STATE_DIM:
                raise ValueError(f"state dimension mismatch: {sd}")
            if pos[0] + sd * 4 > len(data):
                raise ValueError("truncated state vector")
            state = np.asarray(struct.unpack_from(f"<{sd}f", data, pos[0]), dtype=np.float32)
            pos[0] += sd * 4
            na = take("<I")
            actions = []
            for _ in range(na):
                if pos[0] + ACTION_STRUCT.size > len(data):
                    raise ValueError("truncated action row")
                row = ACTION_STRUCT.unpack_from(data, pos[0])
                pos[0] += ACTION_STRUCT.size
                actions.append(list(row[:10]))
            steps.append(
                Step(
                    state,
                    np.asarray(actions, dtype=np.int32),
                    take("<H"),
                    take("<f"),
                    take("<f"),
                    take("<f"),
                    bool(take("<B")),
                )
            )
        trajs.append(steps)
    return trajs


def compute_gae(trajs, gamma, lam):
    for traj in trajs:
        if not traj:
            continue
        advs = np.zeros(len(traj), dtype=np.float32)
        gae = 0.0
        last = len(traj) - 1
        for t in reversed(range(len(traj))):
            step = traj[t]
            terminal = step.done or t == last
            next_value = 0.0 if terminal else traj[t + 1].old_value
            delta = step.reward + gamma * next_value - step.old_value
            gae = delta if terminal else delta + gamma * lam * gae
            advs[t] = gae
        for t, advantage in enumerate(advs):
            traj[t].old_value = advantage + traj[t].old_value


# ─── PPO ────────────────────────────────────────────────────────────────
@dataclass
class Config:
    lr: float
    gamma: float = 0.99
    lam: float = 0.95
    eps: float = 0.2
    vf: float = 0.5
    ent: float = 0.01
    max_norm: float = 0.5
    epochs: int = 10
    batch: int = 256


def train(model, trajs, cfg, device, save_path):
    compute_gae(trajs, cfg.gamma, cfg.lam)
    opt = torch.optim.Adam(model.parameters(), lr=cfg.lr)
    all_steps = [s for t in trajs for s in t]
    n = len(all_steps)
    print(f"Training on {n} steps, {cfg.epochs} epochs")

    for ep in range(cfg.epochs):
        np.random.shuffle(all_steps)
        pl, vl, el, kl, nb = 0, 0, 0, 0, 0

        for start in range(0, n, cfg.batch):
            batch = all_steps[start : start + cfg.batch]
            bs = len(batch)
            ma = max(len(s.actions) for s in batch)

            s_t = torch.zeros(bs, STATE_DIM, device=device)
            at_t = torch.zeros(bs, ma, dtype=torch.long, device=device)
            cid_t = torch.zeros(bs, ma, dtype=torch.long, device=device)
            zon_t = torch.zeros(bs, ma, dtype=torch.long, device=device)
            pos_t = torch.zeros(bs, ma, dtype=torch.long, device=device)
            extra_t = torch.zeros(bs, ma, ACTION_EXTRA_FEATURES, device=device)
            action_mask = torch.zeros(bs, ma, dtype=torch.bool, device=device)
            ch_t = torch.zeros(bs, dtype=torch.long, device=device)
            old_lp = torch.zeros(bs, device=device)
            ret_t = torch.zeros(bs, device=device)

            for i, s in enumerate(batch):
                s_t[i] = torch.from_numpy(s.state)
                na = len(s.actions)
                action_mask[i, :na] = True
                for j in range(na):
                    at_t[i, j] = int(s.actions[j, 0])
                    cid_t[i, j] = int(s.actions[j, 1])
                    zon_t[i, j] = int(s.actions[j, 2])
                    pos_t[i, j] = int(s.actions[j, 3])
                    ability = float(s.actions[j, 4]) / 8.0
                    choice = float(s.actions[j, 5]) / 16.0
                    card_index = 0.0 if int(s.actions[j, 8]) == 65535 else float(s.actions[j, 8]) / 16.0
                    card_count = float(s.actions[j, 9]) / 8.0
                    flags = int(s.actions[j, 6])
                    final_cost = 0.0 if int(s.actions[j, 7]) == 255 else float(s.actions[j, 7]) / 12.0
                    extra_t[i, j] = torch.tensor(
                        [
                            ability,
                            choice,
                            card_index,
                            card_count,
                            1.0 if flags & 1 else 0.0,
                            1.0 if flags & 2 else 0.0,
                            1.0 if flags & 4 else 0.0,
                            1.0 if flags & 8 else 0.0,
                            final_cost,
                        ],
                        device=device,
                    )
                ch_t[i] = s.chosen_idx
                old_lp[i] = s.old_log_prob
                ret_t[i] = float(s.old_value)  # after GAE, this is the return

            new_lp, vals, ent = model.forward(
                s_t, at_t, cid_t, zon_t, pos_t, extra_t, ch_t, action_mask
            )
            adv = ret_t - vals.detach()
            adv = (adv - adv.mean()) / (adv.std() + 1e-8)

            ratio = torch.exp(new_lp - old_lp)
            s1 = ratio * adv
            s2 = torch.clamp(ratio, 1 - cfg.eps, 1 + cfg.eps) * adv
            p_loss = -torch.min(s1, s2).mean()
            v_loss = F.mse_loss(vals, ret_t)
            e_loss = -ent.mean()
            loss = p_loss + cfg.vf * v_loss + cfg.ent * e_loss

            opt.zero_grad()
            loss.backward()
            nn.utils.clip_grad_norm_(model.parameters(), cfg.max_norm)
            opt.step()

            pl += p_loss.item()
            vl += v_loss.item()
            el += e_loss.item()
            kl += (ratio - 1 - new_lp + old_lp).mean().item()
            nb += 1

        if ep % 5 == 0 or ep == cfg.epochs - 1:
            print(
                f"E{ep:4d} pol={pl / nb:.4f} val={vl / nb:.4f} ent={el / nb:.4f} kl={kl / nb:.6f}"
            )

    model.save_weights(save_path)


def train_imitation(model, trajs, device, save_path, epochs, batch_size, lr):
    opt = torch.optim.Adam(model.parameters(), lr=lr)
    all_steps = [step for trajectory in trajs for step in trajectory]
    if not all_steps:
        raise ValueError("no imitation steps")
    print(f"Imitation on {len(all_steps)} steps, {epochs} epochs")
    for epoch in range(epochs):
        np.random.shuffle(all_steps)
        total_loss = 0.0
        batches = 0
        for start in range(0, len(all_steps), batch_size):
            batch = all_steps[start : start + batch_size]
            bs = len(batch)
            ma = max(len(step.actions) for step in batch)
            s_t = torch.zeros(bs, STATE_DIM, device=device)
            at_t = torch.zeros(bs, ma, dtype=torch.long, device=device)
            cid_t = torch.zeros(bs, ma, dtype=torch.long, device=device)
            zon_t = torch.zeros(bs, ma, dtype=torch.long, device=device)
            pos_t = torch.zeros(bs, ma, dtype=torch.long, device=device)
            extra_t = torch.zeros(bs, ma, ACTION_EXTRA_FEATURES, device=device)
            action_mask = torch.zeros(bs, ma, dtype=torch.bool, device=device)
            chosen_t = torch.zeros(bs, dtype=torch.long, device=device)
            for i, step in enumerate(batch):
                s_t[i] = torch.from_numpy(step.state)
                action_mask[i, : len(step.actions)] = True
                for j, values in enumerate(step.actions):
                    at_t[i, j] = int(values[0])
                    cid_t[i, j] = int(values[1])
                    zon_t[i, j] = int(values[2])
                    pos_t[i, j] = int(values[3])
                    ability = float(values[4]) / 8.0
                    choice = float(values[5]) / 16.0
                    card_index = 0.0 if int(values[8]) == 65535 else float(values[8]) / 16.0
                    card_count = float(values[9]) / 8.0
                    flags = int(values[6])
                    final_cost = 0.0 if int(values[7]) == 255 else float(values[7]) / 12.0
                    extra_t[i, j] = torch.tensor(
                        [
                            ability,
                            choice,
                            card_index,
                            card_count,
                            1.0 if flags & 1 else 0.0,
                            1.0 if flags & 2 else 0.0,
                            1.0 if flags & 4 else 0.0,
                            1.0 if flags & 8 else 0.0,
                            final_cost,
                        ],
                        device=device,
                    )
                chosen_t[i] = step.chosen_idx
            log_prob, _, _ = model.forward(
                s_t, at_t, cid_t, zon_t, pos_t, extra_t, chosen_t, action_mask
            )
            loss = -log_prob.mean()
            opt.zero_grad()
            loss.backward()
            nn.utils.clip_grad_norm_(model.parameters(), 0.5)
            opt.step()
            total_loss += loss.item()
            batches += 1
        if epoch % 5 == 0 or epoch == epochs - 1:
            print(f"E{epoch:4d} imitation={total_loss / max(batches, 1):.4f}")
    model.save_weights(save_path)


# ─── Main ───────────────────────────────────────────────────────────────
def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("data_path", nargs="?", default="../ppo_trajectories.bin")
    ap.add_argument("save_path", nargs="?", default="../policy_weights.bin")
    ap.add_argument("--load", default=None)
    ap.add_argument("--epochs", type=int, default=100)
    ap.add_argument("--lr", type=float, default=3e-4)
    ap.add_argument("--batch", type=int, default=256)
    ap.add_argument("--gamma", type=float, default=0.99)
    ap.add_argument("--lam", type=float, default=0.95)
    ap.add_argument("--clip", type=float, default=0.2)
    ap.add_argument("--entropy", type=float, default=0.01)
    ap.add_argument("--imitation", action="store_true")
    args = ap.parse_args()

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"Device: {device}")

    model = PolicyNetTorch().to(device)
    if args.load and os.path.exists(args.load):
        model.load_weights(args.load)

    print(f"Loading {args.data_path}...")
    trajs = load_trajs(args.data_path)
    total = sum(len(t) for t in trajs)
    print(f"Loaded {len(trajs)} trajs, {total} steps")

    cfg = Config(
        lr=args.lr,
        gamma=args.gamma,
        lam=args.lam,
        eps=args.clip,
        ent=args.entropy,
        epochs=args.epochs,
        batch=args.batch,
    )
    if args.imitation:
        train_imitation(model, trajs, device, args.save_path, args.epochs, args.batch, args.lr)
    else:
        train(model, trajs, cfg, device, args.save_path)


if __name__ == "__main__":
    main()
