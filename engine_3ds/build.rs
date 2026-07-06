use std::path::PathBuf;
use std::process::Command;

fn main() {
    let devkitpro = std::env::var("DEVKITPRO").expect("DEVKITPRO must be set");
    let devkitarm = std::env::var("DEVKITARM").expect("DEVKITARM must be set");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let cc = format!("{}/bin/arm-none-eabi-gcc", devkitarm);
    let ar = format!("{}/bin/arm-none-eabi-ar", devkitarm);
    let obj = out_dir.join("ctru_shim.o");
    let archive = out_dir.join("libctru_shim.a");

    // Compile the C file to an object
    let status = Command::new(&cc)
        .args(&[
            "-c",
            "-o",
            obj.to_str().unwrap(),
            "src/ctru_shim.c",
            &format!("-I{}/libctru/include", devkitpro),
            &format!("-I{}/arm-none-eabi/include", devkitarm),
            &format!("-I{}/lib/gcc/arm-none-eabi/16.1.0/include", devkitarm),
            "-march=armv6k",
            "-mtune=mpcore",
            "-mfloat-abi=hard",
            "-mfpu=vfp",
            "-mtp=soft",
            "-DARM11",
            "-D__3DS__",
            "-O3",
        ])
        .status()
        .expect("failed to compile ctru_shim.c");
    assert!(status.success(), "ctru_shim.c compilation failed");

    // Create a static archive from the object
    let status = Command::new(&ar)
        .args(&["crs", archive.to_str().unwrap(), obj.to_str().unwrap()])
        .status()
        .expect("failed to create archive");
    assert!(status.success(), "archive creation failed");

    // Allow-multiple-definition for pthread_atfork:
    // - libsysbase (linked via -lc) defines pthread_atfork but returns ENOSYS
    // - our Rust code overrides it to return 0 (3DS never forks)
    // - this flag prevents the linker error and lets our version win
    println!("cargo:rustc-link-arg=-Wl,--allow-multiple-definition");
    println!("cargo:rustc-link-search=native={}/libctru/lib", devkitpro);
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-arg=-Wl,--start-group");
    println!("cargo:rustc-link-arg=-lctru_shim");
    println!("cargo:rustc-link-arg=-lctru");
    println!("cargo:rustc-link-arg=-Wl,--end-group");

    println!("cargo:rerun-if-changed=src/ctru_shim.c");
}
