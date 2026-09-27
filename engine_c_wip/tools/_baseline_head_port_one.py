#!/usr/bin/env python3
"""Port Rust test files to C, compile them, run them. Built idiom-first:
each rule below names the test that forced it. No guessing: unknown code
becomes compiling TODO comments; a construct that would change control
flow untranspilably skips the whole test function and is counted.

Usage
  python tools/port_one.py <test.rs> [--run] [--install]
  python tools/port_one.py --audit [DIR ...] [--report FILE] [--json FILE]
  python tools/port_one.py --batch <DIR> [--out DIR] [--report FILE]

Output: tests/ported/<stem>.c (own main).  `--run` links + executes in an
ISOLATED build root (never bare `make`: 14 agents share src/**/*.o).
`--install` additionally writes the same generated file to
tests/test_xpile_<stem>.c so `make t T=xpile_<stem>` can build it; that
file is generated, never hand-edited.
"""
import re, pathlib, os, sys, json, subprocess, shutil, argparse

# ──────────────────────────────────────────────────────────────────────
# Root discovery.  The C engine directory has been renamed more than once
# (engine_c -> engine_c_wip), so detect it by CONTENT, never by name.
# ──────────────────────────────────────────────────────────────────────
def _find_engine_dir():
    cur = pathlib.Path(__file__).resolve()
    for _ in range(8):
        for name in ("engine_c", "engine_c_wip", "engine_c_port", "engine"):
            d = cur / name
            if (d / "include" / "test_game.h").is_file() and (d / "tests").is_dir():
                return d
        cur = cur.parent
    raise SystemExit("port_one: cannot locate the C engine dir (no include/test_game.h)")

EC = _find_engine_dir()
RUST_TESTS = EC.parent / "engine" / "tests"
OUTDIR = EC / "tests" / "ported"
ISOBUILD = pathlib.Path(os.environ.get(
    "ISOLATED_BUILD_ROOT",
    os.path.expandvars(r"%LOCALAPPDATA%\Temp\kilo\rb_isobuild"))) / "xpile"

AREA = {"Left": "0", "LeftSide": "0", "Center": "1", "Right": "2", "RightSide": "2"}
# Rust Phase Display -> C rb_phase_name.  (display names, for traces)
PHASE_DISP = {
    "RPS": "RPS", "Choose 1st": "Opening",
    "Mulligan (1st)": "Opening", "Mulligan (2nd)": "Opening",
    "Active": "Active", "Energy": "Energy", "Draw": "Draw", "Main": "Main",
    "LiveCardSet (1st)": "LiveCardSet", "LiveCardSet (2nd)": "LiveCardSet",
    "Perform (1st)": "Performance", "Perform (2nd)": "Performance",
    "Live Result": "Victory",
}
# engine/src/core/types.rs Phase -> RbPhase
PHASE_MAP = {
    "Rps": "RB_PHASE_RPS", "Opening": "RB_PHASE_OPENING",
    "Active": "RB_PHASE_ACTIVE", "Energy": "RB_PHASE_ENERGY",
    "Draw": "RB_PHASE_DRAW", "Main": "RB_PHASE_MAIN",
    "LiveCardSetFirstAttacker": "RB_PHASE_LIVE_SET",
    "LiveCardSetSecondAttacker": "RB_PHASE_LIVE_SET_SECOND",
    "FirstAttackerPerformance": "RB_PHASE_PERFORMANCE",
    "SecondAttackerPerformance": "RB_PHASE_PERFORMANCE_SECOND",
    "LiveVictoryDetermination": "RB_PHASE_VICTORY",
    "GameEnd": "RB_PHASE_DONE", "MulliganFirst": "RB_PHASE_MULLIGAN_FIRST",
    "MulliganSecond": "RB_PHASE_MULLIGAN_SECOND",
}
# engine/src/core/card.rs HeartColor ordinal == RbHeartColor ordinal.
HEART_MAP = {f"Heart{i:02d}": str(i) for i in range(0, 12)}
HEART_MAP.update({"All": "7", "Draw": "8", "Score": "9", "Any": "10", "BAll": "7"})
# game/src/core/card.rs CardOrientation
ORIENT_MAP = {"None": "0", "Active": "1", "Wait": "2"}
# Rust zone field -> C RbPlayer bag field
ZONE = {
    "main_deck": "deck", "deck": "deck", "hand": "hand",
    "waitroom": "discard", "discard": "discard",
    "live_card_zone": "live", "live": "live",
    "success_live_card_zone": "success", "success": "success",
    "energy_zone": "energy", "energy": "energy",
    "energy_deck": "energy_deck",
}
# Rust mods map -> C RbMods array field (for presence probes / Option forms)
MODS_FIELD = {
    "blade": "blade", "score": "score", "cost": "cost",
    "heart": "heart", "need_heart": "need_heart",
    "orientation": "orientation",
}
# Rust mods map -> the existing test_get_*_modifier shim
MODS_GETTER = {"blade": "test_get_blade_modifier",
               "score": "test_get_score_modifier",
               "cost": "test_get_cost_modifier"}
# Choice enum variant -> RbChoiceKind
CHOICE_KIND = {
    "SelectCard": "RB_CHOICE_SELECT_CARD",
    "SelectTarget": "RB_CHOICE_SELECT_TARGET",
    "SelectHeartColor": "RB_CHOICE_SELECT_HEART_COLOR",
    "SelectNumber": "RB_CHOICE_SELECT_NUMBER",
    "SelectPosition": "RB_CHOICE_SELECT_POSITION",
    "SelectAutoAbility": "RB_CHOICE_SELECT_AUTO_ABILITY",
    "SelectString": "RB_CHOICE_SELECT_STRING",
    "NoChoice": "RB_CHOICE_NONE",
}
NO_INLINE = {"load_real_database", "init_test_logger", "start_test_watchdog",
             "answer_play_choice", "fill_decks", "setup_deck"}
# Helpers that get a generated C body instead of being dropped.
RUNTIME_HELPER_CALLS = {
    "fill_decks": "pz_fill_decks",
    "fill_energy_deck": "pz_fill_energy_deck",
    "put_on_deck_top": "pz_put_on_deck_top",
    "drain_auto_choices": "pz_drain",
    "drain_choices": "pz_drain",
}


class Skip(Exception):
    """Raised when a construct cannot be represented without lying."""

    def __init__(self, reason):
        super().__init__(reason)
        self.reason = reason


# ──────────────────────────────────────────────────────────────────────
# Lexical helpers
# ──────────────────────────────────────────────────────────────────────
def card_unescape(s):
    return re.sub(r'\\u\{([0-9a-fA-F]+)\}', lambda m: chr(int(m.group(1), 16)), s)


def strip_comment(line):
    out, instr, i = [], None, 0
    while i < len(line):
        ch = line[i]
        if instr:
            out.append(ch)
            if ch == "\\" and i + 1 < len(line):
                out.append(line[i + 1]); i += 2; continue
            if ch == instr:
                instr = None
            i += 1; continue
        if ch in "\"'":
            instr = ch; out.append(ch); i += 1; continue
        if ch == "/" and i + 1 < len(line) and line[i + 1] == "/":
            break
        out.append(ch); i += 1
    return "".join(out)


def depth_of(s):
    d, instr = 0, None
    for ch in s:
        if instr:
            if ch == instr:
                instr = None
            continue
        if ch in "\"'":
            instr = ch
        elif ch in "([{":
            d += 1
        elif ch in ")]}":
            d -= 1
    return d


def split_top(s, seps):
    """Split at top-level occurrences of any char in `seps`."""
    out, cur, d, instr = [], "", 0, None
    for ch in s:
        if instr:
            cur += ch
            if ch == instr:
                instr = None
            continue
        if ch in "\"'":
            instr = ch; cur += ch
        elif ch in "([{":
            d += 1; cur += ch
        elif ch in ")]}":
            d -= 1; cur += ch
        elif d == 0 and ch in seps:
            out.append(cur); cur = ""
        else:
            cur += ch
    if cur.strip():
        out.append(cur)
    return [x.strip() for x in out]


def split_top_commas(s):
    return split_top(s, ",")


def split_top_ops(s, ops):
    """Split at the LAST top-level operator char (left-assoc arithmetic)."""
    d, instr, hit = 0, None, None
    for i, ch in enumerate(s):
        if instr:
            if ch == instr:
                instr = None
            continue
        if ch in "\"'":
            instr = ch
        elif ch in "([{":
            d += 1
        elif ch in ")]}":
            d -= 1
        elif d == 0 and ch in ops:
            if ch == "-" and (i == 0 or s[i - 1] in "([{,"):
                continue
            if ch == ">" and i + 1 < len(s) and s[i + 1] == ">":
                continue
            hit = i
    if hit is None:
        return None
    if s[hit] == "-" and s[hit:hit + 2] == "->":
        return None
    return s[:hit].strip(), s[hit], s[hit + 1:].strip()


def split_top_cmp(s):
    return split_top_ops(s, "=<>!")


def find_top(s, needle):
    """Index of a top-level substring, or -1."""
    d, instr, i = 0, None, 0
    n = len(needle)
    while i < len(s):
        ch = s[i]
        if instr:
            if ch == instr:
                instr = None
            i += 1; continue
        if ch in "\"'":
            instr = ch; i += 1; continue
        if ch in "([{":
            d += 1
        elif ch in ")]}":
            d -= 1
        elif d == 0 and s[i:i + n] == needle:
            return i
        i += 1
    return -1


def split_call_args(s):
    """s = 'name(a, b, c)' -> ('name', ['a','b','c']) honouring nested brackets."""
    m = re.match(r'^\s*([A-Za-z_][\w:.]*)\s*\(', s)
    if not m:
        return None, None
    name = m.group(1)
    rest = s[m.end() - 1:]
    d, instr, i = 0, None, 0
    while i < len(rest):
        ch = rest[i]
        if instr:
            if ch == instr:
                instr = None
            i += 1; continue
        if ch in "\"'":
            instr = ch
        elif ch in "([{":
            d += 1
        elif ch in ")]}":
            d -= 1
            if d == 0:
                return name, split_top_commas(rest[1:i])
        i += 1
    return name, []


def match_paren(s, start):
    """Index just past the ')' matching the '(' at `start`."""
    d, instr, i = 0, None, start
    while i < len(s):
        ch = s[i]
        if instr:
            if ch == instr:
                instr = None
        elif ch in "\"'":
            instr = ch
        elif ch == "(":
            d += 1
        elif ch == ")":
            d -= 1
            if d == 0:
                return i + 1
        i += 1
    return -1


def logical_lines(body):
    """Join continuation lines into one logical statement per entry.

    A line ENDS a statement when brackets balance and it does not end with
    a continuation token.  A line ending in '{' or '}' always ends one.
    """
    out, cur = [], ""
    for raw in body.split("\n"):
        s = strip_comment(raw).strip()
        if not s and not cur:
            continue
        if not s:
            continue
        cur = (cur + " " + s).strip() if cur else s
        if cur.endswith("{") or cur.endswith("}") or cur in ("};", "})", ");"):
            out.append(cur); cur = ""; continue
        if depth_of(cur) > 0:
            continue
        if cur.endswith((".", "+", "&&", "||", "==", "!=", "=>", ",", "<", ">", "?")):
            continue
        out.append(cur); cur = ""
    if cur:
        out.append(cur)
    return out


# ──────────────────────────────────────────────────────────────────────
# Rust file parsing
# ──────────────────────────────────────────────────────────────────────
def match_block(text, start):
    """start = index just past an opening '{'. Returns (body, end_index)."""
    d, i, instr = 1, start, None
    while i < len(text):
        ch = text[i]
        if instr:
            if ch == "\\":
                i += 2; continue
            if ch == instr:
                instr = None
        elif ch in "\"'":
            instr = ch
        elif ch == "{":
            d += 1
        elif ch == "}":
            d -= 1
            if d == 0:
                return text[start:i], i + 1
        i += 1
    return text[start:], len(text)


def fn_bodies(text):
    """(helpers, tests, consts) from a Rust source file."""
    test_names = {m.group(1) for m in re.finditer(r'#\[test\]\s*(?:#\[[^\]]*\]\s*)*fn\s+(\w+)', text)}
    helpers = {}
    for m in re.finditer(r'(?:^|\n)\s*(?:pub\s+)?(?:pub\(crate\)\s+)?fn\s+(\w+)\s*\(([^)]*)\)', text):
        name = m.group(1)
        if name in test_names:
            continue
        params = [re.sub(r'^&?\s*(?:mut\s+)?', '', p.split(':')[0]).strip()
                  for p in split_top_commas(m.group(2)) if p.strip()]
        brace = text.find("{", m.end())
        if brace < 0 or text[max(0, brace - 40):brace].rstrip().endswith(")"):
            continue
        body, _ = match_block(text, brace + 1)
        helpers[name] = (params, body)
    bodies = []
    for m in re.finditer(r'#\[test\]\s*(?:#\[[^\]]*\]\s*)*fn\s+(\w+)\s*\([^)]*\)', text):
        brace = text.find("{", m.end())
        if brace < 0:
            continue
        # skip `fn f() -> T {` where the return type itself has braces
        ret = text[m.end():brace]
        if "{" in ret or "}" in ret:
            continue
        body, _ = match_block(text, brace + 1)
        bodies.append((m.group(1), body))
    consts = {}
    for m in re.finditer(r'const\s+(\w+)\s*(?::\s*&?\w+\s*)?=\s*"((?:[^"\\]|\\.)*)"', text):
        consts[m.group(1)] = m.group(2)
    return helpers, bodies, consts


def join_method_continuations(text):
    """Rejoin Rust method chains split across lines (backlog item 1).

    `game.state.player1.energy_deck` / `.cards` / `.push(filler);` must
    become ONE logical line so the single-line rules can fire.  Join with
    NO separator.
    """
    lines = text.split("\n")
    out, buf = [], ""
    for raw in lines:
        s = raw.rstrip()
        st = s.strip()
        if not st:
            if buf:
                out.append(buf); buf = ""
            out.append(s); continue
        if st.startswith("//") or st.startswith("/*") or st.startswith("*"):
            if buf:
                out.append(buf); buf = ""
            out.append(s); continue
        if not buf:
            buf = s
        else:
            buf = buf + st
        # a joined fragment is complete when brackets balance (or the
        # fragment opened a block) and it is not still mid-chain
        if (depth_of(buf) <= 0 or buf.rstrip().endswith(("{", "}"))) \
                and not buf.rstrip().endswith("."):
            out.append(buf); buf = ""
    if buf:
        out.append(buf)
    return "\n".join(out)


# ──────────────────────────────────────────────────────────────────────
# Helpers: trivial id inlining + whole-line helper inlining
# ──────────────────────────────────────────────────────────────────────
def map_id(arg, consts, gvar="game"):
    """game.id/new_id argument -> test_id(..) / test_new_id(..), or None."""
    arg = arg.strip().rstrip(";")
    m = re.match(r'^"((?:[^"\\]|\\.)*)"$', arg)
    if m:
        no = card_unescape(m.group(1))
        if not is_card_no(no):
            return None
        return f'test_id(&{gvar}, "{no}")'
    if re.match(r'^\w+$', arg) and arg in consts:
        no = card_unescape(consts[arg])
        if is_card_no(no):
            return f'test_id(&{gvar}, "{no}")'
    return None


def is_card_no(s):
    return bool(re.match(r'^(PL!|LL[-!])', s)) and "-" in s


def inline_trivial(body, helpers, consts):
    """`riko(game)` / `nonfiction(&game)` where the body is one
    game.id/new_id expression -> substitute the test_id text directly."""
    trivial = {}
    for name, (params, hb) in helpers.items():
        kept = [l.strip() for l in hb.split('\n')
                if l.strip() and not l.strip().startswith('//')
                and 'load_real_database' not in l]
        if len(kept) != 1:
            continue
        m = re.match(r'^(?:game|g)\.(id|new_id|id_ref)\(\s*("[^"]+"|\w+)\s*\)\s*;?$', kept[0])
        if not m:
            continue
        fn = {"id": "test_id", "new_id": "test_new_id", "id_ref": "test_id"}[m.group(1)]
        r = map_id(m.group(2), consts)
        if r:
            r = r.replace("test_id(", fn + "(", 1)
            trivial[name] = r
    if not trivial:
        return body
    for n, r in trivial.items():
        body = re.sub(r'\b' + re.escape(n) + r'\s*\(\s*&?\s*(?:mut\s+)?(?:game|g)\s*\)',
                      lambda m: r, body)
    return body


def inline_helpers(body, helpers, consts, depth=0):
    """Whole-line helper calls: `name(..);`, `let v = name(..);`."""
    if depth > 4:
        return body
    out = []
    for line in body.split('\n'):
        s = line.strip()
        m = re.match(r'\s*(?:let\s+(?:mut\s+)?(\w+)\s*=\s*)?(\w+)\s*\(', s)
        if m and m.group(2) in helpers and m.group(2) not in NO_INLINE:
            name, var = m.group(2), m.group(1)
            params, hb = helpers[name]
            end = match_paren(s, s.index('(', m.end(2) - 1))
            if end < 0:
                out.append(line); continue
            args = split_top_commas(s[s.index('(', m.end(2) - 1) + 1:end - 1])
            exp = hb
            for p, a in zip(params, args):
                exp = re.sub(r'\b' + re.escape(p) + r'\b',
                             re.sub(r'^&?\s*(?:mut\s+)?', '', a).strip(), exp)
            exp = inline_trivial(exp, helpers, consts)
            exp = inline_helpers(exp, helpers, consts, depth + 1)
            exp = re.sub(r'\btg\.', 'game.', exp)
            exp = exp.replace("&mut game", "&mut game")
            out.append(f"    // inlined {name}")
            out.extend(logical_lines(exp))
            if var:
                out.append(f"    // let {var} = {name}(...)")
        else:
            out.append(line)
    return '\n'.join(out)


# ──────────────────────────────────────────────────────────────────────
# Translation context
# ──────────────────────────────────────────────────────────────────────
class Ctx:
    def __init__(self, helpers, consts, fname, gvar="game"):
        self.helpers = helpers
        self.consts = consts
        self.fname = fname
        self.g = gvar
        self.declared = set()
        self.lines = []
        self.uses = set()
        self.n_check = 0
        self.n_todo = 0
        self.loop_depth = 0
        self.have_game = False

    def use(self, key):
        self.uses.add(key)

    def ind(self):
        return "    " * (1 + self.loop_depth)

    def fresh(self, name):
        if name not in self.declared:
            self.declared.add(name)
            return name
        k = 2
        while f"{name}_{k}" in self.declared:
            k += 1
        self.declared.add(f"{name}_{k}")
        return f"{name}_{k}"

    def emit(self, s):
        self.lines.append(s)

    def todo(self, src):
        self.n_todo += 1
        self.emit(f"    // TODO: {src}")


# ──────────────────────────────────────────────────────────────────────
# Expression mapping
# ──────────────────────────────────────────────────────────────────────
def c_str(lit):
    """Rust string literal -> C string literal (unicode-unescaped)."""
    body = lit.strip()[1:-1]
    body = card_unescape(body)
    out = []
    for ch in body:
        if ch == '"':
            out.append('\\"')
        elif ch == "\\":
            out.append("\\\\")
        elif ch == "\n":
            out.append("\\n")
        elif ch == "\t":
            out.append("\\t")
        elif ord(ch) < 0x20:
            out.append("\\%03o" % ord(ch))
        else:
            out.append(ch)
    return '"' + "".join(out) + '"'


def is_str_lit(e):
    return len(e) >= 2 and e[0] == '"' and e[-1] == '"' and '"' not in e[1:-1].replace('\\"', "")


def strip_opt_tail(e):
    """Drop the `.copied()` / `.cloned()` / `.as_deref()` tail Rust puts on
    Option-returning expressions, so the base mapper's `$`-anchored rules
    still fire."""
    prev = None
    while prev != e:
        prev = e
        e = re.sub(r'\.(?:copied|cloned|as_deref|as_ref)\(\)$', '', e).strip()
    return e


def map_arena(e, ctx):
    """`game.state.playerN.<zone>.cards...` -> C.

    Returns (kind, payload).  kind is one of:
      'bag_idx'  -> (player_c, bag, index_expr)
      'bag_len'  -> (player_c, bag)
      'stage_idx'-> (player_c, index_expr)
    """
    e = strip_opt_tail(e)
    m = re.match(r'^(?:game|g)\.state\.player([12])\.(\w+)\.cards\.(.+)$', e)
    if m:
        pl, zf, rest = int(m.group(1)) - 1, m.group(2), m.group(3)
        z = ZONE.get(zf)
        if z is None:
            return None
        mm = re.match(r'^len\(\)$', rest)
        if mm:
            return ('bag_len', (pl, z))
        mm = re.match(r'^is_empty\(\)$', rest)
        if mm:
            return ('bag_is_empty', (pl, z))
        mm = re.match(r'^contains\(\s*&?\s*(.+?)\s*\)$', rest)
        if mm:
            return ('bag_contains', (pl, z, mm.group(1)))
        mm = re.match(r'^iter\(\)\.any\(\s*\|\s*[&*]*\s*(\w+)\s*\|\s*(.+?)\s*\)$', rest)
        if mm:
            return ('bag_contains', (pl, z, mm.group(2)))
        mm = re.match(r'^iter\(\)\.position\(\s*\|\s*&?\s*(\w+)\s*\|\s*(.+?)\s*\)$', rest)
        if mm:
            return ('bag_position', (pl, z, mm.group(2)))
        mm = re.match(r'^first\(\)$', rest)
        if mm:
            return ('bag_first', (pl, z))
        return None
    m = re.match(r'^(?:game|g)\.state\.player([12])\.stage\.(.+)$', e)
    if m:
        pl, rest = int(m.group(1)) - 1, m.group(2)
        mm = re.match(r'^stage\[(\d+)\]$', rest)
        if mm:
            return ('stage_idx', (pl, mm.group(1)))
        mm = re.match(r'^stage\.is_waited\(\)$', rest)
        if mm:
            ctx.use("stage_waited")
            return ('raw', f'(game.state.p[{pl}].stage_wait[0] || game.state.p[{pl}].stage_wait[1] || game.state.p[{pl}].stage_wait[2])')
        mm = re.match(r'^is_waited\(\)$', rest)
        if mm:
            ctx.use("stage_waited")
            return ('raw', f'(game.state.p[{pl}].stage_wait[0] || game.state.p[{pl}].stage_wait[1] || game.state.p[{pl}].stage_wait[2])')
        mm = re.match(r'^stage\.is_waited_at\((\w+::\w+)\)$', rest)
        if mm:
            return ('raw', f'game.state.p[{pl}].stage_wait[{AREA.get(mm.group(1).split("::")[-1], "0")}]')
        return None
    return None


def map_mods_opt(e, ctx):
    """`game.state.mods.<map>.get(&id)` -> (probe, bind, none_ok).

    Returns (probe_c, bind_c_or_None, kind) with kind in
    {'int', 'present', 'orientation'}.
    """
    e = e.strip()
    e = strip_opt_tail(e)
    deref = lambda a: re.sub(r'^\s*&\s*', '', a.strip())
    m = re.match(r'^(?:game|g)\.state\.mods\.(orientation)_(?:modifiers|set_modifiers)'
                 r'\.get\(\s*&?\s*(.+?)\s*\)', e)
    if m:
        arg = deref(m.group(2))
        return (f'(game.state.mods.orientation[{arg}] != 0)',
                f'game.state.mods.orientation[{arg}]', 'orientation')
    m = re.match(r'^(?:game|g)\.state\.mods\.get_orientation_modifier\(\s*(.+?)\s*\)', e)
    if m:
        arg = deref(m.group(1))
        return (f'(rb_mods_get_orientation(&game.state.mods, {arg}) != NULL)',
                f'game.state.mods.orientation[{arg}]', 'orientation')
    m = re.match(r'^(?:game|g)\.state\.mods\.(blade|score|cost)_modifiers'
                 r'\.get\(\s*&?\s*(.+?)\s*\)', e)
    if m:
        kind, arg = m.group(1), deref(m.group(2))
        fld = MODS_FIELD[kind]
        probe = f'((game.state.mods.{fld}[{arg}].set != 0) || (game.state.mods.{fld}[{arg}].add != 0))'
        return (probe, f'test_get_{kind}_modifier(&game, {arg})', 'int')
    m = re.match(r'^(?:game|g)\.state\.mods\.(heart|need_heart)_modifiers'
                 r'\.get\(\s*&?\s*(.+?)\s*\)', e)
    if m:
        fld, arg = m.group(1), deref(m.group(2))
        probe = (f'((game.state.mods.{fld}[{arg}][0].set != 0) || (game.state.mods.{fld}[{arg}][0].add != 0)'
                 f' || (game.state.mods.{fld}[{arg}][1].set != 0) || (game.state.mods.{fld}[{arg}][1].add != 0)'
                 f' || (game.state.mods.{fld}[{arg}][2].set != 0) || (game.state.mods.{fld}[{arg}][2].add != 0)'
                 f' || (game.state.mods.{fld}[{arg}][3].set != 0) || (game.state.mods.{fld}[{arg}][3].add != 0)'
                 f' || (game.state.mods.{fld}[{arg}][4].set != 0) || (game.state.mods.{fld}[{arg}][4].add != 0)'
                 f' || (game.state.mods.{fld}[{arg}][5].set != 0) || (game.state.mods.{fld}[{arg}][5].add != 0)'
                 f' || (game.state.mods.{fld}[{arg}][6].set != 0) || (game.state.mods.{fld}[{arg}][6].add != 0))')
        return (probe, None, 'present')
    return None


def cexpr(e, ctx):
    """Rust expression -> C expression text, or None when unresolvable."""
    if e is None:
        return None
    e = e.strip()
    if not e:
        return None
    e = e.rstrip(";").strip()
    if not e:
        return None
    e = strip_opt_tail(e)

    # -- literals -----------------------------------------------------
    if e == "true":
        return "1"
    if e == "false":
        return "0"
    if re.match(r'^-?\d+$', e):
        return e
    if is_str_lit(e):
        return c_str(e)
    m = re.match(r'^-\d+$', e)
    if m:
        return e

    # -- logical / comparison / arithmetic ---------------------------
    i = find_top(e, "||")
    if i >= 0:
        a, b = cexpr(e[:i], ctx), cexpr(e[i + 2:], ctx)
        return f'(({a}) || ({b}))' if a and b else None
    i = find_top(e, "&&")
    if i >= 0:
        a, b = cexpr(e[:i], ctx), cexpr(e[i + 2:], ctx)
        return f'(({a}) && ({b}))' if a and b else None
    sp = split_top_cmp(e)
    if sp:
        lhs, op, rhs = sp
        if op == ">" and rhs.startswith("="):
            lhs, op, rhs = lhs, ">=", rhs[1:]
        elif op == "<" and rhs.startswith("="):
            lhs, op, rhs = lhs, "<=", rhs[1:]
        elif op == "=" and rhs.startswith("="):
            lhs, op, rhs = lhs, "==", rhs[1:]
        elif op == "!" and rhs.startswith("="):
            lhs, op, rhs = lhs, "!=", rhs[1:]
        elif op in ("==", "!=", ">", "<", ">=", "<="):
            pass
        else:
            return None
        a, b = cexpr(lhs, ctx), cexpr(rhs, ctx)
        if a and b:
            return f'(({a}) {op} ({b}))'
        if a and b is None and op == "==":
            return None
        return None
    sp = split_top_ops(e, "+-")
    if sp:
        lhs, op, rhs = sp
        a, b = cexpr(lhs, ctx), cexpr(rhs, ctx)
        if a and b:
            return f'(({a}) {op} ({b}))'
        return None
    if e.startswith("!"):
        a = cexpr(e[1:], ctx)
        return f'(!({a}))' if a else None

    # -- unary / casts ----------------------------------------------
    e2 = re.sub(r'^(?:as\s+(?:i8|i16|i32|i64|u8|u16|u32|usize|isize)\b|as\s+\w+)', '', e).strip()
    e2 = re.sub(r'^\.to_string\(\)$', '', e2).strip()
    e2 = re.sub(r'^\.clone\(\)$', '', e2).strip()
    e2 = re.sub(r'^\.copied\(\)$', '', e2).strip()
    e2 = re.sub(r'^\.unwrap_or_default\(\)$', '', e2).strip()
    m = re.match(r'^\.unwrap_or\((.+)\)$', e2)
    if m:
        e2 = e2[:e2.index(".")] if e2.startswith(".") else m.group(1)
        e2 = m.group(1)
    m = re.match(r'^\.map_or\((.+?),\s*(\|[^|]*\|\s*)?(.+?)\)$', e2)
    if m:
        e2 = m.group(1)
    if e2 != e:
        r = cexpr(e2, ctx)
        if r:
            return r

    # -- enums -------------------------------------------------------
    m = re.match(r'^(?:[\w:]+::)*Phase::(\w+)$', e)
    if m:
        return PHASE_MAP.get(m.group(1))
    m = re.match(r'^(?:[\w:]+::)*MemberArea::(\w+)$', e)
    if m:
        return AREA.get(m.group(1))
    m = re.match(r'^(?:[\w:]+::)*HeartColor::(\w+)$', e)
    if m:
        return HEART_MAP.get(m.group(1))
    m = re.match(r'^(?:[\w:]+::)*CardOrientation::(\w+)$', e)
    if m:
        return ORIENT_MAP.get(m.group(1))
    m = re.match(r'^(?:[\w:]+::)*Side::(P[12])$', e)
    if m:
        return '"%s"' % ("P1" if m.group(1) == "P1" else "P2")
    if e == "None":
        return None

    # -- game / TestGame calls --------------------------------------
    name, args = split_call_args(e)
    if name is not None:
        r = map_call(name, args, ctx, e)
        if r is not None:
            return r

    # -- option-map lookups (backlog item 6) ------------------------
    mo = map_mods_opt(e, ctx)
    if mo:
        probe, bind, _kind = mo
        return bind if bind else probe

    # -- arena reads (backlog item 6) -------------------------------
    ar = map_arena(e, ctx)
    if ar:
        return arena_cexpr(ar, ctx)

    # -- direct state reads -----------------------------------------
    m = re.match(r'^(?:game|g)\.state\.player([12])\.energy_zone\.active_count\(\)$', e)
    if m:
        return f'game.state.p[{int(m.group(1))-1}].energy_active'
    m = re.match(r'^(?:game|g)\.state\.player([12])\.energy_zone\.sub_active$', e)
    if m:
        return f'game.state.p[{int(m.group(1))-1}].energy_active'
    m = re.match(r'^(?:game|g)\.state\.player([12])\.score$', e)
    if m:
        return f'game.state.p[{int(m.group(1))-1}].score'
    m = re.match(r'^(?:game|g)\.state\.player([12])\.life$', e)
    if m:
        return f'game.state.p[{int(m.group(1))-1}].life'
    m = re.match(r'^(?:game|g)\.state\.player([12])\.yell_note_icons$', e)
    if m:
        return f'game.state.p[{int(m.group(1))-1}].yell_note_icons'
    m = re.match(r'^(?:game|g)\.state\.(current_phase|phase|turn_number|turn|active|winner)$', e)
    if m:
        f = {"current_phase": "phase", "phase": "phase", "turn_number": "turn",
             "turn": "turn", "active": "active", "winner": "winner"}[m.group(1)]
        return f'game.state.{f}'
    m = re.match(r'^(?:game|g)\.state\.(live_success|p1_live_won|p2_live_won|loop_detected)\[(\d+)\]$', e)
    if m:
        return f'game.state.{m.group(1)}[{m.group(2)}]'
    for f in ("live_success", "p1_live_won", "p2_live_won", "loop_detected"):
        if e in (f'game.state.{f}', f'g.state.{f}'):
            return e
    m = re.match(r'^(?:game|g)\.state\.mods\.get_(blade|score|cost)_modifier\(\s*(.+?)\s*\)$', e)
    if m:
        return f'test_get_{m.group(1)}_modifier(&game, {cexpr(m.group(2), ctx)})'
    m = re.match(r'^(?:game|g)\.state\.mods\.(heart|need_heart)_modifiers'
                r'\.get\(\s*&?\s*(.+?)\s*\)\s*\.map\(\s*\|m\|\s*m\.get\(\s*&?\s*([^)]*?)\s*\)'
                r'\.copied\(\)\.unwrap_or\(\s*(\d+)\s*\)\s*\)', e)
    if m:
        fld, cid, col, dflt = m.group(1), cexpr(m.group(2), ctx), m.group(3), m.group(4)
        colc = cexpr(col, ctx) if col else "1"
        if colc is None:
            return f'game.state.mods.{fld}[{cid}][1].set + game.state.mods.{fld}[{cid}][1].add'
        return f'(game.state.mods.{fld}[{cid}][{colc}].set + game.state.mods.{fld}[{cid}][{colc}].add)'

    # -- bare identifier --------------------------------------------
    if re.match(r'^[A-Za-z_]\w*$', e):
        return e if e in ctx.declared else None
    return None


def arena_cexpr(ar, ctx):
    kind, pld = ar
    if kind == 'bag_len':
        pl, z = pld
        return f'game.state.p[{pl}].{z}.n'
    if kind == 'bag_is_empty':
        pl, z = pld
        return f'(game.state.p[{pl}].{z}.n == 0)'
    if kind == 'bag_contains':
        pl, z, v = pld
        vc = cexpr(re.sub(r'^\s*&\s*', '', v.strip()), ctx)
        if vc is None:
            return None
        return f'test_zone_has_id(&game, {pl}, "{z}", {vc})'
    if kind == 'bag_position':
        pl, z, v = pld
        vc = cexpr(v, ctx)
        if vc is None:
            return None
        ctx.use("zone_index")
        return f'pz_zone_index(&game, {pl}, "{z}", {vc})'
    if kind == 'bag_first':
        pl, z = pld
        return f'(game.state.p[{pl}].{z}.n > 0 ? game.state.p[{pl}].{z}.cards[0] : -1)'
    if kind == 'stage_idx':
        pl, i = pld
        return f'game.state.p[{pl}].stage[{i}]'
    if kind == 'raw':
        return pld
    return None


def map_call(name, args, ctx, whole):
    """Method / free-function call in expression position -> C, or None."""
    bare = name.split("::")[-1].split(".")[-1]
    g = "game"

    m = re.match(r'^(?:game|g)\.(id|new_id|id_ref)$', bare)
    if bare in ("id", "new_id", "id_ref") and re.match(r'^(?:game|g)\.', name):
        fn = {"id": "test_id", "new_id": "test_new_id", "id_ref": "test_id"}[bare]
        if not args:
            return None
        a = map_id(args[0], ctx.consts, g)
        if a:
            return a.replace("test_id(", fn + "(", 1)
        v = cexpr(args[0], ctx)
        if v and is_str_lit(v):
            return f'{fn}(&{g}, {v})'
        return None

    table = {
        "has_pending_choice": f'test_has_pending_choice(&{g})',
        "pending_choice_count": f'test_pending_choice_count(&{g})',
        "pending_choice_type": f'test_pending_choice_type(&{g})',
        "recalc": f'test_recalc(&{g})',
        "pass": f'test_pass(&{g})',
        "drain_auto_ability_choices": f'test_drain_auto_choices(&{g})',
        "expire_effects": f'test_expire_effects(&{g})',
        "find_card_by_no": None,
    }
    if bare in table and re.match(r'^(?:game|g)\.', name):
        if args:
            return None
        return table[bare]
    if bare == "get_pending_choice" and re.match(r'^(?:game|g)\.', name):
        ctx.use("choice_kind")
        return f'pz_choice_kind(&{g})'
    if bare == "is_waited" and re.match(r'^(?:game|g)\.', name) and args:
        ctx.use("stage_waited")
        return None
    if bare == "get_card_id" and args:
        v = cexpr(args[0], ctx)
        if v and is_str_lit(v):
            return f'test_id(&{g}, {v})'
        return None
    if bare in MODS_GETTER and re.match(r'^(?:game|g)\.state\.mods\.(?:get_)?', name) and len(args) == 1:
        v = cexpr(args[0], ctx)
        if v is None:
            return None
        return f'{MODS_GETTER[bare]}(&{g}, {v})'
    if bare == "get_heart_modifier" and len(args) == 2:
        a, b = cexpr(args[0], ctx), cexpr(args[1], ctx)
        if a and b:
            return f'({a} + game.state.mods.heart[{a}][{b}].add)'
        return None
    if bare == "is_waited" and len(args) == 1:
        v = cexpr(args[0], ctx)
        if v is None:
            return None
        ctx.use("is_waited")
        return f'pz_is_waited(&{g}, {v})'
    return None


# ──────────────────────────────────────────────────────────────────────
# Option / pattern mapping  (backlog item 5)
# ──────────────────────────────────────────────────────────────────────
def map_option(expr, ctx, var):
    """`Some(x) = EXPR` -> (probe_c, bind_c_or_None).

    Only Option-returning forms we can state exactly in C are accepted;
    anything else raises Skip (never an invented meaning).
    """
    expr = expr.strip()
    ar = map_arena(expr, ctx)
    if ar and ar[0] in ('bag_position', 'stage_idx'):
        kind, pld = ar
        if kind == 'bag_position':
            pl, z, v = pld
            vc = cexpr(re.sub(r'^\s*&\s*', '', v.strip()), ctx)
            if vc is None:
                raise Skip(f"if-let position over unmapped value: {expr}")
            ctx.use("zone_index")
            call = f'pz_zone_index(&game, {pl}, "{z}", {vc})'
        else:
            pl, i = pld
            call = f'test_zone_has_id(&game, {pl}, "stage", {i})'
        return (f'({call} >= 0)', call)
    mo = map_mods_opt(expr, ctx)
    if mo:
        probe, bind, _ = mo
        return (probe, bind)
    m = re.match(r'^(?:game|g)\.state\.get_pending_choice\(\)$', expr)
    if m:
        ctx.use("choice_kind")
        return (f'pz_choice_kind(&game) >= 0', None)
    name, args = split_call_args(expr)
    if name:
        bare = name.split("::")[-1]
        if bare == "get_card_id" and args:
            v = cexpr(args[0], ctx)
            if v and is_str_lit(v):
                return (f'(test_id(&game, {v}) >= 0)', f'test_id(&game, {v})')
        if bare == "has_pending_choice":
            return (f'test_has_pending_choice(&game)', None)
        if bare in ("pending_choice_count", "is_waited", "get_pending_choice"):
            c = cexpr(expr, ctx)
            if c:
                return (f'({c} != 0)', c)
    c = cexpr(expr, ctx)
    if c and re.match(r'^-?\d+$|^"[A-Za-z]', c):
        return (f'({c} != 0)', c)
    raise Skip(f"if-let over unmappable expression: {expr}")


CHOICE_RE = re.compile(
    r'^(?:rabuka_engine::ability::types::)?Choice::(\w+)\s*(\{.*\})?$')


def map_choice_pattern(pat, ctx):
    """`Choice::SelectCard { .. }` used as a pattern against a pending
    choice -> (kind_c, bound_field_names)."""
    m = CHOICE_RE.match(pat.strip())
    if not m:
        raise Skip(f"unmappable pattern: {pat}")
    variant = m.group(1)
    if variant not in CHOICE_KIND:
        raise Skip(f"unknown Choice variant: {variant}")
    inner = (m.group(2) or "").strip().strip("{}").strip()
    fields = []
    if inner and inner not in ("..",):
        for part in split_top_commas(inner):
            part = part.strip()
            if part == "..":
                continue
            fm = re.match(r'^(?:ref\s+|mut\s+|ref\s+mut\s+)*(\w+)$', part)
            if not fm:
                raise Skip(f"unmappable pattern field: {part}")
            fields.append(fm.group(1))
    ctx.use("choice_kind")
    return (f'pz_choice_kind(&game) == {CHOICE_KIND[variant]}', fields)


# ──────────────────────────────────────────────────────────────────────
# Statement translation
# ──────────────────────────────────────────────────────────────────────
MACRO_RE = re.compile(r'^([A-Za-z_][\w:!]*)\s*\(')


def split_macro_args(s):
    """s starts at the macro name; returns (name, [args], end_index)."""
    m = re.match(r'^([A-Za-z_][\w:!]*)', s)
    name = m.group(1)
    j = s.index("(", m.end() - 1)
    end = match_paren(s, j)
    if end < 0:
        return name, [], -1
    return name, split_top_commas(s[j + 1:end - 1]), end


def msg_of(args):
    """Pull the human message out of a macro arg list."""
    for a in reversed(args):
        if is_str_lit(a.strip()):
            return a.strip()
    return None


def clean_msg(lit, ctx):
    if lit is None:
        return None
    return c_str(lit)


def transpile_assert(s, ctx, kind):
    name, args, end = split_macro_args(s)
    if end < 0 or not args:
        raise Skip(f"unparsed macro: {s}")
    msg = clean_msg(msg_of(args), ctx)
    if kind == "assert_eq":
        # trailing message already excluded by construction below
        core = args
        if msg:
            core = args[:-1]
        if len(core) < 2:
            raise Skip(f"assert_eq! with {len(core)} operands: {s}")
        a_expr, b_expr = core[0], core[1]
        a = cexpr(a_expr, ctx)
        b = cexpr(b_expr, ctx)
        a, b = unwrap_option_cmp(a_expr, b_expr, a, b, ctx)
        if a is None or b is None:
            ctx.todo(s)
            return
        ctx.n_check += 1
        if (is_str_lit(a_expr.strip()) or 'test_pending_choice_type' in a
                or a.startswith('"')) or (is_str_lit(b_expr.strip()) or b.startswith('"')):
            ctx.emit(f'    CHECK_STR({a}, {b}, {msg or "assert_eq"});')
        else:
            ctx.emit(f'    CHECK_EQ({a}, {b}, {msg or "assert_eq"});')
        return
    core = args
    if msg:
        core = args[:-1]
    if not core:
        raise Skip(f"assert! with no condition: {s}")
    cond = core[0].strip()
    c = cexpr(cond, ctx)
    if c is None:
        # try `lhs == rhs` inside the condition
        sp = split_top_cmp(cond)
        if sp:
            lhs, op, rhs = sp
            a, b = cexpr(lhs, ctx), cexpr(rhs, ctx)
            if a and b:
                ctx.n_check += 1
                ctx.emit(f'    CHECK(({a}) {op} ({b}), {msg or cond});')
                return
        ctx.todo(s)
        return
    ctx.n_check += 1
    ctx.emit(f'    CHECK({c}, {msg or cond});')


def unwrap_option_cmp(a_expr, b_expr, a, b, ctx):
    """`mods.X.get(&id)` vs `Some(2)` / `None` -> presence-aware compare."""
    mo_a = map_mods_opt(a_expr.strip(), ctx)
    mo_b = map_mods_opt(b_expr.strip(), ctx)
    if mo_a and is_none_expr(b_expr):
        return (f'({mo_a[0]} ? 1 : 0)', "0")
    if mo_b and is_none_expr(a_expr):
        return (f'({mo_b[0]} ? 1 : 0)', "0")
    if mo_a:
        a = mo_a[1] or mo_a[0]
        if is_some_expr(b_expr):
            inner = some_inner(b_expr)
            ic = cexpr(inner, ctx)
            if ic is None:
                return (a, b)
            return (a, ic)
    if mo_b:
        b = mo_b[1] or mo_b[0]
        if is_some_expr(a_expr):
            inner = some_inner(a_expr)
            ic = cexpr(inner, ctx)
            if ic is None:
                return (a, b)
            return (ic, b)
    # bare `Some(v)` on one side of a plain (non-Option) comparison
    if is_some_expr(a_expr) and not is_none_expr(b_expr):
        ic = cexpr(some_inner(a_expr), ctx)
        if ic is not None:
            return (ic, b)
    if is_some_expr(b_expr) and not is_none_expr(a_expr):
        ic = cexpr(some_inner(b_expr), ctx)
        if ic is not None:
            return (a, ic)
    return (a, b)


def is_none_expr(e):
    return e.strip() in ("None", "rabuka_engine::core::types::None")


def is_some_expr(e):
    e = e.strip()
    return e.startswith("Some(") and e.endswith(")")


def some_inner(e):
    e = e.strip()
    j = e.index("(")
    return e[j + 1:-1].strip()


def transpile_stmts(ls, ctx, i, ind, stop_at_brace=True):
    while i < len(ls):
        s = ls[i]
        st = s.strip()
        if st.startswith("}"):
            return i + 1
        i = emit_one(ls, i, ctx, ind)
    return i


def emit_one(ls, i, ctx, ind):
    s = ls[i].strip()
    if s.startswith("}"):
        return i + 1
    r = transpile_stmt(ls, i, ctx, ind)
    return r if r > i else i + 1


def ind_of(ctx, ind):
    return ind


def emit_block_head(ls, i, ctx, ind, head_c, skip_braces=True):
    """Emit `head_c {`, the nested statements, and the closing brace."""
    ctx.emit(f"{ind}{head_c} {{")
    ctx.loop_depth += 1
    j = transpile_stmts(ls, ctx, i + 1, ind + "    ")
    ctx.loop_depth -= 1
    ctx.emit(f"{ind}}}")
    return j


def transpile_stmt(ls, i, ctx, ind):
    s = ls[i].strip()
    pad = ind

    # ---- attributes / comments --------------------------------------
    if s.startswith("#["):
        return i + 1
    if s.startswith("#[cfg"):
        # conditional-compilation item: transpile the payload, it is inert here
        return i + 1

    # ---- closing brace of an if/else chain --------------------------
    if s.startswith("}"):
        return i + 1

    # ---- `match` ---------------------------------------------------
    if s.startswith("match ") or s == "match":
        raise Skip("match expression")

    # ---- `if let` --------------------------------------------------
    m = re.match(r'^if\s+let\s+(.+?)\s*=\s*(.+?)\s*\{$', s)
    if m:
        return emit_if_let(ls, i, ctx, ind, m.group(1), m.group(2))

    # ---- `while let` -----------------------------------------------
    m = re.match(r'^while\s+let\s+(.+?)\s*=\s*(.+?)\s*\{$', s)
    if m:
        return emit_while_let(ls, i, ctx, ind, m.group(1), m.group(2))

    # ---- `while` / `loop` ------------------------------------------
    m = re.match(r'^while\s+(.+?)\s*\{$', s)
    if m:
        c = cexpr(m.group(1), ctx)
        if c is None:
            raise Skip(f"while over unmapped condition: {m.group(1)}")
        return emit_block_head(ls, i, ctx, ind, f"while ({c})")
    if s.startswith("loop"):
        raise Skip("loop expression")

    # ---- `for` ------------------------------------------------------
    m = re.match(r'^for\s+(.+?)\s+in\s+(.+?)\s*\{$', s)
    if m:
        var, rng = m.group(1), m.group(2)
        r = re.match(r'^(\d+)\s*\.\.\s*(\d+)$', rng)
        if r and var == "_":
            c = f"for (int {ctx.fresh('i')}_ = {r.group(1)}; {ctx.fresh('i')}_ < {r.group(2)}; {ctx.fresh('i')}_++)"
            return emit_block_head(ls, i, ctx, ind, c)
        r = re.match(r'^(\d+)\s*\.\.=\s*(\d+)$', rng)
        if r and var == "_":
            c = f"for (int {ctx.fresh('i')}_ = {r.group(1)}; {ctx.fresh('i')}_ <= {r.group(2)}; {ctx.fresh('i')}_++)"
            return emit_block_head(ls, i, ctx, ind, c)
        r = re.match(r'^(\w+)\s*\.\.\s*(\d+)$', rng)
        if r:
            v = ctx.fresh(var)
            c = f"for (int {v} = {r.group(1)}; {v} < {r.group(2)}; {v}++)"
            return emit_block_head(ls, i, ctx, ind, c)
        raise Skip(f"for over unmapped range: {rng}")

    # ---- `if` / `else` ----------------------------------------------
    m = re.match(r'^if\s+(.+?)\s*\{$', s)
    if m:
        c = cexpr(m.group(1), ctx)
        if c is None:
            raise Skip(f"if over unmapped condition: {m.group(1)}")
        j = emit_block_head(ls, i, ctx, ind, f"if ({c})")
        return maybe_else(ls, j, ctx, ind)

    # ---- `let` ------------------------------------------------------
    m = re.match(r'^let\s+(?:mut\s+)?(.+?)\s*=\s*(.+?);?$', s)
    if m:
        return emit_let(i, m.group(1), m.group(2), ctx, ind)

    # ---- `return` ---------------------------------------------------
    if s in ("return;", "return"):
        ctx.emit(f"{ind}return;")
        return i + 1

    # ---- block / unsafe --------------------------------------------
    if s == "{":
        j = transpile_stmts(ls, ctx, i + 1, ind)
        if j < len(ls) and ls[j].strip().startswith("}"):
            j += 1
        return j
    if s.startswith("unsafe"):
        nxt = ls[i + 1].strip() if i + 1 < len(ls) else ""
        if nxt == "{":
            j = transpile_stmts(ls, ctx, i + 2, ind)
            if j < len(ls) and ls[j].strip().startswith("}"):
                j += 1
            return j
        raise Skip("unsafe block")

    # ---- plain expression statement ---------------------------------
    if s.endswith(";"):
        emit_expr_stmt(s, ctx, ind)
        return i + 1

    raise Skip(f"unhandled statement: {s[:70]}")


def maybe_else(ls, j, ctx, ind):
    if j >= len(ls):
        return j
    nxt = ls[j].strip()
    if nxt.startswith("}"):
        nxt = nxt[1:].strip()
        if nxt.startswith("else"):
            rest = nxt[3:].strip()
            if rest.startswith("if "):
                m = re.match(r'^else\s+if\s+(.+?)\s*\{$', nxt)
                if m:
                    c = cexpr(m.group(1), ctx)
                    if c is None:
                        raise Skip("else-if over unmapped condition")
                    jj = emit_block_head(ls, j, ctx, ind, f"if ({c})")
                    return maybe_else(ls, jj, ctx, ind)
            if rest == "{":
                return emit_block_head(ls, j, ctx, ind, "else")
            if rest == "":
                return emit_block_head(ls, j, ctx, ind, "else")
    return j


def emit_if_let(ls, i, ctx, ind, pat, expr):
    """Backlog item 5: `if let PAT = EXPR { ... } [else ...]`."""
    # pattern forms
    m = re.match(r'^Some\(\s*(?:ref\s+|mut\s+)*(\w+)\s*\)$', pat.strip())
    if m:
        var = m.group(1)
        probe, bind = map_option(expr, ctx, var)
        if bind is not None and var != "_":
            b = ctx.fresh(var)
            probe = probe.replace(bind, b, 1) if bind in probe else probe
            ctx.emit(f"{ind}int {b} = {bind};")
            head = f"if ({b} >= 0)"
        else:
            head = f"if ({probe})"
        j = emit_block_head(ls, i, ctx, ind, head)
        return maybe_else(ls, j, ctx, ind)
    m = CHOICE_RE.match(pat.strip())
    if m:
        kind, fields = map_choice_pattern(pat, ctx)
        if fields:
            raise Skip(f"if let with pattern field bindings: {pat}")
        j = emit_block_head(ls, i, ctx, ind, f"if ({kind})")
        return maybe_else(ls, j, ctx, ind)
    if pat.strip() in ("Ok", "Ref"):
        raise Skip(f"if let pattern: {pat}")
    raise Skip(f"if let pattern: {pat}")


def emit_while_let(ls, i, ctx, ind, pat, expr):
    m = re.match(r'^Some\(\s*(?:ref\s+|mut\s+)*(\w+)\s*\)$', pat.strip())
    if not m:
        raise Skip(f"while let pattern: {pat}")
    var = m.group(1)
    probe, bind = map_option(expr, ctx, var)
    if bind is not None and var != "_":
        b = ctx.fresh(var)
        head = f"while ((({b} = {bind})) >= 0)"
        ctx.emit(f"{ind}int {b} = -1;")
        ctx.emit(f"{head}) {{")
    else:
        ctx.emit(f"{ind}while ({probe}) {{")
    ctx.loop_depth += 1
    j = transpile_stmts(ls, ctx, i + 1, ind + "    ")
    ctx.loop_depth -= 1
    ctx.emit(f"{ind}}}")
    return maybe_else(ls, j, ctx, ind)


def emit_let(ls_i, lhs, rhs, ctx, ind):
    i = ls_i
    rhs = rhs.rstrip(";").strip()
    lhs = lhs.strip()

    # `let db = load_real_database();`
    if re.match(r'^\w+$', lhs) and re.search(r'\bload_real_database\b', rhs) and \
            'TestGame::new' not in rhs:
        return i + 1
    if re.match(r'^\w+$', lhs) and rhs.endswith('TestGame::new(db)'):
        v = ctx.fresh(lhs)
        ctx.have_game = True
        ctx.emit(f"{ind}TestGame {v};")
        ctx.emit(f"{ind}test_game_new(&{v});")
        return i + 1
    if re.match(r'^\w+$', lhs) and 'TestGame::new' in rhs:
        v = ctx.fresh(lhs)
        ctx.emit(f"{ind}TestGame {v};")
        ctx.emit(f"{ind}test_game_new(&{v});")
        return i + 1

    # tuple destructuring `let (a, b) = EXPR;`
    m = re.match(r'^\((.+)\)$', lhs)
    if m:
        parts = [p.strip() for p in split_top_commas(m.group(1))]
        c = cexpr(rhs, ctx)
        if c is None:
            raise Skip(f"tuple let over unmapped rhs: {rhs}")
        for p in parts:
            v = ctx.fresh(p)
            ctx.emit(f"{ind}int {v} = {c};")
        return i + 1

    v = ctx.fresh(lhs)
    c = cexpr(rhs, ctx)
    if c is None:
        # method-call statement used as a value (e.g. `let n = game.foo();`)
        nm, a2 = split_call_args(rhs)
        if nm and not emit_expr_stmt(rhs + ";", ctx, ind, quiet=True):
            raise Skip(f"let {v} = {rhs}")
        ctx.emit(f"{ind}int {v} = 0; // TODO let {v}")
        ctx.n_todo += 1
        return i + 1
    if c.startswith('"'):
        ctx.emit(f"{ind}const char *{v} = {c};")
    else:
        ctx.emit(f"{ind}int {v} = {c};")
    return 1


def emit_expr_stmt(s, ctx, ind, quiet=False):
    """Translate a `...;` statement.  Returns True if handled."""
    body = s.rstrip()
    if body.endswith(";"):
        body = body[:-1]
    body = body.strip()

    # assertions ---------------------------------------------------
    m = MACRO_RE.match(body)
    if m:
        name = m.group(1).rstrip("!")
        base = name.split("::")[-1]
        if base in ("assert_eq", "assert_ne", "assert", "debug_assert"):
            if base == "debug_assert":
                base = "assert"
            transpile_assert(body, ctx, "assert_eq" if base == "assert_eq" else "assert")
            return True
        if base in ("assert_ability", "assert_abilities", "panic"):
            ctx.todo(body)
            return True
        ctx.todo(body)
        return True

    # inlined-runner calls ------------------------------------------
    name, args = split_call_args(body)
    if name is not None:
        bare = name.split("::")[-1].split(".")[-1]
        first_is_game = bool(re.match(r'^(?:&?\s*(?:mut\s+)?(?:game|g)\s*,)', args[0] if args else ""))
        if bare in RUNTIME_HELPER_CALLS and (first_is_game or
                                             re.match(r'^&mut\s*(?:game|g)$', args[0] if args else "")):
            if bare == "fill_decks":
                ctx.use("fill_decks")
                ctx.emit(f"{ind}pz_fill_decks(&game, {cexpr(args[1], ctx) if len(args) > 1 else '-1'});")
                return True
            if bare == "put_on_deck_top":
                ctx.use("put_on_deck_top")
                ctx.emit(f"{ind}test_insert_deck_top(&game, {cexpr(args[0], ctx)}, {cexpr(args[1], ctx)});")
                return True
            if bare == "fill_energy_deck":
                ctx.use("fill_energy_deck")
                ctx.emit(f"{ind}pz_fill_energy_deck(&game, {cexpr(args[0], ctx)}, {cexpr(args[1], ctx)});")
                return True
            ctx.use("drain")
            ctx.emit(f"{ind}pz_drain(&game);")
            return True

        # TestGame runner methods
        if re.match(r'^(?:game|g)\.', name):
            r = map_stmt_call(bare, args, ctx, body)
            if r is not None:
                ctx.emit(f"{ind}{r}" if r.rstrip().endswith(";") else f"{ind}{r};")
                return True

        # engine-state mutators
        r = map_mutator(body, ctx)
        if r is not None:
            ctx.emit(f"{ind}{r}" if r.rstrip().endswith(";") else f"{ind}{r};")
            return True

    # field writes / calls handled by cexpr
    c = cexpr(body, ctx)
    if c is not None:
        ctx.emit(f"{ind}{c};")
        return True

    # bare field/method write that map_mutator did not claim
    r = map_mutator(body, ctx)
    if r is not None:
        ctx.emit(f"{ind}{r}" if r.rstrip().endswith(";") else f"{ind}{r};")
        return True

    if not quiet:
        ctx.todo(body)
    return False


def map_stmt_call(bare, args, ctx, body):
    g = "game"
    if bare == "pass" and not args:
        return f'test_pass(&{g})'
    if bare == "recalc" and not args:
        return f'test_recalc(&{g})'
    if bare == "expire_effects" and not args:
        return f'test_expire_effects(&{g})'
    if bare == "drain_auto_ability_choices" and not args:
        return f'test_drain_auto_choices(&{g})'
    if bare == "clear_mods_for_card" and len(args) == 1:
        v = cexpr(args[0], ctx)
        return f'test_clear_mods_for_card(&{g}, {v})' if v else None
    if bare == "add_to_hand" and len(args) == 1:
        v = cexpr(args[0], ctx)
        return f'test_add_to_hand(&{g}, {v})' if v else None
    if bare == "add_to_discard" and len(args) == 1:
        v = cexpr(args[0], ctx)
        return f'test_add_to_discard(&{g}, {v})' if v else None
    if bare == "add_to_success" and len(args) == 1:
        v = cexpr(args[0], ctx)
        return f'test_add_to_success(&{g}, {v})' if v else None
    if bare == "add_to_live" and len(args) == 1:
        v = cexpr(args[0], ctx)
        return f'test_add_to_live(&{g}, {v})' if v else None
    if bare == "add_to_deck" and len(args) == 1:
        v = cexpr(args[0], ctx)
        return f'test_add_to_deck(&{g}, {v})' if v else None
    if bare == "give_energy" and len(args) == 1:
        v = cexpr(args[0], ctx)
        return f'test_give_energy(&{g}, {v})' if v else None
    if bare == "add_to_stage" and len(args) == 2:
        a, b = cexpr(args[0], ctx), cexpr(args[1], ctx)
        return f'test_add_to_stage(&{g}, {a}, {b})' if a and b else None
    if bare == "fire_debut" and len(args) == 1:
        v = cexpr(args[0], ctx)
        return f'test_fire_debut(&{g}, {v})' if v else None
    if bare == "set_live_card" and len(args) == 1:
        v = cexpr(args[0], ctx)
        return f'test_set_live_card(&{g}, 0, {v})' if v else None
    if bare == "activate_ability" and len(args) == 1:
        v = cexpr(args[0], ctx)
        return f'test_activate_ability(&{g}, {v})' if v else None
    if bare == "activate_ability_index" and len(args) == 2:
        v, k = cexpr(args[0], ctx), cexpr(args[1], ctx)
        if v and k:
            return f'test_activate_ability(&{g}, {v})'
        return None
    if bare in ("play_to_stage", "try_play_to_stage") and len(args) == 2:
        v, a = cexpr(args[0], ctx), cexpr(args[1], ctx)
        if v and a:
            return f'test_play_to_stage(&{g}, {v}, {a})'
        return None
    if bare == "advance_to_phase" and len(args) == 1:
        p = cexpr(args[0], ctx)
        if p is None:
            m = re.search(r'Phase::(\w+)', args[0])
            p = PHASE_MAP.get(m.group(1)) if m else None
        if p is None:
            return None
        ctx.use("advance_to_phase")
        return f'pz_advance_to_phase(&{g}, {p})'
    if bare == "select_indices" and len(args) == 1:
        m = re.match(r'^&\s*\[\s*(.*?)\s*\]$', args[0].strip())
        if not m:
            return None
        vals = [x.strip() for x in split_top_commas(m.group(1))] if m.group(1).strip() else []
        cs = [cexpr(v, ctx) for v in vals]
        if any(c is None for c in cs):
            return None
        ctx.use("drain")
        if not cs:
            return 'test_resume_choice(&game, -1)'
        return f'test_resume_choice(&{g}, {cs[0]})'
    if bare == "set_active_side" and len(args) == 1:
        return 'test_set_active_side(&game, ' + ('0' if 'P1' in args[0] else '1') + ')'
    if bare in ("add_to_hand_for", "give_energy_for"):
        return None
    return None


def map_mutator(full, ctx):
    """Engine state mutators: mods.add_*, zone.cards.push, field writes, etc.

    `full` is the whole statement text with the trailing ';' already removed.
    """
    full = full.strip()
    m = re.match(r'^(?:game|g)\.state\.player([12])\.(\w+)\.cards\.push\(\s*(.+?)\s*\)$', full)
    if m:
        pl, zf, v = int(m.group(1)) - 1, m.group(2), cexpr(m.group(3), ctx)
        if ZONE.get(zf) == "deck":
            return f'test_add_to_deck_pl(&game, {pl}, {v})' if v else None
        if ZONE.get(zf) == "energy_deck":
            return f'test_add_to_energy_deck(&game, {pl}, {v})' if v else None
        if ZONE.get(zf) == "hand":
            return f'test_add_to_hand(&game, {v})' if v else None
        if ZONE.get(zf) == "discard":
            return f'test_add_to_discard(&game, {v})' if v else None
        if ZONE.get(zf) == "live":
            return f'test_add_to_live(&game, {v})' if v else None
        if ZONE.get(zf) == "success":
            return f'test_add_to_success(&game, {v})' if v else None
        if ZONE.get(zf) == "energy":
            return f'test_add_to_energy(&game, {pl}, {v})' if v else None
        if v:
            return f'if (game.state.p[{pl}].{ZONE[zf]}.n < RB_MAX_ZONE) game.state.p[{pl}].{ZONE[zf]}.cards[game.state.p[{pl}].{ZONE[zf]}.n++] = {v};'
        return None
    m = re.match(r'^(?:game|g)\.state\.player([12])\.(\w+)\.cards\.clear\(\)$', full)
    if m and ZONE.get(m.group(2)):
        return f'game.state.p[{int(m.group(1))-1}].{ZONE[m.group(2)]}.n = 0;'
    m = re.match(r'^(?:game|g)\.state\.player([12])\.main_deck\.cards\.insert\(\s*0\s*,\s*(.+?)\s*\)$', full)
    if m:
        v = cexpr(m.group(2), ctx)
        return f'test_insert_deck_top(&game, {int(m.group(1))-1}, {v})' if v else None
    m = re.match(r'^(?:game|g)\.state\.player([12])\.stage\.stage\s*=\s*\[\s*(.+?)\s*\]$', full)
    if m:
        pl = int(m.group(1)) - 1
        vals = split_top_commas(m.group(2))
        if len(vals) != 3:
            return None
        out = []
        for k, vv in enumerate(vals):
            c = cexpr(vv, ctx)
            if c is None:
                return None
            out.append(f'game.state.p[{pl}].stage[{k}] = {c}; game.state.p[{pl}].stage_wait[{k}] = 0;')
        return " ".join(out)
    m = re.match(r'^(?:game|g)\.state\.player([12])\.stage\.set_area\(\s*([^,]+?)\s*,\s*(.+?)\s*\)$', full)
    if m:
        pl = int(m.group(1)) - 1
        a = cexpr(m.group(2), ctx)
        v = cexpr(m.group(3), ctx)
        if a and v:
            return f'test_add_to_stage(&game, {a}, {v})'
        return None
    m = re.match(r'^(?:game|g)\.state\.player([12])\.stage\.stage\[(\d+)\]\s*=\s*(.+)$', full)
    if m:
        v = cexpr(m.group(3), ctx)
        if v:
            return f'game.state.p[{int(m.group(1))-1}].stage[{m.group(2)}] = {v};'
        return None
    m = re.match(r'^(?:game|g)\.state\.player([12])\.stage\.place_under_card\(\s*([^,]+?)\s*,\s*(.+?)\s*\)$', full)
    if m:
        a, v = cexpr(m.group(2), ctx), cexpr(m.group(3), ctx)
        if a and v:
            return f'test_place_under(&game, {int(m.group(1))-1}, {a}, {v})'
        return None
    # mods mutators
    m = re.match(r'^(?:game|g)\.state\.mods\.(add|set)_(blade|score|cost)_modifier\(\s*(.+?)\s*,\s*(.+?)\s*\)$', full)
    if m:
        v, n = cexpr(m.group(3), ctx), cexpr(m.group(4), ctx)
        if v and n:
            return f'rb_mods_{m.group(1)}_{m.group(2)}(&game.state.mods, {v}, {n})'
    m = re.match(r'^(?:game|g)\.state\.mods\.(add|set)_heart_modifier\(\s*(.+?)\s*,\s*(.+?)\s*,\s*(.+?)\s*\)$', full)
    if m:
        v, c, n = cexpr(m.group(2), ctx), cexpr(m.group(3), ctx), cexpr(m.group(4), ctx)
        if v and c and n:
            return f'rb_mods_{m.group(1)}_heart(&game.state.mods, {v}, {c}, {n})'
    m = re.match(r'^(?:game|g)\.state\.mods\.(?:add|set)_need_heart_modifier\(\s*(.+?)\s*,\s*(.+?)\s*,\s*(.+?)\s*\)$', full)
    if m:
        v, c, n = cexpr(m.group(1), ctx), cexpr(m.group(2), ctx), cexpr(m.group(3), ctx)
        if v and c and n:
            return f'rb_mods_add_need_heart(&game.state.mods, {v}, {c}, {n})'
    m = re.match(r'^(?:game|g)\.state\.mods\.set_orientation_modifier\(\s*(.+?)\s*,\s*(.+?)\s*\)$', full)
    if m:
        v, o = cexpr(m.group(1), ctx), cexpr(m.group(2), ctx)
        if v and o:
            return f'rb_mods_set_orientation(&game.state.mods, {v}, (char[]){(o) == "1" and "active" or "wait"})'
    m = re.match(r'^(?:game|g)\.state\.recalc\w*\(\)$', full)
    if m:
        return 'test_recalc(&game)'
    return None


# ──────────────────────────────────────────────────────────────────────
# Per-test-function porting
# ──────────────────────────────────────────────────────────────────────
def port_test(fname, body, helpers, consts):
    """-> (ctx, ok).  Raises Skip when the function cannot be ported."""
    ctx = Ctx(helpers, consts, fname)
    text = body
    text = inline_trivial(text, helpers, consts)
    text = inline_helpers(text, helpers, consts)
    ls = logical_lines(text)
    # a Rust test body is a sequence; trailing stray braces are tolerated
    transpile_stmts(ls, ctx, 0, "    ")
    return ctx


# ──────────────────────────────────────────────────────────────────────
# C emission
# ──────────────────────────────────────────────────────────────────────
PREAMBLE = '''#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int failures;
static int checks;

#define CHECK(cond, msg) do { \\
    checks++; \\
    if (!(cond)) { \\
        fprintf(stderr, "FAIL %s:%d: %s\\n", __FILE__, __LINE__, (msg)); \\
        failures++; \\
    } else { printf("ok: %s\\n", (msg)); } \\
} while (0)

#define CHECK_EQ(actual, expected, msg) do { \\
    long a_ = (long)(actual), e_ = (long)(expected); \\
    checks++; \\
    if (a_ != e_) { \\
        fprintf(stderr, "FAIL %s:%d: %s (got %ld expected %ld)\\n", __FILE__, __LINE__, (msg), a_, e_); \\
        failures++; \\
    } else { printf("ok: %s\\n", (msg)); } \\
} while (0)

#define CHECK_STR(actual, expected, msg) do { \\
    const char *a_ = (actual), *e_ = (expected); \\
    checks++; \\
    if (!a_ || !e_ || strcmp(a_, e_) != 0) { \\
        fprintf(stderr, "FAIL %s:%d: %s (got %s expected %s)\\n", __FILE__, __LINE__, (msg), a_ ? a_ : "(null)", e_ ? e_ : "(null)"); \\
        failures++; \\
    } else { printf("ok: %s\\n", (msg)); } \\
} while (0)
'''

RUNTIME = {
    "zone_index": '''static int pz_zone_index(TestGame *g, int pl, const char *z, int id)
{
    RbPlayer *p = &g->state.p[pl];
    if (!strcmp(z, "stage")) {
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (p->stage[i] != RB_EMPTY_SLOT && p->stage[i] == id) return i;
        return -1;
    }
    {
        RbBag *b = NULL;
        if (!strcmp(z, "hand")) b = &p->hand;
        else if (!strcmp(z, "deck")) b = &p->deck;
        else if (!strcmp(z, "discard")) b = &p->discard;
        else if (!strcmp(z, "live")) b = &p->live;
        else if (!strcmp(z, "success")) b = &p->success;
        else if (!strcmp(z, "energy")) b = &p->energy;
        else if (!strcmp(z, "energy_deck")) b = &p->energy_deck;
        if (!b) return -1;
        for (int i = 0; i < b->n; i++) if (b->cards[i] == id) return i;
    }
    return -1;
}''',
    "choice_kind": '''static int pz_choice_kind(TestGame *g)
{
    if (!rb_has_pending_choice(&g->state)) return -1;
    return (int)rb_get_pending_choice(&g->state)->kind;
}''',
    "stage_waited": '''static int pz_stage_waited(TestGame *g, int pl)
{
    return g->state.p[pl].stage_wait[0] || g->state.p[pl].stage_wait[1]
        || g->state.p[pl].stage_wait[2];
}''',
    "is_waited": '''static int pz_is_waited(TestGame *g, int id)
{
    for (int pl = 0; pl < 2; pl++)
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (g->state.p[pl].stage[i] == id) return g->state.p[pl].stage_wait[i];
    return 0;
}''',
    "drain": '''static void pz_drain(TestGame *g)
{
    int guard = 0;
    while (rb_has_pending_choice(&g->state) && guard++ < 256)
        rb_resume_with_choice(&g->state, 0);
}''',
    "advance_to_phase": '''static void pz_advance_to_phase(TestGame *g, int target)
{
    for (int i = 0; i < 16; i++) {
        if (g->state.phase == target) return;
        test_pass(g);
        if (g->state.phase == target) return;
        { int guard = 0;
          while (rb_has_pending_choice(&g->state) && guard++ < 64)
              rb_resume_with_choice(&g->state, 0); }
    }
    fprintf(stderr, "  [xpile] advance_to_phase(%d) never reached (stuck at %d)\\n",
            target, g->state.phase);
}''',
    "fill_decks": '''static void pz_fill_decks(TestGame *g, int filler)
{
    for (int pl = 0; pl < 2; pl++) {
        g->state.p[pl].deck.n = 0;
        for (int i = 0; i < 30; i++)
            g->state.p[pl].deck.cards[g->state.p[pl].deck.n++] = filler;
    }
}''',
    "fill_energy_deck": '''static void pz_fill_energy_deck(TestGame *g, int pl, int count)
{
    int eid = test_id(g, "LL-E-001-SD");
    for (int i = 0; i < count; i++)
        g->state.p[pl].energy_deck.cards[g->state.p[pl].energy_deck.n++] = eid;
}''',
    "set_active_side": '''static void test_set_active_side(TestGame *g, int p1_active)
{
    g->state.p[0].is_first_attacker = p1_active;
    g->state.p[1].is_first_attacker = !p1_active;
}''',
}


def emit_c(stem, tests, skipped):
    out = [PREAMBLE]
    used = set()
    for _n, ctx in tests:
        used |= ctx.uses
    for key in sorted(used):
        if key in RUNTIME:
            out.append(RUNTIME[key])
    out.append("")
    for name, ctx in tests:
        out.append(f"/* ported from {name} */")
        out.append(f"static void t_{name}(void)")
        out.append("{")
        out.extend(ctx.lines)
        out.append("}")
        out.append("")
    out.append("int main(void)")
    out.append("{")
    for name, _ctx in tests:
        out.append(f'    printf("--- {name} ---\\n");')
        out.append(f"    t_{name}();")
    out.append('    printf("\\n%s: %d checks, %d failures\\n", "' + stem + '", checks, failures);')
    out.append("    return failures ? 1 : 0;")
    out.append("}")
    out.append("")
    if skipped:
        out.append("/* NOT PORTED (whole function skipped):")
        for name, why in skipped:
            out.append(f" *   {name}: {why}")
        out.append(" */")
        out.append("")
    return "\n".join(out)


# ──────────────────────────────────────────────────────────────────────
# File-level driver
# ──────────────────────────────────────────────────────────────────────
def port_file(rs_path):
    text = rs_path.read_text(encoding="utf-8", errors="replace")
    text = join_method_continuations(text)
    helpers, tests, consts = fn_bodies(text)
    ported, skipped = [], []
    for name, body in tests:
        try:
            ctx = port_test(name, body, helpers, consts)
            ported.append((name, ctx))
        except Skip as ex:
            skipped.append((name, ex.reason))
        except Exception as ex:                      # never crash the batch
            skipped.append((name, f"internal: {type(ex).__name__}: {ex}"))
    return ported, skipped


def rust_assert_count(body):
    n = 0
    for line in body.split("\n"):
        t = line.strip()
        if t.startswith("assert!") or t.startswith("assert_eq!") or \
           t.startswith("assert_ability!") or t.startswith("assert_abilities!"):
            n += 1
    return n


def audit(paths, report=None, jsonout=None):
    files, total_fn, ported_fn, skipped_fn = 0, 0, 0, 0
    rust_asserts, checks, todos = 0, 0, 0
    skip_reasons = {}
    ported_names = []
    for p in paths:
        try:
            text = join_method_continuations(p.read_text(encoding="utf-8", errors="replace"))
        except Exception:
            continue
        helpers, tests, consts = fn_bodies(text)
        if not tests:
            continue
        files += 1
        for name, body in tests:
            total_fn += 1
            rust_asserts += rust_assert_count(body)
            try:
                ctx = port_test(name, body, helpers, consts)
                ported_fn += 1
                checks += ctx.n_check
                todos += ctx.n_todo
                ported_names.append(f"{p.name}::{name}")
            except Skip as ex:
                skipped_fn += 1
                key = ex.reason.split(":", 1)[0][:60]
                skip_reasons[key] = skip_reasons.get(key, 0) + 1
            except Exception as ex:
                skipped_fn += 1
                key = "internal: " + type(ex).__name__
                skip_reasons[key] = skip_reasons.get(key, 0) + 1
    res = {
        "files": files,
        "test_fns_total": total_fn,
        "test_fns_ported": ported_fn,
        "test_fns_skipped": skipped_fn,
        "rust_assert_macros": rust_asserts,
        "assertions_emitted": checks,
        "assertion_todos": todos,
        "todo_ratio_pct": round(100.0 * todos / max(1, checks + todos), 1),
        "skip_reasons": dict(sorted(skip_reasons.items(), key=lambda kv: -kv[1])),
    }
    print(json.dumps(res, indent=2, ensure_ascii=False))
    if report:
        L = ["# port_one.py transpiler coverage audit", ""]
        L.append(f"- Rust files scanned: **{files}**")
        L.append(f"- Rust `#[test]` functions: **{total_fn}**")
        L.append(f"- functions that PORT: **{ported_fn}** ({round(100.0*ported_fn/max(1,total_fn),1)}%)")
        L.append(f"- functions SKIPPED: **{skipped_fn}**")
        L.append(f"- Rust assertion macros seen: **{rust_asserts}**")
        L.append(f"- C assertions emitted (CHECK/CHECK_EQ/CHECK_STR): **{checks}**")
        L.append(f"- untranslated statement TODOs: **{todos}**")
        L.append("")
        L.append("## top skip reasons")
        L.append("")
        for k, v in list(res["skip_reasons"].items())[:30]:
            L.append(f"- {v:5d}  {k}")
        pathlib.Path(report).write_text("\n".join(L) + "\n", encoding="utf-8")
    if jsonout:
        pathlib.Path(jsonout).write_text(json.dumps(
            {"summary": res, "ported": ported_names}, indent=1, ensure_ascii=False),
            encoding="utf-8")
    return res


def batch(paths, outdir):
    outdir = pathlib.Path(outdir)
    outdir.mkdir(parents=True, exist_ok=True)
    n = 0
    for p in paths:
        ported, skipped = port_file(p)
        if not ported:
            continue
        (outdir / f"{p.stem}.c").write_text(emit_c(p.stem, ported, skipped),
                                             encoding="utf-8")
        n += 1
    print(f"batch: wrote {n} files to {outdir}")


def collect_rs(roots):
    out = []
    for r in roots:
        rp = pathlib.Path(r)
        if rp.is_file():
            out.append(rp); continue
        for p in sorted(rp.rglob("*.rs")):
            parts = set(p.parts)
            if "helpers" in parts or p.name == "mod.rs" or p.name == "run_all.rs":
                continue
            out.append(p)
    return out


def link_and_run(cfile, exe=None):
    """Build in an ISOLATED tree (never bare `make`: shared src/**/*.o)."""
    tree = ISOBUILD / "engine_c"
    subprocess.run(["bash", str(EC / "tools" / "isolated_build.sh"), "xpile", "all"],
                   cwd=str(EC), check=True)
    mk = (EC / "Makefile").read_text(encoding="utf-8")
    m = re.search(r'^OBJ_LIB\s*:=\s*(.*)$', mk, re.M)
    objs = m.group(1).split()
    out = exe or str(ISOBUILD / (cfile.stem + ".exe"))
    cmd = ["gcc", "-std=c11", "-O2", "-Wall", "-Iinclude", "-Isrc",
           "-Isrc/core/generated", "-o", out] + objs + [str(cfile)]
    subprocess.run(cmd, cwd=str(tree), check=True)
    print(f"[link] {out}")
    r = subprocess.run([out], cwd=str(tree))
    return r.returncode


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("rs", nargs="*", help="Rust test file(s) or directory(ies)")
    ap.add_argument("--run", action="store_true", help="link + execute the ported C")
    ap.add_argument("--install", action="store_true",
                    help="also write tests/test_xpile_<stem>.c (Makefile-visible)")
    ap.add_argument("--audit", action="store_true", help="coverage report only")
    ap.add_argument("--batch", action="store_true", help="port every .rs under the roots")
    ap.add_argument("--out", default=str(OUTDIR))
    ap.add_argument("--report", default=None)
    ap.add_argument("--json", dest="jsonout", default=None)
    a = ap.parse_args()

    if not a.rs:
        a.rs = [str(RUST_TESTS)]
    paths = collect_rs(a.rs)
    if not paths:
        raise SystemExit("port_one: no Rust test files found")

    if a.audit:
        audit(paths, a.report, a.jsonout)
        return

    if a.batch:
        batch(paths, a.out)
        return

    outdir = pathlib.Path(a.out)
    outdir.mkdir(parents=True, exist_ok=True)
    for p in paths[:1]:
        ported, skipped = port_file(p)
        c = emit_c(p.stem, ported, skipped)
        cf = outdir / f"{p.stem}.c"
        cf.write_text(c, encoding="utf-8")
        checks = sum(x.n_check for _n, x in ported)
        todos = sum(x.n_todo for _n, x in ported)
        print(f"wrote {cf}  fns={len(ported)}/{len(ported)+len(skipped)} "
              f"checks={checks} todos={todos}")
        for n, why in skipped:
            print(f"  SKIP {n}: {why}")
        if a.install:
            tgt = EC / "tests" / f"test_xpile_{p.stem}.c"
            tgt.write_text(c, encoding="utf-8")
            print(f"installed {tgt}  (make t T=xpile_{p.stem})")
        if a.run and ported:
            link_and_run(cf)


if __name__ == "__main__":
    main()
