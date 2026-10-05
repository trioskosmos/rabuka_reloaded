"""Waits for the browser harvest to finish, then does everything else unattended.

Polls the browser's download folder for codes.txt (or the newest partial checkpoint if
the tab was closed early), copies it in, then runs the anonymous card phase and the
analysis. Nothing here needs a browser or a login.
"""

import importlib.util
import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
DOWNLOADS = Path.home() / "Downloads"
LOG = HERE / "auto_pickup.log"
PY = HERE / ".venv" / "Scripts" / "python.exe"

CODE_FILES = ["codes.txt"]
PARTIAL_RE = re.compile(r"(?:codes_partial_p|session_)?p?(\d*)_?codes\.txt$|^(session_)?codes\.txt$")


def log(msg):
    line = f"[{time.strftime('%H:%M:%S')}] {msg}"
    print(line, flush=True)
    with LOG.open("a", encoding="utf-8") as fh:
        fh.write(line + "\n")


def candidate_files():
    if not DOWNLOADS.exists():
        return []
    out = []
    for p in DOWNLOADS.glob("*.txt"):
        if "codes" not in p.name.lower():
            continue
        try:
            n = sum(1 for ln in p.read_text(encoding="utf-8", errors="replace").splitlines()
                    if ln.strip())
        except OSError:
            continue
        out.append((n, p.stat().st_mtime, p))
    return out


def best_candidate(min_codes):
    """Newest qualifying code file.

    A bare codes.txt is the final artefact the script writes on DONE, so it always
    qualifies. Partial checkpoints only qualify once they pass min_codes - otherwise an
    old checkpoint sitting in Downloads gets picked up and the watcher declares victory
    on a fraction of the corpus.
    """
    cands = candidate_files()
    if not cands:
        return None
    cands.sort(key=lambda t: (t[0], t[1]), reverse=True)
    for n, mtime, p in cands:
        if p.name == "codes.txt" or n >= min_codes:
            return n, mtime, p
    return None


def merge_codes(src):
    dest = HERE / "codes.txt"
    have = set()
    if dest.exists():
        have = {l.strip() for l in dest.read_text(encoding="utf-8").splitlines() if l.strip()}
    new = []
    for ln in src.read_text(encoding="utf-8", errors="replace").splitlines():
        c = ln.strip()
        if c and c not in have:
            have.add(c)
            new.append(c)
    with dest.open("a", encoding="utf-8") as fh:
        for c in new:
            fh.write(c + "\n")
    log(f"merged {len(new)} new codes from {src.name} -> {len(have)} total")
    return len(have)


def pick_meta():
    best = None
    for name in ("search_meta.jsonl", "search_meta_partial_p300.jsonl"):
        p = DOWNLOADS / name
        if p.exists():
            best = p
            break
    if not best:
        cands = sorted(DOWNLOADS.glob("search_meta*.jsonl"),
                       key=lambda p: p.stat().st_mtime, reverse=True)
        best = cands[0] if cands else None
    if not best:
        return 0
    dest = HERE / "search_meta.jsonl"
    seen = set()
    if dest.exists():
        for ln in dest.read_text(encoding="utf-8").splitlines():
            if ln.strip():
                try:
                    seen.add(json.loads(ln)["code"])
                except (json.JSONDecodeError, KeyError):
                    pass
    added = 0
    with dest.open("a", encoding="utf-8") as out:
        for ln in best.read_text(encoding="utf-8", errors="replace").splitlines():
            ln = ln.strip()
            if not ln:
                continue
            try:
                row = json.loads(ln)
            except json.JSONDecodeError:
                continue
            c = row.get("code")
            if c and c not in seen:
                seen.add(c)
                out.write(ln + "\n")
                added += 1
    log(f"merged {added} metadata rows from {best.name}")
    return added


def run(script, *args):
    cmd = [str(PY), str(HERE / script), *args]
    log("running: " + " ".join(cmd[1:]))
    p = subprocess.run(cmd, cwd=str(REPO))
    log(f"{script} exited {p.returncode}")
    return p.returncode


def main():
    min_codes = int(os.environ.get("MIN_CODES", "30000"))
    processed = 0
    log(f"waiting for the browser harvest (need codes.txt, or >={min_codes:,} codes)")
    while True:
        best = best_candidate(min_codes)
        if best and best[0] > processed:
            n = best[0]
            log(f"found {best[2].name} with {n:,} codes")
            total = merge_codes(best[2])
            pick_meta()
            processed = total
            log(f"starting the card phase for {total:,} codes")
            run("harvest_all.py", "--skip-codes", "--workers", "8")
            log("running the analysis")
            p = subprocess.run([str(PY), str(HERE / "analyze_combos.py"),
                                "--owners", str(HERE / "search_meta.jsonl")],
                               cwd=str(REPO))
            log(f"analysis exited {p.returncode}")
            log(f"ROUND COMPLETE for {total:,} codes - going back to waiting "
                f"for a larger harvest")
            time.sleep(60)
            continue
        time.sleep(30)


if __name__ == "__main__":
    sys.exit(main())
