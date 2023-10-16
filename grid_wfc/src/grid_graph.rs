use bevy::prelude::*;
use hierarchical_wfc::{Graph, Neighbor};

#[derive(Reflect, Clone)]
#[reflect(Default)]
pub struct GridGraphSettings {
    pub width: usize,
    pub height: usize,
    pub periodic: bool,
}

impl Default for GridGraphSettings {
    fn default() -> Self {
        Self {
            width: 256,
            height: 256,
            periodic: false,
        }
    }
}

pub fn create<F: Clone>(settings: &GridGraphSettings, fill_with: F) -> Graph<F> {
    let size = IVec2::new(settings.width as i32, settings.height as i32);

    let mut nodes_pos = Vec::new();
    for y in 0..settings.height {
        for x in 0..settings.width {
            nodes_pos.push(IVec2::new(x as i32, y as i32));
        }
    }

    let directions = [
        IVec2::new(0, 1),
        IVec2::new(0, -1),
        IVec2::new(-1, 0),
        IVec2::new(1, 0),
    ];

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

            let neighbor_index = (neighbor_pos.y * size.x + neighbor_pos.x) as usize;
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

    pub fn to_ivec2(&self) -> IVec2 {
        match self {
            Self::Up => IVec2::new(0, 1),
            Self::Down => IVec2::new(0, -1),
            Self::Left => IVec2::new(-1, 0),
            Self::Right => IVec2::new(1, 0),
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
