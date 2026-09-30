// Compile the current TD source, changing only its private schema validation
// seam. No generated source is committed or used by the production TD crate.
use std::{env, fs, path::Path};

fn copy_source(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    println!("cargo:rerun-if-changed={}", source.display());
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let destination = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_source(&entry.path(), &destination);
        } else {
            println!("cargo:rerun-if-changed={}", entry.path().display());
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}

fn main() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fixture.join("../../../td/src").canonicalize().unwrap();
    let output = env::var_os("OUT_DIR").unwrap();
    let candidate = Path::new(&output).join("candidate");
    copy_source(&source, &candidate);

    let lib = fs::read_to_string(candidate.join("lib.rs")).unwrap();
    let mut lib = lib.replace("//!", "//").replace("#![no_std]", "");
    for (module, file) in [
        ("schema_kernel", "kernel.rs"),
        ("schema_access", "typed_access.rs"),
    ] {
        let path = fixture.join("src").join(file);
        println!("cargo:rerun-if-changed={}", path.display());
        lib.push_str(&format!(
            "\n#[path = {path:?}]\n#[allow(dead_code)]\nmod {module};\n"
        ));
    }
    fs::write(candidate.join("lib.rs"), lib).unwrap();

    let path = candidate.join("components/data_schema.rs");
    let schema = fs::read_to_string(&path).unwrap();
    let seam = "impl Validate for DataSchema {";
    assert_eq!(schema.matches(seam).count(), 1, "schema seam changed");
    let (types_and_builders, rules) = schema.split_once(seam).unwrap();
    assert!(rules.ends_with("}\n"), "schema rule boundary changed");
    // Fail visibly if builders begin using a helper currently owned by this
    // final validation region, rather than silently replacing more source.
    for helper in [
        "validate_schema_context(",
        "validate_numeric_bounds(",
        "validate_ordered(",
        "format_schema_path(",
    ] {
        assert!(
            !types_and_builders.contains(helper),
            "new caller outside schema seam"
        );
    }
    fs::write(
        &path,
        format!(
            "{types_and_builders}\n{}",
            include_str!("src/public_adapter.rs")
        ),
    )
    .unwrap();
    println!("cargo:rerun-if-changed=src/public_adapter.rs");
}
