use std::{env, fs, path::PathBuf};

fn main() {
    let source = "../../../planning/src/consumer_property_read.rs";
    let appendix = "probes/quiescent.rs";
    println!("cargo:rerun-if-changed={source}");
    println!("cargo:rerun-if-changed={appendix}");
    // Compile the current production program unchanged, except its leading
    // module documentation (include! cannot contain inner doc comments), with
    // a private experimental ownership adapter appended in the same module.
    let production = fs::read_to_string(source).unwrap();
    let body = production
        .lines()
        .skip_while(|line| line.starts_with("//!") || line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let output = format!("{body}\n{}", fs::read_to_string(appendix).unwrap());
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    // Preserve the source's existing relative test-support module path too.
    let support = "../../../planning/tests/support/consumer_property_read.rs";
    println!("cargo:rerun-if-changed={support}");
    let support_dir = out.join("../tests/support");
    fs::create_dir_all(&support_dir).unwrap();
    fs::copy(support, support_dir.join("consumer_property_read.rs")).unwrap();
    fs::write(out.join("quiescent.rs"), output).unwrap();
}
