/* KallistiOS implementation of the WAMR platform API (minimal subset).
 * Single-threaded classic interpreter; malloc-backed "mmap"; no-op locks. */

/* KOS's arch/types.h typedefs int8/uint16/... which clash with WAMR's
 * platform_common.h typedefs (char vs signed char). Rename them away for
 * the kos.h include only — consistent within this TU, and C linkage is
 * unaffected. */
#define int8    kos_int8
#define uint8   kos_uint8
#define int16   kos_int16
#define uint16  kos_uint16
#define int32   kos_int32
#define uint32  kos_uint32
#define int64   kos_int64
#define uint64  kos_uint64
#include <kos.h>
#undef int8
#undef uint8
#undef int16
#undef uint16
#undef int32
#undef uint32
#undef int64
#undef uint64

#include "platform_api_vmcore.h"
#include "platform_api_extension.h"

int
bh_platform_init(void)
{
    return 0;
}

void
bh_platform_destroy(void)
{
}

/* ---- allocator (system allocator mode) ---- */

void *
os_malloc(unsigned size)
{
    return malloc(size);
}

void *
os_realloc(void *ptr, unsigned size)
{
    return realloc(ptr, size);
}

void
os_free(void *ptr)
{
    free(ptr);
}

/* ---- mmap emulation over malloc (linear memory backing) ---- */

#define KOS_MMAP_MAGIC 0x57414D52u /* 'WAMR' */

typedef struct mmap_hdr {
    uint32_t magic;
    uint32_t size;
} mmap_hdr;

static mmap_hdr *
hdr_of(void *addr)
{
    return (mmap_hdr *)((uint8_t *)addr - sizeof(mmap_hdr));
}

void *
os_mmap(void *hint, size_t size, int prot, int flags, int fd)
{
    (void)hint; (void)prot; (void)flags; (void)fd;
    mmap_hdr *h = malloc(sizeof(mmap_hdr) + size);
    if (!h)
        return NULL;
    h->magic = KOS_MMAP_MAGIC;
    h->size = (uint32_t)size;
    return (void *)(h + 1);
}

void
os_munmap(void *addr, size_t size)
{
    (void)size;
    if (!addr)
        return;
    mmap_hdr *h = hdr_of(addr);
    if (h->magic == KOS_MMAP_MAGIC)
        free(h);
}

int
os_mprotect(void *addr, size_t size, int prot)
{
    (void)addr; (void)size; (void)prot;
    return 0;
}

bool
os_mremap_slow_fixup(void *mem, void *old_addr, size_t old_size,
                     void *new_addr, size_t new_size)
{
    (void)mem; (void)old_addr; (void)old_size; (void)new_addr; (void)new_size;
    return true;
}

void *
os_mremap(void *old_addr, size_t old_size, size_t new_size)
{
    /* linear memory only ever grows: allocate-fresh-and-copy is fine */
    void *new_addr = os_mmap(NULL, new_size, 0, 0, -1);
    if (!new_addr)
        return NULL;
    memcpy(new_addr, old_addr, old_size < new_size ? old_size : new_size);
    os_munmap(old_addr, old_size);
    return new_addr;
}

/* ---- time (only used for diagnostics/timers) ---- */

uint64
os_time_get_boot_us(void)
{
    return (uint64)timer_ms_gettime64() * 1000ull;
}

uint64
os_time_thread_cputime_us(void)
{
    return os_time_get_boot_us();
}

/* ---- cache ops (no JIT/AOT: no-ops, but AOT loader may reference) ---- */

void
os_dcache_flush(void)
{
}

void
os_icache_flush(void *start, size_t len)
{
    (void)start; (void)len;
}

/* ---- single-threaded stubs (referenced by shared code paths) ---- */

int
os_mutex_init(korp_mutex *m)
{
    m->dummy = 0;
    return 0;
}

int
os_mutex_lock(korp_mutex *m)
{
    (void)m;
    return 0;
}

int
os_mutex_unlock(korp_mutex *m)
{
    (void)m;
    return 0;
}

int
os_mutex_destroy(korp_mutex *m)
{
    (void)m;
    return 0;
}

korp_tid
os_self_thread(void)
{
    return 1;
}

int
os_thread_join(korp_thread tid, void **ret_val)
{
    (void)tid; (void)ret_val;
    return -1;
}

uint8_t *
os_thread_get_stack_boundary(void)
{
    return NULL;
}

/* referenced by wasm_runtime_invoke_c_api_native; the C-API trap object
 * is never created in this embed mode, so deletion is a no-op */
void
wasm_trap_delete(void *trap)
{
    (void)trap;
}
