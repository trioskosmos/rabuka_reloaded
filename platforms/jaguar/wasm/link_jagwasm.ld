/* Rabuka Jaguar wasm2c-pipeline linker script.
 * - Code + rodata execute in place from cartridge ROM at $802000.
 * - Shell .data/.bss in DRAM at $4000..$41FFF (must stay below TEXT_SCREEN).
 * - 1-bpp text screen fixed at $42000 (hardcoded in boot_jw.S op_list).
 * - malloc arena (wasm linear memory, 22 pages = 1.44MB) at $60000..
 */

ENTRY(_start)

MEMORY
{
  rom   (rx)  : ORIGIN = 0x00802000, LENGTH = 0x003FE000  /* 4MB cart */
  dram  (rw)  : ORIGIN = 0x00004000, LENGTH = 0x0003E000  /* up to text screen */
}

SECTIONS
{
  .text :
  {
    *(.text.startup)
    *(.text*)
    *(.rodata*)
    *(.rdata*)
  } > rom

  .data :
  {
    *(.data*)
    *(.got*)
    *(.got.plt*)
  } > dram AT> rom

  .bss (NOLOAD) :
  {
    *(.bss*)
    *(COMMON)
  } > dram

  /* malloc arena: fixed DRAM window between the text screen and the stack
     ($60000..$1E0000 = 1.5MB; wasm linear memory is 22 pages = 1.44MB) */
  .arena (NOLOAD) :
  {
    __arena_start = 0x00060000;
    __arena_end   = 0x001E0000;
  }

  ASSERT(ADDR(.bss) + SIZEOF(.bss) <= 0x00042000,
         "shell data/bss overflowed into the text screen at $42000")

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
