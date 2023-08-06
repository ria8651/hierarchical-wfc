use anyhow::Result;

use super::{Neighbour, Superposition};

#[derive(Debug, Clone)]
pub struct WfcGraph<C> {
    pub nodes: Vec<C>,
    pub order: Vec<usize>,
    pub neighbors: Vec<Vec<Neighbour>>,
}

impl WfcGraph<Superposition> {
    /// Consumes the graph and returns the collapsed tiles
    pub fn validate(self) -> Result<WfcGraph<usize>> {
        let mut result = WfcGraph {
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
