#ifndef LIBCC_STDIO_H
#define LIBCC_STDIO_H

#include <stddef.h>
#include <stdarg.h>

typedef struct { int _dummy; } FILE;
#define EOF (-1)
#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2

extern FILE *stdin;
extern FILE *stdout;
extern FILE *stderr;

int printf(const char *fmt, ...);
int fprintf(FILE *stream, const char *fmt, ...);
int sprintf(char *buf, const char *fmt, ...);
int snprintf(char *buf, size_t n, const char *fmt, ...);
int vsnprintf(char *buf, size_t n, const char *fmt, va_list ap);
int puts(const char *s);

/* Stream stubs (unused on bare metal; read_file() is not called). */
FILE *fopen(const char *path, const char *mode);
int   fclose(FILE *f);
size_t fread(void *ptr, size_t size, size_t nmemb, FILE *f);
int   fseek(FILE *f, long off, int whence);
long  ftell(FILE *f);

#endif
