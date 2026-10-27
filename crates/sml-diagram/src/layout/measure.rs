//! Node sizes, including nested graphs.

use crate::graph::{Node, NodeKind};
use crate::layout::{Layout, Point, layout};
use crate::text::{self, LINE_HEIGHT};

/// Space between a state's border and its text.
pub const PAD_X: f64 = 12.0;
pub const PAD_Y: f64 = 6.0;
/// Space between the title, detail lines and nested graph.
pub const SECTION_GAP: f64 = 6.0;
const MIN_STATE_WIDTH: f64 = 64.0;

/// A nested graph's layout and its top-left corner relative to the node's top-left.
#[derive(Clone, Debug)]
pub struct ChildLayout {
    pub offset: Point,
    pub layout: Box<Layout>,
}

#[derive(Clone, Debug)]
pub struct Measured {
    pub width: f64,
    pub height: f64,
    pub child: Option<ChildLayout>,
}

pub fn measure(node: &Node) -> Measured {
    let diameter = match node.kind {
        NodeKind::State => return measure_state(node),
        NodeKind::Initial => 14.0,
        NodeKind::History => 24.0,
        NodeKind::Final => 22.0,
    };
    Measured {
        width: diameter,
        height: diameter,
        child: None,
    }
}

fn measure_state(node: &Node) -> Measured {
    let mut text_width = text::width(&node.label);
    for line in &node.details {
        text_width = text_width.max(text::width(line));
    }
    let mut width = (text_width + 2.0 * PAD_X).max(MIN_STATE_WIDTH);
    let mut height = PAD_Y + LINE_HEIGHT;
    if !node.details.is_empty() {
        height += SECTION_GAP + text::to_f64(node.details.len()) * LINE_HEIGHT;
    }
    let Some(graph) = &node.child else {
        return Measured {
            width,
            height: height + PAD_Y,
            child: None,
        };
    };
    let nested = layout(graph);
    height += SECTION_GAP;
    width = width.max(nested.width);
    let offset = Point {
        x: (width - nested.width) / 2.0,
        y: height,
    };
    height += nested.height;
    let child = Some(ChildLayout {
        offset,
        layout: Box::new(nested),
    });
    Measured {
        width,
        height,
        child,
    }
}

#[cfg(test)]
mod tests {
    use super::measure;
    use crate::graph::{Graph, Node};

    #[test]
    fn narrow_child_is_centered_in_a_wider_composite() {
        let mut child = Graph::new("Child");
        child.add_node(Node::state("A"));
        let node = Node::state("A composite state with a very long name").with_child(child);
        let measured = measure(&node);
        let Some(placed) = &measured.child else {
            unreachable!("composite has a child layout");
        };
        let slack = measured.width - placed.layout.width;
        assert!(slack > 0.0, "label is wider than the child");
        assert!(
            (placed.offset.x - slack / 2.0).abs() < 1e-9,
            "child centered"
        );
    }

    #[test]
    fn pseudo_states_are_square_and_states_fit_their_text() {
        for node in [Node::initial(), Node::history(), Node::terminal()] {
            let size = measure(&node);
            assert!((size.width - size.height).abs() < 1e-9 && size.width > 0.0);
        }
        let short = measure(&Node::state("A"));
        let long = measure(&Node::state("A much longer state name"));
        assert!(long.width > short.width);
        let detailed = measure(&Node::state("A").with_detail("entry / x"));
        assert!(detailed.height > short.height);
    }
}
