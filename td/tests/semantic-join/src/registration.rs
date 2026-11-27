//! Complete static registration for the production semantic join tests.
use clinkz_wot_core::*;
use clinkz_wot_planning::{binding_compiler::*, binding_registration::*};
use clinkz_wot_property_read_binding_fixture::{ManualMockBinding, MockCompiler};

pub type Registration = StaticBindingPreparation<ManualMockBinding>;

pub fn registration() -> Registration {
    let compatibility = BindingArtifactCompatibility::new([0x41; 16]);
    let identity = BindingRegistrationIdentity::new(
        BindingId::new(7),
        BindingGeneration::INITIAL,
        BindingConfigurationDigest::new([0x52; 32]),
        compatibility,
        0,
    );
    let input = StaticBindingPreparationInput::new(
        identity,
        BindingRegistrationCapabilities::producer_property_read(),
        BindingExecutionSupport::application_static(),
        StaticBindingCompilerRegistration::new(MockCompiler::new(compatibility)),
        ManualMockBinding::new(compatibility, 0),
        BindingResourceDeclarations::new(
            BindingLifetimeFootprint::new(4, 4096),
            BindingLifetimeFootprint::new(4, 4096),
        ),
        BindingIngressPolicy::hidden(),
        BindingStatusPolicy::new(2, 128),
    );
    match StaticBindingPreparation::new(input) {
        Ok(registration) => registration,
        Err(_) => panic!("complete Producer fixture registration must validate"),
    }
}
