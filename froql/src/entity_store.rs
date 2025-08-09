use std::cell::Cell;

use crate::archetype::{ArchetypeId, ArchetypeRow};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntityId(pub u32);
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntityGeneration(pub u32);

impl EntityId {
    fn as_index(self) -> usize {
        self.0 as usize
    }
}

impl EntityGeneration {
    fn is_alive(self) -> bool {
        debug_assert!(self.0 != 0); // Sentinel value
        self.0 % 2 == 1
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Entity {
    pub generation: EntityGeneration,
    pub id: EntityId,
}

// needed for some trickery in the macro
impl From<&Entity> for Entity {
    fn from(value: &Entity) -> Self {
        *value
    }
}

/// A generational Arena that can only store Entities
pub struct EntityStore {
    slots: Vec<EntitySlot>,
    next_free: usize,
    deferred_creations: Cell<usize>,
}

struct EntitySlot {
    archetype: ArchetypeId,
    /// row in the archetype
    /// if this slot is empty we use it as index for a free list
    row: ArchetypeRow,
    /// even is empty, uneven is filled
    generation: EntityGeneration,
}

impl EntitySlot {
    fn new() -> Self {
        // start with 1 so that we can use 0 as sentinel
        let generation = EntityGeneration(1);
        EntitySlot {
            archetype: SENTINEL_ARCHETYPE,
            row: SENTINEL_ARCHETYPE_ROW,
            generation,
        }
    }

    fn new_empty(previous_free: usize) -> Self {
        // start with 2 so that we can use 0 as sentinel
        let generation = EntityGeneration(2);
        let row = ArchetypeRow(previous_free as u32);
        EntitySlot {
            archetype: SENTINEL_ARCHETYPE,
            row,
            generation,
        }
    }

    fn is_empty(&self) -> bool {
        !self.generation.is_alive()
    }

    fn next_free(&self) -> usize {
        debug_assert!(self.is_empty());
        self.row.0 as usize
    }

    fn fill(&mut self) -> EntityGeneration {
        debug_assert!(self.is_empty());
        self.generation.0 = self.generation.0.wrapping_add(1);
        self.row = SENTINEL_ARCHETYPE_ROW;
        self.archetype = SENTINEL_ARCHETYPE;
        self.generation
    }

    fn empty_out(&mut self, previous_free: usize) -> EntityGeneration {
        debug_assert!(!self.is_empty());
        self.generation.0 = self.generation.0.wrapping_add(1);
        self.row.0 = previous_free as u32;
        self.generation
    }
}

const SENTINEL_ARCHETYPE: ArchetypeId = ArchetypeId(u32::MAX);
const SENTINEL_ARCHETYPE_ROW: ArchetypeRow = ArchetypeRow(u32::MAX);

impl EntityStore {
    pub fn new() -> Self {
        EntityStore {
            slots: Vec::new(),
            next_free: 0,
            deferred_creations: Cell::new(0),
        }
    }

    pub(crate) fn create(&mut self) -> Entity {
        if self.next_free >= self.slots.len() {
            let id = EntityId(self.slots.len() as u32);
            let slot = EntitySlot::new();
            let generation = slot.generation;
            self.slots.push(slot);
            self.next_free = self.slots.len();
            return Entity { generation, id };
        } else {
            let index = self.next_free;
            let slot = &mut self.slots[index];
            self.next_free = slot.next_free();
            let generation = slot.fill();
            let id = EntityId(index as u32);
            return Entity { generation, id };
        }
    }

    pub(crate) fn destroy(&mut self, e: Entity) {
        let index = e.id.0 as usize;
        if let Some(slot) = self.slots.get_mut(index) {
            if slot.generation != e.generation {
                return;
            }
            slot.empty_out(self.next_free);
            self.next_free = index;
        }
    }

    pub(crate) fn create_deferred(&self) -> Entity {
        let mut count = self.deferred_creations.get();
        self.deferred_creations.set(count + 1);

        let mut index = self.next_free;
        while count > 0 && index < self.slots.len() {
            count -= 1;
            let slot = &self.slots[index];
            index = slot.next_free();
        }

        if index < self.slots.len() {
            let slot = &self.slots[index];
            debug_assert!(self.slots[index].is_empty());
            Entity {
                generation: EntityGeneration(slot.generation.0.wrapping_add(1)),
                id: EntityId(index as u32),
            }
        } else {
            Entity {
                generation: EntityGeneration(1),
                id: EntityId((self.slots.len() + count) as u32),
            }
        }
    }

    /// the caller needs to call `create()` return value amounts of times
    /// kinda awkward, but otherwise we'd have to do extra allocations for temporaries
    /// or worse: think
    #[must_use]
    pub(crate) fn realize_deferred(&mut self) -> usize {
        self.deferred_creations.replace(0)
    }

    pub fn set_archetype(&mut self, e: Entity, id: ArchetypeId, row: ArchetypeRow) {
        let index = e.id.0 as usize;
        let slot = &mut self.slots[index];
        assert_eq!(slot.generation, e.generation);
        slot.archetype = id;
        slot.row = row;
    }

    #[doc(hidden)]
    /// sets archetype without checking/needing a generation
    /// for internal use only
    pub fn set_archetype_unchecked(&mut self, eid: EntityId, id: ArchetypeId, row: ArchetypeRow) {
        let index = eid.0 as usize;
        let slot = &mut self.slots[index];
        slot.archetype = id;
        slot.row = row;
    }

    pub fn is_alive(&self, e: Entity) -> bool {
        let index = e.id.0 as usize;
        self.slots
            .get(index)
            .map(|slot| slot.generation == e.generation)
            .unwrap_or(false)
    }

    pub fn get_archetype(&self, e: Entity) -> (ArchetypeId, ArchetypeRow) {
        let index = e.id.0 as usize;
        let slot = &self.slots[index];
        assert_eq!(slot.generation, e.generation);
        (slot.archetype, slot.row)
    }

    pub fn get_archetype_unchecked(&self, id: EntityId) -> (ArchetypeId, ArchetypeRow) {
        let index = id.0 as usize;
        let slot = &self.slots[index];
        assert!(slot.generation.is_alive(), "Entity in slot is not alive.");
        (slot.archetype, slot.row)
    }

    /// Returns Entity for EntityId
    /// Panics if that Entity is not alive.
    pub fn get_from_id(&self, id: EntityId) -> Entity {
        let index = id.0 as usize;
        let slot = &self.slots[index];
        assert!(slot.generation.is_alive(), "Entity in slot is not alive.");
        Entity {
            generation: slot.generation,
            id,
        }
    }

    /// If the entity was not alive before it needs to be moved into the correct archetype row
    #[must_use]
    pub(crate) fn force_alive(&mut self, id: EntityId) -> ForceAliveResult {
        let index = id.as_index();
        if index < self.slots.len() {
            let slot = &mut self.slots[index];
            if slot.generation.is_alive() {
                return ForceAliveResult::WasAliveBefore(Entity {
                    generation: slot.generation,
                    id,
                });
            } else if self.next_free == index {
                self.next_free = slot.next_free();
                slot.fill();
                return ForceAliveResult::MadeAlive(Entity {
                    generation: slot.generation,
                    id,
                });
            } else {
                // update free list, because we may not force the head to be alive
                // but an entity somewhere in the middle of the free list or the end
                let mut prev = self.next_free;
                while self.slots[prev].next_free() != index {
                    prev = self.slots[prev].next_free();
                }
                self.slots[prev].row.0 = self.slots[index].row.0;
                let slot = &mut self.slots[index];
                slot.fill();
                return ForceAliveResult::MadeAlive(Entity {
                    generation: slot.generation,
                    id,
                });
            }
        } else {
            while index >= self.slots.len() {
                if self.next_free == self.slots.len() {
                    // the slot is still empty, so don't need to update self.next_free
                    // u32::MAX is more or less always bigger than the end of the slotarray
                    self.slots.push(EntitySlot::new_empty(u32::MAX as usize));
                } else {
                    self.slots.push(EntitySlot::new_empty(self.next_free));
                    self.next_free = self.slots.len() - 1;
                }
            }
            let slot = &mut self.slots[index];
            self.next_free = slot.next_free();
            slot.fill();
            return ForceAliveResult::MadeAlive(Entity {
                generation: slot.generation,
                id,
            });
        }
    }
}

pub(crate) enum ForceAliveResult {
    MadeAlive(Entity),
    WasAliveBefore(Entity),
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_struct_sizes() {
        assert_eq!(8, size_of::<Entity>());
        assert_eq!(12, size_of::<EntitySlot>());
    }

    #[test]
    fn entity_create() {
        let mut store = EntityStore::new();
        for _ in 0..10 {
            store.create();
        }
        let e = store.create();
        assert_eq!(10, e.id.0);
        assert_eq!(1, e.generation.0);
    }

    #[test]
    fn entity_create_deferred() {
        let mut store = EntityStore::new();
        for _ in 0..10 {
            store.create();
        }
        let e = store.create();
        assert_eq!(11, store.next_free);
        store.destroy(e);
        assert_eq!(10, store.next_free);

        let _e = store.create_deferred();
        let e = store.create_deferred();
        assert_eq!(11, e.id.0);
        assert_eq!(1, e.generation.0);
    }

    #[test]
    fn entity_reuse() {
        let mut store = EntityStore::new();
        let e = store.create();
        store.destroy(e);
        let e = store.create();
        assert_eq!(0, e.id.0);
        assert_eq!(3, e.generation.0);
        store.destroy(e);
        let e = store.create();
        assert_eq!(0, e.id.0);
        assert_eq!(5, e.generation.0);
    }

    #[test]
    fn force_alive() {
        let mut store = EntityStore::new();
        let e1 = store.create();
        assert_eq!(0, e1.id.0);
        assert_eq!(1, e1.generation.0);
        let e = match store.force_alive(EntityId(5)) {
            ForceAliveResult::MadeAlive(entity) => entity,
            ForceAliveResult::WasAliveBefore(_) => unreachable!(),
        };
        assert_eq!(5, e.id.0);
        assert_eq!(3, e.generation.0);

        let e = store.create();
        assert_eq!(4, e.id.0);
        assert_eq!(3, e.generation.0);
        let e = match store.force_alive(e.id) {
            ForceAliveResult::MadeAlive(_) => unreachable!("Wrong result."),
            ForceAliveResult::WasAliveBefore(ent) => ent,
        };
        assert_eq!(4, e.id.0);
        assert_eq!(3, e.generation.0);

        assert_eq!(3, store.create().id.0);
        assert_eq!(2, store.create().id.0);
        assert_eq!(1, store.create().id.0);
        assert_eq!(6, store.create().id.0);
    }

    #[test]
    fn force_alive_twice() {
        let mut store = EntityStore::new();
        let e1 = store.create();
        assert_eq!(0, e1.id.0);
        assert_eq!(1, e1.generation.0);
        let e = match store.force_alive(EntityId(5)) {
            ForceAliveResult::MadeAlive(entity) => entity,
            ForceAliveResult::WasAliveBefore(_) => unreachable!(),
        };
        assert_eq!(5, e.id.0);
        assert_eq!(3, e.generation.0);

        let e2 = match store.force_alive(EntityId(3)) {
            ForceAliveResult::MadeAlive(entity) => entity,
            ForceAliveResult::WasAliveBefore(_) => unreachable!(),
        };
        assert_eq!(3, e2.id.0);
        assert_eq!(3, e2.generation.0);

        assert_eq!(4, store.create().id.0);
        assert_eq!(2, store.create().id.0);
        assert_eq!(1, store.create().id.0);
        assert_eq!(6, store.create().id.0);
        assert_eq!(7, store.create().id.0);
    }

    // this is a regression test
    #[test]
    fn force_alive_and_defer() {
        let mut store = EntityStore::new();
        let e1 = store.create();
        assert_eq!(0, e1.id.0);
        assert_eq!(1, e1.generation.0);
        let e = match store.force_alive(EntityId(2)) {
            ForceAliveResult::MadeAlive(entity) => entity,
            ForceAliveResult::WasAliveBefore(_) => unreachable!(),
        };
        assert_eq!(2, e.id.0);
        assert_eq!(1, store.create().id.0);
        assert_eq!(3, store.create_deferred().id.0);
        assert_eq!(4, store.create_deferred().id.0);
        assert_eq!(5, store.create_deferred().id.0);
    }
}
