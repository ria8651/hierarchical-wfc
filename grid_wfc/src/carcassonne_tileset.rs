use crate::grid_graph::{self, Direction, GridGraphSettings};
use bevy::prelude::*;
use hierarchical_wfc::{Graph, TileRender, TileSet, WaveFunction};
use std::{any::Any, sync::Arc};

const TILE_COUNT: usize = 120;
const DIRECTIONS: usize = 4;

#[derive(Debug, Clone)]
pub struct CarcassonneTileset {
    constraints: Arc<Vec<Vec<WaveFunction>>>,
    weights: Arc<Vec<f32>>,
}

impl Default for CarcassonneTileset {
    fn default() -> Self {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum TileEdgeType {
            Grass,
            Road,
            City,
            River,
        }
        type T = TileEdgeType;

        let tile_edge_types = [
            [T::Grass, T::Road, T::Road, T::Grass],
            [T::City, T::Road, T::City, T::City],
            [T::City, T::Grass, T::City, T::Grass],
            [T::City, T::Road, T::City, T::Road],
            [T::Grass, T::Grass, T::City, T::City],
            [T::City, T::Grass, T::City, T::Grass],
            [T::City, T::City, T::Grass, T::Grass],
            [T::City, T::Grass, T::Grass, T::Grass],
            [T::City, T::Road, T::Road, T::Grass],
            [T::City, T::Road, T::Grass, T::Road],
            [T::City, T::Road, T::Road, T::Road],
            [T::City, T::Grass, T::Road, T::Road],
            [T::Road, T::Road, T::Grass, T::Grass],
            [T::Grass, T::Road, T::Road, T::Grass],
            [T::Grass, T::Road, T::Road, T::Road],
            [T::Grass, T::Grass, T::Grass, T::Grass],
            [T::Grass, T::Road, T::Grass, T::Grass],
            [T::City, T::City, T::City, T::City],
            [T::City, T::Grass, T::City, T::City],
            [T::Grass, T::River, T::Grass, T::Grass],
            [T::Grass, T::River, T::Grass, T::River],
            [T::Grass, T::Road, T::River, T::River],
            [T::Road, T::River, T::River, T::Road],
            [T::River, T::River, T::Grass, T::Grass],
            [T::River, T::River, T::Grass, T::Grass],
            [T::River, T::Grass, T::Grass, T::Grass],
            [T::River, T::River, T::Road, T::City],
            [T::City, T::City, T::River, T::River],
            [T::Road, T::Road, T::River, T::River],
            [T::River, T::City, T::River, T::City],
        ];

        // rotate all tiles to get all possible edge types
        let mut rotated_tile_edge_types = Vec::with_capacity(TILE_COUNT);
        for rotation in 0..4 {
            for edges in tile_edge_types.iter() {
                let mut rotated_edges = vec![T::Grass; DIRECTIONS];
                for (edge_index, edge) in edges.iter().enumerate() {
                    let direction = Direction::from(edge_index);
                    rotated_edges[direction.rotate(rotation) as usize] = *edge;
                }
                rotated_tile_edge_types.push(rotated_edges);
            }
        }

        // convert to allowed neighbors
        let mut allowed_neighbors = Vec::with_capacity(TILE_COUNT);
        for edges in rotated_tile_edge_types.iter() {
            let mut allowed_neighbors_for_tile = Vec::with_capacity(DIRECTIONS);
            for (edge_index, edge) in edges.into_iter().enumerate() {
                let direction = Direction::from(edge_index);
                let mut cell = WaveFunction::empty();

                // add all tiles with this edge type to the neighbor set
                for (other_tile, other_edges) in rotated_tile_edge_types.iter().enumerate() {
                    if other_edges[direction.other() as usize] == *edge {
                        cell.add_tile(other_tile);
                    }
                }

                allowed_neighbors_for_tile.push(cell);
            }
            allowed_neighbors.push(allowed_neighbors_for_tile);
        }

        let mut weights = Vec::with_capacity(TILE_COUNT);
        for _ in 0..TILE_COUNT {
            weights.push(1.0);
        }

        Self {
            constraints: Arc::new(allowed_neighbors),
            weights: Arc::new(weights),
        }
    }
}

impl TileSet for CarcassonneTileset {
    fn tile_count(&self) -> usize {
        TILE_COUNT
    }

    fn directions(&self) -> usize {
        4
    }

    fn get_constraints(&self) -> Arc<Vec<Vec<WaveFunction>>> {
        self.constraints.clone()
    }

    fn get_weights(&self) -> Arc<Vec<f32>> {
        self.weights.clone()
    }

    fn set_weights(&mut self, weights: Vec<f32>) {
        self.weights = Arc::new(weights);
    }

    fn create_graph(&self, settings: Box<dyn Any>) -> Graph<WaveFunction> {
        let settings = settings.downcast_ref::<GridGraphSettings>().unwrap();
        grid_graph::create(settings, WaveFunction::filled(self.tile_count()))
    }

    fn get_tile_paths(&self) -> Vec<(TileRender, Transform)> {
        let mut paths = Vec::new();
        for tile in 0..self.tile_count() {
            let transform = Transform::from_rotation(Quat::from_rotation_z(
                -std::f32::consts::PI / 2.0 * (4 * tile / self.tile_count()) as f32,
            ));
            paths.push((
                TileRender::Sprite(format!(
                    "carcassonne/{}.png",
                    tile % (self.tile_count() / 4)
                )),
                transform,
            ));
        }
        paths
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
