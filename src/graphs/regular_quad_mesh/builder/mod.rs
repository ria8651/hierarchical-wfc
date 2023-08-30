use super::types::*;
mod build;
mod from_regular_grid;
mod utils;

pub struct GraphBuilder {
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
    pub quads: Vec<Quad>,
}
