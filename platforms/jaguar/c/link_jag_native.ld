/* Rabuka Jaguar native-C linker script.
 * - Code + rodata execute in place from cartridge ROM at $802000 (4 MB cart).
 * - .data (small) in DRAM at $4000, copied from its ROM LMA by boot_native.S.
 * - The 1-bpp text screen stays fixed at $42000 (hardcoded in boot_native.S
 *   op_list); .data must not run into it.
 * - .bss (GameState ~1.04 MB + rb arena + sys heap) at $48000, well clear
 *   of the text screen, ending below $1E0000 so the stack ($1FFF00 top,
 *   set by boot_native.S) has ~128 KB.
 */

ENTRY(_start)

MEMORY
{
  rom  (rx)  : ORIGIN = 0x00802000, LENGTH = 0x003FE000  /* 4MB cart */
  dram (rw)  : ORIGIN = 0x00004000, LENGTH = 0x001FC000  /* 2MB DRAM */
}

SECTIONS
{
  .text :
  {
    *(.text.startup)
    *(.text*)
    *(.rodata*)
    *(.rdata*)
    *(.romdata)
  } > rom

  .data 0x00004000 :
  {
    *(.data*)
    *(.got*)
    *(.got.plt*)
  } > dram AT> rom

  .bss 0x00048000 (NOLOAD) :
  {
    *(.bss*)
    *(COMMON)
  } > dram

  ASSERT(ADDR(.data) + SIZEOF(.data) <= 0x00042000,
         "shell .data overflowed into the text screen at $42000")
  ASSERT(ADDR(.bss) + SIZEOF(.bss) <= 0x001E0000,
         ".bss (GameState + arenas) overflowed into the stack below $1F0000")

  /DISCARD/ :
  {
    *(.comment*)
    *(.note*)
    *(.eh_frame*)
  }

  __data_lma = LOADADDR(.data);
  __data_vma = ADDR(.data);
  __data_end = ADDR(.data) + SIZEOF(.data);
  __bss_start = ADDR(.bss);
  __bss_end = ADDR(.bss) + SIZEOF(.bss);
}
