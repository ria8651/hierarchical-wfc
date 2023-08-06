use anyhow::Result;

use super::{Neighbour, Superposition};

#[derive(Debug)]
pub struct Graph<C> {
    pub nodes: Vec<C>,
    pub order: Vec<usize>,
    pub neighbors: Vec<Vec<Neighbour>>,
}

impl Graph<Superposition> {
    /// Consumes the graph and returns the collapsed tiles
    pub fn validate(self) -> Result<Graph<usize>> {
        let mut result = Graph {
            nodes: Vec::new(),
            order: self.order,
            neighbors: self.neighbors,
        };
        for node in 0..self.nodes.len() {
            if let Some(tile) = self.nodes[node].collapse() {
                result.nodes.push(tile);
            } else {
                result.nodes.push(404);
                // return Err(anyhow::anyhow!("Invalid grid"));
            }
        }
        Ok(result)
    }
}
