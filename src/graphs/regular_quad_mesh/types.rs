use bevy::prelude::*;

pub struct Vertex {
    pub pos: IVec3,
    pub neighbours: [Option<usize>; 6],
    pub edges: [Option<usize>; 6],
}

#[derive(Debug)]
pub struct Edge {
    pub pos: IVec3,
    pub from: usize,
    pub to: usize,
    pub quads: Box<[(usize, usize)]>,
    pub tangent: usize,
    pub cotangent: usize,
}
#[derive(Debug)]
pub struct Quad {
    pub pos: IVec3,
    pub normal: usize,
    pub tangent: usize,
    pub cotangent: usize,
    pub verts: [usize; 4],
    pub edges: [usize; 4],
}
