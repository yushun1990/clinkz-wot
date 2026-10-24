use std::process::Command;

#[test]
fn current_authority_state_and_dependency_projections_agree() {
    let output = Command::new(env!("CARGO_BIN_EXE_clinkz-wot-design-check"))
        .arg("check")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
