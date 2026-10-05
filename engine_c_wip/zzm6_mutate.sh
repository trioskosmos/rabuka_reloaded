#!/bin/sh
# Mutation test: temporarily restore the OLD cap read, rebuild, run the same
# sweep, then put the fix back. Proves which numbers my util.c edit moved.
set -e
F=src/ability/util.c
cp "$F" /tmp/zz_util_fixed.c
python3 - <<'PY'
p='src/ability/util.c'
s=open(p,encoding='utf-8',newline='').read()
new='    if (max_str && !strcmp(max_str, "true") && effect->count >= 0) {\n        int cap = effect->count;\n        if (units > cap) units = cap;\n    }\n'
old='    if (max_str && !strcmp(max_str, "true")) {\n        const char *cap_str = eff_extra(effect, "count");\n        if (cap_str) {\n            int cap = atoi(cap_str);\n            if (units > cap) units = cap;\n        }\n    }\n'
assert new in s, "fixed form not found"
open(p,'w',encoding='utf-8',newline='').write(s.replace(new,old))
print("reverted to OLD cap read")
PY
rm -f src/ability/util.o
make -j8 >/dev/null 2>&1 || { echo "BUILD FAILED"; cp /tmp/zz_util_fixed.c "$F"; exit 1; }
echo "rebuilt with OLD cap"