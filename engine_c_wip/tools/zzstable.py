import hashlib, os, subprocess, sys, time
CR = r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine_c_wip'
FILES = ['src/core/game_state_abilities.c', 'src/turn/phase.c', 'src/engine.c',
         'src/web_server.c', 'src/core/modifiers.c', 'src/turn/triggers.c',
         'src/core/zones.c', 'src/core/card.c']

def snap():
    out = {}
    for f in FILES:
        p = os.path.join(CR, f)
        if os.path.exists(p):
            out[f] = hashlib.sha256(open(p, 'rb').read()).hexdigest()[:12]
    return out

def tree_builds():
    r = subprocess.run(['make', '-q', 'rb_engine_t_mechanics'], cwd=CR,
                       capture_output=True, text=True)
    return r.returncode == 0, r.returncode

print('polling for a stable, compiling tree (2 identical snapshots + a clean link)')
prev = None
for i in range(30):
    time.sleep(20)
    s = snap()
    ok, rc = tree_builds()
    moved = [f for f in s if prev and prev.get(f) != s[f]]
    print(f'  t={i*20:<5} link_ok={ok} moved={moved or "-"}')
    prev = s
    if ok and not moved and i > 0:
        print(f'\nSTABLE + LINKS at t={i*20}s')
        for f in FILES:
            print(f'   {f:<40} {s.get(f)}')
        sys.exit(0)
print('\nTIMEOUT')
sys.exit(1)