use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{BoxId, ChangeSet, MethodCallBox, VertexId};
use crate::source_map::SourceLocation;
use crate::types::Type;
use ruby_prism::Node;

/// Build graph from AST
pub struct AstInstaller<'a> {
    genv: &'a mut GlobalEnv,
    lenv: &'a mut LocalEnv,
    changes: ChangeSet,
    source: &'a str,
}

impl<'a> AstInstaller<'a> {
    pub fn new(genv: &'a mut GlobalEnv, lenv: &'a mut LocalEnv, source: &'a str) -> Self {
        Self {
            genv,
            lenv,
            changes: ChangeSet::new(),
            source,
        }
    }

    /// Install node (returns Vertex ID)
    pub fn install_node(&mut self, node: &Node) -> Option<VertexId> {
        // Class definition
        if let Some(class_node) = node.as_class_node() {
            return self.install_class_node(&class_node);
        }

        // Method definition
        if let Some(def_node) = node.as_def_node() {
            return self.install_def_node(&def_node);
        }

        // Instance variable write: @name = value
        if let Some(ivar_write) = node.as_instance_variable_write_node() {
            return self.install_ivar_write(&ivar_write);
        }

        // Instance variable read: @name
        if let Some(ivar_read) = node.as_instance_variable_read_node() {
            return self.install_ivar_read(&ivar_read);
        }

        // self
        if node.as_self_node().is_some() {
            return self.install_self();
        }

        // x = "hello"
        if let Some(write_node) = node.as_local_variable_write_node() {
            let value = write_node.value();
            let val_vtx = self.install_node(&value)?;

            // Convert ConstantId to string (using as_slice())
            let var_name = String::from_utf8_lossy(write_node.name().as_slice()).to_string();
            let var_vtx = self.genv.new_vertex();
            self.lenv.new_var(var_name, var_vtx);

            self.changes.add_edge(val_vtx, var_vtx);
            return Some(var_vtx);
        }

        // x
        if let Some(read_node) = node.as_local_variable_read_node() {
            let var_name = String::from_utf8_lossy(read_node.name().as_slice()).to_string();
            return self.lenv.get_var(&var_name);
        }

        // "hello"
        if node.as_string_node().is_some() {
            return Some(self.genv.new_source(Type::string()));
        }

        // 42
        if node.as_integer_node().is_some() {
            return Some(self.genv.new_source(Type::integer()));
        }

        // [1, 2, 3]
        if node.as_array_node().is_some() {
            return Some(self.genv.new_source(Type::array()));
        }

        // {a: 1}
        if node.as_hash_node().is_some() {
            return Some(self.genv.new_source(Type::hash()));
        }

        // nil
        if node.as_nil_node().is_some() {
            return Some(self.genv.new_source(Type::Nil));
        }

        // true
        if node.as_true_node().is_some() {
            return Some(self.genv.new_source(Type::Instance {
                class_name: "TrueClass".to_string(),
            }));
        }

        // false
        if node.as_false_node().is_some() {
            return Some(self.genv.new_source(Type::Instance {
                class_name: "FalseClass".to_string(),
            }));
        }

        // :symbol
        if node.as_symbol_node().is_some() {
            return Some(self.genv.new_source(Type::Instance {
                class_name: "Symbol".to_string(),
            }));
        }

        // x.upcase (method call)
        if let Some(call_node) = node.as_call_node() {
            // Process receiver
            let recv_vtx = if let Some(receiver) = call_node.receiver() {
                self.install_node(&receiver)?
            } else {
                // If no receiver, assume self (implicit receiver)
                // Not yet supported in current implementation
                return None;
            };

            // Get method name
            let method_name = String::from_utf8_lossy(call_node.name().as_slice()).to_string();

            // Extract source location from AST node with source code
            let location = SourceLocation::from_prism_location_with_source(&node.location(), self.source);

            // Create Vertex for return value
            let ret_vtx = self.genv.new_vertex();

            // Create MethodCallBox with location
            let box_id = BoxId(self.genv.next_box_id);
            self.genv.next_box_id += 1;

            let call_box = MethodCallBox::new(
                box_id,
                recv_vtx,
                method_name,
                ret_vtx,
                Some(location),
            );
            self.genv.boxes.insert(box_id, Box::new(call_box));
            self.genv.add_run(box_id);

            return Some(ret_vtx);
        }

        // Other nodes not yet implemented
        None
    }

    /// Install class definition
    fn install_class_node(&mut self, class_node: &ruby_prism::ClassNode) -> Option<VertexId> {
        // Extract class name
        let class_name = self.extract_class_name(class_node);

        // Enter class scope
        self.genv.enter_class(class_name.clone());

        // Process class body
        if let Some(body) = class_node.body() {
            if let Some(statements) = body.as_statements_node() {
                self.install_statements(&statements);
            }
        }

        // Exit scope
        self.genv.exit_scope();

        // Class definition itself doesn't return a value
        None
    }

    /// Install method definition
    fn install_def_node(&mut self, def_node: &ruby_prism::DefNode) -> Option<VertexId> {
        // Extract method name
        let method_name = String::from_utf8_lossy(def_node.name().as_slice()).to_string();

        // Enter method scope
        self.genv.enter_method(method_name.clone());

        // TODO: Process parameters in future implementation
        // if let Some(params) = def_node.parameters() {
        //     self.install_parameters(&params);
        // }

        // Process method body
        if let Some(body) = def_node.body() {
            if let Some(statements) = body.as_statements_node() {
                self.install_statements(&statements);
            }
        }

        // Exit scope
        self.genv.exit_scope();

        // Method definition itself doesn't return a value
        None
    }

    /// Process multiple statements
    fn install_statements(&mut self, statements: &ruby_prism::StatementsNode) {
        for stmt in &statements.body() {
            self.install_node(&stmt);
        }
    }

    /// Extract class name from ClassNode
    fn extract_class_name(&self, class_node: &ruby_prism::ClassNode) -> String {
        // Try to get constant path
        if let Some(constant_read) = class_node.constant_path().as_constant_read_node() {
            String::from_utf8_lossy(constant_read.name().as_slice()).to_string()
        } else {
            "UnknownClass".to_string()
        }
    }

    /// Install instance variable write: @name = value
    fn install_ivar_write(&mut self, ivar_write: &ruby_prism::InstanceVariableWriteNode) -> Option<VertexId> {
        let ivar_name = String::from_utf8_lossy(ivar_write.name().as_slice()).to_string();
        let value_vtx = self.install_node(&ivar_write.value())?;

        // Add to class scope (not current method scope)
        self.genv.scope_manager.set_instance_var_in_class(ivar_name, value_vtx);

        Some(value_vtx)
    }

    /// Install instance variable read: @name
    fn install_ivar_read(&mut self, ivar_read: &ruby_prism::InstanceVariableReadNode) -> Option<VertexId> {
        let ivar_name = String::from_utf8_lossy(ivar_read.name().as_slice()).to_string();

        // Lookup from scope
        self.genv.scope_manager.lookup_instance_var(&ivar_name)
    }

    /// Install self
    fn install_self(&mut self) -> Option<VertexId> {
        // Get current class name
        if let Some(class_name) = self.genv.scope_manager.current_class_name() {
            // Return instance type of that class
            Some(self.genv.new_source(Type::Instance { class_name }))
        } else {
            // Top-level self is main object
            Some(self.genv.new_source(Type::Instance {
                class_name: "Object".to_string(),
            }))
        }
    }

    /// Finish installation (apply changes and execute Boxes)
    pub fn finish(self) {
        self.genv.apply_changes(self.changes);
        self.genv.run_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_ruby_source;

    #[test]
    fn test_install_literal() {
        let source = r#"x = "hello""#;

        let parse_result = parse_ruby_source(source, "test.rb".to_string()).unwrap();

        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();
        let mut installer = AstInstaller::new(&mut genv, &mut lenv, source);

        // ruby-prism correct API: get top-level node with node()
        let root = parse_result.node();

        // Get statements from ProgramNode
        if let Some(program_node) = root.as_program_node() {
            let statements = program_node.statements();
            for stmt in &statements.body() {
                installer.install_node(&stmt);
            }
        }

        installer.finish();

        let x_vtx = lenv.get_var("x").unwrap();
        assert_eq!(genv.get_vertex(x_vtx).unwrap().show(), "String");
    }

    #[test]
    fn test_install_multiple_vars() {
        let source = r#"
x = "hello"
y = 42
"#;

        let parse_result = parse_ruby_source(source, "test.rb".to_string()).unwrap();

        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();
        let mut installer = AstInstaller::new(&mut genv, &mut lenv, source);

        let root = parse_result.node();

        if let Some(program_node) = root.as_program_node() {
            let statements = program_node.statements();
            for stmt in &statements.body() {
                installer.install_node(&stmt);
            }
        }

        installer.finish();

        let x_vtx = lenv.get_var("x").unwrap();
        let y_vtx = lenv.get_var("y").unwrap();

        assert_eq!(genv.get_vertex(x_vtx).unwrap().show(), "String");
        assert_eq!(genv.get_vertex(y_vtx).unwrap().show(), "Integer");
    }

    #[test]
    fn test_install_method_call() {
        let source = r#"
x = "hello"
y = x.upcase
"#;

        let parse_result = parse_ruby_source(source, "test.rb".to_string()).unwrap();

        let mut genv = GlobalEnv::new();

        // Register String#upcase
        genv.register_builtin_method(Type::string(), "upcase", Type::string());

        let mut lenv = LocalEnv::new();
        let mut installer = AstInstaller::new(&mut genv, &mut lenv, source);

        let root = parse_result.node();

        if let Some(program_node) = root.as_program_node() {
            let statements = program_node.statements();
            for stmt in &statements.body() {
                installer.install_node(&stmt);
            }
        }

        installer.finish();

        let x_vtx = lenv.get_var("x").unwrap();
        let y_vtx = lenv.get_var("y").unwrap();

        assert_eq!(genv.get_vertex(x_vtx).unwrap().show(), "String");
        assert_eq!(genv.get_vertex(y_vtx).unwrap().show(), "String");
    }

    #[test]
    fn test_install_method_chain() {
        let source = r#"
x = "hello"
y = x.upcase.downcase
"#;

        let parse_result = parse_ruby_source(source, "test.rb".to_string()).unwrap();

        let mut genv = GlobalEnv::new();

        // Register String methods
        genv.register_builtin_method(Type::string(), "upcase", Type::string());
        genv.register_builtin_method(Type::string(), "downcase", Type::string());

        let mut lenv = LocalEnv::new();
        let mut installer = AstInstaller::new(&mut genv, &mut lenv, source);

        let root = parse_result.node();

        if let Some(program_node) = root.as_program_node() {
            let statements = program_node.statements();
            for stmt in &statements.body() {
                installer.install_node(&stmt);
            }
        }

        installer.finish();

        let y_vtx = lenv.get_var("y").unwrap();
        assert_eq!(genv.get_vertex(y_vtx).unwrap().show(), "String");
    }

    #[test]
    fn test_class_method_error_detection() {
        let source = r#"
class User
  def test
    x = 123
    y = x.upcase
  end
end
"#;

        let parse_result = parse_ruby_source(source, "test.rb".to_string()).unwrap();

        let mut genv = GlobalEnv::new();

        // Register String#upcase but NOT Integer#upcase
        genv.register_builtin_method(Type::string(), "upcase", Type::string());

        let mut lenv = LocalEnv::new();
        let mut installer = AstInstaller::new(&mut genv, &mut lenv, source);

        let root = parse_result.node();

        if let Some(program_node) = root.as_program_node() {
            let statements = program_node.statements();
            for stmt in &statements.body() {
                installer.install_node(&stmt);
            }
        }

        installer.finish();

        // Type error should be detected: Integer doesn't have upcase method
        assert_eq!(genv.type_errors.len(), 1);
        assert_eq!(genv.type_errors[0].method_name, "upcase");
    }

    #[test]
    fn test_class_with_instance_variable() {
        let source = r#"
class User
  def initialize
    @name = "John"
  end

  def greet
    @name.upcase
  end
end
"#;

        let parse_result = parse_ruby_source(source, "test.rb".to_string()).unwrap();

        let mut genv = GlobalEnv::new();

        // Register String methods
        genv.register_builtin_method(Type::string(), "upcase", Type::string());

        let mut lenv = LocalEnv::new();
        let mut installer = AstInstaller::new(&mut genv, &mut lenv, source);

        let root = parse_result.node();

        if let Some(program_node) = root.as_program_node() {
            let statements = program_node.statements();
            for stmt in &statements.body() {
                installer.install_node(&stmt);
            }
        }

        installer.finish();

        // No type errors should occur - @name is String
        assert_eq!(genv.type_errors.len(), 0);
    }

    #[test]
    fn test_instance_variable_type_error() {
        let source = r#"
class User
  def initialize
    @name = 123
  end

  def greet
    @name.upcase
  end
end
"#;

        let parse_result = parse_ruby_source(source, "test.rb".to_string()).unwrap();

        let mut genv = GlobalEnv::new();

        // Register String#upcase but NOT Integer#upcase
        genv.register_builtin_method(Type::string(), "upcase", Type::string());

        let mut lenv = LocalEnv::new();
        let mut installer = AstInstaller::new(&mut genv, &mut lenv, source);

        let root = parse_result.node();

        if let Some(program_node) = root.as_program_node() {
            let statements = program_node.statements();
            for stmt in &statements.body() {
                installer.install_node(&stmt);
            }
        }

        installer.finish();

        // Type error should be detected: @name is Integer, not String
        assert_eq!(genv.type_errors.len(), 1);
        assert_eq!(genv.type_errors[0].method_name, "upcase");
    }

    #[test]
    fn test_multiple_classes() {
        let source = r#"
class User
  def name
    x = 123
    x.upcase
  end
end

class Post
  def title
    y = "hello"
    y.upcase
  end
end
"#;

        let parse_result = parse_ruby_source(source, "test.rb".to_string()).unwrap();

        let mut genv = GlobalEnv::new();

        // Register String#upcase but NOT Integer#upcase
        genv.register_builtin_method(Type::string(), "upcase", Type::string());

        let mut lenv = LocalEnv::new();
        let mut installer = AstInstaller::new(&mut genv, &mut lenv, source);

        let root = parse_result.node();

        if let Some(program_node) = root.as_program_node() {
            let statements = program_node.statements();
            for stmt in &statements.body() {
                installer.install_node(&stmt);
            }
        }

        installer.finish();

        // Only User#name should have error (Integer#upcase), Post#title is fine
        assert_eq!(genv.type_errors.len(), 1);
        assert_eq!(genv.type_errors[0].method_name, "upcase");
    }
}
