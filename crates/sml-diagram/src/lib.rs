//! Pure-Rust layered graph layout and SVG rendering for state machine diagrams.
//!
//! No Graphviz, no fonts, no dependencies: build a [`Graph`], call [`render_svg`].
//! Nodes are laid out top to bottom in layers (Sugiyama-style); edges are smooth
//! curves with labels placed beside them; states may contain nested graphs.
//!
//! ```
//! use sml_diagram::{Graph, Node, Style, render_svg};
//!
//! let mut graph = Graph::new("Door");
//! let start = graph.add_node(Node::initial());
//! let closed = graph.add_node(Node::state("Closed"));
//! let opened = graph.add_node(Node::state("Opened").with_detail("Ping / log"));
//! graph.add_edge(start, closed, "");
//! graph.add_edge(closed, opened, "Open [is_unlocked]");
//! graph.add_edge(opened, closed, "Close");
//!
//! let svg = render_svg(&graph, Style::Standalone)?;
//! assert!(svg.contains("Open [is_unlocked]"));
//! # Ok::<(), sml_diagram::InvalidEdge>(())
//! ```

mod graph;
mod layout;
mod svg;
mod text;

pub use graph::{Edge, Graph, InvalidEdge, Node, NodeId, NodeKind};
pub use svg::Style;

/// Lays out `graph` and renders it as a single-line SVG document.
///
/// # Errors
///
/// [`InvalidEdge`] when an edge uses a [`NodeId`] that is not part of its graph.
pub fn render_svg(graph: &Graph, style: Style) -> Result<String, InvalidEdge> {
    graph.validate()?;
    let layout = layout::layout(graph);
    Ok(svg::document(graph, &layout, style))
}
