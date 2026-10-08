#![no_std]
// These public types/functions are comparison seams, not proposed product APIs.
#[cfg(feature = "std")]
extern crate std;

macro_rules! projected {
    ($($module:ident),* $(,)?) => {$(
        pub mod $module {
            include!(concat!(env!("OUT_DIR"), "/", stringify!($module), ".rs"));
        }
        pub use $module::*;
    )*};
}

projected!(
    identity, error, operation, metadata, artifact, response, selection
);

mod views;
pub use views::*;
