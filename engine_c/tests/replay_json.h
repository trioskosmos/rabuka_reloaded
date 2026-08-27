/* tests/replay_json.h — tiny JSON loader for replay harness (no cJSON dep)
   Minimal jsmn-like scanner: only handles flat objects with string/int/array
   values needed for replay fixtures. Future: replace with full jsmn. */
#ifndef REPLAY_JSON_H
#define REPLAY_JSON_H
#include <string.h>
#include <stdlib.h>
static const char *rj_find(const char *json, const char *key){
    char pat[64]; snprintf(pat,sizeof(pat),"\"%s\"",key);
    const char *p=strstr(json,pat);
    if(!p) return NULL;
    p=strchr(p,':'); if(!p) return NULL;
    return p+1;
}
static int rj_int(const char *json, const char *key, int def){
    const char *p=rj_find(json,key);
    if(!p) return def;
    while(*p==' '||*p=='\t'||*p=='\n'||*p=='"') p++;
    return atoi(p);
}
#endif
