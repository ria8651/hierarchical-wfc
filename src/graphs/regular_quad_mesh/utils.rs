use bevy::prelude::*;

pub fn ivec3_to_direction(dir: IVec3) -> Option<usize> {
    match dir {
        IVec3::X => Some(0),
        IVec3::NEG_X => Some(1),
        IVec3::Y => Some(2),
        IVec3::NEG_Y => Some(3),
        IVec3::Z => Some(4),
        IVec3::NEG_Z => Some(5),
        _ => None,
    }
}

pub fn get_matching_direction(dir: usize) -> usize {
    dir + 1 - 2 * dir.rem_euclid(2)
}

pub const DIRECTIONS: [IVec3; 6] = [
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Y,
    IVec3::NEG_Y,
    IVec3::Z,
    IVec3::NEG_Z,
];
