use core::fmt;

pub type DiscoveryResult<T> = Result<T, DiscoveryError>;

/// Fixed diagnostics never retain URLs, headers, credentials or remote bodies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiscoveryError {
    Configuration,
    DestinationDenied,
    Transport,
    Trust,
    Authentication,
    Forbidden,
    HttpStatus(u16),
    Protocol,
    UnsupportedMedia,
    InvalidJson,
    InvalidThingDescription,
    AdmissionLimit,
    Limit,
    InvalidContinuation,
    CollectionChanged,
    Cancelled,
    Timeout,
    Interrupted,
}
impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Discovery: {self:?}")
    }
}
#[cfg(feature = "std")]
impl std::error::Error for DiscoveryError {}
