/* engine_c/platforms/cdi/cdi_lib.c — minimal bare-metal support for CD-i.
   Provides the few libc symbols the engine references (mem/string ops, a
   bump-arena allocator) plus no-op stdio stubs. Link with -lgcc for the
   32x32 mul/div helpers the m68000 lacks in hardware. */
#include <stddef.h>
#include <stdint.h>
#include "rabuka.h"   /* route rand() to the engine's deterministic RNG */

/* ── bump arena (one-shot; free() is a no-op, realloc copies) ── */
#define CDI_ARENA_SIZE (4u * 1024u)
static unsigned char cdi_arena[CDI_ARENA_SIZE];
static size_t cdi_off = 0;

void *malloc(size_t n) {
    n = (n + 7u) & ~(size_t)7u;
    if (cdi_off + n > CDI_ARENA_SIZE) return NULL;
    void *p = cdi_arena + cdi_off;
    cdi_off += n;
    return p;
}
void free(void *p) { (void)p; }
void *calloc(size_t a, size_t b) {
    size_t n = a * b;
    void *p = malloc(n);
    if (p) { unsigned char *q = (unsigned char *)p; while (n--) *q++ = 0; }
    return p;
}
void *realloc(void *p, size_t n) {
    if (!p) return malloc(n);
    void *q = malloc(n);
    return q; /* callers that grow re-read source; old block leaks (one-shot) */
}

/* ── mem / string ── */
void *memcpy(void *d, const void *s, size_t n) {
    unsigned char *a = d; const unsigned char *b = s;
    while (n--) *a++ = *b++; return d;
}
void *memmove(void *d, const void *s, size_t n) {
    unsigned char *a = d; const unsigned char *b = s;
    if (a < b) { while (n--) *a++ = *b++; }
    else { a += n; b += n; while (n--) *--a = *--b; }
    return d;
}
void *memset(void *d, int v, size_t n) {
    unsigned char *a = d; unsigned char c = (unsigned char)v;
    while (n--) *a++ = c; return d;
}
int memcmp(const void *s1, const void *s2, size_t n) {
    const unsigned char *a = s1, *b = s2;
    while (n--) { if (*a != *b) return (int)*a - (int)*b; a++; b++; }
    return 0;
}
size_t strlen(const char *s) { size_t n = 0; while (*s++) n++; return n; }
int strcmp(const char *a, const char *b) {
    while (*a && *a == *b) { a++; b++; }
    return (int)(unsigned char)*a - (int)(unsigned char)*b;
}
char *strncpy(char *d, const char *s, size_t n) {
    size_t i = 0;
    while (i < n && s[i]) { d[i] = s[i]; i++; }
    while (i < n) d[i++] = 0;
    return d;
}
char *strstr(const char *h, const char *n) {
    if (!*n) return (char *)h;
    for (; *h; h++) {
        const char *p = h, *q = n;
        while (*p && *p == *q) { p++; q++; }
        if (!*q) return (char *)h;
    }
    return NULL;
}
static const char *strchr(const char *s, int c) {
    while (*s) { if (*s == (char)c) return s; s++; }
    return (c == 0) ? (char *)s : NULL;
}
char *strtok(char *s, const char *d) {
    static char *cur = NULL;
    if (s) cur = s;
    if (!cur) return NULL;
    while (*cur && strchr(d, *cur)) cur++;
    if (!*cur) return NULL;
    char *tok = cur;
    while (*cur && !strchr(d, *cur)) cur++;
    if (*cur) *cur++ = 0;
    return tok;
}
long strtol(const char *s, char **end, int base);
int atoi(const char *s) { return (int)strtol(s, NULL, 10); }
long strtol(const char *s, char **end, int base) {
    long v = 0; int neg = 0;
    while (*s == ' ' || *s == '\t') s++;
    if (*s == '-') { neg = 1; s++; } else if (*s == '+') s++;
    if (base == 0) base = (*s == '0') ? 8 : 10;
    while (*s) {
        int d; char c = *s;
        if (c >= '0' && c <= '9') d = c - '0';
        else if (c >= 'a' && c <= 'f') d = c - 'a' + 10;
        else if (c >= 'A' && c <= 'F') d = c - 'A' + 10;
        else break;
        if (d >= base) break;
        v = v * base + d; s++;
    }
    if (end) *end = (char *)s;
    return neg ? -v : v;
}

/* ── stdio stubs (CD-i has no terminal; debug output is dropped) ── */
void *stderr = (void *)0;
int fprintf(void *f, const char *fmt, ...) { (void)f; (void)fmt; return 0; }
int printf(const char *fmt, ...) { (void)fmt; return 0; }
int snprintf(char *b, size_t n, const char *fmt, ...) { (void)b; (void)n; (void)fmt; return 0; }
int vfprintf(void *f, const char *fmt, void *ap) { (void)f; (void)fmt; (void)ap; return 0; }
void abort(void) { for (;;) {} }

/* engine uses rand() in a few effects; route it to the deterministic RNG so
   replays stay stable (rb_seed drives it). */
int rand(void) { return (int)(rb_rand() & 0x7fffffff); }
void srand(unsigned s) { rb_seed((uint32_t)s); }
