use std::{
    cell::{Ref, RefMut},
    ops::Deref,
};

use crate::{
    entity_store::{Entity, EntityId},
    world::World,
};

pub struct EntityViewDeferred<'a> {
    pub id: Entity,
    pub world: &'a World,
}

impl<'a> Deref for EntityViewDeferred<'a> {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl<'me> EntityViewDeferred<'me> {
    pub fn from_id_unchecked(world: &'me World, id: EntityId) -> Self {
        let entity = world.bookkeeping.entities.get_from_id(id);
        Self { id: entity, world }
    }
}
