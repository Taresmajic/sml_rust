//! Layered (Sugiyama-style) layout, top to bottom.
//!
//! 1. [`acyclic`]: reverse back edges found by DFS so the graph flows downward.
//! 2. [`rank`]: longest-path layering on doubled ranks; odd ranks hold edge labels.
//! 3. [`network`]: split long edges into dummy nodes, one of which carries the label.
//! 4. [`order`]: barycenter sweeps plus adjacent swaps to reduce crossings.
//! 5. [`position`]: x by isotonic regression towards neighbours, y by rank rows.
//! 6. [`route`]: polylines through dummies, clipped to node outlines; self-loops.

mod acyclic;
pub mod measure;
mod network;
mod order;
mod position;
mod rank;
mod route;
#[cfg(test)]
mod tests;

use crate::graph::{Graph, NodeKind};
use crate::text;

pub use measure::ChildLayout;

/// Blank space around a laid-out graph.
pub const MARGIN: f64 = 16.0;
/// Horizontal distance from an edge to its label text.
pub const LABEL_GAP: f64 = 8.0;
/// How far the innermost self-loop reaches right of its node.
pub const LOOP_REACH: f64 = 26.0;
/// Extra reach of each further self-loop on the same node.
pub const LOOP_STEP: f64 = 10.0;

/// A point in SVG user units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// Positions for every node and edge of a [`Graph`], index-aligned with it.
#[derive(Clone, Debug)]
pub struct Layout {
    pub width: f64,
    pub height: f64,
    pub nodes: Vec<PlacedNode>,
    pub edges: Vec<PlacedEdge>,
}

#[derive(Clone, Debug)]
pub struct PlacedNode {
    pub center: Point,
    pub width: f64,
    pub height: f64,
    pub child: Option<ChildLayout>,
}

#[derive(Clone, Debug)]
pub enum EdgePath {
    /// Points from source outline to target outline.
    Polyline(Vec<Point>),
    /// Self-loop on the right side of a node, from `from` around to `to`.
    Loop { from: Point, to: Point, reach: f64 },
}

#[derive(Clone, Debug)]
pub struct PlacedEdge {
    pub path: EdgePath,
    /// Left end, vertical middle of the label text.
    pub label: Option<Point>,
}

/// A non-self edge, by graph edge index.
#[derive(Clone, Copy, Debug)]
pub struct Link {
    pub edge: usize,
    pub from: usize,
    pub to: usize,
    pub label_width: Option<f64>,
}

/// Lays out `graph`; nested graphs are laid out first and placed inside their nodes.
pub fn layout(graph: &Graph) -> Layout {
    let measured: Vec<_> = graph.nodes.iter().map(measure::measure).collect();
    let mut links = Vec::new();
    let mut loops = Vec::new();
    for (index, edge) in graph.edges.iter().enumerate() {
        let label_width = (!edge.label.is_empty()).then(|| text::width(&edge.label));
        if edge.from == edge.to {
            loops.push((index, edge.from.0, label_width));
        } else {
            let (from, to) = (edge.from.0, edge.to.0);
            links.push(Link {
                edge: index,
                from,
                to,
                label_width,
            });
        }
    }
    let roots = root_order(graph);
    let dfs = acyclic::search(graph.nodes.len(), &links, &roots);
    let ranks = rank::assign(graph.nodes.len(), &links, &dfs.reversed);
    let loop_space = route::loop_space(graph.nodes.len(), &loops);
    let mut net = network::Network::build(&measured, &ranks, &links, &dfs.reversed, &loop_space);
    order::minimize_crossings(&mut net, &dfs.discovery);
    position::assign(&mut net);
    route::build(graph, measured, &net, &links, &dfs.reversed, &loops)
}

/// Pseudo-states first so DFS (and thus the downward flow) starts at the initial state.
fn root_order(graph: &Graph) -> Vec<usize> {
    let mut roots = Vec::new();
    for pass in [true, false] {
        for (index, node) in graph.nodes.iter().enumerate() {
            let pseudo = match node.kind {
                NodeKind::Initial | NodeKind::History => true,
                NodeKind::State | NodeKind::Final => false,
            };
            if pseudo == pass {
                roots.push(index);
            }
        }
    }
    roots
}

#[cfg(test)]
mod root_tests {
    use super::root_order;
    use crate::graph::{Graph, Node};

    #[test]
    fn pseudo_states_come_first_then_graph_order() {
        let mut graph = Graph::new("Roots");
        graph.add_node(Node::state("A"));
        graph.add_node(Node::terminal());
        graph.add_node(Node::history());
        graph.add_node(Node::state("B"));
        graph.add_node(Node::initial());
        assert_eq!(root_order(&graph), [2, 4, 0, 1, 3]);
    }
}
