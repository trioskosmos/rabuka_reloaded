/* Native harness: run the REAL transpiled engine through a full playable
 * game (mode->deck->mulligan->match) by auto-pressing A, then report the
 * bump allocator high-water mark. This measures the exact binary we ship,
 * including the mulligan path that OOM'd the Dreamcast at 512KB. */
#include <stdio.h>
#include <stdint.h>
#include "rabuka_wasm.h"

struct w2c_host { int unused; };
w2c_0x24rabuka__wasm0x2Ewasm g_inst;
struct w2c_host g_host;

void w2c_host_host_clear_screen(struct w2c_host *h) { (void)h; }
void w2c_host_host_println(struct w2c_host *h, uint32_t p, uint32_t n) { (void)h; (void)p; (void)n; }

/* toggle A pressed/released every call -> repeated "just pressed A" edges,
 * driving mode select -> deck select -> mulligan -> every turn */
uint32_t w2c_host_host_poll_buttons(struct w2c_host *h) {
    (void)h;
    static int t = 0;
    return (t++ & 1) ? 1u : 0u;
}
void w2c_host_host_wait_vblank(struct w2c_host *h) { (void)h; }

int main(void) {
    wasm_rt_init();
    wasm2c_0x24rabuka__wasm0x2Ewasm_instantiate(&g_inst, &g_host);
    uint32_t r = w2c_0x24rabuka__wasm0x2Ewasm_rabuka_wasm_game_run(&g_inst, 0x5EEDu);
    uint32_t hw = w2c_0x24rabuka__wasm0x2Ewasm_rabuka_wasm_heap_highwater(&g_inst);
    printf("game_run result=%u heap_highwater=%u bytes (%u KB)\n", r, hw, hw / 1024);
    return 0;
}
