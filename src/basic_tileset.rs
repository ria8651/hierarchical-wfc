use crate::{
    graph_wfc::{Cell, Direction},
    tileset::{AllowedNeighbors, TileSet},
};
use bevy::utils::HashMap;

pub struct BasicTileset;

impl TileSet for BasicTileset {
    const TILE_COUNT: usize = 17;
    type Direction = Direction;

    fn allowed_neighbors() -> AllowedNeighbors {
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
            (0, [T::Air, T::Air, T::Air, T::Air]),
            (1, [T::Air, T::DirtLeft, T::Air, T::GrassDirt]),
            (2, [T::Air, T::Dirt, T::GrassDirt, T::GrassDirt]),
            (3, [T::Air, T::DirtRight, T::GrassDirt, T::Air]),
            (4, [T::DirtLeft, T::DirtLeft, T::Air, T::Dirt]),
            (5, [T::Dirt, T::Dirt, T::Dirt, T::Dirt]),
            (6, [T::DirtRight, T::DirtRight, T::Dirt, T::Air]),
            (7, [T::Air, T::Dirt, T::GrassDirt, T::DirtTop]),
            (8, [T::DirtLeft, T::Dirt, T::DirtTop, T::Dirt]),
            (9, [T::Dirt, T::Air, T::DirtAir, T::DirtAir]),
            (10, [T::DirtRight, T::Dirt, T::Dirt, T::DirtTop]),
            (11, [T::Air, T::Dirt, T::DirtTop, T::GrassDirt]),
            (12, [T::DirtLeft, T::Air, T::Air, T::DirtAir]),
            (13, [T::Air, T::Air, T::Air, T::GrassDirtAir]),
            (14, [T::Air, T::Air, T::GrassDirtAir, T::GrassDirtAir]),
            (15, [T::Air, T::Air, T::GrassDirtAir, T::Air]),
            (16, [T::DirtRight, T::Air, T::DirtAir, T::Air]),
        ];

        // convert to allowed neighbors
        let mut allowed_neighbors = HashMap::new();
        for (tile, edges) in tile_edge_types {
            let mut neighbors = HashMap::new();
            for (edge_index, edge) in edges.into_iter().enumerate() {
                let direction = Direction::from(edge_index);

                if edge == T::Air && tile != 0 {
                    // special case for air
                    neighbors
                        .entry(direction)
                        .or_insert(Cell::empty())
                        .add_tile(0);
                } else {
                    // add all tiles with this edge type to the neighbor set
                    for (other_tile, other_edges) in tile_edge_types.iter() {
                        if other_edges[direction.other() as usize] == edge {
                            neighbors
                                .entry(direction)
                                .or_insert(Cell::empty())
                                .add_tile(*other_tile);
                        }
                    }
                }
            }
            allowed_neighbors.insert(tile, neighbors);
        }
        allowed_neighbors
    }

    fn get_tile_paths() -> Vec<String> {
        let mut paths = Vec::new();
        for tile in 0..=16 {
            paths.push(format!("tileset/{}.png", tile));
        }
        paths
    }
}
