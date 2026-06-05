macro_rules! impl_builder_default {
    ($($builder:ty),* $(,)?) => {
        $(
            impl Default for $builder {
                fn default() -> Self {
                    Self::new()
                }
            }
        )*
    };
}

pub mod affordance;
pub mod context;
pub mod data_schema;
pub mod form;
pub mod link;
pub mod security_scheme;
pub mod util;
