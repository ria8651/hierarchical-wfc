use bevy::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct Neighbour {
    pub arc_type: usize,
    pub index: usize,
}
