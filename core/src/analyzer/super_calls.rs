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
    use crate::graph::VertexId;
    use crate::parser::ParseSession;
    use crate::types::Type;

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

    /// Helper: get the type string for a vertex ID
    fn get_type_show(genv: &GlobalEnv, vtx: VertexId) -> String {
        if let Some(vertex) = genv.get_vertex(vtx) {
            vertex.show()
        } else if let Some(source) = genv.get_source(vtx) {
            source.ty.show()
        } else {
            panic!("vertex {:?} not found as either Vertex or Source", vtx);
        }
    }

    #[test]
    fn test_super_basic() {
        let source = r#"
class Animal
  def speak
    "..."
  end
end

class Dog < Animal
  def speak
    super
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Dog"), "speak")
            .expect("Dog#speak should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "String");
    }

    #[test]
    fn test_super_with_method_chain() {
        let source = r#"
class Animal
  def speak
    "hello"
  end
end

class Dog < Animal
  def speak
    super.upcase
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Dog"), "speak")
            .expect("Dog#speak should be registered");
        assert!(info.return_vertex.is_some());
    }

    #[test]
    fn test_super_with_arguments() {
        let source = r#"
class Base
  def greet(name)
    name
  end
end

class Child < Base
  def greet(name)
    super(name)
  end
end

Child.new.greet("Alice")
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Child"), "greet")
            .expect("Child#greet should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "String");
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
    fn test_super_explicit_empty_args() {
        let source = r#"
class Animal
  def speak
    "hello"
  end
end

class Dog < Animal
  def speak
    super()
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Dog"), "speak")
            .expect("Dog#speak should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "String");
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
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("Foo#bar should be registered");
        assert!(info.return_vertex.is_some());
    }

    #[test]
    fn test_super_qualified_superclass() {
        let source = r#"
module Animals
  class Pet
    def name
      "pet"
    end
  end
end

class Dog < Animals::Pet
  def name
    super
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Dog"), "name")
            .expect("Dog#name should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "String");
    }

    #[test]
    fn test_super_multi_level_inheritance() {
        let source = r#"
class A
  def foo
    "hello"
  end
end

class B < A
  def foo
    super
  end
end

class C < B
  def foo
    super
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("C"), "foo")
            .expect("C#foo should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "String");
    }
}
