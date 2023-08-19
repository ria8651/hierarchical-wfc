use std::{collections::HashSet, fs, io::BufReader};

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum ConstraintNodeModel {
    Node(String),
    NodeSocket { node: String, socket: String },
}

#[derive(Deserialize, Debug)]

pub struct SemanticNodeModel {
    pub sockets: HashMap<String, String>,
    pub symmetries: Vec<String>,
    pub assets: HashMap<String, String>,
}
impl Default for SemanticNodeModel {
    fn default() -> Self {
        Self {
            sockets: HashMap::new(),
            symmetries: Vec::new(),
            assets: HashMap::new(),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]

pub enum DagNodeModel {
    Meta(HashMap<String, DagNodeModel>),
    Leaf,
}

#[derive(Deserialize, Debug)]
pub struct TileSetModel {
    pub directions: Vec<String>,
    pub symmetries: HashMap<String, HashMap<String, String>>,
    pub semantic_nodes: HashMap<String, SemanticNodeModel>,
    pub constraints: Vec<[ConstraintNodeModel; 2]>,
    pub semantic_dag: DagNodeModel,
}

impl TileSetModel {
    fn traverse_dag(node: &DagNodeModel, new_nodes: &mut HashSet<String>) {
        match node {
            DagNodeModel::Leaf => {}
            DagNodeModel::Meta(children) => {
                for (key, child) in children {
                    new_nodes.insert(key.clone());
                    Self::traverse_dag(&child, new_nodes);
                }
            }
        }
    }

    pub fn from_asset(asset_path: String) -> anyhow::Result<Self> {
        let file = fs::File::open(format!("assets/{}", asset_path)).unwrap();
        let reader = BufReader::new(file);
        match serde_json::from_reader::<BufReader<fs::File>, TileSetModel>(reader) {
            Ok(mut model) => {
                let mut dag_nodes: HashSet<String> = HashSet::new();
                Self::traverse_dag(&model.semantic_dag, &mut dag_nodes);
                for node in dag_nodes {
                    if !model.semantic_nodes.contains_key(&node) {
                        model
                            .semantic_nodes
                            .insert(node, SemanticNodeModel::default());
                    }
                }

                anyhow::Ok(model)
            }
            Err(e) => Err(e.into()),
        }
    }
}
