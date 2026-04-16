pub mod error;
pub mod state;
pub mod config;
pub mod services;
pub mod models;

/// Macro to define topic constant groups with an `all()` method.
macro_rules! define_topics {
    ($name:ident { $($const_name:ident => $value:expr),* $(,)? }) => {
        pub struct $name;
        impl $name {
            $(pub const $const_name: &'static str = $value;)*

            pub fn all() -> &'static [&'static str] {
                &[$($value),*]
            }
        }
    };
}

pub mod topics;

pub type AppResult<T> = Result<T, error::AppError>;
