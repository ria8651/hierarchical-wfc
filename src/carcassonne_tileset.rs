use crate::{
    graph::{Cell, Graph},
    graph_grid::{self, GridGraphSettings},
    tileset::TileSet,
    wfc::Direction,
};

#[derive(Debug)]
pub struct CarcassonneTileset {
    constraints: [[Cell; Self::DIRECTIONS]; Self::TILE_COUNT],
}

#[allow(dead_code)]
impl CarcassonneTileset {
    pub fn new() -> Self {
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
        let mut rotated_tile_edge_types = [[T::Grass; Self::DIRECTIONS]; Self::TILE_COUNT];
        for rotation in 0..4 {
            for (tile, edges) in tile_edge_types.iter().enumerate() {
                let mut rotated_edges = [T::Grass, T::Grass, T::Grass, T::Grass];
                for (edge_index, edge) in edges.iter().enumerate() {
                    let direction = Direction::from(edge_index);
                    rotated_edges[direction.rotate(rotation) as usize] = *edge;
                }
                rotated_tile_edge_types[Self::TILE_COUNT / 4 * rotation + tile] = rotated_edges;
            }
        }

        // convert to allowed neighbors
        let mut allowed_neighbors = [[Cell::empty(); Self::DIRECTIONS]; Self::TILE_COUNT];
        for (tile, edges) in rotated_tile_edge_types.iter().enumerate() {
            for (edge_index, edge) in edges.into_iter().enumerate() {
                let direction = Direction::from(edge_index);

                // add all tiles with this edge type to the neighbor set
                for (other_tile, other_edges) in rotated_tile_edge_types.iter().enumerate() {
                    if other_edges[direction.other() as usize] == *edge {
                        allowed_neighbors[tile][edge_index].add_tile(other_tile);
                    }
                }
            }
        }

        Self {
            constraints: allowed_neighbors,
        }
    }
}

impl TileSet for CarcassonneTileset {
    type GraphSettings = GridGraphSettings;

    const TILE_COUNT: usize = 120;
    const DIRECTIONS: usize = 4;

    fn get_constraints(&self) -> &[[Cell; Self::DIRECTIONS]; Self::TILE_COUNT] {
        &self.constraints
    }

    fn get_tile_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();
        for tile in 0..Self::TILE_COUNT / 4 {
            paths.push(format!("carcassonne/{}.png", tile));
        }
        paths
    }

    fn create_graph(&self, settings: &Self::GraphSettings) -> Graph<Cell> {
        graph_grid::create::<Self>(settings)
    }
}
