/* Embeds rabuka_wasm.wasm into the ELF as a read-only data section.
 * Compiled by sh-elf-gcc with the same flags as the rest of the build,
 * so endianness/model match automatically (objcopy -O elf32-shl produced
 * objects ld refused to merge against the -ml link). */
    .section .rodata
    .global _binary_rabuka_wasm_wasm_start
_binary_rabuka_wasm_wasm_start:
    .incbin "rabuka_wasm.wasm"
    .global _binary_rabuka_wasm_wasm_end
_binary_rabuka_wasm_wasm_end:
    .balign 4
