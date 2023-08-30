use bevy::prelude::*;

use crate::wfc::{Neighbour, WfcGraph};

#[derive(Reflect, Component, Clone)]
#[reflect(Default)]
pub struct GraphSettings {
    pub width: usize,
    pub height: usize,
    pub periodic: bool,
}

impl Default for GraphSettings {
    fn default() -> Self {
        Self {
            width: 10,
            height: 10,
            periodic: false,
        }
    }
}

pub fn create_grid_graph<F: Clone>(settings: &GraphSettings, fill_with: F) -> WfcGraph<F> {
    let size = IVec2::new(settings.width as i32, settings.height as i32);

    let mut nodes_pos = Vec::new();
    for x in 0..settings.width {
        for y in 0..settings.height {
            nodes_pos.push(IVec2::new(x as i32, y as i32));
        }
    }

    let arc_types = [
        IVec2::new(0, 1),
        IVec2::new(0, -1),
        IVec2::new(-1, 0),
        IVec2::new(1, 0),
    ];

    let mut neighbors: Vec<Box<[_]>> = Vec::new();
    for pos in nodes_pos.iter() {
        let mut node_neighbors = Vec::new();
        for (i, arc_t_vec) in arc_types.iter().enumerate() {
            let mut neighbour_pos = *pos + *arc_t_vec;
            if neighbour_pos.cmpge(size).any() && !settings.periodic {
                continue;
            }
            if neighbour_pos.cmplt(IVec2::ZERO).any() && !settings.periodic {
                continue;
            }
            if settings.periodic {
                neighbour_pos = IVec2::new(
                    neighbour_pos.x.rem_euclid(size.x),
                    neighbour_pos.y.rem_euclid(size.y),
                );
            }

            let neighbour_index = (neighbour_pos.x * size.y + neighbour_pos.y) as usize;
            node_neighbors.push(Neighbour {
                arc_type: i,
                index: neighbour_index,
            });
        }
        neighbors.push(node_neighbors.into());
    }

    let tiles = vec![fill_with; nodes_pos.len()];

    WfcGraph {
        nodes: tiles,
        neighbours: neighbors.into(),
        order: Vec::new(),
    }
}
