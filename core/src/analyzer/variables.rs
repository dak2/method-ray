//! Variable Handlers - Processing Ruby variables
//!
//! This module is responsible for:
//! - Local variable read/write (x, x = value)
//! - Instance variable read/write (@name, @name = value)
//! - Class variable read/write (@@name, @@name = value)
//! - self node handling

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use crate::types::Type;

/// Install local variable write: x = value
pub(crate) fn install_local_var_write(
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
pub(crate) fn install_local_var_read(lenv: &LocalEnv, var_name: &str) -> Option<VertexId> {
    lenv.get_var(var_name)
}

/// Install instance variable write: @name = value
///
/// If @name already has a pre-allocated VertexId (e.g., from attr_reader),
/// an edge is added from value_vtx to the existing vertex so types propagate.
/// Otherwise, value_vtx is registered directly as the ivar's VertexId.
pub(crate) fn install_ivar_write(
    genv: &mut GlobalEnv,
    ivar_name: String,
    value_vtx: VertexId,
) -> VertexId {
    if let Some(existing_vtx) = genv.scope_manager.lookup_instance_var(&ivar_name) {
        genv.add_edge(value_vtx, existing_vtx);
        existing_vtx
    } else {
        genv.scope_manager
            .set_instance_var_in_class(ivar_name, value_vtx);
        value_vtx
    }
}

/// Install instance variable read: @name
pub(crate) fn install_ivar_read(genv: &GlobalEnv, ivar_name: &str) -> Option<VertexId> {
    genv.scope_manager.lookup_instance_var(ivar_name)
}

/// Install self node
/// Uses the fully qualified name if available (e.g., Api::V1::User instead of just User)
pub(crate) fn install_self(genv: &mut GlobalEnv) -> VertexId {
    if let Some(qualified_name) = genv.scope_manager.current_qualified_name() {
        genv.new_source(Type::instance(&qualified_name))
    } else {
        genv.new_source(Type::instance("Object"))
    }
}

/// Install class variable write: @@name = value
///
/// If @@name already has a VertexId (e.g., from a previous assignment),
/// an edge is added from value_vtx to the existing vertex so types propagate.
/// Otherwise, value_vtx is registered directly as the cvar's VertexId.
pub(crate) fn install_class_var_write(
    genv: &mut GlobalEnv,
    cvar_name: String,
    value_vtx: VertexId,
) -> VertexId {
    if let Some(existing_vtx) = genv.scope_manager.lookup_class_var(&cvar_name) {
        genv.add_edge(value_vtx, existing_vtx);
        existing_vtx
    } else {
        genv.scope_manager.set_class_var_in_class(cvar_name, value_vtx);
        value_vtx
    }
}

/// Install class variable read: @@name
pub(crate) fn install_class_var_read(genv: &GlobalEnv, cvar_name: &str) -> Option<VertexId> {
    genv.scope_manager.lookup_class_var(cvar_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Type;

    fn setup_class_scope(genv: &mut GlobalEnv) {
        genv.enter_class("TestClass".to_string(), None);
        genv.enter_method("test_method".to_string());
    }

    #[test]
    fn test_install_class_var_write_new() {
        let mut genv = GlobalEnv::new();
        setup_class_scope(&mut genv);

        let value_vtx = genv.new_source(Type::integer());
        let result_vtx = install_class_var_write(&mut genv, "@@count".to_string(), value_vtx);

        assert_eq!(result_vtx, value_vtx);
    }

    #[test]
    fn test_install_class_var_read_after_write() {
        let mut genv = GlobalEnv::new();
        setup_class_scope(&mut genv);

        let value_vtx = genv.new_source(Type::integer());
        install_class_var_write(&mut genv, "@@count".to_string(), value_vtx);

        let read_vtx = install_class_var_read(&genv, "@@count");
        assert_eq!(read_vtx, Some(value_vtx));
    }

    #[test]
    fn test_install_class_var_read_undefined() {
        let genv = GlobalEnv::new();
        assert_eq!(install_class_var_read(&genv, "@@undefined"), None);
    }

    #[test]
    fn test_install_class_var_write_twice_merges() {
        let mut genv = GlobalEnv::new();
        setup_class_scope(&mut genv);

        let str_vtx = genv.new_source(Type::string());
        let vtx1 = install_class_var_write(&mut genv, "@@var".to_string(), str_vtx);

        let int_vtx = genv.new_source(Type::integer());
        let vtx2 = install_class_var_write(&mut genv, "@@var".to_string(), int_vtx);

        // Second write returns the same VertexId (edge added, not overwritten)
        assert_eq!(vtx1, vtx2);
    }
}
