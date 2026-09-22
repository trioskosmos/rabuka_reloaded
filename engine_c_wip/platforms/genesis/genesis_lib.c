/* engine_c/platforms/genesis/genesis_lib.c — minimal bare-metal support for
    the Mega Drive / Genesis (Motorola 68000). Provides the libc symbols the
    engine references (mem/string ops, a bump-arena allocator, rand routing)
    plus no-op stdio stubs. Link with -lgcc for the 32x32 mul/div helpers. */
#include <stddef.h>
#include <stdint.h>
#include "rabuka.h"   /* routes rand() to the engine's deterministic RNG */

/* Genesis has 64 KB work RAM; leave room for the stack + static .bss/.data. */
#define GENESIS_ARENA_SIZE (48u * 1024u)
static unsigned char genesis_arena[GENESIS_ARENA_SIZE];
static size_t genesis_off = 0;

/* Defined here (declared extern in data.c under RB_ROM_STRINGS): RAM pointer
   arrays that index directly into the ROM-embedded card/ability string blobs. */
char **g_card_strings_rom = NULL;
char **g_strings_rom = NULL;

void *malloc(size_t n) {
    if (n == 0) n = 1;
    n = (n + 7u) & ~(size_t)7u;
    if (genesis_off + n > GENESIS_ARENA_SIZE) return NULL;
    void *p = genesis_arena + genesis_off;
    genesis_off += n;
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
    return q; /* callers re-read source; old block leaks (one-shot arena) */
}

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
int strcasecmp(const char *a, const char *b) {
    while (*a) {
        int ca = *a, cb = *b;
        if (ca >= 'A' && ca <= 'Z') ca += 32;
        if (cb >= 'A' && cb <= 'Z') cb += 32;
        if (ca != cb) return ca - cb;
        a++; b++;
    }
    return -(unsigned char)*b;
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
char *strchr(const char *s, int c) {
    while (*s) { if (*s == (char)c) return (char *)s; s++; }
    return (c == 0) ? (char *)s : NULL;
}
int strncmp(const char *a, const char *b, size_t n) {
    while (n--) {
        if (*a != *b) return (int)(unsigned char)*a - (int)(unsigned char)*b;
        if (!*a) break;
        a++; b++;
    }
    return 0;
}
char *strncat(char *d, const char *s, size_t n) {
    char *p = d + strlen(d);
    while (n-- && *s) *p++ = *s++;
    *p = 0;
    return d;
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
unsigned long strtoul(const char *s, char **end, int base);
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
unsigned long strtoul(const char *s, char **end, int base) {
    unsigned long v = 0; int neg = 0;
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
        v = v * base + (unsigned long)d; s++;
    }
    if (end) *end = (char *)s;
    return neg ? -v : v;
}

/* ── stdio stubs (no terminal on the console; debug output is dropped) ── */
void *stderr = (void *)0;
void *stdout = (void *)0;
int fprintf(void *f, const char *fmt, ...) { (void)f; (void)fmt; return 0; }
int printf(const char *fmt, ...) { (void)fmt; return 0; }
int snprintf(char *b, size_t n, const char *fmt, ...) { (void)b; (void)n; (void)fmt; return 0; }
int vfprintf(void *f, const char *fmt, void *ap) { (void)f; (void)fmt; (void)ap; return 0; }
void abort(void) { for (;;) {} }

/* newlib reentrancy / ctype globals: referenced by a few optimized libc calls
   that the toolchain may emit. We override the string functions ourselves, so
   these are never actually dereferenced — provide them only to satisfy the
   linker. */
void *_impure_ptr = (void *)0;
char  _ctype_[1];

/* engine uses rand() in a few effects; route it to the deterministic RNG so
   replays stay stable (rb_seed drives it). */
int rand(void) { return (int)(rb_rand() & 0x7fffffff); }
void srand(unsigned s) { rb_seed((uint32_t)s); }
