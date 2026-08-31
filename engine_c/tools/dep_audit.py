#!/usr/bin/env python3
"""dep_audit.py — honest audit of the engine_c C port.

Parses every function body in src/**/*.c (excluding main.c / debug_*.c /
test_game.c), and decides whether each function actually *does* something or is
a placeholder. This is the antidote to "it's basically done": it measures
substance, not mere existence.

Classification:
  REAL        — mutates state, has control flow, or delegates to a real helper.
  DONE_SMALL  — pure getter / accessor / constant (legitimately tiny).
  STUB_*      — MARKER (TODO/stub note), EMPTY (body is just `return;`),
                NOOP (returns a constant / only calls a clear-and-resume helper
                and mutates nothing — the classic "compiles but does nothing").

It then builds the call graph and prints a BOTTOM-UP work order: stub functions
whose unimplemented callees are themselves all implemented come first (depth 1);
functions that depend on other stubs come later. That is the order to port in.

Run:  python3 tools/dep_audit.py [--report DEPENDENCY_AUDIT.md]
"""
import os, re, sys

ROOT = "."
SRC_DIR = os.path.join(ROOT, "src")
HEADER = os.path.join(ROOT, "include", "rabuka.h")

EXCLUDE = {"main.c", "debug_umi.c", "test_game.c"}

KEYWORDS = {
    "int","char","void","long","short","unsigned","signed","float","double",
    "const","static","struct","enum","typedef","union","return","if","for",
    "while","switch","else","do","sizeof","inline","extern","bool","uint8_t",
    "uint16_t","uint32_t","uint64_t","int8_t","int16_t","int32_t","int64_t",
    "size_t","intptr_t","uintptr_t","ssize_t",
}

# Calls that are "trivial" — a handler whose ONLY work is one of these is a stub
# (it clears/resumes the choice but never applies the player's selection). NOTE:
# rb_drain_ability_queue / rb_resume_with_choice are REAL engine drivers and are
# deliberately NOT here; only the clear-and-resume no-op helpers indicate a stub.
TRIVIAL = {
    "rb_resolver_clear_choice_state_and_resume",
    "rb_resolver_clear_choice_state",
    "rb_resolver_clear_choice_meta",
    "rb_clear_pending_choice",
}

MARKERS = [
    r"\bTODO\b", r"\bSTUB\b", r"\bFIXME\b", r"\bXXX\b",
    r"not yet", r"no-op", r"\bplaceholder\b", r"approximat", r"simplified",
    r"fallback", r"degrade", r"\bassume\b", r"\bunknown\b", r"unsupported",
    r"\bignored\b", r"\bbypass\b", r"\bphantom\b", r"not tracked",
    r"\bstub\b", r"not implemented", r"unimplemented", r"\bNYI\b", r"\bTBD\b",
    r"best-effort", r"skeleton", r"not ported", r"unfinished",
]
MARKER_RE = re.compile("|".join(MARKERS), re.IGNORECASE)

EMPTY_SET = {"", "return 0;", "return 1;", "return;", "return NULL;",
             "return 0", "return 1", "return NULL"}

def strip_c(text):
    out=[]; i=0; n=len(text)
    while i<n:
        c=text[i]
        if c=='/' and i+1<n and text[i+1]=='/':
            j=text.find('\n',i)
            if j<0: j=n
            out.append(' '*(j-i)); i=j; continue
        if c=='/' and i+1<n and text[i+1]=='*':
            j=text.find('*/',i+2)
            if j<0: j=n-1
            out.append(' '*(j+2-i)); i=j+2; continue
        if c=='"' or c=="'":
            out.append(c); i+=1
            while i<n:
                out.append(text[i])
                if text[i]=='\\':
                    if i+1<n: out.append(text[i+1]); i+=2; continue
                if text[i]==c:
                    i+=1; break
                i+=1
            continue
        out.append(c); i+=1
    return ''.join(out)

def skip_ws(t,i):
    while i<len(t) and t[i] in ' \t\r\n': i+=1
    return i

def match_paren(t,i):
    depth=0; n=len(t)
    while i<n:
        c=t[i]
        if c=='(' : depth+=1
        elif c==')':
            depth-=1
            if depth==0: return i
        i+=1
    return n-1

def match_brace(t,i):
    depth=0; n=len(t)
    while i<n:
        c=t[i]
        if c=='{' : depth+=1
        elif c=='}':
            depth-=1
            if depth==0: return i
        i+=1
    return n-1

IDENT_RE = re.compile(r'[A-Za-z_]\w*')
CALL_RE  = re.compile(r'([A-Za-z_]\w*)\s*\(')

def find_functions(text):
    funcs=[]; i=0; n=len(text)
    while i<n:
        c=text[i]
        if c=='{': i=match_brace(text,i)+1; continue
        if c=='}': i+=1; continue
        if c=='(' or c==')': i+=1; continue
        m=IDENT_RE.match(text,i)
        if not m:
            i+=1; continue
        name=m.group(0)
        if name in KEYWORDS:
            i=m.end(); continue
        j=skip_ws(text,m.end())
        if j>=n or text[j]!='(':
            i=m.end(); continue
        close=match_paren(text,j)
        k=skip_ws(text,close+1)
        if k>=n or text[k]!='{':
            i=close+1; continue
        bodyclose=match_brace(text,k)
        funcs.append((name, text[k+1:bodyclose]))
        i=bodyclose+1
    return funcs

def classify(body, callees_defined, defined_set):
    s=body.strip()
    if s in EMPTY_SET:
        return "STUB_EMPTY"
    ctrl = len(re.findall(r'\b(if|for|while|switch|else|case|do)\b', body))
    rb_calls = set(re.findall(r'\brb_[A-Za-z_]\w*\b', body))
    rb_real = [c for c in rb_calls if c not in TRIVIAL]
    state_write = bool(re.search(r'(\w+->\w+|\w+\[\w*\])\s*=', body))
    called_names = {m.group(1) for m in CALL_RE.finditer(body)} - KEYWORDS
    # a call to something we did NOT define (libc / declared helper) = real delegation
    external_calls = called_names - defined_set
    has_call = bool(called_names)
    marker = MARKER_RE.search(body)
    if marker:
        return "STUB_MARKER"
    if not state_write and not has_call:
        # A stub ignores its inputs and returns a hard-coded constant.  Drop the
        # standard `(void)param;` silence casts; if what remains is just
        # `return <const>;` it's a stub, otherwise it derives a value (accessor).
        body_n = re.sub(r'\(void\)\s*\w+\s*;', '', body).strip()
        if re.fullmatch(r'return\s+(0|1|2|3|NULL|-1)\s*;', body_n) or body_n == "return;":
            return "STUB_NOOP"
        return "DONE_SMALL"   # returns a variable/field/func result — legit accessor
    if state_write or ctrl > 1 or rb_real or external_calls:
        return "REAL"
    # has_call but only delegates to TRIVIAL defined helpers, no state write, no flow
    return "STUB_NOOP"

def collect_calls(body, defined):
    return {m.group(1) for m in CALL_RE.finditer(body) if m.group(1) in defined}

def parse_header_decls(path):
    decls=set()
    if not os.path.exists(path): return decls
    with open(path,encoding="utf-8",errors="replace") as f:
        txt=strip_c(f.read())
    for m in re.finditer(r'\brb_[A-Za-z_]\w*\s*\(', txt):
        decls.add(m.group(0)[:-1].strip())
    return decls

def main():
    gen_report = False
    report_path = "DEPENDENCY_AUDIT.md"
    for a in sys.argv[1:]:
        if a.startswith("--report"):
            gen_report=True
            if "=" in a: report_path=a.split("=",1)[1]

    # Only audit files that are actually compiled. The Makefile SRC list is the
    # source of truth — ignore stray *_frag_*.c fragments and other non-built files.
    src_list=[]
    mk=os.path.join(ROOT,"Makefile")
    if os.path.exists(mk):
        with open(mk,encoding="utf-8",errors="replace") as f:
            for line in f:
                if line.strip().startswith("SRC") and ":=" in line:
                    toks=line.split(":=")[1].split()
                    src_list=toks
                    break
    files=[]
    for tok in src_list:
        if not tok.endswith(".c"): continue
        if os.path.basename(tok) in EXCLUDE: continue
        p=os.path.join(ROOT, tok)
        if os.path.exists(p): files.append(p)
    # Fallback: if Makefile parse failed, walk src/ (still excluding EXCLUDE).
    if not files:
        for dp,_,fns in os.walk(SRC_DIR):
            for fn in fns:
                if fn.endswith(".c") and fn not in EXCLUDE:
                    files.append(os.path.join(dp,fn))
    files.sort()

    per_file={}
    defined={}      # name -> path
    for path in files:
        with open(path,encoding="utf-8",errors="replace") as f:
            raw=f.read()
        text=strip_c(raw)
        funcs=find_functions(text)
        recs=[]
        for name,body in funcs:
            defined[name]=path
            recs.append(dict(name=name, body=body))
        per_file[path]=recs

    defined_set=set(defined)
    for path,recs in per_file.items():
        for r in recs:
            r["calls"]=collect_calls(r["body"], defined_set)
            r["cls"]=classify(r["body"], r["calls"], defined_set)
            r["branches"]=len(re.findall(r'\b(if|for|while|switch|else|case|do)\b', r["body"])) + r["body"].count('?')
            r["rb_real"]=len([c for c in re.findall(r'\brb_[A-Za-z_]\w*\b', r["body"]) if c not in TRIVIAL])
            r["assigns"]=len(re.findall(r'(?<![<>=!])=(?!=)', r["body"]))

    # Preprocessor #ifdef/#else often yields two definitions of one name (a fake
    # arena stub + the real libc path). Keep the most-substantive definition.
    # Also drop duplicate name entries within a file's list.
    RANK={"REAL":3,"DONE_SMALL":2,"STUB_NOOP":1,"STUB_MARKER":1,"STUB_EMPTY":0}
    best={}
    for path,recs in per_file.items():
        for r in recs:
            cur=best.get(r["name"])
            if cur is None or RANK[r["cls"]]>RANK[cur["cls"]]:
                best[r["name"]]=r
    for path,recs in per_file.items():
        seen=set()
        uniq=[]
        for r in recs:
            b=best[r["name"]]
            if b["name"] in seen: continue
            seen.add(b["name"]); uniq.append(b)
        recs[:]=uniq
    defined_set=set(best)

    # Header `static inline` helpers (e.g. rb_saturate_u8) live in rabuka.h, not a
    # .c file — register them so they aren't falsely reported as "missing".
    hdr_defs=set()
    for h in [os.path.join(ROOT,"include","rabuka.h")]:
        if os.path.exists(h):
            with open(h,encoding="utf-8",errors="replace") as f:
                for nm,_ in find_functions(strip_c(f.read())):
                    hdr_defs.add(nm)
    defined_set |= hdr_defs

    # Fixpoint: a stub that only *delegates* to an already-real/DONE helper is
    # itself real (e.g. rb_eval_condition_for_host -> eval_condition_inner_host).
    # Propagate so chains of delegators all become REAL.
    REALISH={"REAL","DONE_SMALL"}
    changed=True
    while changed:
        changed=False
        for name,r in best.items():
            if r["cls"]!="STUB_NOOP": continue
            for c in r["calls"]:
                cr=best.get(c)
                if cr and cr["cls"] in REALISH and c not in TRIVIAL:
                    r["cls"]="REAL"; changed=True; break

    decls=parse_header_decls(HEADER)
    missing_declared=sorted(d for d in decls if d not in defined_set)
    all_called=set()
    for _,recs in per_file.items():
        for r in recs: all_called|=r["calls"]
    unresolved=sorted(c for c in all_called if c.startswith("rb_") and c not in defined_set and c not in decls)

    UNIMPL={"STUB_MARKER","STUB_EMPTY","STUB_NOOP"}
    unimpl_names={r["name"] for _,recs in per_file.items() for r in recs if r["cls"] in UNIMPL}

    depth_cache={}
    def depth(name):
        if name in depth_cache: return depth_cache[name]
        if name not in unimpl_names:
            depth_cache[name]=0; return 0
        rec=None
        for _,recs in per_file.items():
            for r in recs:
                if r["name"]==name: rec=r; break
            if rec: break
        uc=[c for c in rec["calls"] if c in unimpl_names]
        d=1 if not uc else 1+max(depth(c) for c in uc)
        depth_cache[name]=d; return d
    for _,recs in per_file.items():
        for r in recs: r["depth"]=depth(r["name"])

    counts={}
    for _,recs in per_file.items():
        for r in recs: counts[r["cls"]]=counts.get(r["cls"],0)+1
    order=[]
    for _,recs in per_file.items():
        for r in recs:
            if r["cls"] in UNIMPL: order.append(r)
    order.sort(key=lambda r:(r["depth"], r["name"]))
    maxd=max((r["depth"] for r in order), default=0)

    lines=[]
    w=lambda s="": lines.append(s)
    w("# engine_c — Dependency / Substance Audit")
    w()
    w("Generated by `tools/dep_audit.py`. This measures **substance**, not "
      "existence: a function that compiles but only returns a constant or just "
      "clears-and-resumes the choice is a stub, not a port. Work the list "
      "**bottom-up** (depth 1 first).")
    w()
    w("## Summary")
    w()
    tot=sum(len(v) for v in per_file.values())
    real=counts.get("REAL",0); small=counts.get("DONE_SMALL",0)
    stub=counts.get("STUB_NOOP",0)+counts.get("STUB_MARKER",0)+counts.get("STUB_EMPTY",0)
    w(f"- Engine `.c` files scanned (excl. main/debug/test_game): {len(per_file)}")
    w(f"- Functions defined: {tot}")
    w(f"- **REAL (substantive):** {real}")
    w(f"- DONE_SMALL (legit tiny getters/accessors): {small}")
    w(f"- **STUB_NOOP (returns constant / only clears+resumes):** {counts.get('STUB_NOOP',0)}")
    w(f"- **STUB_MARKER (TODO/stub note):** {counts.get('STUB_MARKER',0)}")
    w(f"- **STUB_EMPTY (body is just `return;`):** {counts.get('STUB_EMPTY',0)}")
    w(f"- **TOTAL STUBS: {stub} / {tot} ({100*stub//max(tot,1)}%)**")
    w(f"- Declared in `rabuka.h` but never defined (MISSING): {len(missing_declared)}")
    w(f"- Called `rb_*` but neither defined nor declared (UNRESOLVED): {len(unresolved)}")
    w()
    if missing_declared:
        w("## Missing functions (declared in rabuka.h, no body anywhere)")
        w()
        for m in missing_declared: w(f"- `{m}`")
        w()
    if unresolved:
        w("## Unresolved rb_ calls (called but undefined & undeclared)")
        w()
        for u in unresolved: w(f"- `{u}`")
        w()

    w("## Bottom-up work order (implement lowest depth first)")
    w()
    w("**depth** = 1 + max depth of any *unimplemented* callee. depth 1 = "
      "depends only on already-implemented code. Fill depth-1 stubs first; "
      "everything above unblocks as you go.")
    w()
    for d in range(1, maxd+1):
        grp=[r for r in order if r["depth"]==d]
        if not grp: continue
        w(f"### Depth {d} — {len(grp)} stub function(s)")
        w()
        w("| function | file | class | unimplemented callees (blockers) |")
        w("| --- | --- | --- | --- |")
        for r in grp:
            rel=os.path.relpath(defined[r["name"]], ROOT)
            uc=sorted(c for c in r["calls"] if c in unimpl_names)
            blk=", ".join(f"`{c}`" for c in uc) or "—"
            w(f"| `{r['name']}` | {rel} | {r['cls']} | {blk} |")
        w()

    w("## Per-file detail (all functions)")
    w()
    for path in files:
        recs=per_file[path]
        if not recs: continue
        rel=os.path.relpath(path, ROOT)
        w(f"### {rel}")
        w()
        w("| function | class | br | rb* | = | depth |")
        w("| --- | --- | ---: | ---: | ---: | ---: |")
        for r in sorted(recs, key=lambda x:x["name"]):
            w(f"| `{r['name']}` | {r['cls']} | {r['branches']} | {r['rb_real']} | "
              f"{r['assigns']} | {r['depth']} |")
        w()

    out="\n".join(lines)
    if gen_report:
        with open(report_path,"w",encoding="utf-8") as f:
            f.write(out)
        print(f"Wrote {report_path}")
    # console summary
    print("=== CONSOLE SUMMARY ===")
    print(f"files={len(per_file)} total_fns={tot} REAL={real} DONE_SMALL={small} "
          f"STUB_NOOP={counts.get('STUB_NOOP',0)} STUB_MARKER={counts.get('STUB_MARKER',0)} "
          f"STUB_EMPTY={counts.get('STUB_EMPTY',0)}")
    print(f"MISSING(decl,no body)={len(missing_declared)} "
          f"UNRESOLVED(rb_* undef&undecl)={len(unresolved)} max_depth={maxd}")
    print(f"\nDepth-1 stubs (implement first, {sum(1 for r in order if r['depth']==1)}):")
    for r in order:
        if r["depth"]==1:
            print(f"  {r['name']} [{r['cls']}]")
    print(f"\nDepth-2 stubs: {sum(1 for r in order if r['depth']==2)}")
    print(f"Depth-3+ stubs: {sum(1 for r in order if r['depth']>=3)}")

if __name__=="__main__":
    main()
