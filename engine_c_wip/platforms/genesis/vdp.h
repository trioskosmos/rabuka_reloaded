#ifndef RABUKA_GENESIS_VDP_H
#define RABUKA_GENESIS_VDP_H

void vdp_init(void);
void vdp_clear(uint8_t color_idx);
void vdp_puts(int row, int col, const char *s);
void vdp_write_result_word(int addr, uint16_t val); /* poke a 16-bit result into SRAM */

#endif
