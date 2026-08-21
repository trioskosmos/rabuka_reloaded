/* Freestanding libc for the Jaguar wasm2c shell.
 * - string/memory helpers (no libc on this toolchain)
 * - bump malloc family over a static DRAM arena, serving wasm-rt's
 *   calloc/realloc calls for linear memory + tables (arena is sized to the
 *   22-page initial==max linear memory with headroom; growth never happens)
 */

#include <stddef.h>
#include <stdint.h>

/* linker script symbols */
extern unsigned char __arena_start[];
extern unsigned char __arena_end[];

void *memcpy(void *dst, const void *src, size_t n) {
    unsigned char *d = dst; const unsigned char *s = src;
    while (n--) *d++ = *s++;
    return dst;
}
void *memmove(void *dst, const void *src, size_t n) {
    unsigned char *d = dst; const unsigned char *s = src;
    if (d < s) { while (n--) *d++ = *s++; }
    else { d += n; s += n; while (n--) *--d = *--s; }
    return dst;
}
void *memset(void *p, int c, size_t n) {
    unsigned char *q = p;
    while (n--) *q++ = (unsigned char)c;
    return p;
}
int memcmp(const void *a, const void *b, size_t n) {
    const unsigned char *x = a, *y = b;
    while (n--) { if (*x != *y) return (int)*x - (int)*y; x++; y++; }
    return 0;
}
size_t strlen(const char *s) {
    size_t n = 0; while (*s++) n++; return n;
}
void abort(void) { for (;;) { __asm__ volatile("nop"); } }

/* assert() lands here even with NDEBUG (wasm2c's instantiate calls it) */
void __assert_fail(const char *expr, const char *file, unsigned int line,
                   const char *func) {
    (void)expr; (void)file; (void)line; (void)func;
    abort();
}

/* ---- bump allocator over [__arena_start, __arena_end) ----
 * Every block carries a 16-byte header holding its payload size so realloc
 * can copy exactly min(old, new) bytes. */

static unsigned char *arena_cur;

static void arena_reset(void) {
    arena_cur = __arena_start;
}

void *malloc(size_t n) {
    if (arena_cur == 0) arena_reset();
    uintptr_t p = ((uintptr_t)arena_cur + 15u) & ~(uintptr_t)15u;
    unsigned char *hdr = (unsigned char *)p;
    unsigned char *payload = hdr + 16;
    if (n > (uintptr_t)__arena_end - (uintptr_t)payload) return NULL;
    *(size_t *)hdr = n;
    arena_cur = payload + n;
    return payload;
}
void free(void *p) { (void)p; }
void *calloc(size_t nmemb, size_t size) {
    void *p = malloc(nmemb * size);
    if (p) memset(p, 0, nmemb * size);
    return p;
}
void *realloc(void *old, size_t n) {
    void *p = malloc(n);
    if (p && old) {
        size_t old_n = *(size_t *)((unsigned char *)old - 16);
        memcpy(p, old, old_n < n ? old_n : n);
    }
    return p;
}

/* ---- stubs for wasm-rt shared-memory paths (unused: WASM_ENABLE_SHARED
 * memory is off; the linker still pulls the symbols from mem-impl) ---- */
int pthread_mutex_init(void *m, void *a) { (void)m; (void)a; return 0; }
int pthread_mutex_lock(void *m) { (void)m; return 0; }
int pthread_mutex_unlock(void *m) { (void)m; return 0; }
int pthread_mutex_destroy(void *m) { (void)m; return 0; }
/* FILE machinery does not exist; error paths only */
int stderr_placeholder;
void *stderr = &stderr_placeholder;
int fprintf(void *f, const char *fmt, ...) { (void)f; (void)fmt; return 0; }
