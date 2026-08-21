/* WAMR classic-interpreter test harness: loads rabuka_wasm.wasm directly
 * (no wasm2c!), registers the four host imports, runs the playable flow.
 * Mirrors native_host.c so output can be compared 1:1. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "wasm_export.h"

static wasm_module_inst_t g_inst = NULL;
static int poll_count = 0;

static int32_t
host_poll_buttons(wasm_exec_env_t env)
{
    (void)env;
    poll_count++;
    return (poll_count % 6 == 0) ? 1 : 0; /* press A every 6 polls */
}

static void
host_clear_screen(wasm_exec_env_t env)
{
    (void)env;
    printf("\n========== CLEAR ==========\n");
}

static void
host_println(wasm_exec_env_t env, int32_t ptr, int32_t len)
{
    if (!g_inst) {
        g_inst = wasm_runtime_get_module_inst(env);
    }
    if (!wasm_runtime_validate_app_addr(g_inst, ptr, len)) {
        printf("<println bad addr %u/%u>\n", (unsigned)ptr, (unsigned)len);
        return;
    }
    const char *p = wasm_runtime_addr_app_to_native(g_inst, ptr);
    fwrite(p, 1, (size_t)len, stdout);
    printf("\n");
}

static void
host_wait_vblank(wasm_exec_env_t env)
{
    (void)env;
}

static NativeSymbol natives[] = {
    { "host_clear_screen", host_clear_screen, "()", NULL },
    { "host_println",      host_println,      "(ii)", NULL },
    { "host_poll_buttons", host_poll_buttons, "()i", NULL },
    { "host_wait_vblank",  host_wait_vblank,  "()", NULL },
};

int
main(int argc, char **argv)
{
    const char *path = argc > 1 ? argv[1] :
        "/mnt/c/rust_targets/wasm32-unknown-unknown/release/rabuka_wasm.wasm";

    RuntimeInitArgs init;
    memset(&init, 0, sizeof(init));
    init.mem_alloc_type = Alloc_With_System_Allocator;
    if (!wasm_runtime_full_init(&init)) {
        printf("runtime init FAILED\n");
        return 1;
    }

    FILE *f = fopen(path, "rb");
    if (!f) { printf("cannot open %s\n", path); return 1; }
    fseek(f, 0, SEEK_END);
    long sz = ftell(f);
    fseek(f, 0, SEEK_SET);
    unsigned char *buf = malloc((size_t)sz); /* module must stay alive */
    if (fread(buf, 1, (size_t)sz, f) != (size_t)sz) { printf("short read\n"); return 1; }
    fclose(f);

    /* register imports BEFORE load: import linking happens in the loader */
    if (!wasm_runtime_register_natives("host", natives,
                                       sizeof(natives) / sizeof(natives[0]))) {
        printf("register_natives FAILED\n");
        return 1;
    }

    char err[128];
    wasm_module_t module = wasm_runtime_load(buf, (uint32_t)sz, err, sizeof(err));
    if (!module) { printf("load FAILED: %s\n", err); return 1; }

    g_inst = wasm_runtime_instantiate(module, 64 * 1024, 0, err, sizeof(err));
    if (!g_inst) { printf("instantiate FAILED: %s\n", err); return 1; }

    printf("instantiated (%ld bytes wasm), running game...\n", sz);
    fflush(stdout);

    wasm_function_inst_t run =
        wasm_runtime_lookup_function(g_inst, "rabuka_wasm_game_run");
    if (!run) { printf("export lookup FAILED\n"); return 1; }

    uint32_t wargs[1] = { 0x5EEDu };
    const char *ex = NULL;
    uint32_t hw = 0;
    wasm_function_inst_t hf = NULL;

    {
        wasm_exec_env_t env = wasm_runtime_create_exec_env(g_inst, 256 * 1024);
        if (!wasm_runtime_call_wasm(env, run, 1, wargs)) {
            ex = wasm_runtime_get_exception(g_inst);
            printf("call FAILED: %s\n", ex ? ex : "(no exception string)");
            return 1;
        }
        printf("\nGAME RETURNED: %u\n", wargs[0]);

        hf = wasm_runtime_lookup_function(g_inst, "rabuka_wasm_heap_highwater");
        if (hf && wasm_runtime_call_wasm(env, hf, 0, &hw))
            printf("HEAP HIGHWATER: %u bytes\n", hw);
    }
    return 0;
}
