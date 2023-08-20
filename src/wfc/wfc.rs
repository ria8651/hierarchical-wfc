use super::{Neighbour, Superposition, WfcGraph};
use rand::Rng;

pub struct WaveFunctionCollapse;

impl WaveFunctionCollapse {
    pub fn min_entropy<R: Rng>(graph: &WfcGraph<Superposition>, rng: &mut R) -> Option<usize> {
        let mut min_entropy = usize::MAX;
        let mut min_index = None;
        let mut with_min: usize = 0; // Track how many nodes has the lowest entropy found
        for (index, node) in graph.nodes.iter().enumerate() {
            let entropy = node.count_bits();
            if entropy > 1 && entropy <= min_entropy {
                with_min += 1;
                if entropy < min_entropy {
                    with_min = 1;
                    min_entropy = entropy;
                    min_index = Some(index);
                } else {
                    with_min += 1;

                    // Select new node so that all nodes with min_entropy have equal chance of been chosen
                    if rng.gen_bool(1.0f64 / with_min as f64) {
                        min_entropy = entropy;
                        min_index = Some(index);
                    }
                }
            }
        }
        min_index
    }

    pub fn collapse<R: Rng>(
        graph: &mut WfcGraph<Superposition>,
        constraints: &Vec<Vec<Superposition>>,
        weights: &Vec<u32>,
        rng: &mut R,
    ) {
        // Self::pre_propagate(graph, constraints);

        // let start_node = Self::min_entropy(graph, rng).unwrap();
        // println!("\t{}: {}", graph.order.len(), graph.nodes[start_node]);
        // graph.nodes[start_node].select_random(rng, weights);
        // graph.order.push(start_node);

        let mut stack = Vec::from_iter(0..graph.nodes.len());
        while let Some(index) = stack.pop() {
            // propagate changes for node from stack
            for i in 0..graph.neighbors[index].len() {
                let neighbor = graph.neighbors[index][i];
                if WaveFunctionCollapse::propagate(graph, index, neighbor, &constraints) {
                    stack.push(neighbor.index);
                }
            }

            // once all changes are propagated the stack will be empty
            if stack.len() == 0 {
                if let Some(index) = Self::min_entropy(graph, rng) {
                    graph.nodes[index].select_random(rng, weights);
                    println!("\t{}: {}", graph.order.len(), graph.nodes[index]);

                    graph.order.push(index);
                    stack.push(index);
                }
            }
        }
    }

    fn pre_propagate(graph: &mut WfcGraph<Superposition>, constraints: &Vec<Vec<Superposition>>) {
        let mut stack = Vec::from_iter(0..graph.nodes.len());
        while let Some(node_id) = stack.pop() {
            for i in 0..graph.neighbors[node_id].len() {
                let neighbor = graph.neighbors[node_id][i];
                if WaveFunctionCollapse::propagate(graph, node_id, neighbor, &constraints) {
                    stack.push(neighbor.index);
                    println!("Pushed: {}", neighbor.index);
                }
            }
        }
    }

    /// Returns true if the tile was updated
    pub fn propagate(
        graph: &mut WfcGraph<Superposition>,
        index: usize,
        neighbour: Neighbour,
        allowed_neighbors: &Vec<Vec<Superposition>>,
    ) -> bool {
        let mut updated = false;

        let mut allowed = Superposition::empty();
        for tile in graph.nodes[index].tile_iter() {
            allowed = Superposition::join(&allowed, &allowed_neighbors[tile][neighbour.arc_type]);
        }
        // Prevent error from spreading to entire graph
        // so we can see what went wrong
        if graph.nodes[index].count_bits() == 0 {
            if !graph.order.contains(&index) {
                graph.order.push(index);
            }
            return false;
        }

        // Propagate to specified neighbour
        let neighbor_tiles = graph.nodes[neighbour.index].clone();
        let new_tiles = Superposition::intersect(&neighbor_tiles, &allowed);
        if new_tiles.count_bits() < neighbor_tiles.count_bits() {
            if new_tiles.count_bits() == 1 {
                graph.order.push(neighbour.index)
            }
            updated = true;
            graph.nodes[neighbour.index] = new_tiles;
        }

        updated
    }
}