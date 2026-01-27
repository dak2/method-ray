//! Variable Handlers - Processing Ruby variables
//!
//! This module is responsible for:
//! - Local variable read/write (x, x = value)
//! - Instance variable read/write (@name, @name = value)
//! - self node handling

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use crate::types::Type;

/// Install local variable write: x = value
pub fn install_local_var_write(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    var_name: String,
    value_vtx: VertexId,
) -> VertexId {
    let var_vtx = genv.new_vertex();
    lenv.new_var(var_name, var_vtx);
    changes.add_edge(value_vtx, var_vtx);
    var_vtx
}

/// Install local variable read: x
pub fn install_local_var_read(lenv: &LocalEnv, var_name: &str) -> Option<VertexId> {
    lenv.get_var(var_name)
}

/// Install instance variable write: @name = value
pub fn install_ivar_write(
    genv: &mut GlobalEnv,
    ivar_name: String,
    value_vtx: VertexId,
) -> VertexId {
    genv.scope_manager
        .set_instance_var_in_class(ivar_name, value_vtx);
    value_vtx
}

/// Install instance variable read: @name
pub fn install_ivar_read(genv: &GlobalEnv, ivar_name: &str) -> Option<VertexId> {
    genv.scope_manager.lookup_instance_var(ivar_name)
}

/// Install self node
/// Uses the fully qualified name if available (e.g., Api::V1::User instead of just User)
pub fn install_self(genv: &mut GlobalEnv) -> VertexId {
    if let Some(qualified_name) = genv.scope_manager.current_qualified_name() {
        genv.new_source(Type::instance(&qualified_name))
    } else {
        genv.new_source(Type::instance("Object"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_self_at_top_level() {
        let mut genv = GlobalEnv::new();

        let vtx = install_self(&mut genv);
        assert_eq!(genv.get_source(vtx).unwrap().ty.show(), "Object");
    }

    #[test]
    fn test_local_var_read_not_found() {
        let lenv = LocalEnv::new();

        assert_eq!(install_local_var_read(&lenv, "unknown"), None);
    }
}
