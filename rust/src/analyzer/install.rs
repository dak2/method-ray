//! AST Installer - AST traversal and graph construction
//!
//! This module is responsible for:
//! - Traversing the Ruby AST (Abstract Syntax Tree)
//! - Coordinating the graph construction process

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use ruby_prism::Node;

use super::definitions::{exit_scope, extract_class_name, install_class, install_method};
use super::dispatch::{
    dispatch_needs_child, dispatch_simple, finish_ivar_write, finish_local_var_write,
    finish_method_call, DispatchResult, NeedsChildKind,
};
use super::parameters::{
    install_keyword_rest_parameter, install_optional_parameter, install_required_parameter,
    install_rest_parameter,
};

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

        // Try simple dispatch first (no child processing needed)
        match dispatch_simple(self.genv, self.lenv, node) {
            DispatchResult::Vertex(vtx) => return Some(vtx),
            DispatchResult::NotHandled => {}
        }

        // Check if node needs child processing
        if let Some(kind) = dispatch_needs_child(node, self.source) {
            return self.process_needs_child(kind);
        }

        None
    }

    /// Process nodes that need child evaluation first
    fn process_needs_child(&mut self, kind: NeedsChildKind) -> Option<VertexId> {
        match kind {
            NeedsChildKind::IvarWrite { ivar_name, value } => {
                let value_vtx = self.install_node(&value)?;
                Some(finish_ivar_write(self.genv, ivar_name, value_vtx))
            }
            NeedsChildKind::LocalVarWrite { var_name, value } => {
                let value_vtx = self.install_node(&value)?;
                Some(finish_local_var_write(
                    self.genv,
                    self.lenv,
                    &mut self.changes,
                    var_name,
                    value_vtx,
                ))
            }
            NeedsChildKind::MethodCall {
                receiver,
                method_name,
                location,
            } => {
                let recv_vtx = self.install_node(&receiver)?;
                Some(finish_method_call(
                    self.genv,
                    recv_vtx,
                    method_name,
                    location,
                ))
            }
        }
    }

    /// Install class definition
    fn install_class_node(&mut self, class_node: &ruby_prism::ClassNode) -> Option<VertexId> {
        let class_name = extract_class_name(class_node);
        install_class(self.genv, class_name);

        if let Some(body) = class_node.body() {
            if let Some(statements) = body.as_statements_node() {
                self.install_statements(&statements);
            }
        }

        exit_scope(self.genv);
        None
    }

    /// Install method definition
    fn install_def_node(&mut self, def_node: &ruby_prism::DefNode) -> Option<VertexId> {
        let method_name = String::from_utf8_lossy(def_node.name().as_slice()).to_string();
        install_method(self.genv, method_name);

        // Process parameters BEFORE processing body
        // This ensures parameters are available as local variables in the method body
        if let Some(params_node) = def_node.parameters() {
            self.install_parameters(&params_node);
        }

        if let Some(body) = def_node.body() {
            if let Some(statements) = body.as_statements_node() {
                self.install_statements(&statements);
            }
        }

        exit_scope(self.genv);
        None
    }

    /// Install method parameters as local variables
    fn install_parameters(&mut self, params_node: &ruby_prism::ParametersNode) {
        // Required parameters: def foo(a, b)
        for node in params_node.requireds().iter() {
            if let Some(req_param) = node.as_required_parameter_node() {
                let name = String::from_utf8_lossy(req_param.name().as_slice()).to_string();
                install_required_parameter(self.genv, self.lenv, name);
            }
        }

        // Optional parameters: def foo(a = 1, b = "hello")
        for node in params_node.optionals().iter() {
            if let Some(opt_param) = node.as_optional_parameter_node() {
                let name = String::from_utf8_lossy(opt_param.name().as_slice()).to_string();
                let default_value = opt_param.value();

                // Process default value to get its type
                if let Some(default_vtx) = self.install_node(&default_value) {
                    install_optional_parameter(
                        self.genv,
                        self.lenv,
                        &mut self.changes,
                        name,
                        default_vtx,
                    );
                } else {
                    // Fallback to untyped if default can't be processed
                    install_required_parameter(self.genv, self.lenv, name);
                }
            }
        }

        // Rest parameter: def foo(*args)
        if let Some(rest_node) = params_node.rest() {
            if let Some(rest_param) = rest_node.as_rest_parameter_node() {
                if let Some(name_id) = rest_param.name() {
                    let name = String::from_utf8_lossy(name_id.as_slice()).to_string();
                    install_rest_parameter(self.genv, self.lenv, name);
                }
            }
        }

        // Keyword rest parameter: def foo(**kwargs)
        if let Some(kwrest_node) = params_node.keyword_rest() {
            if let Some(kwrest_param) = kwrest_node.as_keyword_rest_parameter_node() {
                if let Some(name_id) = kwrest_param.name() {
                    let name = String::from_utf8_lossy(name_id.as_slice()).to_string();
                    install_keyword_rest_parameter(self.genv, self.lenv, name);
                }
            }
        }
    }

    /// Process multiple statements
    fn install_statements(&mut self, statements: &ruby_prism::StatementsNode) {
        for stmt in &statements.body() {
            self.install_node(&stmt);
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
    use crate::types::Type;

    #[test]
    fn test_install_literal() {
        let source = r#"x = "hello""#;
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
}
