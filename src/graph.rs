use anyhow::Result;
use bevy::prelude::*;
use rand::Rng;

pub const TILE_U32S: usize = 4;

#[derive(Debug)]
pub struct Graph<C> {
    pub tiles: Vec<C>,
    pub neighbors: Vec<Vec<Neighbor>>,
}

impl Graph<Cell> {
    /// Consumes the graph and returns the collapsed tiles
    pub fn validate(self) -> Result<Graph<usize>> {
        let mut result = Graph {
            tiles: Vec::new(),
            neighbors: self.neighbors,
        };
        for node in 0..self.tiles.len() {
            if let Some(tile) = self.tiles[node].collapse() {
                result.tiles.push(tile);
            } else {
                return Err(anyhow::anyhow!("Invalid grid"));
            }
        }
        Ok(result)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Neighbor {
    pub direction: usize,
    pub index: usize,
}

#[derive(Deref, DerefMut, Clone, Copy, PartialEq, Eq)]
pub struct Cell(pub [u32; TILE_U32S]);

impl Cell {
    /// Cell fill with ones up to size
    pub fn filled(size: usize) -> Self {
        let mut result = [0; TILE_U32S];
        for i in 0..size {
            result[i / 32] |= 1 << (i % 32);
        }
        Self(result)
    }

    pub fn empty() -> Self {
        Self([0; TILE_U32S])
    }

    pub fn add_tile(&mut self, tile: usize) {
        self[tile / 32] |= 1 << (tile % 32);
    }

    /// Leaves a random bit set to 1 and the rest to 0
    pub fn select_random<R: Rng>(&mut self, rng: &mut R) {
        let selected = rng.gen_range(0..self.count_bits());
        let mut count = 0;
        for i in 0..TILE_U32S {
            for j in 0..32 {
                if self[i] & (1 << j) != 0 {
                    if count != selected {
                        self[i] &= !(1 << j);
                    }
                    count += 1;
                }
            }
        }
    }

    /// Returns the one and only tile if there is only one
    pub fn collapse(&self) -> Option<usize> {
        if self.count_bits() == 1 {
            Some(self.tile_iter().next().unwrap())
        } else {
            None
        }
    }

    pub fn join(a: &Self, b: &Self) -> Self {
        let mut result = [0; TILE_U32S];
        for i in 0..TILE_U32S {
            result[i] = a[i] | b[i];
        }
        Self(result)
    }

    pub fn intersect(a: &Self, b: &Self) -> Self {
        let mut result = [0; TILE_U32S];
        for i in 0..TILE_U32S {
            result[i] = a[i] & b[i];
        }
        Self(result)
    }

    /// Counts the number of bits set to 1
    pub fn count_bits(&self) -> usize {
        let mut result = 0;
        for i in 0..TILE_U32S {
            result += self.0[i].count_ones() as usize;
        }
        result
    }

    /// Returns an iterator over all the set bits
    pub fn tile_iter(&self) -> impl Iterator<Item = usize> + '_ {
        (0..TILE_U32S).flat_map(move |i| {
            (0..32).filter_map(move |j| {
                if self[i] & (1 << j) != 0 {
                    Some(i * 32 + j)
                } else {
                    None
                }
            })
        })
    }
}

impl std::fmt::Debug for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // print all the bits
        for i in 0..TILE_U32S {
            for j in 0..32 {
                if self[i] & (1 << j) != 0 {
                    write!(f, "1")?;
                } else {
                    write!(f, "0")?;
                }
            }
        }
        Ok(())
    }
}

impl std::fmt::Display for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // print the number of bits
        write!(f, "{}", self.count_bits())
    }
}
