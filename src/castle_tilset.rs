use super::{
    graph::{Graph, Superposition},
    graph_grid::{self, GridGraphSettings},
    tileset::TileSet,
    wfc::Direction,
};

#[derive(Debug, Default)]
pub struct CastleTileset;

impl TileSet for CastleTileset {
    type GraphSettings = GridGraphSettings;

    // const TILE_COUNT: usize = 120;
    // const DIRECTIONS: usize = 4;

    fn tile_count(&self) -> usize {
        11
    }

    fn arc_types(&self) -> usize {
        4
    }

    // 0 - wall horizontal
    // 1 - wall vertical
    // 2 - pillar
    // 3 - core

    fn get_constraints(&self) -> Vec<Vec<Superposition>> {
        fn get_horizontal(walls: &[usize]) -> Superposition {
            Superposition::from_iter(walls.to_owned().into_iter())
        }
        fn get_vertical(walls: &[usize]) -> Superposition {
            Superposition::from_iter(walls.to_owned().into_iter().map(|i| i + 1))
        }
        // n   : horizontal
        // n+1 : vertical
        // let mut index = 0;
        // let short_walls: [usize; 4] = [
        //     0, // Full
        //     2, // Slit
        //     4, // Window
        //     6, // Arch
        // ];
        // let tall_walls: [usize; 2] = [
        //     8,  // Full
        //     10, // Entrance
        // ];
        // let short_pilar: usize = 11;
        // let tall_pilar: usize = 12;
        // let beam_connector: usize = 13;
        // let open_space: usize = 14;

        // let mut allowed: Vec<Vec<Superposition>> = Vec::with_capacity(14);

        // for i in short_walls.clone() {
        //     allowed.push(vec![
        //         Superposition::single(open_space),
        //         Superposition::single(open_space),
        //         Superposition::single(short_pilar),
        //         Superposition::single(short_pilar),
        //     ]);
        //     allowed.push(vec![
        //         Superposition::single(short_pilar),
        //         Superposition::single(short_pilar),
        //         Superposition::single(open_space),
        //         Superposition::single(open_space),
        //     ]);
        // }

        // for i in tall_walls.clone() {
        //     allowed.push(vec![
        //         Superposition::single(open_space),
        //         Superposition::single(open_space),
        //         Superposition::single(tall_pilar),
        //         Superposition::single(tall_pilar),
        //     ]);
        //     allowed.push(vec![
        //         Superposition::single(tall_pilar),
        //         Superposition::single(tall_pilar),
        //         Superposition::single(open_space),
        //         Superposition::single(open_space),
        //     ]);
        // }

        // // Short pillar
        // allowed.push(vec![
        //     get_vertical(&short_walls),
        //     get_vertical(&short_walls),
        //     get_horizontal(&short_walls),
        //     get_horizontal(&short_walls),
        // ]);

        // // Tall pillar
        // allowed.push(vec![
        //     get_vertical(&tall_walls),
        //     get_vertical(&tall_walls),
        //     get_horizontal(&tall_walls),
        //     get_horizontal(&tall_walls),
        // ]);

        // // Beam conector
        // allowed.push(vec![
        //     Superposition::empty(),
        //     Superposition::empty(),
        //     Superposition::empty(),
        //     Superposition::empty(),
        // ]);

        // // Open space
        // allowed.push(vec![
        //     Superposition::join(&get_horizontal(&short_walls), &get_horizontal(&tall_walls)),
        //     Superposition::join(&get_horizontal(&short_walls), &get_horizontal(&tall_walls)),
        //     Superposition::join(&get_vertical(&short_walls), &get_vertical(&tall_walls)),
        //     Superposition::join(&get_vertical(&short_walls), &get_vertical(&tall_walls)),
        // ]);
        // return allowed;

        let mut allowed: Vec<Vec<Superposition>> = Vec::new();

        let short_walls: [usize; 3] = [0, 2, 4];
        let short_pilar: usize = 6;
        let open_space: usize = 7;

        let open_pillar: usize = 8;
        let open_wall: usize = 9;

        for i in short_walls.clone() {
            allowed.push(vec![
                Superposition::single(open_space),
                Superposition::single(open_space),
                Superposition::single(short_pilar),
                Superposition::single(short_pilar),
            ]);
            allowed.push(vec![
                Superposition::single(short_pilar),
                Superposition::single(short_pilar),
                Superposition::single(open_space),
                Superposition::single(open_space),
            ]);
        }

        // Short pilar
        allowed.push(vec![
            get_vertical(&short_walls) + (open_wall + 1),
            get_vertical(&short_walls) + (open_wall + 1),
            get_horizontal(&short_walls) + open_wall,
            get_horizontal(&short_walls) + open_wall,
        ]);

        // Open space
        allowed.push(vec![
            get_horizontal(&short_walls) + open_wall,
            get_horizontal(&short_walls) + open_wall,
            get_vertical(&short_walls) + (open_wall + 1),
            get_vertical(&short_walls) + (open_wall + 1),
        ]);

        // Open pilar
        allowed.push(vec![
            Superposition::single(open_wall + 1),
            Superposition::single(open_wall + 1),
            Superposition::single(open_wall),
            Superposition::single(open_wall),
        ]);

        // Open wall
        allowed.push(vec![
            Superposition::single(open_space),
            Superposition::single(open_space),
            Superposition::from_iter([short_pilar, open_pillar].into_iter()),
            Superposition::from_iter([short_pilar, open_pillar].into_iter()),
        ]);
        allowed.push(vec![
            Superposition::from_iter([short_pilar, open_pillar].into_iter()),
            Superposition::from_iter([short_pilar, open_pillar].into_iter()),
            Superposition::single(open_space),
            Superposition::single(open_space),
        ]);

        dbg!(&allowed);
        return allowed;
        // return vec![
        //     vec![
        //         Superposition::from_iter([3].into_iter()),
        //         Superposition::from_iter([3].into_iter()),
        //         Superposition::from_iter([2].into_iter()),
        //         Superposition::from_iter([2].into_iter()),
        //     ],
        //     vec![
        //         Superposition::from_iter([2].into_iter()),
        //         Superposition::from_iter([2].into_iter()),
        //         Superposition::from_iter([3].into_iter()),
        //         Superposition::from_iter([3].into_iter()),
        //     ],
        //     vec![
        //         Superposition::from_iter([1].into_iter()),
        //         Superposition::from_iter([1].into_iter()),
        //         Superposition::from_iter([0].into_iter()),
        //         Superposition::from_iter([0].into_iter()),
        //     ],
        //     vec![
        //         Superposition::from_iter([0].into_iter()),
        //         Superposition::from_iter([0].into_iter()),
        //         Superposition::from_iter([1].into_iter()),
        //         Superposition::from_iter([1].into_iter()),
        //     ],
        // ];
        // allowed
    }

    fn get_weights(&self) -> Vec<u32> {
        let mut weights = Vec::with_capacity(self.tile_count());
        for _ in 0..self.tile_count() {
            weights.push(100);
        }
        weights
    }

    fn get_tile_paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = Vec::new();
        // for tile in 0..self.tile_count() / 4 {
        //     paths.push(format!(
        //         "gltf/castle/{}.gltf",
        //         match tile {
        //             0 => "p-short",
        //             1 => "w-short",
        //             _ => "",
        //         }
        //     ));
        // }
        paths
    }

    fn create_graph(&self, settings: &Self::GraphSettings) -> Graph<Superposition> {
        let cell = Superposition::filled(self.tile_count());
        graph_grid::create(settings, cell)
    }
}
