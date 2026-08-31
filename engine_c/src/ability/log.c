#include "rabuka.h"
#include <string.h>
#include <stdlib.h>

/* Ported from engine/src/ability/log.rs
   AbilityLogItem verdict buffer — diagnostic, gated by ABILITY_DEBUG.
   Mirror Rust thread_local VERDICT_BUFFER as a simple global ring. */

typedef struct {
    char text[256];
    char kind[32];
    int passed;
} RbLogItem;

#define RB_LOG_BUF_MAX 32   /* trimmed for 64 KB Genesis RAM */

static RbLogItem g_log_buf[RB_LOG_BUF_MAX];
static int g_log_n = 0;
static int g_log_enabled = 0;

void rb_log_set_enabled(int enabled){ g_log_enabled = enabled; }

void rb_log_push_verdict(const char *text, const char *kind, int passed){
    if(!g_log_enabled) return;
    if(g_log_n>=RB_LOG_BUF_MAX) return;
    strncpy(g_log_buf[g_log_n].text, text?text:"", 255);
    strncpy(g_log_buf[g_log_n].kind, kind?kind:"", 31);
    g_log_buf[g_log_n].passed = passed;
    g_log_n++;
}

int rb_log_buffer_len(void){
    if(!g_log_enabled) return 0;
    return g_log_n;
}

void rb_log_clear_verdicts(void){
    if(!g_log_enabled) return;
    g_log_n = 0;
}

int rb_log_drain_verdicts(RbLogItem *out, int max){
    if(!g_log_enabled) return 0;
    int n = g_log_n < max ? g_log_n : max;
    for(int i=0;i<n;i++) out[i]=g_log_buf[i];
    g_log_n = 0;
    return n;
}
