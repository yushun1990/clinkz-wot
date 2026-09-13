#![no_std]

use serde_json::Number;

pub const LOCAL_ARBITRARY_PRECISION_CFG: bool = cfg!(feature = "arbitrary_precision");

// This is the only documented borrowed decimal-text method on Number.
// It compiles when the downstream graph unifies arbitrary_precision, but
// fails in the required base graph. The fixture intentionally keeps this
// call unconditional to expose the conflicting compile cells.
pub fn borrowed_decimal(number: &Number) -> &str {
    number.as_str()
}
