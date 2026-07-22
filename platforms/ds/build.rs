use std::path::PathBuf;
use std::process::Command;

fn main() {
    let devkitpro_raw = std::env::var("DEVKITPRO").unwrap_or_else(|_| "C:/devkitPro".to_string());
    // Convert MSYS2 paths (e.g. /opt/devkitpro) to Windows paths (C:/devkitPro)
    let devkitpro = if devkitpro_raw.starts_with('/') || devkitpro_raw.starts_with('\\') {
        // Try common Windows install locations
        let candidates = ["C:/devkitPro", "D:/devkitPro"];
        candidates
            .iter()
            .find(|c| std::path::Path::new(c).exists())
            .map(|s| s.to_string())
            .unwrap_or(devkitpro_raw)
    } else {
        devkitpro_raw
    };
    let devkitarm =
        std::env::var("DEVKITARM").unwrap_or_else(|_| format!("{}/devkitARM", devkitpro));

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let cc = format!("{}/bin/arm-none-eabi-gcc", devkitarm);
    let ar = format!("{}/bin/arm-none-eabi-ar", devkitarm);
    let obj = out_dir.join("nds_shim.o");
    let archive = out_dir.join("libnds_shim.a");

    let include_nds = format!("{}/libnds/include", devkitpro);
    let include_devkitarm = format!("{}/arm-none-eabi/include", devkitarm);
    let include_calico = format!("{}/calico/include", devkitpro);

    let status = Command::new(&cc)
        .args(&[
            "-c",
            "-o",
            obj.to_str().unwrap(),
            "src/nds_shim.c",
            &format!("-I{}", include_nds),
            &format!("-I{}", include_devkitarm),
            &format!("-I{}", include_calico),
            "-march=armv5te",
            "-mtune=arm9tdmi",
            "-marm",
            "-mthumb-interwork",
            "-DARM9",
            "-D__NDS__",
            "-Os",
        ])
        .status()
        .expect("failed to compile nds_shim.c");
    if !status.success() {
        panic!("nds_shim.c compilation failed");
    }

    let status = Command::new(&ar)
        .args(&["crs", archive.to_str().unwrap(), obj.to_str().unwrap()])
        .status()
        .expect("failed to create archive");
    assert!(status.success(), "archive creation failed");

    println!("cargo:rerun-if-changed=src/nds_shim.c");
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=nds_shim");

    let libnds_lib = format!("{}/libnds/lib", devkitpro);
    println!("cargo:rustc-link-search=native={}", libnds_lib);
    println!("cargo:rustc-link-lib=static=nds9");

    let calico_lib = format!("{}/calico/lib", devkitpro);
    println!("cargo:rustc-link-search=native={}", calico_lib);
    println!("cargo:rustc-link-lib=static=calico_ds9");

    let arm_lib = format!("{}/arm-none-eabi/lib", devkitarm);
    println!("cargo:rustc-link-search=native={}", arm_lib);
    println!("cargo:rustc-link-lib=static=sysbase");
    println!("cargo:rustc-link-lib=static=c");

    let gcc_lib = format!("{}/lib/gcc/arm-none-eabi/16.1.0", devkitarm);
    println!("cargo:rustc-link-search=native={}", gcc_lib);
    println!("cargo:rustc-link-lib=static=gcc");
}
