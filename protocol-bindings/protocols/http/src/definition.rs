//! HTTP-specific immutable compilation, separate from a checked destination.
use crate::*;
use clinkz_wot_core::{consumer::ConsumerDefinition, host_accounts::*};

#[cfg(feature = "prepare")]
pub(crate) mod preparation;

// Field order keeps the original capacity alive through physical destruction.
struct CompiledRequest {
    #[cfg_attr(not(feature = "prepare"), allow(dead_code))]
    configuration: ConsumerBindingConfiguration,
    instructions: HttpArtifact,
    #[cfg_attr(not(feature = "prepare"), allow(dead_code))]
    dependencies: Vec<u8>,
    _funding: [HostCapacityReservation; 5],
}

/// One explicit, source-free HTTP definition. Contains no endpoint, executor,
/// credentials, policy, instance authority or native resource. Clones retain
/// the same physical instructions and their original account; there is no cache.
#[derive(Clone)]
pub struct HttpRequestDefinition(Arc<CompiledRequest>);
impl HttpRequestDefinition {
    /// Extract the immutable compilation from a mapping made with
    /// `HttpDefinitionCompiler`. Ordinary endpoint-specialized artifacts do not
    /// establish a reusable dependency contract and return `None`.
    pub fn from_prepared(mapping: &ConsumerDefinition) -> Option<Self> {
        mapping
            .artifact()
            .artifact()
            .try_payload::<HttpBoundRequest>(COMPATIBILITY)
            .map(|bound| bound.definition.clone())
    }
    pub fn shares_compilation_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// Opaque owned artifact for one freshly qualified destination. Construction
/// belongs to HTTP preparation; this artifact carries no logical authority.
pub struct HttpBoundRequest {
    definition: HttpRequestDefinition,
    target: [u8; 256],
    target_len: u16,
}

pub(crate) fn instructions(
    envelope: &BindingArtifactEnvelope<HostBindingArtifact>,
) -> Option<HttpArtifact> {
    if let Some(bound) = envelope
        .artifact()
        .try_payload::<HttpBoundRequest>(COMPATIBILITY)
    {
        let mut program = bound.definition.0.instructions;
        program.target = bound.target;
        program.target_len = bound.target_len;
        Some(program)
    } else {
        // The measured inline adapter remains a supported specialized mapping.
        envelope
            .artifact()
            .try_payload::<HttpArtifact>(COMPATIBILITY)
            .copied()
    }
}
