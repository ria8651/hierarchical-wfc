use grid_wfc::{
    basic_tileset::BasicTileset, carcassonne_tileset::CarcassonneTileset,
    mxgmn_tileset::MxgmnTileset,
};
use hierarchical_wfc::TileSet;
use rand::Rng;
use std::sync::Arc;

const ITERATIONS: usize = 1000;
const MAP_SIZE: usize = 64;

fn main() {
    let tilesets = load_tilesets();

    let mut rng = rand::thread_rng();
    let mut seed: u64 = rng.gen();

    for (tileset, tileset_name) in tilesets {}
}

pub fn load_tilesets() -> Vec<(Arc<dyn TileSet>, String)> {
    // load tilesets
    let mut tile_sets: Vec<(Arc<dyn TileSet>, String)> = vec![
        (
            Arc::new(CarcassonneTileset::default()),
            "CarcassonneTileset".to_string(),
        ),
        (
            Arc::new(BasicTileset::default()),
            "BasicTileset".to_string(),
        ),
    ];

    let paths = std::fs::read_dir("assets/mxgmn").unwrap();
    for path in paths {
        let path = path.unwrap().path();
        if let Some(ext) = path.extension() {
            if ext == "xml" {
                let name = path.file_stem().unwrap();
                if name == "Castle" || name == "FloorPlan" || name == "Summer" || name == "Circuit"
                {
                    tile_sets.push((
                        Arc::new(MxgmnTileset::new(&path, None).unwrap()),
                        path.file_stem().unwrap().to_str().unwrap().to_string(),
                    ));
                }
            }
        }
    }

    tile_sets
}
