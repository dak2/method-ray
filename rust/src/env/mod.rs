//! Environment management for type inference
//!
//! This module provides the core data structures for managing
//! type inference state including global and local environments.

pub mod box_manager;
pub mod global_env;
pub mod local_env;
pub mod method_registry;
pub mod scope;
pub mod type_error;
pub mod vertex_manager;

pub use global_env::GlobalEnv;
pub use local_env::LocalEnv;
