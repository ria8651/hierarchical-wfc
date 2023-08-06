use bevy::prelude::*;
use rand::{distributions::WeightedIndex, prelude::Distribution, Rng};

pub const TILE_U32S: usize = 4;

#[derive(Deref, DerefMut, Clone, Copy)]
pub struct Superposition(pub [u32; TILE_U32S]);
impl Superposition {
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

    pub fn single(tile: usize) -> Self {
        let mut cell = Self([0; TILE_U32S]);
        cell[tile / 32] |= 1 << (tile % 32);
        return cell;
    }

    pub fn from_iter(tiles: impl Iterator<Item = usize>) -> Self {
        let mut cell = Self([0; TILE_U32S]);
        for tile in tiles {
            cell[tile / 32] |= 1 << (tile % 32);
        }
        return cell;
    }

    /// Leaves a random bit set to 1 and the rest to 0
    pub fn select_random<R: Rng>(&mut self, rng: &mut R, weights: &Vec<u32>) {
        let mut weighted_rng = WeightedIndex::new(weights).unwrap();
        for i in 0..TILE_U32S {
            for j in 0..32 {
                let index = i * 32 + j;
                if self[i] & (1 << j) == 0 && index < weights.len() {
                    weighted_rng.update_weights(&[(index, &0)]).unwrap();
                }
            }
        }

        let selected = weighted_rng.sample(rng);
        self.0 = [0; TILE_U32S];
        self.add_tile(selected);
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

impl std::ops::Add<Superposition> for Superposition {
    type Output = Superposition;
    fn add(self, rhs: Superposition) -> Self::Output {
        return Self::join(&self, &rhs);
    }
}
impl std::ops::Add<usize> for Superposition {
    type Output = Superposition;
    fn add(self, rhs: usize) -> Self::Output {
        return Self::join(&self, &Superposition::single(rhs));
    }
}

impl std::fmt::Debug for Superposition {
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

impl std::fmt::Display for Superposition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // print the number of bits
        write!(f, "{}", self.count_bits())
    }
}

impl PartialEq for Superposition {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
    fn ne(&self, other: &Self) -> bool {
        self.0 != other.0
    }
}

impl Eq for Superposition {}
