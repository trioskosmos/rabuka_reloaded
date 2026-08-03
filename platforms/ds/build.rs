use std::path::PathBuf;
use std::process::Command;

fn main() {
    let blocksds = std::env::var("BLOCKSDS")
        .unwrap_or_else(|_| "/opt/wonderful/thirdparty/blocksds/core".to_string());
    let blocksds = PathBuf::from(&blocksds);
    let wonderful =
        std::env::var("WONDERFUL_TOOLCHAIN").unwrap_or_else(|_| "/opt/wonderful".to_string());
    let gcc_bin = PathBuf::from(&wonderful).join("toolchain/gcc-arm-none-eabi/bin");
    let gcc = gcc_bin.join("arm-none-eabi-gcc");
    let ar = gcc_bin.join("arm-none-eabi-ar");

    let libnds = blocksds.join("libs/libnds");
    assert!(
        libnds.join("lib/libnds9.a").exists(),
        "libnds9.a not found under {libnds:?}. Set BLOCKSDS to the BlocksDS core dir."
    );

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let obj = out.join("nds_shim.o");
    let archive = out.join("libnds_shim.a");

    let inc = [
        libnds.join("include").display().to_string(),
        wonderful.clone() + "/toolchain/gcc-arm-none-eabi/arm-none-eabi/include",
    ];

    let status = Command::new(&gcc)
        .args(&[
            "-c",
            "-o",
            obj.to_str().unwrap(),
            "src/nds_shim.c",
            "-mthumb-interwork",
            "-march=armv5te",
            "-mcpu=arm946e-s",
            "-DARM9",
            "-D__NDS__",
            "-D__BLOCKSDS__",
            "-Os",
        ])
        .args(inc.iter().flat_map(|p| ["-I".to_string(), p.clone()]))
        .status()
        .expect("failed to run arm-none-eabi-gcc");
    assert!(status.success(), "nds_shim.c compilation failed");

    let status = Command::new(&ar)
        .args(["crs", archive.to_str().unwrap(), obj.to_str().unwrap()])
        .status()
        .expect("failed to run arm-none-eabi-ar");
    assert!(status.success(), "archive creation failed");

    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=nds_shim");

    println!(
        "cargo:rustc-link-search=native={}",
        libnds.join("lib").display()
    );
    let profile = std::env::var("PROFILE").unwrap_or_default();
    println!(
        "cargo:rustc-link-lib=static={}",
        if profile == "debug" { "nds9d" } else { "nds9" }
    );
    println!("cargo:rerun-if-changed=src/nds_shim.c");
    println!("cargo:rerun-if-env-changed=BLOCKSDS");
}
