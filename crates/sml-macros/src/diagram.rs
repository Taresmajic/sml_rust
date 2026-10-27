//! Diagram of a transition table: an inline SVG for the machine's rustdoc and a
//! runtime `sml::Diagram` impl that nests sub-machine diagrams in composite states.
//!
//! UML conventions: entry/exit actions and internal transitions are listed inside the
//! state; `X` is a final state; a machine with history starts from an `H` pseudo-state.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use sml_diagram::{Graph, Node, Style, render_svg};
use syn::{Error, Expr, Result, Type};

use crate::ast::GuardExpr;
use crate::model::{Model, State, StateKind, Transition, TransitionTrigger};

/// Pseudo-state the diagram starts from.
#[derive(Clone, Copy)]
enum Start {
    Initial,
    History,
}

struct StateNode<'a> {
    label: String,
    details: Vec<String>,
    terminal: bool,
    composite: Option<&'a Type>,
}

/// Node 0 is the start pseudo-state; node `i + 1` is the model's state `i`.
struct Spec<'a> {
    title: String,
    start: Start,
    states: Vec<StateNode<'a>>,
    edges: Vec<(usize, usize, String)>,
}

/// Doc lines embedding the diagram (composite states shown by reference).
pub fn doc_lines(model: &Model) -> Result<Vec<String>> {
    let spec = Spec::of(model);
    let svg = render_svg(&spec.doc_graph(), Style::Inline).map_err(|err| {
        Error::new(
            model.name.span(),
            format!("sml: failed to render the diagram: {err}"),
        )
    })?;
    // A `<div>` line opens a raw HTML block, so markdown leaves the SVG text alone.
    Ok(vec![
        String::new(),
        r#"<div class="sml-diagram">"#.to_owned(),
        svg,
        "</div>".to_owned(),
    ])
}

/// `impl sml::Diagram`, building the graph at runtime with nested sub-machines.
pub fn diagram_impl(model: &Model) -> TokenStream {
    let spec = Spec::of(model);
    let name = &model.name;
    let title = &spec.title;
    let start = match spec.start {
        Start::Initial => quote! { ::sml::diagram::Node::initial() },
        Start::History => quote! { ::sml::diagram::Node::history() },
    };
    let states = spec.states.iter().map(|state| {
        let base = if state.terminal {
            quote! { ::sml::diagram::Node::terminal() }
        } else {
            let label = &state.label;
            quote! { ::sml::diagram::Node::state(#label) }
        };
        let details = &state.details;
        let child = state.composite.map(|ty| {
            quote! { .with_child(<#ty as ::sml::Diagram>::graph()) }
        });
        quote! { graph.add_node(#base #(.with_detail(#details))* #child) }
    });
    let edges = spec.edges.iter().map(|(from, to, label)| {
        quote! { graph.add_edge(nodes[#from], nodes[#to], #label); }
    });
    quote! {
        impl ::sml::Diagram for #name {
            fn graph() -> ::sml::diagram::Graph {
                let mut graph = ::sml::diagram::Graph::new(#title);
                let nodes = [graph.add_node(#start), #(#states),*];
                #(#edges)*
                graph
            }
        }
    }
}

impl<'a> Spec<'a> {
    fn of(model: &'a Model) -> Self {
        let mut states: Vec<StateNode<'a>> = model.states.iter().map(state_node).collect();
        let mut edges = vec![(0, model.initial + 1, String::new())];
        for transition in &model.transitions {
            let label = row_label(transition);
            match transition.target {
                Some(target) => edges.push((transition.source + 1, target + 1, label)),
                None => states[transition.source].details.push(label),
            }
        }
        let start = if model.history {
            Start::History
        } else {
            Start::Initial
        };
        Self {
            title: model.name.to_string(),
            start,
            states,
            edges,
        }
    }

    /// Graph for rustdoc: sub-machines are not visible here, so composites say `state<T>`.
    fn doc_graph(&self) -> Graph {
        let mut graph = Graph::new(&self.title);
        let mut nodes = vec![graph.add_node(match self.start {
            Start::Initial => Node::initial(),
            Start::History => Node::history(),
        })];
        for state in &self.states {
            let mut node = if state.terminal {
                Node::terminal()
            } else {
                Node::state(&state.label)
            };
            if let Some(ty) = state.composite {
                node = node.with_detail(format!(
                    "state<{}>",
                    compact(&ty.to_token_stream().to_string())
                ));
            }
            for detail in &state.details {
                node = node.with_detail(detail);
            }
            nodes.push(graph.add_node(node));
        }
        for (from, to, label) in &self.edges {
            graph.add_edge(nodes[*from], nodes[*to], label);
        }
        graph
    }
}

fn state_node(state: &State) -> StateNode<'_> {
    let mut details = Vec::new();
    if let Some(actions) = actions_label(&state.entry) {
        details.push(format!("entry {actions}"));
    }
    if let Some(actions) = actions_label(&state.exit) {
        details.push(format!("exit {actions}"));
    }
    let terminal = match state.kind {
        StateKind::Terminate => true,
        StateKind::Simple | StateKind::Composite(_) => false,
    };
    StateNode {
        label: state.name.to_string(),
        details,
        terminal,
        composite: state.composite(),
    }
}

/// `Event [guard] / action`, omitting absent parts.
fn row_label(transition: &Transition) -> String {
    let mut parts = Vec::new();
    match &transition.trigger {
        TransitionTrigger::Anonymous => {}
        TransitionTrigger::Event(ty) => parts.push(type_label(ty)),
        TransitionTrigger::Unexpected(None) => parts.push("unexpected_event<_>".to_owned()),
        TransitionTrigger::Unexpected(Some(ty)) => {
            parts.push(format!("unexpected_event<{}>", type_label(ty)));
        }
    }
    if let Some(guard) = &transition.guard {
        parts.push(format!("[{}]", guard_label(guard)));
    }
    if let Some(actions) = actions_label(&transition.actions) {
        parts.push(actions);
    }
    parts.join(" ")
}

fn actions_label(actions: &[Expr]) -> Option<String> {
    match actions {
        [] => None,
        [single] => Some(format!("/ {}", expr_label(single))),
        many => {
            let names: Vec<String> = many.iter().map(expr_label).collect();
            Some(format!("/ ({})", names.join(", ")))
        }
    }
}

fn guard_label(guard: &GuardExpr) -> String {
    match guard {
        GuardExpr::Leaf(expr) => expr_label(expr),
        GuardExpr::Not(inner) => match inner.as_ref() {
            GuardExpr::Leaf(_) | GuardExpr::Not(_) => format!("!{}", guard_label(inner)),
            GuardExpr::And(..) | GuardExpr::Or(..) => format!("!({})", guard_label(inner)),
        },
        GuardExpr::And(left, right) => {
            format!("{} && {}", and_operand(left), and_operand(right))
        }
        GuardExpr::Or(left, right) => format!("{} || {}", guard_label(left), guard_label(right)),
    }
}

/// `||` binds weaker than `&&`, so it needs parentheses inside a conjunction.
fn and_operand(guard: &GuardExpr) -> String {
    match guard {
        GuardExpr::Or(..) => format!("({})", guard_label(guard)),
        GuardExpr::Leaf(_) | GuardExpr::Not(_) | GuardExpr::And(..) => guard_label(guard),
    }
}

/// Paths as written; closures as `{closure}` (their bodies would swamp the diagram).
fn expr_label(expr: &Expr) -> String {
    match expr {
        Expr::Closure(_) => "{closure}".to_owned(),
        other => compact(&other.to_token_stream().to_string()),
    }
}

fn type_label(ty: &Type) -> String {
    compact(&ty.to_token_stream().to_string())
}

/// Drops the spaces the token printer puts around punctuation: `a :: B < u8 >` -> `a::B<u8>`.
fn compact(printed: &str) -> String {
    let chars: Vec<char> = printed.chars().collect();
    let mut out = String::with_capacity(printed.len());
    for (index, &ch) in chars.iter().enumerate() {
        if ch == ' ' {
            let before = index.checked_sub(1).map(|i| chars[i]);
            let after = chars.get(index + 1).copied();
            let word =
                |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '\'');
            if !(word(before) && word(after)) {
                continue;
            }
        }
        out.push(ch);
        if ch == ',' {
            out.push(' ');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Spec, compact, doc_lines, guard_label};
    use crate::ast::GuardExpr;

    fn leaf(name: &str) -> GuardExpr {
        GuardExpr::Leaf(syn::parse_str(name).unwrap_or_else(|_| syn::parse_quote!(invalid)))
    }

    #[test]
    fn compact_removes_printer_spaces_around_punctuation() {
        assert_eq!(compact("crate :: events :: Open"), "crate::events::Open");
        assert_eq!(compact("Wrapper < u8 , String >"), "Wrapper<u8, String>");
        assert_eq!(compact("& 'static str"), "&'static str");
        assert_eq!(compact("dyn Any"), "dyn Any");
        assert_eq!(compact("dyn _Private"), "dyn _Private");
    }

    fn model(table: &str) -> crate::model::Model {
        let def = syn::parse_str::<crate::ast::MachineDef>(table);
        let built = def.and_then(crate::model::build);
        assert!(built.is_ok(), "table must be valid");
        built.unwrap_or_else(|_| unreachable!())
    }

    #[test]
    fn doc_lines_embed_the_svg_in_a_raw_html_block() {
        let lines = doc_lines(&model(
            "machine Door; *Closed + Open = Opened, Opened + Close = Closed,",
        ));
        let lines = lines.unwrap_or_default();
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], "", "blank line ends the summary paragraph");
        assert_eq!(lines[1], r#"<div class="sml-diagram">"#);
        assert!(
            lines[2].starts_with("<svg") && lines[2].ends_with("</svg>"),
            "{}",
            lines[2]
        );
        for text in [">Closed<", ">Opened<", ">Open<", ">Close<"] {
            assert!(lines[2].contains(text), "missing {text}");
        }
        assert!(
            !lines[2].contains("prefers-color-scheme"),
            "inline style inherits the theme"
        );
        assert_eq!(lines[3], "</div>");
    }

    #[test]
    fn doc_graph_shows_composites_by_reference_and_lists_details() {
        let model = model(
            "machine Outer; *Idle(H) + Go = state<inner::Sub>, Idle + on_entry / reset, \
             Sub + Ping / log, Sub + Stop = X,",
        );
        let graph = Spec::of(&model).doc_graph();
        let labels: Vec<&str> = graph.nodes().iter().map(sml_diagram::Node::label).collect();
        assert_eq!(labels, ["", "Idle", "Sub", ""]);
        assert_eq!(graph.nodes()[0].kind(), sml_diagram::NodeKind::History);
        assert_eq!(graph.nodes()[3].kind(), sml_diagram::NodeKind::Final);
        assert_eq!(graph.nodes()[1].details(), ["entry / reset"]);
        assert_eq!(
            graph.nodes()[2].details(),
            ["state<inner::Sub>", "Ping / log"]
        );
        let edges: Vec<&str> = graph.edges().iter().map(sml_diagram::Edge::label).collect();
        assert_eq!(edges, ["", "Go", "Stop"]);
    }

    #[test]
    fn guard_labels_keep_precedence() {
        let or = GuardExpr::Or(Box::new(leaf("a")), Box::new(leaf("b")));
        let and = GuardExpr::And(Box::new(or), Box::new(GuardExpr::Not(Box::new(leaf("c")))));
        assert_eq!(guard_label(&and), "(a || b) && !c");
        let not_and = GuardExpr::Not(Box::new(GuardExpr::And(
            Box::new(leaf("a")),
            Box::new(leaf("b")),
        )));
        assert_eq!(guard_label(&not_and), "!(a && b)");
    }
}
