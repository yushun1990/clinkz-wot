#![cfg_attr(not(feature = "std"), no_std)]
//! Explicit W3C WoT Discovery clients. Retrieved metadata grants no authority.
//! The Host `http` feature supplies direct TD GET and bounded Directory listing.
//! Applications separately select and prepare interactions with the Engine.

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod error;
pub use error::{DiscoveryError, DiscoveryResult};
#[cfg(feature = "http")]
pub mod http;
