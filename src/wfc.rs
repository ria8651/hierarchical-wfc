use crate::graph::{Graph, Neighbor, Superposition};
use rand::Rng;

pub struct GraphWfc;

impl GraphWfc {
    pub fn collapse<R: Rng>(
        graph: &mut Graph<Superposition>,
        constraints: &Vec<Vec<Superposition>>,
        weights: &Vec<u32>,
        rng: &mut R,
    ) {
        let start_node = rng.gen_range(0..graph.nodes.len());
        graph.nodes[start_node].select_random(rng, weights);
        graph.order.push(start_node);

        let mut stack = vec![start_node];
        while let Some(index) = stack.pop() {
            // propagate changes for node from stack
            for i in 0..graph.neighbors[index].len() {
                let neighbor = graph.neighbors[index][i];
                if GraphWfc::propagate(graph, index, neighbor, &constraints) {
                    stack.push(neighbor.index);
                }
            }

            // once all changes are propagated the stack will be empty
            if stack.len() == 0 {
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

                if let Some(index) = min_index {
                    graph.nodes[index].select_random(rng, weights);

                    // assert!(
                    //     !graph.order.contains(&index),
                    //     "{} in [{}]",
                    //     &index,
                    //     graph
                    //         .order
                    //         .iter()
                    //         .map(|e| format!("{}", e))
                    //         .collect::<Vec<_>>()
                    //         .join(", ")
                    // );
                    graph.order.push(index);
                    stack.push(index);
                }
            }
        }
    }

    /// Returns true if the tile was updated
    pub fn propagate(
        graph: &mut Graph<Superposition>,
        index: usize,
        neighbor: Neighbor,
        allowed_neighbors: &Vec<Vec<Superposition>>,
    ) -> bool {
        let mut updated = false;

        let mut allowed = Superposition::empty();
        for tile in graph.nodes[index].tile_iter() {
            allowed = Superposition::join(&allowed, &allowed_neighbors[tile][neighbor.arc_type]);
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
        let neighbor_tiles = graph.nodes[neighbor.index].clone();
        let new_tiles = Superposition::intersect(&neighbor_tiles, &allowed);
        if new_tiles.count_bits() < neighbor_tiles.count_bits() {
            if new_tiles.count_bits() == 1 {
                // dbg!(("From propagate", neighbor.index));
                // assert!(
                //     !graph.order.contains(&neighbor.index),
                //     "{} in [{}]",
                //     &neighbor.index,
                //     graph
                //         .order
                //         .iter()
                //         .map(|e| format!("{}", e))
                //         .collect::<Vec<_>>()
                //         .join(", ")
                // );
                graph.order.push(neighbor.index)
            }
            updated = true;
            graph.nodes[neighbor.index] = new_tiles;
        }

        updated
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub enum Direction {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
}

impl Direction {
    pub fn other(&self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    pub fn rotate(&self, rotation: usize) -> Self {
        match rotation {
            0 => *self,
            1 => match self {
                Self::Up => Self::Right,
                Self::Down => Self::Left,
                Self::Left => Self::Up,
                Self::Right => Self::Down,
            },
            2 => match self {
                Self::Up => Self::Down,
                Self::Down => Self::Up,
                Self::Left => Self::Right,
                Self::Right => Self::Left,
            },
            3 => match self {
                Self::Up => Self::Left,
                Self::Down => Self::Right,
                Self::Left => Self::Down,
                Self::Right => Self::Up,
            },
            _ => panic!("Invalid rotation: {}", rotation),
        }
    }
}

impl From<usize> for Direction {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::Up,
            1 => Self::Down,
            2 => Self::Left,
            3 => Self::Right,
            _ => panic!("Invalid direction: {}", value),
        }
    }
}
