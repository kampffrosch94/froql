use std::fmt::{self, Debug};

use crate::{entity_store::Entity, relation_vec::RelationVec, world::World};

enum ComponentDebugHelper<'a> {
    DebugFn {
        ptr: *const u8,
        debug_fn: fn(*const u8, &mut fmt::Formatter<'_>) -> Result<(), fmt::Error>,
    },
    JustName(&'a str),
}

impl Debug for ComponentDebugHelper<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DebugFn { debug_fn, ptr } => (debug_fn)(ptr.clone(), f),
            Self::JustName(name) => f.debug_struct(name).finish_non_exhaustive(),
        }
    }
}

pub fn debug_view(
    f: &mut fmt::Formatter,
    world: &World,
    entity: Entity,
    name: &str,
) -> fmt::Result {
    let bk = &world.bookkeeping;
    let (aid, _row) = bk.entities.get_archetype(entity);
    let a = &bk.archetypes[aid.0 as usize];

    let mut builder = f.debug_struct(name);
    builder
        .field("id", &entity.id)
        .field("generation", &entity.generation);
    let mut components = Vec::new();
    for comp_id in &a.components {
        let comp = &bk.components[comp_id.as_index()];
        if let Some(debug_fn) = comp.debug_fn {
            let ptr = bk.get_component(entity, *comp_id);
            let helper = ComponentDebugHelper::DebugFn { ptr, debug_fn };
            components.push(helper);
        } else {
            if comp_id.is_relation() {
                let ptr = bk.get_component(entity, *comp_id) as *const RelationVec;
                let rel_vec = unsafe { &*ptr };
                let name = &comp.name;
                if comp_id.is_target() {
                    builder.field(&format!("{name}<target> of"), &&rel_vec[..]);
                } else {
                    builder.field(&format!("{name}<origin> to"), &&rel_vec[..]);
                }
            } else {
                let helper = ComponentDebugHelper::JustName(&comp.name);
                components.push(helper);
            }
        }
    }
    builder.field("components", &components);
    builder.finish()
}
