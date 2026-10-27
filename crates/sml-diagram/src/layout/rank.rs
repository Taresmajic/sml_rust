//! Longest-path layering on doubled ranks.
//!
//! Real nodes sit on even ranks; every edge spans at least two ranks, so the odd
//! rank right after its source is free for the edge's label.

use std::collections::VecDeque;

use crate::layout::Link;

/// Ranks per node; each link `from -> to` (or `to -> from` when reversed) goes down.
pub fn assign(node_count: usize, links: &[Link], reversed: &[bool]) -> Vec<usize> {
    let mut outgoing = vec![Vec::new(); node_count];
    let mut indegree = vec![0usize; node_count];
    for (link, &flip) in links.iter().zip(reversed) {
        let (from, to) = if flip {
            (link.to, link.from)
        } else {
            (link.from, link.to)
        };
        outgoing[from].push(to);
        indegree[to] += 1;
    }
    let mut ranks = vec![0; node_count];
    let mut remaining = indegree.clone();
    let mut queue: VecDeque<usize> = (0..node_count).filter(|&n| indegree[n] == 0).collect();
    while let Some(node) = queue.pop_front() {
        for &next in &outgoing[node] {
            ranks[next] = ranks[next].max(ranks[node] + 2);
            remaining[next] -= 1;
            if remaining[next] == 0 {
                queue.push_back(next);
            }
        }
    }
    for node in 0..node_count {
        let lowest_successor = outgoing[node].iter().map(|&next| ranks[next]).min();
        if indegree[node] == 0
            && let Some(lowest) = lowest_successor
        {
            ranks[node] = lowest - 2;
        }
    }
    ranks
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
    fn longest_path_with_doubled_ranks() {
        let links = [link(0, 1), link(1, 2), link(0, 2)];
        assert_eq!(assign(3, &links, &[false; 3]), [0, 2, 4]);
    }

    #[test]
    fn reversed_links_point_upwards() {
        let links = [link(0, 1), link(1, 0)];
        assert_eq!(assign(2, &links, &[false, true]), [0, 2]);
    }

    #[test]
    fn late_sources_are_pulled_down_next_to_their_successor() {
        let links = [link(0, 1), link(1, 2), link(3, 2)];
        assert_eq!(assign(4, &links, &[false; 3]), [0, 2, 4, 2]);
    }
}
