use crate::graph::{Cell, Graph, Neighbor};
use rand::Rng;

pub struct GraphWfc;

impl GraphWfc {
    pub fn collapse<R: Rng>(graph: &mut Graph<Cell>, constraints: &Vec<Vec<Cell>>, rng: &mut R) {
        let start_node = rng.gen_range(0..graph.tiles.len());

        // update cell
        graph.tiles[start_node].select_random(rng);

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
                    graph.tiles[pos].select_random(rng);
                    stack.push(pos);
                }
            }
        }

        // for y in (0..grid_wfc.grid[0].len()).rev() {
        //     for x in 0..grid_wfc.grid.len() {
        //         let tiles = &grid_wfc.grid[x][y];
        //         print!("{:<22}", format!("{:?}", tiles));
        //     }
        //     println!();
        // }
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
