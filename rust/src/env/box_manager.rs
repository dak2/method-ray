//! Box management and execution queue
//!
//! Handles registration and execution of Box instances (reactive computations).

use crate::graph::{BoxId, BoxTrait};
use std::collections::{HashMap, HashSet, VecDeque};

/// Manages boxes and their execution queue
#[allow(dead_code)]
pub struct BoxManager {
    /// All registered boxes
    pub boxes: HashMap<BoxId, Box<dyn BoxTrait>>,
    /// Queue of boxes to be executed
    pub run_queue: VecDeque<BoxId>,
    /// Set to prevent duplicate queue entries
    run_queue_set: HashSet<BoxId>,
    /// Next box ID to allocate
    pub next_box_id: usize,
}

impl Default for BoxManager {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl BoxManager {
    /// Create a new empty box manager
    pub fn new() -> Self {
        Self {
            boxes: HashMap::new(),
            run_queue: VecDeque::new(),
            run_queue_set: HashSet::new(),
            next_box_id: 0,
        }
    }

    /// Get a box by ID
    pub fn get(&self, id: BoxId) -> Option<&Box<dyn BoxTrait>> {
        self.boxes.get(&id)
    }

    /// Remove a box and return it (for temporary mutation)
    pub fn remove(&mut self, id: BoxId) -> Option<Box<dyn BoxTrait>> {
        self.boxes.remove(&id)
    }

    /// Insert a box back after temporary removal
    pub fn insert(&mut self, id: BoxId, box_instance: Box<dyn BoxTrait>) {
        self.boxes.insert(id, box_instance);
    }

    /// Check if a box exists
    pub fn contains(&self, id: BoxId) -> bool {
        self.boxes.contains_key(&id)
    }

    /// Add a box to the execution queue
    pub fn add_run(&mut self, box_id: BoxId) {
        if !self.run_queue_set.contains(&box_id) {
            self.run_queue.push_back(box_id);
            self.run_queue_set.insert(box_id);
        }
    }

    /// Pop the next box from the queue
    pub fn pop_run(&mut self) -> Option<BoxId> {
        if let Some(box_id) = self.run_queue.pop_front() {
            self.run_queue_set.remove(&box_id);
            Some(box_id)
        } else {
            None
        }
    }

    /// Check if the queue is empty
    pub fn queue_is_empty(&self) -> bool {
        self.run_queue.is_empty()
    }

    /// Get the number of registered boxes
    pub fn len(&self) -> usize {
        self.boxes.len()
    }

    /// Check if there are no registered boxes
    pub fn is_empty(&self) -> bool {
        self.boxes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_run_prevents_duplicates() {
        let mut manager = BoxManager::new();

        let id = BoxId(0);
        manager.add_run(id);
        manager.add_run(id); // Should be ignored

        assert_eq!(manager.run_queue.len(), 1);
    }

    #[test]
    fn test_pop_run() {
        let mut manager = BoxManager::new();

        let id1 = BoxId(0);
        let id2 = BoxId(1);
        manager.add_run(id1);
        manager.add_run(id2);

        assert_eq!(manager.pop_run(), Some(id1));
        assert_eq!(manager.pop_run(), Some(id2));
        assert_eq!(manager.pop_run(), None);
    }
}
