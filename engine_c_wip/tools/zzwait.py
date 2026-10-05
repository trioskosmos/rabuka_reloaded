import hashlib, os, sys, time
P = r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine_c_wip\src\core\game_state_abilities.c'
def h():
    b = open(P, 'rb').read()
    return hashlib.sha256(b).hexdigest(), len(b)
a = h()
print('t=0   ', a[0][:16], a[1])
for i in range(1, 9):
    time.sleep(30)
    b = h()
    changed = 'CHANGED' if b != a else 'stable '
    print(f't={i*30:<4}', b[0][:16], b[1], changed)
    a = b