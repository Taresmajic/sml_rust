//! SVG output. Everything is drawn in `currentColor`, so the diagram follows the
//! surrounding text colour (e.g. the rustdoc theme) unless a standalone palette is set.
#![expect(
    clippy::format_push_string,
    reason = "documents are a few KB; `write!` would add impossible fmt::Error paths"
)]

use crate::graph::{Graph, Node, NodeKind};
use crate::layout::measure::{PAD_X, PAD_Y, SECTION_GAP};
use crate::layout::{EdgePath, Layout, PlacedEdge, PlacedNode, Point};
use crate::text::{BASELINE_SHIFT, FONT_FAMILY, FONT_SIZE, LINE_HEIGHT, escape, to_f64};

const STROKE: &str = r#"stroke="currentColor" stroke-width="1.3""#;
const STANDALONE_STYLE: &str = "<style>svg{color:#1f2328;background:#ffffff}\
     @media (prefers-color-scheme:dark){svg{color:#e6edf3;background:#0d1117}}</style>";

/// Colour handling of the produced SVG.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// Own light/dark palette and background; for `.svg` files and `<img>` tags.
    Standalone,
    /// Inherits colour from the embedding page; for inline HTML such as rustdoc.
    Inline,
}

pub fn document(graph: &Graph, layout: &Layout, style: Style) -> String {
    let id = element_id(&graph.title);
    let (width, height) = (num(layout.width), num(layout.height));
    let title = escape(&graph.title);
    let mut out = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" class="sml-diagram" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-label="{title} state machine" font-family="{FONT_FAMILY}" font-size="{FONT_SIZE}" style="max-width:100%;height:auto">"#,
    );
    out.push_str(&format!("<title>{title}</title>"));
    if style == Style::Standalone {
        out.push_str(STANDALONE_STYLE);
    }
    out.push_str(&format!(
        r#"<defs><marker id="{id}-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0 0L10 5L0 10z" fill="currentColor"/></marker></defs>"#,
    ));
    draw(&mut out, graph, layout, &id);
    out.push_str("</svg>");
    out
}

/// `sml-` plus the title with every character outside `[A-Za-z0-9_-]` replaced.
fn element_id(title: &str) -> String {
    let mut id = String::from("sml-");
    for ch in title.chars() {
        id.push(if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            ch
        } else {
            '-'
        });
    }
    id
}

/// Formats a coordinate with at most one decimal.
fn num(value: f64) -> String {
    let rounded = (value * 10.0).round() / 10.0;
    if rounded.fract().abs() < f64::EPSILON {
        format!("{rounded:.0}")
    } else {
        format!("{rounded:.1}")
    }
}

fn draw(out: &mut String, graph: &Graph, layout: &Layout, id: &str) {
    for (edge, placed) in graph.edges.iter().zip(&layout.edges) {
        draw_edge(out, &edge.label, placed, id);
    }
    for (node, placed) in graph.nodes.iter().zip(&layout.nodes) {
        draw_node(out, node, placed, id);
    }
}

fn draw_edge(out: &mut String, label: &str, placed: &PlacedEdge, id: &str) {
    let path = match &placed.path {
        EdgePath::Polyline(points) => smooth(points),
        EdgePath::Loop { from, to, reach } => {
            let control = from.x + reach * 4.0 / 3.0;
            let lift = reach * 0.75;
            format!(
                "M{} {}C{} {} {} {} {} {}",
                num(from.x),
                num(from.y),
                num(control),
                num(from.y - lift),
                num(control),
                num(to.y + lift),
                num(to.x),
                num(to.y)
            )
        }
    };
    out.push_str(&format!(
        r#"<path d="{path}" fill="none" {STROKE} marker-end="url(#{id}-arrow)"/>"#
    ));
    if let Some(at) = placed.label {
        text(out, at.x, at.y, "start", "", label);
    }
}

/// Path through `points` with vertical tangents at every point: each segment is a
/// cubic whose control points sit straight below/above its ends. Such a curve stays
/// inside the bounding box of its points, so it cannot overshoot into labels or past
/// the canvas edge, and it reads naturally in a top-to-bottom layout.
fn smooth(points: &[Point]) -> String {
    let Some(first) = points.first() else {
        return String::new();
    };
    let mut d = format!("M{} {}", num(first.x), num(first.y));
    for pair in points.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        let middle = f64::midpoint(from.y, to.y);
        d.push_str(&format!(
            "C{} {} {} {} {} {}",
            num(from.x),
            num(middle),
            num(to.x),
            num(middle),
            num(to.x),
            num(to.y)
        ));
    }
    d
}

fn text(out: &mut String, x: f64, y: f64, anchor: &str, extra: &str, content: &str) {
    out.push_str(&format!(
        r#"<text x="{}" y="{}" text-anchor="{anchor}" fill="currentColor"{extra}>{}</text>"#,
        num(x),
        num(y + BASELINE_SHIFT),
        escape(content)
    ));
}

fn circle(out: &mut String, center: Point, radius: f64, paint: &str) {
    out.push_str(&format!(
        r#"<circle cx="{}" cy="{}" r="{}" {paint}/>"#,
        num(center.x),
        num(center.y),
        num(radius)
    ));
}

fn draw_node(out: &mut String, node: &Node, placed: &PlacedNode, id: &str) {
    let center = placed.center;
    let radius = placed.width / 2.0;
    match node.kind {
        NodeKind::State => draw_state(out, node, placed, id),
        NodeKind::Initial => circle(out, center, radius, r#"fill="currentColor""#),
        NodeKind::History => {
            circle(out, center, radius, &format!(r#"fill="none" {STROKE}"#));
            text(
                out,
                center.x,
                center.y,
                "middle",
                r#" font-weight="600""#,
                "H",
            );
        }
        NodeKind::Final => {
            circle(out, center, radius, &format!(r#"fill="none" {STROKE}"#));
            circle(out, center, radius * 0.55, r#"fill="currentColor""#);
        }
    }
}

fn draw_state(out: &mut String, node: &Node, placed: &PlacedNode, id: &str) {
    let (left, top) = (
        placed.center.x - placed.width / 2.0,
        placed.center.y - placed.height / 2.0,
    );
    out.push_str(&format!(
        r#"<rect x="{}" y="{}" width="{}" height="{}" rx="8" fill="currentColor" fill-opacity="0.07" {STROKE}/>"#,
        num(left),
        num(top),
        num(placed.width),
        num(placed.height)
    ));
    let title_middle = top + PAD_Y + LINE_HEIGHT / 2.0;
    text(
        out,
        placed.center.x,
        title_middle,
        "middle",
        r#" font-weight="600""#,
        &node.label,
    );
    let mut cursor = top + PAD_Y + LINE_HEIGHT;
    if !node.details.is_empty() {
        separator(out, left, placed.width, cursor + SECTION_GAP / 2.0);
        cursor += SECTION_GAP;
        for (index, line) in node.details.iter().enumerate() {
            let middle = cursor + (to_f64(index) + 0.5) * LINE_HEIGHT;
            text(out, left + PAD_X, middle, "start", "", line);
        }
    }
    if let (Some(graph), Some(child)) = (&node.child, &placed.child) {
        separator(
            out,
            left,
            placed.width,
            top + child.offset.y - SECTION_GAP / 2.0,
        );
        let (x, y) = (left + child.offset.x, top + child.offset.y);
        out.push_str(&format!(
            r#"<g transform="translate({} {})">"#,
            num(x),
            num(y)
        ));
        draw(out, graph, &child.layout, id);
        out.push_str("</g>");
    }
}

fn separator(out: &mut String, left: f64, width: f64, y: f64) {
    out.push_str(&format!(
        r#"<line x1="{}" y1="{y}" x2="{}" y2="{y}" stroke="currentColor" stroke-opacity="0.35"/>"#,
        num(left),
        num(left + width),
        y = num(y)
    ));
}

#[cfg(test)]
mod tests {
    use super::{element_id, num, smooth};
    use crate::layout::Point;

    #[test]
    fn numbers_have_at_most_one_decimal() {
        assert_eq!(num(12.0), "12");
        assert_eq!(num(12.34), "12.3");
        assert_eq!(num(12.36), "12.4");
        assert_eq!(num(-0.25), "-0.3", "halves round away from zero");
        assert_eq!(num(0.04), "0");
        assert_eq!(num(100.96), "101");
    }

    #[test]
    fn nested_graph_is_translated_to_its_centered_offset() {
        let mut child = crate::Graph::new("Child");
        child.add_node(crate::Node::state("A"));
        let mut graph = crate::Graph::new("Parent");
        graph.add_node(
            crate::Node::state("A composite state with a very long name").with_child(child),
        );
        let layout = crate::layout::layout(&graph);
        let (placed, offset) = (
            &layout.nodes[0],
            layout.nodes[0].child.as_ref().map(|c| c.offset),
        );
        let svg = super::document(&graph, &layout, super::Style::Inline);
        let left = placed.center.x - placed.width / 2.0;
        let top = placed.center.y - placed.height / 2.0;
        let offset = offset.unwrap_or(crate::layout::Point {
            x: f64::NAN,
            y: f64::NAN,
        });
        assert!(offset.x > 0.0, "the child is narrower than the composite");
        let expected = format!(
            "translate({} {})",
            num(left + offset.x),
            num(top + offset.y)
        );
        assert!(svg.contains(&expected), "{expected} not in {svg}");
    }

    #[test]
    fn element_ids_keep_only_safe_characters() {
        assert_eq!(element_id("Door_2-a"), "sml-Door_2-a");
        assert_eq!(element_id("a b<c>"), "sml-a-b-c-");
        assert_eq!(element_id(""), "sml-");
    }

    #[test]
    fn paths_use_vertical_tangents_between_points() {
        let points = [
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 20.0 },
            Point { x: 10.0, y: 30.0 },
        ];
        assert_eq!(smooth(&points), "M0 0C0 10 10 10 10 20C10 25 10 25 10 30");
        assert_eq!(smooth(&points[..1]), "M0 0");
        assert_eq!(smooth(&[]), "");
    }
}
