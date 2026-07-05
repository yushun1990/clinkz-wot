//! Foundational TD/TM data types shared across the data-contract layer.
//!
//! This module historically held every cross-cutting value type in one
//! 957-line file. It now splits the vocabulary into cohesive submodules while
//! preserving the flat `data_type::*` re-export surface used across the crate:
//!
//! - [`uri`] — `UriReference`, `FormHref`, `AbsoluteUri`, `BaseUri`, and form
//!   target resolution.
//! - [`operation`] — the `Operation` form-op vocabulary.
//! - [`metadata`] — `Metadata`, `MetadataHelper`, `MultiLanguage`.
//! - [`version`] — `VersionInfo`, `ThingModelVersionInfo`.
//! - [`response`] — `ExpectedResponse`, `AdditionalExpectedResponse`.
//!
//! [`ExtensionMap`] is the cross-cutting "unknown extension fields" container
//! used by every structured value below; it lives at this module's root so the
//! submodules can share it via `super::ExtensionMap`.

use alloc::{collections::BTreeMap, string::String};

use crate::validate::Validate;

pub mod metadata;
pub mod operation;
pub mod response;
pub mod uri;
pub mod version;

pub(crate) use metadata::METADATA_KEYS;
pub use metadata::{Metadata, MetadataHelper, MultiLanguage};
pub use operation::Operation;
pub use response::{AdditionalExpectedResponse, ExpectedResponse};
pub use uri::{
    AbsoluteUri, BaseUri, FormHref, ResolveFormHrefError, ResolvedFormHref, UriReference,
    resolve_form_href,
};
pub use version::{ThingModelVersionInfo, VersionInfo};

/// Extension fields preserved from unknown TD terms.
pub type ExtensionMap = BTreeMap<String, serde_json::Value>;

impl Validate for ExtensionMap {
    fn validate_with_level(
        &self,
        _level: crate::validate::ValidationLevel,
    ) -> Result<(), crate::validate::ValidateError> {
        Ok(())
    }
}
