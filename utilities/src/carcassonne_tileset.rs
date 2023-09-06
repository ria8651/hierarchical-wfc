use crate::graph_grid::{self, Direction, GridGraphSettings};
use bevy::prelude::*;
use hierarchical_wfc::{Graph, TileSet, WaveFunction};

#[derive(Debug, Default, Clone)]
pub struct CarcassonneTileset;

impl TileSet for CarcassonneTileset {
    type GraphSettings = GridGraphSettings;

    fn tile_count(&self) -> usize {
        120
    }

    fn directions(&self) -> usize {
        4
    }

    fn get_constraints(&self) -> Vec<Vec<WaveFunction>> {
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
        let mut rotated_tile_edge_types = Vec::with_capacity(self.tile_count());
        for rotation in 0..4 {
            for edges in tile_edge_types.iter() {
                let mut rotated_edges = vec![T::Grass; self.directions()];
                for (edge_index, edge) in edges.iter().enumerate() {
                    let direction = Direction::from(edge_index);
                    rotated_edges[direction.rotate(rotation) as usize] = *edge;
                }
                rotated_tile_edge_types.push(rotated_edges);
            }
        }

        // convert to allowed neighbors
        let mut allowed_neighbors = Vec::with_capacity(self.tile_count());
        for edges in rotated_tile_edge_types.iter() {
            let mut allowed_neighbors_for_tile = Vec::with_capacity(self.directions());
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

        allowed_neighbors
    }

    fn get_weights(&self) -> Vec<f32> {
        let mut weights = Vec::with_capacity(self.tile_count());
        for _ in 0..self.tile_count() {
            weights.push(1.0);
        }
        weights
    }

    fn get_tile_paths(&self) -> Vec<(String, Transform)> {
        let mut paths = Vec::new();
        for tile in 0..self.tile_count() {
            let transform = Transform::from_rotation(Quat::from_rotation_z(
                -std::f32::consts::PI / 2.0 * (4 * tile / self.tile_count()) as f32,
            ));
            paths.push((
                format!("carcassonne/{}.png", tile % (self.tile_count() / 4)),
                transform,
            ));
        }
        paths
    }

    fn create_graph(&self, settings: &Self::GraphSettings) -> Graph<WaveFunction> {
        let cell = WaveFunction::filled(self.tile_count());
        graph_grid::create(settings, cell)
    }
}
