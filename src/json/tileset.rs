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
    pub label: String,
    pub sockets: HashMap<String, String>,
    #[serde(default)]
    pub optional: Vec<String>,
    pub symmetries: Vec<String>,
}
impl Default for SemanticNodeModel {
    fn default() -> Self {
        Self {
            label: "".to_string(),
            sockets: HashMap::new(),
            optional: Vec::new(),
            symmetries: Vec::new(),
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
pub struct AssetModel {
    pub path: String,
    pub nodes: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
pub struct TileSetModel {
    pub directions: Vec<String>,
    pub symmetries: HashMap<String, HashMap<String, String>>,
    pub semantic_nodes: Vec<SemanticNodeModel>,
    pub constraints: Vec<[ConstraintNodeModel; 2]>,
    pub semantic_dag: DagNodeModel,
    pub assets: HashMap<String, AssetModel>,
}

impl TileSetModel {
    fn traverse_dag(node: &DagNodeModel, new_nodes: &mut HashSet<String>) {
        match node {
            DagNodeModel::Leaf => {}
            DagNodeModel::Meta(children) => {
                for (key, child) in children {
                    new_nodes.insert(key.clone());
                    Self::traverse_dag(child, new_nodes);
                }
            }
        }
    }

    pub fn from_asset(asset_path: String) -> anyhow::Result<Self> {
        let file = fs::File::open(format!("assets/{}", asset_path)).unwrap();
        let reader = BufReader::new(file);

        match serde_json::from_reader::<BufReader<fs::File>, TileSetModel>(reader) {
            Ok(mut model) => {
                let mut semantic_labels: HashSet<String> =
                    HashSet::from_iter(model.semantic_nodes.iter().map(|n| n.label.clone()));
                semantic_labels.insert("root".to_string());
                model.semantic_nodes.insert(
                    0,
                    SemanticNodeModel {
                        label: "root".to_string(),
                        ..Default::default()
                    },
                );

                let mut dag_nodes: HashSet<String> = HashSet::new();
                Self::traverse_dag(&model.semantic_dag, &mut dag_nodes);
                for node in dag_nodes {
                    if !semantic_labels.contains(&node) {
                        semantic_labels.insert(node.clone());
                        model.semantic_nodes.push(SemanticNodeModel {
                            label: node,
                            ..Default::default()
                        });
                    }
                }

                anyhow::Ok(model)
            }
            Err(e) => Err(e.into()),
        }
    }
}
