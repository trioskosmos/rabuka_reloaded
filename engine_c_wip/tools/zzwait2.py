import hashlib, os, subprocess, sys, time

CR = r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine_c_wip'
P = os.path.join(CR, 'src', 'core', 'game_state_abilities.c')

def h():
    return hashlib.sha256(open(P, 'rb').read()).hexdigest()[:16]

def compiles():
    """Compile JUST this translation unit. A whole-suite run is far too slow to poll."""
    r = subprocess.run(
        ['cc', '-std=c11', '-O0', '-fsyntax-only',
         '-Iinclude', '-Isrc', '-Isrc/core/generated',
         'src/core/game_state_abilities.c'],
        cwd=CR, capture_output=True, text=True)
    if r.returncode == 0:
        return True, ''
    errs = [l for l in (r.stdout + r.stderr).splitlines() if ' error:' in l]
    return False, (errs[0][:100] if errs else 'nonzero rc, no error line')

last = h()
print('polling until game_state_abilities.c both STOPS CHANGING and COMPILES')
prev_state = None
for i in range(40):
    time.sleep(20)
    cur = h()
    ok, err = compiles()
    state = (cur, ok)
    if state != prev_state:
        print(f'  t={i*20:<5} hash={cur} compiles={ok} {err}')
        prev_state = state
    if ok and cur == last:
        print(f'\nSTABLE + COMPILING at t={i*20}s  hash={cur}')
        sys.exit(0)
    last = cur
print('\nTIMEOUT: still moving or still broken')
sys.exit(1)