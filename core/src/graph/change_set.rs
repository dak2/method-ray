use super::r#box::BoxId;
use super::VertexId;

/// Manages edge changes for type propagation
#[derive(Debug, Clone)]
pub struct ChangeSet {
    new_edges: Vec<(VertexId, VertexId)>,
    edges: Vec<(VertexId, VertexId)>,
    /// Boxes to reschedule for later execution
    reschedule_boxes: Vec<BoxId>,
}

impl Default for ChangeSet {
    fn default() -> Self {
        Self::new()
    }
}

impl ChangeSet {
    pub fn new() -> Self {
        Self {
            new_edges: Vec::new(),
            edges: Vec::new(),
            reschedule_boxes: Vec::new(),
        }
    }

    /// Add edge
    pub fn add_edge(&mut self, src: VertexId, dst: VertexId) {
        self.new_edges.push((src, dst));
    }

    /// Request to reschedule a Box for later execution
    pub fn reschedule(&mut self, box_id: BoxId) {
        self.reschedule_boxes.push(box_id);
    }

    /// Get and clear boxes that need to be rescheduled
    pub fn take_reschedule_boxes(&mut self) -> Vec<BoxId> {
        std::mem::take(&mut self.reschedule_boxes)
    }

    /// Commit changes and return list of added/removed edges
    pub fn reinstall(&mut self) -> Vec<EdgeUpdate> {
        // Remove duplicates
        self.new_edges.sort_by_key(|&(src, dst)| (src.0, dst.0));
        self.new_edges.dedup();

        let mut updates = Vec::new();

        // New edges
        for &(src, dst) in &self.new_edges {
            if !self.edges.contains(&(src, dst)) {
                updates.push(EdgeUpdate::Add { src, dst });
            }
        }

        // Removed edges
        for &(src, dst) in &self.edges {
            if !self.new_edges.contains(&(src, dst)) {
                updates.push(EdgeUpdate::Remove { src, dst });
            }
        }

        // Commit edges
        std::mem::swap(&mut self.edges, &mut self.new_edges);
        self.new_edges.clear();

        updates
    }
}

/// Edge update type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeUpdate {
    Add { src: VertexId, dst: VertexId },
    Remove { src: VertexId, dst: VertexId },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_set_default() {
        let mut cs = ChangeSet::default();
        cs.add_edge(VertexId(1), VertexId(2));
        let updates = cs.reinstall();
        assert_eq!(updates.len(), 1);
    }

    #[test]
    fn test_change_set_add() {
        let mut cs = ChangeSet::new();

        cs.add_edge(VertexId(1), VertexId(2));
        cs.add_edge(VertexId(2), VertexId(3));

        let updates = cs.reinstall();

        assert_eq!(updates.len(), 2);
        assert!(updates.contains(&EdgeUpdate::Add {
            src: VertexId(1),
            dst: VertexId(2)
        }));
        assert!(updates.contains(&EdgeUpdate::Add {
            src: VertexId(2),
            dst: VertexId(3)
        }));
    }

    #[test]
    fn test_change_set_dedup() {
        let mut cs = ChangeSet::new();

        cs.add_edge(VertexId(1), VertexId(2));
        cs.add_edge(VertexId(1), VertexId(2)); // Duplicate

        let updates = cs.reinstall();

        assert_eq!(updates.len(), 1); // Duplicates removed
    }

    #[test]
    fn test_change_set_remove() {
        let mut cs = ChangeSet::new();

        // First commit
        cs.add_edge(VertexId(1), VertexId(2));
        cs.add_edge(VertexId(2), VertexId(3));
        cs.reinstall();

        // Second time: keep only (1,2)
        cs.add_edge(VertexId(1), VertexId(2));
        let updates = cs.reinstall();

        assert_eq!(updates.len(), 1);
        assert!(updates.contains(&EdgeUpdate::Remove {
            src: VertexId(2),
            dst: VertexId(3)
        }));
    }
}
