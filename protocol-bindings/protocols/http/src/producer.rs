//! Source-free native Producer configuration and instructions. No TD or
//! Planning dependency is needed to retain these runtime values.
use super::*;

/// Immutable listen operands for the initial loopback HTTP Producer adapter.
/// Construction is pure. A native installation driver is not yet supplied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HttpProducerConfig {
    pub(super) address: SocketAddr,
}
impl HttpProducerConfig {
    pub fn for_loopback(address: SocketAddr) -> CoreResult<Self> {
        if !address.ip().is_loopback() || address.port() == 0 {
            return Err(producer_unsupported());
        }
        Ok(Self { address })
    }
    pub fn listen_address(&self) -> SocketAddr {
        self.address
    }
}

/// Owned immutable native instructions, without a Copy/inline requirement on
/// any shared compiler interface. This concrete first program is inline.
/// Possession grants no installation, publication, dispatch or accept authority.
#[derive(Debug, Eq, PartialEq)]
pub struct HttpProducerProgram {
    pub(super) address: SocketAddr,
    pub(super) target: [u8; 64],
    pub(super) target_len: u8,
    pub(super) authority_end: u8,
    pub(super) origin: [u8; 64],
    pub(super) origin_len: u8,
}
impl HttpProducerProgram {
    pub fn listen_address(&self) -> SocketAddr {
        self.address
    }
    pub fn target(&self) -> &str {
        std::str::from_utf8(&self.target[..self.target_len as usize]).unwrap()
    }
    pub fn authority(&self) -> &str {
        &self.target()[7..self.authority_end as usize]
    }
    pub fn path_and_query(&self) -> &str {
        std::str::from_utf8(&self.origin[..self.origin_len as usize]).unwrap()
    }
}
pub(super) fn producer_unsupported() -> CoreError {
    CoreError::UnsupportedOperation(
        ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
            .with_redacted_cause(621, "unsupported HTTP Producer mapping"),
    )
}
