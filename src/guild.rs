// use crate::{Executer, Peasant};
// use anyhow::Result;
// use bevy::utils::HashMap;
// use std::any::TypeId;

// pub struct Guild {
//     executers: HashMap<TypeId, Box<dyn Executer>>,
// }

// impl Guild {
//     pub fn queue_peasant<E: Executer + 'static>(&mut self, peasant: Peasant) -> Result<()> {
//         let type_id = TypeId::of::<E>();
//         if !self.executers.contains_key(&type_id) {
//             self.executers.insert(type_id, Box::new(E::default()));
//         }

//         self.executers[&type_id].queue_peasant(peasant);

//         Ok(())
//     }
// }
