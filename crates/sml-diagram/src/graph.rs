//! Renderer-independent description of a diagram.

/// Index of a node within its [`Graph`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(pub(crate) usize);

/// How a node is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// Rounded box with a label, optional detail lines and optional nested graph.
    State,
    /// Initial pseudo-state: a filled dot.
    Initial,
    /// History pseudo-state: a circled `H`.
    History,
    /// Final / terminate state: a bullseye.
    Final,
}

/// A node: a state or pseudo-state.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub(crate) kind: NodeKind,
    pub(crate) label: String,
    pub(crate) details: Vec<String>,
    pub(crate) child: Option<Box<Graph>>,
}

/// A directed, optionally labelled edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge {
    pub(crate) from: NodeId,
    pub(crate) to: NodeId,
    pub(crate) label: String,
}

/// Directed graph whose nodes may contain nested graphs (composite states).
///
/// ```
/// use sml_diagram::{Graph, Node};
///
/// let mut graph = Graph::new("Door");
/// let start = graph.add_node(Node::initial());
/// let closed = graph.add_node(Node::state("Closed").with_detail("entry / reset"));
/// let open = graph.add_node(Node::state("Open"));
/// graph.add_edge(start, closed, "");
/// graph.add_edge(closed, open, "Open [is_unlocked]");
/// graph.add_edge(open, closed, "Close");
/// assert_eq!(graph.nodes().len(), 3);
/// assert!(graph.validate().is_ok());
/// ```
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Graph {
    pub(crate) title: String,
    pub(crate) nodes: Vec<Node>,
    pub(crate) edges: Vec<Edge>,
}

impl Node {
    /// A state box labelled `label`.
    pub fn state(label: impl Into<String>) -> Self {
        Self::new(NodeKind::State, label.into())
    }

    /// The initial pseudo-state.
    #[must_use]
    pub fn initial() -> Self {
        Self::new(NodeKind::Initial, String::new())
    }

    /// The (shallow) history pseudo-state.
    #[must_use]
    pub fn history() -> Self {
        Self::new(NodeKind::History, String::new())
    }

    /// The final / terminate state.
    #[must_use]
    pub fn terminal() -> Self {
        Self::new(NodeKind::Final, String::new())
    }

    fn new(kind: NodeKind, label: String) -> Self {
        Self {
            kind,
            label,
            details: Vec::new(),
            child: None,
        }
    }

    /// Adds a line below the label, e.g. `entry / reset`. Ignored for pseudo-states.
    #[must_use]
    pub fn with_detail(mut self, line: impl Into<String>) -> Self {
        self.details.push(line.into());
        self
    }

    /// Nests `child` inside this state (composite state). Ignored for pseudo-states.
    #[must_use]
    pub fn with_child(mut self, child: Graph) -> Self {
        self.child = Some(Box::new(child));
        self
    }

    /// How the node is drawn.
    #[must_use]
    pub fn kind(&self) -> NodeKind {
        self.kind
    }

    /// Label text (empty for pseudo-states).
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Lines drawn below the label.
    #[must_use]
    pub fn details(&self) -> &[String] {
        &self.details
    }

    /// Nested graph of a composite state.
    #[must_use]
    pub fn child(&self) -> Option<&Graph> {
        self.child.as_deref()
    }
}

impl Edge {
    /// Source node.
    #[must_use]
    pub fn from(&self) -> NodeId {
        self.from
    }

    /// Target node.
    #[must_use]
    pub fn to(&self) -> NodeId {
        self.to
    }

    /// Label text (may be empty).
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
}

impl Graph {
    /// Empty graph; `title` names the diagram for accessibility and element ids.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Adds `node`, returning its id.
    pub fn add_node(&mut self, node: Node) -> NodeId {
        self.nodes.push(node);
        NodeId(self.nodes.len() - 1)
    }

    /// Adds an edge; an empty `label` draws no text. Ids from another graph are
    /// reported by [`Graph::validate`] (and by rendering).
    pub fn add_edge(&mut self, from: NodeId, to: NodeId, label: impl Into<String>) {
        self.edges.push(Edge {
            from,
            to,
            label: label.into(),
        });
    }

    /// Diagram title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Nodes in insertion order.
    #[must_use]
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Edges in insertion order.
    #[must_use]
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// The node `id` refers to, if it belongs to this graph.
    #[must_use]
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.0)
    }

    /// Checks that every edge references a node of this graph (recursively).
    ///
    /// # Errors
    ///
    /// Returns the offending edge's index and endpoint when it points past the node list,
    /// which happens when mixing [`NodeId`]s of different graphs.
    pub fn validate(&self) -> Result<(), InvalidEdge> {
        for (index, edge) in self.edges.iter().enumerate() {
            for id in [edge.from, edge.to] {
                if id.0 >= self.nodes.len() {
                    return Err(InvalidEdge {
                        graph: self.title.clone(),
                        edge: index,
                        node: id.0,
                    });
                }
            }
        }
        for node in &self.nodes {
            if let Some(child) = &node.child {
                child.validate()?;
            }
        }
        Ok(())
    }
}

/// An edge refers to a node that does not exist in its graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidEdge {
    /// Title of the graph containing the edge.
    pub graph: String,
    /// Index of the edge, in insertion order.
    pub edge: usize,
    /// The out-of-range node index.
    pub node: usize,
}

impl std::fmt::Display for InvalidEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "graph `{}`: edge #{} refers to node #{}, which does not exist \
             (was the NodeId taken from another graph?)",
            self.graph, self.edge, self.node
        )
    }
}

impl std::error::Error for InvalidEdge {}

#[cfg(test)]
mod tests {
    use super::{Graph, Node, NodeKind};

    #[test]
    fn accessors_return_what_was_built() {
        let mut child = Graph::new("Child");
        child.add_node(Node::state("Inner"));
        let mut graph = Graph::new("Parent");
        let a = graph.add_node(Node::state("A").with_detail("entry / x").with_child(child));
        let end = graph.add_node(Node::terminal());
        graph.add_edge(a, end, "stop");

        assert_eq!(graph.title(), "Parent");
        assert_eq!(graph.nodes().len(), 2);
        let node = graph.node(a);
        assert_eq!(node.map(Node::label), Some("A"));
        assert_eq!(node.map(Node::details), Some(&["entry / x".to_owned()][..]));
        assert_eq!(node.and_then(Node::child).map(Graph::title), Some("Child"));
        assert_eq!(graph.node(end).map(Node::kind), Some(NodeKind::Final));
        assert!(graph.node(end).and_then(Node::child).is_none());

        let edge = &graph.edges()[0];
        assert_eq!((edge.from(), edge.to(), edge.label()), (a, end, "stop"));
        assert!(graph.validate().is_ok());
    }
}
