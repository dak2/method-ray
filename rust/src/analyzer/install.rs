//! AST Installer - AST traversal and graph construction
//!
//! This module is responsible for:
//! - Traversing the Ruby AST (Abstract Syntax Tree)
//! - Coordinating the graph construction process

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{BlockParameterTypeBox, ChangeSet, VertexId};
use ruby_prism::Node;

use super::blocks::{enter_block_scope, exit_block_scope, install_block_parameter};
use super::definitions::{
    exit_scope, extract_class_name, extract_module_name, install_class, install_method,
    install_module,
};
use super::dispatch::{
    dispatch_needs_child, dispatch_simple, finish_ivar_write, finish_local_var_write,
    finish_method_call, DispatchResult, NeedsChildKind,
};
use super::literals::install_literal;
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

        // Module definition
        if let Some(module_node) = node.as_module_node() {
            return self.install_module_node(&module_node);
        }

        // Method definition
        if let Some(def_node) = node.as_def_node() {
            return self.install_def_node(&def_node);
        }

        // Block node (standalone block, e.g., lambda { |x| x })
        if let Some(block_node) = node.as_block_node() {
            return self.install_block_node(&block_node);
        }

        // Try simple dispatch first (no child processing needed)
        match dispatch_simple(self.genv, self.lenv, node) {
            DispatchResult::Vertex(vtx) => return Some(vtx),
            DispatchResult::NotHandled => {}
        }

        // Literals (String, Integer, Array, Hash, nil, true, false, Symbol)
        if let Some(vtx) = self.install_literal_node(node) {
            return Some(vtx);
        }

        // Check if node needs child processing
        if let Some(kind) = dispatch_needs_child(node, self.source) {
            return self.process_needs_child(kind);
        }

        None
    }

    /// Install literal node
    ///
    /// Handles all literals including Array with element type inference
    fn install_literal_node(&mut self, node: &Node) -> Option<VertexId> {
        // Array literals need special handling for element type inference
        if node.as_array_node().is_some() {
            let elements: Vec<Node> = node.as_array_node().unwrap().elements().iter().collect();
            return self.install_array_literal_elements(elements);
        }

        // Other literals (String, Integer, Hash, nil, true, false, Symbol)
        install_literal(self.genv, node)
    }

    /// Install array literal with pre-collected elements
    fn install_array_literal_elements(&mut self, elements: Vec<Node>) -> Option<VertexId> {
        use crate::types::Type;
        use std::collections::HashSet;

        if elements.is_empty() {
            return Some(self.genv.new_source(Type::array()));
        }

        let mut element_types: HashSet<Type> = HashSet::new();

        for element in &elements {
            if let Some(vtx) = self.install_node(element) {
                if let Some(source) = self.genv.get_source(vtx) {
                    element_types.insert(source.ty.clone());
                } else if let Some(vertex) = self.genv.get_vertex(vtx) {
                    for ty in vertex.types.keys() {
                        element_types.insert(ty.clone());
                    }
                }
            }
        }

        let array_type = if element_types.is_empty() {
            Type::array()
        } else if element_types.len() == 1 {
            let elem_type = element_types.into_iter().next().unwrap();
            Type::array_of(elem_type)
        } else {
            let types_vec: Vec<Type> = element_types.into_iter().collect();
            let union_type = Type::Union(types_vec);
            Type::array_of(union_type)
        };

        Some(self.genv.new_source(array_type))
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
                block,
            } => {
                let recv_vtx = self.install_node(&receiver)?;

                // Process block if present (e.g., `x.each { |i| ... }`)
                // Collect block parameter vertex IDs for type inference
                let mut block_param_vtxs: Vec<VertexId> = Vec::new();
                if let Some(block_node) = block {
                    // Block may be a BlockNode or BlockArgumentNode
                    if let Some(bn) = block_node.as_block_node() {
                        block_param_vtxs = self.install_block_node_with_params(&bn);
                    }
                }

                // Create BlockParameterTypeBox if block has parameters
                if !block_param_vtxs.is_empty() {
                    let box_id = self.genv.alloc_box_id();
                    let block_box = BlockParameterTypeBox::new(
                        box_id,
                        recv_vtx,
                        method_name.clone(),
                        block_param_vtxs,
                    );
                    self.genv.register_box(box_id, Box::new(block_box));
                }

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

    /// Install module definition
    fn install_module_node(&mut self, module_node: &ruby_prism::ModuleNode) -> Option<VertexId> {
        let module_name = extract_module_name(module_node);
        install_module(self.genv, module_name);

        if let Some(body) = module_node.body() {
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

    /// Install block node
    ///
    /// Processes blocks like `{ |x| x.to_s }` or `do |item| item.upcase end`
    fn install_block_node(&mut self, block_node: &ruby_prism::BlockNode) -> Option<VertexId> {
        // Use the version that collects param vtxs, but discard them
        self.install_block_node_with_params(block_node);
        None
    }

    /// Install block node and return block parameter vertex IDs
    ///
    /// This is used when processing method calls with blocks to collect
    /// the block parameter vertices for type inference via BlockParameterTypeBox.
    fn install_block_node_with_params(
        &mut self,
        block_node: &ruby_prism::BlockNode,
    ) -> Vec<VertexId> {
        // Enter block scope
        enter_block_scope(self.genv);

        let mut param_vtxs = Vec::new();

        // Process block parameters BEFORE processing body
        // block_node.parameters() returns Option<Node>, need to convert to BlockParametersNode
        if let Some(params_node) = block_node.parameters() {
            if let Some(block_params) = params_node.as_block_parameters_node() {
                param_vtxs = self.install_block_parameters_with_vtxs(&block_params);
            }
        }

        // Process block body
        if let Some(body) = block_node.body() {
            if let Some(statements) = body.as_statements_node() {
                self.install_statements(&statements);
            } else {
                // Single expression body
                self.install_node(&body);
            }
        }

        // Exit block scope
        exit_block_scope(self.genv);

        param_vtxs
    }

    /// Install block parameters as local variables
    ///
    /// Block parameters like `|x, y|` are registered as local variables
    /// with Bot (untyped) type.
    #[allow(dead_code)]
    fn install_block_parameters(&mut self, block_params: &ruby_prism::BlockParametersNode) {
        // Just call the version that returns vtxs and discard the result
        self.install_block_parameters_with_vtxs(block_params);
    }

    /// Install block parameters and return their vertex IDs
    ///
    /// This version is used when we need to track the block parameter vertices
    /// for type inference from the method's RBS block signature.
    fn install_block_parameters_with_vtxs(
        &mut self,
        block_params: &ruby_prism::BlockParametersNode,
    ) -> Vec<VertexId> {
        let mut vtxs = Vec::new();

        // BlockParametersNode contains a ParametersNode
        if let Some(params) = block_params.parameters() {
            // Process required parameters (most common in blocks)
            for node in params.requireds().iter() {
                if let Some(req_param) = node.as_required_parameter_node() {
                    let name = String::from_utf8_lossy(req_param.name().as_slice()).to_string();
                    let vtx = install_block_parameter(self.genv, self.lenv, name);
                    vtxs.push(vtx);
                }
            }

            // Optional parameters in blocks: { |x = 1| ... }
            for node in params.optionals().iter() {
                if let Some(opt_param) = node.as_optional_parameter_node() {
                    let name = String::from_utf8_lossy(opt_param.name().as_slice()).to_string();
                    let default_value = opt_param.value();

                    if let Some(default_vtx) = self.install_node(&default_value) {
                        let vtx = install_optional_parameter(
                            self.genv,
                            self.lenv,
                            &mut self.changes,
                            name,
                            default_vtx,
                        );
                        vtxs.push(vtx);
                    } else {
                        let vtx = install_block_parameter(self.genv, self.lenv, name);
                        vtxs.push(vtx);
                    }
                }
            }

            // Rest parameter in blocks: { |*args| ... }
            if let Some(rest_node) = params.rest() {
                if let Some(rest_param) = rest_node.as_rest_parameter_node() {
                    if let Some(name_id) = rest_param.name() {
                        let name = String::from_utf8_lossy(name_id.as_slice()).to_string();
                        let vtx = install_rest_parameter(self.genv, self.lenv, name);
                        vtxs.push(vtx);
                    }
                }
            }
        }

        vtxs
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

    #[test]
    fn test_install_module_with_method() {
        let source = r#"
module Utils
  def helper
    x = "test"
  end
end
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

        // After processing, we should be back at top-level scope
        assert_eq!(genv.scope_manager.current_module_name(), None);
        assert_eq!(genv.scope_manager.current_class_name(), None);
    }

    #[test]
    fn test_install_nested_module_class() {
        let source = r#"
module Api
  class User
    def greet
      name = "hello"
    end
  end
end
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

        // After processing, we should be back at top-level scope
        assert_eq!(genv.scope_manager.current_module_name(), None);
        assert_eq!(genv.scope_manager.current_class_name(), None);
    }
}
