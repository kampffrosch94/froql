#![allow(dead_code)]

#[derive(Clone, Copy, Debug)]
struct EntityId(u32);
#[derive(Clone, Copy, Debug, PartialEq)]
struct EntityGeneration(u32);
#[derive(Clone, Copy, Debug)]
struct ArchetypeId(u32);
#[derive(Clone, Copy, Debug)]
struct ArchetypeRow(u32);

#[derive(Clone, Copy, Debug)]
pub struct Entity {
    gen: EntityGeneration,
    id: EntityId,
}

/// A generational Arena that can only store Entities
pub struct EntityStore {
    slots: Vec<EntitySlot>,
    next_free: usize,
}

struct EntitySlot {
    archetype: ArchetypeId,
    /// row in the archetype
    /// if this slot is empty we use it as index for a free list
    row: ArchetypeRow,
    /// even is empty, uneven is filled
    gen: EntityGeneration,
}

impl EntitySlot {
    fn new() -> Self {
        // start with 1 so that we can use 0 as sentinel
        let gen = EntityGeneration(1);
        EntitySlot {
            archetype: EMPTY_ARCHETYPE,
            row: EMPTY_ARCHETYPE_ROW,
            gen,
        }
    }

    fn is_empty(&self) -> bool {
        self.gen.0 % 2 == 0
    }

    fn next_free(&self) -> usize {
        self.row.0 as usize
    }

    fn fill(&mut self) -> EntityGeneration {
        debug_assert!(self.is_empty());
        self.gen.0 = self.gen.0.wrapping_add(1);
        self.row = EMPTY_ARCHETYPE_ROW;
        self.archetype = EMPTY_ARCHETYPE;
        self.gen
    }

    fn empty_out(&mut self, previous_free: usize) -> EntityGeneration {
        debug_assert!(!self.is_empty());
        self.gen.0 = self.gen.0.wrapping_add(1);
        self.row.0 = previous_free as u32;
        self.gen
    }
}

const EMPTY_ARCHETYPE: ArchetypeId = ArchetypeId(0);
const EMPTY_ARCHETYPE_ROW: ArchetypeRow = ArchetypeRow(0);

impl EntityStore {
    pub fn new() -> Self {
        EntityStore {
            slots: Vec::new(),
            next_free: 0,
        }
    }

    pub fn create(&mut self) -> Entity {
        if self.next_free >= self.slots.len() {
            let id = EntityId(self.slots.len() as u32);
            let slot = EntitySlot::new();
            let gen = slot.gen;
            self.slots.push(slot);
            self.next_free += 1;
            return Entity { gen, id };
        } else {
            let index = self.next_free;
            let slot = &mut self.slots[index];
            self.next_free = slot.next_free();
            let gen = slot.fill();
            let id = EntityId(index as u32);
            return Entity { gen, id };
        }
    }

    pub fn destroy(&mut self, e: Entity) {
        let index = e.id.0 as usize;
        if let Some(slot) = self.slots.get_mut(index) {
            if slot.gen != e.gen {
                return;
            }
            slot.empty_out(self.next_free);
            self.next_free = index;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_struct_sizes() {
        assert_eq!(8, size_of::<Entity>());
        assert_eq!(12, size_of::<EntitySlot>());
    }
}
