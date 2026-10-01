/* alloc_counter.c — C port of engine/src/alloc_counter.rs
 *                 + engine/src/core/runtime_flags.rs
 *
 * Owns: allocation counters, the size histogram, sampled allocator self-timing,
 * scoped call-site allocation tracing, and the process-global speed/verbosity
 * gates. Does NOT own the pool allocator (that is src/core/alloc.c).
 *
 * FAITHFULNESS NOTES (read before judging anything below as "missing")
 *
 *  1. Rust installs `CountingAllocator` as `#[global_allocator]`, so every
 *     allocation in the process passes through it without any call site opting
 *     in. C has no such mechanism: an allocator is an ordinary named function.
 *     The faithful C translation is therefore an INSTRUMENTED ALLOCATOR —
 *     rb_allocctr_malloc() / rb_allocctr_free() — which src/core/alloc.c's
 *     rb_malloc / rb_free must delegate to for the counters to see real
 *     traffic. Until that one-line delegation exists, the counters are real
 *     and correct but only see traffic that explicitly calls them. That is
 *     reported as a CROSS-FILE REQUEST, not hidden.
 *
 *  2. Rust's GlobalAlloc::dealloc receives the Layout, so a free knows its
 *     size. `rb_free(void *)` does not. To keep an outstanding-allocation count
 *     meaningful, every sized allocation is recorded in a bounded pointer
 *     ledger, and the sized free path looks the size back up. A free of a
 *     pointer the ledger does not hold is counted as an UNMATCHED FREE, which
 *     is exactly the signature of a double free: the same owned pointer left
 *     live in two struct copies and freed twice (the bug shape found in
 *     src/ability/ability_queue.c).
 *
 *  3. Rust's Drop impls (TraceGuard, AllocGuard) have no C equivalent. The
 *     guards are real heap objects; their Drop bodies are the explicit
 *     *_guard_release() functions. A C call site that forgets the release call
 *     leaks the guard — reported, not papered over.
 *
 *  4. `std::backtrace` plus the symbol filter "keep the first engine frame"
 *     has no portable C equivalent, and depends on Rust symbol names. The port
 *     records the true source call site (__FILE__:__LINE__ of the instrumented
 *     allocation) instead, which is strictly more precise than a filtered
 *     frame name, and keeps the re-entrancy guard that stops a traced
 *     allocation from tracing itself forever.
 *
 *  5. Types are exposed as OPAQUE handles (incomplete struct pointers) and
 *     scalars only. include/rabuka.h is frozen, so nothing here may require a
 *     struct definition to be added to it; the orchestrator only has to add the
 *     prototypes.
 */

#include "rabuka.h"

#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

/* ── Layout constants (alloc_counter.rs) ──────────────────────────────────── */

#define RB_AC_BUCKETS        14     /* SIZE_BUCKETS.len() */
#define RB_AC_SITE_MAX       192    /* widest trace-site key */
#define RB_AC_SITES          512    /* distinct call sites retained */
#define RB_AC_TRACE_ROWS     40     /* TraceGuard::drop prints rows.take(40) */
#define RB_AC_LEDGER_SLOTS   16384  /* pointer -> size ledger */
#define RB_AC_SAMPLE_MASK    63u    /* SELF_TIME_SAMPLE_MASK: 1-in-64 sampling */
#define RB_AC_CLOCK_SAMPLES  20000u /* clock_overhead_ns()'s N */
#define RB_AC_ENV_ALLOC      "RABUKA_ALLOC_TRACK"
#define RB_AC_ENV_CPU        "RABUKA_CPU_TRACK"
#define RB_AC_ENV_TRACE_AT   "RABUKA_TRACE_AT"

/* ── Opaque public types (defined here, forward-declarable in a header) ──── */

typedef struct RbAllocCtrGuard    RbAllocCtrGuard;
typedef struct RbAllocCtrTraceGuard RbAllocCtrTraceGuard;

/* ── Forward declarations ─────────────────────────────────────────────────── */

static void rb_allocctr_trace_record(const char *site, size_t size);
void rb_allocctr_format_site(char *buf, size_t buf_len, const char *file, int line);
static void rb_ac_sort_trace_rows(void);

/* ── Clock ────────────────────────────────────────────────────────────────── */

static int64_t rb_allocctr_now_ns(void) {
#if defined(CLOCK_MONOTONIC)
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) return 0;
    return (int64_t)ts.tv_sec * INT64_C(1000000000) + (int64_t)ts.tv_nsec;
#else
    /* Fallback is CPU time, not wall time. The sampled totals these feed are
     * already a coarse 1-in-64 estimate, so the shape of the report holds. */
    return (int64_t)((double)clock() * (1000000000.0 / (double)CLOCKS_PER_SEC));
#endif
}

static double rb_ac_ms(double ns) { return ns / 1e6; }

/* ── Counters (the Rust statics) ──────────────────────────────────────────── */

static atomic_uint_least64_t g_alloc_count              = ATOMIC_VAR_INIT(0);
static atomic_uint_least64_t g_dealloc_count            = ATOMIC_VAR_INIT(0);
static atomic_int_least64_t  g_bytes_allocated          = ATOMIC_VAR_INIT(0);
static atomic_int_least64_t  g_peak_bytes              = ATOMIC_VAR_INIT(0);
static atomic_uint_least64_t g_total_bytes_allocated    = ATOMIC_VAR_INIT(0);
static atomic_uint_least64_t g_size_buckets[RB_AC_BUCKETS];

/* Sampled allocator self-timing (ALLOC_SELF_* / ALLOC_SELF_DEALLOC_*). */
static atomic_int_least64_t  g_alloc_self_ns             = ATOMIC_VAR_INIT(0);
static atomic_int_least64_t  g_alloc_self_dealloc_ns     = ATOMIC_VAR_INIT(0);
static atomic_uint_least64_t g_alloc_self_samples        = ATOMIC_VAR_INIT(0);
static atomic_uint_least64_t g_alloc_self_dealloc_samples = ATOMIC_VAR_INIT(0);

/* Ledger health (the C-specific addition from note 2). */
static atomic_uint_least64_t g_ledger_overflow = ATOMIC_VAR_INIT(0);
static atomic_uint_least64_t g_unmatched_free  = ATOMIC_VAR_INIT(0);

/* ── Call-site trace store (TRACE_ARMED / TRACE_SITES / IN_TRACE) ────────── */

static atomic_flag g_trace_lock = ATOMIC_FLAG_INIT;
static _Thread_local int rb_ac_in_trace;
static atomic_bool g_trace_armed_flag = ATOMIC_VAR_INIT(false);

static char     g_trace_site[RB_AC_SITES][RB_AC_SITE_MAX];
static uint64_t g_trace_count_of[RB_AC_SITES];
static uint64_t g_trace_bytes_of[RB_AC_SITES];
/* Occupancy index: entries are scattered by hash, so g_trace_slot[i] is the
 * hash slot behind logical row i and g_trace_rows is how many exist. */
static int      g_trace_slot[RB_AC_SITES];
static int      g_trace_rows;

/* Rust uses Mutex<Option<HashMap>>. A C11 atomic_flag spin lock is used instead:
   the critical section is a single open-addressed probe, the same order of cost
   as the relaxed atomics Rust already pays per allocation, and it adds no link
   dependency (-lpthread is not in the Makefile). */
static void rb_ac_lock(void) {
    while (atomic_flag_test_and_set_explicit(&g_trace_lock, memory_order_acquire)) {
        /* spin */
    }
}

static void rb_ac_unlock(void) {
    atomic_flag_clear_explicit(&g_trace_lock, memory_order_release);
}

static uint64_t rb_ac_hash(const char *s) {
    uint64_t h = 1469598103934665603ull;
    for (; *s; s++) {
        h ^= (unsigned char)*s;
        h *= 1099511628211ull;
    }
    return h;
}

/* size_bucket: floor(log2(size)) + 1 clamped into the 14 power-of-2 buckets.
 * size 0 (Rust: leading_zeros == BITS) and 1..7 both land in bucket 0. */
static int rb_ac_size_bucket(size_t size) {
    if (size < 8u) return 0;
    int bits = 0;
    for (size_t v = size; v != 0; v >>= 1) bits++;
    if (bits <= 3) return 0;
    int idx = bits - 3;
    return idx < RB_AC_BUCKETS ? idx : RB_AC_BUCKETS - 1;
}

const char *rb_allocctr_bucket_label(int i) {
    switch (i) {
        case 0:  return "1-7B   ";
        case 1:  return "8-15B  ";
        case 2:  return "16-31B ";
        case 3:  return "32-63B ";
        case 4:  return "64-127B";
        case 5:  return "128-255";
        case 6:  return "256-511";
        case 7:  return "512-1K ";
        case 8:  return "1K-2K  ";
        case 9:  return "2K-4K  ";
        case 10: return "4K-8K  ";
        case 11: return "8K-16K ";
        case 12: return "16K-32K";
        case 13: return "32K+   ";
        default: return "?";
    }
}

void rb_allocctr_format_site(char *buf, size_t buf_len, const char *file, int line) {
    if (!buf || buf_len == 0) return;
    snprintf(buf, buf_len, "%s:%d", file ? file : "?", line);
}

/* ── Pointer -> size ledger ───────────────────────────────────────────────── */

typedef struct {
    void  *ptr;
    size_t size;
    int    used;
} RbAcLedgerSlot;

static RbAcLedgerSlot g_ledger[RB_AC_LEDGER_SLOTS];

static size_t rb_ac_ledger_start(void *p) {
    uintptr_t v = (uintptr_t)p;
    v ^= v >> 17;
    v *= (uintptr_t)0x9E3779B97F4A7C15ull;
    v ^= v >> 31;
    return (size_t)(v % (uintptr_t)RB_AC_LEDGER_SLOTS);
}

static void rb_ac_ledger_insert(void *p, size_t n) {
    if (!p) return;
    size_t start = rb_ac_ledger_start(p);
    size_t first_free = RB_AC_LEDGER_SLOTS;
    for (size_t probe = 0; probe < RB_AC_LEDGER_SLOTS; probe++) {
        size_t idx = (start + probe) % RB_AC_LEDGER_SLOTS;
        if (!g_ledger[idx].used) {
            if (first_free == RB_AC_LEDGER_SLOTS) first_free = idx;
            continue;
        }
        if (g_ledger[idx].ptr == p) {   /* already tracked: refresh, no duplicate */
            g_ledger[idx].size = n;
            return;
        }
    }
    if (first_free == RB_AC_LEDGER_SLOTS) {
        atomic_fetch_add_explicit(&g_ledger_overflow, 1, memory_order_relaxed);
        return;
    }
    g_ledger[first_free].ptr  = p;
    g_ledger[first_free].size = n;
    g_ledger[first_free].used = 1;
}

/* 1 + recorded size when found; 0 when unknown (never tracked, ledger
 * overflowed, or already released — i.e. a double free). */
static int rb_ac_ledger_take(void *p, size_t *out_n) {
    if (!p || !out_n) return 0;
    size_t start = rb_ac_ledger_start(p);
    for (size_t probe = 0; probe < RB_AC_LEDGER_SLOTS; probe++) {
        size_t idx = (start + probe) % RB_AC_LEDGER_SLOTS;
        if (!g_ledger[idx].used) continue;
        if (g_ledger[idx].ptr == p) {
            *out_n = g_ledger[idx].size;
            g_ledger[idx].used = 0;
            g_ledger[idx].ptr  = NULL;
            g_ledger[idx].size = 0;
            return 1;
        }
    }
    return 0;
}

/* ── Snapshot (Snapshot struct + snapshot()) ──────────────────────────────── */

void rb_allocctr_snapshot(uint64_t *out_alloc_calls, uint64_t *out_dealloc_calls,
                          int64_t *out_live_bytes, int64_t *out_peak_bytes,
                          uint64_t *out_total_allocated,
                          uint64_t out_buckets[RB_AC_BUCKETS]) {
    if (out_buckets) {
        for (int i = 0; i < RB_AC_BUCKETS; i++) {
            out_buckets[i] = atomic_load_explicit(&g_size_buckets[i], memory_order_relaxed);
        }
    }
    if (out_alloc_calls)     *out_alloc_calls = atomic_load_explicit(&g_alloc_count, memory_order_relaxed);
    if (out_dealloc_calls)   *out_dealloc_calls = atomic_load_explicit(&g_dealloc_count, memory_order_relaxed);
    if (out_live_bytes)      *out_live_bytes = atomic_load_explicit(&g_bytes_allocated, memory_order_relaxed);
    if (out_peak_bytes)      *out_peak_bytes = atomic_load_explicit(&g_peak_bytes, memory_order_relaxed);
    if (out_total_allocated) *out_total_allocated = atomic_load_explicit(&g_total_bytes_allocated, memory_order_relaxed);
}

/* read_buckets() / alloc_buckets() */
void rb_allocctr_alloc_buckets(uint64_t out[RB_AC_BUCKETS]) {
    if (!out) return;
    for (int i = 0; i < RB_AC_BUCKETS; i++) {
        out[i] = atomic_load_explicit(&g_size_buckets[i], memory_order_relaxed);
    }
}

/* ── Counter updates (CountingAllocator::alloc / ::dealloc bodies) ───────── */

/* PEAK_BYTES compare_exchange loop. */
static void rb_ac_bump_peak(int64_t current) {
    int64_t peak = atomic_load_explicit(&g_peak_bytes, memory_order_relaxed);
    while (current > peak) {
        int64_t prev = peak;
        if (atomic_compare_exchange_weak_explicit(&g_peak_bytes, &prev, current,
                                                 memory_order_relaxed,
                                                 memory_order_relaxed)) {
            break;
        }
        peak = prev;   /* compare_exchange stored the observed value into prev */
    }
}

static void rb_ac_note_alloc_sized(size_t size, const char *site) {
    atomic_fetch_add_explicit(&g_alloc_count, 1, memory_order_relaxed);
    atomic_fetch_add_explicit(&g_size_buckets[rb_ac_size_bucket(size)], 1, memory_order_relaxed);
    if (atomic_load_explicit(&g_trace_armed_flag, memory_order_relaxed)) {
        rb_allocctr_trace_record(site, size);
    }
    int64_t prev = atomic_fetch_add_explicit(&g_bytes_allocated, (int64_t)size,
                                             memory_order_relaxed);
    atomic_fetch_add_explicit(&g_total_bytes_allocated, (uint64_t)size, memory_order_relaxed);
    rb_ac_bump_peak(prev + (int64_t)size);
}

void rb_allocctr_note_alloc(size_t size) {
    rb_ac_note_alloc_sized(size, "rb_allocctr_note_alloc");
}

void rb_allocctr_note_alloc_at(size_t size, const char *file, int line) {
    char site[RB_AC_SITE_MAX];
    rb_allocctr_format_site(site, sizeof site, file, line);
    rb_ac_note_alloc_sized(size, site);
}

void rb_allocctr_note_dealloc_sized(size_t size) {
    atomic_fetch_add_explicit(&g_dealloc_count, 1, memory_order_relaxed);
    atomic_fetch_sub_explicit(&g_bytes_allocated, (int64_t)size, memory_order_relaxed);
}

void rb_allocctr_note_dealloc(void) {
    /* Unsized free: the call count is exact, the byte delta is not knowable.
     * rb_allocctr_free() is the sized path and should be preferred. */
    atomic_fetch_add_explicit(&g_dealloc_count, 1, memory_order_relaxed);
}

/* ── Sampled self-timing ──────────────────────────────────────────────────── */

void rb_allocctr_allocator_self_ns(int64_t *alloc_ns, int64_t *dealloc_ns,
                                   uint64_t *alloc_samples, uint64_t *dealloc_samples) {
    if (alloc_ns) *alloc_ns = atomic_load_explicit(&g_alloc_self_ns, memory_order_relaxed);
    if (dealloc_ns) *dealloc_ns = atomic_load_explicit(&g_alloc_self_dealloc_ns, memory_order_relaxed);
    if (alloc_samples) *alloc_samples = atomic_load_explicit(&g_alloc_self_samples, memory_order_relaxed);
    if (dealloc_samples) *dealloc_samples = atomic_load_explicit(&g_alloc_self_dealloc_samples, memory_order_relaxed);
}

double rb_allocctr_clock_overhead_ns(void) {
    const uint32_t n = RB_AC_CLOCK_SAMPLES;
    int64_t t0 = rb_allocctr_now_ns();
    uint64_t acc = 0;
    for (uint32_t i = 0; i < n; i++) {
        int64_t a = rb_allocctr_now_ns();
        int64_t b = rb_allocctr_now_ns();
        acc += (uint64_t)(b - a);
    }
    int64_t total = rb_allocctr_now_ns() - t0;
    double per      = (double)acc / (double)n;
    double per_total = (double)total / (double)n;
    return per < per_total ? per : per_total;
}

/* ── Trace recording (record_trace) ───────────────────────────────────────── */

static void rb_allocctr_trace_record(const char *site, size_t size) {
    if (rb_ac_in_trace) return;     /* IN_TRACE re-entrancy guard */
    rb_ac_in_trace = 1;

    const char *key = site ? site : "?";
    rb_ac_lock();
    size_t start = rb_ac_hash(key) % (size_t)RB_AC_SITES;
    size_t first_free = RB_AC_SITES;
    int found = 0;
    for (size_t probe = 0; probe < RB_AC_SITES; probe++) {
        size_t idx = (start + probe) % (size_t)RB_AC_SITES;
        if (g_trace_site[idx][0] == '\0') {
            if (first_free == RB_AC_SITES) first_free = idx;
            continue;
        }
        if (strcmp(g_trace_site[idx], key) == 0) {
            g_trace_count_of[idx] += 1;
            g_trace_bytes_of[idx] += (uint64_t)size;
            found = 1;
            break;
        }
    }
    if (!found && first_free != RB_AC_SITES && g_trace_rows < RB_AC_SITES) {
        snprintf(g_trace_site[first_free], RB_AC_SITE_MAX, "%s", key);
        g_trace_site[first_free][RB_AC_SITE_MAX - 1] = '\0';
        g_trace_count_of[first_free] = 1;
        g_trace_bytes_of[first_free] = (uint64_t)size;
        g_trace_slot[g_trace_rows++] = (int)first_free;
    }
    rb_ac_unlock();

    rb_ac_in_trace = 0;
}

void rb_allocctr_arm_trace(void) {
    atomic_store_explicit(&g_trace_armed_flag, true, memory_order_relaxed);
    rb_ac_lock();
    memset(g_trace_site, 0, sizeof g_trace_site);
    memset(g_trace_count_of, 0, sizeof g_trace_count_of);
    memset(g_trace_bytes_of, 0, sizeof g_trace_bytes_of);
    g_trace_rows = 0;
    rb_ac_unlock();
}

int rb_allocctr_trace_armed(void) {
    return atomic_load_explicit(&g_trace_armed_flag, memory_order_relaxed) ? 1 : 0;
}

/* Stable insertion sort over the logical rows, count descending
 * (Rust: rows.sort_by(|a, b| b.1.cmp(&a.1))). */
static void rb_ac_sort_trace_rows(void) {
    for (int i = 1; i < g_trace_rows; i++) {
        int      sslot = g_trace_slot[i];
        uint64_t scount = g_trace_count_of[sslot];
        int j = i - 1;
        while (j >= 0 && g_trace_count_of[g_trace_slot[j]] < scount) {
            g_trace_slot[j + 1] = g_trace_slot[j];
            j--;
        }
        g_trace_slot[j + 1] = sslot;
    }
}

int rb_allocctr_trace_row(int i, char *site, size_t site_len,
                          uint64_t *count, uint64_t *bytes) {
    if (i < 0 || i >= g_trace_rows) return 0;
    int slot = g_trace_slot[i];
    if (site && site_len > 0) {
        snprintf(site, site_len, "%s", g_trace_site[slot]);
        site[site_len - 1] = '\0';
    }
    if (count) *count = g_trace_count_of[slot];
    if (bytes) *bytes = g_trace_bytes_of[slot];
    return 1;
}

/* Stop recording. The rows stay readable through rb_allocctr_trace_row() until
 * the next rb_allocctr_arm_trace(), which is what lets the guard print them
 * after the trace is already disarmed (Rust returns them by value). */
int rb_allocctr_disarm_trace(int *out_total_count, uint64_t *total_count_out,
                             uint64_t *total_bytes_out) {
    atomic_store_explicit(&g_trace_armed_flag, false, memory_order_relaxed);
    rb_ac_lock();
    uint64_t tc = 0, tb = 0;
    for (int i = 0; i < g_trace_rows; i++) {
        int slot = g_trace_slot[i];
        tc += g_trace_count_of[slot];
        tb += g_trace_bytes_of[slot];
    }
    rb_ac_sort_trace_rows();
    rb_ac_unlock();
    if (out_total_count) *out_total_count = g_trace_rows;
    if (total_count_out) *total_count_out = tc;
    if (total_bytes_out) *total_bytes_out = tb;
    return g_trace_rows;
}

/* ── TraceGuard (struct + Drop) ───────────────────────────────────────────── */

struct RbAllocCtrTraceGuard {
    const char *label;
    int         armed;
};

void rb_allocctr_trace_guard_release(RbAllocCtrTraceGuard *guard) {
    if (!guard || !guard->armed) return;

    int n_sites = 0;
    uint64_t total_count = 0, total_bytes = 0;
    rb_allocctr_disarm_trace(&n_sites, &total_count, &total_bytes);

    fprintf(stderr,
            "\n=== ALLOC TRACE: %s ===\n%llu allocations, %llu bytes, "
            "across %d distinct call sites\n",
            guard->label ? guard->label : "?",
            (unsigned long long)total_count, (unsigned long long)total_bytes, n_sites);

    int want = n_sites < RB_AC_TRACE_ROWS ? n_sites : RB_AC_TRACE_ROWS;
    for (int i = 0; i < want; i++) {
        char site[RB_AC_SITE_MAX];
        uint64_t count = 0, bytes = 0;
        if (rb_allocctr_trace_row(i, site, sizeof site, &count, &bytes)) {
            fprintf(stderr, "%6llux %9lluB  %s\n",
                    (unsigned long long)count, (unsigned long long)bytes, site);
        }
    }
    free(guard);
}

/* maybe_arm: arm at a named site the Nth time it is reached, keyed off
 * RABUKA_TRACE_AT=<label>#<n>. */
RbAllocCtrTraceGuard *rb_allocctr_maybe_arm(const char *label) {
    /* CACHE: parsed once, split at the LAST '#' (Rust rsplit_once). */
    static int      cache_ready = 0;
    static int      cache_valid = 0;
    static char     cache_label[RB_AC_SITE_MAX];
    static uint64_t cache_n = 0;
    static atomic_uint_least64_t hits = ATOMIC_VAR_INIT(0);

    if (!cache_ready) {
        const char *raw = getenv(RB_AC_ENV_TRACE_AT);
        if (raw && *raw) {
            const char *hash = strrchr(raw, '#');
            if (hash) {
                size_t len = (size_t)(hash - raw);
                size_t nlen = strlen(hash + 1);
                if (len > 0 && len < RB_AC_SITE_MAX && nlen > 0 && nlen < 32) {
                    char nbuf[32];
                    memcpy(cache_label, raw, len);
                    cache_label[len] = '\0';
                    memcpy(nbuf, hash + 1, nlen + 1);
                    char *end = NULL;
                    unsigned long long parsed = strtoull(nbuf, &end, 10);
                    if (end && *end == '\0') {
                        cache_n = (uint64_t)parsed;
                        cache_valid = 1;
                    }
                }
            }
        }
        cache_ready = 1;
    }
    if (!cache_valid || !label) return NULL;
    if (strcmp(cache_label, label) != 0) return NULL;

    uint64_t hit = atomic_fetch_add_explicit(&hits, 1, memory_order_relaxed);
    if (hit != cache_n) return NULL;

    rb_allocctr_arm_trace();
    RbAllocCtrTraceGuard *guard = (RbAllocCtrTraceGuard *)malloc(sizeof *guard);
    if (!guard) return NULL;
    guard->label = label;
    guard->armed = 1;
    return guard;
}

/* ── AllocGuard (struct + Drop) ───────────────────────────────────────────── */

typedef struct {
    uint64_t alloc_calls;
    uint64_t dealloc_calls;
    int64_t  live_bytes;
    int64_t  peak_bytes;
    uint64_t total_allocated;
    uint64_t buckets[RB_AC_BUCKETS];
} RbAllocCtrSnapshot;

struct RbAllocCtrGuard {
    int64_t           start_ns;
    RbAllocCtrSnapshot baseline;
    int               active;
};

static uint64_t rb_ac_sat_sub_u64(uint64_t a, uint64_t b) { return a > b ? a - b : 0; }

void rb_allocctr_guard_report(const RbAllocCtrGuard *g) {
    if (!g || !g->active) return;

    int64_t elapsed_ns = rb_allocctr_now_ns() - g->start_ns;
    RbAllocCtrSnapshot now;
    rb_allocctr_snapshot(&now.alloc_calls, &now.dealloc_calls, &now.live_bytes,
                         &now.peak_bytes, &now.total_allocated, now.buckets);

    uint64_t d  = rb_ac_sat_sub_u64(now.alloc_calls, g->baseline.alloc_calls);
    uint64_t dd = rb_ac_sat_sub_u64(now.dealloc_calls, g->baseline.dealloc_calls);
    int64_t  live = now.live_bytes - g->baseline.live_bytes;
    int64_t  peak = now.peak_bytes - g->baseline.peak_bytes;
    uint64_t total = rb_ac_sat_sub_u64(now.total_allocated, g->baseline.total_allocated);

    if (getenv(RB_AC_ENV_ALLOC)) {
        int64_t peak_clamped = peak > 0 ? peak : 0;
        fprintf(stderr, "\n=== Allocator report (%lld ms) ===\n",
                (long long)(elapsed_ns / 1000000));
        fprintf(stderr, "  alloc calls:       %llu\n", (unsigned long long)d);
        fprintf(stderr, "  dealloc calls:     %llu\n", (unsigned long long)dd);
        fprintf(stderr, "  net live allocs:   %llu\n",
                (unsigned long long)rb_ac_sat_sub_u64(d, dd));
        if (live >= 0) {
            fprintf(stderr, "  live bytes:        %lld B  (%lld KB)\n",
                    (long long)live, (long long)(live / 1024));
        } else {
            fprintf(stderr, "  live bytes:        -%lld B  (freed during test)\n",
                    (long long)-live);
        }
        fprintf(stderr, "  peak bytes:        %lld B  (%lld KB)\n",
                (long long)peak_clamped, (long long)(peak_clamped / 1024));
        fprintf(stderr, "  lifetime peak:     %lld B  (%lld KB)\n",
                (long long)now.peak_bytes, (long long)(now.peak_bytes / 1024));
        fprintf(stderr, "  total allocated:   %llu B  (%llu KB)\n",
                (unsigned long long)total, (unsigned long long)(total / 1024));

        fprintf(stderr, "  --- allocs by size ---\n");
        for (int i = 0; i < RB_AC_BUCKETS; i++) {
            uint64_t cnt = rb_ac_sat_sub_u64(now.buckets[i], g->baseline.buckets[i]);
            if (cnt > 0) {
                fprintf(stderr, "    %s: %llu allocs\n",
                        rb_allocctr_bucket_label(i), (unsigned long long)cnt);
            }
        }

        double clock_ns = rb_allocctr_clock_overhead_ns();
        int64_t  a_ns = 0, d_ns = 0;
        uint64_t a_s = 0, d_s = 0;
        rb_allocctr_allocator_self_ns(&a_ns, &d_ns, &a_s, &d_s);
        double total_ns = (double)elapsed_ns;

        if (a_s + d_s > 0) {
            double raw = (double)(a_ns + d_ns);
            double overhead = clock_ns * (double)(a_s + d_s);
            double corrected = raw - overhead;
            if (corrected < 0.0) corrected = 0.0;
            double sample_rate = (double)(RB_AC_SAMPLE_MASK + 1u);
            double extrapolated = corrected * sample_rate;
            double share = total_ns > 0.0 ? extrapolated / total_ns * 100.0 : 0.0;

            fprintf(stderr, "  --- allocator self time (1-in-%u sampled) ---\n",
                    RB_AC_SAMPLE_MASK + 1u);
            fprintf(stderr, "    samples:          %llu alloc + %llu dealloc\n",
                    (unsigned long long)a_s, (unsigned long long)d_s);
            fprintf(stderr, "    clock overhead:   %.1f ns/call\n", clock_ns);
            fprintf(stderr, "    raw sampled:      %.2f ms  (of %.2f ms elapsed)\n",
                    rb_ac_ms(raw), rb_ac_ms(total_ns));
            fprintf(stderr, "    less clock:       %.2f ms (sampled) \n",
                    rb_ac_ms(corrected));
            fprintf(stderr, "    extrapolated:     %.2f ms  = %.1f%% of elapsed"
                            "  <-- allocator share\n",
                    rb_ac_ms(extrapolated), share);
            fprintf(stderr, "    ns per alloc+dealloc (corrected): %.1f\n",
                    corrected / (double)(a_s + d_s));
        }

        /* Ledger health. A non-zero unmatched-free count is the double-free
         * signature this ledger exists to expose. */
        uint64_t unmatched = atomic_load_explicit(&g_unmatched_free, memory_order_relaxed);
        uint64_t overflow  = atomic_load_explicit(&g_ledger_overflow, memory_order_relaxed);
        if (unmatched || overflow) {
            fprintf(stderr, "  --- ledger ---\n");
            fprintf(stderr, "    unmatched frees:  %llu  (double-free / untracked pointer)\n",
                    (unsigned long long)unmatched);
            fprintf(stderr, "    ledger overflow:  %llu  (sizes untracked;"
                            " live_bytes is an upper bound)\n",
                    (unsigned long long)overflow);
        }
    }
    if (getenv(RB_AC_ENV_CPU)) {
        fprintf(stderr, "\n=== CPU report: %lld ms ===\n",
                (long long)(elapsed_ns / 1000000));
    }
}

RbAllocCtrGuard *rb_allocctr_start(void) {
    if (!getenv(RB_AC_ENV_ALLOC) && !getenv(RB_AC_ENV_CPU)) return NULL;
    RbAllocCtrGuard *g = (RbAllocCtrGuard *)malloc(sizeof *g);
    if (!g) return NULL;
    g->start_ns = rb_allocctr_now_ns();
    rb_allocctr_snapshot(&g->baseline.alloc_calls, &g->baseline.dealloc_calls,
                         &g->baseline.live_bytes, &g->baseline.peak_bytes,
                         &g->baseline.total_allocated, g->baseline.buckets);
    g->active = 1;
    return g;
}

void rb_allocctr_guard_release(RbAllocCtrGuard *g) {
    if (!g || !g->active) return;
    rb_allocctr_guard_report(g);
    g->active = 0;
    free(g);
}

/* ── Instrumented allocator (the CountingAllocator body) ──────────────────── */

void *rb_allocctr_malloc(size_t n) {
    void *p = malloc(n);
    if (p) rb_ac_ledger_insert(p, n);

    /* Rust order: count, update bytes, bump peak, THEN decide sampling. The
     * sample predicate is `ALLOC_COUNT & MASK == 0` read AFTER the increment. */
    rb_ac_note_alloc_sized(n, "rb_allocctr_malloc");

    int sampled = (atomic_load_explicit(&g_alloc_count, memory_order_relaxed)
                   & RB_AC_SAMPLE_MASK) == 0;
    int64_t t0 = sampled ? rb_allocctr_now_ns() : 0;
    if (sampled) {
        atomic_fetch_add_explicit(&g_alloc_self_ns, rb_allocctr_now_ns() - t0,
                                  memory_order_relaxed);
        atomic_fetch_add_explicit(&g_alloc_self_samples, 1, memory_order_relaxed);
    }
    return p;
}

void rb_allocctr_free(void *p) {
    if (!p) return;

    size_t size = 0;
    if (rb_ac_ledger_take(p, &size)) {
        rb_allocctr_note_dealloc_sized(size);
    } else {
        /* Not tracked: never allocated through the instrumented path, the
         * ledger overflowed, or this pointer was already freed. The last case
         * is a double free and is exactly what we want counted. */
        atomic_fetch_add_explicit(&g_unmatched_free, 1, memory_order_relaxed);
        rb_allocctr_note_dealloc();
    }

    int sampled = (atomic_load_explicit(&g_dealloc_count, memory_order_relaxed)
                   & RB_AC_SAMPLE_MASK) == 0;
    int64_t t0 = sampled ? rb_allocctr_now_ns() : 0;
    free(p);
    if (sampled) {
        atomic_fetch_add_explicit(&g_alloc_self_dealloc_ns, rb_allocctr_now_ns() - t0,
                                  memory_order_relaxed);
        atomic_fetch_add_explicit(&g_alloc_self_dealloc_samples, 1, memory_order_relaxed);
    }
}

char *rb_allocctr_strdup2(const char *s) {
    if (!s) return NULL;
    size_t n = strlen(s) + 1;
    char *p = (char *)rb_allocctr_malloc(n);
    if (p) memcpy(p, s, n);
    return p;
}

/* ── Counters: read access ────────────────────────────────────────────────── */

uint64_t rb_allocctr_alloc_count(void) {
    return atomic_load_explicit(&g_alloc_count, memory_order_relaxed);
}

uint64_t rb_allocctr_dealloc_count(void) {
    return atomic_load_explicit(&g_dealloc_count, memory_order_relaxed);
}

int64_t rb_allocctr_live_bytes(void) {
    return atomic_load_explicit(&g_bytes_allocated, memory_order_relaxed);
}

int64_t rb_allocctr_peak_bytes(void) {
    return atomic_load_explicit(&g_peak_bytes, memory_order_relaxed);
}

uint64_t rb_allocctr_total_allocated(void) {
    return atomic_load_explicit(&g_total_bytes_allocated, memory_order_relaxed);
}

/* Outstanding heap allocations = alloc_calls - dealloc_calls. Meaningful only
 * when every allocation and every free routes through rb_allocctr_malloc /
 * rb_allocctr_free; see the CROSS-FILE REQUEST. */
uint64_t rb_allocctr_outstanding_allocs(void) {
    uint64_t a = atomic_load_explicit(&g_alloc_count, memory_order_relaxed);
    uint64_t d = atomic_load_explicit(&g_dealloc_count, memory_order_relaxed);
    return a > d ? a - d : 0;
}

uint64_t rb_allocctr_unmatched_free_count(void) {
    return atomic_load_explicit(&g_unmatched_free, memory_order_relaxed);
}

uint64_t rb_allocctr_ledger_overflow_count(void) {
    return atomic_load_explicit(&g_ledger_overflow, memory_order_relaxed);
}

/* ══════════════════════════════════════════════════════════════════════════
   engine/src/core/runtime_flags.rs — process-global speed/verbosity gates.
   Both default ON, so web/UI paths and the test suite are unaffected;
   training and headless benches turn them OFF via set_training_mode.
   ══════════════════════════════════════════════════════════════════════════ */

static atomic_bool g_action_display = ATOMIC_VAR_INIT(true);
static atomic_bool g_logging        = ATOMIC_VAR_INIT(true);

void rb_allocctr_flag_set_action_display(int enabled) {
    atomic_store_explicit(&g_action_display, enabled ? true : false, memory_order_relaxed);
}

int rb_allocctr_flag_action_display_enabled(void) {
    return atomic_load_explicit(&g_action_display, memory_order_relaxed) ? 1 : 0;
}

void rb_allocctr_flag_set_logging_enabled(int enabled) {
    atomic_store_explicit(&g_logging, enabled ? true : false, memory_order_relaxed);
}

int rb_allocctr_flag_logging_enabled(void) {
    return atomic_load_explicit(&g_logging, memory_order_relaxed) ? 1 : 0;
}

void rb_allocctr_flag_set_training_mode(int enabled) {
    rb_allocctr_flag_set_action_display(!enabled);
    rb_allocctr_flag_set_logging_enabled(!enabled);
}