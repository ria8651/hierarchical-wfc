use crate::{
    grid_wfc::Direction,
    tileset::{AllowedNeighbors, TileSet},
};
use bevy::utils::{HashMap, HashSet};
use rand::Rng;

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
        }
        type T = TileEdgeType;

        let tile_edge_types = [
            (0, [T::City, T::Road, T::City, T::City]),
            (1, [T::City, T::Grass, T::City, T::Grass]),
            (2, [T::City, T::Road, T::City, T::Road]),
            (3, [T::Grass, T::Grass, T::City, T::City]),
            (4, [T::City, T::Grass, T::City, T::Grass]),
            (5, [T::City, T::City, T::Grass, T::Grass]),
            (6, [T::City, T::Grass, T::Grass, T::Grass]),
            (7, [T::City, T::Road, T::Road, T::Grass]),
            (8, [T::City, T::Road, T::Grass, T::Road]),
            (9, [T::City, T::Road, T::Road, T::Road]),
            (10, [T::City, T::Grass, T::Road, T::Road]),
            (11, [T::Road, T::Road, T::Grass, T::Grass]),
            (12, [T::Grass, T::Road, T::Road, T::Grass]),
            (13, [T::Grass, T::Road, T::Road, T::Road]),
            (14, [T::Grass, T::Grass, T::Grass, T::Grass]),
            (15, [T::Grass, T::Road, T::Grass, T::Grass]),
            (16, [T::City, T::City, T::City, T::City]),
            (17, [T::City, T::Grass, T::City, T::City]),
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
                rotated_tile_edge_types.push((18 * rotation + *tile, rotated_edges));
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
        rng.gen_range(0..18 * 4)
    }

    fn all_tiles() -> HashSet<Self::Tile> {
        (0..18 * 4).collect()
    }
}
