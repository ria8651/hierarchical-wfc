use bevy::{math::ivec3, prelude::IVec3};

pub fn ivec3_to_index(pos: IVec3, size: IVec3) -> usize {
    pos.dot(ivec3(1, size.x, size.x * size.y)) as usize
}

pub fn index_to_ivec3(index: usize, size: IVec3) -> IVec3 {
    let i = index as i32;
    ivec3(
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
