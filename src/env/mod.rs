pub mod global_env;
pub mod local_env;
pub mod scope;

pub use global_env::{GlobalEnv, TypeError};
pub use local_env::LocalEnv;
pub use scope::{Scope, ScopeId, ScopeKind, ScopeManager};
