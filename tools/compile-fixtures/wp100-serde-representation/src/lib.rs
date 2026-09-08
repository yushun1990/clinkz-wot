#![no_std]

extern crate alloc;

/// Forces both public dependency identities into this downstream build.
pub const fn supported_representation_surface() -> usize {
    core::mem::size_of::<clinkz_wot_td::thing::Thing>() + core::mem::size_of::<serde_json::Value>()
}

/// Preserves the concrete #71 IndexMap-capacity counterexample in the
/// downstream feature cell. The TD dependency must reject the cell before
/// this value could cross its retained-source boundary.
#[cfg(feature = "preserve-order")]
pub fn rejected_overcapacity_map() -> serde_json::Value {
    let mut map = serde_json::Map::with_capacity(16_384);
    map.insert(
        alloc::string::String::from("value"),
        serde_json::Value::Bool(true),
    );
    serde_json::Value::Object(map)
}

/// Preserves the concrete #71 String-backed Number counterexample in the
/// downstream feature cell. The TD dependency must reject this representation.
#[cfg(feature = "arbitrary-precision")]
pub fn rejected_long_number() -> serde_json::Value {
    let digits = alloc::string::String::from("9").repeat(16_384);
    serde_json::from_str(&digits).expect("digits form a JSON number")
}
