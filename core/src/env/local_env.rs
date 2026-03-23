use crate::graph::VertexId;
use std::collections::HashMap;

/// Local environment: mapping of local variable names to VertexIDs
pub struct LocalEnv {
    locals: HashMap<String, VertexId>,
}

impl Default for LocalEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalEnv {
    pub fn new() -> Self {
        Self {
            locals: HashMap::new(),
        }
    }

    /// Register variable
    pub fn new_var(&mut self, name: String, vtx_id: VertexId) {
        self.locals.insert(name, vtx_id);
    }

    /// Get variable
    pub fn get_var(&self, name: &str) -> Option<VertexId> {
        self.locals.get(name).copied()
    }

    /// Remove a variable from the local environment.
    /// Used for scoped variables like rescue's `=> e` binding.
    pub fn remove_var(&mut self, name: &str) {
        self.locals.remove(name);
    }

    /// Get all variables
    pub fn all_vars(&self) -> impl Iterator<Item = (&String, &VertexId)> {
        self.locals.iter()
    }
}
