//! `diagram` feature: graph built from the transition table, nested composites, SVG.
#![cfg(feature = "diagram")]

use sml::diagram::{Graph, NodeKind};
use sml::{Diagram, sml};

struct Ctx;
struct Open;
struct Close;
struct Code;
struct Next;
struct Kick;

fn is_unlocked(_: &Ctx) -> bool {
    true
}
fn alarm(_: &Ctx) -> bool {
    false
}
fn log(_: &mut Ctx) {}

sml! {
    machine Door<Ctx>;
    *Closed + Open [is_unlocked && !(alarm || |_: &Ctx| false)] / log = Opened,
     Closed + Code / (log, |_: &mut Ctx| {}),
     Closed + on_entry / log,
     Closed + on_exit / (log, log),
     Opened + Close = Closed,
     Opened + Kick = X,
     Opened + unexpected_event<_> = Opened,
}

sml! {
    machine Inner<Ctx>;
    *A(H) + Next = B,
}

sml! {
    machine Outer<Ctx>;
    *Idle + Next = state<Inner>,
     Inner + Close = Idle,
}

fn edge_labels(graph: &Graph) -> Vec<(String, String, &str)> {
    let name = |id| {
        graph
            .node(id)
            .map_or_else(String::new, |node| match node.kind() {
                NodeKind::State => node.label().to_owned(),
                NodeKind::Initial => "*".to_owned(),
                NodeKind::History => "H".to_owned(),
                NodeKind::Final => "X".to_owned(),
            })
    };
    graph
        .edges()
        .iter()
        .map(|e| (name(e.from()), name(e.to()), e.label()))
        .collect()
}

#[test]
fn transitions_become_labelled_edges() {
    let graph = Door::graph();
    assert_eq!(graph.title(), "Door");
    let labels = edge_labels(&graph);
    let expected = [
        ("*", "Closed", ""),
        (
            "Closed",
            "Opened",
            "Open [is_unlocked && !(alarm || {closure})] / log",
        ),
        ("Opened", "Closed", "Close"),
        ("Opened", "X", "Kick"),
        ("Opened", "Opened", "unexpected_event<_>"),
    ];
    let expected: Vec<_> = expected
        .iter()
        .map(|(a, b, l)| ((*a).to_owned(), (*b).to_owned(), *l))
        .collect();
    assert_eq!(labels, expected);
}

#[test]
fn entry_exit_and_internal_transitions_are_listed_in_the_state() {
    let graph = Door::graph();
    let closed = graph.nodes().iter().find(|n| n.label() == "Closed");
    let details = closed.map(sml::diagram::Node::details);
    assert_eq!(
        details,
        Some(
            &[
                "entry / log".to_owned(),
                "exit / (log, log)".to_owned(),
                "Code / (log, {closure})".to_owned()
            ][..]
        )
    );
}

#[test]
fn composite_state_nests_the_sub_machine_with_history() {
    let graph = Outer::graph();
    let inner = graph.nodes().iter().find(|n| n.label() == "Inner");
    let child = inner.and_then(|n| n.child());
    assert_eq!(child.map(Graph::title), Some("Inner"));
    let child_labels = child.map(edge_labels);
    assert_eq!(
        child_labels,
        Some(vec![
            ("H".to_owned(), "A".to_owned(), ""),
            ("A".to_owned(), "B".to_owned(), "Next")
        ])
    );
}

#[test]
fn svg_contains_states_and_nested_states() {
    let svg = Outer::diagram_svg();
    assert!(svg.starts_with("<svg"), "{svg}");
    for text in [">Idle<", ">Inner<", ">A<", ">B<", ">Next<", ">H<"] {
        assert!(svg.contains(text), "missing {text}");
    }
    assert!(svg.contains("prefers-color-scheme:dark"));
}
