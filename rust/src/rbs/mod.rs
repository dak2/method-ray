//! RBS type loading and conversion

pub mod converter;

#[cfg(feature = "ruby-ffi")]
pub mod error;
#[cfg(feature = "ruby-ffi")]
pub mod loader;

pub use converter::RbsTypeConverter;

#[cfg(feature = "ruby-ffi")]
pub use error::RbsError;
#[cfg(feature = "ruby-ffi")]
pub use loader::{register_rbs_methods, RbsLoader, RbsMethodInfo};
