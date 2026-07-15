"""Policy + Value network trained via REINFORCE from game experience.
No heuristics. No game knowledge. Just plays games and learns what wins."""

import struct, sys
import numpy as np
import torch
import torch.nn as nn
import torch.optim as optim
from pathlib import Path

EMBED_DIM = 128
HIDDEN = 64
NUM_CARDS = 2400
NUM_ACTION_TYPES = 25
GAMMA = 0.99


class PolicyValueNet(nn.Module):
    def __init__(self):
        super().__init__()
        self.card_embed = nn.Embedding(NUM_CARDS, EMBED_DIM, padding_idx=0)
        self.action_embed = nn.Embedding(NUM_ACTION_TYPES, 16)
        trunk_in = (
            EMBED_DIM * 2 + 16 + EMBED_DIM
        )  # my_emb(128) + opp_emb(128) + action_type(16) + action_card(128) = 400
        self.trunk = nn.Sequential(
            nn.Linear(trunk_in, HIDDEN),
            nn.ReLU(),
            nn.Linear(HIDDEN, HIDDEN),
            nn.ReLU(),
        )
        self.value_head = nn.Linear(HIDDEN, 1)  # V(s) ∈ [-1,1]
        self.policy_head = nn.Linear(HIDDEN, 1)  # logit for this action
        self._init()

    def _init(self):
        for m in self.modules():
            if isinstance(m, nn.Linear):
                nn.init.xavier_uniform_(m.weight)
                nn.init.zeros_(m.bias)
        nn.init.normal_(self.card_embed.weight, std=0.02)
        nn.init.normal_(self.action_embed.weight, std=0.02)

    def embed_state(self, my_ids, opp_ids):
        my_emb = self.card_embed(my_ids).sum(dim=1)  # (B, 128)
        opp_emb = self.card_embed(opp_ids).sum(dim=1)  # (B, 128)
        return my_emb, opp_emb

    def forward(self, my_ids, opp_ids, action_type_ids, action_card_ids):
        my_emb, opp_emb = self.embed_state(my_ids, opp_ids)
        act_type_emb = self.action_embed(action_type_ids).squeeze(1)  # (B, 16)
        act_card_emb = self.card_embed(action_card_ids).squeeze(1)  # (B, 128)
        x = torch.cat([my_emb, opp_emb, act_type_emb, act_card_emb], dim=1)
        h = self.trunk(x)
        value = self.value_head(h).squeeze(-1).tanh()
        logit = self.policy_head(h).squeeze(-1)
        return logit, value

    def state_value(self, my_ids, opp_ids):
        my_emb, opp_emb = self.embed_state(my_ids, opp_ids)
        x = torch.cat([my_emb, opp_emb], dim=1)
        x = torch.cat(
            [x, torch.zeros(x.size(0), 16 + EMBED_DIM, device=x.device)], dim=1
        )
        h = self.trunk(x)
        return self.value_head(h).squeeze(-1).tanh()


def load_data(path):
    data = Path(path).read_bytes()
    pos, examples = 0, []
    while pos < len(data):
        hand_len = data[pos]
        pos += 1
        hand = list(struct.unpack(f"<{hand_len}h", data[pos : pos + hand_len * 2]))
        pos += hand_len * 2
        my_s = list(struct.unpack("<3h", data[pos : pos + 6]))
        pos += 6
        opp_s = list(struct.unpack("<3h", data[pos : pos + 6]))
        pos += 6
        act_card = struct.unpack("<h", data[pos : pos + 2])[0]
        pos += 2
        act_type = data[pos]
        pos += 1
        outcome = struct.unpack("<f", data[pos : pos + 4])[0]
        pos += 4
        examples.append((hand, my_s, opp_s, act_card, act_type, outcome))
    return examples


def collate(examples, device):
    my_list, opp_list, atypes, acards, outcomes = [], [], [], [], []
    for hand, my_s, opp_s, act_card, act_type, outcome in examples:
        my_ids = [max(0, min(c, NUM_CARDS - 1)) for c in hand + my_s if c >= 0]
        opp_ids = [max(0, min(c, NUM_CARDS - 1)) for c in opp_s if c >= 0]
        if not my_ids:
            my_ids = [0]
        if not opp_ids:
            opp_ids = [0]
        my_list.append(my_ids)
        opp_list.append(opp_ids)
        atypes.append(act_type)
        acards.append(max(0, min(act_card, NUM_CARDS - 1)))
        outcomes.append(outcome)
    # Pad
    max_my = max(len(m) for m in my_list)
    max_opp = max(len(o) for o in opp_list)
    my_pad = torch.zeros(len(examples), max_my, dtype=torch.long)
    opp_pad = torch.zeros(len(examples), max_opp, dtype=torch.long)
    for i, m in enumerate(my_list):
        my_pad[i, : len(m)] = torch.tensor(m)
    for i, o in enumerate(opp_list):
        opp_pad[i, : len(o)] = torch.tensor(o)
    return (
        my_pad.to(device),
        opp_pad.to(device),
        torch.tensor(atypes, dtype=torch.long).unsqueeze(1).to(device),
        torch.tensor(acards, dtype=torch.long).unsqueeze(1).to(device),
        torch.tensor(outcomes, dtype=torch.float32).to(device),
    )


def main():
    data_path = sys.argv[1] if len(sys.argv) > 1 else "../train_data.bin"
    epochs = int(sys.argv[2]) if len(sys.argv) > 2 else 20
    out_path = sys.argv[3] if len(sys.argv) > 3 else "../policy_weights.bin"

    print(f"Loading {data_path}...")
    examples = load_data(data_path)
    print(f"Loaded {len(examples)} (state, action, outcome) triples")

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"Device: {device}")

    model = PolicyValueNet().to(device)
    optimizer = optim.Adam(model.parameters(), lr=0.001)

    split = int(len(examples) * 0.9)
    train, val = examples[:split], examples[split:]
    batch_size = 512

    for epoch in range(epochs):
        model.train()
        np.random.shuffle(train)
        total_loss = 0
        for i in range(0, len(train), batch_size):
            batch = train[i : i + batch_size]
            my_pad, opp_pad, atypes, acards, targets = collate(batch, device)
            logit, value = model(my_pad, opp_pad, atypes, acards)

            # Value loss: predict outcome
            value_loss = nn.MSELoss()(value, targets)
            # Policy loss: encourage actions that led to wins
            # REINFORCE with value baseline
            advantage = targets - value.detach()
            policy_loss = -advantage * logit
            policy_loss = policy_loss.mean()

            loss = value_loss + 0.5 * policy_loss
            optimizer.zero_grad()
            loss.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()
            total_loss += loss.item() * len(batch)

        model.eval()
        val_loss = 0
        with torch.no_grad():
            for i in range(0, len(val), batch_size):
                batch = val[i : i + batch_size]
                my_pad, opp_pad, atypes, acards, targets = collate(batch, device)
                _, value = model(my_pad, opp_pad, atypes, acards)
                val_loss += nn.MSELoss()(value, targets).item() * len(batch)

        print(
            f"Epoch {epoch + 1:2d}: train={total_loss / len(train):.4f} val={val_loss / len(val):.4f}"
        )

    # Save
    torch.save(model.state_dict(), Path(out_path).with_suffix(".pt"))

    with open(out_path, "wb") as f:
        # card_embed: [NUM_CARDS, EMBED_DIM]
        f.write(
            model.card_embed.weight.detach().cpu().numpy().astype(np.float32).tobytes()
        )
        # action_embed: [NUM_ACTION_TYPES, 16]
        f.write(
            model.action_embed.weight.detach()
            .cpu()
            .numpy()
            .astype(np.float32)
            .tobytes()
        )
        # trunk: 2 linear layers
        for layer in model.trunk:
            if isinstance(layer, nn.Linear):
                f.write(
                    layer.weight.detach().cpu().numpy().astype(np.float32).tobytes()
                )
                f.write(layer.bias.detach().cpu().numpy().astype(np.float32).tobytes())
        # value_head
        f.write(
            model.value_head.weight.detach().cpu().numpy().astype(np.float32).tobytes()
        )
        f.write(
            model.value_head.bias.detach().cpu().numpy().astype(np.float32).tobytes()
        )
        # policy_head
        f.write(
            model.policy_head.weight.detach().cpu().numpy().astype(np.float32).tobytes()
        )
        f.write(
            model.policy_head.bias.detach().cpu().numpy().astype(np.float32).tobytes()
        )

    print(f"Saved to {out_path}")


if __name__ == "__main__":
    main()
