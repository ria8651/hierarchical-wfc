use crate::graph_grid::{self, Direction, GridGraphSettings};
use anyhow::Result;
use bevy::{prelude::*, utils::HashMap};
use hierarchical_wfc::{Graph, TileSet, WaveFunction};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone)]
pub struct MxgmnTileset {
    tile_count: usize,
    constraints: Vec<Vec<WaveFunction>>,
    weights: Vec<f32>,
    tile_paths: Vec<(String, Transform)>,
}

impl MxgmnTileset {
    // based off https://github.com/mxgmn/WaveFunctionCollapse/blob/master/SimpleTiledModel.cs
    pub fn new(path: &Path, subset_name: Option<String>) -> Result<Self> {
        let name = path.file_stem().unwrap().to_str().unwrap();
        let binding = path.parent().unwrap().join(name);
        let mut image_folder: Vec<_> = binding.components().collect();
        image_folder.remove(0);
        let image_folder: PathBuf = image_folder.iter().collect();

        let xml = std::fs::read_to_string(path).unwrap();
        let config: Config = serde_xml_rs::from_str(&xml).unwrap();

        let mut subset: Vec<String> = config.tiles.tile.iter().map(|t| t.name.clone()).collect();
        if let Some(subset_name) = subset_name {
            let subsets = config.subsets.ok_or(anyhow::anyhow!("subset not found"))?;
            let config_subset = subsets
                .subset
                .iter()
                .find(|s| s.name == subset_name)
                .ok_or(anyhow::anyhow!("subset not found"))?;
            subset = config_subset.tile.iter().map(|t| t.name.clone()).collect();
        }

        let mut action: Vec<Vec<usize>> = Vec::new();
        let mut first_occurrence = HashMap::new();
        let mut weights = Vec::new();
        let mut tile_paths = Vec::new();
        for tile in config.tiles.tile.iter() {
            if !subset.contains(&tile.name) {
                continue;
            }

            let base = action.len();
            first_occurrence.insert(tile.name.clone(), base);

            let (cardinality, a, b): (
                usize,
                Box<dyn Fn(usize) -> usize>,
                Box<dyn Fn(usize) -> usize>,
            ) = match tile.symmetry {
                'L' => (
                    4,
                    Box::new(|i| (i + 1) % 4),
                    Box::new(|i| if i % 2 == 0 { i + 1 } else { i - 1 }),
                ),
                'T' => (
                    4,
                    Box::new(|i| (i + 1) % 4),
                    Box::new(|i| if i % 2 == 0 { i } else { 4 - i }),
                ),
                'I' => (
                    2, //
                    Box::new(|i| 1 - i),
                    Box::new(|i| i),
                ),
                '\\' => (
                    2, //
                    Box::new(|i| 1 - i),
                    Box::new(|i| 1 - i),
                ),
                'F' => (
                    8,
                    Box::new(|i| if i < 4 { (i + 1) % 4 } else { 4 + (i - 1) % 4 }),
                    Box::new(|i| if i < 4 { i + 4 } else { i - 4 }),
                ),
                'X' => (
                    1, //
                    Box::new(|i| i),
                    Box::new(|i| i),
                ),
                _ => unreachable!("unsupported symmetry"),
            };

            for t in 0..cardinality {
                let mut map = vec![base; 8];

                map[0] += t;
                map[1] += a(t);
                map[2] += a(a(t));
                map[3] += a(a(a(t)));
                map[4] += b(t);
                map[5] += b(a(t));
                map[6] += b(a(a(t)));
                map[7] += b(a(a(a(t))));

                action.push(map);

                if config.unique {
                    let path = image_folder.join(&format!("{} {}.png", tile.name, t));
                    tile_paths.push((path.to_str().unwrap().to_string(), Transform::IDENTITY));
                } else {
                    let path = image_folder.join(&format!("{}.png", tile.name));
                    let transform = Transform::from_rotation(Quat::from_rotation_z(
                        std::f32::consts::PI / 2.0 * t as f32,
                    ))
                    .with_scale(Vec3::new(
                        if t >= 4 { -1.0 } else { 1.0 },
                        1.0,
                        1.0,
                    ));
                    tile_paths.push((path.to_str().unwrap().to_string(), transform));
                }
                weights.push(tile.weight);
            }
        }

        let tile_count = action.len();
        let mut constraints = vec![vec![WaveFunction::empty(); 4]; tile_count];
        for neighbor in config.neighbors.neighbor.iter() {
            let mut left = neighbor.left.split(" ");
            let mut right = neighbor.right.split(" ");
            let left = (
                left.next().unwrap(),
                left.next()
                    .and_then(|f| f.parse::<usize>().ok())
                    .unwrap_or(0),
            );
            let right = (
                right.next().unwrap(),
                right
                    .next()
                    .and_then(|f| f.parse::<usize>().ok())
                    .unwrap_or(0),
            );
            if !subset.contains(&left.0.to_string()) || !subset.contains(&right.0.to_string()) {
                continue;
            }

            let l = action[first_occurrence[left.0]][left.1];
            let d = action[l][1];
            let r = action[first_occurrence[right.0]][right.1];
            let u = action[r][1];

            constraints[r][2].add_tile(l);
            constraints[action[r][6]][2].add_tile(action[l][6]);
            constraints[action[l][4]][2].add_tile(action[r][4]);
            constraints[action[l][2]][2].add_tile(action[r][2]);

            constraints[u][1].add_tile(d);
            constraints[action[d][6]][1].add_tile(action[u][6]);
            constraints[action[u][4]][1].add_tile(action[d][4]);
            constraints[action[d][2]][1].add_tile(action[u][2]);
        }

        // make sure all constraints are reciprocal
        for i in 0..tile_count {
            for direction in 0..4 {
                for allowed in constraints[i][direction].clone().tile_iter() {
                    let other_direction = Direction::from(direction).other();
                    constraints[allowed][other_direction as usize].add_tile(i);
                }
            }
        }

        // make sure no constraint is empty
        for i in 0..tile_count {
            for direction in 0..4 {
                if constraints[i][direction].count_bits() == 0 {
                    println!(
                        "empty constraint found: Tile: {} Direction: {}",
                        i, direction
                    );
                }
            }
        }

        // println!("{a:?}");
        // println!("{b:?}");
        // println!("{}", a == b);

        // for tile in 0..tile_count {
        //     for direction in 0..4 {
        //         if a[tile][direction] != b[tile][direction] {
        //             println!("vvvvvvvvvvvvv");
        //             println!("{:?}", a[tile][direction]);
        //             println!("{:?}", b[tile][direction]);
        //             println!("^^^^^^^^^^^^^");

        //         } else {
        //             println!("{:?}", a[tile][direction]);
        //         }
        //     }
        // }

        Ok(Self {
            tile_count,
            constraints,
            weights,
            tile_paths,
        })
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

    fn get_tile_paths(&self) -> Vec<(String, Transform)> {
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
    #[serde(default)]
    unique: bool,
    tiles: Tiles,
    neighbors: Neighbors,
    subsets: Option<Subsets>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Tiles {
    tile: Vec<Tile>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Tile {
    name: String,
    symmetry: char,
    #[serde(default = "default_weight")]
    weight: f32,
}

fn default_weight() -> f32 {
    1.0
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
