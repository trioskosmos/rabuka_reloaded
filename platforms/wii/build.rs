fn main() {
    let devkitpro = std::env::var("DEVKITPRO").unwrap_or_else(|_| "C:/devkitPro".to_string());
    let devkitppc =
        std::env::var("DEVKITPPC").unwrap_or_else(|_| format!("{}/devkitPPC", devkitpro));
    let gcc = format!("{}/bin/powerpc-eabi-gcc", devkitppc);
    let inc = format!("{}/libogc/include", devkitpro);
    let lib = format!("{}/libogc/lib/wii", devkitpro);
    let out = std::env::var("OUT_DIR").unwrap();

    // Compile entry.c (PAD init + rabuka_main call)
    let eo = format!("{}/entry.o", out);
    cc(&gcc, "entry.c", &eo, &inc, &[]);

    // Compile display.c (all GX + system font)
    let do_ = format!("{}/display.o", out);
    cc(&gcc, "display.c", &do_, &inc, &[]);

    println!("cargo:rustc-link-search=native={}", lib);
    println!("cargo:rustc-link-arg={}", &eo);
    println!("cargo:rustc-link-arg={}", &do_);
}

fn cc(gcc: &str, src: &str, out: &str, inc: &str, extra: &[&str]) {
    use std::process::Command;
    let mut a: Vec<&str> = vec![
        "-mrvl",
        "-meabi",
        "-mhard-float",
        "-c",
        src,
        "-o",
        out,
        "-I",
        inc,
    ];
    for e in extra {
        a.push("-I");
        a.push(e);
    }
    let s = Command::new(gcc).args(&a).status().unwrap();
    assert!(s.success(), "{} failed", src);
}
