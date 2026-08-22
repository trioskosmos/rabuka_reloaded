import re, os, subprocess, sys

ROOT = r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded'
ENGINE = os.path.join(ROOT, 'engine')

KEEP = {'test_ai_vs_ai', 'display_desc', 'label_jp', 'opponent_has_performed'}

SUSPECTS = """resolver.rs: activating_card_name
types.rs: add_child
tracking.rs: add_prohibition_effect
modifiers.rs: add_revealed_card
zones.rs: all_heart_icons
util.rs: area_to_index
encoding.rs: as_usize
modifiers.rs: assign_card_instance_id
zones.rs: can_pay_energy
modifiers.rs: check_success_zone_draw_condition
mod.rs: cheer_blade_heart_count_mut
mod.rs: cheer_revealed_cards_first
mod.rs: cheer_revealed_cards_mut
zones.rs: clear_area
modifiers.rs: clear_card_instance_tracking
modifiers.rs: clear_turn_limit_tracking
card.rs: destination_str
card.rs: destination_zone
game_setup.rs: SKIP_display_desc
card.rs: dump_last_trace_placeholder""".strip()

# Build full target list from saved scan output
targets = []  # (relpath_from_engine_src, fn_name)
with open(os.path.join(ROOT, 'engine', 'dead_suspects.txt'), encoding='utf-8') as f:
    for line in f:
        line = line.strip()
        if not line:
            continue
        path_part, name = line.rsplit(': ', 1)
        if name in KEEP:
            continue
        rel = path_part.replace('engine/src/', '').replace('engine\\src\\', '').replace('\\', '/')
        targets.append((rel, name))


def strip_line_for_braces(line):
    # remove string literal contents and line comments so brace counts are safe
    out = []
    i = 0
    n = len(line)
    in_str = False
    while i < n:
        c = line[i]
        if in_str:
            if c == '\\':
                i += 2
                continue
            if c == '"':
                in_str = False
            i += 1
            continue
        if c == '"':
            in_str = True
            out.append(c)
            i += 1
            continue
        if c == '/' and i + 1 < n and line[i+1] == '/':
            break
        out.append(c)
        i += 1
    return ''.join(out)


def find_fn_span(lines, name):
    """Return (start_idx, end_idx) inclusive of attr/doc/fn lines, or None."""
    fn_re = re.compile(r'^(\s*)pub(?:\([^)]*\))?\s+(?:async\s+)?fn\s+' + re.escape(name) + r'\b')
    for idx, line in enumerate(lines):
        m = fn_re.match(line)
        if not m:
            continue
        indent = m.group(1)
        # walk back over doc comments and attributes
        start = idx
        while start > 0:
            prev = lines[start - 1]
            ps = prev.strip()
            if ps.startswith('///') or ps.startswith('#[') or ps.startswith('//'):
                # include plain comments only if they directly precede (contiguous)
                start -= 1
            else:
                break
        # skip cfg-gated items entirely (unsafe to judge liveness)
        gated = any('#[cfg(' in lines[j] for j in range(start, idx + 1))
        if gated:
            return None
        # find end: brace matching from first '{' at/after idx
        depth = 0
        seen_open = False
        j = idx
        while j < len(lines):
            for ch in strip_line_for_braces(lines[j]):
                if ch == '{':
                    depth += 1
                    seen_open = True
                elif ch == '}':
                    depth -= 1
            if seen_open and depth == 0:
                return (start, j)
            j += 1
        return None
    return None


def process_file(rel, names):
    path = os.path.join(ENGINE, 'src', rel)
    removed = []
    for name in sorted(names, key=len, reverse=True):
        with open(path, encoding='utf-8') as f:
            text = f.read()
        lines = text.split('\n')
        span = find_fn_span(lines, name)
        if span is None:
            continue
        s, e = span
        # also swallow one following blank line
        end = e + 1
        if end < len(lines) and lines[end].strip() == '':
            end += 1
        new_lines = lines[:s] + lines[end:]
        with open(path, 'w', encoding='utf-8', newline='') as f:
            f.write('\n'.join(new_lines))
        removed.append(name)
    return removed


def git_checkout(rel):
    p = 'engine/src/' + rel
    subprocess.run(['git', 'checkout', '--', p], cwd=ROOT, capture_output=True)


def compile_ok():
    r = subprocess.run(['cargo', 'check', '--quiet'], cwd=ENGINE, capture_output=True,
                       text=True, timeout=600)
    return r.returncode == 0, r.stderr


# group targets by file
from collections import defaultdict
by_file = defaultdict(list)
for rel, name in targets:
    by_file[rel].append(name)

skipped_gated = []
removed_all = []
failed_files = []

for rel in sorted(by_file):
    names = by_file[rel]
    # detect cfg-gated ones first without writing
    path = os.path.join(ENGINE, 'src', rel)
    with open(path, encoding='utf-8') as f:
        lines = f.read().split('\n')
    remaining = []
    for name in names:
        if find_fn_span(lines, name) is None:
            skipped_gated.append(f'{rel}: {name}')
        else:
            remaining.append(name)
    if not remaining:
        continue
    removed = process_file(rel, remaining)
    ok, err = compile_ok()
    if not ok:
        git_checkout(rel)
        failed_files.append((rel, removed, err[:400]))
    else:
        removed_all.extend((rel, n) for n in removed)

report = []
report.append(f'REMOVED {len(removed_all)}:')
for rel, n in removed_all:
    report.append(f'  {rel}: {n}')
report.append(f'SKIPPED (cfg-gated or not found) {len(skipped_gated)}:')
for s in skipped_gated:
    report.append(f'  {s}')
report.append(f'FAILED & RESTORED {len(failed_files)}:')
for rel, rem, err in failed_files:
    report.append(f'  {rel}: {rem}\n    ERR: {err}')
open(os.path.join(ENGINE, '_cut_report.txt'), 'w', encoding='utf-8').write('\n'.join(report))
print('done. removed:', len(removed_all), 'skipped:', len(skipped_gated), 'failed:', len(failed_files))
