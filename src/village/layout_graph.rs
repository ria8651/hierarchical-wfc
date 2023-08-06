use crate::wfc::{Graph, Neighbour};
use bevy::{
    math::{ivec3, vec3},
    prelude::*,
};

#[derive(Reflect)]
#[reflect(Default)]
pub struct LayoutGraphSettings {
    pub x_size: usize,
    pub y_size: usize,
    pub z_size: usize,
    pub periodic: bool,
}

impl LayoutGraphSettings {
    pub fn posf32_from_index(&self, index: usize) -> Vec3 {
        let (i, j, k) = (
            index.rem_euclid(self.x_size),
            index.div_euclid(self.x_size).rem_euclid(self.y_size),
            index.div_euclid(self.x_size * self.y_size),
        );
        vec3(
            (i as f32) * 2.0 + 1.0,
            (j as f32) * 3.0 + 1.5,
            (k as f32) * 2.0 + 1.0,
        )
    }
}

impl Default for LayoutGraphSettings {
    fn default() -> Self {
        Self {
            x_size: 10,
            y_size: 1,
            z_size: 10,
            periodic: false,
        }
    }
}

const DIRECTIONS: [IVec3; 6] = [
    IVec3 { x: 1, y: 0, z: 0 },
    IVec3 { x: -1, y: 0, z: 0 },
    IVec3 { x: 0, y: 1, z: 0 },
    IVec3 { x: 0, y: -1, z: 0 },
    IVec3 { x: 0, y: 0, z: 1 },
    IVec3 { x: 0, y: 0, z: -1 },
];

pub fn create_layout_graph<F: Clone>(settings: &LayoutGraphSettings, fill_with: F) -> Graph<F> {
    let mut neighbors: Vec<Vec<Neighbour>> = Vec::new();
    let x_size = settings.x_size as i32;
    let y_size = settings.y_size as i32;
    let z_size = settings.z_size as i32;

    for z in 0..z_size {
        for y in 0..y_size {
            for x in 0..x_size {
                let mut current_neighbours = Vec::new();
                let pos = ivec3(x, y, z);
                for (arc_type, delta) in DIRECTIONS.into_iter().enumerate() {
                    let n_pos = pos + delta;
                    if n_pos.cmpge(IVec3::ZERO).all()
                        && n_pos.cmplt(ivec3(x_size, y_size, z_size)).all()
                    {
                        let (i, j, k) = (n_pos.x as usize, n_pos.y as usize, n_pos.z as usize);
                        let index = i + j * settings.x_size + k * settings.x_size * settings.y_size;

                        current_neighbours.push(Neighbour {
                            arc_type,
                            index: index,
                        });
                    }
                }

                neighbors.push(current_neighbours);
            }
        }
    }
    let tiles = vec![fill_with; (x_size * y_size * z_size) as usize];

    Graph {
        nodes: tiles,
        neighbors,
        order: Vec::new(),
    }
}
