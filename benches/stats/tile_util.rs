use hierarchical_wfc::{Graph, Neighbor};

pub struct Tile<'a, T: Copy> {
    pub value: T,
    pub neigbhours: &'a [Neighbor],
}

impl<'a, T: Copy> Tile<'a, T> {
    pub fn tile_in_dir(&self, graph: &'a Graph<T>, direction: usize) -> Option<Tile<'a, T>> {
        self.neigbhours
            .iter()
            .flat_map(
                |Neighbor {
                     index,
                     direction: dir,
                 }| {
                    if *dir == direction {
                        return Some(Tile {
                            value: graph.tiles[*index],
                            neigbhours: graph.neighbors[*index].as_slice(),
                        });
                    } else {
                        return None;
                    }
                },
            )
            .next()
    }
}
