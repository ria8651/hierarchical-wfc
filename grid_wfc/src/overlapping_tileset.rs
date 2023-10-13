use crate::overlapping_graph::{self, OverlappingGraphSettings};
use bevy::{prelude::*, utils::HashMap};
use hierarchical_wfc::{Graph, TileRender, TileSet, WaveFunction};
use std::{any::Any, sync::Arc};

#[derive(Hash, PartialEq, Eq, Debug, Clone)]
struct Pattern {
    tiles: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct OverlappingTileset {
    tile_count: usize,
    overlap: usize,
    patterns: Arc<Vec<Pattern>>,
    constraints: Arc<Vec<Vec<WaveFunction>>>,
    weights: Arc<Vec<f32>>,
    tile_colors: Vec<Color>,
}

impl OverlappingTileset {
    pub fn new(sample: Vec<Vec<usize>>, overlap: usize) -> Self {
        let overlap = overlap as i32;
        let size = IVec2::new(sample.len() as i32, sample[0].len() as i32);

        let mut patterns_hash = HashMap::new();
        for y in 0..size.y {
            for x in 0..size.x {
                let mut tiles = Vec::new();
                for py in -overlap..=overlap {
                    for px in -overlap..=overlap {
                        let sx = (x + px).rem_euclid(size.x);
                        let sy = (y + py).rem_euclid(size.y);
                        tiles.push(sample[sx as usize][sy as usize]);
                    }
                }
                let pattern = Pattern { tiles };
                if let Some(weight) = patterns_hash.get_mut(&pattern) {
                    *weight += 1.0;
                } else {
                    patterns_hash.insert(pattern, 1.0);
                }
            }
        }

        let mut patterns = Vec::new();
        let mut weights = Vec::new();
        for (pattern, weight) in patterns_hash {
            patterns.push(pattern);
            weights.push(weight);
        }

        let tile_count = patterns.len();
        let pattern_width = overlap * 2 + 1;
        let offsets = overlap * 2;
        let directions_width = offsets * 2 + 1;
        let directions = directions_width * directions_width;

        let mut constraints =
            vec![vec![WaveFunction::filled(tile_count); directions as usize]; tile_count];
        for (i, pattern) in patterns.iter().enumerate() {
            for (j, other) in patterns.iter().enumerate() {
                for oy in -offsets..=offsets {
                    'offsets: for ox in -offsets..=offsets {
                        let direction_index = (oy + offsets) * directions_width + ox + offsets;
                        for y in 0..pattern_width as i32 {
                            let sy = y - oy;
                            if sy < 0 || sy >= pattern_width as i32 {
                                continue;
                            }
                            for x in 0..pattern_width as i32 {
                                let sx = x - ox;
                                if sx < 0 || sx >= pattern_width as i32 {
                                    continue;
                                }

                                let tile1 = pattern.tiles[(y * pattern_width as i32 + x) as usize];
                                let tile2 = other.tiles[(sy * pattern_width as i32 + sx) as usize];

                                if tile1 != tile2 {
                                    constraints[i][direction_index as usize].remove_tile(j);
                                    continue 'offsets;
                                }
                            }
                        }

                        // println!(
                        //     "pattern({}) == pattern({}) with offset ({}, {})",
                        //     i, j, ox, oy
                        // );
                    }
                }
            }
        }

        // for tile in 0..tile_count {
        //     for other in 0..tile_count {
        //         for direction in 0..directions {
        //             if constraints[tile][direction as usize].contains(other) {
        //                 let direction_vec = IVec2::new(
        //                     (direction % directions_width) as i32 - offsets,
        //                     (direction / directions_width) as i32 - offsets,
        //                 );
        //                 println!(
        //                     "pattern({}) -> pattern({}) with direction {:?}",
        //                     tile, other, direction_vec
        //                 );
        //             }
        //         }
        //     }
        // }

        // println!("constraints({}): {:?}", constraints.len(), constraints);
        // println!("weights({}): {:?}", weights.len(), weights);

        let mut tile_colors = Vec::new();
        for i in 0..tile_count {
            let value = i as f32 / tile_count as f32;
            tile_colors.push(Color::rgb(value, value, value));
        }

        Self {
            tile_count,
            overlap: overlap as usize,
            patterns: Arc::new(patterns),
            constraints: Arc::new(constraints),
            weights: Arc::new(weights),
            tile_colors,
        }
    }

    pub fn get_center_tile(&self, index: usize) -> (usize, Color) {
        let tile = self.patterns[index].tiles[self.overlap * (self.overlap * 2 + 1) + self.overlap];
        (tile, self.tile_colors[tile])
    }

    pub fn from_image(path: &str, overlap: usize) -> Self {
        let image = image::open(path).unwrap();
        let image = image.to_rgba8();
        let size = IVec2::new(image.width() as i32, image.height() as i32);

        let mut tiles = HashMap::new();
        for y in 0..size.y {
            for x in 0..size.x {
                let pixel = image.get_pixel(x as u32, y as u32);
                let color = Color::rgb(
                    pixel[0] as f32 / 255.0,
                    pixel[1] as f32 / 255.0,
                    pixel[2] as f32 / 255.0,
                );
                let tile = (pixel[0] as usize) << 16 | (pixel[1] as usize) << 8 | pixel[2] as usize;
                tiles.insert(tile, (color, 0));
            }
        }

        let mut colors = Vec::new();
        for (i, (_, (color, tile_index))) in tiles.iter_mut().enumerate() {
            *tile_index = i;
            colors.push(*color);
        }

        let mut sample = Vec::new();
        for y in (0..size.y).rev() {
            let mut row = Vec::new();
            for x in 0..size.x {
                let pixel = image.get_pixel(x as u32, y as u32);
                let tile = (pixel[0] as usize) << 16 | (pixel[1] as usize) << 8 | pixel[2] as usize;
                row.push(tiles.get(&tile).unwrap().1);
            }
            sample.push(row);
        }

        let mut tileset = Self::new(sample, overlap);
        tileset.tile_colors = colors;

        tileset
    }
}

impl TileSet for OverlappingTileset {
    fn tile_count(&self) -> usize {
        self.tile_count
    }

    fn directions(&self) -> usize {
        (self.overlap * 2 + 1) * (self.overlap * 2 + 1)
    }

    fn get_constraints(&self) -> Arc<Vec<Vec<WaveFunction>>> {
        self.constraints.clone()
    }

    fn get_weights(&self) -> Arc<Vec<f32>> {
        self.weights.clone()
    }

    fn set_weights(&mut self, weights: Vec<f32>) {
        self.weights = Arc::new(weights);
    }

    fn create_graph(&self, settings: Box<dyn Any>) -> Graph<WaveFunction> {
        let settings = settings.downcast_ref::<OverlappingGraphSettings>().unwrap();
        overlapping_graph::create(settings, WaveFunction::filled(self.tile_count()))
    }

    fn get_tile_paths(&self) -> Vec<(TileRender, Transform)> {
        let mut tile_render = Vec::new();
        for tile in 0..self.tile_count {
            let value = tile as f32 / self.tile_count as f32;
            tile_render.push((
                TileRender::Color(Color::rgb(value, value, value)),
                Transform::IDENTITY,
            ));
        }
        tile_render
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
