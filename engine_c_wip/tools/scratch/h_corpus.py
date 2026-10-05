"""Build and run EVERY tests/test_*.c, capturing both streams as RAW BYTES.

Why not `make run-all-tests`: it routes everything through the data-install
gate and its per-suite logs are written with shell redirection whose encoding
depends on the shell. This runner links each suite against the already-built
OBJ_LIB (the same command line `make t-run` uses, with GATE_DEP blanked exactly
as `make t-nogate` does), captures stdout/stderr byte-exact, and prints one
line per suite. A suite that fails to BUILD is reported as BUILD RED with a
non-zero case count of 0 -- never as green.

usage: python tools/scratch/h_corpus.py [--only=stem,...] [--jobs=N]
"""
import concurrent.futures as cf
import os
import subprocess
import sys
import threading

ROOT = os.path.abspath('.')
CC = 'gcc'
CFLAGS = '-std=c11 -O2 -Wall -Iinclude -Isrc -Isrc/core/generated'
LDFLAGS = '-Wl,--stack,33554432'
OBJ_LIB = [
    'src/ability/enums', 'src/ability/types', 'src/ability/vm',
    'src/ability/condition', 'src/ability/choice', 'src/ability/ability_queue',
    'src/ability/dynamic_count', 'src/ability/util', 'src/ability/cost',
    'src/ability/compound', 'src/ability/resolver',
    'src/ability/effects/executor', 'src/ability/effects/ability_effects',
    'src/ability/effects/custom', 'src/ability/effects/move',
    'src/ability/effects/move_missing', 'src/ability/effects/rules',
    'src/ability/effects/look', 'src/ability/effects/state',
    'src/ability/effects/ability', 'src/ability/effects/misc',
    'src/ability/effects/draw', 'src/ability/effects/score',
    'src/ability/log', 'src/ability/debug', 'src/ability/abilities_gen',
    'src/ability/describe', 'src/ability/choice_renderer',
    'src/game/deck_parser', 'src/game/deck_builder', 'src/game/game_setup',
    'src/core/card', 'src/core/data', 'src/core/alloc',
    'src/core/game_state_core', 'src/core/modifiers',
    'src/core/stats_pipeline', 'src/core/game_state_abilities',
    'src/core/tracking', 'src/core/zones', 'src/bytecode_data',
    'src/core/generated/bytecode_blob', 'src/core/generated/gen_data',
    'src/core/gen_data_rom', 'src/turn/phase', 'src/turn/live',
    'src/turn/triggers', 'src/engine', 'src/test_game', 'src/web_server',
]
OUT = 'zzcorpus'
_lock = threading.Lock()


def stems():
    return sorted(f[5:-2] for f in os.listdir('tests') if f.startswith('test_') and f.endswith('.c'))


def run(stem):
    exe = 'zzcorpus/rb_t_%s.exe' % stem
    src = 'tests/test_%s.c' % stem
    objs = [o + '.o' for o in OBJ_LIB]
    missing = [o for o in objs if not os.path.exists(o)]
    if missing:
        return stem, 'BUILD RED (missing objects: %s)' % missing[0], b'', b''
    r = subprocess.run([CC] + CFLAGS.split() + LDFLAGS.split() + ['-o', exe] + objs + [src],
                       capture_output=True)
    if r.returncode != 0:
        err = r.stderr
        return stem, 'BUILD RED (rc=%d)' % r.returncode, b'', err
    with open(OUT + '/' + stem + '.out', 'wb') as o, open(OUT + '/' + stem + '.err', 'wb') as e:
        rc = subprocess.call([exe], stdout=o, stderr=e, cwd=ROOT)
    return stem, rc, open(OUT + '/' + stem + '.out', 'rb').read(), \
        open(OUT + '/' + stem + '.err', 'rb').read()


def main():
    os.makedirs(OUT, exist_ok=True)
    only = None
    jobs = 4
    for a in sys.argv:
        if a.startswith('--only='):
            only = set(a.split('=', 1)[1].split(','))
        if a.startswith('--jobs='):
            jobs = int(a.split('=', 1)[1])
    todo = [s for s in stems() if only is None or s in only]
    print('=== %d suites ===' % len(todo))
    with cf.ThreadPoolExecutor(max_workers=jobs) as ex:
        for stem, rc, o, e in ex.map(run, todo):
            print('DONE %s' % stem, flush=True)


if __name__ == '__main__':
    main()
