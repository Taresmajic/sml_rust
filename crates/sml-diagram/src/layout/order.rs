//! In-layer ordering: barycenter sweeps plus adjacent transpositions.

use crate::layout::network::Network;
use crate::text::to_f64;

const SWEEPS: usize = 16;
const TRANSPOSE_PASSES: usize = 4;

pub fn minimize_crossings(net: &mut Network, discovery: &[usize]) {
    initial_order(net, discovery);
    let mut best = net.layers.clone();
    let mut best_crossings = crossings(net);
    for sweep in 0..SWEEPS {
        if best_crossings == 0 {
            break;
        }
        let downward = sweep % 2 == 0;
        let layer_count = net.layers.len();
        for step in 0..layer_count {
            let layer = if downward {
                step
            } else {
                layer_count - 1 - step
            };
            sort_layer(net, layer, downward);
        }
        transpose(net);
        let current = crossings(net);
        if current < best_crossings {
            best_crossings = current;
            best.clone_from(&net.layers);
        }
    }
    net.layers = best;
}

/// DFS discovery order: chains follow their origin node.
fn initial_order(net: &mut Network, discovery: &[usize]) {
    for layer in &mut net.layers {
        layer.sort_by_key(|&id| {
            let node = &net.nodes[id];
            (discovery[node.origin], node.dummy, id)
        });
    }
}

fn positions(net: &Network) -> Vec<usize> {
    let mut position = vec![0; net.nodes.len()];
    for layer in &net.layers {
        for (index, &id) in layer.iter().enumerate() {
            position[id] = index;
        }
    }
    position
}

/// Orders `layer` by the mean position of its neighbours in the layer above
/// (`downward`) or below; nodes without such neighbours keep their position.
fn sort_layer(net: &mut Network, layer: usize, downward: bool) {
    let position = positions(net);
    let neighbours = if downward { &net.up } else { &net.down };
    let mut keyed: Vec<(f64, usize)> = net.layers[layer]
        .iter()
        .map(|&id| {
            let adjacent = &neighbours[id];
            let key = if adjacent.is_empty() {
                to_f64(position[id])
            } else {
                let sum: usize = adjacent.iter().map(|&n| position[n]).sum();
                to_f64(sum) / to_f64(adjacent.len())
            };
            (key, id)
        })
        .collect();
    keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
    net.layers[layer] = keyed.into_iter().map(|(_, id)| id).collect();
}

/// Swaps adjacent nodes while that reduces crossings with both neighbouring layers.
fn transpose(net: &mut Network) {
    for _ in 0..TRANSPOSE_PASSES {
        let mut improved = false;
        for layer in 0..net.layers.len() {
            let mut position = positions(net);
            for index in 1..net.layers[layer].len() {
                let (a, b) = (net.layers[layer][index - 1], net.layers[layer][index]);
                if pair_crossings(net, &position, a, b) > pair_crossings(net, &position, b, a) {
                    net.layers[layer].swap(index - 1, index);
                    position.swap(a, b);
                    improved = true;
                }
            }
        }
        if !improved {
            return;
        }
    }
}

/// Crossings between edges of `left` and `right` when `left` is placed first.
fn pair_crossings(net: &Network, position: &[usize], left: usize, right: usize) -> usize {
    let mut count = 0;
    for neighbours in [&net.up, &net.down] {
        for &l in &neighbours[left] {
            for &r in &neighbours[right] {
                if position[l] > position[r] {
                    count += 1;
                }
            }
        }
    }
    count
}

pub fn crossings(net: &Network) -> usize {
    let position = positions(net);
    let mut count = 0;
    for layer in &net.layers {
        let mut segments = Vec::new();
        for &id in layer {
            for &below in &net.down[id] {
                segments.push((position[id], position[below]));
            }
        }
        // Collected in layer order, so `a` never starts right of a later `b`.
        for (i, a) in segments.iter().enumerate() {
            for b in &segments[i + 1..] {
                if a.0 < b.0 && a.1 > b.1 {
                    count += 1;
                }
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::{
        crossings, initial_order, minimize_crossings, pair_crossings, positions, transpose,
    };
    use crate::layout::network::{Network, VNode};

    /// Real nodes in `layers` (ids in order), connected by `edges` between adjacent layers.
    fn network(layers: &[&[usize]], edges: &[(usize, usize)]) -> Network {
        let count = layers.iter().map(|layer| layer.len()).sum();
        let mut nodes = Vec::new();
        for (rank, layer) in layers.iter().enumerate() {
            for _ in *layer {
                nodes.push(VNode {
                    left: 1.0,
                    right: 1.0,
                    height: 0.0,
                    rank,
                    weight: 1.0,
                    origin: 0,
                    dummy: false,
                });
            }
        }
        let (mut up, mut down) = (vec![Vec::new(); count], vec![Vec::new(); count]);
        for &(from, to) in edges {
            down[from].push(to);
            up[to].push(from);
        }
        Network {
            nodes,
            layers: layers.iter().map(|layer| layer.to_vec()).collect(),
            up,
            down,
            chains: Vec::new(),
            labels: Vec::new(),
            x: Vec::new(),
            y: Vec::new(),
            row_height: Vec::new(),
            width: 0.0,
            height: 0.0,
        }
    }

    /// `[0, 1]` over `[2, 3]` with 0->3 and 1->2: one crossing.
    fn crossed() -> Network {
        network(&[&[0, 1], &[2, 3]], &[(0, 3), (1, 2)])
    }

    #[test]
    fn counts_crossings_between_adjacent_layers() {
        let mut net = crossed();
        assert_eq!(crossings(&net), 1);
        net.layers[1] = vec![3, 2];
        assert_eq!(crossings(&net), 0);
    }

    #[test]
    fn shared_endpoints_are_not_crossings() {
        let net = network(&[&[0, 1], &[2, 3]], &[(0, 2), (0, 3), (1, 3)]);
        assert_eq!(crossings(&net), 0);
    }

    #[test]
    fn every_crossing_pair_counts_once() {
        let net = network(&[&[0, 1, 2], &[3, 4, 5]], &[(0, 5), (1, 4), (2, 3)]);
        assert_eq!(crossings(&net), 3);
    }

    #[test]
    fn crossings_sum_over_all_layer_pairs() {
        let net = network(
            &[&[0, 1], &[2, 3], &[4, 5]],
            &[(0, 3), (1, 2), (2, 5), (3, 4)],
        );
        assert_eq!(crossings(&net), 2);
    }

    #[test]
    fn pair_crossings_compare_neighbour_positions() {
        let net = crossed();
        let position = positions(&net);
        assert_eq!(pair_crossings(&net, &position, 2, 3), 1);
        assert_eq!(pair_crossings(&net, &position, 3, 2), 0);
        assert_eq!(pair_crossings(&net, &position, 0, 1), 1);
        let fan_in = network(&[&[0, 1], &[2]], &[(0, 2), (1, 2)]);
        let position = positions(&fan_in);
        assert_eq!(
            pair_crossings(&fan_in, &position, 0, 1),
            0,
            "shared neighbour"
        );
    }

    #[test]
    fn transpose_swaps_neighbours_that_cross() {
        let mut net = crossed();
        transpose(&mut net);
        assert_eq!(net.layers, [vec![1, 0], vec![2, 3]]);
        assert_eq!(crossings(&net), 0);
    }

    #[test]
    fn minimize_crossings_untangles() {
        let mut net = crossed();
        minimize_crossings(&mut net, &[0]);
        assert_eq!(crossings(&net), 0);
    }

    #[test]
    fn initial_order_follows_discovery_of_each_origin() {
        let mut net = crossed();
        net.nodes[1].origin = 1;
        net.nodes[3].origin = 1;
        initial_order(&mut net, &[1, 0]);
        assert_eq!(net.layers, [vec![1, 0], vec![3, 2]]);
    }
}
