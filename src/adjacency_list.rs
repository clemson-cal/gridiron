use core::hash::Hash;
use std::collections::HashMap;

/// A minimal directed graph structure that stores only edges
pub struct AdjacencyList<K> {
    outgoing: HashMap<K, Vec<K>>,
    incoming: HashMap<K, Vec<K>>,
}

impl<K> AdjacencyList<K>
where
    K: Hash + Eq + Clone,
{
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the number of edges in the graph.
    pub fn len(&self) -> usize {
        self.incoming.iter().map(|(_, edges)| edges.len()).sum()
    }

    /// Determine whether there are any edges in the graph.
    pub fn is_empty(&self) -> bool {
        self.incoming.iter().all(|(_, edges)| edges.is_empty())
    }

    /// Insert an edge from a -> b. Duplicate and circular edges are allowed.
    pub fn insert(&mut self, a0: K, b0: K) {
        let a1 = a0.clone();
        let b1 = b0.clone();
        self.outgoing.entry(a0).or_default().push(b0);
        self.incoming.entry(b1).or_default().push(a1);
    }

    /// Determine whether the given edge exists.
    pub fn contains(&mut self, a: &K, b: &K) -> bool {
        self.outgoing
            .get(a)
            .and_then(|edges| edges.iter().find(|&k| k == b))
            .is_some()
    }

    /// Remove an edge if it exists.
    pub fn remove(&mut self, a0: K, b0: K) {
        let a1 = a0.clone();
        let b1 = b0.clone();
        self.outgoing
            .entry(a0)
            .and_modify(|edges| edges.retain(|k| k != &b0));
        self.incoming
            .entry(b1)
            .and_modify(|edges| edges.retain(|k| k != &a1));
    }

    /// Return an iterator over the vertices with edges emanating from the given
    /// vertex.
    pub fn outgoing_edges(&self, a: &K) -> impl Iterator<Item = &K> {
        self.outgoing
            .get(a)
            .into_iter()
            .flat_map(|edges| edges.iter())
    }

    /// Return an iterator over the vertices with edges pointing to the given
    /// vertex.
    pub fn incoming_edges(&self, b: &K) -> impl Iterator<Item = &K> {
        self.incoming
            .get(b)
            .into_iter()
            .flat_map(|edges| edges.iter())
    }
}

impl<K> Default for AdjacencyList<K> {
    fn default() -> Self {
        Self {
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::AdjacencyList;

    #[test]
    fn graph_contained_works() {
        let mut edges = AdjacencyList::new();
        edges.insert(0, 1);
        assert!(edges.contains(&0, &1));
        assert!(!edges.contains(&1, &0));
    }

    #[test]
    fn graph_has_the_correct_length() {
        let mut edges = AdjacencyList::new();
        edges.insert(0, 1);
        edges.insert(1, 0);
        edges.insert(1, 1);
        edges.insert(0, 0);
        assert_eq!(edges.len(), 4);
    }

    #[test]
    fn graph_can_remove_edge() {
        let mut edges = AdjacencyList::new();
        edges.insert(0, 1);
        edges.insert(1, 0);
        edges.remove(1, 0);
        assert!(edges.contains(&0, &1));
        assert!(!edges.contains(&1, &0));
        assert_eq!(edges.len(), 1);
    }

    #[test]
    fn graph_can_iterate_incoming_and_outgoing_edges() {
        let mut edges = AdjacencyList::new();
        edges.insert(0, 1);
        edges.insert(0, 2);
        edges.insert(0, 3);

        edges.insert(4, 1);
        edges.insert(4, 2);

        assert_eq!(edges.incoming_edges(&1).count(), 2);
        assert_eq!(edges.incoming_edges(&3).count(), 1);
        assert_eq!(edges.outgoing_edges(&0).count(), 3);
        assert_eq!(edges.outgoing_edges(&4).count(), 2);
    }
}
