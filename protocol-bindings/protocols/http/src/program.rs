//! HTTP vocabulary/default interpretation belongs with the HTTP executor.
use super::*;

pub(super) const COMPATIBILITY: BindingArtifactCompatibility =
    BindingArtifactCompatibility::new(*b"clinkz-http-v002");

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Span {
    name: (u16, u16),
    value: (u16, u16),
}
/// Source-free native request instructions. All retained backing is inline.
#[derive(Clone, Copy, Debug)]
pub(super) struct HttpArtifact {
    pub(super) operation: Operation,
    pub(super) target: [u8; 64],
    pub(super) target_len: u8,
    pub(super) method: &'static str,
    pub(super) headers: [Span; 8],
    pub(super) count: u8,
    pub(super) bytes: [u8; 1024],
    // Retain one native layout with and without the preparation facet.
    #[allow(dead_code)]
    pub(super) used: u16,
}
impl Default for HttpArtifact {
    fn default() -> Self {
        Self {
            operation: Operation::ReadProperty,
            target: [0; 64],
            target_len: 0,
            method: "GET",
            headers: [Span::default(); 8],
            count: 0,
            bytes: [0; 1024],
            used: 0,
        }
    }
}
impl HttpArtifact {
    pub(super) fn target(&self) -> &str {
        std::str::from_utf8(&self.target[..self.target_len as usize]).unwrap()
    }
    pub(super) fn method(&self) -> &'static str {
        self.method
    }
    pub(super) fn headers(&self) -> impl Iterator<Item = (&str, &str)> {
        self.headers[..self.count as usize].iter().map(|s| {
            let text = |(start, end)| {
                std::str::from_utf8(&self.bytes[start as usize..end as usize]).unwrap()
            };
            (text(s.name), text(s.value))
        })
    }
    #[cfg(feature = "prepare")]
    pub(super) fn header(&mut self, name: &str, value: &str) -> CoreResult<()> {
        if self.count == 8
            || self.used as usize + name.len() + value.len() > self.bytes.len()
            || hyper::header::HeaderName::from_bytes(name.as_bytes()).is_err()
            || hyper::header::HeaderValue::from_str(value).is_err()
            || [
                "host",
                "connection",
                "content-length",
                "transfer-encoding",
                "content-encoding",
                "trailer",
                "te",
                "upgrade",
                "authorization",
                "proxy-authorization",
                "cookie",
            ]
            .iter()
            .any(|h| name.eq_ignore_ascii_case(h))
            || (name.eq_ignore_ascii_case("accept")
                && (value != "application/json"
                    || self
                        .headers()
                        .any(|(n, _)| n.eq_ignore_ascii_case("accept"))))
            || (name.eq_ignore_ascii_case("content-type")
                && (value != "application/json"
                    || self
                        .headers()
                        .any(|(n, _)| n.eq_ignore_ascii_case("content-type"))))
        {
            return Err(super::compiler::unsupported());
        }
        let start = self.used as usize;
        self.bytes[start..start + name.len()].copy_from_slice(name.as_bytes());
        let middle = start + name.len();
        self.bytes[middle..middle + value.len()].copy_from_slice(value.as_bytes());
        self.used = (middle + value.len()) as u16;
        self.headers[self.count as usize] = Span {
            name: (start as u16, middle as u16),
            value: (middle as u16, self.used),
        };
        self.count += 1;
        Ok(())
    }
}
