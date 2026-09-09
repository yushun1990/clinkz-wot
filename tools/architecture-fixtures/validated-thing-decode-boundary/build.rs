// Non-production observation only. Compile the current parser with a read
// observer; do not copy or replace its parsing rules in this fixture.
use std::{env, fs, path::PathBuf};

fn main() {
    let source = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("../../../td/src/rfc3339.rs");
    println!("cargo:rerun-if-changed={}", source.display());
    let text = fs::read_to_string(source).unwrap();
    let seam = "self.bytes.get(self.pos).copied()";
    assert_eq!(
        text.matches(seam).count(),
        1,
        "parser observation seam changed"
    );
    let observed = text.replace(seam, "{ let byte = self.bytes.get(self.pos).copied(); if byte.is_some() { crate::record_read(self.pos); } byte }");
    // include! cannot accept the module's inner documentation comments.
    let observed = observed.replace("//!", "//");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("observed.rs"),
        observed,
    )
    .unwrap();
}
