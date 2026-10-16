//! Matching portable ownership evidence reuses the production Runtime tests.
#![cfg(not(feature = "std"))]
#![allow(dead_code)]

mod consumer_property_read {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../servient/tests/consumer_property_read/support.rs"
    ));
    mod ownership {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../servient/tests/consumer_property_read/ownership.rs"
        ));
    }
}
