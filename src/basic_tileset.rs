use crate::{
    graph_wfc::{Cell, Direction},
    tileset::TileSet,
};

pub struct BasicTileset;

impl TileSet for BasicTileset {
    const TILE_COUNT: usize = 17;
    const DIRECTIONS: usize = 4;

    fn allowed_neighbors() -> [[Cell; Self::DIRECTIONS]; Self::TILE_COUNT] {
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
        let mut allowed_neighbors = [[Cell::empty(); Self::DIRECTIONS]; Self::TILE_COUNT];
        for (tile, edges) in tile_edge_types.iter().enumerate() {
            for (edge_index, edge) in edges.into_iter().enumerate() {
                let direction = Direction::from(edge_index);

                if *edge == T::Air && tile != 0 {
                    // special case for air
                    allowed_neighbors[tile][edge_index].add_tile(0);
                } else {
                    // add all tiles with this edge type to the neighbor set
                    for (other_tile, other_edges) in tile_edge_types.iter().enumerate() {
                        if other_edges[direction.other() as usize] == *edge {
                            allowed_neighbors[tile][edge_index].add_tile(other_tile);
                        }
                    }
                }
            }
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
