#![no_std]

extern crate alloc;

#[cfg(test)]
mod tests {
    use validated_thing_feature_access_borrow::{LOCAL_ARBITRARY_PRECISION_CFG, borrowed_decimal};
    use validated_thing_feature_access_portable::decimal_byte;

    #[test]
    fn dependency_feature_is_unified_but_not_exposed_as_local_cfg() {
        assert!(!LOCAL_ARBITRARY_PRECISION_CFG);
        let number: serde_json::Number =
            serde_json::from_str("9007199254740993.0000000000000000000001").unwrap();
        assert_eq!(borrowed_decimal(&number), number.as_str());
        assert_eq!(
            borrowed_decimal(&number),
            "9007199254740993.0000000000000000000001"
        );
        for (position, byte) in borrowed_decimal(&number).bytes().enumerate() {
            assert_eq!(decimal_byte(&number, position).unwrap(), Some(byte));
        }
        assert_eq!(
            decimal_byte(&number, borrowed_decimal(&number).len()).unwrap(),
            None
        );
    }

    #[test]
    fn long_exponent_late_byte_is_accessible_without_bulk_display() {
        let mut source = alloc::string::String::from("1e+");
        source.extend(core::iter::repeat_n('0', 65_536));
        source.push('1');
        let number: serde_json::Number = serde_json::from_str(&source).unwrap();
        assert_eq!(borrowed_decimal(&number), source);
        assert_eq!(decimal_byte(&number, source.len() - 1).unwrap(), Some(b'1'));
        assert_eq!(decimal_byte(&number, source.len()).unwrap(), None);
    }
}
