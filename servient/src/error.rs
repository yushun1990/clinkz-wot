use clinkz_wot_core::CoreError;
use core::fmt;
pub type ServientResult<T> = Result<T, ServientError>;
#[derive(Debug)]
#[non_exhaustive]
pub enum ServientError {
    Serve(CoreError),
    MissingThingId,
}
impl From<CoreError> for ServientError {
    fn from(error: CoreError) -> Self {
        Self::Serve(error)
    }
}
impl ServientError {
    pub const fn as_core(&self) -> Option<&CoreError> {
        match self {
            Self::Serve(error) => Some(error),
            _ => None,
        }
    }
}
impl fmt::Display for ServientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serve(error) => error.fmt(f),
            Self::MissingThingId => f.write_str("Producer TD is missing an id"),
        }
    }
}
#[cfg(feature = "std")]
impl std::error::Error for ServientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.as_core().map(|e| e as _)
    }
}
