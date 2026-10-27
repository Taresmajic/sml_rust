//! Proper layered graph: real nodes plus dummy nodes on every intermediate rank.

use crate::layout::measure::Measured;
use crate::layout::route::LoopSpace;
use crate::layout::{LABEL_GAP, Link};
use crate::text::LINE_HEIGHT;

/// Half-width reserved for an edge passing through a rank.
const DUMMY_HALF_WIDTH: f64 = 1.0;
/// Dummies pull harder towards their neighbours, which straightens long edges.
const DUMMY_WEIGHT: f64 = 2.0;

pub struct VNode {
    /// Extent left of the anchor x.
    pub left: f64,
    /// Extent right of the anchor x (labels and self-loops live here).
    pub right: f64,
    pub height: f64,
    pub rank: usize,
    pub weight: f64,
    /// Real node this vnode is, or whose outgoing chain it belongs to.
    pub origin: usize,
    pub dummy: bool,
}

/// Vnodes `0..n` are the real nodes, in graph order; dummies follow.
pub struct Network {
    pub nodes: Vec<VNode>,
    pub layers: Vec<Vec<usize>>,
    pub up: Vec<Vec<usize>>,
    pub down: Vec<Vec<usize>>,
    /// Per link: vnodes from the upper endpoint to the lower one.
    pub chains: Vec<Vec<usize>>,
    /// Per link: the dummy that carries its label.
    pub labels: Vec<Option<usize>>,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    /// Height of each rank's row.
    pub row_height: Vec<f64>,
    pub width: f64,
    pub height: f64,
}

impl Network {
    pub fn build(
        measured: &[Measured],
        ranks: &[usize],
        links: &[Link],
        reversed: &[bool],
        loops: &LoopSpace,
    ) -> Self {
        let mut nodes = Vec::new();
        for (index, size) in measured.iter().enumerate() {
            nodes.push(VNode {
                left: size.width / 2.0,
                right: size.width / 2.0 + loops.extent[index],
                height: size.height.max(loops.height[index]),
                rank: ranks[index],
                weight: 1.0,
                origin: index,
                dummy: false,
            });
        }
        let mut net = Self {
            nodes,
            layers: Vec::new(),
            up: Vec::new(),
            down: Vec::new(),
            chains: Vec::new(),
            labels: Vec::new(),
            x: Vec::new(),
            y: Vec::new(),
            row_height: Vec::new(),
            width: 0.0,
            height: 0.0,
        };
        for (link, &flip) in links.iter().zip(reversed) {
            net.add_chain(link, flip, ranks);
        }
        net.connect_and_layer();
        net
    }

    fn add_chain(&mut self, link: &Link, flip: bool, ranks: &[usize]) {
        let (upper, lower) = if flip {
            (link.to, link.from)
        } else {
            (link.from, link.to)
        };
        // Label next to the edge's actual source: top for downward edges, bottom otherwise.
        let label_rank = if flip {
            ranks[lower] - 1
        } else {
            ranks[upper] + 1
        };
        let mut chain = vec![upper];
        let mut label = None;
        for rank in ranks[upper] + 1..ranks[lower] {
            let label_width = link.label_width.filter(|_| rank == label_rank);
            let id = self.nodes.len();
            self.nodes.push(VNode {
                left: DUMMY_HALF_WIDTH,
                right: label_width.map_or(DUMMY_HALF_WIDTH, |width| width + 2.0 * LABEL_GAP),
                height: label_width.map_or(0.0, |_| LINE_HEIGHT),
                rank,
                weight: DUMMY_WEIGHT,
                origin: upper,
                dummy: true,
            });
            if label_width.is_some() {
                label = Some(id);
            }
            chain.push(id);
        }
        chain.push(lower);
        self.chains.push(chain);
        self.labels.push(label);
    }

    fn connect_and_layer(&mut self) {
        let count = self.nodes.len();
        self.up = vec![Vec::new(); count];
        self.down = vec![Vec::new(); count];
        for chain in &self.chains {
            for pair in chain.windows(2) {
                self.down[pair[0]].push(pair[1]);
                self.up[pair[1]].push(pair[0]);
            }
        }
        let rank_count = self.nodes.iter().map(|n| n.rank + 1).max().unwrap_or(0);
        self.layers = vec![Vec::new(); rank_count];
        for (id, node) in self.nodes.iter().enumerate() {
            self.layers[node.rank].push(id);
        }
        self.x = vec![0.0; count];
        self.y = vec![0.0; count];
    }
}
