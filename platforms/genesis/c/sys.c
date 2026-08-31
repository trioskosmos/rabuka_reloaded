/*
 * Minimal bare-metal C runtime for Sega Genesis / Mega Drive (m68k 68000).
 * Provides the libc symbols engine_c needs: bump allocator, string/stdlib
 * helpers, and a printf family that writes to the VDP text console.
 *
 * No host OS, no filesystem. Everything is freestanding.
 */
#include <stddef.h>
#include <stdarg.h>
#include <stdint.h>
#include <ctype.h>
#include <stdio.h>
#include <string.h>

/* ---- console output hook (implemented in console.c) ---- */
void __console_out(int c);

/* ===================================================================== */
/*                          ALLOCATOR (bump arena)                       */
/* ===================================================================== */
#define HEAP_SIZE 0xD000u
static unsigned char g_heap[HEAP_SIZE];
static unsigned long g_off = 0;

void *malloc(size_t n) {
    n = (n + 7u) & ~7u;            /* 8-byte align */
    if (g_off + n > HEAP_SIZE) return NULL;
    void *p = g_heap + g_off;
    g_off += n;
    return p;
}
void free(void *p) { (void)p; }    /* bump arena: never reclaim */
void *calloc(size_t n, size_t s) {
    size_t t = n * s;
    void *p = malloc(t);
    if (p) memset(p, 0, t);
    return p;
}
void *realloc(void *p, size_t n) {
    if (!p) return malloc(n);
    void *q = malloc(n);
    if (q) memcpy(q, p, n);        /* arena: old block never reused */
    return q;
}

/* ===================================================================== */
/*                            MEMORY / STRING                            */
/* ===================================================================== */
void *memcpy(void *dst, const void *src, size_t n) {
    unsigned char *d = (unsigned char *)dst;
    const unsigned char *s = (const unsigned char *)src;
    while (n--) *d++ = *s++;
    return dst;
}
void *memmove(void *dst, const void *src, size_t n) {
    unsigned char *d = (unsigned char *)dst;
    const unsigned char *s = (const unsigned char *)src;
    if (d < s) { while (n--) *d++ = *s++; }
    else { d += n; s += n; while (n--) *--d = *--s; }
    return dst;
}
void *memset(void *dst, int c, size_t n) {
    unsigned char *d = (unsigned char *)dst;
    unsigned char v = (unsigned char)c;
    while (n--) *d++ = v;
    return dst;
}
int memcmp(const void *a, const void *b, size_t n) {
    const unsigned char *x = (const unsigned char *)a;
    const unsigned char *y = (const unsigned char *)b;
    while (n--) { int d = (int)*x++ - (int)*y++; if (d) return d; }
    return 0;
}
void *memchr(const void *s, int c, size_t n) {
    const unsigned char *p = (const unsigned char *)s;
    while (n--) { if (*p == (unsigned char)c) return (void *)(uintptr_t)p; p++; }
    return NULL;
}

size_t strlen(const char *s) {
    const char *p = s;
    while (*p) p++;
    return (size_t)(p - s);
}
int strcmp(const char *a, const char *b) {
    while (*a && *a == *b) { a++; b++; }
    return (int)(unsigned char)*a - (int)(unsigned char)*b;
}
int strncmp(const char *a, const char *b, size_t n) {
    while (n--) { if (!*a) return (int)(unsigned char)*a - (int)(unsigned char)*b;
                 if (*a != *b) return (int)(unsigned char)*a - (int)(unsigned char)*b; a++; b++; }
    return 0;
}
char *strcpy(char *dst, const char *src) {
    char *d = dst;
    while ((*d++ = *src++));
    return dst;
}
char *strncpy(char *dst, const char *src, size_t n) {
    char *d = dst;
    while (n--) { *d++ = *src; if (*src) src++; }
    return dst;
}
char *strcat(char *dst, const char *src) {
    char *d = dst + strlen(dst);
    while ((*d++ = *src++));
    return dst;
}
char *strncat(char *dst, const char *src, size_t n) {
    char *d = dst + strlen(dst);
    while (n-- && *src) *d++ = *src++;
    *d = 0;
    return dst;
}
char *strchr(const char *s, int c) {
    while (*s) { if (*(unsigned char *)s == (unsigned char)c) return (char *)(uintptr_t)s; s++; }
    if (c == 0) return (char *)(uintptr_t)s;
    return NULL;
}
char *strrchr(const char *s, int c) {
    const char *last = NULL;
    while (*s) { if (*(unsigned char *)s == (unsigned char)c) last = s; s++; }
    if (c == 0) return (char *)(uintptr_t)s;
    return (char *)(uintptr_t)last;
}
char *strstr(const char *hay, const char *needle) {
    if (!*needle) return (char *)(uintptr_t)hay;
    size_t nl = strlen(needle);
    while (*hay) {
        if (strncmp(hay, needle, nl) == 0) return (char *)(uintptr_t)hay;
        hay++;
    }
    return NULL;
}
size_t strspn(const char *s, const char *set) {
    size_t n = 0;
    while (*s && strchr(set, *(unsigned char *)s)) { n++; s++; }
    return n;
}
size_t strcspn(const char *s, const char *set) {
    size_t n = 0;
    while (*s && !strchr(set, *(unsigned char *)s)) { n++; s++; }
    return n;
}
char *strpbrk(const char *s, const char *set) {
    while (*s) { if (strchr(set, *(unsigned char *)s)) return (char *)(uintptr_t)s; s++; }
    return NULL;
}
static char *g_strtok_state = NULL;
char *strtok(char *s, const char *delim) {
    if (s) g_strtok_state = s;
    if (!g_strtok_state) return NULL;
    g_strtok_state += strspn(g_strtok_state, delim);
    if (!*g_strtok_state) { g_strtok_state = NULL; return NULL; }
    char *tok = g_strtok_state;
    char *e = strpbrk(tok, delim);
    if (e) { *e = 0; g_strtok_state = e + 1; }
    else g_strtok_state = NULL;
    return tok;
}
char *strdup(const char *s) {
    size_t n = strlen(s) + 1;
    char *p = (char *)malloc(n);
    if (p) memcpy(p, s, n);
    return p;
}

/* ===================================================================== */
/*                              STDLIB MATH                              */
/* ===================================================================== */
int abs(int v) { return v < 0 ? -v : v; }
long labs(long v) { return v < 0 ? -v : v; }

static unsigned long g_rand = 1;
int rand(void) {
    g_rand = g_rand * 1103515245u + 12345u;
    return (int)((g_rand >> 16) & 0x7FFF);
}
void srand(unsigned int seed) { g_rand = seed ? seed : 1; }

long strtol(const char *s, char **end, int base) {
    while (isspace((unsigned char)*s)) s++;
    int neg = 0;
    if (*s == '+') s++; else if (*s == '-') { neg = 1; s++; }
    if (base == 0) {
        if (*s == '0' && (s[1] == 'x' || s[1] == 'X')) { base = 16; s += 2; }
        else base = 10;
    }
    long v = 0;
    while (1) {
        int d;
        char c = *s;
        if (c >= '0' && c <= '9') d = c - '0';
        else if (c >= 'a' && c <= 'f') d = c - 'a' + 10;
        else if (c >= 'A' && c <= 'F') d = c - 'A' + 10;
        else break;
        if (d >= base) break;
        v = v * base + d;
        s++;
    }
    if (end) *end = (char *)s;
    return neg ? -v : v;
}
long strtoul(const char *s, char **end, int base) { return (long)strtol(s, end, base); }
long long strtoll(const char *s, char **end, int base) { return (long long)strtol(s, end, base); }
int atoi(const char *s) { return (int)strtol(s, NULL, 10); }
long atol(const char *s) { return strtol(s, NULL, 10); }

void exit(int code) { (void)code; for (;;); }
void abort(void) { for (;;); }

/* ===================================================================== */
/*                          PRINTF FAMILY                                */
/* ===================================================================== */
#define OUTBUF 600

static void out_num(unsigned long long v, int base, int width, int pad, int upper) {
    char tmp[24];
    int i = 0;
    if (v == 0) tmp[i++] = '0';
    while (v) {
        int d = (int)(v % base);
        tmp[i++] = (char)(d < 10 ? ('0' + d) : (upper ? ('A' + d - 10) : ('a' + d - 10)));
        v /= base;
    }
    while (i < width) tmp[i++] = (char)(pad ? pad : ' ');
    while (i--) __console_out(tmp[i]);
}

static void fmt_putc(char c) { __console_out(c); }

int vsnprintf(char *buf, size_t n, const char *fmt, va_list ap) {
    size_t written = 0;
    const char *p = fmt;
    /* helper to emit one char respecting the buffer cap */
    #define EMIT(ch) do { if (written + 1 < n && n > 0) buf[written] = (ch); if (written < n) written++; } while (0)
    while (*p) {
        if (*p != '%') { EMIT(*p++); continue; }
        p++;
        int pad = 0, width = 0, left = 0;
        while (*p == '-') { left = 1; p++; }
        while (*p == '0') { pad = '0'; p++; }
        while (*p >= '0' && *p <= '9') { width = width * 10 + (*p - '0'); p++; }
        int longness = 0;
        if (*p == 'l') { longness = 1; p++; if (*p == 'l') { longness = 2; p++; } }
        int spec = *p++;
        char cb[OUTBUF];
        size_t ci = 0;
        int did = 0;
        switch (spec) {
            case 'c': { int ch = va_arg(ap, int); cb[ci++] = (char)ch; did = 1; break; }
            case 's': {
                const char *s = va_arg(ap, const char *);
                if (!s) s = "(null)";
                while (*s) cb[ci++] = *s++;
                did = 1; break;
            }
            case 'd': case 'i': {
                long long v = (long long)(longness == 2 ? va_arg(ap, long long)
                                    : longness == 1 ? va_arg(ap, long)
                                    : va_arg(ap, int));
                if (v < 0) { cb[ci++] = '-'; v = -v; }
                char t[24]; int ti = 0;
                if (v == 0) t[ti++] = '0';
                while (v) { t[ti++] = (char)('0' + (v % 10)); v /= 10; }
                while (ti--) cb[ci++] = t[ti];
                did = 1; break;
            }
            case 'u': {
                unsigned long long v = (longness == 2 ? va_arg(ap, unsigned long long)
                                    : longness == 1 ? va_arg(ap, unsigned long)
                                    : va_arg(ap, unsigned int));
                char t[24]; int ti = 0;
                if (v == 0) t[ti++] = '0';
                while (v) { t[ti++] = (char)('0' + (v % 10)); v /= 10; }
                while (ti--) cb[ci++] = t[ti];
                did = 1; break;
            }
            case 'x': case 'X': {
                unsigned long long v = (longness == 2 ? va_arg(ap, unsigned long long)
                                    : longness == 1 ? va_arg(ap, unsigned long)
                                    : va_arg(ap, unsigned int));
                char t[24]; int ti = 0;
                if (v == 0) t[ti++] = '0';
                while (v) { int d = (int)(v % 16); t[ti++] = (char)(d < 10 ? ('0'+d) : (spec=='X'?('A'+d-10):('a'+d-10))); v /= 16; }
                while (ti--) cb[ci++] = t[ti];
                did = 1; break;
            }
            case 'p': {
                unsigned long long v = (unsigned long long)(uintptr_t)va_arg(ap, void *);
                char t[24]; int ti = 0;
                if (v == 0) t[ti++] = '0';
                while (v) { int d = (int)(v % 16); t[ti++] = (char)(d < 10 ? ('0'+d) : ('a'+d-10)); v /= 16; }
                while (ti--) cb[ci++] = t[ti];
                did = 1; break;
            }
            case '%': cb[ci++] = '%'; did = 1; break;
            default: cb[ci++] = '?'; did = 1; break;
        }
        if (left) { for (size_t k = 0; k < ci; k++) EMIT(cb[k]); }
        else { for (int k = (int)ci; k < width; k++) EMIT(pad ? pad : ' '); for (size_t k = 0; k < ci; k++) EMIT(cb[k]); }
        (void)did;
    }
    #undef EMIT
    if (n > 0) buf[(written < n) ? written : n - 1] = 0;
    return (int)written;
}

int snprintf(char *buf, size_t n, const char *fmt, ...) {
    va_list ap; va_start(ap, fmt);
    int r = vsnprintf(buf, n, fmt, ap);
    va_end(ap);
    return r;
}
int sprintf(char *buf, const char *fmt, ...) {
    va_list ap; va_start(ap, fmt);
    int r = vsnprintf(buf, (size_t)-1, fmt, ap);
    va_end(ap);
    return r;
}
int printf(const char *fmt, ...) {
    char b[OUTBUF];
    va_list ap; va_start(ap, fmt);
    vsnprintf(b, sizeof b, fmt, ap);
    va_end(ap);
    for (char *p = b; *p; p++) __console_out(*p);
    return 0;
}
int fprintf(FILE *stream, const char *fmt, ...) {
    (void)stream;
    char b[OUTBUF];
    va_list ap; va_start(ap, fmt);
    vsnprintf(b, sizeof b, fmt, ap);
    va_end(ap);
    for (char *p = b; *p; p++) __console_out(*p);
    return 0;
}
int puts(const char *s) {
    for (; *s; s++) __console_out(*s);
    __console_out('\n');
    return 0;
}

/* stream stubs (unused; read_file() is host-only and not linked) */
FILE *stdin = (FILE *)0, *stdout = (FILE *)0, *stderr = (FILE *)0;
FILE *fopen(const char *path, const char *mode) { (void)path; (void)mode; return NULL; }
int   fclose(FILE *f) { (void)f; return -1; }
size_t fread(void *ptr, size_t size, size_t nmemb, FILE *f) { (void)ptr; (void)size; (void)nmemb; (void)f; return 0; }
int   fseek(FILE *f, long off, int whence) { (void)f; (void)off; (void)whence; return -1; }
long  ftell(FILE *f) { (void)f; return -1; }
