use crate::graph::{Graph, Neighbor};
use crate::graph_grid::GridGraphSettings;
use bevy::prelude::*;

fn directions() -> [IVec2; 8] {
    [
        IVec2::new(0, 1),   // up
        IVec2::new(1, 1),   // up right
        IVec2::new(1, 0),   // right
        IVec2::new(1, -1),  // down right
        IVec2::new(0, -1),  // down
        IVec2::new(-1, -1), // down left
        IVec2::new(-1, 0),  // left
        IVec2::new(-1, 1),  // up left
    ]
}

pub fn create<F: Clone>(settings: &GridGraphSettings, fill_with: F) -> Graph<F> {
    let size = IVec2::new(settings.width as i32, settings.height as i32);

    let mut nodes_pos = Vec::new();
    for x in 0..settings.width {
        for y in 0..settings.height {
            nodes_pos.push(IVec2::new(x as i32, y as i32));
        }
    }

    let directions = directions();

    let mut neighbors = Vec::new();
    for pos in nodes_pos.iter() {
        let mut node_neighbors = Vec::new();
        for (i, dir_vec) in directions.iter().enumerate() {
            let mut neighbor_pos = *pos + *dir_vec;
            if neighbor_pos.cmpge(size).any() && !settings.periodic {
                continue;
            }
            if neighbor_pos.cmplt(IVec2::ZERO).any() && !settings.periodic {
                continue;
            }
            if settings.periodic {
                neighbor_pos = IVec2::new(
                    neighbor_pos.x.rem_euclid(size.x),
                    neighbor_pos.y.rem_euclid(size.y),
                );
            }

            let neighbor_index = (neighbor_pos.x * size.y + neighbor_pos.y) as usize;
            node_neighbors.push(Neighbor {
                direction: i,
                index: neighbor_index,
            });
        }
        neighbors.push(node_neighbors);
    }

    let tiles = vec![fill_with; nodes_pos.len()];

    Graph { tiles, neighbors }
}

#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub enum Direction8D {
    Up = 0,
    UpRight = 1,
    Right = 2,
    DownRight = 3,
    Down = 4,
    DownLeft = 5,
    Left = 6,
    UpLeft = 7,
}

impl Direction8D {
    pub fn other(&self) -> Self {
        self.rotate(4)
    }

    pub fn rotate(&self, rotation: usize) -> Self {
        if rotation == 0 {
            return *self;
        }
        if rotation >= 8 {
            panic!("Invalid rotation: {}", rotation);
        }

        let current_direction = *self as usize;
        let new_direction = (current_direction + rotation) % 8;
        Direction8D::from(new_direction)
    }
}

impl From<usize> for Direction8D {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::Up,
            1 => Self::UpRight,
            2 => Self::Right,
            3 => Self::DownRight,
            4 => Self::Down,
            5 => Self::DownLeft,
            6 => Self::Left,
            7 => Self::UpLeft,
            _ => panic!("Invalid direction: {}", value),
        }
    }
}
