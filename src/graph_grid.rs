use crate::{
    graph::{Cell, Graph, Neighbor},
    tileset::TileSet,
};
use bevy::prelude::*;

#[derive(Reflect)]
#[reflect(Default)]
pub struct GridGraphSettings {
    pub width: usize,
    pub height: usize,
    pub periodic: bool,
}

impl Default for GridGraphSettings {
    fn default() -> Self {
        Self {
            width: 10,
            height: 10,
            periodic: false,
        }
    }
}

pub fn create<T: TileSet>(settings: &GridGraphSettings) -> Graph<Cell> {
    let size = IVec2::new(settings.width as i32, settings.height as i32);

    let mut nodes_pos = Vec::new();
    for x in 0..settings.width {
        for y in 0..settings.height {
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

            let neighbor_index = (neighbor_pos.x * size.y + neighbor_pos.y) as usize;
            node_neighbors.push(Neighbor {
                direction: i,
                index: neighbor_index,
            });
        }
        neighbors.push(node_neighbors);
    }

    let filled = Cell::filled(T::TILE_COUNT);
    let tiles = vec![filled; nodes_pos.len()];

    Graph { tiles, neighbors }
}
