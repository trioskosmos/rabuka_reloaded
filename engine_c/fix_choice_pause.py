import re, os

# Fix all effects files - add rb_queue_pause_for_choice after rb_emit_choice
files = [
    'src/ability/effects/draw.c',
    'src/ability/effects/look.c',
    'src/ability/effects/misc.c',
    'src/ability/effects/move.c',
    'src/ability/effects/move_missing.c',
    'src/ability/effects/state.c'
]

for f in files:
    p = os.path.join('engine_c', f)
    src = open(p, encoding='utf-8').read()
    # After rb_emit_choice(...); add rb_queue_pause_for_choice(g, &g->queue.pending);
    src = re.sub(
        r'(rb_emit_choice\([^;]+;)\s*(?=\n)',
        r'\1\n    rb_queue_pause_for_choice(g, &g->queue.pending);',
        src
    )
    open(p, 'w', encoding='utf-8').write(src)
    print(f'Fixed {f}')

print('Done')