#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Golden-file comparison harness for abilities.json.

Compares `cards/abilities.json` against a reference (golden) copy. Prints a
one-number summary (X/Y effects match exactly) plus per-path structural diffs
so every parser change is measured.

Usage:
    python test_against_golden.py              # compare committed abilities.json
    python test_against_golden.py --regenerate  # re-run extract+process, then compare

The golden file path defaults to the rabuka_reloaded-master reference copy; override
with the GOLDEN_ABILITIES env var.
"""
import argparse
import json
import os
import sys
from collections import Counter, defaultdict
from pathlib import Path

HERE = Path(__file__).parent
ROOT = HERE.parent
DEFAULT_GOLDEN = r'C:\Users\trios\Downloads\rabuka_reloaded-master (2)\rabuka_reloaded-master\cards\abilities.json'


def load(path):
    with open(path, encoding='utf-8') as f:
        return json.load(f)


def jnorm(v):
    """Canonical signature for equality (sorted keys, normalized)."""
    return json.dumps(v, sort_keys=True, ensure_ascii=False)


def diff_paths(a, b, prefix=''):
    """Yield (path, cur_val, ref_val) tuples where structure differs."""
    if isinstance(a, dict) and isinstance(b, dict):
        for k in sorted(set(a) | set(b)):
            p = f"{prefix}.{k}" if prefix else k
            if k not in a:
                yield (p, '<MISSING>', _sig(b[k]))
            elif k not in b:
                yield (p, _sig(a[k]), '<MISSING-in-ref>')
            else:
                yield from diff_paths(a[k], b[k], p)
    elif isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            yield (prefix + '[len]', len(a), len(b))
        else:
            for i, (x, y) in enumerate(zip(a, b)):
                yield from diff_paths(x, y, f"{prefix}[{i}]")
    else:
        if _sig(a) != _sig(b):
            yield (prefix, _sig(a), _sig(b))


def _sig(v):
    if isinstance(v, dict):
        return 'dict{' + ','.join(sorted(v.keys())) + '}'
    if isinstance(v, list):
        return 'list[' + str(len(v)) + ']'
    return v


def compare(cur, ref, out, verbose=False, max_examples=2):
    cur_by = {a['triggerless_text']: a for a in cur['unique_abilities']}
    ref_by = {a['triggerless_text']: a for a in ref['unique_abilities']}
    shared = set(cur_by) & set(ref_by)
    only_cur = set(cur_by) - set(ref_by)
    only_ref = set(ref_by) - set(cur_by)

    exact = 0
    differ = []
    for t in shared:
        if jnorm(cur_by[t].get('effect')) == jnorm(ref_by[t].get('effect')):
            exact += 1
        else:
            differ.append(t)

    out.append(f"shared abilities : {len(shared)}")
    out.append(f"exact effect match: {exact} ({100*exact/max(len(shared),1):.1f}%)")
    out.append(f"differ           : {len(differ)}")
    out.append(f"only in cur      : {len(only_cur)}")
    out.append(f"only in ref      : {len(only_ref)}")
    out.append(f"cost mismatches  : {sum(1 for t in shared if cur_by[t].get('cost') != ref_by[t].get('cost'))}")
    out.append("")

    # Per-path structural breakdown
    by_leaf = Counter()
    examples = defaultdict(list)
    for t in differ:
        for path, cv, rv in diff_paths(cur_by[t].get('effect') or {}, ref_by[t].get('effect') or {}):
            by_leaf[path] += 1
            if len(examples[path]) < max_examples:
                examples[path].append((t[:80], cv, rv))

    if by_leaf:
        out.append("=== structural differences by path ===")
        for path, n in by_leaf.most_common():
            out.append(f"\n[{n}x] {path}")
            for txt, cv, rv in examples[path]:
                out.append(f"   cur = {cv}")
                out.append(f"   ref = {rv}")

    if verbose:
        out.append("\n=== differing texts ===")
        for t in sorted(differ):
            out.append(" - " + t[:140])

    return len(differ)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--golden', default=os.environ.get('GOLDEN_ABILITIES', DEFAULT_GOLDEN))
    ap.add_argument('--current', default=str(ROOT / 'abilities.json'))
    ap.add_argument('--regenerate', action='store_true',
                    help='Re-run extract_card_abilities.py before comparing')
    ap.add_argument('-v', '--verbose', action='store_true')
    args = ap.parse_args()

    if args.regenerate:
        print("Regenerating abilities.json ...")
        import subprocess
        subprocess.check_call([sys.executable, str(HERE / 'extract_card_abilities.py')],
                              cwd=str(ROOT))

    cur = load(args.current)
    ref = load(args.golden)

    out = []
    n = compare(cur, ref, out, verbose=args.verbose)
    print('\n'.join(out))
    print()
    print(f"RESULT: {n} differences" + ("  [PASS]" if n == 0 else "  [FAIL]"))
    return 1 if n else 0


if __name__ == '__main__':
    sys.exit(main())
