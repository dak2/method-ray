//! Compound assignment handlers (`x += 1`, `x ||= val`, `x &&= val`).
//!
//! `||=` and `&&=` use the same Union(existing, val) approximation — a sound
//! over-approximation without condition narrowing.

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use crate::types::Type;

use super::calls::install_method_call;
use super::variables::{
    install_class_var_read, install_class_var_write, install_constant_read, install_constant_write,
    install_global_var_read, install_global_var_write, install_ivar_read, install_ivar_write,
    install_local_var_read, install_local_var_write,
};

pub(crate) enum CompoundVarKind {
    Local(String),
    Ivar(String),
    ClassVar(String),
    GlobalVar(String),
    Constant(String),
}

impl CompoundVarKind {
    fn read(&self, genv: &GlobalEnv, lenv: &LocalEnv) -> Option<VertexId> {
        match self {
            Self::Local(name) => install_local_var_read(lenv, name),
            Self::Ivar(name) => install_ivar_read(genv, name),
            Self::ClassVar(name) => install_class_var_read(genv, name),
            Self::GlobalVar(name) => install_global_var_read(genv, name),
            Self::Constant(name) => install_constant_read(genv, name),
        }
    }

    fn write(
        self,
        genv: &mut GlobalEnv,
        lenv: &mut LocalEnv,
        changes: &mut ChangeSet,
        value_vtx: VertexId,
    ) -> VertexId {
        match self {
            Self::Local(name) => install_local_var_write(genv, lenv, changes, name, value_vtx),
            Self::Ivar(name) => install_ivar_write(genv, name, value_vtx),
            Self::ClassVar(name) => install_class_var_write(genv, name, value_vtx),
            Self::GlobalVar(name) => install_global_var_write(genv, name, value_vtx),
            Self::Constant(name) => install_constant_write(genv, name, value_vtx),
        }
    }
}

pub(crate) enum CompoundOp {
    Operator(String),
    Logical,
}

pub(crate) fn process_compound_write(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    var_kind: CompoundVarKind,
    op: CompoundOp,
    value_vtx: VertexId,
) -> VertexId {
    match op {
        CompoundOp::Operator(operator) => {
            let current_vtx = var_kind.read(genv, lenv)
                .unwrap_or_else(|| genv.new_source(Type::Nil));
            let result_vtx = install_method_call(
                genv, current_vtx, operator, vec![value_vtx], None, None, false,
            );
            var_kind.write(genv, lenv, changes, result_vtx)
        }
        CompoundOp::Logical => {
            let merge_vtx = genv.new_vertex();
            if let Some(current_vtx) = var_kind.read(genv, lenv) {
                genv.add_edge(current_vtx, merge_vtx);
            }
            genv.add_edge(value_vtx, merge_vtx);
            var_kind.write(genv, lenv, changes, merge_vtx)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_write() {
        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();
        let mut changes = ChangeSet::new();

        let int_vtx = genv.new_source(Type::integer());
        install_local_var_write(&mut genv, &mut lenv, &mut changes, "x".to_string(), int_vtx);

        let rhs_vtx = genv.new_source(Type::integer());
        let result = process_compound_write(
            &mut genv, &mut lenv, &mut changes,
            CompoundVarKind::Local("x".to_string()),
            CompoundOp::Operator("+".to_string()),
            rhs_vtx,
        );

        assert_ne!(result, int_vtx);
        assert_eq!(install_local_var_read(&lenv, "x"), Some(result));
    }

    #[test]
    fn test_operator_write_uninitialized() {
        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();
        let mut changes = ChangeSet::new();

        let rhs_vtx = genv.new_source(Type::integer());
        let result = process_compound_write(
            &mut genv, &mut lenv, &mut changes,
            CompoundVarKind::Local("x".to_string()),
            CompoundOp::Operator("+".to_string()),
            rhs_vtx,
        );

        assert_eq!(install_local_var_read(&lenv, "x"), Some(result));
    }

    #[test]
    fn test_logical_write_produces_union() {
        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();
        let mut changes = ChangeSet::new();

        let str_vtx = genv.new_source(Type::string());
        install_local_var_write(&mut genv, &mut lenv, &mut changes, "x".to_string(), str_vtx);

        let int_vtx = genv.new_source(Type::integer());
        let result = process_compound_write(
            &mut genv, &mut lenv, &mut changes,
            CompoundVarKind::Local("x".to_string()),
            CompoundOp::Logical,
            int_vtx,
        );

        assert_eq!(install_local_var_read(&lenv, "x"), Some(result));
        genv.apply_changes(changes);
        genv.run_all();
        let types = genv.get_receiver_types(result).unwrap();
        assert!(types.contains(&Type::string()));
        assert!(types.contains(&Type::integer()));
    }

    #[test]
    fn test_logical_write_uninitialized() {
        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();
        let mut changes = ChangeSet::new();

        let str_vtx = genv.new_source(Type::string());
        let result = process_compound_write(
            &mut genv, &mut lenv, &mut changes,
            CompoundVarKind::Local("x".to_string()),
            CompoundOp::Logical,
            str_vtx,
        );

        genv.apply_changes(changes);
        genv.run_all();
        let types = genv.get_receiver_types(result).unwrap();
        assert!(types.contains(&Type::string()));
    }

    #[test]
    fn test_ivar_compound_write() {
        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();
        let mut changes = ChangeSet::new();
        genv.enter_class("TestClass".to_string(), None);
        genv.enter_method("test_method".to_string());

        let int_vtx = genv.new_source(Type::integer());
        install_ivar_write(&mut genv, "@count".to_string(), int_vtx);

        let rhs_vtx = genv.new_source(Type::integer());
        let result = process_compound_write(
            &mut genv, &mut lenv, &mut changes,
            CompoundVarKind::Ivar("@count".to_string()),
            CompoundOp::Operator("+".to_string()),
            rhs_vtx,
        );

        assert_eq!(result, int_vtx);
    }
}
