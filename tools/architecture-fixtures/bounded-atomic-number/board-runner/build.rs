use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=link.x");

    if env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("arm") {
        return;
    }

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    fs::copy("link.x", out.join("link.x")).expect("copy board linker script");

    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rustc-link-arg-bin=wp100-atomic-number-m4=-Tlink.x");
    println!(
        "cargo:rustc-link-arg-bin=wp100-atomic-number-m4=-Map={}",
        out.join("wp100-atomic-number-m4.map").display()
    );
}
