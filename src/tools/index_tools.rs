use bevy::{
    math::{ivec3, uvec3},
    prelude::*,
};

pub fn ivec3_to_index(pos: IVec3, size: IVec3) -> usize {
    pos.dot(ivec3(1, size.x, size.x * size.y)) as usize
}

pub fn uvec3_to_index(pos: UVec3, size: UVec3) -> usize {
    pos.dot(uvec3(1, size.x, size.x * size.y)) as usize
}

pub fn index_to_ivec3(index: usize, size: IVec3) -> IVec3 {
    let i = index as i32;
    ivec3(
        i.rem_euclid(size.x),
        i.div_euclid(size.x).rem_euclid(size.y),
        i.div_euclid(size.x * size.y),
    )
}

pub fn index_to_uvec3(index: usize, size: UVec3) -> UVec3 {
    let i = index as u32;
    uvec3(
        i.rem_euclid(size.x),
        i.div_euclid(size.x).rem_euclid(size.y),
        i.div_euclid(size.x * size.y),
    )
}

pub fn ivec3_in_bounds(pos: IVec3, min: IVec3, max: IVec3) -> bool {
    (min.x..max.x).contains(&pos.x)
        && (min.y..max.y).contains(&pos.y)
        && (min.z..max.z).contains(&pos.z)
}
