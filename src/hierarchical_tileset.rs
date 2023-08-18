use crate::{
    graph::{Cell, Graph},
    graph_grid::GridGraphSettings,
    graph_grid_8D::{create, Direction8D},
    tileset::TileSet,
};

#[derive(Debug, Default)]
pub struct HierarchicalTileset;

impl TileSet for HierarchicalTileset {
    type GraphSettings = GridGraphSettings;

    fn tile_count(&self) -> usize {
        2 + 1 * 8
    }

    fn directions(&self) -> usize {
        8
    }

    fn get_constraints(&self) -> Vec<Vec<Cell>> {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum TileType {
            Ocean,
            Shore,
            Sand,
        }
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum TileEdgeType {
            Any,
            Ocean,
            Shore,
            Sand,
            OceanShore,
            ShoreSand,
        }
        type T = TileEdgeType;

        let tile_edge_types = [
            [
                T::OceanShore,
                T::OceanShore,
                T::OceanShore,
                T::OceanShore,
                T::OceanShore,
                T::OceanShore,
                T::OceanShore,
                T::OceanShore,
            ],
            [
                T::Ocean,
                T::OceanShore,
                T::Any,
                T::ShoreSand,
                T::Sand,
                T::ShoreSand,
                T::Any,
                T::OceanShore,
            ],
            [
                T::ShoreSand,
                T::ShoreSand,
                T::ShoreSand,
                T::ShoreSand,
                T::ShoreSand,
                T::ShoreSand,
                T::ShoreSand,
                T::ShoreSand,
            ],
        ];

        let tiles_to_rotate = [1];

        // rotate all tiles to get all possible edge types
        let mut rotated_tile_edge_types = Vec::with_capacity(self.tile_count());
        for (edges_index, edges) in tile_edge_types.iter().enumerate() {
            if tiles_to_rotate.contains(&edges_index) {
                for rotation in 0..self.directions() {
                    let mut rotated_edges = vec![T::Any; self.directions()];
                    for (edge_index, edge) in edges.iter().enumerate() {
                        let direction = Direction8D::from(edge_index);
                        rotated_edges[direction.rotate(rotation) as usize] = *edge;
                    }
                    rotated_tile_edge_types.push(rotated_edges);
                }
            } else {
                rotated_tile_edge_types.push(edges.to_vec());
            }
        }

        // convert to allowed neighbors
        // %TODO: this code has not been yet converted to the new graph_grid_8D
        let mut allowed_neighbors = Vec::with_capacity(self.tile_count());
        for edges in rotated_tile_edge_types.iter() {
            let mut allowed_neighbors_for_tile = Vec::with_capacity(self.directions());
            for (edge_index, edge) in edges.into_iter().enumerate() {
                let direction = Direction8D::from(edge_index);
                let mut cell = Cell::empty();

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
            paths.push(format!("carcassonne/{}.png", tile));
        }
        paths
    }

    fn create_graph(&self, settings: &Self::GraphSettings) -> Graph<Cell> {
        let cell = Cell::filled(self.tile_count());
        graph_grid_8D::create(settings, cell)
    }
}
