use crate::{
    castle::facade_graph::FacadeTileset,
    wfc::{Neighbour, Superposition},
};
use bevy::{math::vec3, prelude::*};

use self::types::*;

pub mod builder;
pub mod types;
pub mod utils;

#[derive(Component, Default)]
pub struct GraphSettings;

#[derive(Component)]
pub struct GraphData {
    pub vertices: Box<[Vertex]>,
    pub edges: Box<[Edge]>,
    pub quads: Box<[Quad]>,
}

impl GraphData {
    pub fn get_node_pos(&self, node: usize) -> Vec3 {
        vec3(2.0, 3.0, 2.0) * {
            if node < self.vertices.len() {
                1.0 * self.vertices[node].pos.as_vec3()
            } else if node < self.vertices.len() + self.edges.len() {
                0.5 * self.edges[node - self.vertices.len()].pos.as_vec3()
            } else {
                0.25 * self.quads[node - self.vertices.len() - self.edges.len()]
                    .pos
                    .as_vec3()
            }
        }
    }
}
