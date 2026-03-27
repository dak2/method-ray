use crate::graph::VertexId;
use std::collections::HashMap;

/// Scope ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(pub usize);

/// Scope kind
#[derive(Debug, Clone)]
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
        return_vertex: Option<VertexId>, // Merge vertex for return statements
    },
    Block,
}

/// Scope information
#[derive(Debug, Clone)]
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

    /// Constants (simple name → qualified name)
    pub constants: HashMap<String, String>,
}

impl Scope {
    pub fn new(id: ScopeId, kind: ScopeKind, parent: Option<ScopeId>) -> Self {
        Self {
            id,
            kind,
            parent,
            local_vars: HashMap::new(),
            instance_vars: HashMap::new(),
            class_vars: HashMap::new(),
            constants: HashMap::new(),
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

    /// Add class variable
    pub fn set_class_var(&mut self, name: String, vtx: VertexId) {
        self.class_vars.insert(name, vtx);
    }

    /// Get class variable
    pub fn get_class_var(&self, name: &str) -> Option<VertexId> {
        self.class_vars.get(name).copied()
    }
}

/// Scope manager
#[derive(Debug)]
pub struct ScopeManager {
    scopes: HashMap<ScopeId, Scope>,
    next_id: usize,
    current_scope: ScopeId,
}

impl Default for ScopeManager {
    fn default() -> Self {
        Self::new()
    }
}

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

    /// Walk scopes from current scope up to the top-level, yielding each scope
    fn walk_scopes(&self) -> impl Iterator<Item = &Scope> + '_ {
        let scopes = &self.scopes;
        let mut current = Some(self.current_scope);
        std::iter::from_fn(move || {
            let scope_id = current?;
            let scope = scopes.get(&scope_id)?;
            current = scope.parent;
            Some(scope)
        })
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
        self.walk_scopes().find_map(|scope| scope.get_local_var(name))
    }

    /// Lookup constant in current scope or parent scopes (simple name → qualified name)
    pub fn lookup_constant(&self, simple_name: &str) -> Option<String> {
        self.walk_scopes()
            .find_map(|scope| scope.constants.get(simple_name).cloned())
    }

    /// Lookup instance variable in enclosing class scope
    pub fn lookup_instance_var(&self, name: &str) -> Option<VertexId> {
        self.walk_scopes()
            .find(|scope| matches!(&scope.kind, ScopeKind::Class { .. }))
            .and_then(|scope| scope.get_instance_var(name))
    }

    /// Set instance variable in enclosing class scope
    pub fn set_instance_var_in_class(&mut self, name: String, vtx: VertexId) {
        let class_scope_id = self.walk_scopes()
            .find(|scope| matches!(&scope.kind, ScopeKind::Class { .. }))
            .map(|scope| scope.id);
        if let Some(scope_id) = class_scope_id {
            if let Some(scope) = self.scopes.get_mut(&scope_id) {
                scope.set_instance_var(name, vtx);
            }
        }
    }

    /// Get current class name (simple name, not qualified)
    pub fn current_class_name(&self) -> Option<String> {
        self.walk_scopes().find_map(|scope| {
            if let ScopeKind::Class { name, .. } = &scope.kind {
                Some(name.clone())
            } else {
                None
            }
        })
    }

    /// Get current module name (simple name, not qualified)
    pub fn current_module_name(&self) -> Option<String> {
        self.walk_scopes().find_map(|scope| {
            if let ScopeKind::Module { name } = &scope.kind {
                Some(name.clone())
            } else {
                None
            }
        })
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
        let mut path_segments: Vec<&str> = self.walk_scopes()
            .filter_map(|scope| match &scope.kind {
                ScopeKind::Class { name, .. } | ScopeKind::Module { name } => Some(name.as_str()),
                _ => None,
            })
            .collect();

        if path_segments.is_empty() {
            return None;
        }

        // Reverse to get from outermost to innermost
        path_segments.reverse();
        Some(path_segments.join("::"))
    }

    /// Get current method name from nearest enclosing method scope
    pub fn current_method_name(&self) -> Option<String> {
        self.walk_scopes().find_map(|scope| {
            if let ScopeKind::Method { name, .. } = &scope.kind {
                Some(name.clone())
            } else {
                None
            }
        })
    }

    /// Get superclass name from nearest enclosing class scope
    pub fn current_superclass(&self) -> Option<String> {
        self.walk_scopes().find_map(|scope| {
            if let ScopeKind::Class { superclass, .. } = &scope.kind {
                superclass.clone()
            } else {
                None
            }
        })
    }

    /// Get return_vertex from the nearest enclosing method scope
    pub fn current_method_return_vertex(&self) -> Option<VertexId> {
        self.walk_scopes().find_map(|scope| {
            if let ScopeKind::Method { return_vertex, .. } = &scope.kind {
                *return_vertex
            } else {
                None
            }
        })
    }

    /// Lookup instance variable in enclosing module scope
    pub fn lookup_instance_var_in_module(&self, name: &str) -> Option<VertexId> {
        self.walk_scopes()
            .find(|scope| matches!(&scope.kind, ScopeKind::Module { .. }))
            .and_then(|scope| scope.get_instance_var(name))
    }

    /// Set instance variable in enclosing module scope
    pub fn set_instance_var_in_module(&mut self, name: String, vtx: VertexId) {
        let module_scope_id = self.walk_scopes()
            .find(|scope| matches!(&scope.kind, ScopeKind::Module { .. }))
            .map(|scope| scope.id);
        if let Some(scope_id) = module_scope_id {
            if let Some(scope) = self.scopes.get_mut(&scope_id) {
                scope.set_instance_var(name, vtx);
            }
        }
    }

    /// Lookup class variable in enclosing class scope.
    ///
    /// Note: Only searches `ScopeKind::Class` scopes. Module-scoped @@var and
    /// inheritance-chain traversal are not supported in v0.2.0.
    pub fn lookup_class_var(&self, name: &str) -> Option<VertexId> {
        self.walk_scopes()
            .find(|scope| matches!(&scope.kind, ScopeKind::Class { .. }))
            .and_then(|scope| scope.get_class_var(name))
    }

    /// Set class variable in enclosing class scope.
    ///
    /// No-op if there is no enclosing `ScopeKind::Class` scope (e.g., top-level or module scope).
    /// Module-scoped @@var support is planned for a future version.
    pub fn set_class_var_in_class(&mut self, name: String, vtx: VertexId) {
        let class_scope_id = self.walk_scopes()
            .find(|scope| matches!(&scope.kind, ScopeKind::Class { .. }))
            .map(|scope| scope.id);
        if let Some(scope_id) = class_scope_id {
            if let Some(scope) = self.scopes.get_mut(&scope_id) {
                scope.set_class_var(name, vtx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_var_set_and_get() {
        let mut scope = Scope::new(ScopeId(0), ScopeKind::TopLevel, None);
        let vtx = VertexId(1);
        scope.set_class_var("@@count".to_string(), vtx);
        assert_eq!(scope.get_class_var("@@count"), Some(vtx));
        assert_eq!(scope.get_class_var("@@missing"), None);
    }

    #[test]
    fn test_lookup_class_var_from_method_scope() {
        let mut manager = ScopeManager::new();

        // Class scope
        let class_id = manager.new_scope(ScopeKind::Class {
            name: "Counter".to_string(),
            superclass: None,
        });
        manager.enter_scope(class_id);

        let vtx = VertexId(42);
        manager.set_class_var_in_class("@@count".to_string(), vtx);

        // Method scope (inside the class)
        let method_id = manager.new_scope(ScopeKind::Method {
            name: "increment".to_string(),
            receiver_type: Some("Counter".to_string()),
            return_vertex: None,
        });
        manager.enter_scope(method_id);

        // Should find @@count through the class scope
        assert_eq!(manager.lookup_class_var("@@count"), Some(vtx));
    }

    #[test]
    fn test_set_class_var_noop_without_class_scope() {
        let mut manager = ScopeManager::new();
        // At top-level, no class scope exists
        manager.set_class_var_in_class("@@var".to_string(), VertexId(1));
        assert_eq!(manager.lookup_class_var("@@var"), None);
    }
}
