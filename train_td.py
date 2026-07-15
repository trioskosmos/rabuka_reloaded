"""TD policy+value network: V(s) = r + γV(s'), policy advantage = target - V(s)"""

import struct, sys, time
import numpy as np
import torch
import torch.nn as nn
import torch.optim as optim
from pathlib import Path

EMBED_DIM = 128
HIDDEN = 64
NUM_CARDS = 2400
NUM_ACTION_TYPES = 25
GAMMA = 0.95


class TDNet(nn.Module):
    def __init__(self):
        super().__init__()
        self.ce = nn.Embedding(NUM_CARDS, EMBED_DIM, padding_idx=0)
        self.ae = nn.Embedding(NUM_ACTION_TYPES, 16)
        self.t0 = nn.Linear(EMBED_DIM * 2 + 16 + EMBED_DIM, HIDDEN)
        self.t1 = nn.Linear(HIDDEN, HIDDEN)
        self.vh = nn.Linear(HIDDEN, 1)
        self.ph = nn.Linear(HIDDEN, 1)
        for m in self.modules():
            if isinstance(m, nn.Linear):
                nn.init.xavier_uniform_(m.weight)
                nn.init.zeros_(m.bias)
        nn.init.normal_(self.ce.weight, std=0.02)
        nn.init.normal_(self.ae.weight, std=0.02)

    def forward_action(self, my_ids, op_ids, at, ac):
        me = self.ce(my_ids).sum(1)
        oe = self.ce(op_ids).sum(1)
        ate = self.ae(at).squeeze(1)
        ace = self.ce(ac).squeeze(1)
        x = torch.cat([me, oe, ate, ace], dim=1)
        h = torch.relu(self.t0(x))
        h = torch.relu(self.t1(h))
        return self.ph(h).squeeze(-1), self.vh(h).squeeze(-1)

    def forward_value(self, my_ids, op_ids):
        me = self.ce(my_ids).sum(1)
        oe = self.ce(op_ids).sum(1)
        x = torch.cat(
            [me, oe, torch.zeros(me.size(0), 16 + EMBED_DIM, device=me.device)], dim=1
        )
        h = torch.relu(self.t0(x))
        h = torch.relu(self.t1(h))
        return self.vh(h).squeeze(-1)


def load_data(path, max_examples=200000):
    data = Path(path).read_bytes()
    pos, ex = 0, []
    while pos < len(data) and len(ex) < max_examples:
        hl = data[pos]
        pos += 1
        hand = list(struct.unpack(f"<{hl}h", data[pos : pos + hl * 2]))
        pos += hl * 2
        ms = list(struct.unpack("<3h", data[pos : pos + 6]))
        pos += 6
        os = list(struct.unpack("<3h", data[pos : pos + 6]))
        pos += 6
        ac = struct.unpack("<h", data[pos : pos + 2])[0]
        pos += 2
        at = data[pos]
        pos += 1
        rw = struct.unpack("<f", data[pos : pos + 4])[0]
        pos += 4
        nhl = data[pos]
        pos += 1
        nhand = list(struct.unpack(f"<{nhl}h", data[pos : pos + nhl * 2]))
        pos += nhl * 2
        nms = list(struct.unpack("<3h", data[pos : pos + 6]))
        pos += 6
        nos = list(struct.unpack("<3h", data[pos : pos + 6]))
        pos += 6
        my = [max(0, min(c, NUM_CARDS - 1)) for c in hand + ms if c >= 0] or [0]
        op = [max(0, min(c, NUM_CARDS - 1)) for c in os if c >= 0] or [0]
        nmy = [max(0, min(c, NUM_CARDS - 1)) for c in nhand + nms if c >= 0] or [0]
        nop = [max(0, min(c, NUM_CARDS - 1)) for c in nos if c >= 0] or [0]
        ex.append((my, op, at, max(0, min(ac, NUM_CARDS - 1)), rw, nmy, nop))
    return ex


def collate(batch, device):
    bs = len(batch)
    my_p = torch.zeros(
        bs, max(len(x[0]) for x in batch), dtype=torch.long, device=device
    )
    op_p = torch.zeros(
        bs, max(len(x[1]) for x in batch), dtype=torch.long, device=device
    )
    nmp = torch.zeros(
        bs, max(len(x[5]) for x in batch), dtype=torch.long, device=device
    )
    nop = torch.zeros(
        bs, max(len(x[6]) for x in batch), dtype=torch.long, device=device
    )
    ats = torch.zeros(bs, 1, dtype=torch.long, device=device)
    acs = torch.zeros(bs, 1, dtype=torch.long, device=device)
    rw = torch.zeros(bs, dtype=torch.float32, device=device)
    for j, (mids, oids, atid, acid, rd, nmids, noids) in enumerate(batch):
        my_p[j, : len(mids)] = torch.tensor(mids, device=device)
        op_p[j, : len(oids)] = torch.tensor(oids, device=device)
        nmp[j, : len(nmids)] = torch.tensor(nmids, device=device)
        nop[j, : len(noids)] = torch.tensor(noids, device=device)
        ats[j] = atid
        acs[j] = acid
        rw[j] = rd
    return my_p, op_p, ats, acs, rw, nmp, nop


def save_weights(model, path):
    with open(path, "wb") as f:
        f.write(model.ce.weight.detach().cpu().numpy().astype(np.float32).tobytes())
        f.write(model.ae.weight.detach().cpu().numpy().astype(np.float32).tobytes())
        f.write(model.t0.weight.detach().cpu().numpy().astype(np.float32).tobytes())
        f.write(model.t0.bias.detach().cpu().numpy().astype(np.float32).tobytes())
        f.write(model.t1.weight.detach().cpu().numpy().astype(np.float32).tobytes())
        f.write(model.t1.bias.detach().cpu().numpy().astype(np.float32).tobytes())
        f.write(model.vh.weight.detach().cpu().numpy().astype(np.float32).tobytes())
        f.write(model.vh.bias.detach().cpu().numpy().astype(np.float32).tobytes())
        f.write(model.ph.weight.detach().cpu().numpy().astype(np.float32).tobytes())
        f.write(model.ph.bias.detach().cpu().numpy().astype(np.float32).tobytes())
    print(f"  Saved {path}", flush=True)


def main():
    data_path = sys.argv[1] if len(sys.argv) > 1 else "../td_data.bin"
    epochs = int(sys.argv[2]) if len(sys.argv) > 2 else 10
    out_path = sys.argv[3] if len(sys.argv) > 3 else "../td_weights.bin"

    print(f"Loading {data_path}...", flush=True)
    t0 = time.time()
    data = load_data(data_path, 500000)
    print(f"  {len(data)} examples ({time.time() - t0:.1f}s)", flush=True)

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"Device: {device}", flush=True)

    model = TDNet().to(device)
    opt = optim.Adam(model.parameters(), lr=0.001)

    np.random.shuffle(data)
    spl = int(len(data) * 0.9)
    tr, va = data[:spl], data[spl:]
    bs = 65536

    t_start = time.time()
    for epoch in range(epochs):
        model.train()
        tl = 0.0
        for i in range(0, len(tr), bs):
            mp, op, ats, acs, rw, nmp, nop = collate(tr[i : i + bs], device)
            logit, val = model.forward_action(mp, op, ats, acs)
            with torch.no_grad():
                nv = model.forward_value(nmp, nop).tanh()
            td = rw + GAMMA * nv
            vl = nn.MSELoss()(val.tanh(), td)
            adv = td - val.tanh().detach()
            pl = (-adv * logit).mean()
            loss = vl + 0.5 * pl
            opt.zero_grad()
            loss.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            opt.step()
            tl += vl.item() * len(tr[i : i + bs])

        model.eval()
        vl2 = 0.0
        with torch.no_grad():
            for i in range(0, len(va), bs):
                mp, op, ats, acs, rw, nmp, nop = collate(va[i : i + bs], device)
                _, val = model.forward_action(mp, op, ats, acs)
                nv = model.forward_value(nmp, nop).tanh()
                td = rw + GAMMA * nv
                vl2 += nn.MSELoss()(val.tanh(), td).item() * len(va[i : i + bs])

        elapsed = time.time() - t_start
        print(
            f"E{epoch + 1:2d} train={tl / len(tr):.4f} val={vl2 / len(va):.4f} ({elapsed:.0f}s)",
            flush=True,
        )
        save_weights(model, f"{out_path}.e{epoch + 1}")

    save_weights(model, out_path)
    print(f"Done in {time.time() - t_start:.0f}s", flush=True)


if __name__ == "__main__":
    main()
