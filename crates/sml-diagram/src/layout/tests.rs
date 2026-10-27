//! Layout invariants over random graphs.

use proptest::prelude::*;

use crate::graph::{Graph, Node, NodeKind};
use crate::layout::{EdgePath, Layout, layout};
use crate::text::{self, LINE_HEIGHT};
use crate::{Style, render_svg};

const EPS: f64 = 1e-6;

#[derive(Clone, Copy, Debug)]
struct Rect {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

impl Rect {
    fn overlaps(self, other: Self) -> bool {
        self.left < other.right - EPS
            && other.left < self.right - EPS
            && self.top < other.bottom - EPS
            && other.top < self.bottom - EPS
    }

    fn within(self, width: f64, height: f64) -> bool {
        self.left >= -EPS
            && self.top >= -EPS
            && self.right <= width + EPS
            && self.bottom <= height + EPS
    }
}

fn node_rects(layout: &Layout) -> Vec<Rect> {
    layout
        .nodes
        .iter()
        .map(|n| Rect {
            left: n.center.x - n.width / 2.0,
            top: n.center.y - n.height / 2.0,
            right: n.center.x + n.width / 2.0,
            bottom: n.center.y + n.height / 2.0,
        })
        .collect()
}

fn label_rects(graph: &Graph, layout: &Layout) -> Vec<Rect> {
    let mut rects = Vec::new();
    for (edge, placed) in graph.edges.iter().zip(&layout.edges) {
        if let Some(at) = placed.label {
            rects.push(Rect {
                left: at.x,
                top: at.y - LINE_HEIGHT / 2.0,
                right: at.x + text::width(&edge.label),
                bottom: at.y + LINE_HEIGHT / 2.0,
            });
        }
    }
    rects
}

fn node_strategy() -> impl Strategy<Value = Node> {
    prop_oneof![
        6 => "[A-Za-z][A-Za-z0-9_]{0,14}".prop_map(Node::state),
        1 => Just(Node::initial()),
        1 => Just(Node::history()),
        1 => Just(Node::terminal()),
        2 => ("[A-Z][a-z]{0,8}", "[a-z /]{1,24}")
            .prop_map(|(label, detail)| Node::state(label).with_detail(detail)),
    ]
}

fn flat_graph() -> impl Strategy<Value = Graph> {
    (1usize..9)
        .prop_flat_map(|count| {
            let nodes = prop::collection::vec(node_strategy(), count);
            let edges = prop::collection::vec((0..count, 0..count, "[a-z \\[\\]/]{0,18}"), 0..14);
            (nodes, edges)
        })
        .prop_map(|(nodes, edges)| {
            let mut graph = Graph::new("Random");
            let ids: Vec<_> = nodes.into_iter().map(|node| graph.add_node(node)).collect();
            for (from, to, label) in edges {
                graph.add_edge(ids[from], ids[to], label);
            }
            graph
        })
}

fn graph_strategy() -> impl Strategy<Value = Graph> {
    (flat_graph(), prop::option::of(flat_graph())).prop_map(|(mut graph, child)| {
        if let Some(child) = child {
            graph.add_node(Node::state("Composite").with_child(child));
        }
        graph
    })
}

fn assert_invariants(graph: &Graph, layout: &Layout) -> Result<(), TestCaseError> {
    prop_assert_eq!(layout.nodes.len(), graph.nodes.len());
    prop_assert_eq!(layout.edges.len(), graph.edges.len());
    prop_assert!(layout.width.is_finite() && layout.height.is_finite());
    for (index, (edge, placed)) in graph.edges.iter().zip(&layout.edges).enumerate() {
        prop_assert_eq!(
            placed.label.is_some(),
            !edge.label.is_empty(),
            "edge {} label placement",
            index
        );
    }
    let nodes = node_rects(layout);
    let labels = label_rects(graph, layout);
    for (i, rect) in nodes.iter().enumerate() {
        prop_assert!(
            rect.within(layout.width, layout.height),
            "node {i} outside canvas"
        );
        for (j, other) in nodes.iter().enumerate().skip(i + 1) {
            prop_assert!(!rect.overlaps(*other), "nodes {i} and {j} overlap");
        }
        for (j, label) in labels.iter().enumerate() {
            prop_assert!(!rect.overlaps(*label), "node {i} overlaps label {j}");
        }
    }
    for (i, label) in labels.iter().enumerate() {
        prop_assert!(
            label.within(layout.width, layout.height),
            "label {i} outside canvas"
        );
        for (j, other) in labels.iter().enumerate().skip(i + 1) {
            prop_assert!(!label.overlaps(*other), "labels {i} and {j} overlap");
        }
    }
    for (index, edge) in layout.edges.iter().enumerate() {
        match &edge.path {
            EdgePath::Polyline(points) => {
                prop_assert!(points.len() >= 2);
                for point in points {
                    let inside = point.x >= -EPS
                        && point.y >= -EPS
                        && point.x <= layout.width + EPS
                        && point.y <= layout.height + EPS;
                    prop_assert!(inside, "edge {index} point {point:?} outside canvas");
                }
            }
            EdgePath::Loop { from, to, reach } => {
                prop_assert!(
                    from.x + reach <= layout.width + EPS,
                    "loop {index} outside canvas"
                );
                prop_assert!(from.y >= -EPS && to.y <= layout.height + EPS);
            }
        }
    }
    for (node, placed) in graph.nodes.iter().zip(&layout.nodes) {
        if let (Some(child), Some(placed_child)) = (&node.child, &placed.child) {
            assert_invariants(child, &placed_child.layout)?;
        }
    }
    Ok(())
}

/// Every opened tag is closed in order (the renderer emits no comments or CDATA).
fn well_formed(svg: &str) -> bool {
    let mut stack: Vec<&str> = Vec::new();
    let mut rest = svg;
    while let Some(start) = rest.find('<') {
        let Some(end) = rest[start..].find('>') else {
            return false;
        };
        let tag = &rest[start + 1..start + end];
        rest = &rest[start + end + 1..];
        if let Some(name) = tag.strip_prefix('/') {
            if stack.pop() != Some(name) {
                return false;
            }
        } else if !tag.ends_with('/') {
            stack.push(tag.split_whitespace().next().unwrap_or(""));
        }
    }
    stack.is_empty() && rest.trim().is_empty()
}

/// No geometry attribute (anything but `aria-label`) holds `NaN` or `inf`; text is ignored.
fn numbers_finite(svg: &str) -> bool {
    let mut rest = svg;
    while let Some(start) = rest.find('<') {
        let end = rest[start..]
            .find('>')
            .map_or(rest.len(), |end| start + end);
        let tag = &rest[start..end];
        rest = &rest[end..];
        for attribute in tag.split("\" ") {
            let Some((name, value)) = attribute.split_once("=\"") else {
                continue;
            };
            if !name.ends_with("aria-label") && (value.contains("NaN") || value.contains("inf")) {
                return false;
            }
        }
    }
    true
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn layouts_have_no_overlaps_and_fit_the_canvas(graph in graph_strategy()) {
        assert_invariants(&graph, &layout(&graph))?;
    }

    #[test]
    fn rendering_is_well_formed_and_deterministic(graph in graph_strategy()) {
        let svg = render_svg(&graph, Style::Standalone).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert!(well_formed(&svg), "{}", svg);
        prop_assert!(numbers_finite(&svg), "{}", svg);
        prop_assert!(!svg.contains('\n'), "rustdoc embedding needs a single line");
        let again = render_svg(&graph, Style::Standalone).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(svg, again);
    }
}

#[test]
fn tree_has_no_crossings_and_flows_downward() {
    let mut graph = Graph::new("Tree");
    let root = graph.add_node(Node::initial());
    let mut frontier = vec![root];
    for depth in 0..3 {
        let mut next = Vec::new();
        for &parent in &frontier {
            for side in ["L", "R"] {
                let child = graph.add_node(Node::state(format!("{side}{depth}")));
                graph.add_edge(parent, child, side);
                next.push(child);
            }
        }
        frontier = next;
    }
    let placed = layout(&graph);
    for edge in &graph.edges {
        let (from, to) = (&placed.nodes[edge.from.0], &placed.nodes[edge.to.0]);
        assert!(from.center.y < to.center.y, "tree edges point down");
    }
    let mut segments = Vec::new();
    for edge in &placed.edges {
        let EdgePath::Polyline(points) = &edge.path else {
            unreachable!("tree has no self-loops");
        };
        segments.push((points[0], points[points.len() - 1]));
    }
    for (i, a) in segments.iter().enumerate() {
        for b in &segments[i + 1..] {
            let same_band = (a.0.y - b.0.y).abs() < EPS;
            if same_band {
                let order_top = a.0.x < b.0.x;
                let order_bottom = a.1.x < b.1.x;
                assert_eq!(order_top, order_bottom, "edges cross");
            }
        }
    }
}

#[test]
fn back_edge_points_up_and_pseudo_states_are_circles() {
    let mut graph = Graph::new("Cycle");
    let start = graph.add_node(Node::initial());
    let a = graph.add_node(Node::state("A"));
    let b = graph.add_node(Node::state("B"));
    let end = graph.add_node(Node::terminal());
    graph.add_edge(start, a, "");
    graph.add_edge(a, b, "go");
    graph.add_edge(b, a, "back");
    graph.add_edge(b, end, "stop");
    let placed = layout(&graph);
    assert!(placed.nodes[a.0].center.y < placed.nodes[b.0].center.y);
    let EdgePath::Polyline(back) = &placed.edges[2].path else {
        unreachable!("back edge is a polyline");
    };
    assert!(
        back[0].y > back[back.len() - 1].y,
        "back edge drawn from B up to A"
    );
    assert_eq!(graph.nodes[end.0].kind, NodeKind::Final);
    assert!((placed.nodes[end.0].width - placed.nodes[end.0].height).abs() < EPS);
}

#[test]
fn invalid_edge_is_reported_with_context() {
    let mut other = Graph::new("Other");
    other.add_node(Node::state("X"));
    let foreign = other.add_node(Node::state("Y"));
    let mut graph = Graph::new("Main");
    let only = graph.add_node(Node::state("A"));
    graph.add_edge(only, foreign, "bad");
    let err = render_svg(&graph, Style::Inline)
        .err()
        .map(|e| e.to_string());
    assert_eq!(
        err.as_deref(),
        Some(
            "graph `Main`: edge #0 refers to node #1, which does not exist (was the NodeId taken from another graph?)"
        )
    );
}
