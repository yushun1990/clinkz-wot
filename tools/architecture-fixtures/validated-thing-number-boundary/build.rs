// Observe the installed Rust float parser's lexical front end, not a new
// product parser. No upstream source or representation is vendored or pinned.
use std::{env, fs, path::PathBuf, process::Command};

fn replace_once(text: String, old: &str, new: &str) -> String {
    assert_eq!(
        text.matches(old).count(),
        1,
        "observation seam changed: {old}"
    );
    text.replace(old, new)
}

fn main() {
    println!("cargo:rerun-if-env-changed=RUSTC");
    let rustc = env::var_os("RUSTC").unwrap();
    let output = Command::new(&rustc)
        .args(["--print", "sysroot"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let sysroot = String::from_utf8(output.stdout).unwrap();
    let source =
        PathBuf::from(sysroot.trim()).join("lib/rustlib/src/rust/library/core/src/num/dec2flt");
    let read = |name: &str| {
        let path = source.join(name);
        println!("cargo:rerun-if-changed={}", path.display());
        fs::read_to_string(&path)
            .unwrap_or_else(|error| {
                panic!(
                    "{}: {error}; install rust-src for this observation fixture",
                    path.display()
                )
            })
            .replace("//!", "//")
    };
    let mut common = read("common.rs");
    common = replace_once(
        common,
        "tmp.copy_from_slice(&self[..8]);",
        "crate::record(self, 8); tmp.copy_from_slice(&self[..8]);",
    );
    common = replace_once(common, "s.split_first()", "crate::split_first(s)");

    let mut parser = read("parse.rs");
    // Non-finite spellings cannot be JSON Numbers. Keep the complete finite
    // lexical parser; do not duplicate its loops, decisions, or Decimal fields.
    let end = parser
        .find("/// Try to parse a special, non-finite float.")
        .expect("finite parser seam changed");
    parser.truncate(end);
    parser = replace_once(parser, "use crate::num::dec2flt::float::RawFloat;", "");
    parser = parser.replace("crate::num::dec2flt::", "super::");
    assert_eq!(parser.matches("s.split_first()").count(), 4);
    assert_eq!(parser.matches("p.split_first()").count(), 1);
    parser = parser.replace("s.split_first()", "crate::split_first(s)");
    parser = parser.replace("p.split_first()", "crate::split_first(p)");
    parser = replace_once(parser, "s.first()", "crate::first(s)");

    let decimal = read("decimal.rs");
    let start = decimal
        .find("#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]")
        .expect("Decimal seam changed");
    let end = decimal
        .find("impl Decimal {")
        .expect("Decimal impl seam changed");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(out.join("common.rs"), common).unwrap();
    fs::write(out.join("parse.rs"), parser).unwrap();
    fs::write(out.join("decimal.rs"), &decimal[start..end]).unwrap();
}
