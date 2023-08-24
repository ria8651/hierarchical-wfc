use crate::wfc::*;
use bevy::prelude::*;
use rand::rngs::StdRng;

#[derive(Component)]
pub struct WfcEntityMarker;

#[derive(Component, Debug)]
pub struct WfcInitialData {
    pub label: Option<String>,
    pub graph: WfcGraph<Superposition>,
    pub constraints: Vec<Vec<Superposition>>,
    pub weights: Vec<u32>,
    pub rng: StdRng,
}

#[derive(Component)]
pub struct WfcFCollapsedData {
    pub label: Option<String>,
    pub graph: WfcGraph<usize>,
}

#[derive(Component)]
pub struct WfcParentPasses(pub Vec<Entity>);

#[derive(Component)]
pub struct WfcPendingParentMarker;

#[derive(Component)]
pub struct WfcPassReadyMarker;

#[allow(dead_code)]
pub fn wfc_ready_system(
    mut commands: Commands,
    q_pending: Query<(Entity, &WfcParentPasses), With<WfcPendingParentMarker>>,
    q_parent: Query<With<WfcFCollapsedData>>,
) {
    for (child, WfcParentPasses(parents)) in q_pending.iter() {
        if 'ready: {
            for parent in parents {
                match q_parent.get(*parent) {
                    Ok(_) => {}
                    Err(_) => {
                        break 'ready false;
                    }
                }
            }
            true
        } {
            let mut entity_commands = commands.entity(child);
            entity_commands.remove::<WfcPendingParentMarker>();
            entity_commands.insert(WfcPassReadyMarker);
        }
    }
}
#[allow(dead_code)]
pub fn wfc_collapse_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut WfcInitialData)>,
) {
    for (entity, mut initial_data) in query.iter_mut() {
        dbg!("Collapsing Entity");
        let WfcInitialData {
            label,
            graph,
            constraints,
            weights,
            rng,
        } = initial_data.as_mut();

        WaveFunctionCollapse::collapse(graph, constraints, weights, rng);
        let mut entity_commands = commands.entity(entity);
        entity_commands.remove::<WfcInitialData>();
        match graph.validate() {
            Ok(result) => {
                entity_commands.insert(WfcFCollapsedData {
                    label: Option::take(label),
                    graph: result,
                });
            }
            Err(_) => {}
        };
    }
}
