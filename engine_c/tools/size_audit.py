#!/usr/bin/env python3
"""size_audit.py — Rust-vs-C gap analysis for the engine_c port.

`tools/dep_audit.py` finds stub *bodies* inside the C that exists. This tool finds
the complementary gap: **Rust functions that have no C equivalent at all**. It maps
each compiled C file to its Rust twin(s), then reports:

  1. Per-file C/Rust line ratio (non-blank, non-comment) — how much of the Rust
     twin's bulk is present in C.
  2. Function-count delta (Rust fns − C fns) — an *estimate* of how many Rust
     functions are simply absent from C.
  3. A best-effort name-level gap list (heuristic — verify manually).

This is what answers "most functions are not present at all": a C file at 10% of
its Rust twin's line count, or with 30 fewer functions, is missing the majority
of its logic regardless of how many stub functions it defines.

Run:
    python3 tools/size_audit.py [--rust ../engine/src] [--report SIZE_AUDIT.md]
"""
import os, re, sys

ROOT = "."
RUST_DEFAULT = "../engine/src"

# C file (relative to engine_c) -> Rust twin(s) (relative to RUST root).
MAPPING = {
    "src/ability/vm.c": ["ability/vm.rs", "ability/effect_decoder_gen.rs",
                          "ability/condition_decoder_gen.rs"],
    "src/ability/condition.c": ["ability/condition.rs", "ability/condition/card.rs",
                                 "ability/condition/compound.rs", "ability/condition/state.rs"],
    "src/ability/choice.c": ["ability/choice.rs"],
    "src/ability/compound.c": ["ability/compound.rs"],
    "src/ability/ability_queue.c": ["ability_queue.rs", "triggers.rs"],
    "src/ability/dynamic_count.c": ["ability/dynamic_count.rs"],
    "src/ability/util.c": ["ability/util.rs"],
    "src/ability/cost.c": ["ability/cost.rs"],
    "src/ability/resolver.c": ["ability/resolver.rs"],
    "src/ability/effects/move.c": ["ability/move_cards.rs"],
    "src/ability/effects/look.c": ["ability/look.rs"],
    "src/ability/effects/draw.c": ["ability/effects/draw.rs"],
    "src/ability/effects/misc.c": ["ability/effects/misc.rs"],
    "src/ability/effects/ability.c": ["ability/effects/ability_effects.rs"],
    "src/ability/effects/state.c": ["ability/effects/state.rs", "ability/effects/misc.rs"],
    "src/ability/effects/score.c": ["ability/effects/score.rs"],
    "src/core/card.c": ["core/card.rs"],
    "src/core/data.c": ["core/mod.rs", "core/types.rs"],
    "src/core/alloc.c": ["core/pool.rs"],
    "src/core/modifiers.c": ["core/game_modifiers.rs", "core/modifiers.rs"],
    "src/core/stats_pipeline.c": ["core/stats_pipeline.rs"],
    "src/core/game_state_abilities.c": ["core/game_state/abilities.rs"],
    "src/core/tracking.c": ["core/game_state/tracking.rs"],
    "src/core/zones.c": ["core/zones.rs", "core/player.rs"],
    "src/turn/live.c": ["turn/live.rs"],
    "src/turn/phase.c": ["turn/phases.rs"],
    "src/turn/triggers.c": ["turn/triggers.rs"],
    "src/engine.c": ["main.rs", "lib.rs", "turn/mod.rs", "core/game.rs"],
}

# words that cause false matches in name-gap scan (too generic across the
# Rust/C naming conventions). Excluded from token matching.
NAME_STOP = {
    "fn","pub","self","mut","async","unsafe","const","new","default","clone",
    "fmt","drop","eq","partialeq","hash","from","into","to","str","string",
    "ref","arc","rc","boxed","option","result","vec","vecdeque","hashmap",
    "btreemap","iter","iterator","builder","with","get","set","is","has",
    "check","do","run","make","create","update","apply","handle","on","for",
    "the","a","an","of","and","or","as","impl","trait","mod",
}

def strip_c(text):
    out=[]; i=0; n=len(text)
    while i<n:
        c=text[i]
        if c=='/' and i+1<n and text[i+1]=='/':
            j=text.find('\n',i);  out.append(' '*(j-i if j>=0 else 0)); i=(j if j>=0 else n); continue
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
                if text[i]==c: i+=1; break
                i+=1
            continue
        out.append(c); i+=1
    return ''.join(out)

def rust_strip(text):
    out=[]; i=0; n=len(text)
    while i<n:
        c=text[i]
        if c=='/' and i+1<n and text[i+1]=='/':
            j=text.find('\n',i); out.append(' '*(j-i if j>=0 else 0)); i=(j if j>=0 else n); continue
        if c=='/' and i+1<n and text[i+1]=='*':
            j=text.find('*/',i+2)
            if j<0: j=n-1
            out.append(' '*(j+2-i)); i=j+2; continue
        if c=='"':
            out.append(c); i+=1
            while i<n:
                out.append(text[i])
                if text[i]=='\\':
                    if i+1<n: out.append(text[i+1]); i+=2; continue
                if text[i]=='"': i+=1; break
                i+=1
            continue
        out.append(c); i+=1
    return ''.join(out)

def nonblank_lines(text):
    return [l for l in text.split('\n') if l.strip()]

def c_function_names(text):
    from dep_audit import find_functions
    fns=find_functions(text)
    return [name for name,_ in fns]

def rust_function_names(text):
    lines=text.split('\n')
    out=[]; prev=""
    for ln in lines:
        s=ln.strip()
        if '#[test]' in s or 'cfg(test' in s:
            prev=s; continue
        m=re.search(r'(?:^|[^A-Za-z_])fn\s+([A-Za-z_]\w*)', ln)
        if m:
            if 'test' in prev:
                prev=""; continue
            out.append(m.group(1))
        prev=s
    return out

def tokens(name):
    s=re.sub(r'(?<=[a-z0-9])(?=[A-Z])','_',name)
    s=re.sub(r'(?<=[A-Z])(?=[A-Z][a-z])','_',s)
    parts=re.split(r'[_\s]+', s.lower())
    return [p for p in parts if p and p not in NAME_STOP]

# Rust-only helpers that by construction have no direct C twin (constructors,
# trait glue, iterators). Excluded from the gap estimate to avoid inflation.
RUST_ONLY = {
    "new","default","clone","fmt","drop","eq","partialeq","hash","parse","from_str",
    "to_string","to_owned","build","boxed","as_ref","as_mut","into","from","try_from",
    "deref","deref_mut","index","into_iter","iter","next","size_hint","debug","display",
    "cmp","ord","partial_cmp","borrow","to_vec","spawn","run","main","default_impl",
    "default_method","into_inner","as_slice","as_mut_slice","len","is_empty","clear",
    "push","pop","extend","reserve","capacity","get_mut","get_ref","entry","default_",
}
def is_rust_only(name):
    n=name.lower()
    if n in RUST_ONLY: return True
    for p in ("__","test_","with_","into_","as_","to_","from_","new_","get_","set_"):
        if n.startswith(p): return True
    return False

def names_match(rust_name, c_tokens_by_fn):
    rt=set(tokens(rust_name))
    if not rt: return False
    for ct in c_tokens_by_fn:
        if not ct: continue
        inter=rt & ct
        if inter and (rt==ct or len(inter)/len(rt|ct) >= 0.34 or rt<=ct or ct<=rt):
            return True
        # bridge naming drift: eval<->evaluate, cond<->condition, blade<->blade_heart
        for a in rt:
            if len(a) < 4: continue
            for b in ct:
                if len(b) >= 4 and (a in b or b in a):
                    return True
    return False

def main():
    rust_root=RUST_DEFAULT
    report_path="SIZE_AUDIT.md"
    args=sys.argv[1:]
    for a in args:
        if a.startswith("--rust"):
            rust_root=a.split("=",1)[1] if "=" in a else args[args.index(a)+1]
        elif a.startswith("--report"):
            report_path=a.split("=",1)[1] if "=" in a else args[args.index(a)+1]

    # determine compiled C files from Makefile SRC
    src_list=[]
    mk=os.path.join(ROOT,"Makefile")
    if os.path.exists(mk):
        with open(mk,encoding="utf-8",errors="replace") as f:
            for line in f:
                if line.strip().startswith("SRC") and ":=" in line:
                    src_list=line.split(":=")[1].split(); break
    c_files=[t for t in src_list if t.endswith(".c") and os.path.basename(t) not in
             ("main.c","test_game.c","debug_umi.c")]

    rows=[]
    tot_c=0; tot_r=0; tot_cf=0; tot_rf=0; tot_rf_port=0
    name_gap=[]
    for cf in c_files:
        if cf not in MAPPING: continue
        cp=os.path.join(ROOT,cf)
        if not os.path.exists(cp): continue
        with open(cp,encoding="utf-8",errors="replace") as f: ctext=f.read()
        cstrip=strip_c(ctext)
        c_lines=len(nonblank_lines(cstrip))
        c_fns=c_function_names(cstrip)
        c_tok={fn:set(tokens(fn[3:] if fn.startswith("rb_") else fn)) for fn in c_fns}

        rust_files=MAPPING[cf]
        r_lines=0; r_fns=[]
        for rf in rust_files:
            rp=os.path.join(rust_root,rf)
            if not os.path.exists(rp): continue
            with open(rp,encoding="utf-8",errors="replace") as f: rtext=f.read()
            rstrip=rust_strip(rtext)
            r_lines+=len(nonblank_lines(rstrip))
            r_fns+=rust_function_names(rstrip)
        # dedupe rust fn names per file group
        r_fns=list(dict.fromkeys(r_fns))
        # drop Rust-only helpers (constructors / trait glue) so the gap isn't inflated
        r_fns_port=[n for n in r_fns if not is_rust_only(n)]
        cf_count=len(c_fns); rf_count=len(r_fns); rf_port=len(r_fns_port)
        ratio = (100*c_lines//r_lines) if r_lines else -1
        delta = rf_port - cf_count
        tot_c+=c_lines; tot_r+=r_lines; tot_cf+=cf_count; tot_rf+=rf_count; tot_rf_port+=rf_port
        rows.append((cf, c_lines, r_lines, ratio, cf_count, rf_port, delta, rust_files))

        # name-gap: rust port-target fns with no confident C twin
        for rn in r_fns_port:
            if not names_match(rn, list(c_tok.values())):
                name_gap.append((cf, rn))

    rows.sort(key=lambda r:(r[6] if r[6]>0 else -1), reverse=True)

    lines=[]
    w=lambda s="": lines.append(s)
    w("# engine_c — Rust↔C Size / Function Gap Audit")
    w()
    w("Generated by `tools/size_audit.py`. This answers the **\"functions not present")
    w("at all\"** question that `dep_audit.py` (stub bodies) cannot: for each compiled")
    w("C file, how much of its Rust twin's bulk and how many of its functions are")
    w("actually present in C.")
    w()
    w("## Summary")
    w()
    w(f"- C files audited (mapped to a Rust twin): {len(rows)}")
    w(f"- Total C lines (non-blank/comment): {tot_c}   Total Rust lines: {tot_r}")
    w(f"- Total C functions: {tot_cf}   Total Rust functions (all): {tot_rf}")
    w(f"- Total Rust **port-target** functions (excl. constructors/trait glue): {tot_rf_port}")
    w(f"- **Function-count gap (Rust port-targets − C, summed): {tot_rf_port - tot_cf}** "
      f"(approx. unported functions)")
    w(f"- Best-effort unmatched Rust function names: {len(name_gap)} (heuristic)")
    w()
    w("## Per-file gap (sorted by missing-function count, worst first)")
    w()
    w("| C file | C lines | Rust lines | C/Rust % | C fns | Rust port fns | missing ≈ | Rust twin(s) |")
    w("| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |")
    for cf,cl,rl,ratio,cfc,rfc,delta,rusts in rows:
        rusts_s=", ".join(os.path.basename(r) for r in rusts)
        w(f"| `{cf}` | {cl} | {rl} | {ratio}% | {cfc} | {rfc} | {delta} | {rusts_s} |")
    w()
    w("## Best-effort unmatched Rust function names (heuristic — verify manually)")
    w()
    w("Token-overlap match against C `rb_*` names. High recall, imperfect precision:")
    w("some listed names DO have a C twin under a different name. Use as an")
    w("investigation starting point, not gospel.")
    w()
    # group by C file
    by_c={}
    for cf,rn in name_gap: by_c.setdefault(cf,[]).append(rn)
    for cf in sorted(by_c, key=lambda k:-len(by_c[k])):
        w(f"- `{cf}` ({len(by_c[cf])} unmatched): " + ", ".join(f"`{n}`" for n in by_c[cf][:60]))
        if len(by_c[cf])>60: w(f"  _...and {len(by_c[cf])-60} more_")
    w()

    out="\n".join(lines)
    with open(report_path,"w",encoding="utf-8") as f: f.write(out)
    print(f"Wrote {report_path}")
    print(f"files={len(rows)} C_lines={tot_c} Rust_lines={tot_r} "
          f"C_fns={tot_cf} Rust_port_fns={tot_rf_port} gap={tot_rf_port-tot_cf} "
          f"unmatched_names={len(name_gap)}")
    print("Worst file gaps (missing fns):")
    for cf,cl,rl,ratio,cfc,rfc,delta,rusts in rows[:8]:
        if delta>0: print(f"  {cf}: C={cfc} Rust={rfc} missing≈{delta} ({ratio}%)")

if __name__=="__main__":
    main()
