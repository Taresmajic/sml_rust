//! Cycle breaking by depth-first search.

use crate::layout::Link;

pub struct Search {
    /// Per link: reversed for layering because it closes a cycle.
    pub reversed: Vec<bool>,
    /// Per node: DFS discovery index; used as the initial in-layer order.
    pub discovery: Vec<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mark {
    Unvisited,
    OnStack,
    Done,
}

/// DFS from `roots` in order; links into a node still on the stack are back edges.
pub fn search(node_count: usize, links: &[Link], roots: &[usize]) -> Search {
    let mut outgoing = vec![Vec::new(); node_count];
    for (index, link) in links.iter().enumerate() {
        outgoing[link.from].push(index);
    }
    let mut marks = vec![Mark::Unvisited; node_count];
    let mut reversed = vec![false; links.len()];
    let mut discovery = vec![0; node_count];
    let mut discovered = 0;
    for &root in roots {
        if marks[root] != Mark::Unvisited {
            continue;
        }
        let mut stack = vec![(root, 0)];
        marks[root] = Mark::OnStack;
        discovery[root] = discovered;
        discovered += 1;
        while let Some((node, next)) = stack.last_mut() {
            let Some(&link) = outgoing[*node].get(*next) else {
                marks[*node] = Mark::Done;
                stack.pop();
                continue;
            };
            *next += 1;
            let to = links[link].to;
            match marks[to] {
                Mark::Unvisited => {
                    marks[to] = Mark::OnStack;
                    discovery[to] = discovered;
                    discovered += 1;
                    stack.push((to, 0));
                }
                Mark::OnStack => reversed[link] = true,
                Mark::Done => {}
            }
        }
    }
    Search {
        reversed,
        discovery,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(from: usize, to: usize) -> Link {
        Link {
            edge: 0,
            from,
            to,
            label_width: None,
        }
    }

    #[test]
    fn reverses_exactly_the_back_edge_of_a_cycle() {
        let links = [link(0, 1), link(1, 2), link(2, 0), link(0, 2)];
        let search = search(3, &links, &[0, 1, 2]);
        assert_eq!(search.reversed, [false, false, true, false]);
        assert_eq!(search.discovery, [0, 1, 2]);
    }

    #[test]
    fn unreachable_nodes_are_searched_from_later_roots() {
        let links = [link(1, 0)];
        let search = search(2, &links, &[0, 1]);
        assert_eq!(search.reversed, [false]);
        assert_eq!(search.discovery, [0, 1]);
    }
}
