#!/usr/bin/env python3
"""Port ONE Rust test file to C, compile it, run it. Built slowly, verified.

Usage: python tools/port_one.py <test.rs> [--run]
Output: tests/ported/<stem>.c (own main). --run links + executes.

Built idiom-first: each rule below names the test that forced it.
No guessing: unknown code becomes compiling TODO comments.
"""
import re, pathlib, os, sys, subprocess

def _root():
    cur = pathlib.Path(__file__).resolve()
    for _ in range(8):
        if (cur / "engine" / "tests").is_dir() and (cur / "engine_c").is_dir():
            return cur
        cur = cur.parent
    return pathlib.Path(os.getcwd()).resolve()

ROOT = _root()
EC = ROOT / "engine_c"
AREA = {"Left": "0", "LeftSide": "0", "Center": "1", "Right": "2", "RightSide": "2"}
# Rust Phase Display -> C rb_phase_name.
PHASE_DISP = {
    "RPS": "RPS", "Choose 1st": "Opening",
    "Mulligan (1st)": "Opening", "Mulligan (2nd)": "Opening",
    "Active": "Active", "Energy": "Energy", "Draw": "Draw", "Main": "Main",
    "LiveCardSet (1st)": "LiveCardSet", "LiveCardSet (2nd)": "LiveCardSet",
    "Perform (1st)": "Performance", "Perform (2nd)": "Performance",
    "Live Result": "Victory",
}
NO_INLINE = {"load_real_database", "init_test_logger", "start_test_watchdog",
             "answer_play_choice", "fill_decks", "setup_deck"}

def card_unescape(s):
    return re.sub(r'\\u\{([0-9a-fA-F]+)\}', lambda m: chr(int(m.group(1), 16)), s)

def split_top_commas(s):
    out, depth, cur, instr = [], 0, "", None
    for ch in s:
        if instr:
            cur += ch
            if ch == instr: instr = None
            continue
        if ch in '"\'':
            instr = ch; cur += ch
        elif ch in '([':
            depth += 1; cur += ch
        elif ch in ')]':
            depth -= 1; cur += ch
        elif ch == ',' and depth == 0:
            out.append(cur); cur = ''
        else:
            cur += ch
    if cur.strip(): out.append(cur)
    return [x.strip() for x in out]

def fn_bodies(text):
    """(helpers, tests, consts): helpers {name:(params,body)}, tests [(name,body)]."""
    tests = [m.group(1) for m in re.finditer(r'#\[test\]\s*fn\s+(\w+)', text)]
    helpers = {}
    for m in re.finditer(r'fn\s+(\w+)\s*\(([^)]*)\)\s*(?:->\s*[^{]*?)?\s*\{', text):
        name = m.group(1)
        if name in tests or name in NO_INLINE: continue
        params = [re.sub(r'^&?\s*mut\s*', '', p.split(':')[0]).strip()
                  for p in m.group(2).split(',') if p.strip()]
        d, i, start = 1, m.end(), m.end()
        while i < len(text) and d > 0:
            if text[i] == '{': d += 1
            elif text[i] == '}': d -= 1
            i += 1
        helpers[name] = (params, text[start:i-1])
    bodies = []
    for m in re.finditer(r'#\[test\]\s*fn\s+(\w+)\s*\([^)]*\)\s*\{', text):
        d, i, start = 1, m.end(), m.end()
        while i < len(text) and d > 0:
            if text[i] == '{': d += 1
            elif text[i] == '}': d -= 1
            i += 1
        bodies.append((m.group(1), text[start:i-1]))
    consts = dict(re.findall(r'const\s+(\w+)\s*[^=]*=\s*"([^"]+)"', text))
    return helpers, bodies, consts

def map_id(arg, consts):
    """game.id/new_id argument -> test_id(..) or None."""
    arg = arg.strip()
    m = re.match(r'^"([^"]+)"$', arg)
    if m: return f'test_id(&tg, "{card_unescape(m.group(1))}")'
    if re.match(r'^\w+$', arg) and arg in consts:
        cl = consts[arg]
        if cl.startswith('PL!') or cl.startswith('LL-'):
            return f'test_id(&tg, "{cl}")'
    return None

def map_board(e, declared):
    """Board reads -> C. Returns None when unresolvable."""
    e = e.strip()
    m = re.match(r'(?:game|tg)\.state\.player(\d+)\.stage\.stage\[(\d+)\]', e)
    if m: return f"tg.state.p[{int(m.group(1))-1}].stage[{m.group(2)}]"
    m = re.match(r'(?:game|tg)\.state\.player(\d+)\.energy_zone\.active_count\(\)', e)
    if m: return f"tg.state.p[{int(m.group(1))-1}].energy_active"
    m = re.match(r'(?:game|tg)\.state\.player(\d+)\.(\w+)\.cards\.len\(\)', e)
    if m:
        z = {"main_deck": "deck", "deck": "deck", "hand": "hand",
             "waitroom": "discard", "discard": "discard",
             "live_card_zone": "live", "live": "live",
             "success_live_card_zone": "success", "success": "success",
             "energy_zone": "energy", "energy": "energy"}.get(m.group(2))
        if z is None: return None
        return f"tg.state.p[{int(m.group(1))-1}].{z}.n"
    m = re.match(r'(?:game|tg)\.state\.player(\d+)\.(\w+)\.cards\.contains\(&(\w+)\)', e)
    if m:
        z = {"main_deck": "deck", "deck": "deck", "hand": "hand",
             "waitroom": "discard", "discard": "discard",
             "live_card_zone": "live", "live": "live",
             "success_live_card_zone": "success", "success": "success",
             "energy_zone": "energy", "energy": "energy"}.get(m.group(2), m.group(2))
        return f'test_zone_has_id(&tg, {int(m.group(1))-1}, "{z}", {m.group(3)})'
    m = re.match(r'game\.state\.mods\.get_(blade|score|cost)_modifier\((\w+)\)', e)
    if m: return f'test_get_{m.group(1)}_modifier(&tg, {m.group(2)})'
    m = re.match(r'game\.state\.mods\.(blade|score|cost)_modifiers\.get\s*\(\s*&?(\w+)\s*\)', e)
    if m: return f'test_get_{m.group(1)}_modifier(&tg, {m.group(2)})'
    if re.match(r'^\w+$', e): return e
    m = re.match(r'game\.state\.(current_phase|phase|turn)\b', e)
    if m:
        f = "phase" if m.group(1) == "current_phase" else m.group(1)
        return f"tg.state.{f}"
    return None

def map_expected(e, declared):
    e = e.strip()
    if e == "true": return "1"
    if e == "false": return "0"
    m = re.match(r'^(-?\d+)$', e)
    if m: return m.group(1)
    m = re.match(r'^Phase::(\w+)$', e)
    if m and m.group(1) in PHASE_MAP: return PHASE_MAP[m.group(1)]
    m = re.match(r'^(\w+)\s*([+-])\s*(\d+)$', e)
    if m and m.group(1) in declared:
        return f"{m.group(1)} {m.group(2)} {m.group(3)}"
    if re.match(r'^\w+$', e) and e in declared: return e
    return None


class St:
    def __init__(self, consts, func, helpers):
        self.out = []
        self.declared = set()
        self.consts = consts
        self.func = func
        self.helpers = helpers
        self.seen_tg = False
        self.real = False
    def decl(self, v): self.declared.add(v)
    def emit(self, s): self.out.append(s)
    def mark_real(self): self.real = True
    def emit_id(self, var, card):
        card = card_unescape(card)
        if var not in self.declared:
            self.out.append(f'    int {var} = test_id(&tg, "{card}");')
            self.decl(var)
        else:
            self.out.append(f'    {var} = test_id(&tg, "{card}");')

def inline_trivial(body, helpers, consts):
    """riko(game) where body is one game.id/new_id expr -> test_id text."""
    trivial = {}
    for name, (params, hb) in helpers.items():
        kept = [l.strip() for l in hb.split('\n')
                if l.strip() and not l.strip().startswith('//')
                and 'load_real_database' not in l]
        if len(kept) != 1: continue
        m = re.match(r'^(?:game|g)\.(id|new_id)\(\s*("[^"]+"|\w+)\s*\)\s*;?$', kept[0])
        if not m: continue
        r = map_id(m.group(2), consts)
        if r: trivial[name] = r
    if not trivial: return body
    return '\n'.join(
        re.sub(r'\b' + re.escape(n) + r'\s*\(\s*&?(?:mut\s+)?(?:game|g|tg)\s*\)', r, ln)
        if not ln.strip().startswith('//') else ln
        for ln in body.split('\n') for n, r in [next(iter(trivial.items()))]
    ) if False else _trivial_sub(body, trivial)

def _trivial_sub(body, trivial):
    out = []
    for ln in body.split('\n'):
        if not ln.strip().startswith('//'):
            for n, r in trivial.items():
                ln = re.sub(r'\b' + re.escape(n) + r'\s*\(\s*&?(?:mut\s+)?(?:game|g|tg)\s*\)', r, ln)
        out.append(ln)
    return '\n'.join(out)

def inline_helpers(body, helpers, consts):
    """Whole-line helper calls: `name(..);`, `let v = name(..);`."""
    out = []
    for line in body.split('\n'):
        s = line.strip()
        m = re.match(r'\s*(?:let\s+(?:mut\s+)?(\w+)\s*=\s*)?(\w+)\s*\(', s)
        if m and m.group(2) in helpers:
            name, var = m.group(2), m.group(1)
            params, hb = helpers[name]
            d, j = 1, m.end()
            while j < len(s) and d > 0:
                if s[j] in '([': d += 1
                elif s[j] in ')]': d -= 1
                j += 1
            args = split_top_commas(s[m.end():j-1])
            exp = hb
            for p, a in zip(params, args):
                exp = re.sub(r'\b' + re.escape(p) + r'\b',
                             re.sub(r'^&?\s*mut\s*', '', a).strip(), exp)
            exp = inline_helpers(exp, helpers, consts)
            exp = re.sub(r'\btg\.', 'game.', exp)
            ret, seg = None, exp.split('\n')
            for k in range(len(seg) - 1, -1, -1):
                t = seg[k].strip()
                if not t or t.startswith('//'): continue
                rm = re.match(r'^\((.+)\)\s*$', t)
                if rm:
                    ret = [x.strip() for x in split_top_commas(rm.group(1))]
                    seg = seg[:k]
                elif (not t.endswith(';') and not t.endswith('{')
                        and not t.endswith('}')
                        and not re.match(r'^(if|while|for|match|return|let\s)\b', t)):
                    ret = [t]
                    seg = seg[:k]
                break
            out.append(f"    // inlined {name}")
            out.extend(seg)
            if var:
                out.append(f"    int {var} = {ret[0]};" if ret else f"    int {var} = 0;")
        else:
            out.append(line)
    return '\n'.join(out)
