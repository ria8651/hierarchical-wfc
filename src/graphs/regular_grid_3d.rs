use crate::wfc::{Neighbour, WfcGraph};
use bevy::{math::ivec3, prelude::*};
use itertools::{iproduct, Itertools};

#[derive(Component, Clone)]
pub struct GraphSettings {
    pub size: UVec3,
    pub spacing: Vec3,
}

#[derive(Component, Clone)]
pub struct GraphData {
    pub node_positions: Box<[IVec3]>,
}

pub fn create_graph<F: Clone>(
    settings: &GraphSettings,
    create_node: &dyn Fn((usize, IVec3)) -> F,
) -> (GraphData, WfcGraph<F>) {
    let GraphSettings { size, spacing: _ } = settings;
    let mut neighbors: Vec<Box<[Neighbour]>> = Vec::new();
    let mut positions = Vec::with_capacity((size.x * size.y * size.z) as usize);
    for z in 0..size.z {
        for y in 0..size.y {
            for x in 0..size.x {
                let mut current_neighbours = Vec::new();
                let pos = ivec3(x as i32, y as i32, z as i32);
                positions.push(pos);
                for (arc_type, delta) in DIRECTIONS.into_iter().enumerate() {
                    let n_pos = pos + delta;
                    if n_pos.cmpge(IVec3::ZERO).all()
                        && n_pos
                            .cmplt(ivec3(size.x as i32, size.y as i32, size.z as i32))
                            .all()
                    {
                        let (i, j, k) = (n_pos.x as usize, n_pos.y as usize, n_pos.z as usize);
                        let index = i + j * size.x as usize + k * size.x as usize * size.y as usize;

                        current_neighbours.push(Neighbour { arc_type, index });
                    }
                }

                neighbors.push(current_neighbours.into());
            }
        }
    }
    let tiles = iproduct!(0..size.z as i32, 0..size.y as i32, 0..size.x as i32)
        .enumerate()
        .map(|(i, (z, y, x))| create_node((i, ivec3(x, y, z))))
        .collect_vec();

    (
        GraphData {
            node_positions: positions.into(),
        },
        WfcGraph {
            nodes: tiles,
            neighbours: neighbors.into(),
            order: Vec::new(),
        },
    )
}

pub fn create_cuboid<F: Clone>(
    min: IVec3,
    max: IVec3,
    create_node: &dyn Fn((usize, IVec3)) -> F,
) -> (GraphData, WfcGraph<F>) {
    let size = max - min;
    let mut neighbors: Vec<Box<[Neighbour]>> = Vec::new();
    let mut positions = Vec::with_capacity((size.x * size.y * size.z) as usize);
    for z in 0..size.z {
        for y in 0..size.y {
            for x in 0..size.x {
                let mut current_neighbours = Vec::new();
                let pos = ivec3(x as i32, y as i32, z as i32);
                positions.push(pos + min);
                for (arc_type, delta) in DIRECTIONS.into_iter().enumerate() {
                    let n_pos = pos + delta;
                    if n_pos.cmpge(IVec3::ZERO).all()
                        && n_pos
                            .cmplt(ivec3(size.x as i32, size.y as i32, size.z as i32))
                            .all()
                    {
                        let (i, j, k) = (n_pos.x as usize, n_pos.y as usize, n_pos.z as usize);
                        let index = i + j * size.x as usize + k * size.x as usize * size.y as usize;

                        current_neighbours.push(Neighbour { arc_type, index });
                    }
                }

                neighbors.push(current_neighbours.into());
            }
        }
    }
    let tiles = iproduct!(0..size.z as i32, 0..size.y as i32, 0..size.x as i32)
        .enumerate()
        .map(|(i, (z, y, x))| create_node((i, ivec3(x, y, z))))
        .collect_vec();

    (
        GraphData {
            node_positions: positions.into(),
        },
        WfcGraph {
            nodes: tiles,
            neighbours: neighbors.into(),
            order: Vec::new(),
        },
    )
}

const DIRECTIONS: [IVec3; 6] = [
    IVec3 { x: 1, y: 0, z: 0 },
    IVec3 { x: -1, y: 0, z: 0 },
    IVec3 { x: 0, y: 1, z: 0 },
    IVec3 { x: 0, y: -1, z: 0 },
    IVec3 { x: 0, y: 0, z: 1 },
    IVec3 { x: 0, y: 0, z: -1 },
];
