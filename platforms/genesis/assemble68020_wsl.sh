#!/bin/bash
# Retry assembly with 68020 permissions; count pass/fail + error classes.
export PATH=/usr/bin:/bin
cd /root/gen-target-asm/linked
ok=0
fail=0
: > as_errors20.log
find /root/gen-target-asm/m68k-unknown-none-elf/release -name '*.s' | while IFS= read -r f; do
    if /usr/bin/m68k-linux-gnu-as -march=68020 "$f" -o "$OUT/$(basename "$f").o" 2>>as_errors20.log; then
        echo ok >> tally.txt
    else
        echo fail >> tally.txt
    fi
done
echo "ok=$(grep -c ok tally.txt) fail=$(grep -c fail tally.txt)"
echo "--- error classes ---"
grep -o 'Error:[^-]*' as_errors20.log | sort | uniq -c | sort -rn | head -8
