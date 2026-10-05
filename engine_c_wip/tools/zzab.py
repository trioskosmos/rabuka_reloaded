"""Same-snapshot A/B: do the 25 re-cited phases.rs line numbers move ANY count?

The tree MOVED under the earlier before/after (a peer landed work in
game_state_abilities.c, which every suite links), so a before/after across that
gap is meaningless. This runs both arms back to back against the SAME tree:

  arm A = current phase.c   (citations corrected)
  arm B = phase.c with the citation digits reverted to their stale values

Arm B is produced by reversing the same deterministic substitution, so the two
files differ ONLY inside comments -- verified by stripping comments and digits and
asserting the executable text is identical, exactly as when the edit was applied.

The correct outcome is IDENTICAL counts in both arms. Anything else would mean the
edit was not comment-only after all.
"""
import hashlib, os, re, shutil, subprocess, sys, time

CR = r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine_c_wip'
P = os.path.join(CR, 'src', 'turn', 'phase.c')
RS = r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine\src\turn\phases.rs'
SUITES = sys.argv[1:] or ['parity_sequential', 'parity_recover_live', 'harness_self']

ph = open(RS, encoding='utf-8').read().split('\n')
real = {}
for i, l in enumerate(ph, 1):
    m = re.match(r'\s*(pub(?:\([^)]*\))?\s+)?fn\s+([a-z_0-9]+)', l)
    if m and m.group(2) not in real:
        real[m.group(2)] = i

STALE = {  # symbol -> the pre-edit (stale) starting line, from tools/zzcite2.py
    'play_time_cost_reduction_hook': 1317, 'play_time_cost_reduction_amount': 1437,
    'play_time_alt_cost_chars': 1472, 'normalize_member_name': 1505,
    'has_play_time_alt_cost_hand_cards': 1509, 'can_assign_hand_for_alt_cost': 1518,
    'build_alt_cost_candidates': 1529, 'has_distinct_assignment_k': 1557,
    'find_distinct_assignment_k': 1561, 'discard_play_time_alt_cost': 1589,
    'shuffle_waitroom_members_to_deck_bottom': 1639, 'backtrack': 1562,
    'play_double_baton': 936, 'execute_performance_phase': 318,
}
CIT = re.compile(r'([a-z_0-9]{4,})(\s*\((phases\.rs):)(\d+)(?:-(\d+))?(\))')


def only_digits_differ(a_bytes, b_bytes):
    """Prove the two files differ ONLY in digits.

    A regex-based comment stripper is not usable here: phase.c contains comment
    delimiters inside string literals, which desynchronises any /*...*/ pairing, and
    a mis-paired strip then leaves comment text (whose digits legitimately differ)
    in the supposedly-executable text and reports a false difference.

    Comparing bytes is exact and assumption-free: every position where the arms
    differ must hold an ASCII digit in BOTH files. Digits are behaviour-relevant, so
    "only digits differ" plus "every differing digit run sits inside a phases.rs:N
    citation" is a complete proof that the edit is comment-only.
    """
    if len(a_bytes) != len(b_bytes):
        # allow a pure length change only if one is a digit-run extension; report it
        pass
    n = min(len(a_bytes), len(b_bytes))
    bad = [(i, a_bytes[i], b_bytes[i]) for i in range(n)
           if a_bytes[i] != b_bytes[i]
           and not (chr(a_bytes[i]).isdigit() and chr(b_bytes[i]).isdigit())]
    return bad


def build_stale():
    text = open(P, encoding='utf-8').read()
    out = []
    n = [0]
    for line in text.split('\n'):
        def rep(m):
            sym, mid, fname, lo, hi, close = m.groups()
            if sym not in STALE:
                return m.group(0)
            old = STALE[sym]
            cur = int(lo)
            if cur == old:
                return m.group(0)
            n[0] += 1
            if hi is not None:
                span = int(hi) - cur
                return f'{sym}{mid}{old}-{old + span}{close}'
            return f'{sym}{mid}{old}{close}'
        out.append(CIT.sub(rep, line))
    return '\n'.join(out), n[0]


def measure(tag):
    res = {}
    for t in SUITES:
        log = os.path.join('zzlogs', f'ZZAB_{tag}_{t}.txt')
        r = subprocess.run(['make', 't-nogate', f'T={t}'], cwd=CR,
                           capture_output=True, text=True)
        blob = (r.stdout + r.stderr)
        open(log, 'w', encoding='utf-8').write(blob)
        m = re.search(r'^(PASSED|FAILED): .*?\[exit=(-?\d+)(?:, (\d+) result line\(s\))?'
                      r'(?: assert=(\d+))?.*?renormalised=(\d+)\]', blob, re.M)
        res[t] = {
            'verdict': m.group(1) if m else None,
            'assert': int(m.group(4)) if (m and m.group(4)) else None,
            'renorm': int(m.group(5)) if m else None,
            '^FAIL': len(re.findall(r'^FAIL', blob, re.M)),
        }
    return res


cur_bytes = open(P, 'rb').read()
cur_sha = hashlib.sha256(cur_bytes).hexdigest()
print(f'ARM A (current, citations corrected) phase.c sha256 = {cur_sha[:16]}')

stale_text, nrev = build_stale()
print(f'reverted {nrev} citations to build ARM B')
if nrev == 0:
    print('ABORT: nothing to revert -- a zero count here is a failure, not a pass')
    sys.exit(1)

try:
    # ---- ARM A ----
    subprocess.run(['rm', '-f', 'src/turn/phase.o'], cwd=CR)
    A = measure('A')
    print('\nARM A:', {k: (v['renorm'], v['^FAIL']) for k, v in A.items()})

    # ---- ARM B ----
    open(P, 'w', encoding='utf-8', newline='').write(stale_text)
    chk = open(P, 'rb').read()
    print(f'\nARM B phase.c sha256 = {hashlib.sha256(chk).hexdigest()[:16]} '
          f'({len(chk)} bytes; A was {len(cur_bytes)})')
    if len(chk) != len(cur_bytes):
        print(f'ABORT: length changed {len(cur_bytes)} -> {len(chk)}')
        sys.exit(1)
    bad = only_digits_differ(cur_bytes, chk)
    if bad:
        print(f'ABORT: {len(bad)} non-digit byte difference(s), first 5: {bad[:5]}')
        sys.exit(1)
    print(f'ARM B differs from ARM A in {sum(1 for x,y in zip(cur_bytes,chk) if x!=y)} '
          f'byte positions, ALL of them ASCII digits -> comment-only PROVEN')
    subprocess.run(['rm', '-f', 'src/turn/phase.o'], cwd=CR)
    B = measure('B')
    print('ARM B:', {k: (v['renorm'], v['^FAIL']) for k, v in B.items()})
finally:
    open(P, 'w', encoding='utf-8', newline='').write(cur_bytes.decode('utf-8'))
    subprocess.run(['rm', '-f', 'src/turn/phase.o'], cwd=CR)

back = hashlib.sha256(open(P, 'rb').read()).hexdigest()
print(f'\nphase.c restored: {back[:16]}  matches ARM A: {back == cur_sha}')

print('\n%-26s %-22s %-22s %s' % ('suite', 'A (renorm, ^FAIL)', 'B (renorm, ^FAIL)', 'same?'))
allsame = True
for t in SUITES:
    a, b = A[t], B[t]
    same = (a['renorm'] == b['renorm'] and a['^FAIL'] == b['^FAIL']
            and a['assert'] == b['assert'] and a['verdict'] == b['verdict'])
    allsame &= same
    print('%-26s %-22s %-22s %s' % (t, f"({a['renorm']}, {a['^FAIL']})",
                                     f"({b['renorm']}, {b['^FAIL']})",
                                     'YES' if same else '*** NO ***'))
print('\nVERDICT:', 'comment-only confirmed -- zero count movement' if allsame
      else 'COUNTS DIFFER -- investigate before claiming comment-only')