use crate::graph::VertexId;
use std::collections::HashMap;

/// Local environment: mapping of local variable names to VertexIDs
pub struct LocalEnv {
    locals: HashMap<String, VertexId>,
}

#[allow(dead_code)]
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

    /// Get all variables
    pub fn all_vars(&self) -> impl Iterator<Item = (&String, &VertexId)> {
        self.locals.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_env() {
        let mut lenv = LocalEnv::new();

        lenv.new_var("x".to_string(), VertexId(1));
        lenv.new_var("y".to_string(), VertexId(2));

        assert_eq!(lenv.get_var("x"), Some(VertexId(1)));
        assert_eq!(lenv.get_var("y"), Some(VertexId(2)));
        assert_eq!(lenv.get_var("z"), None);
    }

    #[test]
    fn test_local_env_override() {
        let mut lenv = LocalEnv::new();

        lenv.new_var("x".to_string(), VertexId(1));
        lenv.new_var("x".to_string(), VertexId(2)); // Override

        assert_eq!(lenv.get_var("x"), Some(VertexId(2)));
    }
}
