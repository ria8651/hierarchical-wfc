use crate::graph_grid::{self, GridGraphSettings};
use bevy::utils::HashMap;
use hierarchical_wfc::{Graph, TileSet, WaveFunction};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Default, Clone)]
pub struct MxgmnTileset {
    tile_count: usize,
    constraints: Vec<Vec<WaveFunction>>,
    weights: Vec<f32>,
    tile_paths: Vec<String>,
}

impl MxgmnTileset {
    pub fn new(path: String) -> Self {
        let path = Path::new(&path);
        let name = path.file_stem().unwrap().to_str().unwrap();
        let image_folder = path.parent().unwrap().join(name);

        let xml = std::fs::read_to_string(path).unwrap();
        let config: Config = serde_xml_rs::from_str(&xml).unwrap();

        let tile_count = config.tiles.tile.len();

        let mut tile_ids = HashMap::new();
        let mut weights = Vec::new();
        let mut tile_paths = Vec::new();
        for (i, tile) in config.tiles.tile.iter().enumerate() {
            tile_ids.insert(tile.name.clone(), i);
            weights.push(tile.weight);

            let path = image_folder.join(&format!("{}.png", tile.name));
            tile_paths.push(path.to_str().unwrap().to_string());
        }

        let inner = vec![
            WaveFunction::filled(tile_count),
            WaveFunction::filled(tile_count),
            WaveFunction::empty(),
            WaveFunction::empty(),
        ];
        let mut constraints = vec![inner; tile_count];
        for neighbor in config.neighbors.neighbor.iter() {
            if !tile_ids.contains_key(&neighbor.left) {
                continue;
            }
            if !tile_ids.contains_key(&neighbor.right) {
                continue;
            }

            let left = tile_ids[&neighbor.left];
            let right = tile_ids[&neighbor.right];
            constraints[left][3].add_tile(right);
            constraints[right][2].add_tile(left);
            match config.tiles.tile[left].symmetry.as_str() {
                "X" | "I" | "T" => {
                    constraints[left][2].add_tile(right);
                }
                _ => {}
            }
            match config.tiles.tile[right].symmetry.as_str() {
                "X" | "I" | "T" => {
                    constraints[right][3].add_tile(left);
                }
                _ => {}
            }
        }

        Self {
            tile_count,
            constraints,
            weights,
            tile_paths,
        }
    }
}

impl TileSet for MxgmnTileset {
    type GraphSettings = GridGraphSettings;

    fn tile_count(&self) -> usize {
        self.tile_count
    }

    fn directions(&self) -> usize {
        4
    }

    fn get_constraints(&self) -> Vec<Vec<WaveFunction>> {
        self.constraints.clone()
    }

    fn get_weights(&self) -> Vec<f32> {
        self.weights.clone()
    }

    fn get_tile_paths(&self) -> Vec<String> {
        self.tile_paths.clone()
    }

    fn create_graph(&self, settings: &Self::GraphSettings) -> Graph<WaveFunction> {
        let cell = WaveFunction::filled(self.tile_count());
        graph_grid::create(settings, cell)
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename = "set")]
struct Config {
    tiles: Tiles,
    neighbors: Neighbors,
    subsets: Subsets,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Tiles {
    tile: Vec<Tile>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Tile {
    name: String,
    symmetry: String,
    weight: f32,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Neighbors {
    neighbor: Vec<Neighbor>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Neighbor {
    left: String,
    right: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Subsets {
    subset: Vec<Subset>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Subset {
    name: String,
    tile: Vec<TileName>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename = "tile")]
struct TileName {
    name: String,
}
