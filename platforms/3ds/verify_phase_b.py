"""Phase B equivalence check: compare setup.rs handler bodies against the
original Step::Setup match-arm bodies in the pre-refactor bin (git HEAD)."""

import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(ROOT))

sys.path.insert(0, ROOT)
import extract_setup as es  # reuses find_body_end + arm parser

TMP = (
    os.path.join(ROOT, "/tmp/orig_bin_phase_b.rs")
    if False
    else os.path.join(ROOT, "orig_bin_phase_b.rs")
)


def normalize(s):
    s = re.sub(r"//.*", "", s)
    s = re.sub(r"/\*.*?\*/", "", s, flags=re.S)
    # Protect string/char literals so whitespace inside them is not stripped.
    literals = []

    def stash(m):
        # Rust strips a `\` line continuation plus leading whitespace of the
        # next line inside string literals, so normalize that away.
        lit = re.sub(r"\\[ \t]*\r?\n[ \t\r\n]*", "", m.group(0))
        literals.append(lit)
        return "@@%d@@" % (len(literals) - 1)

    s = re.sub(r'"(?:\\.|[^"\\])*"', stash, s, flags=re.S)
    s = re.sub(r"'(?:\\.|[^'\\])*'", stash, s, flags=re.S)
    # Rustfmt drops trailing commas in call/array/struct literals (semantically
    # neutral); strip them on both sides, then drop all remaining whitespace.
    prev = None
    while prev != s:
        prev = s
        s = re.sub(r",\s*([\)\]}>])", r"\1", s)
    s = re.sub(r"\s+", "", s)
    for i, lit in enumerate(literals):
        s = s.replace("@@%d@@" % i, lit)
    return s.strip()


def extract_arms(src):
    marker = "Step::Setup(ref cards, ref decks, ref phase, ref dirty) => {"
    idx = src.index(marker)
    mj = src.index("match phase.clone() {", idx)
    mopen = src.index("{", mj)
    mend = es.find_body_end(src, mopen)
    starts = [
        m.start() + mopen + 20
        for m in re.finditer(r"(?m)^ {20}SetupPhase::", src[mopen:mend])
    ]
    arms = {}
    for s in starts:
        name_m = re.match(r"SetupPhase::(\w+)", src[s:])
        assert name_m is not None
        arm_name = name_m.group(1)
        arrow = src.index("=>", s)
        after = src[arrow + 2 :]
        k = 0
        while after[k] in " \t\n":
            k += 1
        expr_full_start = arrow + 2 + k
        body_open = src.index("{", expr_full_start)
        body_end = es.find_body_end(src, body_open)
        expr = src[expr_full_start:body_end]
        arms[arm_name] = expr
    return arms


def extract_handlers(src):
    out = {}
    for arm_name, (fn_name, _) in es.SIGS.items():
        pat = re.compile(r"fn\s+" + re.escape(fn_name) + r"\s*\(")
        m = pat.search(src)
        assert m, f"handler {fn_name} not found in setup.rs"
        brace = src.index("{", m.end())
        end = es.find_body_end(src, brace)
        body = src[brace + 1 : end - 1]
        out[arm_name] = body
    return out


def main():
    if not os.path.exists(TMP):
        blob = subprocess.run(
            ["git", "-C", REPO, "show", "HEAD:platforms/3ds/src/bin/rabuka_3ds.rs"],
            capture_output=True,
            check=True,
            text=True,
        ).stdout
        with open(TMP, "w", encoding="utf-8", newline="\n") as f:
            f.write(blob)

    orig = open(TMP, encoding="utf-8").read()
    setup_src = open(os.path.join(ROOT, "src", "setup.rs"), encoding="utf-8").read()

    arms = extract_arms(orig)
    handlers = extract_handlers(setup_src)

    missing = set(es.SIGS) - set(arms)
    if missing:
        print("MISSING arms in orig:", sorted(missing))
        return 1
    missing_h = set(es.SIGS) - set(handlers)
    if missing_h:
        print("MISSING handlers in setup.rs:", sorted(missing_h))
        return 1

    ok = True
    for arm_name in es.SIGS:
        m = normalize(arms[arm_name]) == normalize(handlers[arm_name])
        if not m:
            ok = False
            print(f"DIFF {arm_name}")
            a, b = arms[arm_name], handlers[arm_name]
            print("  ORIG:", normalize(a)[:120])
            print("  NEW :", normalize(b)[:120])
        else:
            print(f"OK   {arm_name}")
    print()
    print("ALL MATCH" if ok else "DIFFERENCES FOUND")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
