//! Attribute accessor support - synthesize getter/setter methods for attr_reader/attr_writer/attr_accessor

use crate::env::GlobalEnv;
use crate::types::Type;

use super::dispatch::AttrKind;

/// Register synthesized getter/setter methods for attr_reader/attr_writer/attr_accessor.
///
/// - attr_reader :name → registers User#name (getter, return = @name vertex)
/// - attr_writer :name → registers User#name= (setter, param → @name edge)
/// - attr_accessor :name → registers both getter and setter
///
/// If @name has no VertexId yet, one is pre-allocated so that later assignments
/// (e.g., `@name = "Alice"` in initialize) propagate into the same vertex.
pub(crate) fn process_attr_declaration(
    genv: &mut GlobalEnv,
    kind: AttrKind,
    attr_names: Vec<String>,
) {
    let Some(qualified_name) = genv.scope_manager.current_qualified_name() else {
        return;
    };
    let recv_ty = Type::instance(&qualified_name);

    for attr_name in attr_names {
        let ivar_name = format!("@{}", attr_name);

        // Get or pre-allocate VertexId for @name
        let ivar_vtx = match genv.scope_manager.lookup_instance_var(&ivar_name) {
            Some(vtx) => vtx,
            None => {
                let vtx = genv.new_vertex();
                genv.scope_manager
                    .set_instance_var_in_class(ivar_name, vtx);
                vtx
            }
        };

        // Register getter (attr_reader / attr_accessor)
        if matches!(kind, AttrKind::Reader | AttrKind::Accessor) {
            genv.register_user_method(recv_ty.clone(), &attr_name, ivar_vtx, vec![]);
        }

        // Register setter (attr_writer / attr_accessor)
        if matches!(kind, AttrKind::Writer | AttrKind::Accessor) {
            let param_vtx = genv.new_vertex();
            genv.add_edge(param_vtx, ivar_vtx);
            genv.register_user_method(
                recv_ty.clone(),
                &format!("{}=", attr_name),
                ivar_vtx,
                vec![param_vtx],
            );
        }
    }
}
