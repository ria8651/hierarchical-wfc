use bevy::utils::hashbrown::HashMap;
use hierarchical_wfc::wfc::{Neighbour, WfcGraph};
use itertools::Itertools;

pub fn subgraph_with_positions<T, U, P: Clone>(
    graph: &WfcGraph<T>,
    predicate: &dyn Fn(usize, &T) -> Option<U>,
    positions: &[P],
) -> (WfcGraph<U>, Box<[P]>) {
    let mut nodes: Vec<U> = Vec::new();
    let mut new_positions: Vec<P> = Vec::new();
    let mut new_indices = vec![None; graph.nodes.len()];
    let mut old_indices = Vec::new();

    for (index, node) in graph.nodes.iter().enumerate() {
        if let Some(node) = predicate(index, node) {
            nodes.push(node);
            new_positions.push(positions[index].clone());
            new_indices[index] = Some(old_indices.len());
            old_indices.push(index);
        }
    }

    (
        WfcGraph {
            nodes,
            order: graph
                .order
                .iter()
                .flat_map(|index| new_indices[*index])
                .collect_vec(),
            neighbours: old_indices
                .iter()
                .map(|old_index| {
                    graph.neighbours[*old_index]
                        .iter()
                        .flat_map(|neighbour| {
                            if let Some(new_index) = new_indices[neighbour.index] {
                                Some(Neighbour {
                                    arc_type: neighbour.arc_type,
                                    index: new_index,
                                })
                            } else {
                                None
                            }
                        })
                        .collect()
                })
                .collect(),
        },
        new_positions.into(),
    )
}

pub fn graph_merge<T, U, V, P: Eq + PartialEq + std::hash::Hash + Clone + Copy>(
    lhs: (&WfcGraph<T>, &[P]),
    rhs: (&WfcGraph<U>, &[P]),
    merge_nodes: &dyn Fn(Option<&T>, Option<&U>) -> V,
) -> (WfcGraph<V>, Box<[P]>) {
    let mut positions: HashMap<P, (Option<usize>, Option<usize>)> =
        HashMap::with_capacity(lhs.0.nodes.len().max(rhs.0.nodes.len()));

    for (index, pos) in lhs.1.iter().enumerate() {
        positions.entry(*pos).or_insert((None, None)).0 = Some(index);
    }
    for (index, pos) in rhs.1.iter().enumerate() {
        positions.entry(*pos).or_insert((None, None)).1 = Some(index);
    }

    let positions = positions
        .into_iter()
        .sorted_by(|(_, a), (_, b)| Ord::cmp(&a.0.min(a.1), &b.0.min(b.1)))
        .collect_vec();

    let position_map: HashMap<_, _> = positions
        .iter()
        .enumerate()
        .map(|(i, (p, _))| (*p, i))
        .collect();

    // let nodes = positions.iter().map()
    let neighbours: Box<[Box<[Neighbour]>]> = positions
        .iter()
        .map(|(_, (lhs_index, rhs_index))| {
            let lhs: Box<dyn Iterator<Item = Neighbour>> = if let Some(index) = lhs_index {
                Box::new(lhs.0.neighbours[*index].iter().map(|neighbour| Neighbour {
                    arc_type: neighbour.arc_type,
                    index: *(position_map.get(&lhs.1[neighbour.index]).unwrap()),
                }))
            } else {
                Box::new(None.into_iter())
            };
            let rhs: Box<dyn Iterator<Item = Neighbour>> = if let Some(index) = rhs_index {
                Box::new(rhs.0.neighbours[*index].iter().map(|neighbour| Neighbour {
                    arc_type: neighbour.arc_type,
                    index: *(position_map.get(&rhs.1[neighbour.index]).unwrap()),
                }))
            } else {
                Box::new(None.into_iter())
            };
            lhs.chain(rhs)
                .sorted_by(|a, b| Ord::cmp(&a.arc_type, &b.arc_type))
                .dedup_by(|a, b| {
                    if &a.arc_type == &b.arc_type {
                        assert_eq!(a.index, b.index);
                    }
                    &a.arc_type == &b.arc_type
                })
                .collect::<Box<[Neighbour]>>()
        })
        .collect::<Box<[_]>>();

    let nodes = positions
        .iter()
        .map(|(_, (a, b))| {
            let lhs = a.and_then(|a| Some(&lhs.0.nodes[a]));
            let rhs = b.and_then(|b| Some(&rhs.0.nodes[b]));
            merge_nodes(lhs, rhs)
        })
        .collect_vec();

    let order = (0..positions.len()).collect_vec();

    (
        WfcGraph {
            nodes,
            order,
            neighbours,
        },
        positions.into_iter().map(|(p, _)| p).collect::<Box<[P]>>(),
    )
}
