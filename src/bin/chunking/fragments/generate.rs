use bevy::{
    math::{uvec3, vec3, Vec3Swizzles},
    prelude::*,
    utils::{HashMap, HashSet},
};
use hierarchical_wfc::{
    castle::LayoutTileset,
    graphs::regular_grid_3d,
    tools::index_tools::{index_to_ivec3, index_to_uvec3, ivec3_to_index, uvec3_to_index},
    wfc::{Neighbour, Superposition, TileSet, WaveFunctionCollapse, WfcGraph},
};
use itertools::{iproduct, Itertools};
use rand::{rngs::StdRng, SeedableRng};

#[derive(Event)]
pub enum ChunkLoadEvent {
    Load(IVec3),
    Unload(IVec3),
}

#[derive(Resource, Default)]
pub struct LoadedFragments {
    loaded: HashMap<IVec3, Entity>,
}

#[derive(Resource, Default)]
pub struct LoadedChunks {
    loaded: HashMap<IVec3, Entity>,
    waiting: HashMap<IVec3, HashSet<IVec3>>,
    waited_by: HashMap<IVec3, HashSet<IVec3>>,
}

#[derive(Event)]

pub enum FragmentLoadEvent {
    Load(IVec3),
    Unload(IVec3),
}

#[derive(Event)]
pub struct FragmentGenerateEvent(IVec3);

#[derive(Event)]
pub struct FragmentGeneratedEvent(IVec3);

#[derive(Resource)]
pub struct LayoutSettings {
    pub tileset: LayoutTileset,
    pub settings: regular_grid_3d::GraphSettings,
}
#[derive(Component)]

pub struct CollapsedData {
    pub graph: WfcGraph<usize>,
}

#[derive(Event)]
pub struct ChunkGenerateEvent(IVec3);

#[derive(Component)]
pub struct GenerateDebugMarker;

#[derive(Resource, Default)]
pub struct GenerationDebugSettings {
    pub debug_fragments: bool,
    pub debug_chunks: bool,
}

#[derive(Component)]
pub struct ChunkMarker;

#[derive(Component)]
pub struct FragmentMarker;

pub fn fragments_for_chunk(chunk_pos: IVec3) -> [IVec3; 4] {
    [IVec3::ZERO, IVec3::X, IVec3::Z, IVec3::Z + IVec3::X].map(|delta| chunk_pos + delta)
}

pub mod systems {
    use super::*;
    use bevy::prelude::*;

    pub fn transform_chunk_loads(
        mut ev_load_chunk: EventReader<ChunkLoadEvent>,
        mut ev_generate_fragment: EventWriter<FragmentGenerateEvent>,
        mut loaded_chunks: ResMut<LoadedChunks>,
        loaded_fragments: ResMut<LoadedFragments>,
    ) {
        for load_chunk in ev_load_chunk.iter() {
            if let ChunkLoadEvent::Load(chunk_pos) = load_chunk {
                let mut fragments_ready = true;
                if !loaded_chunks.loaded.contains_key(chunk_pos)
                    && !loaded_chunks.waiting.contains_key(chunk_pos)
                {
                    let mut waiting = HashSet::new();

                    for fragment_pos in fragments_for_chunk(*chunk_pos) {
                        if !loaded_fragments.loaded.contains_key(&fragment_pos) {
                            ev_generate_fragment.send(FragmentGenerateEvent(fragment_pos));
                            waiting.insert(fragment_pos);
                            let value = loaded_chunks.waited_by.entry(fragment_pos).or_default();
                            value.insert(*chunk_pos);
                        }
                    }
                    loaded_chunks.waiting.insert(*chunk_pos, waiting);
                }
            }
        }
    }

    pub fn generate_fragments(
        mut commands: Commands,
        layout_settings: Res<LayoutSettings>,
        mut ev_generate_fragment: EventReader<FragmentGenerateEvent>,
        mut ev_generated_fragment: EventWriter<FragmentGeneratedEvent>,
        mut loaded_fragments: ResMut<LoadedFragments>,
        mut debug_settings: ResMut<GenerationDebugSettings>,
    ) {
        let tileset = &layout_settings.tileset;
        let weights = tileset.get_weights();
        let constraints = tileset.get_constraints();

        for event in ev_generate_fragment.iter() {
            if let FragmentGenerateEvent(pos) = event {
                let (data, mut graph) =
                    regular_grid_3d::create_graph(&layout_settings.settings, &|(_, _)| {
                        Superposition::filled(layout_settings.tileset.tile_count())
                    });

                let seed = pos.to_array();
                let mut seed: Vec<u8> = seed.map(|i| i.to_be_bytes()).concat().into();
                seed.extend([0u8; 20].into_iter());
                let seed: [u8; 32] = seed.try_into().unwrap();

                WaveFunctionCollapse::collapse(
                    &mut graph,
                    &constraints,
                    &weights,
                    &mut StdRng::from_seed(seed),
                );

                if let Ok(result) = graph.validate() {
                    let mut fragment_commands = commands.spawn((
                        FragmentMarker,
                        Transform::from_translation(
                            pos.as_vec3()
                                * layout_settings.settings.size.as_vec3()
                                * layout_settings.settings.spacing,
                        ),
                        layout_settings.settings.clone(),
                        data,
                        CollapsedData { graph: result },
                    ));
                    if debug_settings.debug_fragments {
                        fragment_commands.insert(GenerateDebugMarker);
                    }
                    let fragment = fragment_commands.id();
                    loaded_fragments.loaded.insert(*pos, fragment);
                    ev_generated_fragment.send(FragmentGeneratedEvent(*pos));
                }
            }
        }
    }

    pub fn apply_loaded_fragments(
        mut ev_generated_fragment: EventReader<FragmentGeneratedEvent>,
        mut ev_generate_chunk: EventWriter<ChunkGenerateEvent>,
        mut loaded_chunks: ResMut<LoadedChunks>,
    ) {
        for FragmentGeneratedEvent(fragment_pos) in ev_generated_fragment.iter() {
            let chunks = loaded_chunks
                .waited_by
                .get(fragment_pos)
                .unwrap()
                .iter()
                .copied()
                .collect_vec();
            for chunk_pos in chunks {
                if let Some(waiting) = loaded_chunks.waiting.get_mut(&chunk_pos) {
                    waiting.remove(fragment_pos);

                    if waiting.is_empty() {
                        loaded_chunks.waiting.remove(&chunk_pos);
                    }
                }

                ev_generate_chunk.send(ChunkGenerateEvent(chunk_pos));
            }
        }
    }

    pub fn generate_chunks(
        mut commands: Commands,
        mut ev_generate_chunk: EventReader<ChunkGenerateEvent>,
        loaded_fragments: ResMut<LoadedFragments>,
        mut loaded_chunks: ResMut<LoadedChunks>,
        debug_settings: Res<GenerationDebugSettings>,
        q_fragments: Query<(
            &regular_grid_3d::GraphSettings,
            &regular_grid_3d::GraphData,
            &CollapsedData,
        )>,
        layout_settings: Res<LayoutSettings>,
    ) {
        let fill_with = Superposition::filled(layout_settings.tileset.tile_count());
        let tileset = &layout_settings.tileset;
        let weights = tileset.get_weights();
        let constraints = tileset.get_constraints();

        for ChunkGenerateEvent(pos) in ev_generate_chunk.iter() {
            let chunk_pos = *pos;
            let fragments_positions = fragments_for_chunk(chunk_pos);

            let fragment_entities = fragments_positions
                .map(|frag: IVec3| loaded_fragments.loaded.get(&frag).unwrap().clone());

            let fragments: [(
                &regular_grid_3d::GraphSettings,
                &regular_grid_3d::GraphData,
                &CollapsedData,
            ); 4] = q_fragments.get_many(fragment_entities).unwrap();
            // let mut corner_sizes = [UVec3::ZERO; 4];
            // let mut corner_graphs = [None, None, None, None];

            let (merged_settings, merged_data, mut merged_graph) =
                merge_corners(fragments, &layout_settings);

            // for (i, corner) in [UVec3::ZERO, UVec3::X, UVec3::Z, UVec3::X + UVec3::Z]
            //     .into_iter()
            //     .enumerate()
            // {
            //     let (settings, data, CollapsedData { graph }) = fragments[i];
            //     let (size, graph) = extract_corner(corner, settings, data, graph, &fill_with);
            //     corner_sizes[i] = size;
            //     corner_graphs[i] = Some(graph);
            // }

            // let corner_graphs: [WfcGraph<Superposition>; 4] = corner_graphs.map(|g| g.unwrap());

            // assert!(
            //     corner_sizes[0] == corner_sizes[1]
            //         && corner_sizes[1] == corner_sizes[2]
            //         && corner_sizes[2] == corner_sizes[3]
            // );

            // let (merged_data, merged_settings, mut merged_graph) = merged_corners(
            //     corner_graphs,
            //     corner_sizes[0],
            //     layout_settings.settings.spacing,
            // );

            let seed = pos.to_array();
            let mut seed: Vec<u8> = seed.map(|i| i.to_be_bytes()).concat().into();
            seed.extend([0u8; 20].into_iter());
            let seed: [u8; 32] = seed.try_into().unwrap();

            WaveFunctionCollapse::collapse(
                &mut merged_graph,
                &constraints,
                &weights,
                &mut StdRng::from_seed(seed),
            );

            if let Ok(result) = merged_graph.validate() {
                let mut chunk_commands = commands.spawn((
                    ChunkMarker,
                    Transform::from_translation(
                        merged_settings.spacing
                            * (pos.as_vec3() * (layout_settings.settings.size).as_vec3()
                                + Vec3::Y * 2.0),
                    ),
                    merged_data,
                    merged_settings,
                    CollapsedData { graph: result },
                ));
                if debug_settings.debug_chunks {
                    chunk_commands.insert(GenerateDebugMarker);
                }

                let chunk = chunk_commands.id();
                loaded_chunks.loaded.insert(chunk_pos, chunk);
            }
        }
    }
}

pub fn merge_corners(
    fragments: [(
        &regular_grid_3d::GraphSettings,
        &regular_grid_3d::GraphData,
        &CollapsedData,
    ); 4],
    layout_settings: &LayoutSettings,
) -> (
    regular_grid_3d::GraphSettings,
    regular_grid_3d::GraphData,
    WfcGraph<Superposition>,
) {
    let fill_with = Superposition::filled(layout_settings.tileset.tile_count());

    let chunk_size = layout_settings.settings.size + uvec3(2, 0, 2);
    let quadrant_size = chunk_size / 2;
    let fragment_size = layout_settings.settings.size;

    let chunk_node_count = (chunk_size.x * chunk_size.y * chunk_size.z) as usize;
    let fragment_node_count = (fragment_size.x * fragment_size.y * fragment_size.z) as usize;

    let mut chunk_nodes: Vec<Superposition> = Vec::with_capacity(chunk_node_count);
    let mut chunk_node_positions: Vec<IVec3> = Vec::with_capacity(chunk_node_count);

    let mut new_indices = [
        vec![None; fragment_node_count],
        vec![None; fragment_node_count],
        vec![None; fragment_node_count],
        vec![None; fragment_node_count],
    ];

    let mut old_indices: Vec<(usize, usize)> = Vec::with_capacity(chunk_node_count);

    for (chunk_node_index, (z, y, x)) in iproduct!(
        0..chunk_size.z as usize,
        0..chunk_size.y as usize,
        0..chunk_size.x as usize
    )
    .enumerate()
    {
        let chunk_node_pos = uvec3(x as u32, y as u32, z as u32);
        let fragment_node_pos = uvec3(
            (fragment_size.x + chunk_node_pos.x - quadrant_size.x).rem_euclid(fragment_size.x),
            (fragment_size.y + chunk_node_pos.y - quadrant_size.y).rem_euclid(fragment_size.y),
            (fragment_size.z + chunk_node_pos.z - quadrant_size.z).rem_euclid(fragment_size.z),
        );
        let fragment_node_index = uvec3_to_index(fragment_node_pos, fragment_size);

        let fragment_pos =
            uvec3(1, 0, 1) * chunk_node_pos / uvec3(quadrant_size.x, 1, quadrant_size.z);
        let fragment_index = (fragment_pos.x + fragment_pos.z * 2) as usize;

        new_indices[fragment_index][fragment_node_index] = Some(chunk_node_index);
        old_indices.push((fragment_index, fragment_node_index));

        if x == 0 || z == 0 || x == chunk_size.x as usize - 1 || z == chunk_size.z as usize - 1 {
            chunk_nodes.push(Superposition::single(
                fragments[fragment_index].2.graph.nodes[fragment_node_index],
            ));
        } else {
            chunk_nodes.push(fill_with.clone());
        }

        chunk_node_positions.push(
            fragments[fragment_index].1.node_positions[fragment_node_index]
                + fragment_size.as_ivec3() * fragment_pos.as_ivec3(),
        );
    }

    let mut chunk_node_neighbours = old_indices
        .iter()
        .map(|(fragment_index, fragment_node_index)| {
            fragments[*fragment_index].2.graph.neighbours[*fragment_node_index]
                .iter()
                .filter_map(|Neighbour { arc_type, index }| {
                    if let Some(index) = new_indices[*fragment_index][*index] {
                        Some(Neighbour {
                            arc_type: *arc_type,
                            index,
                        })
                    } else {
                        None
                    }
                })
                .collect_vec()
        })
        .collect::<Box<[Vec<_>]>>();

    for (chunk_node_index, (z, y, x)) in iproduct!(
        0..chunk_size.z as usize,
        0..chunk_size.y as usize,
        0..chunk_size.x as usize
    )
    .enumerate()
    {
        let chunk_node_pos = uvec3(x as u32, y as u32, z as u32);

        if x == quadrant_size.x as usize - 1 {
            let x_neighbour = uvec3_to_index(chunk_node_pos + UVec3::X, chunk_size);
            chunk_node_neighbours[chunk_node_index].push(Neighbour {
                arc_type: 0,
                index: x_neighbour,
            });
            chunk_node_neighbours[x_neighbour].push(Neighbour {
                arc_type: 1,
                index: chunk_node_index,
            });
        }
        if z == quadrant_size.z as usize - 1 {
            let z_neighbour = uvec3_to_index(chunk_node_pos + UVec3::Z, chunk_size);
            chunk_node_neighbours[chunk_node_index].push(Neighbour {
                arc_type: 4,
                index: z_neighbour,
            });
            chunk_node_neighbours[z_neighbour].push(Neighbour {
                arc_type: 5,
                index: chunk_node_index,
            });
        }
    }

    let chunk_node_neighbours = chunk_node_neighbours
        .into_iter()
        .map(|x| x.to_owned().into_boxed_slice())
        .collect::<Box<[Box<[Neighbour]>]>>();

    let chunk_node_order: Vec<usize> = (0..chunk_node_count).into_iter().collect_vec();

    (
        regular_grid_3d::GraphSettings {
            size: chunk_size,
            spacing: layout_settings.settings.spacing,
        },
        regular_grid_3d::GraphData {
            node_positions: chunk_node_positions.into(),
        },
        WfcGraph {
            nodes: chunk_nodes,
            neighbours: chunk_node_neighbours,
            order: chunk_node_order,
        },
    )
}

pub struct GenerationPlugin;
impl Plugin for GenerationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                systems::apply_loaded_fragments,
                systems::generate_chunks,
                systems::generate_fragments,
                systems::transform_chunk_loads,
            ),
        )
        .insert_resource(LayoutSettings {
            settings: regular_grid_3d::GraphSettings {
                size: uvec3(8, 1, 8),
                spacing: vec3(2.0, 3.0, 2.0),
            },
            tileset: LayoutTileset,
        })
        .init_resource::<LoadedChunks>()
        .init_resource::<LoadedFragments>()
        .init_resource::<GenerationDebugSettings>()
        .add_event::<FragmentGenerateEvent>()
        .add_event::<FragmentGeneratedEvent>()
        .add_event::<ChunkLoadEvent>()
        .add_event::<ChunkGenerateEvent>();
    }
}
