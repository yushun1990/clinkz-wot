#![no_std]
//! Protocol-neutral WoT Discovery: Introduction → Exploration → continuation
//! sessions (baseline v4.0 §6 / phase-p1).
//!
//! The data model (`DirectoryQuery`, filters, items, registration carriers,
//! errors) is available on every build. The async trait surface
//! (`DirectoryReader`, `DirectorySession`, `ThingDescriptionResolver`,
//! `DirectoryPublisher`, `DirectoryWatch`, `Discoverer`) and the in-memory
//! reference backend require the `async` feature. Servient integration
//! (`Servient` holding `Arc<dyn Discoverer>`) lands in P3.

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod directory;
pub mod endpoint;
pub mod error;
#[cfg(feature = "async")]
pub mod publisher;
#[cfg(feature = "async")]
pub mod resolver;
#[cfg(feature = "async")]
pub mod session;
pub mod watch;

#[cfg(feature = "async")]
pub mod backend;
#[cfg(feature = "async")]
pub use backend::InMemoryDirectory;
#[cfg(feature = "async")]
mod discoverer;
#[cfg(all(feature = "std", feature = "async"))]
pub mod storage;

pub use directory::{
    CapabilityFilter, ConsistencyMode, ContinuationToken, CountMode, CountValue, DirectoryBatch,
    DirectoryFilter, DirectoryItem, DirectoryPatch, DirectoryQuery, DirectoryRegistration,
    DirectoryStats, LeaseState, LeaseToken, ProjectionMode, RegistrationAck, Revision,
    SummaryFields, ThingFragment,
};
#[cfg(feature = "async")]
pub use directory::{DirectoryReader, DirectorySession};

pub use endpoint::{AuthHint, DiscoveryEndpoint, EndpointKind, IntroductionSource};
#[cfg(feature = "async")]
pub use endpoint::{DirectUrlIntroducer, Introducer};

pub use error::{DiscoveryError, DiscoveryResult};

#[cfg(feature = "async")]
pub use publisher::DirectoryPublisher;

#[cfg(feature = "async")]
pub use resolver::ThingDescriptionResolver;

pub use watch::DirectoryChange;
#[cfg(feature = "std")]
pub use watch::DirectoryWatch;

#[cfg(feature = "async")]
pub use resolver::ThingLinkResolver;
#[cfg(feature = "async")]
pub use session::{DiscoverySession, ProcessState, ThingDiscoveryProcess};

// The Discoverer facade + DiscoveryFilter + DirectoryRef live behind `async`.
#[cfg(feature = "async")]
pub use discoverer::{DirectoryRef, Discoverer, DiscoveryFilter, LocalDiscoverer};

#[cfg(all(feature = "std", feature = "async"))]
pub use storage::{SharedInMemoryDirectory, shared_in_memory_directory};
