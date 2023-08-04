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
        let start_node = rng.gen_range(0..graph.tiles.len());

        // update cell
        graph.tiles[start_node].select_random(rng, weights);

        let mut stack = vec![start_node];
        while let Some(index) = stack.pop() {
            for i in 0..graph.neighbors[index].len() {
                // propagate changes
                let neighbor = graph.neighbors[index][i];
                if GraphWfc::propagate(graph, index, neighbor, &constraints) {
                    stack.push(neighbor.index);
                }
            }

            if stack.len() == 0 {
                // find next cell to update
                let mut min_entropy = usize::MAX;
                let mut min_pos = None;
                for (index, node) in graph.tiles.iter().enumerate() {
                    let entropy = node.count_bits();
                    if entropy > 1 && entropy < min_entropy {
                        min_entropy = entropy;
                        min_pos = Some(index);
                    }
                }

                if let Some(pos) = min_pos {
                    // update cell
                    println!("\nSelecting!");
                    dbg!(weights);
                    dbg!(graph.tiles[pos]);

                    graph.tiles[pos].select_random(rng, weights);
                    dbg!(graph.tiles[pos]);
                    stack.push(pos);
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
        for tile in graph.tiles[index].tile_iter() {
            allowed = Superposition::join(&allowed, &allowed_neighbors[tile][neighbor.arc_type]);
        }

        let neighbor_tiles = graph.tiles[neighbor.index].clone();
        let new_tiles = Superposition::intersect(&neighbor_tiles, &allowed);
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
