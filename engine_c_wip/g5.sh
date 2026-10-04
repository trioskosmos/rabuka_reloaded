cd 'C:/Users/trios/AppData/Local/Temp/kilo/w0/after/engine_c_wip'
OBJ_LIB=$(sed -n 's/^OBJ_LIB *:= *//p' Makefile)
echo "--- compile ---"
gcc -std=c11 -O2 -Wall -Wextra -Iinclude -Isrc -Isrc/core/generated -o rh.exe $OBJ_LIB tests/test_parity_required_hearts.c 2>&1 | grep -E 'error|defined but not used' | head
gcc -std=c11 -O2 -Wall -Wextra -o rv.exe tools/verdict_runner.c 2>&1 | head
echo "--- run through the runner ---"
./rv.exe ./rh.exe parity_required_hearts > tests/parity_required_hearts.log 2>&1
echo "rc=$?"
tail -1 tests/parity_required_hearts.log
echo "premise failures: $(grep -c '^FAIL: premise:' tests/parity_required_hearts.log || true)"
grep '^FAIL: premise:' tests/parity_required_hearts.log | head -4
rm -f rh.exe rv.exe
