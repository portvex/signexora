use std::fmt;

use crate::{Error, Result, Signal};

/// Identifier of a node in a [`SignalGraph`]. Stable for the life of the session.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct NodeId(u32);

impl NodeId {
    /// The raw number behind the id, as written in saved sessions.
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// An id from its raw number (for reading saved sessions). The id refers to a node
    /// only if a graph has one with that number; lookups of unknown ids return `None`.
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// What produced a node: an operation name plus its parameters.
///
/// Parameters are stored as JSON so the graph can be saved and replayed without every
/// operation type being known to `sn-core`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Operation {
    /// Operation name, e.g. `"load"`, `"band_select"`, `"phase_detector"`.
    pub name: String,
    /// Operation parameters.
    pub params: serde_json::Value,
}

impl Operation {
    /// New operation record.
    pub fn new(name: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            params,
        }
    }
}

/// One signal in the session with its provenance.
#[derive(Debug, Clone)]
pub struct Node {
    /// Node identifier.
    pub id: NodeId,
    /// Human-readable label shown in the session tree.
    pub label: String,
    /// How this node was produced.
    pub operation: Operation,
    /// Nodes this one was derived from (empty for sources).
    pub inputs: Vec<NodeId>,
    /// The resulting signal.
    pub signal: Signal,
}

/// The derived-signal DAG of a session.
///
/// Nodes can only refer to nodes that already exist, so the graph is acyclic by
/// construction. Nodes are never mutated, except that [`SignalGraph::release_signal`]
/// may drop the samples of a node that is no longer used; "undo" is navigating back
/// to a parent.
#[derive(Debug, Default)]
pub struct SignalGraph {
    nodes: Vec<Node>,
}

impl SignalGraph {
    /// Empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a source signal (a loaded file or a synthesised signal).
    pub fn add_source(
        &mut self,
        label: impl Into<String>,
        operation: Operation,
        signal: Signal,
    ) -> NodeId {
        self.push(label.into(), operation, Vec::new(), signal)
    }

    /// Add a signal derived from existing nodes.
    pub fn add_derived(
        &mut self,
        label: impl Into<String>,
        operation: Operation,
        inputs: &[NodeId],
        signal: Signal,
    ) -> Result<NodeId> {
        if inputs.is_empty() {
            return Err(Error::NoInputs(operation.name));
        }
        if let Some(&missing) = inputs.iter().find(|id| self.get(**id).is_none()) {
            return Err(Error::UnknownNode(missing));
        }
        Ok(self.push(label.into(), operation, inputs.to_vec(), signal))
    }

    fn push(
        &mut self,
        label: String,
        operation: Operation,
        inputs: Vec<NodeId>,
        signal: Signal,
    ) -> NodeId {
        let id = NodeId(u32::try_from(self.nodes.len()).unwrap_or(u32::MAX));
        self.nodes.push(Node {
            id,
            label,
            operation,
            inputs,
            signal,
        });
        id
    }

    /// Look up a node.
    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.0 as usize)
    }

    /// Drop the samples of node `id` to free their memory, leaving an empty signal of
    /// the same kind, sample rate, centre frequency and unit. The node, its id, label,
    /// operation and inputs stay, so ids and lineage remain valid. For nodes the
    /// session has discarded; anything still reading the old samples keeps its own
    /// reference. Returns false when there is no such node.
    pub fn release_signal(&mut self, id: NodeId) -> bool {
        match self.nodes.get_mut(id.0 as usize) {
            Some(node) => {
                node.signal = node.signal.empty_like();
                true
            }
            None => false,
        }
    }

    /// All nodes in creation order.
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.iter()
    }

    /// Number of nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// True when the session has no signals.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Nodes with no inputs (loaded or synthesised signals).
    pub fn sources(&self) -> impl Iterator<Item = &Node> {
        self.nodes.iter().filter(|n| n.inputs.is_empty())
    }

    /// Nodes derived directly from `id`.
    pub fn children(&self, id: NodeId) -> impl Iterator<Item = &Node> {
        self.nodes.iter().filter(move |n| n.inputs.contains(&id))
    }

    /// The chain of operations that produced `id`, from its sources to `id` itself,
    /// in an order where every node comes after its inputs.
    pub fn lineage(&self, id: NodeId) -> Result<Vec<NodeId>> {
        if self.get(id).is_none() {
            return Err(Error::UnknownNode(id));
        }
        let mut seen = vec![false; self.nodes.len()];
        let mut stack = vec![id];
        while let Some(cur) = stack.pop() {
            let idx = cur.0 as usize;
            let (Some(flag), Some(node)) = (seen.get_mut(idx), self.nodes.get(idx)) else {
                return Err(Error::UnknownNode(cur));
            };
            if *flag {
                continue;
            }
            *flag = true;
            stack.extend(node.inputs.iter().copied());
        }
        // Node ids increase with creation order and inputs always precede their
        // outputs, so ascending id order is a valid topological order.
        Ok(seen
            .iter()
            .enumerate()
            .filter(|(_, s)| **s)
            .filter_map(|(i, _)| u32::try_from(i).ok().map(NodeId))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::*;
    use crate::SampleKind;

    fn sig() -> Signal {
        Signal::real(vec![0.0_f32; 16], 8000.0).unwrap()
    }

    #[test]
    fn release_signal_frees_the_samples_and_keeps_the_node() {
        let mut g = SignalGraph::new();
        let samples: Arc<[f32]> = Arc::from(vec![1.0_f32; 64]);
        let signal = Signal::real(Arc::clone(&samples), 8000.0)
            .unwrap()
            .with_center_frequency(1e6);
        let src = g.add_source("rec.wav", Operation::new("load", json!({})), signal);
        let child = g
            .add_derived("copy", Operation::new("copy", json!({})), &[src], sig())
            .unwrap();
        assert_eq!(Arc::strong_count(&samples), 2);
        assert!(g.release_signal(src));
        assert_eq!(Arc::strong_count(&samples), 1);
        let node = g.get(src).unwrap();
        assert!(node.signal.is_empty());
        assert_eq!(node.signal.kind(), SampleKind::Real);
        assert!((node.signal.sample_rate() - 8000.0).abs() < 1e-9);
        assert!((node.signal.center_frequency() - 1e6).abs() < 1e-3);
        assert_eq!(node.label, "rec.wav");
        assert_eq!(g.lineage(child).unwrap(), vec![src, child]);
        assert!(!g.release_signal(NodeId(99)));
    }

    #[test]
    fn builds_lineage_in_topological_order() {
        let mut g = SignalGraph::new();
        let src = g.add_source(
            "rec.wav",
            Operation::new("load", json!({"path": "rec.wav"})),
            sig(),
        );
        let other = g.add_source("ref.wav", Operation::new("load", json!({})), sig());
        let band = g
            .add_derived(
                "band",
                Operation::new("band_select", json!({"lo": 300})),
                &[src],
                sig(),
            )
            .unwrap();
        let det = g
            .add_derived(
                "det",
                Operation::new("phase_detector", json!({})),
                &[band],
                sig(),
            )
            .unwrap();
        let ccf = g
            .add_derived(
                "ccf",
                Operation::new("ccf", json!({})),
                &[det, other],
                sig(),
            )
            .unwrap();

        assert_eq!(g.lineage(ccf).unwrap(), vec![src, other, band, det, ccf]);
        assert_eq!(g.lineage(band).unwrap(), vec![src, band]);
        assert_eq!(
            g.children(src).map(|n| n.id).collect::<Vec<_>>(),
            vec![band]
        );
        assert_eq!(g.sources().count(), 2);
    }

    #[test]
    fn rejects_unknown_or_missing_inputs() {
        let mut g = SignalGraph::new();
        let op = || Operation::new("x", json!({}));
        assert!(matches!(
            g.add_derived("a", op(), &[], sig()),
            Err(Error::NoInputs(_))
        ));
        assert!(matches!(
            g.add_derived("a", op(), &[NodeId(7)], sig()),
            Err(Error::UnknownNode(NodeId(7)))
        ));
        assert!(g.lineage(NodeId(0)).is_err());
    }
}
