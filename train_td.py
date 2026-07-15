"""TD policy+value network with pre-converted tensors for speed."""

import struct, sys, time
import numpy as np
import torch
import torch.nn as nn
import torch.optim as optim
from pathlib import Path

EMBED_DIM = 128
HIDDEN = 64
N = 2400
AT = 25
GAMMA = 0.95


class Net(nn.Module):
    def __init__(self):
        super().__init__()
        self.ce = nn.Embedding(N, EMBED_DIM, padding_idx=0)
        self.ae = nn.Embedding(AT, 16)
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

    def fe(self, my, op):
        return self.ce(my).sum(1), self.ce(op).sum(1)

    def fa(self, me, oe, at, ac):
        ate = self.ae(at).squeeze(1)
        ace = self.ce(ac).squeeze(1)
        x = torch.cat([me, oe, ate, ace], dim=1)
        h = torch.relu(self.t0(x))
        h = torch.relu(self.t1(h))
        return self.ph(h).squeeze(-1), self.vh(h).squeeze(-1)

    def fv(self, my, op):
        me = self.ce(my).sum(1)
        oe = self.ce(op).sum(1)
        x = torch.cat(
            [me, oe, torch.zeros(me.size(0), 16 + EMBED_DIM, device=me.device)], dim=1
        )
        h = torch.relu(self.t0(x))
        h = torch.relu(self.t1(h))
        return self.vh(h).squeeze(-1)


def load(path, max_ex=500000):
    d = Path(path).read_bytes()
    ex = []
    p = 0
    while p < len(d) and len(ex) < max_ex:
        hl = d[p]
        p += 1
        h = list(struct.unpack(f"<{hl}h", d[p : p + hl * 2]))
        p += hl * 2
        ms = list(struct.unpack("<3h", d[p : p + 6]))
        p += 6
        os = list(struct.unpack("<3h", d[p : p + 6]))
        p += 6
        ac = struct.unpack("<h", d[p : p + 2])[0]
        p += 2
        at = d[p]
        p += 1
        rw = struct.unpack("<f", d[p : p + 4])[0]
        p += 4
        nhl = d[p]
        p += 1
        nh = list(struct.unpack(f"<{nhl}h", d[p : p + nhl * 2]))
        p += nhl * 2
        nms = list(struct.unpack("<3h", d[p : p + 6]))
        p += 6
        nos = list(struct.unpack("<3h", d[p : p + 6]))
        p += 6
        my = [max(0, min(c, N - 1)) for c in h + ms if c >= 0] or [0]
        op = [max(0, min(c, N - 1)) for c in os if c >= 0] or [0]
        nm = [max(0, min(c, N - 1)) for c in nh + nms if c >= 0] or [0]
        no_ = [max(0, min(c, N - 1)) for c in nos if c >= 0] or [0]
        ex.append((my, op, at, max(0, min(ac, N - 1)), rw, nm, no_))
    return ex


def to_tensors(examples, device):
    """Pre-convert ALL examples to tensors once."""
    n = len(examples)
    max_my = max(len(x[0]) for x in examples)
    max_op = max(len(x[1]) for x in examples)
    max_nm = max(len(x[5]) for x in examples)
    max_no = max(len(x[6]) for x in examples)
    mp = torch.zeros(n, max_my, dtype=torch.long, device=device)
    op = torch.zeros(n, max_op, dtype=torch.long, device=device)
    nmp = torch.zeros(n, max_nm, dtype=torch.long, device=device)
    nop = torch.zeros(n, max_no, dtype=torch.long, device=device)
    ats = torch.zeros(n, 1, dtype=torch.long, device=device)
    acs = torch.zeros(n, 1, dtype=torch.long, device=device)
    rw = torch.zeros(n, dtype=torch.float32, device=device)
    for i, (m, o, at, ac, rd, nm, no_) in enumerate(examples):
        mp[i, : len(m)] = torch.tensor(m, device=device)
        op[i, : len(o)] = torch.tensor(o, device=device)
        nmp[i, : len(nm)] = torch.tensor(nm, device=device)
        nop[i, : len(no_)] = torch.tensor(no_, device=device)
        ats[i] = at
        acs[i] = ac
        rw[i] = rd
    return mp, op, ats, acs, rw, nmp, nop


def save(m, path):
    with open(path, "wb") as f:
        for w in [
            m.ce.weight,
            m.ae.weight,
            m.t0.weight,
            m.t0.bias,
            m.t1.weight,
            m.t1.bias,
            m.vh.weight,
            m.vh.bias,
            m.ph.weight,
            m.ph.bias,
        ]:
            f.write(w.detach().cpu().numpy().astype(np.float32).tobytes())
    print(f"  Saved {path}", flush=True)


def main():
    data_path = sys.argv[1] if len(sys.argv) > 1 else "../shaped_data.bin"
    epochs = int(sys.argv[2]) if len(sys.argv) > 2 else 10
    out_path = sys.argv[3] if len(sys.argv) > 3 else "../shaped_weights.bin"

    print(f"Loading {data_path}...", flush=True)
    t0 = time.time()
    ex = load(data_path)
    print(f"  {len(ex)} examples ({time.time() - t0:.1f}s)", flush=True)

    dev = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"Device: {dev}", flush=True)

    model = Net().to(dev)
    opt = optim.Adam(model.parameters(), lr=0.001)

    np.random.shuffle(ex)
    spl = int(len(ex) * 0.9)
    tr, va = ex[:spl], ex[spl:]

    print("Converting to tensors...", flush=True)
    t0 = time.time()
    tr_t = to_tensors(tr, dev)
    va_t = to_tensors(va, dev)
    print(f"  Done ({time.time() - t0:.1f}s)", flush=True)

    for epoch in range(epochs):
        model.train()
        model.zero_grad()
        mp, op, ats, acs, rw, nmp, nop = tr_t
        me, oe = model.fe(mp, op)
        nme, noe = model.fe(nmp, nop)
        logit, val = model.fa(me, oe, ats, acs)
        with torch.no_grad():
            nv = model.fv(nmp, nop).tanh()
        td = rw + GAMMA * nv
        vl = nn.MSELoss()(val.tanh(), td)
        adv = td - val.tanh().detach()
        pl = (-adv * logit).mean()
        loss = vl + 0.5 * pl
        loss.backward()
        torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
        opt.step()

        model.eval()
        with torch.no_grad():
            mp2, op2, _, _, _, nmp2, nop2 = va_t
            _, val2 = model.fa(
                *model.fe(mp2, op2),
                ats.new_zeros(va_t[0].size(0), 1),
                acs.new_zeros(va_t[0].size(0), 1),
            )
            nv2 = model.fv(nmp2, nop2).tanh()
            td2 = rw.new_zeros(va_t[0].size(0)) + GAMMA * nv2
            vl2 = nn.MSELoss()(val2.tanh(), td2)

        print(
            f"E{epoch + 1:2d} train={vl.item():.4f} val={vl2.item():.4f} ({time.time() - t0:.1f}s)",
            flush=True,
        )
        save(model, f"{out_path}.e{epoch + 1}")
        t0 = time.time()

    save(model, out_path)


if __name__ == "__main__":
    main()
