use bevy::{prelude::*, utils::HashMap};
use core_wfc::{TileRender, TileSet, WaveFunction};
use std::{any::Any, sync::Arc, path::Path};

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
    /// overlap is the radius of the overlap, symmetry follows the same rules as mxgmn
    pub fn new(sample: Vec<Vec<usize>>, overlap: usize, symmetry: usize) -> Self {
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
                let mut pattern = Pattern { tiles };

                for i in 0..symmetry {
                    if i > 0 {
                        if i % 2 == 1 {
                            pattern.reflect((overlap * 2 + 1) as usize);
                        } else {
                            pattern.rotate((overlap * 2 + 1) as usize);
                        }
                    }

                    if let Some(weight) = patterns_hash.get_mut(&pattern) {
                        *weight += 1.0;
                    } else {
                        patterns_hash.insert(pattern.clone(), 1.0);
                    }
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

        let directions = vec![
            IVec2::new(0, 1),
            IVec2::new(0, -1),
            IVec2::new(-1, 0),
            IVec2::new(1, 0),
        ];

        let mut constraints = vec![vec![WaveFunction::filled(tile_count); 4]; tile_count];
        for (i, pattern) in patterns.iter().enumerate() {
            for (j, other) in patterns.iter().enumerate() {
                'directions: for (k, direction) in directions.iter().enumerate() {
                    for y in 0..pattern_width {
                        let sy = y - direction.y;
                        if sy < 0 || sy >= pattern_width {
                            continue;
                        }
                        for x in 0..pattern_width {
                            let sx = x - direction.x;
                            if sx < 0 || sx >= pattern_width {
                                continue;
                            }

                            let tile1 = pattern.tiles[(y * pattern_width + x) as usize];
                            let tile2 = other.tiles[(sy * pattern_width + sx) as usize];

                            if tile1 != tile2 {
                                constraints[i][k].remove_tile(j);
                                continue 'directions;
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

    pub fn from_image(path: &Path, overlap: usize, symmetry: usize) -> Self {
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
        for x in 0..size.x {
            let mut column = Vec::new();
            for y in (0..size.y).rev() {
                let pixel = image.get_pixel(x as u32, y as u32);
                let tile = (pixel[0] as usize) << 16 | (pixel[1] as usize) << 8 | pixel[2] as usize;
                column.push(tiles.get(&tile).unwrap().1);
            }
            sample.push(column);
        }

        let mut tileset = Self::new(sample, overlap, symmetry);
        tileset.tile_colors = colors;

        tileset
    }
}

impl TileSet for OverlappingTileset {
    fn tile_count(&self) -> usize {
        self.tile_count
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

    fn get_render_tile(&self, pattern: usize) -> usize {
        self.get_center_tile(pattern).0
    }

    fn get_render_tile_assets(&self) -> Vec<(TileRender, Transform)> {
        let mut tile_render = Vec::new();
        for color in self.tile_colors.iter() {
            tile_render.push((TileRender::Color(*color), Transform::IDENTITY));
        }
        tile_render
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Pattern {
    fn rotate(&mut self, size: usize) {
        let mut new_tiles = Vec::new();
        for y in 0..size {
            for x in 0..size {
                new_tiles.push(self.tiles[x * size + (size - 1 - y)]);
            }
        }
        self.tiles = new_tiles;
    }

    fn reflect(&mut self, size: usize) {
        let mut new_tiles = Vec::new();
        for y in 0..size {
            for x in 0..size {
                new_tiles.push(self.tiles[y * size + (size - 1 - x)]);
            }
        }
        self.tiles = new_tiles;
    }
}
