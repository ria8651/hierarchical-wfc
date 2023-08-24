use bevy::prelude::*;
use rand::{distributions::WeightedIndex, prelude::Distribution, Rng};

pub const TILE_U32S: usize = 4;

#[derive(Deref, DerefMut, Clone, Copy)]
pub struct Superposition {
    #[deref]
    pub bits: [u32; TILE_U32S],
    pub num_bits: Option<usize>,
}
#[allow(dead_code)]
impl Superposition {
    /// Cell fill with ones up to size
    pub fn filled(size: usize) -> Self {
        let mut result = [0; TILE_U32S];
        for i in 0..size {
            result[i / 32] |= 1 << (i % 32);
        }
        Self {
            bits: result,
            num_bits: Some(size),
        }
    }

    pub fn empty() -> Self {
        Self {
            bits: [0; TILE_U32S],
            num_bits: None,
        }
    }

    pub fn empty_sized(size: usize) -> Self {
        Self {
            bits: [0; TILE_U32S],
            num_bits: Some(size),
        }
    }

    pub fn add_tile(&mut self, tile: usize) {
        self.bits[tile / 32] |= 1 << (tile % 32);
    }

    pub fn add_other(&mut self, other: &Self) {
        self.bits = Superposition::join(&self, other).bits;
    }

    pub fn contains(&self, tile: usize) -> bool {
        0 != (self.bits[tile / 32] & 1 << (tile % 32))
    }

    pub fn single(tile: usize) -> Self {
        let mut cell = Self {
            bits: [0; TILE_U32S],
            num_bits: None,
        };
        cell.bits[tile / 32] |= 1 << (tile % 32);
        return cell;
    }

    pub fn from_iter(tiles: impl Iterator<Item = usize>) -> Self {
        let mut cell = Self {
            bits: [0; TILE_U32S],
            num_bits: None,
        };
        for tile in tiles {
            cell.bits[tile / 32] |= 1 << (tile % 32);
        }
        cell.num_bits = None;
        return cell;
    }

    pub fn from_iter_sized(tiles: impl Iterator<Item = usize>, size: usize) -> Self {
        let mut cell = Self {
            bits: [0; TILE_U32S],
            num_bits: None,
        };
        for tile in tiles {
            cell.bits[tile / 32] |= 1 << (tile % 32);
        }
        cell.num_bits = Some(size);
        return cell;
    }

    /// Leaves a random bit set to 1 and the rest to 0
    pub fn select_random<R: Rng>(&mut self, rng: &mut R, weights: &Vec<u32>) {
        let mut weighted_rng = WeightedIndex::new(weights).unwrap();
        for i in 0..TILE_U32S {
            for j in 0..32 {
                let index = i * 32 + j;
                if self.bits[i] & (1 << j) == 0 && index < weights.len() {
                    weighted_rng.update_weights(&[(index, &0)]).unwrap();
                }
            }
        }

        let selected = weighted_rng.sample(rng);
        self.bits = [0; TILE_U32S];
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
            result[i] = a.bits[i] | b.bits[i];
        }
        let mut num_bits = None;
        if let (Some(a_size), Some(b_size)) = (a.num_bits, b.num_bits) {
            assert!(
                a_size == b_size,
                "Confit between explicit superposition lengths: ({} vs {})",
                a_size,
                b_size
            );
            num_bits = Some(a_size);
        }

        Self {
            bits: result,
            num_bits,
        }
    }

    pub fn intersect(a: &Self, b: &Self) -> Self {
        let mut result = [0; TILE_U32S];
        for i in 0..TILE_U32S {
            result[i] = a.bits[i] & b.bits[i];
        }
        let mut num_bits = None;
        if let (Some(a_size), Some(b_size)) = (a.num_bits, b.num_bits) {
            assert!(
                a_size == b_size,
                "Confit between explicit superposition lengths: \n\t{}\n\t{}",
                a,
                b
            );
            num_bits = Some(a_size);
        }
        Self {
            bits: result,
            num_bits,
        }
    }

    /// Counts the number of bits set to 1
    pub fn count_bits(&self) -> usize {
        let mut result = 0;
        for i in 0..TILE_U32S {
            result += self.bits[i].count_ones() as usize;
        }
        result
    }

    /// Returns an iterator over all the set bits
    pub fn tile_iter(&self) -> impl Iterator<Item = usize> + '_ {
        (0..TILE_U32S).flat_map(move |i| {
            (0..32).filter_map(move |j| {
                if self.bits[i] & (1 << j) != 0 {
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
                if self.num_bits.is_some_and(|num_bits| i * 32 + j >= num_bits) {
                    break;
                }
                if self.bits[i] & (1 << j) != 0 {
                    write!(f, "1")?;
                } else {
                    write!(f, "0")?;
                }
            }
        }

        if let Some(bit_count) = self.num_bits {
            let bit_count = format!("[{} states]", bit_count);
            write!(f, "{}", bit_count)?;
        }

        Ok(())
    }
}

impl std::fmt::Display for Superposition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // print all the bits
        for i in 0..TILE_U32S {
            for j in 0..32 {
                if self.num_bits.is_some_and(|num_bits| i * 32 + j >= num_bits) {
                    break;
                }
                if self.bits[i] & (1 << j) != 0 {
                    write!(f, "1")?;
                } else {
                    write!(f, "0")?;
                }
            }
        }

        if let Some(bit_count) = self.num_bits {
            write!(f, " [{} states]", bit_count)?;
        }

        Ok(())
    }
}

impl PartialEq for Superposition {
    fn eq(&self, other: &Self) -> bool {
        self.bits == other.bits
    }
    fn ne(&self, other: &Self) -> bool {
        self.bits != other.bits
    }
}

impl Eq for Superposition {}
