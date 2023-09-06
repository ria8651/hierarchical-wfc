use crate::{
    graph::{Cell, Graph},
    graph_grid::GridGraphSettings,
    graph_grid_8D::{self, Direction8D},
    tileset::TileSet,
};

#[derive(Debug, Default)]
pub struct HierarchicalTileset;

impl HierarchicalTileset {
    const ROTATED_TILES: [usize; 1] = [1];
}

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
        enum EdgeType {
            Any,
            Ocean,
            Sand,
            OceanShore,
            ShoreSand,
        }

        fn edge_to_tiles(edge: EdgeType) -> Vec<TileType> {
            match edge {
                EdgeType::Any => vec![TileType::Ocean, TileType::Shore, TileType::Sand],
                EdgeType::Ocean => vec![TileType::Ocean],
                EdgeType::Sand => vec![TileType::Sand],
                EdgeType::OceanShore => vec![TileType::Ocean, TileType::Shore],
                EdgeType::ShoreSand => vec![TileType::Shore, TileType::Sand],
            }
        }
        fn tile_to_indices(tile_type: TileType) -> Vec<usize> {
            match tile_type {
                TileType::Ocean => vec![0],
                TileType::Shore => (1..=8).collect(),
                TileType::Sand => vec![9],
            }
        }
        fn index_to_tile(index: usize) -> TileType {
            match index {
                0 => TileType::Ocean,
                1..=8 => TileType::Shore,
                9 => TileType::Sand,
                _ => panic!("Invalid tile index"),
            }
        }

        let tile_edge_types = [
            [
                EdgeType::OceanShore,
                EdgeType::OceanShore,
                EdgeType::OceanShore,
                EdgeType::OceanShore,
                EdgeType::OceanShore,
                EdgeType::OceanShore,
                EdgeType::OceanShore,
                EdgeType::OceanShore,
            ],
            [
                EdgeType::Ocean,
                EdgeType::OceanShore,
                EdgeType::Any,
                EdgeType::ShoreSand,
                EdgeType::Sand,
                EdgeType::ShoreSand,
                EdgeType::Any,
                EdgeType::OceanShore,
            ],
            [
                EdgeType::ShoreSand,
                EdgeType::ShoreSand,
                EdgeType::ShoreSand,
                EdgeType::ShoreSand,
                EdgeType::ShoreSand,
                EdgeType::ShoreSand,
                EdgeType::ShoreSand,
                EdgeType::ShoreSand,
            ],
        ];

        // rotate all tiles to get all possible edge types
        let mut rotated_tile_edge_types = Vec::with_capacity(self.tile_count());
        for (edges_index, edges) in tile_edge_types.iter().enumerate() {
            if HierarchicalTileset::ROTATED_TILES.contains(&edges_index) {
                for rotation in 0..self.directions() {
                    let mut rotated_edges = vec![EdgeType::Any; self.directions()];
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

        // Convert to allowed neighbors
        let mut allowed_neighbors = Vec::with_capacity(self.tile_count());

        for (tile_idx, edges) in rotated_tile_edge_types.iter().enumerate() {
            let mut allowed_neighbors_for_tile = Vec::with_capacity(self.directions());

            for (dir_idx, edge) in edges.iter().enumerate() {
                let dir = Direction8D::from(dir_idx);
                let tile_types = edge_to_tiles(*edge);

                let mut cell = Cell::empty();
                for tile_type in &tile_types {
                    let indices = tile_to_indices(*tile_type);
                    for idx in indices {
                        // Use rotated_tile_edge_types to determine if the neighboring tile also accepts
                        // the current tile as its neighbor in the opposite direction.
                        let other_edge = rotated_tile_edge_types[idx][dir.other() as usize];
                        if edge_to_tiles(other_edge).contains(&index_to_tile(tile_idx)) {
                            cell.add_tile(idx);
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
        let mut weights = Vec::with_capacity(self.tile_count());
        weights.push(100);
        for _ in 0..HierarchicalTileset::ROTATED_TILES.len() * self.directions() {
            weights.push(100 / self.directions() as u32);
        }
        weights.push(100);
        weights
    }

    fn get_tile_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();
        paths.push("hierarchical/layer0/0.png".to_string());
        for _ in 0..self.directions() {
            paths.push("hierarchical/layer0/1.png".to_string());
        }
        paths.push("hierarchical/layer0/2.png".to_string());
        paths
    }

    fn create_graph(&self, settings: &Self::GraphSettings) -> Graph<Cell> {
        let cell = Cell::filled(self.tile_count());
        let mut graph = graph_grid_8D::create(settings, cell);

        // // Fill boundaries of graph with ocean
        // let mut ocean_cell = Cell::empty();
        // ocean_cell.add_tile(0);

        // // 1. Fill the top edge
        // for i in 0..settings.width {
        //     graph.tiles[i] = ocean_cell;
        // }

        // // 2. Fill the bottom edge
        // let start_of_bottom_edge = settings.width * (settings.height - 1);
        // for i in start_of_bottom_edge..(settings.width * settings.height) {
        //     graph.tiles[i] = ocean_cell;
        // }

        // // 3. Fill the left edge
        // for i in (0..(settings.width * settings.height)).step_by(settings.width) {
        //     graph.tiles[i] = ocean_cell;
        // }

        // // 4. Fill the right edge
        // for i in (settings.width - 1..(settings.width * settings.height)).step_by(settings.width) {
        //     graph.tiles[i] = ocean_cell;
        // }

        graph
    }
}
