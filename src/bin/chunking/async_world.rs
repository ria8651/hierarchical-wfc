use std::any::TypeId;

use bevy::prelude::*;
use tokio::sync::broadcast;

pub trait AsyncEvent {}

pub struct Channel<T: AsyncEvent> {
    tx: broadcast::Receiver<T>,
    rx: broadcast::Receiver<T>,
}

#[derive(Resource)]
pub struct AsyncWorld {
    // pub channels: HashMap<TypeId, Box>,
}

impl AsyncWorld {
    pub fn add_event<T: AsyncEvent>(&self) {}

    fn init_system() {}
}
