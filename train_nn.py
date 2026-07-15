"""Train 128-dim card embeddings + MLP value network with TD discounting."""

import struct
import numpy as np
import torch
import torch.nn as nn
import torch.optim as optim
from pathlib import Path

EMBED_DIM = 128
HIDDEN = 64
NUM_CARDS = 2400
GAMMA = 0.99  # discount factor for TD learning


class ValueNet(nn.Module):
    def __init__(self, num_cards=NUM_CARDS):
        super().__init__()
        self.embeddings = nn.Embedding(num_cards, EMBED_DIM, padding_idx=0)
        self.net = nn.Sequential(
            nn.Linear(EMBED_DIM * 2, HIDDEN),
            nn.ReLU(),
            nn.Linear(HIDDEN, 1),
            nn.Tanh(),
        )
        nn.init.normal_(self.embeddings.weight, std=0.02)
        for m in self.net:
            if isinstance(m, nn.Linear):
                nn.init.normal_(m.weight, std=0.02)
                nn.init.zeros_(m.bias)

    def forward(self, my_ids, opp_ids):
        my_emb = self.embeddings(my_ids).sum(dim=1)
        opp_emb = self.embeddings(opp_ids).sum(dim=1)
        x = torch.cat([my_emb, opp_emb], dim=1)
        return self.net(x).squeeze(-1)


def load_data(path: str, max_examples: int = None):
    """Load binary data with steps_remaining for TD discounting.
    Format per entry:
      u8 hand_len
      [i16; hand_len] hand cards
      [i16; 3] my_stage
      [i16; 3] opp_stage
      f32 target (final margin)
      u16 steps_remaining
    """
    data = Path(path).read_bytes()
    pos = 0
    my_list, opp_list, targets, discounts = [], [], [], []
    while pos < len(data):
        if max_examples and len(my_list) >= max_examples:
            break
        hand_len = data[pos]
        pos += 1
        hand = struct.unpack(f"<{hand_len}h", data[pos : pos + hand_len * 2])
        pos += hand_len * 2
        my_stage = struct.unpack("<3h", data[pos : pos + 6])
        pos += 6
        opp_stage = struct.unpack("<3h", data[pos : pos + 6])
        pos += 6
        target = struct.unpack("<f", data[pos : pos + 4])[0]
        pos += 4
        steps_rem = struct.unpack("<H", data[pos : pos + 2])[0]
        pos += 2

        my_ids = [
            max(0, min(c, NUM_CARDS - 1)) for c in list(hand) + list(my_stage) if c >= 0
        ]
        opp_ids = [max(0, min(c, NUM_CARDS - 1)) for c in list(opp_stage) if c >= 0]
        if not my_ids:
            my_ids = [0]

        my_list.append(my_ids)
        opp_list.append(opp_ids)
        targets.append(target)
        discounts.append(GAMMA**steps_rem)

    discounts = np.array(discounts, dtype=np.float32)
    targets = np.array(targets, dtype=np.float32) * discounts  # TD discount
    return my_list, opp_list, targets


def collate(batch):
    my_ids, opp_ids, targets = zip(*batch)
    my_lens = [len(m) for m in my_ids]
    opp_lens = [len(o) for o in opp_ids]
    max_my = max(my_lens)
    max_opp = max(opp_lens)

    my_padded = torch.zeros(len(batch), max_my, dtype=torch.long)
    opp_padded = torch.zeros(len(batch), max_opp, dtype=torch.long)
    for i, m in enumerate(my_ids):
        my_padded[i, : len(m)] = torch.tensor(m, dtype=torch.long)
    for i, o in enumerate(opp_ids):
        opp_padded[i, : len(o)] = torch.tensor(o, dtype=torch.long)

    targets = torch.tensor(targets, dtype=torch.float32)
    return my_padded, opp_padded, targets


def main():
    import sys

    data_path = sys.argv[1] if len(sys.argv) > 1 else "../training_data.bin"
    epochs = int(sys.argv[2]) if len(sys.argv) > 2 else 20
    out_path = sys.argv[3] if len(sys.argv) > 3 else "../card_weights.bin"

    print(f"Loading data from {data_path} with γ={GAMMA} TD discounting ...")
    my_list, opp_list, targets = load_data(data_path)
    n = len(my_list)
    print(
        f"Loaded {n} examples, TD targets range [{targets.min():.4f}, {targets.max():.4f}]"
    )

    split = int(n * 0.9)
    train_data = list(zip(my_list[:split], opp_list[:split], targets[:split]))
    val_data = list(zip(my_list[split:], opp_list[split:], targets[split:]))

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"Using device: {device}")

    model = ValueNet().to(device)
    optimizer = optim.Adam(model.parameters(), lr=0.001)
    scheduler = optim.lr_scheduler.StepLR(optimizer, step_size=5, gamma=0.5)
    batch_size = 1024

    for epoch in range(epochs):
        model.train()
        total_loss = 0.0
        np.random.shuffle(train_data)
        for i in range(0, len(train_data), batch_size):
            batch = train_data[i : i + batch_size]
            my_pad, opp_pad, tgt = collate(batch)
            my_pad, opp_pad, tgt = my_pad.to(device), opp_pad.to(device), tgt.to(device)
            pred = model(my_pad, opp_pad)
            loss = nn.MSELoss()(pred, tgt)
            optimizer.zero_grad()
            loss.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()
            total_loss += loss.item() * len(batch)

        model.eval()
        val_loss = 0.0
        with torch.no_grad():
            for i in range(0, len(val_data), batch_size):
                batch = val_data[i : i + batch_size]
                my_pad, opp_pad, tgt = collate(batch)
                my_pad, opp_pad, tgt = (
                    my_pad.to(device),
                    opp_pad.to(device),
                    tgt.to(device),
                )
                pred = model(my_pad, opp_pad)
                val_loss += nn.MSELoss()(pred, tgt).item() * len(batch)

        scheduler.step()
        print(
            f"Epoch {epoch + 1:2d}: train MSE={total_loss / len(train_data):.6f}  val MSE={val_loss / len(val_data):.6f}"
        )

    torch.save(model.state_dict(), Path(str(out_path) + ".pt"))
    print(f"Saved PyTorch weights")

    with open(out_path, "wb") as f:
        emb = model.embeddings.weight.detach().cpu().numpy().astype(np.float32)
        f.write(emb.tobytes())
        w1 = model.net[0].weight.detach().cpu().numpy().astype(np.float32)
        f.write(w1.tobytes())
        b1 = model.net[0].bias.detach().cpu().numpy().astype(np.float32)
        f.write(b1.tobytes())
        w2 = model.net[2].weight.detach().cpu().numpy().astype(np.float32)
        f.write(w2.tobytes())
        b2 = model.net[2].bias.detach().cpu().numpy().astype(np.float32)
        f.write(b2.tobytes())

    print(f"Saved flat binary to {out_path} ({Path(out_path).stat().st_size} bytes)")


if __name__ == "__main__":
    main()
