use crate::{
    graph_wfc::Direction,
    tileset::{AllowedNeighbors, TileSet},
};
use bevy::utils::{HashMap, HashSet};
use rand::Rng;

const NUM_TILES: u32 = 30;

#[derive(Debug)]
pub struct CarcassonneTileset;

impl TileSet for CarcassonneTileset {
    type Tile = u32;
    type Direction = Direction;

    fn allowed_neighbors() -> AllowedNeighbors<Self> {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum TileEdgeType {
            Grass,
            Road,
            City,
            River,
        }
        type T = TileEdgeType;

        let tile_edge_types = [
            (0, [T::Grass, T::Road, T::Road, T::Grass]),
            (1, [T::City, T::Road, T::City, T::City]),
            (2, [T::City, T::Grass, T::City, T::Grass]),
            (3, [T::City, T::Road, T::City, T::Road]),
            (4, [T::Grass, T::Grass, T::City, T::City]),
            (5, [T::City, T::Grass, T::City, T::Grass]),
            (6, [T::City, T::City, T::Grass, T::Grass]),
            (7, [T::City, T::Grass, T::Grass, T::Grass]),
            (8, [T::City, T::Road, T::Road, T::Grass]),
            (9, [T::City, T::Road, T::Grass, T::Road]),
            (10, [T::City, T::Road, T::Road, T::Road]),
            (11, [T::City, T::Grass, T::Road, T::Road]),
            (12, [T::Road, T::Road, T::Grass, T::Grass]),
            (13, [T::Grass, T::Road, T::Road, T::Grass]),
            (14, [T::Grass, T::Road, T::Road, T::Road]),
            (15, [T::Grass, T::Grass, T::Grass, T::Grass]),
            (16, [T::Grass, T::Road, T::Grass, T::Grass]),
            (17, [T::City, T::City, T::City, T::City]),
            (18, [T::City, T::Grass, T::City, T::City]),
            (19, [T::Grass, T::River, T::Grass, T::Grass]),
            (20, [T::Grass, T::River, T::Grass, T::River]),
            (21, [T::Grass, T::Road, T::River, T::River]),
            (22, [T::Road, T::River, T::River, T::Road]),
            (23, [T::River, T::River, T::Grass, T::Grass]),
            (24, [T::River, T::River, T::Grass, T::Grass]),
            (25, [T::River, T::Grass, T::Grass, T::Grass]),
            (26, [T::River, T::River, T::Road, T::City]),
            (27, [T::City, T::City, T::River, T::River]),
            (28, [T::Road, T::Road, T::River, T::River]),
            (29, [T::River, T::City, T::River, T::City]),
        ];

        // rotate all tiles to get all possible edge types
        let mut rotated_tile_edge_types: Vec<(u32, [TileEdgeType; 4])> = Vec::new();
        for rotation in 0..4 {
            let bleh = match rotation {
                0 => [0, 1, 2, 3],
                1 => [2, 3, 1, 0],
                2 => [1, 0, 3, 2],
                3 => [3, 2, 0, 1],
                _ => unreachable!(),
            };

            for (tile, edges) in tile_edge_types.iter() {
                let mut rotated_edges = [T::Grass, T::Grass, T::Grass, T::Grass];
                for (edge_index, edge) in edges.iter().enumerate() {
                    rotated_edges[bleh[edge_index]] = *edge;
                }
                rotated_tile_edge_types.push((NUM_TILES * rotation + *tile, rotated_edges));
            }
        }

        // convert to allowed neighbors
        let mut allowed_neighbors: AllowedNeighbors<Self> = HashMap::new();
        for (tile, edges) in rotated_tile_edge_types.clone() {
            let mut neighbors = HashMap::new();
            for (edge_index, edge) in edges.into_iter().enumerate() {
                let direction = Direction::from(edge_index);

                // add all tiles with this edge type to the neighbor set
                for (other_tile, other_edges) in rotated_tile_edge_types.iter() {
                    if other_edges[direction.other() as usize] == edge {
                        neighbors
                            .entry(direction)
                            .or_insert(HashSet::new())
                            .insert(*other_tile);
                    }
                }
            }
            allowed_neighbors.insert(tile, neighbors);
        }
        allowed_neighbors
    }

    fn random_tile<R: Rng>(rng: &mut R) -> Self::Tile {
        rng.gen_range(0..NUM_TILES * 4)
    }

    fn all_tiles() -> HashSet<Self::Tile> {
        (0..NUM_TILES * 4).collect()
    }

    fn get_tile_paths() -> Vec<String> {
        let mut paths = Vec::new();
        for tile in 0..NUM_TILES {
            paths.push(format!("carcassonne/{}.png", tile));
        }
        paths
    }
}
