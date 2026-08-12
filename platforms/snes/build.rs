// Build the SNES ROM: assemble crt0.S, compile the bytecode data object, and
// link with the LoROM linker script + llvm-mos-sdk init libraries.
use std::process::Command;

fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    let sdk = std::env::var("LLVM_MOS_SDK").unwrap_or_default();
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    // Compile crt0.S with -mcpu=mosw65816 so the assembler accepts 65816 ops.
    let cc = format!("{}/bin/mos-common-clang", sdk);

    let status = Command::new(&cc)
        .args(["-c", "-mcpu=mosw65816", "-x", "assembler"])
        .arg(format!("{manifest}/crt0.S"))
        .arg("-o")
        .arg(out.join("crt0.o"))
        .status()
        .unwrap();
    assert!(status.success(), "crt0.S compilation failed");

    // Compile the bytecode chunk data object (defines BYTECODE_C0..C3 externs).
    let status = Command::new(&cc)
        .args(["-c", "-mcpu=mosw65816", "-x", "c"])
        .arg(format!("{manifest}/bytecode_data.c"))
        .arg("-o")
        .arg(out.join("bytecode_data.o"))
        .status()
        .unwrap();
    assert!(status.success(), "bytecode_data.c compilation failed");

    // Link crt0.o and bytecode_data.o as the first objects (before Rust code).
    println!("cargo:rustc-link-arg=-Wl,{}", out.join("crt0.o").display());
    println!("cargo:rustc-link-arg=-Wl,{}", out.join("bytecode_data.o").display());

    // LoROM linker script wrapper.
    let wrapper = out.join("lorom-wrapper.ld");
    std::fs::write(
        &wrapper,
        format!(
            "SEARCH_DIR(\"{sdk}/mos-platform/common/lib\");\nINCLUDE \"{manifest}/lorom.ld\"\n"
        ),
    )
    .unwrap();
    println!("cargo:rustc-link-arg=-T{}", wrapper.display());

    // Init libraries from the SDK (startup sections).
    println!("cargo:rustc-link-arg=-Wl,--whole-archive");
    println!("cargo:rustc-link-arg=-l:libinit-stack.a");
    println!("cargo:rustc-link-arg=-l:libcopy-data.a");
    println!("cargo:rustc-link-arg=-l:libzero-bss.a");
    println!("cargo:rustc-link-arg=-l:libexit-loop.a");
    println!("cargo:rustc-link-arg=-Wl,--no-whole-archive");

    println!("cargo:rerun-if-changed=crt0.S");
    println!("cargo:rerun-if-changed=bytecode_data.c");
    println!("cargo:rerun-if-changed=lorom.ld");
    println!("cargo:rerun-if-env-changed=TARGET");
    println!("cargo:rerun-if-env-changed=LLVM_MOS_SDK");
    let _ = (&target, &cc);
}
