use crate::wfc::{
    graph_grid::{create_grid_graph, GridGraphSettings},
    Direction, Superposition, TileSet, WfcGraph,
};

#[derive(Default)]
pub struct BasicTileset;

impl TileSet for BasicTileset {
    type GraphSettings = GridGraphSettings;

    // const TILE_COUNT: usize = 17;
    // const DIRECTIONS: usize = 4;

    fn tile_count(&self) -> usize {
        17
    }

    fn arc_types(&self) -> usize {
        4
    }

    fn get_constraints(&self) -> Box<[Box<[Superposition]>]> {
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
            let mut allowed_neighbors_for_tile = Vec::with_capacity(self.arc_types());
            for (edge_index, edge) in edges.iter().enumerate() {
                let direction = Direction::from(edge_index);
                let mut cell = Superposition::empty();

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
            allowed_neighbors.push(allowed_neighbors_for_tile.into());
        }

        allowed_neighbors.into()
    }

    fn get_weights(&self) -> Vec<u32> {
        vec![100; self.tile_count()]
    }

    fn get_tile_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();
        for tile in 0..=16 {
            paths.push(format!("tileset/{}.png", tile));
        }
        paths
    }

    fn create_graph(&self, settings: &Self::GraphSettings) -> WfcGraph<Superposition> {
        let cell = Superposition::filled(self.tile_count());
        create_grid_graph(settings, cell)
    }
}
