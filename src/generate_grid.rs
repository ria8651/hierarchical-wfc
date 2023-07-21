use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use rand::Rng;
pub struct Voronoi;
impl Voronoi {
    pub fn voronoi(width: u32, height: u32, size: f32) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::LineList);
        let mut rng = rand::thread_rng();
        let mut points = vec![[0.0, 0.0, 0.0]; (width * height) as usize];
        let mut edges: Vec<Vec<u32>> = vec![vec![]; points.len()];

        for i in 0..width {
            for j in 0..height {
                points[(i + j * width) as usize] = [
                    size * (i as f32 + rng.gen_range(0.0..1.0)) / (width as f32),
                    size * (j as f32 + rng.gen_range(0.0..1.0)) / (height as f32),
                    0.0,
                ];
            }
        }

        for i in 0..width {
            for j in 0..height {
                if i + 1 < width {
                    edges[(i + j * width) as usize].push(i + j * width + 1);
                }
                if j + 1 < height {
                    edges[(i + j * width) as usize].push(i + j * width + width);
                }
                if i + 1 < width && j + 1 < height {
                    let p11 = points[(i + j * width) as usize];
                    let p12 = points[(i + j * width + 1 + width) as usize];
                    let p21 = points[(i + j * width + 1) as usize];
                    let p22 = points[(i + j * width + width) as usize];

                    let d1 = (p11[0] - p12[0]).powi(2) + (p11[1] - p12[1]).powi(2);
                    let d2 = (p21[0] - p22[0]).powi(2) + (p21[1] - p22[1]).powi(2);

                    if d1 < d2 {
                        edges[(i + j * width + width) as usize].push(i + j * width + width + 1);
                    } else {
                        edges[(i + j * width + 1) as usize].push(i + j * width + width);
                    }
                }
            }
        }

        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; points.len()]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; points.len()]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, points);

        let mut edge_indices = vec![];
        for (i, nodes) in edges.into_iter().enumerate() {
            for node in nodes {
                edge_indices.push(i as u32);
                edge_indices.push(node as u32);
            }
        }

        mesh.set_indices(Some(Indices::U32(edge_indices)));
        return mesh;
    }
}
