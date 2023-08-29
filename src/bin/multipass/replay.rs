use bevy::prelude::*;

use hierarchical_wfc::{
    materials::tile_pbr_material::TilePbrMaterial, wfc::bevy_passes::WfcFCollapsedData,
};

#[derive(Component)]
pub struct ReplayPassProgress {
    pub remainder: f32,
    pub current: usize,
    pub length: usize,
    pub duration: f32,
    pub playing: bool,
}
impl Default for ReplayPassProgress {
    fn default() -> Self {
        Self {
            remainder: 0.0,
            current: 0,
            length: 0,
            duration: 2.5,
            playing: false,
        }
    }
}

#[derive(Component)]
pub struct ReplayOrder(pub usize);

pub fn replay_generation_system(
    mut commands: Commands,
    mut q_passes: Query<(
        &mut ReplayPassProgress,
        &WfcFCollapsedData,
        Option<&ReplayTileMapMaterials>,
        &Children,
    )>,
    q_blocks: Query<&mut DebugBlocks>,
    q_tiles: Query<(Entity, &ReplayOrder)>,
    time: Res<Time>,
    mut tile_materials: ResMut<Assets<TilePbrMaterial>>,
) {
    for (mut progress, _collapsed_data, materials, children) in q_passes.iter_mut() {
        for DebugBlocks { material_handle } in q_blocks.iter_many(children) {
            if let Some(material) = tile_materials.get_mut(&material_handle) {
                material.order_cut_off = progress.current as u32;
            };
        }

        for (tile_entity, ReplayOrder(tile_order)) in q_tiles.iter_many(children) {
            commands
                .entity(tile_entity)
                .insert(if tile_order >= &progress.current {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                });
        }

        if let Some(ReplayTileMapMaterials(materials)) = materials {
            for material in materials.iter() {
                tile_materials
                    .get_mut(material)
                    .expect("Entity with non-existent tilemap material")
                    .order_cut_off = progress.current as u32;
            }
        }

        if progress.playing {
            progress.remainder +=
                time.delta_seconds() * (progress.length as f32 / progress.duration);
            progress.current += progress.remainder as usize;
            progress.remainder = progress.remainder.rem_euclid(1.0);

            if progress.current >= progress.length {
                progress.current = progress.length;
                progress.playing = false;
                progress.remainder = 0.0;
            }
        }
    }
}

#[derive(Component)]
pub struct DebugBlocks {
    pub material_handle: Handle<TilePbrMaterial>,
}

#[derive(Component)]
pub struct ReplayTileMapMaterials(pub Vec<Handle<TilePbrMaterial>>);
