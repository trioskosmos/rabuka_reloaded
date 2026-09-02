import re
p = 'tools/gen_tests.py'
src = open(p, encoding='utf-8').read()
old = '''            exp = hbody
            # A helper may contain the test crate's own setup lines that the
            # line-based transpiler cannot emit.  The transpiler's own rules
            # turn `load_real_database` into a no-op comment and
            # `let mut game = TestGame::new(...)` into
            # `TestGame tg; test_game_new(&tg);`, so we can keep the helper
            # and just drop those lines here.
            kept = []
            for hl in exp.split('\\n'):
                hs = hl.strip()
                if 'load_real_database' in hs:
                    continue
                # A helper may leave a bare `TestGame::new(db)` expression
                # statement behind after stripping the `let` binding —
                # drop it rather than emit broken C.
                if re.match(r'^TestGame::new\\s*\\(', hs):
                    continue
                # `db.clone()` / `db` references that survive after stripping
                # `let db = load_real_database();` — drop them too.
                if re.match(r'^db(\\.clone\\(\\))?\\s*$', hs):
                    continue
                # Keep `let mut game = TestGame::new(...)` — the transpiler's
                # own rule rewrites it to `TestGame tg; test_game_new(&tg);`,
                # binding the helper's game to the caller's `tg`.
                if re.match(r'\\s*let\\s+(?:mut\\s+)?game\\s*=\\s*TestGame::new\\s*\\(', hs):
                    kept.append(hl); continue
            exp = '\\n'.join(kept)'''
new = '''            exp = hbody
            # Drop only lines that belong to the test crate's own setup and
            # would otherwise break the C transpiler.  Do NOT filter the whole
            # body — a plain helper like fill_decks has no such lines and
            # dropping them all would silently delete the helper's work.
            kept = []
            for hl in exp.split('\\n'):
                hs = hl.strip()
                if 'load_real_database' in hs:
                    continue
                # A helper may leave a bare `TestGame::new(db)` expression
                # statement behind after stripping the `let` binding —
                # drop it rather than emit broken C.
                if re.match(r'^TestGame::new\\s*\\(', hs):
                    continue
                # `db.clone()` / `db` references that survive after stripping
                # `let db = load_real_database();` — drop them too.
                if re.match(r'^db(\\.clone\\(\\))?\\s*$', hs):
                    continue
                kept.append(hl)
            exp = '\\n'.join(kept)'''
assert old in src, 'old not found'
src = src.replace(old, new)
open(p, 'w', encoding='utf-8').write(src)
import py_compile; py_compile.compile(p, doraise=True); print('ok')