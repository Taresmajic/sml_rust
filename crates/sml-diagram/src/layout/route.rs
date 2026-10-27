//! Edge geometry: polylines through dummies, clipped to node outlines; self-loops.

use crate::graph::{Graph, NodeKind};
use crate::layout::measure::Measured;
use crate::layout::network::Network;
use crate::layout::{
    EdgePath, LABEL_GAP, LOOP_REACH, LOOP_STEP, Layout, Link, PlacedEdge, PlacedNode, Point,
};
use crate::text::{LINE_HEIGHT, to_f64};

/// Edges attach to state boxes at least this far from a side (the corner radius).
const CORNER_CLEARANCE: f64 = 8.0;
/// Fraction of a circle's half-width over which edges may attach (keeps them on the arc's
/// top/bottom rather than its sides).
const CIRCLE_SPAN: f64 = 0.6;
/// Vertical distance of a self-loop's ends from the node's middle.
const LOOP_SPREAD: f64 = 6.0;

/// A self-loop: graph edge index, node, label width.
pub type SelfLoop = (usize, usize, Option<f64>);

/// Space each node needs for its self-loops and their labels.
pub struct LoopSpace {
    /// Width needed right of the node.
    pub extent: Vec<f64>,
    /// Height of the node's stacked loop labels.
    pub height: Vec<f64>,
}

pub fn loop_space(node_count: usize, loops: &[SelfLoop]) -> LoopSpace {
    let mut count = vec![0usize; node_count];
    let mut widest_label = vec![None::<f64>; node_count];
    for &(_, node, label_width) in loops {
        count[node] += 1;
        if let Some(width) = label_width {
            widest_label[node] = Some(widest_label[node].map_or(width, |w| w.max(width)));
        }
    }
    let mut extent = vec![0.0; node_count];
    let mut height = vec![0.0; node_count];
    for node in 0..node_count {
        if count[node] == 0 {
            continue;
        }
        let label = widest_label[node].map_or(0.0, |width| LABEL_GAP + width);
        extent[node] = outermost_reach(count[node]) + label;
        height[node] = to_f64(count[node]) * LINE_HEIGHT;
    }
    LoopSpace { extent, height }
}

/// Reach of the outermost of `count` nested loops; labels start beyond it.
fn outermost_reach(count: usize) -> f64 {
    LOOP_REACH + to_f64(count.saturating_sub(1)) * LOOP_STEP
}

pub fn build(
    graph: &Graph,
    measured: Vec<Measured>,
    net: &Network,
    links: &[Link],
    reversed: &[bool],
    loops: &[SelfLoop],
) -> Layout {
    let nodes: Vec<PlacedNode> = measured
        .into_iter()
        .enumerate()
        .map(|(id, size)| PlacedNode {
            center: Point {
                x: net.x[id],
                y: net.y[id],
            },
            width: size.width,
            height: size.height,
            child: size.child,
        })
        .collect();
    let mut edges: Vec<Option<PlacedEdge>> = vec![None; graph.edges.len()];
    for (index, link) in links.iter().enumerate() {
        let chain = &net.chains[index];
        let mut inner = Vec::new();
        for &id in &chain[1..chain.len() - 1] {
            // Vertical through the whole row so the bend happens between rows,
            // never across a neighbouring label.
            let half = net.row_height[net.nodes[id].rank] / 2.0;
            let (x, y) = (net.x[id], net.y[id]);
            if half > 0.0 {
                inner.push(Point { x, y: y - half });
                inner.push(Point { x, y: y + half });
            } else {
                inner.push(Point { x, y });
            }
        }
        if reversed[index] {
            inner.reverse();
        }
        let label = net.labels[index].map(|id| Point {
            x: net.x[id] + LABEL_GAP,
            y: net.y[id],
        });
        let path = polyline(graph, &nodes, link, inner);
        edges[link.edge] = Some(PlacedEdge { path, label });
    }
    place_loops(&nodes, loops, &mut edges);
    Layout {
        width: net.width,
        height: net.height,
        nodes,
        edges: edges.into_iter().flatten().collect(),
    }
}

/// Nested loops on each node's right side; labels stacked around the node's middle.
fn place_loops(nodes: &[PlacedNode], loops: &[SelfLoop], edges: &mut [Option<PlacedEdge>]) {
    let mut total = vec![0usize; nodes.len()];
    for &(_, node, _) in loops {
        total[node] += 1;
    }
    let mut seen = vec![0usize; nodes.len()];
    for &(edge, node, label_width) in loops {
        let placed = &nodes[node];
        let nth = to_f64(seen[node]);
        seen[node] += 1;
        let side = placed.center.x + placed.width / 2.0;
        let (y, reach) = (placed.center.y, LOOP_REACH + nth * LOOP_STEP);
        let spread = LOOP_SPREAD * (1.0 + nth / 2.0);
        let from = Point {
            x: side,
            y: y - spread,
        };
        let to = Point {
            x: side,
            y: y + spread,
        };
        let stack_offset = (nth - (to_f64(total[node]) - 1.0) / 2.0) * LINE_HEIGHT;
        let label = label_width.map(|_| Point {
            x: side + outermost_reach(total[node]) + LABEL_GAP,
            y: y + stack_offset,
        });
        edges[edge] = Some(PlacedEdge {
            path: EdgePath::Loop { from, to, reach },
            label,
        });
    }
}

fn polyline(graph: &Graph, nodes: &[PlacedNode], link: &Link, inner: Vec<Point>) -> EdgePath {
    let (source, target) = (&nodes[link.from], &nodes[link.to]);
    let first = inner.first().copied().unwrap_or(target.center);
    let last = inner.last().copied().unwrap_or(source.center);
    let mut points = Vec::with_capacity(inner.len() + 2);
    points.push(attach(source, graph.nodes[link.from].kind, first));
    points.extend(inner);
    points.push(attach(target, graph.nodes[link.to].kind, last));
    EdgePath::Polyline(points)
}

/// Where an edge heading to/from `toward` meets `node`: on the top or bottom face (edges
/// run vertically at their ends), below/above `toward`, kept clear of rounded corners.
fn attach(node: &PlacedNode, kind: NodeKind, toward: Point) -> Point {
    let side = if toward.y < node.center.y { -1.0 } else { 1.0 };
    let half_width = node.width / 2.0;
    match kind {
        NodeKind::State => {
            let inset = CORNER_CLEARANCE.min(half_width);
            let reach = half_width - inset;
            Point {
                x: toward.x.clamp(node.center.x - reach, node.center.x + reach),
                y: node.center.y + side * node.height / 2.0,
            }
        }
        NodeKind::Initial | NodeKind::History | NodeKind::Final => {
            let offset = (toward.x - node.center.x).clamp(-half_width, half_width) * CIRCLE_SPAN;
            let rise = (half_width * half_width - offset * offset).sqrt();
            Point {
                x: node.center.x + offset,
                y: node.center.y + side * rise,
            }
        }
    }
}
