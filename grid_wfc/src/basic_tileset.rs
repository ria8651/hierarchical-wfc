use crate::grid_graph::Direction;
use bevy::prelude::*;
use hierarchical_wfc::{TileSet, WaveFunction};
use std::sync::Arc;

const TILE_COUNT: usize = 17;
const DIRECTIONS: usize = 4;

#[derive(Debug, Clone)]
pub struct BasicTileset {
    constraints: Arc<Vec<Vec<WaveFunction>>>,
    weights: Arc<Vec<f32>>,
}

impl Default for BasicTileset {
    fn default() -> Self {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum TileEdgeType {
            Air,
            Dirt,
            GrassDirt,
            DirtAir,
            DirtLeft,
            DirtRight,
            DirtTop,
            GrassDirtAir,
        }
        type T = TileEdgeType;

        let tile_edge_types = [
            [T::Air, T::Air, T::Air, T::Air],
            [T::Air, T::DirtLeft, T::Air, T::GrassDirt],
            [T::Air, T::Dirt, T::GrassDirt, T::GrassDirt],
            [T::Air, T::DirtRight, T::GrassDirt, T::Air],
            [T::DirtLeft, T::DirtLeft, T::Air, T::Dirt],
            [T::Dirt, T::Dirt, T::Dirt, T::Dirt],
            [T::DirtRight, T::DirtRight, T::Dirt, T::Air],
            [T::Air, T::Dirt, T::GrassDirt, T::DirtTop],
            [T::DirtLeft, T::Dirt, T::DirtTop, T::Dirt],
            [T::Dirt, T::Air, T::DirtAir, T::DirtAir],
            [T::DirtRight, T::Dirt, T::Dirt, T::DirtTop],
            [T::Air, T::Dirt, T::DirtTop, T::GrassDirt],
            [T::DirtLeft, T::Air, T::Air, T::DirtAir],
            [T::Air, T::Air, T::Air, T::GrassDirtAir],
            [T::Air, T::Air, T::GrassDirtAir, T::GrassDirtAir],
            [T::Air, T::Air, T::GrassDirtAir, T::Air],
            [T::DirtRight, T::Air, T::DirtAir, T::Air],
        ];

        // convert to allowed neighbors
        let mut allowed_neighbors = Vec::with_capacity(TILE_COUNT);
        for (tile, edges) in tile_edge_types.iter().enumerate() {
            let mut allowed_neighbors_for_tile = Vec::with_capacity(DIRECTIONS);
            for (edge_index, edge) in edges.into_iter().enumerate() {
                let direction = Direction::from(edge_index);
                let mut cell = WaveFunction::empty();

                if *edge == T::Air && tile != 0 {
                    // special case for air
                    cell.add_tile(0);
                } else {
                    // add all tiles with this edge type to the neighbor set
                    for (other_tile, other_edges) in tile_edge_types.iter().enumerate() {
                        if other_edges[direction.other() as usize] == *edge {
                            cell.add_tile(other_tile);
                        }
                    }
                }

                allowed_neighbors_for_tile.push(cell);
            }
            allowed_neighbors.push(allowed_neighbors_for_tile);
        }

        // dont allow tile 8 and 10 to be next to each other
        allowed_neighbors[10][Direction::Right as usize].remove_tile(8);
        allowed_neighbors[8][Direction::Left as usize].remove_tile(10);

        let mut weights = Vec::new();
        for _ in 0..TILE_COUNT {
            weights.push(1.0);
        }

        Self {
            constraints: Arc::new(allowed_neighbors),
            weights: Arc::new(weights),
        }
    }
}

impl TileSet for BasicTileset {
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

    fn get_tile_paths(&self) -> Vec<(String, Transform)> {
        let mut paths = Vec::new();
        for tile in 0..=16 {
            paths.push((format!("tileset/{}.png", tile), Transform::IDENTITY));
        }
        paths
    }
}
