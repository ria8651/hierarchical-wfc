use crate::graph_grid::{self, GridGraphSettings};
use hierarchical_wfc::{Direction, Graph, TileSet, WaveFunction};

#[derive(Default, Clone)]
pub struct BasicTileset;

impl TileSet for BasicTileset {
    type GraphSettings = GridGraphSettings;

    // const TILE_COUNT: usize = 17;
    // const DIRECTIONS: usize = 4;

    fn tile_count(&self) -> usize {
        17
    }

    fn directions(&self) -> usize {
        4
    }

    fn get_constraints(&self) -> Vec<Vec<WaveFunction>> {
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
        let mut allowed_neighbors = Vec::with_capacity(self.tile_count());
        for (tile, edges) in tile_edge_types.iter().enumerate() {
            let mut allowed_neighbors_for_tile = Vec::with_capacity(self.directions());
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

        allowed_neighbors
    }

    fn get_weights(&self) -> Vec<u32> {
        let mut weights = Vec::new();
        for _ in 0..self.tile_count() {
            weights.push(100);
        }
        weights
    }

    fn get_tile_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();
        for tile in 0..=16 {
            paths.push(format!("tileset/{}.png", tile));
        }
        paths
    }

    fn create_graph(&self, settings: &Self::GraphSettings) -> Graph<WaveFunction> {
        let cell = WaveFunction::filled(self.tile_count());
        graph_grid::create(settings, cell)
    }
}
