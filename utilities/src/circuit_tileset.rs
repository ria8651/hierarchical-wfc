use crate::graph_grid::{self, Direction, GridGraphSettings};
use hierarchical_wfc::{Graph, TileSet, WaveFunction};

#[derive(Debug, Default, Clone)]
pub struct CircuitTileset;

impl TileSet for CircuitTileset {
    type GraphSettings = GridGraphSettings;

    fn tile_count(&self) -> usize {
        14 * 4
    }

    fn directions(&self) -> usize {
        4
    }

    fn get_constraints(&self) -> Vec<Vec<WaveFunction>> {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum TileEdgeType {
            Component,
            ComponentPcb,
            Bridge,
            Tract,
            Pcb,
        }
        type T = TileEdgeType;

        let tile_edge_types = [
            [T::Tract, T::Tract, T::Bridge, T::Bridge],
            [T::Component, T::Component, T::Component, T::Component],
            [T::Tract, T::Component, T::ComponentPcb, T::ComponentPcb],
            [T::Pcb, T::ComponentPcb, T::ComponentPcb, T::Pcb],
            [T::Tract, T::Tract, T::Tract, T::Tract],
            [T::Tract, T::Pcb, T::Pcb, T::Tract],
            [T::Pcb, T::Pcb, T::Pcb, T::Pcb],
            [T::Pcb, T::Tract, T::Tract, T::Tract],
            [T::Tract, T::Tract, T::Pcb, T::Pcb],
            [T::Bridge, T::Tract, T::Pcb, T::Pcb],
            [T::Tract, T::Pcb, T::Pcb, T::Tract],
            [T::Pcb, T::Pcb, T::Tract, T::Tract],
            [T::Tract, T::Pcb, T::Pcb, T::Pcb],
            [T::Pcb, T::Pcb, T::Bridge, T::Bridge],
        ];

        // rotate all tiles to get all possible edge types
        let mut rotated_tile_edge_types = Vec::with_capacity(self.tile_count());
        for rotation in 0..4 {
            for edges in tile_edge_types.iter() {
                let mut rotated_edges = vec![T::Component; self.directions()];
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

    fn get_weights(&self) -> Vec<u32> {
        let mut weights = Vec::with_capacity(self.tile_count());
        for _ in 0..self.tile_count() {
            weights.push(100);
        }
        weights
    }

    fn get_tile_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();
        for tile in 0..self.tile_count() / 4 {
            paths.push(format!("circuit/{}.png", tile));
        }
        paths
    }

    fn create_graph(&self, settings: &Self::GraphSettings) -> Graph<WaveFunction> {
        let cell = WaveFunction::filled(self.tile_count());
        graph_grid::create(settings, cell)
    }
}
