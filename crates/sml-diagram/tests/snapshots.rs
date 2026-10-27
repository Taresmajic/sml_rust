//! Golden SVGs for reference diagrams: any change to layout or drawing shows up here.
//!
//! After an intentional change, regenerate with `UPDATE_SNAPSHOTS=1 cargo test -p
//! sml-diagram --test snapshots` and inspect the files in `tests/snapshots/`.

use std::path::PathBuf;

use sml_diagram::{Graph, Node, Style, render_svg};

fn door() -> Graph {
    let mut graph = Graph::new("Door");
    let start = graph.add_node(Node::initial());
    let closed = graph.add_node(
        Node::state("Closed")
            .with_detail("entry / reset")
            .with_detail("Code / store_code"),
    );
    let opened = graph.add_node(Node::state("Opened").with_detail("Ping / log"));
    let alarmed = graph.add_node(Node::state("Alarmed"));
    let end = graph.add_node(Node::terminal());
    graph.add_edge(start, closed, "");
    graph.add_edge(closed, opened, "Open [is_unlocked && !alarm] / log_open");
    graph.add_edge(closed, alarmed, "Open [alarm]");
    graph.add_edge(closed, closed, "Code [code_is_42] / store_code");
    graph.add_edge(opened, closed, "Close / log_close");
    graph.add_edge(opened, end, "Kick");
    graph.add_edge(alarmed, end, "Kick");
    graph
}

fn composite() -> Graph {
    let mut inner = Graph::new("Inner");
    let history = inner.add_node(Node::history());
    let a = inner.add_node(Node::state("A").with_detail("entry / log"));
    let b = inner.add_node(Node::state("B"));
    let end = inner.add_node(Node::terminal());
    inner.add_edge(history, a, "");
    inner.add_edge(a, b, "Next");
    inner.add_edge(b, a, "Next");
    inner.add_edge(b, end, "Finish / log");

    let mut graph = Graph::new("Outer");
    let start = graph.add_node(Node::initial());
    let idle = graph.add_node(Node::state("Idle"));
    let sub = graph.add_node(
        Node::state("Inner")
            .with_detail("entry / log")
            .with_child(inner),
    );
    let done = graph.add_node(Node::state("Done"));
    graph.add_edge(start, idle, "");
    graph.add_edge(idle, sub, "Next");
    graph.add_edge(sub, idle, "Leave");
    graph.add_edge(sub, done, "[sub_done]");
    graph
}

fn tangled() -> Graph {
    let mut graph = Graph::new("Tangled & <Cyclic>");
    let start = graph.add_node(Node::initial());
    let states: Vec<_> = (0..7)
        .map(|i| graph.add_node(Node::state(format!("S{i}"))))
        .collect();
    graph.add_edge(start, states[0], "");
    let edges = [
        (0, 1, "e1"),
        (0, 2, "e2"),
        (1, 3, "e3"),
        (2, 3, "e4"),
        (3, 4, "e5"),
        (4, 0, "reset"),
        (2, 5, "e6"),
        (5, 6, "e7"),
        (6, 1, "back"),
        (1, 6, "jump"),
        (4, 4, "tick"),
        (4, 4, "tock"),
        (6, 6, ""),
    ];
    for (from, to, label) in edges {
        graph.add_edge(states[from], states[to], label);
    }
    graph
}

fn check(name: &str, graph: &Graph, style: Style) {
    let actual = render_svg(graph, style).map_err(|err| err.to_string());
    assert!(actual.is_ok(), "{name}: {actual:?}");
    let actual = actual.unwrap_or_default();
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "snapshots",
        &format!("{name}.svg"),
    ]
    .iter()
    .collect();
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        let written = std::fs::write(&path, &actual);
        assert!(written.is_ok(), "writing {}: {written:?}", path.display());
        return;
    }
    let expected = std::fs::read_to_string(&path);
    assert!(
        expected.is_ok(),
        "missing snapshot {}; create it with UPDATE_SNAPSHOTS=1",
        path.display()
    );
    assert!(
        expected.ok().as_deref() == Some(actual.as_str()),
        "{name} changed; inspect, then re-run with UPDATE_SNAPSHOTS=1 to accept"
    );
}

#[test]
fn door_standalone() {
    check("door", &door(), Style::Standalone);
}

#[test]
fn door_inline() {
    check("door_inline", &door(), Style::Inline);
}

#[test]
fn composite_standalone() {
    check("composite", &composite(), Style::Standalone);
}

#[test]
fn tangled_standalone() {
    check("tangled", &tangled(), Style::Standalone);
}
