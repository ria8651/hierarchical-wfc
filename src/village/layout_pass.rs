use super::layout_graph::LayoutGraphSettings;
use super::{
    super::{
        graph::{Graph, Superposition},
        tileset::TileSet,
    },
    layout_graph,
};

#[derive(Debug, Default)]
pub struct LayoutTileset;

impl TileSet for LayoutTileset {
    type GraphSettings = LayoutGraphSettings;

    // const TILE_COUNT: usize = 120;
    // const DIRECTIONS: usize = 4;

    fn tile_count(&self) -> usize {
        2
    }

    fn arc_types(&self) -> usize {
        3
    }

    fn get_constraints(&self) -> Vec<Vec<Superposition>> {
        fn get_horizontal(walls: &[usize]) -> Superposition {
            Superposition::from_iter(walls.to_owned().into_iter())
        }
        fn get_vertical(walls: &[usize]) -> Superposition {
            Superposition::from_iter(walls.to_owned().into_iter().map(|i| i + 1))
        }

        let mut allowed: Vec<Vec<Superposition>> = Vec::new();
        let air = 0;
        let building = 1;

        // Air
        allowed.push(vec![
            Superposition::from_iter([air, building].into_iter()),
            Superposition::from_iter([air, building].into_iter()),
            Superposition::from_iter([air].into_iter()),
        ]);

        // Building
        allowed.push(vec![
            Superposition::from_iter([building].into_iter()),
            Superposition::from_iter([air, building].into_iter()),
            Superposition::from_iter([air, building].into_iter()),
        ]);
        dbg!(&allowed);
        return allowed;
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

        paths
    }

    fn create_graph(&self, settings: &Self::GraphSettings) -> Graph<Superposition> {
        let cell = Superposition::filled(self.tile_count());
        layout_graph::create(settings, cell)
    }
}
