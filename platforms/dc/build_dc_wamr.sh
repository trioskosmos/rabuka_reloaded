#!/bin/bash
# Dreamcast build — WAMR classic interpreter pipeline.
#
# rust -> wasm32 -> [embed .wasm as blob] + WAMR interpreter (C) -> sh-elf-gcc
# No wasm2c: target-side code is the interpreter (~100-200KB) instead of
# ~3-4MB of transpiled engine. The 2MB wasm blob is data, loaded at boot.
#
# WAMR core is compiled against PURE NEWLIB (-nostdinc, no KOS headers):
# KOS's arch/types.h typedefs int8/uint16/etc. and clashes with WAMR's
# platform_common.h typedefs (char vs signed char). The WAMR core needs
# nothing from KOS; the shell (kos.h) links it all back together.
#
# Requires: /root/kos + /root/sh-elf toolchain, /root/wamr clone,
# wasm at /mnt/c/rust_targets/wasm32-unknown-unknown/release/rabuka_wasm.wasm
set -e
source /root/kos/environ.sh >/dev/null

WAMR=/root/wamr
DCBUILD=/root/dcbuild-wamr
REPO=/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/platforms/dc
WASM_SRC=/mnt/c/rust_targets/wasm32-unknown-unknown/release/rabuka_wasm.wasm

SH_GCC=/root/sh-elf/bin/sh-elf-gcc
GCC_INC=$(/root/sh-elf/bin/sh-elf-gcc -print-file-name=include)
GCC_FIXED=$(/root/sh-elf/bin/sh-elf-gcc -print-file-name=include-fixed)
NEWLIB_INC=/root/sh-elf/sh-elf/include

mkdir -p "$DCBUILD"
cd "$DCBUILD"
cp "$REPO/wasm/dc_main_wamr.c" .
cp "$REPO/wasm/runtime/sjis_table.c" "$REPO/wasm/runtime/sjis_table.h" .
cp "$REPO/wamr_kos/platform_internal.h" "$REPO/wamr_kos/kos_platform.c" .
cp "$REPO/wamr_kos/wasm_blob.S" .

# embed the wasm module via .incbin (compiled with the same -ml flags as
# the rest of the build; objcopy-produced blobs get rejected by ld on
# endianness merge)
cp "$WASM_SRC" rabuka_wasm.wasm

# SH-4 CPU flags shared by both compile modes (from kos environ_dreamcast.sh)
CPUFLAGS="-m4-single -ml -mfsrra -mfsca -ffunction-sections -fdata-sections \
-matomic-model=soft-imask -std=gnu99"

# pure-newlib include set for WAMR core (NO kos headers)
NINC="-nostdinc -I$GCC_INC -I$GCC_FIXED -I$NEWLIB_INC \
-I$DCBUILD \
-I$WAMR/core/iwasm/include \
-I$WAMR/core/iwasm/interpreter \
-I$WAMR/core/iwasm/common \
-I$WAMR/core/iwasm/libraries/libc-builtin \
-I$WAMR/core/shared/platform/include \
-I$WAMR/core/shared/mem-alloc \
-I$WAMR/core/shared/utils"

DEFS="-DWASM_ENABLE_INTERP=1 -DWASM_ENABLE_FAST_INTERP=1 \
-DWASM_ENABLE_AOT=0 -DWASM_ENABLE_JIT=0 -DWASM_ENABLE_FAST_JIT=0 \
-DWASM_ENABLE_SIMD=0 -DWASM_ENABLE_GC=0 -DWASM_ENABLE_STRINGREF=0 \
-DWASM_ENABLE_LIBC_BUILTIN=1 -DWASM_ENABLE_LIBC_WASI=0 \
-DWASM_ENABLE_MULTI_MODULE=0 -DWASM_ENABLE_THREAD_MGR=0 \
-DWASM_ENABLE_SHARED_MEMORY=0 -DWASM_ENABLE_MINI_LOADER=0 \
-DWASM_ENABLE_BULK_MEMORY=1 -DWASM_ENABLE_BULK_MEMORY_OPT=1 \
-DWASM_ENABLE_REF_TYPES=1 -DWASM_ENABLE_CALL_INDIRECT_OVERLONG=1 \
-DWASM_ENABLE_EXTENDED_CONST_EXPR=0 -DWASM_ENABLE_QUICK_AOT_ENTRY=0 \
-DWASM_DISABLE_HW_BOUND_CHECK=1 -DWASM_DISABLE_STACK_HW_BOUND_CHECK=1 \
-DWASM_DISABLE_WAKEUP_BLOCKING_OP=1 -DWASM_DISABLE_WRITE_GS_BASE=1 \
-DWASM_ENABLE_AOT_INTRINSICS=0 -DWASM_ENABLE_MODULE_INST_CONTEXT=0 \
-DWASM_GLOBAL_HEAP_SIZE=10485760 \
-DBH_PLATFORM_KOS -DBUILD_TARGET_ARM \
-DBH_MALLOC=wasm_runtime_malloc -DBH_FREE=wasm_runtime_free"

echo "=== [1/4] WAMR interpreter core (pure newlib) ==="
OBJS=""

$SH_GCC $CPUFLAGS -Oz -g0 $NINC $DEFS -c wasm_blob.S -o wasm_blob.o

for f in \
  $WAMR/core/shared/utils/bh_assert.c \
  $WAMR/core/shared/utils/bh_bitmap.c \
  $WAMR/core/shared/utils/bh_common.c \
  $WAMR/core/shared/utils/bh_hashmap.c \
  $WAMR/core/shared/utils/bh_leb128.c \
  $WAMR/core/shared/utils/bh_list.c \
  $WAMR/core/shared/utils/bh_log.c \
  $WAMR/core/shared/utils/bh_queue.c \
  $WAMR/core/shared/utils/bh_vector.c \
  $WAMR/core/shared/utils/runtime_timer.c \
  $WAMR/core/shared/mem-alloc/mem_alloc.c \
  $WAMR/core/shared/mem-alloc/ems/ems_alloc.c \
  $WAMR/core/shared/mem-alloc/ems/ems_gc.c \
  $WAMR/core/shared/mem-alloc/ems/ems_hmu.c \
  $WAMR/core/shared/mem-alloc/ems/ems_kfc.c \
  ; do
  o=$(basename "$f" .c).o
  $SH_GCC $CPUFLAGS -Oz -g0 $NINC $DEFS -c "$f" -o "$o" &
done
wait

# kos_platform.c needs kos.h -> normal kos-cc include world
kos-cc -Oz -g0 $DEFS -I$DCBUILD -I$WAMR/core/iwasm/include \
    -I$WAMR/core/shared/platform/include -c kos_platform.c -o kos_platform.o

for f in \
  $WAMR/core/iwasm/interpreter/wasm_loader.c \
  $WAMR/core/iwasm/interpreter/wasm_interp_fast.c \
  $WAMR/core/iwasm/interpreter/wasm_runtime.c \
  $WAMR/core/iwasm/common/wasm_exec_env.c \
  $WAMR/core/iwasm/common/wasm_loader_common.c \
  $WAMR/core/iwasm/common/wasm_memory.c \
  $WAMR/core/iwasm/common/wasm_native.c \
  $WAMR/core/iwasm/common/wasm_runtime_common.c \
  $WAMR/core/iwasm/common/wasm_shared_memory.c \
  $WAMR/core/iwasm/common/wasm_blocking_op.c \
  $WAMR/core/iwasm/common/arch/invokeNative_general.c \
  $WAMR/core/iwasm/libraries/libc-builtin/libc_builtin_wrapper.c \
  ; do
  o=$(basename "$f" .c).o
  # -O2: the interpreter dispatch loop is the hottest code on the target;
  # size-opt (-Oz) cost measurable speed. Loader is hot too in FI mode
  # (pre-decodes every opcode at load time).
  $SH_GCC $CPUFLAGS -O2 -g0 $NINC $DEFS -c "$f" -o "$o"
done

OBJS="wasm_blob.o bh_assert.o bh_bitmap.o bh_common.o bh_hashmap.o bh_leb128.o \
bh_list.o bh_log.o bh_queue.o bh_vector.o runtime_timer.o mem_alloc.o \
ems_alloc.o ems_gc.o ems_hmu.o ems_kfc.o kos_platform.o wasm_loader.o \
wasm_interp_fast.o wasm_runtime.o wasm_exec_env.o wasm_loader_common.o \
wasm_memory.o wasm_native.o wasm_runtime_common.o wasm_shared_memory.o \
wasm_blocking_op.o invokeNative_general.o libc_builtin_wrapper.o"
echo "interpreter OK"

echo "=== [2/4] shell + sjis table ==="
kos-cc -O2 -I$WAMR/core/iwasm/include -c dc_main_wamr.c -o dc_main_wamr.o
kos-cc -O2 -c sjis_table.c -o sjis_table.o
echo "shell OK"

echo "=== [3/4] link ELF ==="
kos-cc -O2 -o rabuka_dc_wamr.elf dc_main_wamr.o sjis_table.o \
    $OBJS -Wl,--gc-sections
sh-elf-size rabuka_dc_wamr.elf || true

echo "=== [4/4] copy output ==="
mkdir -p "$REPO/output"
cp rabuka_dc_wamr.elf "$REPO/output/"
echo "ALL DONE: $REPO/output/rabuka_dc_wamr.elf"
