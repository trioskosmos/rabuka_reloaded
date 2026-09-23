# -*- coding: utf-8 -*-
"""Parse V7_DEBUG action-score trace and write a full decision-by-decision report.

Usage: python analyze_v7_trace.py <logfile> <out.txt>
"""
import re
import sys
from collections import Counter


def parse_value(s):
    m = re.search(r"value=(-?inf|nan|-?\d+\.\d+)", s)
    if not m:
        return None
    t = m.group(1)
    if t == "-inf":
        return float("-inf")
    if t == "nan":
        return float("nan")
    return float(t)


def parse_m(ln):
    d = {"raw": ln, "fc": -1, "t": None, "me": None}
    for key in ("t", "me"):
        m = re.search(rf"{key}=(\d+)", ln)
        if m:
            d[key] = int(m.group(1))
    m = re.search(r"fc=(-?\d+)", ln)
    if m:
        d["fc"] = int(m.group(1))
    m = re.search(r"act=(\S+)", ln)
    d["act"] = m.group(1) if m else "?"
    m = re.search(r"card=(\S+)", ln)
    d["card"] = m.group(1) if m else "-"
    m = re.search(r"area=(\S+)", ln)
    d["area"] = m.group(1) if m else "-"
    d["v"] = parse_value(ln)
    m = re.search(r"status=(\S+)", ln)
    d["status"] = m.group(1) if m else "-"
    m = re.search(r"choices=(\d+)", ln)
    d["choices"] = int(m.group(1)) if m else 0
    m = re.search(r"depth=(\d+)", ln)
    d["depth"] = int(m.group(1)) if m else 0
    for key, pat in (
        ("st", r"st=(-?\d+)"),
        ("bl", r"bl=(-?\d+)"),
        ("h", r"h=(-?\d+)"),
        ("en", r"en=(-?\d+)"),
        ("hand", r"hand=(-?\d+)"),
        ("ammo", r"ammo=(-?\d+)"),
        ("succ", r"succ=(-?\d+)"),
    ):
        m = re.search(pat, ln)
        d[key] = int(m.group(1)) if m else None
    d["baton"] = "[BATON]" in ln
    return d


def parse_ch(ln):
    d = {"raw": ln}
    m = re.search(r"t=(\d+)", ln)
    d["t"] = int(m.group(1)) if m else None
    m = re.search(r"me=(\d+)", ln)
    d["me"] = int(m.group(1)) if m else None
    m = re.search(r"act=(\S+)", ln)
    d["act"] = m.group(1) if m else "?"
    m = re.search(r"card=(\S+)", ln)
    d["card"] = m.group(1) if m else "-"
    m = re.search(r"score=(-?inf|nan|-?\d+\.\d+)", ln)
    if m:
        t = m.group(1)
        d["score"] = float("-inf") if t == "-inf" else (
            float("nan") if t == "nan" else float(t)
        )
    else:
        d["score"] = None
    m = re.search(r"note=(.*)$", ln)
    d["note"] = m.group(1).strip() if m else ""
    return d


# Accept both Debug snake_case and legacy PascalCase spellings.
MAIN_ACTS = {
    "pass", "Pass",
    "play_member_to_stage", "PlayMemberToStage",
    "use_ability", "UseAbility",
}
PASS_ACTS = {"pass", "Pass"}
PLAY_ACTS = {"play_member_to_stage", "PlayMemberToStage"}
USE_ACTS = {"use_ability", "UseAbility"}


def main():
    log_path, out_path = sys.argv[1], sys.argv[2]
    lines = open(log_path, encoding="utf-8", errors="replace").read().splitlines()

    blocks = []
    cur = []
    for ln in lines:
        if ln.startswith("V7M "):
            cur.append(parse_m(ln))
        elif ln.startswith("V7CH "):
            blocks.append((cur, parse_ch(ln)))
            cur = []
    if cur:
        blocks.append((cur, None))

    out = open(out_path, "w", encoding="utf-8")
    w = out.write
    w(f"log={log_path} decisions={len(blocks)}\n\n")

    stupid = Counter()
    main_blocks = 0

    for bi, (ms, ch) in enumerate(blocks):
        if not ch:
            continue
        if ch["act"] not in MAIN_ACTS:
            continue
        main_blocks += 1
        t, me = ch["t"], ch["me"]
        state = next((p for p in ms if p.get("st") is not None and p.get("st") != -1), None)
        w(f"=== #{bi} t={t} me={me} CHOSEN act={ch['act']} card={ch['card']} "
          f"score={ch['score']} note={ch['note']}\n")
        if state:
            w(f"    state st={state['st']} bl={state['bl']} h={state['h']} "
              f"en={state['en']} hand={state['hand']} ammo={state['ammo']} "
              f"succ={state['succ']}\n")

        scored = [p for p in ms if p["act"] in MAIN_ACTS]
        scored.sort(key=lambda p: -(p["v"] if p["v"] is not None else float("-inf")))

        for p in scored:
            flag = ""
            v = p["v"]
            is_pass = p["act"] in PASS_ACTS
            if v is None:
                flag = " [NO_VALUE]"
                stupid["missing_value"] += 1
            elif ch["act"] in PASS_ACTS and not is_pass and v > 0:
                flag = " *** STUPID: positive nonpass, chose Pass"
                stupid["pass_with_positive"] += 1
            elif ch["act"] not in PASS_ACTS and is_pass and ch["score"] is not None and v > ch["score"]:
                flag = " *** STUPID: Pass scored higher than chosen"
                stupid["pass_higher"] += 1
            elif (
                ch["act"] in PLAY_ACTS
                and p["act"] == ch["act"]
                and p["card"] == ch["card"]
                and v is not None
                and v < 0
                and ch["score"] is not None
                and ch["score"] < 0
            ):
                flag = " *** STUPID: chose negative deploy over Pass=0"
                stupid["negative_deploy"] += 1
            baton = " [BATON]" if p["baton"] else ""
            vs = f"{v:9.2f}" if v is not None else "      ???"
            w(f"    {vs}  {p['act']:22s} {p['card']:24s} area={p['area']:8s} "
              f"fc={p['fc']:3d} st={p['status']} ch={p['choices']} d={p['depth']}"
              f"{baton}{flag}\n")

        if ch["act"] in PASS_ACTS:
            nonpass = [p for p in scored if p["act"] not in PASS_ACTS and p["v"] is not None]
            if nonpass:
                mx = max(nonpass, key=lambda p: p["v"])
                w(f"    >> max nonpass = {mx['v']:.2f} ({mx['act']} {mx['card']})\n")
                if mx["v"] > 0:
                    w(f"    >> BUG? max nonpass > 0 but Pass chosen\n")
                    stupid["pass_bug_summary"] += 1
                elif mx["v"] == 0 and any(
                    "BATON" in (p.get("st") or "") or "baton" in (p.get("st") or "").lower()
                    or "PlayMember" in p["act"] for p in nonpass if p["v"] == 0
                ):
                    # 0–0 tie: Pass wins by index unless baton tie-break fires.
                    # Class A (see game_s42_report.md t3/t4/t6).
                    baton0 = any(p["v"] == 0 and "PlayMember" in p["act"] for p in nonpass)
                    if baton0:
                        w(f"    >> CLASS A: Pass over 0.00 member deploy (tie)\n")
                        stupid["pass_tie_over_zero_member"] += 1
            else:
                w(f"    >> no scored nonpass (all missing value?)\n")
        w("\n")

    w("\n==== SUMMARY ====\n")
    w(f"main decisions: {main_blocks}\n")
    for k, v in sorted(stupid.items()):
        w(f"  {k}: {v}\n")
    if not stupid:
        w("  (no stupid flags from this game)\n")
    out.close()
    print(f"wrote {out_path}")


if __name__ == "__main__":
    main()
