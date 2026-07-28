use std::path::PathBuf;
use std::process::Command;

fn main() {
    let devkitpro = std::env::var("DEVKITPRO").expect("DEVKITPRO must be set");
    let devkitarm = std::env::var("DEVKITARM").expect("DEVKITARM must be set");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let cc = format!("{}/bin/arm-none-eabi-gcc", devkitarm);
    let ar = format!("{}/bin/arm-none-eabi-ar", devkitarm);
    let _obj = out_dir.join("ctru_shim.o");
    let archive = out_dir.join("libctru_shim.a");

    let objs = [
        "ctru_shim.c",
        "quirc.c",
        "decode.c",
        "identify.c",
        "version_db.c",
    ];
    for src in &objs {
        let src_path = format!("src/{}", src);
        let obj_file = out_dir.join(src.replace(".c", ".o"));
        let status = Command::new(&cc)
            .args(&[
                "-c",
                "-o",
                obj_file.to_str().unwrap(),
                &src_path,
                &format!("-I{}/libctru/include", devkitpro),
                &format!("-I{}/arm-none-eabi/include", devkitarm),
                &format!("-I{}/lib/gcc/arm-none-eabi/16.1.0/include", devkitarm),
                "-I",
                "src/", // for quirc.h, quirc_internal.h
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
            .expect(&format!("failed to compile {}", src));
        assert!(status.success(), "{} compilation failed", src);
    }

    // Create a static archive from the objects
    let mut ar_obj_paths: Vec<String> = Vec::new();
    for src in &objs {
        ar_obj_paths.push(
            out_dir
                .join(src.replace(".c", ".o"))
                .to_string_lossy()
                .to_string(),
        );
    }
    let mut ar_args: Vec<&str> = vec!["crs", archive.to_str().unwrap()];
    for p in &ar_obj_paths {
        ar_args.push(p);
    }
    let status = Command::new(&ar)
        .args(&ar_args)
        .status()
        .expect("failed to create archive");
    assert!(status.success(), "archive creation failed");

    // Tell cargo to rerun this build script if the C source changes
    println!("cargo:rerun-if-changed=src/ctru_shim.c");
    println!("cargo:rerun-if-changed=src/quirc.c");

    // Allow-multiple-definition for pthread_atfork:
    // - libsysbase (linked via -lc) defines pthread_atfork but returns ENOSYS
    // - our Rust code overrides it to return 0 (3DS never forks)
    // - this flag prevents the linker error and lets our version win
    println!("cargo:rustc-link-arg=-Wl,--allow-multiple-definition");
    println!("cargo:rustc-link-search=native={}/libctru/lib", devkitpro);
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-arg=-Wl,--start-group");
    println!("cargo:rustc-link-arg=-lctru_shim");
    println!("cargo:rustc-link-arg=-lcitro2d");
    println!("cargo:rustc-link-arg=-lcitro3d");
    println!("cargo:rustc-link-arg=-lctru");
    println!("cargo:rustc-link-arg=-lndsp");
    println!("cargo:rustc-link-arg=-lvorbisidec");
    println!("cargo:rustc-link-arg=-lm");
    println!("cargo:rustc-link-arg=-Wl,--end-group");

    // --- Card texture conversion ---
    // Rerun if any webp source changed
    let webp_dir = std::path::Path::new("../web_ui/img/cards_webp");
    if webp_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(webp_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "webp").unwrap_or(false) {
                    println!("cargo:rerun-if-changed={}", path.display());
                }
            }
        }
    }
    println!("cargo:rerun-if-changed=scripts/convert_cards.py");
    println!("cargo:rerun-if-changed=romfs/cards_manifest.json");
    if let Ok(entries) = std::fs::read_dir("romfs/cards") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "t3x").unwrap_or(false) {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }
    println!("cargo:rerun-if-changed=romfs/font.bcfnt");
    if let Ok(entries) = std::fs::read_dir("romfs/locales") {
        for entry in entries.flatten() {
            println!("cargo:rerun-if-changed={}", entry.path().display());
        }
    }
}
