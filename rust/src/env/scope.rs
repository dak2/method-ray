use crate::graph::VertexId;
use std::collections::HashMap;

/// Scope ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(pub usize);

/// Scope kind
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ScopeKind {
    TopLevel,
    Class {
        name: String,
        superclass: Option<String>,
    },
    Module {
        name: String,
    },
    Method {
        name: String,
        receiver_type: Option<String>, // Receiver class/module name
    },
    Block,
}

/// Scope information
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Scope {
    pub id: ScopeId,
    pub kind: ScopeKind,
    pub parent: Option<ScopeId>,

    /// Local variables
    pub local_vars: HashMap<String, VertexId>,

    /// Instance variables (class/module scope only)
    pub instance_vars: HashMap<String, VertexId>,

    /// Class variables (class scope only)
    pub class_vars: HashMap<String, VertexId>,
}

#[allow(dead_code)]
impl Scope {
    pub fn new(id: ScopeId, kind: ScopeKind, parent: Option<ScopeId>) -> Self {
        Self {
            id,
            kind,
            parent,
            local_vars: HashMap::new(),
            instance_vars: HashMap::new(),
            class_vars: HashMap::new(),
        }
    }

    /// Add local variable
    pub fn set_local_var(&mut self, name: String, vtx: VertexId) {
        self.local_vars.insert(name, vtx);
    }

    /// Get local variable
    pub fn get_local_var(&self, name: &str) -> Option<VertexId> {
        self.local_vars.get(name).copied()
    }

    /// Add instance variable
    pub fn set_instance_var(&mut self, name: String, vtx: VertexId) {
        self.instance_vars.insert(name, vtx);
    }

    /// Get instance variable
    pub fn get_instance_var(&self, name: &str) -> Option<VertexId> {
        self.instance_vars.get(name).copied()
    }
}

/// Scope manager
#[derive(Debug)]
pub struct ScopeManager {
    scopes: HashMap<ScopeId, Scope>,
    next_id: usize,
    current_scope: ScopeId,
}

#[allow(dead_code)]
impl ScopeManager {
    pub fn new() -> Self {
        let top_level = Scope::new(ScopeId(0), ScopeKind::TopLevel, None);

        let mut scopes = HashMap::new();
        scopes.insert(ScopeId(0), top_level);

        Self {
            scopes,
            next_id: 1,
            current_scope: ScopeId(0),
        }
    }

    /// Create a new scope
    pub fn new_scope(&mut self, kind: ScopeKind) -> ScopeId {
        let id = ScopeId(self.next_id);
        self.next_id += 1;

        let scope = Scope::new(id, kind, Some(self.current_scope));
        self.scopes.insert(id, scope);

        id
    }

    /// Enter a scope
    pub fn enter_scope(&mut self, scope_id: ScopeId) {
        self.current_scope = scope_id;
    }

    /// Exit current scope
    pub fn exit_scope(&mut self) {
        if let Some(scope) = self.scopes.get(&self.current_scope) {
            if let Some(parent) = scope.parent {
                self.current_scope = parent;
            }
        }
    }

    /// Get current scope
    pub fn current_scope(&self) -> &Scope {
        self.scopes.get(&self.current_scope).unwrap()
    }

    /// Get current scope mutably
    pub fn current_scope_mut(&mut self) -> &mut Scope {
        self.scopes.get_mut(&self.current_scope).unwrap()
    }

    /// Get scope by ID
    pub fn get_scope(&self, id: ScopeId) -> Option<&Scope> {
        self.scopes.get(&id)
    }

    /// Get scope by ID mutably
    pub fn get_scope_mut(&mut self, id: ScopeId) -> Option<&mut Scope> {
        self.scopes.get_mut(&id)
    }

    /// Lookup variable in current scope or parent scopes
    pub fn lookup_var(&self, name: &str) -> Option<VertexId> {
        let mut current = Some(self.current_scope);

        while let Some(scope_id) = current {
            if let Some(scope) = self.scopes.get(&scope_id) {
                if let Some(vtx) = scope.get_local_var(name) {
                    return Some(vtx);
                }
                current = scope.parent;
            } else {
                break;
            }
        }

        None
    }

    /// Lookup instance variable in enclosing class scope
    pub fn lookup_instance_var(&self, name: &str) -> Option<VertexId> {
        let mut current = Some(self.current_scope);

        while let Some(scope_id) = current {
            if let Some(scope) = self.scopes.get(&scope_id) {
                // Walk up to class scope
                match &scope.kind {
                    ScopeKind::Class { .. } => {
                        return scope.get_instance_var(name);
                    }
                    _ => {
                        current = scope.parent;
                    }
                }
            } else {
                break;
            }
        }

        None
    }

    /// Set instance variable in enclosing class scope
    pub fn set_instance_var_in_class(&mut self, name: String, vtx: VertexId) {
        let mut current = Some(self.current_scope);

        while let Some(scope_id) = current {
            if let Some(scope) = self.scopes.get(&scope_id) {
                // Find class scope and set variable
                match &scope.kind {
                    ScopeKind::Class { .. } => {
                        if let Some(class_scope) = self.scopes.get_mut(&scope_id) {
                            class_scope.set_instance_var(name, vtx);
                        }
                        return;
                    }
                    _ => {
                        current = scope.parent;
                    }
                }
            } else {
                break;
            }
        }
    }

    /// Get current class name (simple name, not qualified)
    pub fn current_class_name(&self) -> Option<String> {
        let mut current = Some(self.current_scope);

        while let Some(scope_id) = current {
            if let Some(scope) = self.scopes.get(&scope_id) {
                if let ScopeKind::Class { name, .. } = &scope.kind {
                    return Some(name.clone());
                }
                current = scope.parent;
            } else {
                break;
            }
        }

        None
    }

    /// Get current module name (simple name, not qualified)
    pub fn current_module_name(&self) -> Option<String> {
        let mut current = Some(self.current_scope);

        while let Some(scope_id) = current {
            if let Some(scope) = self.scopes.get(&scope_id) {
                if let ScopeKind::Module { name } = &scope.kind {
                    return Some(name.clone());
                }
                current = scope.parent;
            } else {
                break;
            }
        }

        None
    }

    /// Get current fully qualified name by traversing all parent class/module scopes
    ///
    /// For example, in:
    /// ```ruby
    /// module Api
    ///   module V1
    ///     class User
    ///       def greet; end
    ///     end
    ///   end
    /// end
    /// ```
    /// When inside `greet`, this returns `Some("Api::V1::User")`
    pub fn current_qualified_name(&self) -> Option<String> {
        let mut path_segments: Vec<String> = Vec::new();
        let mut current = Some(self.current_scope);

        // Traverse from current scope up to top-level, collecting class/module names
        while let Some(scope_id) = current {
            if let Some(scope) = self.scopes.get(&scope_id) {
                match &scope.kind {
                    ScopeKind::Class { name, .. } => {
                        // If the name already contains ::, it's a qualified name from AST
                        // (e.g., `class Api::User` defined at top level)
                        if name.contains("::") {
                            path_segments.push(name.clone());
                        } else {
                            path_segments.push(name.clone());
                        }
                    }
                    ScopeKind::Module { name } => {
                        if name.contains("::") {
                            path_segments.push(name.clone());
                        } else {
                            path_segments.push(name.clone());
                        }
                    }
                    _ => {}
                }
                current = scope.parent;
            } else {
                break;
            }
        }

        if path_segments.is_empty() {
            return None;
        }

        // Reverse to get from outermost to innermost
        path_segments.reverse();

        // Join all segments, handling cases where segments may already contain ::
        let mut result = String::new();
        for segment in path_segments {
            if !result.is_empty() {
                result.push_str("::");
            }
            result.push_str(&segment);
        }

        Some(result)
    }

    /// Lookup instance variable in enclosing module scope
    pub fn lookup_instance_var_in_module(&self, name: &str) -> Option<VertexId> {
        let mut current = Some(self.current_scope);

        while let Some(scope_id) = current {
            if let Some(scope) = self.scopes.get(&scope_id) {
                match &scope.kind {
                    ScopeKind::Module { .. } => {
                        return scope.get_instance_var(name);
                    }
                    _ => {
                        current = scope.parent;
                    }
                }
            } else {
                break;
            }
        }

        None
    }

    /// Set instance variable in enclosing module scope
    pub fn set_instance_var_in_module(&mut self, name: String, vtx: VertexId) {
        let mut current = Some(self.current_scope);

        while let Some(scope_id) = current {
            if let Some(scope) = self.scopes.get(&scope_id) {
                match &scope.kind {
                    ScopeKind::Module { .. } => {
                        if let Some(module_scope) = self.scopes.get_mut(&scope_id) {
                            module_scope.set_instance_var(name, vtx);
                        }
                        return;
                    }
                    _ => {
                        current = scope.parent;
                    }
                }
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_manager_creation() {
        let sm = ScopeManager::new();
        assert_eq!(sm.current_scope().id, ScopeId(0));
        assert!(matches!(sm.current_scope().kind, ScopeKind::TopLevel));
    }

    #[test]
    fn test_scope_manager_new_scope() {
        let mut sm = ScopeManager::new();

        let class_id = sm.new_scope(ScopeKind::Class {
            name: "User".to_string(),
            superclass: None,
        });

        assert_eq!(class_id, ScopeId(1));
        assert_eq!(sm.current_scope().id, ScopeId(0)); // Still in top-level
    }

    #[test]
    fn test_scope_manager_enter_exit() {
        let mut sm = ScopeManager::new();

        let class_id = sm.new_scope(ScopeKind::Class {
            name: "User".to_string(),
            superclass: None,
        });

        sm.enter_scope(class_id);
        assert_eq!(sm.current_scope().id, ScopeId(1));

        sm.exit_scope();
        assert_eq!(sm.current_scope().id, ScopeId(0));
    }

    #[test]
    fn test_scope_manager_local_var() {
        let mut sm = ScopeManager::new();

        sm.current_scope_mut()
            .set_local_var("x".to_string(), VertexId(10));

        assert_eq!(sm.lookup_var("x"), Some(VertexId(10)));
        assert_eq!(sm.lookup_var("y"), None);
    }

    #[test]
    fn test_scope_manager_nested_lookup() {
        let mut sm = ScopeManager::new();

        // Top level: x = 10
        sm.current_scope_mut()
            .set_local_var("x".to_string(), VertexId(10));

        // Enter class
        let class_id = sm.new_scope(ScopeKind::Class {
            name: "User".to_string(),
            superclass: None,
        });
        sm.enter_scope(class_id);

        // Class level: y = 20
        sm.current_scope_mut()
            .set_local_var("y".to_string(), VertexId(20));

        // Can lookup both x (from parent) and y (from current)
        assert_eq!(sm.lookup_var("x"), Some(VertexId(10)));
        assert_eq!(sm.lookup_var("y"), Some(VertexId(20)));
    }

    #[test]
    fn test_scope_manager_current_class_name() {
        let mut sm = ScopeManager::new();

        assert_eq!(sm.current_class_name(), None);

        let class_id = sm.new_scope(ScopeKind::Class {
            name: "User".to_string(),
            superclass: None,
        });
        sm.enter_scope(class_id);

        assert_eq!(sm.current_class_name(), Some("User".to_string()));

        // Enter method within class
        let method_id = sm.new_scope(ScopeKind::Method {
            name: "test".to_string(),
            receiver_type: None,
        });
        sm.enter_scope(method_id);

        // Should still find parent class name
        assert_eq!(sm.current_class_name(), Some("User".to_string()));
    }

    #[test]
    fn test_scope_manager_module_scope() {
        let mut sm = ScopeManager::new();

        assert_eq!(sm.current_module_name(), None);

        let module_id = sm.new_scope(ScopeKind::Module {
            name: "Utils".to_string(),
        });
        sm.enter_scope(module_id);

        assert_eq!(sm.current_module_name(), Some("Utils".to_string()));

        // Enter method within module
        let method_id = sm.new_scope(ScopeKind::Method {
            name: "helper".to_string(),
            receiver_type: Some("Utils".to_string()),
        });
        sm.enter_scope(method_id);

        // Should still find parent module name
        assert_eq!(sm.current_module_name(), Some("Utils".to_string()));

        sm.exit_scope(); // exit method
        sm.exit_scope(); // exit module

        assert_eq!(sm.current_module_name(), None);
    }

    #[test]
    fn test_scope_manager_module_instance_var() {
        let mut sm = ScopeManager::new();

        let module_id = sm.new_scope(ScopeKind::Module {
            name: "Config".to_string(),
        });
        sm.enter_scope(module_id);

        // Set instance variable in module
        sm.set_instance_var_in_module("@setting".to_string(), VertexId(100));

        // Enter method within module
        let method_id = sm.new_scope(ScopeKind::Method {
            name: "get_setting".to_string(),
            receiver_type: Some("Config".to_string()),
        });
        sm.enter_scope(method_id);

        // Should find instance variable from module scope
        assert_eq!(
            sm.lookup_instance_var_in_module("@setting"),
            Some(VertexId(100))
        );
    }

    #[test]
    fn test_current_qualified_name_simple_class() {
        let mut sm = ScopeManager::new();

        // module Api; class User; end; end
        let class_id = sm.new_scope(ScopeKind::Class {
            name: "User".to_string(),
            superclass: None,
        });
        sm.enter_scope(class_id);

        assert_eq!(
            sm.current_qualified_name(),
            Some("User".to_string())
        );
    }

    #[test]
    fn test_current_qualified_name_nested_module_class() {
        let mut sm = ScopeManager::new();

        // module Api
        let api_id = sm.new_scope(ScopeKind::Module {
            name: "Api".to_string(),
        });
        sm.enter_scope(api_id);

        // module V1
        let v1_id = sm.new_scope(ScopeKind::Module {
            name: "V1".to_string(),
        });
        sm.enter_scope(v1_id);

        // class User
        let user_id = sm.new_scope(ScopeKind::Class {
            name: "User".to_string(),
            superclass: None,
        });
        sm.enter_scope(user_id);

        assert_eq!(
            sm.current_qualified_name(),
            Some("Api::V1::User".to_string())
        );

        // def greet
        let method_id = sm.new_scope(ScopeKind::Method {
            name: "greet".to_string(),
            receiver_type: None,
        });
        sm.enter_scope(method_id);

        // Inside method, should still get the qualified class name
        assert_eq!(
            sm.current_qualified_name(),
            Some("Api::V1::User".to_string())
        );
    }

    #[test]
    fn test_current_qualified_name_with_inline_qualified_class() {
        let mut sm = ScopeManager::new();

        // class Api::User (defined at top level with qualified name)
        let class_id = sm.new_scope(ScopeKind::Class {
            name: "Api::User".to_string(),
            superclass: None,
        });
        sm.enter_scope(class_id);

        assert_eq!(
            sm.current_qualified_name(),
            Some("Api::User".to_string())
        );
    }

    #[test]
    fn test_current_qualified_name_at_top_level() {
        let sm = ScopeManager::new();

        // At top level, no class/module
        assert_eq!(sm.current_qualified_name(), None);
    }
}
