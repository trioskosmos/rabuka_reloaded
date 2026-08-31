#ifndef LIBCC_STDLIB_H
#define LIBCC_STDLIB_H

#include <stddef.h>

void  *malloc(size_t n);
void  *calloc(size_t n, size_t size);
void  *realloc(void *p, size_t n);
void   free(void *p);

int    atoi(const char *s);
long   atol(const char *s);
long   strtol(const char *s, char **end, int base);
long   strtoul(const char *s, char **end, int base);
long long strtoll(const char *s, char **end, int base);

int    abs(int v);
long   labs(long v);

int    rand(void);
void   srand(unsigned int seed);

void   exit(int code);
void   abort(void);

#endif
