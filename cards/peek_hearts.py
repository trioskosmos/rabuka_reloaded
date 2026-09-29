import io, json
from pathlib import Path

ROOT = Path(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded")
cards = json.load(io.open(ROOT / "cards" / "cards.json", encoding="utf-8"))


def need_total(c):
    nh = c.get("need_heart") or {}
    return sum(v for v in nh.values() if isinstance(v, (int, float)))


rows = []
for no, c in cards.items():
    if c.get("type") != "ライブ":
        continue
    if c.get("unit") != "μ's" and c.get("series") != "ラブライブ！":
        continue
    rows.append((need_total(c), no, c.get("name"), c.get("need_heart") or {}))

rows.sort()
totals = sorted({r[0] for r in rows})
print("distinct 必要ハートの合計 among 『μ's』 lives:", totals)
print()
print("candidates straddling 8:")
for t, no, name, nh in rows:
    if 6 <= t <= 10:
        print(f"  total={t:<3} {no:<22} {name:<28} need_heart={json.dumps(nh, ensure_ascii=False)}")
print()
below8 = [r for r in rows if r[0] == max([t for t in totals if t < 8], default=None)]
above8 = [r for r in rows if r[0] == min([t for t in totals if t >= 8], default=None)]
if below8:
    print("highest total below 8:", below8[0][:3], below8[0][3])
if above8:
    print("lowest total at/above 8:", above8[0][:3], above8[0][3])
