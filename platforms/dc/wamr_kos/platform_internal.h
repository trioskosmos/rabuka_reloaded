/* Minimal WAMR platform shim for KallistiOS (Dreamcast / SH-4).
 *
 * Single-threaded classic-interpreter build: no thread manager, no JIT/AOT,
 * no hardware-trap bounds checks (SH-4 has no usable signal-based traps
 * under KOS). Sync primitives are no-ops; the game is turn-based and the
 * engine runs entirely on one thread.
 */
#ifndef _PLATFORM_INTERNAL_H_KOS
#define _PLATFORM_INTERNAL_H_KOS

#include <inttypes.h>
#include <stdbool.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <ctype.h>
#include <limits.h>
#include <setjmp.h>
#include <errno.h>

#ifndef BH_PLATFORM_KOS
#define BH_PLATFORM_KOS
#endif

#define BH_APPLET_PRESERVED_STACK_SIZE (32 * 1024)
#define BH_THREAD_DEFAULT_PRIORITY 0

/* no-op sync types (single-threaded) */
typedef int korp_tid;
typedef struct { int dummy; } korp_mutex;
typedef struct { int dummy; } korp_cond;
typedef int korp_thread;
typedef struct { int dummy; } korp_rwlock;
typedef struct { int dummy; } korp_sem;

#define OS_THREAD_MUTEX_INITIALIZER { 0 }

#define os_thread_local_attribute

typedef int bh_socket_t;

typedef jmp_buf korp_jmpbuf;
#define os_setjmp setjmp
#define os_longjmp longjmp
#define os_alloca alloca

#define os_getpagesize() 4096

#define os_printf printf
#define os_vprintf vprintf

typedef int os_file_handle;
typedef void *os_dir_stream;
typedef int os_raw_file_handle;
typedef int os_poll_file_handle;
typedef unsigned int os_nfds_t;
typedef int os_timespec;

static inline os_file_handle
os_get_invalid_handle(void)
{
    return -1;
}

#endif /* _PLATFORM_INTERNAL_H_KOS */
