use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::utils::HashSet;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[derive(Clone, PartialEq)]
pub struct Node {
    wfc: WfcNode,
    position: Vec2,
    adj: Vec<u32>,
}
impl Eq for Node {}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum WfcState {
    River,
    RiverBank,
    Grass,
    Hills,
}
impl WfcState {
    pub fn all() -> HashSet<Self> {
        return HashSet::from_iter(
            [Self::River, Self::RiverBank, Self::Grass, Self::Hills].into_iter(),
        );
    }
    pub fn single(self) -> HashSet<Self> {
        return HashSet::from_iter([self].into_iter());
    }
}
#[derive(Clone, Eq, PartialEq)]
struct WfcNode {
    states: HashSet<WfcState>,
}
impl WfcNode {
    fn entropy(&self) -> u32 {
        return self.states.len() as u32;
    }
    pub fn collapse(&self, rng: &mut StdRng) -> WfcState {
        self.states
            .iter()
            .nth(rng.gen_range(0..self.states.len()))
            .unwrap()
            .clone()
    }

    pub fn constrain(&mut self, neighbour: &WfcState) -> bool {
        let allowed = match neighbour {
            WfcState::Grass => [WfcState::Grass, WfcState::Hills, WfcState::RiverBank].iter(),
            WfcState::Hills => [WfcState::Grass, WfcState::Hills].iter(),
            WfcState::River => [WfcState::River, WfcState::RiverBank].iter(),
            WfcState::RiverBank => [WfcState::RiverBank, WfcState::River, WfcState::Grass].iter(),
        };
        let allowed_set = HashSet::from_iter(allowed.map(|e| e.clone()));
        let old = self.states.clone();
        self.states = self.states.intersection(&allowed_set).copied().collect();

        return self.states != old;
    }
}

impl Default for WfcNode {
    fn default() -> Self {
        return WfcNode {
            states: WfcState::all(),
        };
    }
}

pub struct PlanarGraph {
    nodes: Vec<Node>,
}

impl PlanarGraph {
    pub fn add_edge(&mut self, u: u32, v: u32) {
        self.nodes[u as usize].adj.push(v);
        self.nodes[v as usize].adj.push(u);
    }
    pub fn node(&self, i: u32) -> &Node {
        return &self.nodes[i as usize];
    }

    pub fn new_voronoi(width: u32, height: u32, size: f32) -> PlanarGraph {
        let mut rng = rand::thread_rng();
        let mut nodes = vec![
            Node {
                wfc: WfcNode::default(),
                position: Vec2::ZERO,
                adj: vec![],
            };
            (width * height) as usize
        ];

        for i in 0..width {
            for j in 0..height {
                nodes[(i + j * width) as usize].position = Vec2::new(
                    size * (i as f32 + rng.gen_range(0.0..1.0)) / (width as f32),
                    size * (j as f32 + rng.gen_range(0.0..1.0)) / (height as f32),
                );
                if i == 0 {
                    nodes[(i + j * width) as usize].wfc.states = WfcState::Hills.single();
                } else if i == width - 1 {
                    nodes[(i + j * width) as usize].wfc.states = WfcState::River.single();
                }
            }
        }

        let mut graph = PlanarGraph { nodes: nodes };

        for i in 0..width {
            for j in 0..height {
                if i + 1 < width {
                    graph.add_edge(i + j * width, i + j * width + 1);
                }
                if j + 1 < height {
                    graph.add_edge(i + j * width, i + j * width + width);
                }
                if i + 1 < width && j + 1 < height {
                    let p11 = graph.node(i + j * width).position;
                    let p12 = graph.node(i + j * width + 1 + width).position;
                    let p21 = graph.node(i + j * width + 1).position;
                    let p22 = graph.node(i + j * width + width).position;

                    let d1 = (p11[0] - p12[0]).powi(2) + (p11[1] - p12[1]).powi(2);
                    let d2 = (p21[0] - p22[0]).powi(2) + (p21[1] - p22[1]).powi(2);

                    if d1 < d2 {
                        graph.add_edge(i + j * width + width, i + j * width + width + 1);
                    } else {
                        graph.add_edge(i + j * width + 1, i + j * width + width);
                    }
                }
            }
        }

        return graph;
    }

    pub fn mesh_edges(&self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::LineList);

        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; self.nodes.len()]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; self.nodes.len()]);
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            self.nodes
                .iter()
                .map(|node| node.position.extend(0.0))
                .rev()
                .collect::<Vec<_>>(),
        );

        let mut edge_indices = vec![];
        for (i, nodes) in self.nodes.iter().enumerate() {
            for node in nodes.adj.iter() {
                edge_indices.push(i as u32);
                edge_indices.push(*node as u32);
            }
        }

        mesh.set_indices(Some(Indices::U32(edge_indices)));
        return mesh;
    }
    pub fn mesh_nodes(&self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

        mesh.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            vec![[0., 1., 0.]; 3 * self.nodes.len()],
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_UV_0,
            self.nodes
                .iter()
                .flat_map(|_| {
                    [
                        Vec2::new(0.0, 1.0),
                        Vec2::new(-0.866025403784, -0.5),
                        Vec2::new(0.866025403784, -0.5),
                    ]
                })
                .rev()
                .collect::<Vec<_>>(),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_COLOR,
            self.nodes
                .iter()
                .map(|node| match node.wfc.states.iter().next() {
                    Some(WfcState::Grass) => Color::hex("a9dc76").unwrap().as_linear_rgba_f32(),
                    Some(WfcState::River) => Color::hex("78dce8").unwrap().as_linear_rgba_f32(),
                    Some(WfcState::RiverBank) => Color::hex("ffd866").unwrap().as_linear_rgba_f32(),
                    Some(WfcState::Hills) => Color::hex("fcfcfa").unwrap().as_linear_rgba_f32(),
                    _ => [1.0, 0.0, 1.0, 1.0],
                })
                .flat_map(|e| [e, e, e])
                .rev()
                .collect::<Vec<_>>(),
        );

        const POINT_SIZE: f32 = 0.0125;
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            self.nodes
                .iter()
                .flat_map(|node| {
                    [
                        node.position.extend(0.01) + POINT_SIZE * Vec3::new(0.0, 1.0, 0.0),
                        node.position.extend(0.01)
                            + POINT_SIZE * Vec3::new(-0.866025403784, -0.5, 0.0),
                        node.position.extend(0.01)
                            + POINT_SIZE * Vec3::new(0.866025403784, -0.5, 0.0),
                    ]
                })
                .rev()
                .collect::<Vec<_>>(),
        );

        return mesh;
    }
}

impl Wfc for PlanarGraph {
    fn collapse(&mut self, seed: u64) {
        let mut rng = StdRng::seed_from_u64(seed);

        let mut remaining = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, _)| i as u32)
            .collect::<Vec<u32>>();

        // Get lowest entropy node (this is very bad rn)
        while let Some((i, u)) = remaining.clone().iter().enumerate().reduce(|prev, next| {
            if self.node(*prev.1).wfc.entropy() as f32
                + (if rng.gen_bool(0.5) { -0.5f32 } else { 0.5f32 })
                <= self.node(*next.1).wfc.entropy() as f32
            {
                return prev;
            } else {
                return next;
            }
        }) {
            remaining.remove(i);
            let collapsed = self.node(*u).wfc.collapse(&mut rng).clone();
            for v in self.nodes[*u as usize].adj.clone() {
                self.nodes[v as usize].wfc.constrain(&collapsed);
                // remaining.remove(remaining.iter().position(|w| *w == v).unwrap());
            }
            self.nodes[*u as usize].wfc.states = collapsed.single();
        }
    }

    fn validate(&self) {
        for (i, node) in self.nodes.iter().enumerate() {
            if node.wfc.states.len() != 1 {
                println!("{}", i);
                dbg!(node.wfc.states.iter());
            }
            // assert!(node.wfc.states.len() == 1);
        }
    }
}

pub trait Wfc {
    fn collapse(&mut self, seed: u64);
    fn validate(&self);
}
