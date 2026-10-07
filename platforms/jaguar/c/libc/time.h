#ifndef LIBCC_TIME_H
#define LIBCC_TIME_H

#include <stddef.h>

/* Bare metal: no wall clock. vm.c includes this header
   but calls nothing from it; the stubs exist so the
   header is complete. */
typedef long time_t;
typedef long clock_t;

#define CLOCKS_PER_SEC 1000000L
#define CLOCK_REALTIME  0
#define CLOCK_MONOTONIC 1

struct timespec { time_t tv_sec; long tv_nsec; };

time_t time(time_t *t);
clock_t clock(void);
int clock_gettime(int clk_id, struct timespec *tp);

#endif
