use bevy::prelude::*;

#[derive(Default, Component, Clone)]
pub struct RegenerateSettings {
    pub min: Vec3,
    pub max: Vec3,
}
