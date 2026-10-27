//! Coordinates: x by repeated order-preserving least squares, y by rank rows.

use crate::layout::MARGIN;
use crate::layout::network::Network;

const ROUNDS: usize = 24;
const GAP_NODES: f64 = 28.0;
const GAP_MIXED: f64 = 14.0;
const GAP_DUMMIES: f64 = 6.0;
const RANK_GAP: f64 = 18.0;

pub fn assign(net: &mut Network) {
    assign_x(net);
    assign_y(net);
}

fn separation(net: &Network, left: usize, right: usize) -> f64 {
    let (a, b) = (&net.nodes[left], &net.nodes[right]);
    let gap = match (a.dummy, b.dummy) {
        (false, false) => GAP_NODES,
        (true, true) => GAP_DUMMIES,
        (true, false) | (false, true) => GAP_MIXED,
    };
    a.right + gap + b.left
}

fn assign_x(net: &mut Network) {
    let layer_count = net.layers.len();
    for round in 0..ROUNDS {
        let downward = round % 2 == 0;
        for step in 0..layer_count {
            let layer = if downward {
                step
            } else {
                layer_count - 1 - step
            };
            place_layer(net, layer, downward);
        }
    }
    let left = net
        .nodes
        .iter()
        .zip(&net.x)
        .map(|(n, x)| x - n.left)
        .fold(f64::INFINITY, f64::min);
    let shift = if left.is_finite() {
        MARGIN - left
    } else {
        MARGIN
    };
    let mut right: f64 = 0.0;
    for (node, x) in net.nodes.iter().zip(&mut net.x) {
        *x += shift;
        right = right.max(*x + node.right);
    }
    net.width = right + MARGIN;
}

/// Moves a layer's nodes as close as possible to the mean x of their neighbours in
/// the previous layer (or the other side when there are none), keeping order and gaps.
fn place_layer(net: &mut Network, layer: usize, downward: bool) {
    let ids = &net.layers[layer];
    let mut targets = Vec::with_capacity(ids.len());
    let mut weights = Vec::with_capacity(ids.len());
    let mut offsets = Vec::with_capacity(ids.len());
    let mut offset = 0.0;
    for (index, &id) in ids.iter().enumerate() {
        if index > 0 {
            offset += separation(net, ids[index - 1], id);
        }
        let (primary, secondary) = if downward {
            (&net.up[id], &net.down[id])
        } else {
            (&net.down[id], &net.up[id])
        };
        let neighbours = if primary.is_empty() {
            secondary
        } else {
            primary
        };
        let desired = if neighbours.is_empty() {
            net.x[id]
        } else {
            let sum: f64 = neighbours.iter().map(|&n| net.x[n]).sum();
            sum / crate::text::to_f64(neighbours.len())
        };
        targets.push(desired - offset);
        weights.push(net.nodes[id].weight);
        offsets.push(offset);
    }
    let fitted = isotonic(&targets, &weights);
    for (index, &id) in ids.iter().enumerate() {
        net.x[id] = fitted[index] + offsets[index];
    }
}

/// Weighted least-squares non-decreasing fit (pool adjacent violators).
pub fn isotonic(targets: &[f64], weights: &[f64]) -> Vec<f64> {
    struct Block {
        value: f64,
        weight: f64,
        len: usize,
    }
    let mut blocks: Vec<Block> = Vec::with_capacity(targets.len());
    for (&value, &weight) in targets.iter().zip(weights) {
        let mut block = Block {
            value,
            weight,
            len: 1,
        };
        while let Some(previous) = blocks.last() {
            if previous.value <= block.value {
                break;
            }
            let total = previous.weight + block.weight;
            block.value = (previous.value * previous.weight + block.value * block.weight) / total;
            block.weight = total;
            block.len += previous.len;
            blocks.pop();
        }
        blocks.push(block);
    }
    let mut fitted = Vec::with_capacity(targets.len());
    for block in blocks {
        fitted.extend(std::iter::repeat_n(block.value, block.len));
    }
    fitted
}

fn assign_y(net: &mut Network) {
    let mut top = MARGIN;
    for layer in &net.layers {
        let height = layer
            .iter()
            .map(|&id| net.nodes[id].height)
            .fold(0.0, f64::max);
        for &id in layer {
            net.y[id] = top + height / 2.0;
        }
        net.row_height.push(height);
        top += height + RANK_GAP;
    }
    net.height = top - RANK_GAP + MARGIN;
}

#[cfg(test)]
mod tests {
    use super::isotonic;

    fn close(a: &[f64], b: &[f64]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9)
    }

    #[test]
    fn isotonic_keeps_sorted_input() {
        assert!(close(
            &isotonic(&[1.0, 2.0, 3.0], &[1.0; 3]),
            &[1.0, 2.0, 3.0]
        ));
    }

    #[test]
    fn isotonic_pools_violators_by_weight() {
        assert!(close(&isotonic(&[3.0, 1.0], &[1.0, 1.0]), &[2.0, 2.0]));
        assert!(close(&isotonic(&[4.0, 1.0], &[1.0, 2.0]), &[2.0, 2.0]));
        assert!(close(
            &isotonic(&[1.0, 5.0, 3.0, 0.0], &[1.0; 4]),
            &[1.0, 8.0 / 3.0, 8.0 / 3.0, 8.0 / 3.0]
        ));
    }

    #[test]
    fn isotonic_of_nothing_is_nothing() {
        assert!(isotonic(&[], &[]).is_empty());
    }
}
