use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use clinkz_wot_core::Payload;
use zenoh::{bytes::ZBytes, sample::Sample};

const DEFAULT_CONTENT_TYPE: &str = "application/octet-stream";

pub(super) fn payload_from_sample(sample: &Sample, content_type_hint: Option<&str>) -> Payload {
    let content_type = content_type_hint
        .map(ToString::to_string)
        .unwrap_or_else(|| sample.encoding().to_string());
    let content_type = if content_type.is_empty() {
        String::from(DEFAULT_CONTENT_TYPE)
    } else {
        content_type
    };

    Payload::new(bytes_from_zbytes(sample.payload()), content_type)
}

fn bytes_from_zbytes(bytes: &ZBytes) -> Vec<u8> {
    bytes.to_bytes().into_owned()
}
