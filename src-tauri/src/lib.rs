/// Macro to define topic constants for IPC message validation.
///
/// Generates a struct with `&'static str` constants and a `values()` method
/// that returns all topic strings for whitelist registration.
#[macro_export]
macro_rules! define_topics {
    ($name:ident { $($const_name:ident => $value:expr),* $(,)? }) => {
        pub struct $name;

        impl $name {
            $(pub const $const_name: &'static str = $value;)*

            pub fn values() -> Vec<&'static str> {
                vec![$($value),*]
            }
        }
    };
}

pub mod config;
pub mod error;
pub mod state;
pub mod models;
pub mod topics;
pub mod services;
