"""Convert non-static GameState/TestGame locals to `static`, WITHOUT moving §N.

§N (test_harness_self.c) counts a `static TestGame|GameState` declaration whose
OWN function body carries no initialiser (test_game_new / rb_game_init /
rb_mods_init). So a blanket conversion of the 834 remaining locals would ADD
§N sites -- the very counter this task must not re-baseline. Only sites whose
enclosing function already initialises are converted; the rest are reported as
SKIPPED with the reason.

usage: python tools/scratch/h_static_fix.py [--write] [--only=stem,...]
"""
import hashlib
import os
import sys

ROOT = 'tests'
TYPES = (b'GameState', b'TestGame')
INIT_NAMES = (b'test_game_new', b'rb_game_init', b'rb_mods_init')


def fn_bodies(lines):
    """Yield (start, end_exclusive) index ranges of top-level function bodies.

    Mirrors the brace-walk §N's scanner uses: a `{` at depth 0 opens a body,
    and it closes when depth returns to 0. Comments are not blanked here --
    that only ever makes this converter MORE conservative, because a
    `test_game_new(` named inside a comment cannot rescue a site.
    """
    depth = 0
    start = None
    for i, line in enumerate(lines):
        o = line.count(b'{')
        c = line.count(b'}')
        if depth == 0 and o > 0:
            start = i
            depth += o - c
            if depth <= 0:
                yield (start, i + 1)
                depth = 0
                start = None
            continue
        if depth > 0:
            depth += o - c
            if depth <= 0:
                yield (start, i + 1)
                depth = 0
                start = None


def has_init(body):
    joined = b'\n'.join(body)
    for nm in INIT_NAMES:
        k = joined.find(nm)
        while k >= 0:
            r = k + len(nm)
            while r < len(joined) and joined[r] in (0x20, 0x09):
                r += 1
            if r < len(joined) and joined[r:r + 1] == b'(':
                return True
            k = joined.find(nm, k + 1)
    return False


def classify(path):
    before = open(path, 'rb').read()
    lines = before.split(b'\n')
    bodies = list(fn_bodies(lines))
    convert = [False] * len(lines)
    reasons = []
    for s, e in bodies:
        init = has_init(lines[s:e + 1])
        for i in range(s, min(e, len(lines))):
            line = lines[i]
            j = 0
            while j < len(line) and line[j] in (0x20, 0x09):
                j += 1
            body_l = line[j:]
            tlen = 0
            for t in TYPES:
                if body_l.startswith(t):
                    tlen = len(t)
                    break
            if not tlen:
                continue
            nxt = body_l[tlen:tlen + 1]
            if nxt not in (b' ', b'\t'):
                continue
            # A POINTER is 8 bytes, not a 1 MB frame: `TestGame *tg = &t_a;`
            # aliases a static board and must not be touched. (test_harness_self.c
            # §B counts this spelling -- it tests the byte immediately after the
            # type name, so `TestGame *tg` IS counted there -- which is a
            # detector imprecision. It is left alone here: the count may only
            # ever go DOWN, and this task does not own the number's meaning.)
            k = tlen
            while k < len(body_l) and body_l[k] in (0x20, 0x09):
                k += 1
            if body_l[k:k + 1] == b'*':
                continue
            if b'static' in line or b'extern' in line:
                continue
            if init:
                convert[i] = True
            else:
                reasons.append((i + 1, line.rstrip(b'\r').decode('utf-8', 'replace')[:90]))
    return before, lines, convert, reasons


def main():
    write = '--write' in sys.argv
    only = None
    for a in sys.argv:
        if a.startswith('--only='):
            only = set(a.split('=', 1)[1].split(','))
    grand = 0
    skipped = []
    for fn in sorted(os.listdir(ROOT)):
        if not fn.endswith('.c'):
            continue
        if only is not None and fn[:-2] not in only:
            continue
        path = os.path.join(ROOT, fn)
        before, lines, convert, reasons = classify(path)
        n = sum(1 for c in convert if c)
        if not n and not reasons:
            continue
        for ln, txt in reasons:
            skipped.append((fn, ln, txt))
        if not n:
            continue
        grand += n
        out = b'\n'.join(
            (lines[i][:len(lines[i]) - len(lines[i].lstrip(b' \t'))] + b'static '
             + lines[i].lstrip(b' \t')) if convert[i] else lines[i]
            for i in range(len(lines)))
        if not write:
            print('WOULD CONVERT %-46s %d' % (fn, n))
            continue
        if open(path, 'rb').read() != before:
            print('SKIP (moved mid-edit) %s' % fn)
            continue
        try:
            with open(path, 'wb') as f:
                f.write(out)
        except OSError as e:
            # A file another process holds open (OneDrive sync, a peer's
            # editor) must not abort the sweep of the other 30-odd files.
            print('SKIP (locked: %s) %s' % (e.strerror, fn))
            continue
        after = open(path, 'rb').read()
        ok = (after.count(b'\r') == before.count(b'\r')
              and after.count(b'\n') == before.count(b'\n')
              and sum(1 for b in after if b > 127)
              == sum(1 for b in before if b > 127)
              and len(after) == len(before) + n * 7)
        print('%-46s %3d  cr %d->%d lf %d->%d na %d->%d  %s' % (
            fn, n, before.count(b'\r'), after.count(b'\r'),
            before.count(b'\n'), after.count(b'\n'),
            sum(1 for b in before if b > 127),
            sum(1 for b in after if b > 127),
            'OK' if ok else '*** CHECK FAILED ***'))
    print('TOTAL converted = %d' % grand)
    print('SKIPPED (would move the §N counter up): %d' % len(skipped))
    for fn, ln, txt in skipped[:40]:
        print('   %s:%d  %s' % (fn, ln, txt))


if __name__ == '__main__':
    main()
