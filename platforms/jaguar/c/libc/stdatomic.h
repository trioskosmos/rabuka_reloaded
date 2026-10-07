#ifndef LIBCC_STDATOMIC_H
#define LIBCC_STDATOMIC_H

#include <stdbool.h>

/* Single-threaded bare metal: every atomic is a plain
   volatile access. The engine's only atomic is the
   ability-debug flag (ability/debug.c), set from the
   shell and read in the VM -- no interrupt handler or
   second core touches it, so relaxed and seq_cst are
   the same thing here. */
typedef volatile _Bool atomic_bool;
typedef volatile int atomic_int;
typedef volatile unsigned atomic_uint;
typedef volatile long atomic_long;
typedef volatile unsigned long atomic_ulong;
typedef volatile long long atomic_llong;
typedef volatile unsigned long long atomic_ullong;

#define ATOMIC_VAR_INIT(v) (v)

typedef enum {
    memory_order_relaxed,
    memory_order_consume,
    memory_order_acquire,
    memory_order_release,
    memory_order_acq_rel,
    memory_order_seq_cst
} memory_order;

#define atomic_load_explicit(obj, order) (*(obj))
#define atomic_store_explicit(obj, val, order) (*(obj) = (val))
#define atomic_load(obj) (*(obj))
#define atomic_store(obj, val) (*(obj) = (val))
#define atomic_exchange_explicit(obj, val, order) \
    __atomic_exchange_n(obj, val, 0)
#define atomic_fetch_add_explicit(obj, val, order) \
    __atomic_fetch_add(obj, val, 0)
#define atomic_fetch_sub_explicit(obj, val, order) \
    __atomic_fetch_sub(obj, val, 0)
#define atomic_fetch_or_explicit(obj, val, order) \
    __atomic_fetch_or(obj, val, 0)
#define atomic_fetch_and_explicit(obj, val, order) \
    __atomic_fetch_and(obj, val, 0)
#define atomic_fetch_xor_explicit(obj, val, order) \
    __atomic_fetch_xor(obj, val, 0)

typedef volatile int atomic_flag;
#define ATOMIC_FLAG_INIT { 0 }
#define atomic_flag_clear_explicit(f, order) (*(f) = 0)
#define atomic_flag_test_and_set_explicit(f, order) \
    __atomic_test_and_set(f, 0)

#endif
