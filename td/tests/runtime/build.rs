use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=memory.x");
    match env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
        "none" => {
            let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
            fs::copy("memory.x", out.join("memory.x")).unwrap();
            println!("cargo:rustc-link-search={}", out.display());
            println!("cargo:rustc-link-arg=-Tmemory.x");
        }
        // libc supplies native startup, memory intrinsics and write(2). Neither
        // the runner nor its Rust dependency graph links Rust std or malloc.
        "linux" => println!("cargo:rustc-link-lib=c"),
        os => panic!("runtime witness supports Linux and bare ARM, got {os}"),
    }
}
