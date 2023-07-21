use bevy::utils::{HashMap, HashSet};
use rand::Rng;

use crate::grid_wfc::TileSet;

pub struct BasicTileset;

impl TileSet for BasicTileset {
    type Tile = u32;

    fn allowed_neighbors() -> HashMap<Self::Tile, [HashSet<Self::Tile>; 4]> {
        [
            (
                0,
                [
                    [0].into(),
                    [0, 1, 2, 3].into(),
                    [0, 3, 6].into(),
                    [0, 1, 4].into(),
                ],
            ),
            (1, [[0].into(), [4].into(), [0].into(), [2, 3].into()]),
            (2, [[0].into(), [5].into(), [1, 2].into(), [2, 3].into()]),
            (3, [[0].into(), [6].into(), [1, 2].into(), [0].into()]),
            (4, [[1, 4].into(), [4].into(), [0].into(), [5, 6].into()]),
            (5, [[2, 5].into(), [5].into(), [4, 5].into(), [5, 6].into()]),
            (6, [[3, 6].into(), [6].into(), [5, 4].into(), [0].into()]),
        ]
        .into()
    }

    fn random_tile<R: Rng>(rng: &mut R) -> Self::Tile {
        rng.gen_range(0..7)
    }

    fn all_tiles() -> HashSet<Self::Tile> {
        [0, 1, 2, 3, 4, 5, 6].into()
    }
}
