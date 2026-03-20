//! Super call handling: `super` and `super(args)`
//!
//! Ruby's `super` calls the same-named method on the parent class.
//! - `super(args)` → SuperNode: explicit arguments
//! - `super` (bare) → ForwardingSuperNode: implicit argument forwarding
//!
//! Note: ForwardingSuperNode (bare `super`) is treated as a zero-argument
//! call. In Ruby, bare `super` forwards all arguments from the enclosing
//! method, but replicating this requires parameter-vertex forwarding that
//! is not yet implemented. Return type inference is unaffected.

use ruby_prism::{ForwardingSuperNode, SuperNode};

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use crate::source_map::SourceLocation as SL;
use crate::types::Type;

/// Process SuperNode: `super(args)` — explicit arguments
pub(crate) fn process_super_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    super_node: &SuperNode,
) -> Option<VertexId> {
    let location = SL::from_prism_location_with_source(&super_node.location(), source);
    process_super_call(genv, lenv, changes, source, super_node.arguments(), location)
}

/// Process ForwardingSuperNode: `super` — implicit argument forwarding
pub(crate) fn process_forwarding_super_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    node: &ForwardingSuperNode,
) -> Option<VertexId> {
    let location = SL::from_prism_location_with_source(&node.location(), source);
    process_super_call(genv, lenv, changes, source, None, location)
}

/// Resolve a super call by looking up the same-named method on the superclass.
///
/// Returns `None` if there is no enclosing method scope (super outside a method)
/// or no explicit superclass declared on the enclosing class.
fn process_super_call(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    arguments: Option<ruby_prism::ArgumentsNode>,
    location: SL,
) -> Option<VertexId> {
    let method_name = genv.scope_manager.current_method_name()?;
    let superclass_name = genv.scope_manager.current_superclass()?;
    let recv_vtx = genv.new_source(Type::instance(&superclass_name));

    let (arg_vtxs, kw) = if let Some(args) = arguments {
        super::dispatch::collect_arguments(genv, lenv, changes, source, args.arguments().iter())
    } else {
        (vec![], None)
    };

    Some(super::calls::install_method_call(
        genv,
        recv_vtx,
        method_name,
        arg_vtxs,
        kw,
        Some(location),
        false, // super calls cannot use safe navigation
    ))
}

#[cfg(test)]
mod tests {
    use crate::analyzer::install::AstInstaller;
    use crate::env::{GlobalEnv, LocalEnv};
    use crate::parser::ParseSession;

    /// Helper: parse Ruby source, process with AstInstaller, and return GlobalEnv
    fn analyze(source: &str) -> GlobalEnv {
        let session = ParseSession::new();
        let parse_result = session.parse_source(source, "test.rb").unwrap();
        let root = parse_result.node();
        let program = root.as_program_node().unwrap();

        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();

        let mut installer = AstInstaller::new(&mut genv, &mut lenv, source);
        for stmt in &program.statements().body() {
            installer.install_node(&stmt);
        }
        installer.finish();

        genv
    }

    #[test]
    fn test_super_outside_method_ignored() {
        let source = r#"
class Foo < Object
  super
end
"#;
        analyze(source);
    }

    #[test]
    fn test_super_without_superclass_ignored() {
        let source = r#"
class Foo
  def bar
    super
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&crate::types::Type::instance("Foo"), "bar")
            .expect("Foo#bar should be registered");
        assert!(info.return_vertex.is_some());
    }
}
