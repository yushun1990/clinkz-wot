#[path = "../plan_builder.rs"]
mod plan_builder;
use std::{env, fs, path::PathBuf};
fn main() {
    let output = plan_builder::build();
    let plan = &output.logical_plans()[0];
    let target = output.artifacts()[0].artifact().payload().target().unwrap();
    assert_eq!(plan.resolved_target(), target);
    let generated = format!(
        "pub static IMAGE: runtime_contracts_probe::PlanImage = runtime_contracts_probe::PlanImage {{ operation: runtime_contracts_probe::Operation::{:?}, property: {:?}, form_index: {}, resolved_target: {:?}, binding_target: {:?}, content_type: {:?} }};",
        plan.operation(),
        plan.property_name(),
        plan.form_index(),
        plan.resolved_target(),
        target,
        plan.content_type()
    );
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("plan.rs"),
        generated,
    )
    .unwrap();
    println!("cargo:rerun-if-changed=../plan_builder.rs");
}
