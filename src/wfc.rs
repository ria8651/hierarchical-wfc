use crate::graph::{Cell, Graph, Neighbor};
use rand::Rng;
use std::time::Instant;

pub struct GraphWfc;

impl GraphWfc {
    /// Returns true if returned early
    pub fn collapse<R: Rng>(
        graph: &mut Graph<Cell>,
        constraints: &Vec<Vec<Cell>>,
        weights: &Vec<u32>,
        rng: &mut R,
        timeout: Option<f64>,
    ) -> bool {
        let start = Instant::now();

        let mut stack = Vec::new();
        while let Some(cell) = GraphWfc::lowest_entropy(graph) {
            // collapse cell
            graph.tiles[cell].select_random(rng, weights);

            // propagate changes
            stack.push(cell);
            while let Some(index) = stack.pop() {
                for i in 0..graph.neighbors[index].len() {
                    // propagate changes
                    let neighbor = graph.neighbors[index][i];
                    if GraphWfc::propagate(graph, index, neighbor, &constraints) {
                        stack.push(neighbor.index);
                    }
                }
            }

            if let Some(timeout) = timeout {
                if start.elapsed().as_secs_f64() > timeout {
                    return true;
                }
            }
        }

        false

        // for y in (0..grid_wfc.grid[0].len()).rev() {
        //     for x in 0..grid_wfc.grid.len() {
        //         let tiles = &grid_wfc.grid[x][y];
        //         print!("{:<22}", format!("{:?}", tiles));
        //     }
        //     println!();
        // }
    }

    pub fn lowest_entropy(graph: &mut Graph<Cell>) -> Option<usize> {
        let mut rng = rand::thread_rng();

        // find next cell to update
        let mut min_entropy = usize::MAX;
        let mut min_index = None;
        let mut with_min: usize = 0; // Track how many nodes has the lowest entropy found
        for (index, node) in graph.tiles.iter().enumerate() {
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

    /// Returns true if the tile was updated
    pub fn propagate(
        graph: &mut Graph<Cell>,
        index: usize,
        neighbor: Neighbor,
        allowed_neighbors: &Vec<Vec<Cell>>,
    ) -> bool {
        let mut updated = false;

        let mut allowed = Cell::empty();
        for tile in graph.tiles[index].tile_iter() {
            allowed = Cell::join(&allowed, &allowed_neighbors[tile][neighbor.direction]);
        }

        let neighbor_tiles = graph.tiles[neighbor.index].clone();
        let new_tiles = Cell::intersect(&neighbor_tiles, &allowed);
        if new_tiles != neighbor_tiles {
            updated = true;
            graph.tiles[neighbor.index] = new_tiles;
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
        self.rotate(2)
    }

    pub fn rotate(&self, rotation: usize) -> Self {
        if rotation == 0 {
            return *self;
        }
        if rotation >= 4 {
            panic!("Invalid rotation: {}", rotation);
        }

        // Array that specifies the correct rotation order
        let rotation_order = [Self::Up, Self::Right, Self::Down, Self::Left];
        let current_idx = rotation_order.iter().position(|&dir| dir == *self).unwrap();
        let new_idx = (current_idx + rotation) % 4;
        rotation_order[new_idx]
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
