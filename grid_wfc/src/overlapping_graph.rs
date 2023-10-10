use bevy::prelude::*;
use hierarchical_wfc::{Graph, Neighbor};

#[derive(Reflect, Clone)]
#[reflect(Default)]
pub struct OverlappingGraphSettings {
    pub width: usize,
    pub height: usize,
    pub overlap: usize,
    pub periodic: bool,
}

impl Default for OverlappingGraphSettings {
    fn default() -> Self {
        Self {
            width: 16,
            height: 16,
            overlap: 1,
            periodic: false,
        }
    }
}

pub fn create<F: Clone>(settings: &OverlappingGraphSettings, fill_with: F) -> Graph<F> {
    let size = IVec2::new(settings.width as i32, settings.height as i32);

    let mut nodes_pos = Vec::new();
    for x in 0..settings.width {
        for y in 0..settings.height {
            nodes_pos.push(IVec2::new(x as i32, y as i32));
        }
    }

    let overlap = settings.overlap as i32;
    let overlap_width = overlap * 2 + 1;
    let mut neighbors = Vec::new();
    for pos in nodes_pos.iter() {
        let mut node_neighbors = Vec::new();
        for x in -overlap..=overlap {
            for y in -overlap..=overlap {
                let mut neighbor_pos = *pos + IVec2::new(x, y);
                if settings.periodic {
                    neighbor_pos = IVec2::new(
                        neighbor_pos.x.rem_euclid(size.x),
                        neighbor_pos.y.rem_euclid(size.y),
                    );
                } else if neighbor_pos.cmpge(size).any() || neighbor_pos.cmplt(IVec2::ZERO).any() {
                    continue;
                }

                let neighbor_index = (neighbor_pos.x * size.y + neighbor_pos.y) as usize;
                let direction_index = overlap_width * (x + overlap) + (y + overlap);
                node_neighbors.push(Neighbor {
                    direction: direction_index as usize,
                    index: neighbor_index,
                });
            }
        }
        neighbors.push(node_neighbors);
    }

    let tiles = vec![fill_with; nodes_pos.len()];

    Graph { tiles, neighbors }
}
