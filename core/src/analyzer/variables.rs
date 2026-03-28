//! Variable Handlers - Processing Ruby variables
//!
//! This module is responsible for:
//! - Local variable read/write (x, x = value)
//! - Instance variable read/write (@name, @name = value)
//! - Class variable read/write (@@name, @@name = value)
//! - Global variable read/write ($var, $var = value)
//! - Constant read/write (CONST = value, CONST)
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

/// Install global variable write: $var = value
///
/// Delegates to [`GlobalEnv::set_global_var`] for edge behavior details.
pub(crate) fn install_global_var_write(
    genv: &mut GlobalEnv,
    gvar_name: String,
    value_vtx: VertexId,
) -> VertexId {
    genv.set_global_var(gvar_name, value_vtx)
}

/// Install global variable read: $var
pub(crate) fn install_global_var_read(genv: &GlobalEnv, gvar_name: &str) -> Option<VertexId> {
    genv.get_global_var(gvar_name)
}

pub(crate) fn install_constant_write(
    genv: &mut GlobalEnv,
    const_name: String,
    value_vtx: VertexId,
) -> VertexId {
    let key = match genv.scope_manager.current_qualified_name() {
        Some(ns) => format!("{}::{}", ns, const_name),
        None => const_name,
    };
    genv.set_constant(key, value_vtx)
}

pub(crate) fn install_constant_read(
    genv: &GlobalEnv,
    const_name: &str,
) -> Option<VertexId> {
    if let Some(ns) = genv.scope_manager.current_qualified_name() {
        let mut current_ns = ns.as_str();
        loop {
            let key = format!("{}::{}", current_ns, const_name);
            if let Some(vtx) = genv.get_constant(&key) {
                return Some(vtx);
            }
            match current_ns.rfind("::") {
                Some(pos) => current_ns = &current_ns[..pos],
                None => break,
            }
        }
    }
    genv.get_constant(const_name)
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

        assert_eq!(vtx1, vtx2);
    }

    #[test]
    fn test_global_var_write_and_read() {
        let mut genv = GlobalEnv::new();
        let value_vtx = genv.new_source(Type::instance("String"));
        let result_vtx = install_global_var_write(&mut genv, "$config".to_string(), value_vtx);
        assert_eq!(result_vtx, value_vtx);

        let read_vtx = install_global_var_read(&genv, "$config");
        assert_eq!(read_vtx, Some(value_vtx));
    }

    #[test]
    fn test_global_var_read_unregistered() {
        let genv = GlobalEnv::new();
        let read_vtx = install_global_var_read(&genv, "$unknown");
        assert_eq!(read_vtx, None);
    }

    #[test]
    fn test_global_var_write_twice_returns_same_vertex() {
        let mut genv = GlobalEnv::new();
        let vtx1 = genv.new_source(Type::instance("String"));
        let first = install_global_var_write(&mut genv, "$data".to_string(), vtx1);

        let vtx2 = genv.new_source(Type::instance("Integer"));
        let second = install_global_var_write(&mut genv, "$data".to_string(), vtx2);

        assert_eq!(first, second);
        assert_eq!(install_global_var_read(&genv, "$data"), Some(first));
    }

    #[test]
    fn test_global_var_write_twice_propagates_via_vertex() {
        let mut genv = GlobalEnv::new();
        let var_vtx = genv.new_vertex();
        install_global_var_write(&mut genv, "$data".to_string(), var_vtx);

        let str_src = genv.new_source(Type::instance("String"));
        install_global_var_write(&mut genv, "$data".to_string(), str_src);

        let types = genv.get_receiver_types(var_vtx).unwrap();
        assert!(types.contains(&Type::instance("String")));
    }

    #[test]
    fn test_constant_write_twice_merges() {
        let mut genv = GlobalEnv::new();
        let str_vtx = genv.new_source(Type::string());
        let vtx1 = install_constant_write(&mut genv, "VAL".to_string(), str_vtx);

        let int_vtx = genv.new_source(Type::integer());
        let vtx2 = install_constant_write(&mut genv, "VAL".to_string(), int_vtx);

        assert_eq!(vtx1, vtx2);
    }

    #[test]
    fn test_constant_read_undefined() {
        let genv = GlobalEnv::new();
        assert_eq!(install_constant_read(&genv, "UNDEFINED"), None);
    }

    #[test]
    fn test_constant_read_nested_namespace_walk() {
        let mut genv = GlobalEnv::new();

        genv.enter_class("Api".to_string(), None);
        let api_vtx = genv.new_source(Type::string());
        install_constant_write(&mut genv, "VERSION".to_string(), api_vtx);

        genv.enter_class("V1".to_string(), None);
        genv.enter_class("Users".to_string(), None);
        genv.enter_method("index".to_string());

        let read = install_constant_read(&genv, "VERSION");
        assert_eq!(read, Some(api_vtx));
    }

    #[test]
    fn test_constant_read_prefers_inner_namespace() {
        let mut genv = GlobalEnv::new();

        let top_vtx = genv.new_source(Type::string());
        install_constant_write(&mut genv, "NAME".to_string(), top_vtx);

        genv.enter_class("Config".to_string(), None);
        let class_vtx = genv.new_source(Type::integer());
        install_constant_write(&mut genv, "NAME".to_string(), class_vtx);

        genv.enter_method("get_name".to_string());

        let read = install_constant_read(&genv, "NAME");
        assert_eq!(read, Some(class_vtx));
    }
}
