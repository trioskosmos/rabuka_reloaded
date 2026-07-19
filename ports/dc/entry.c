extern int game_main(void);

// C ABI on SH-ELF adds a leading underscore.
// "arch_main" in C becomes "_arch_main" in the symbol table.
__attribute__((used, section(".text")))
int arch_main(void) {
    return game_main();
}
