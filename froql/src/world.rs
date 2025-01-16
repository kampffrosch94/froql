use std::{
    any::TypeId,
    cell::{Ref, RefCell, RefMut},
};

use crate::{
    bookkeeping::Bookkeeping,
    component::{Component, ComponentId, RELATION},
    entity_store::Entity,
    entity_view_mut::EntityViewMut,
    relation::Relation,
};

pub struct World {
    pub bookkeeping: Bookkeeping,
}

impl World {
    pub fn new() -> Self {
        World {
            bookkeeping: Bookkeeping::new(),
        }
    }

    /// Used internally to register both components and relations
    /// because Relations are a special kind of component
    /// and Components are meant to be wrapped in `RefCell`
    fn register_component_inner<T: 'static>(&mut self, flags: u32) -> ComponentId {
        let tid = TypeId::of::<T>();
        if let Some(cid) = self.bookkeeping.get_component_id(tid) {
            return cid;
        }
        let mut cid = ComponentId::from_usize(self.bookkeeping.components.len());
        cid = cid.set_flags(flags);
        self.bookkeeping.components.push(Component::new::<T>(cid));
        self.bookkeeping.component_map.insert(tid, cid);
        return cid;
    }

    pub fn register_component<T: 'static>(&mut self) -> ComponentId {
        if size_of::<T>() > 0 {
            self.register_component_inner::<RefCell<T>>(0)
        } else {
            self.register_component_inner::<T>(0)
        }
    }

    // mostly there for use in query
    #[doc(hidden)]
    pub fn get_component_id<T: 'static>(&self) -> ComponentId {
        let tid = if size_of::<T>() > 0 {
            TypeId::of::<RefCell<T>>()
        } else {
            TypeId::of::<T>()
        };
        self.bookkeeping
            .get_component_id(tid)
            // TODO general error msg handler for T
            .unwrap_or_else(|| panic!("ComponentType is not registered."))
    }

    pub fn create(&mut self) -> Entity {
        self.bookkeeping.create()
    }

    pub fn create_mut(&mut self) -> EntityViewMut {
        EntityViewMut {
            id: self.bookkeeping.create(),
            world: self,
        }
    }

    pub fn is_alive(&self, e: Entity) -> bool {
        self.bookkeeping.is_alive(e)
    }

    pub fn add_component<T: 'static>(&mut self, e: Entity, val: T) {
        let cid = self.register_component::<T>();

        if size_of::<T>() > 0 {
            let val = RefCell::new(val);
            let dst = self.bookkeeping.add_component(e, cid) as *mut RefCell<T>;
            unsafe {
                std::ptr::write(dst, val);
            }
        } else {
            self.bookkeeping.add_component_zst(e, cid);
        }
    }

    #[track_caller]
    pub fn get_component<T: 'static>(&self, e: Entity) -> Ref<T> {
        if size_of::<T>() > 0 {
            let tid = TypeId::of::<RefCell<T>>();
            let cid = self.bookkeeping.get_component_id(tid).unwrap(); // TODO error msg
            let ptr = self.bookkeeping.get_component(e, cid) as *const RefCell<T>;
            let cell = unsafe { &*ptr };
            cell.borrow()
        } else {
            // if we don't panic here we can't return a Ref<T> in the other branch
            // with `generic_const_exprs` this could be a compile time error
            panic!("Can't get reference to ZST component.")
        }
    }

    #[track_caller]
    pub fn get_component_mut<T: 'static>(&self, e: Entity) -> RefMut<T> {
        if size_of::<T>() > 0 {
            let tid = TypeId::of::<RefCell<T>>();
            let cid = self.bookkeeping.get_component_id(tid).unwrap(); // TODO error msg
            let ptr = self.bookkeeping.get_component(e, cid) as *const RefCell<T>;
            let cell = unsafe { &*ptr };
            cell.borrow_mut()
        } else {
            // if we don't panic here we can't return a Ref<T> in the other branch
            // with `generic_const_exprs` this could be a compile time error
            panic!("Can't get reference to ZST component.")
        }
    }

    #[track_caller]
    pub fn has_component<T: 'static>(&self, e: Entity) -> bool {
        let tid: TypeId = if size_of::<T>() > 0 {
            TypeId::of::<RefCell<T>>()
        } else {
            TypeId::of::<T>()
        };
        let cid = self.bookkeeping.get_component_id(tid).unwrap(); // TODO error msg
        self.bookkeeping.has_component(e, cid)
    }

    #[track_caller]
    pub fn remove_component<T: 'static>(&mut self, e: Entity) {
        let tid: TypeId = if size_of::<T>() > 0 {
            TypeId::of::<RefCell<T>>()
        } else {
            TypeId::of::<T>()
        };
        let cid = self.bookkeeping.get_component_id(tid).unwrap(); // TODO error msg
        self.bookkeeping.remove_component(e, cid);
    }

    pub fn destroy(&mut self, e: Entity) {
        self.bookkeeping.destroy(e);
    }
}

// relation stuff in separate impl block
impl World {
    pub fn register_relation<T: 'static>(&mut self) {
        self.register_component_inner::<Relation<T>>(RELATION);
    }

    pub fn register_relation_flags<T: 'static>(&mut self, flags: u32) {
        // TODO: error if component is already registered
        self.register_component_inner::<Relation<T>>(flags | RELATION);
    }

    pub fn add_relation<T: 'static>(&mut self, from: Entity, to: Entity) {
        let origin_cid = self.register_component_inner::<Relation<T>>(RELATION);
        self.bookkeeping.add_relation(origin_cid, from, to);
    }

    pub fn has_relation<T: 'static>(&self, from: Entity, to: Entity) -> bool {
        let o_tid = TypeId::of::<Relation<T>>();
        let origin_cid = self.bookkeeping.get_component_id(o_tid).unwrap(); // TODO error msg
        self.bookkeeping.has_relation(origin_cid, from, to)
    }

    pub fn remove_relation<T: 'static>(&mut self, from: Entity, to: Entity) {
        let cid = self.register_component_inner::<Relation<T>>(RELATION);
        self.bookkeeping.remove_relation(cid, from, to);
    }

    pub fn relation_targets<'a, T: 'static>(
        &'a self,
        from: Entity,
    ) -> impl Iterator<Item = Entity> + use<'a, T> {
        let o_tid = TypeId::of::<Relation<T>>();
        let origin_cid = self.bookkeeping.get_component_id(o_tid).unwrap(); // TODO error msg
        self.bookkeeping
            .relation_partners(origin_cid, from)
            .into_iter()
            .flat_map(|it| it)
    }

    pub fn relation_origins<'a, T: 'static>(
        &'a self,
        to: Entity,
    ) -> impl Iterator<Item = Entity> + use<'a, T> {
        let tid = TypeId::of::<Relation<T>>();
        let target_cid = self
            .bookkeeping
            .get_component_id(tid)
            .unwrap() // TODO error msg
            .flip_target();
        self.bookkeeping
            // same logic as with target, just different parameter
            .relation_partners(target_cid, to)
            .into_iter()
            .flat_map(|it| it)
    }
}

#[cfg(test)]
mod test {
    use crate::query;

    use crate::{
        archetype::ArchetypeId,
        component::{CASCADING_DESTRUCT, EXCLUSIVE, SYMMETRIC},
        entity_store::EntityId,
        relation_vec::RelationVec,
    };

    use super::*;

    #[test]
    fn create_and_get() {
        struct Pos(i32, i32);
        struct Name(String);

        let mut world = World::new();
        let e = world.create();
        world.add_component(e, Pos(4, 2));
        world.add_component(e, Name("Player".to_string()));
        let other = world.create();
        world.add_component(other, Pos(5, 4));
        world.add_component(other, Name("Other".to_string()));

        let pos = world.get_component::<Pos>(e);
        let name = world.get_component::<Name>(e);
        assert_eq!(pos.0, 4);
        assert_eq!(pos.1, 2);
        assert_eq!(name.0, "Player");
        let pos = world.get_component::<Pos>(other);
        let name = world.get_component::<Name>(other);
        assert_eq!(pos.0, 5);
        assert_eq!(pos.1, 4);
        assert_eq!(name.0, "Other");
    }

    #[test]
    fn create_remove_get() {
        struct Pos(i32, i32);
        struct Name(String);

        let mut world = World::new();
        let e = world.create();
        world.add_component(e, Pos(4, 2));
        world.add_component(e, Name("Player".to_string()));
        assert!(world.has_component::<Pos>(e));
        assert!(world.has_component::<Name>(e));
        let other = world.create();
        world.add_component(other, Pos(5, 4));
        world.add_component(other, Name("Other".to_string()));

        world.remove_component::<Pos>(e);
        world.remove_component::<Name>(e);
        assert!(!world.has_component::<Pos>(e));
        assert!(!world.has_component::<Name>(e));

        let pos = world.get_component::<Pos>(other);
        let name = world.get_component::<Name>(other);
        assert_eq!(pos.0, 5);
        assert_eq!(pos.1, 4);
        assert_eq!(name.0, "Other");
    }

    #[test]
    fn create_destroy_get() {
        struct Pos(i32, i32);
        struct Name(String);

        let mut world = World::new();
        let e = world.create();
        world.add_component(e, Pos(4, 2));
        world.add_component(e, Name("Player".to_string()));
        let other = world.create();
        world.add_component(other, Pos(5, 4));
        world.add_component(other, Name("Other".to_string()));

        world.destroy(e);

        let pos = world.get_component::<Pos>(other);
        let name = world.get_component::<Name>(other);
        assert_eq!(pos.0, 5);
        assert_eq!(pos.1, 4);
        assert_eq!(name.0, "Other");
    }

    #[test]
    fn component_mut() {
        struct Pos(i32, i32);

        let mut world = World::new();
        let e = world.create();
        world.add_component(e, Pos(4, 2));
        let pos = world.get_component::<Pos>(e);
        assert_eq!(pos.0, 4);
        assert_eq!(pos.1, 2);
        drop(pos); // need to release ref

        let mut pos = world.get_component_mut::<Pos>(e);
        pos.0 = 20;
        pos.1 = 30;
        drop(pos); // need to release refmut

        let pos = world.get_component::<Pos>(e);
        assert_eq!(pos.0, 20);
        assert_eq!(pos.1, 30);
    }

    #[test]
    fn relation_simple() {
        enum Rel {}

        let mut world = World::new();
        world.register_relation::<Rel>();
        let a = world.create();
        let b = world.create();
        assert!(!world.has_relation::<Rel>(a, b));
        world.add_relation::<Rel>(a, b);
        assert!(world.has_relation::<Rel>(a, b));
        world.remove_relation::<Rel>(a, b);
        assert!(!world.has_relation::<Rel>(a, b));

        // removing multiple times is no problem
        world.remove_relation::<Rel>(a, b);
        world.remove_relation::<Rel>(a, b);
        world.remove_relation::<Rel>(a, b);
        assert!(!world.has_relation::<Rel>(a, b));
    }

    #[test]
    fn relation_check_one_side() {
        enum Rel {}

        let mut world = World::new();
        world.register_relation::<Rel>();
        let a = world.create();
        let b = world.create();
        world.add_relation::<Rel>(a, b);
        let targets: Vec<Entity> = world.relation_targets::<Rel>(a).collect();
        assert_eq!(&[b][..], &targets[..]);
        let origins: Vec<Entity> = world.relation_origins::<Rel>(b).collect();
        assert_eq!(&[a][..], &origins[..]);
    }

    #[test]
    fn relation_entity_destroy() {
        enum Rel {}

        let mut world = World::new();
        world.register_relation::<Rel>();
        let a = world.create();
        let b = world.create();
        assert!(!world.has_relation::<Rel>(a, b));
        world.add_relation::<Rel>(a, b);
        assert!(world.has_relation::<Rel>(a, b));
        assert_eq!(1, world.relation_origins::<Rel>(b).count());
        world.destroy(a);
        assert_eq!(0, world.relation_origins::<Rel>(b).count());
        assert!(!world.has_relation::<Rel>(a, b));
    }

    #[test]
    fn relation_exlusive() {
        enum Rel {}

        let mut world = World::new();
        world.register_relation_flags::<Rel>(EXCLUSIVE);
        let a = world.create();
        let b = world.create();
        let c = world.create();
        assert!(!world.has_relation::<Rel>(a, b));
        assert!(!world.has_relation::<Rel>(a, c));
        world.add_relation::<Rel>(a, b);
        assert!(world.has_relation::<Rel>(a, b));
        assert!(!world.has_relation::<Rel>(a, c));
        world.add_relation::<Rel>(a, c);
        assert!(world.has_relation::<Rel>(a, c));
        assert!(!world.has_relation::<Rel>(a, b));
    }

    #[test]
    fn relation_asymmetric() {
        enum Rel {}
        let mut world = World::new();
        world.register_relation_flags::<Rel>(0);
        let a = world.create();
        let b = world.create();
        world.add_relation::<Rel>(a, b);
        assert!(world.has_relation::<Rel>(a, b));
        assert!(!world.has_relation::<Rel>(b, a));
    }

    #[test]
    fn relation_symmetric() {
        enum Rel {}

        let mut world = World::new();
        world.register_relation_flags::<Rel>(SYMMETRIC);
        let a = world.create();
        let b = world.create();
        assert!(!world.has_relation::<Rel>(a, b));
        assert!(!world.has_relation::<Rel>(b, a));
        world.add_relation::<Rel>(a, b);
        assert!(world.has_relation::<Rel>(a, b));
        assert!(world.has_relation::<Rel>(b, a));
    }

    #[test]
    fn relation_cascading() {
        enum Rel {}

        let mut world = World::new();
        world.register_relation_flags::<Rel>(CASCADING_DESTRUCT);
        let a = world.create();
        let b = world.create();
        world.add_relation::<Rel>(a, b);

        assert!(world.has_relation::<Rel>(a, b));
        assert!(world.is_alive(a));
        assert!(world.is_alive(b));

        world.destroy(a);
        assert!(!world.is_alive(a));
        assert!(!world.is_alive(b));
    }

    #[test]
    fn zst_component() {
        struct Comp {}

        let mut world = World::new();
        world.register_component::<Comp>();
        let a = world.create();
        assert!(!world.has_component::<Comp>(a));
        world.add_component(a, Comp {});
        assert!(world.has_component::<Comp>(a));
        world.remove_component::<Comp>(a);
        assert!(!world.has_component::<Comp>(a));
    }

    // TODO move into integration test
    #[test]
    fn manual_query_trivial() {
        #[derive(Debug)]
        struct CompA(usize);
        #[derive(Debug)]
        struct CompB(String);
        struct CompC {}

        let mut world = World::new();
        let a = world.create();
        world.add_component(a, CompA(42));
        world.add_component(a, CompB("Hello".to_string()));
        let b = world.create();
        world.add_component(b, CompA(21));
        let c = world.create();
        world.add_component(c, CompA(42));
        world.add_component(c, CompB("Hello".to_string()));
        world.add_component(c, CompC {});

        let mut counter = 0;
        for (comp_a, comp_b) in {
            let world: &World = &world;
            let bk = &world.bookkeeping;
            let components = [
                world.get_component_id::<CompA>(),
                world.get_component_id::<CompB>(),
            ];
            let archetype_ids = bk.matching_archetypes(&components, &[]);
            assert_eq!(archetype_ids.len(), 2);
            archetype_ids.into_iter().flat_map(move |aid| {
                let arch = &bk.archetypes[aid.0 as usize];
                let mut col_ids = [usize::MAX; 2];
                arch.find_multiple_columns(&components, &mut col_ids);
                (0..(&arch.columns[col_ids[0]]).len()).map(move |row| unsafe {
                    (
                        (&*((&arch.columns[col_ids[0]]).get(row) as *const RefCell<CompA>))
                            .borrow(),
                        (&*((&arch.columns[col_ids[1]]).get(row) as *const RefCell<CompB>))
                            .borrow(),
                    )
                })
            })
        } {
            println!("{comp_a:?}");
            println!("{comp_b:?}");
            assert_eq!(42, comp_a.0);
            assert_eq!("Hello", &comp_b.0);
            counter += 1;
        }
        assert_eq!(2, counter);
    }

    #[test]
    fn manual_query_relation() {
        enum Attack {}

        #[derive(Debug)]
        struct Unit(String);
        #[derive(Debug)]
        struct Health(isize);

        let mut world = World::new();
        let player = world.create();
        world.add_component(player, Unit("Player".to_string()));
        let goblin_a = world.create();
        world.add_component(goblin_a, Health(10));
        world.add_component(goblin_a, Unit("Goblin A".to_string()));
        world.add_relation::<Attack>(player, goblin_a);

        let goblin_b = world.create();
        world.add_component(goblin_b, Health(10));
        world.add_component(goblin_b, Unit("Goblin B".to_string()));
        world.add_relation::<Attack>(player, goblin_b);

        // this should not be matched by the query below
        // bad example I know, but I need something
        let trap = world.create();
        world.add_relation::<Attack>(trap, goblin_b);

        let origins_a: Vec<Entity> = world.relation_origins::<Attack>(goblin_a).collect();
        assert_eq!(&[player], origins_a.as_slice());
        let origins_b: Vec<Entity> = world.relation_origins::<Attack>(goblin_b).collect();
        assert_eq!(&[player, trap], origins_b.as_slice());

        let mut counter = 0;

        // manual query for:
        // query!(world, Unit(me), Unit(other), Hp(me), Attack(other, me))
        for (me, other, mut hp) in {
            let world: &World = &world;
            let bk = &world.bookkeeping;
            let components_me = [
                world.get_component_id::<Unit>(),
                world.get_component_id::<Health>(),
                bk.get_component_id_unchecked(TypeId::of::<Relation<Attack>>())
                    .flip_target(),
            ];
            let components_other = [
                world.get_component_id::<Unit>(),
                bk.get_component_id_unchecked(TypeId::of::<Relation<Attack>>()),
            ];
            let archetype_ids_me = bk.matching_archetypes(&components_me, &[]);
            let archetype_ids_other = bk.matching_archetypes(&components_other, &[]);

            assert_eq!(1, archetype_ids_me.len());
            assert_eq!(1, archetype_ids_other.len());

            archetype_ids_me.into_iter().flat_map(move |aid| {
                let arch_me = &bk.archetypes[aid.0 as usize];
                let mut col_ids_me = [usize::MAX; 3];
                arch_me.find_multiple_columns(&components_me, &mut col_ids_me);
                // need to clone before moving
                let archetype_ids_other = archetype_ids_other.clone();
                (0..(&arch_me.columns[col_ids_me[0]]).len()).flat_map(move |row_me| unsafe {
                    let rel_attack =
                        &*((&arch_me.columns[col_ids_me[2]]).get(row_me) as *const RelationVec);
                    assert!(
                        rel_attack.len() > 0,
                        "Entity should not be in archetype if it has no relation"
                    );
                    // need to clone before moving - Again :/
                    let archetype_ids_other = archetype_ids_other.clone();
                    rel_attack
                        .iter()
                        .map(|id_raw| {
                            let id = EntityId(*id_raw);
                            bk.entities.get_archetype_unchecked(id).0
                        })
                        .filter(move |id: &ArchetypeId| archetype_ids_other.contains(id))
                        .flat_map(move |other_id| {
                            let arch_other = &bk.archetypes[other_id.0 as usize];
                            let mut col_ids_other = [usize::MAX; 1];
                            // don't actually need the relations col here, so can slice it off
                            arch_other
                                .find_multiple_columns(&components_other[0..1], &mut col_ids_other);
                            (0..(&arch_other.columns[col_ids_other[0]]).len()).map(
                                move |row_other| {
                                    (
                                        (&*((&arch_me.columns[col_ids_me[0]]).get(row_me)
                                            as *const RefCell<Unit>))
                                            .borrow(),
                                        (&*((&arch_other.columns[col_ids_other[0]]).get(row_other)
                                            as *const RefCell<Unit>))
                                            .borrow(),
                                        (&*((&arch_me.columns[col_ids_me[1]]).get(row_me)
                                            as *const RefCell<Health>))
                                            .borrow_mut(),
                                    )
                                },
                            )
                        })
                })
            })
        } {
            println!("{me:?} attacked by {other:?}");
            hp.0 -= 5;
            println!("Hp now: {hp:?}");
            counter += 1;
        }
        assert_eq!(2, counter);
    }

    use crate::archetype::ArchetypeRow; // TODO something about that
    #[test]
    fn proc_query_relation() {
        enum Attack {}

        #[derive(Debug)]
        struct Unit(String);
        #[derive(Debug)]
        struct Health(isize);

        let mut world = World::new();
        let player = world.create();
        world.add_component(player, Unit("Player".to_string()));
        let goblin_a = world.create();
        world.add_component(goblin_a, Health(10));
        world.add_component(goblin_a, Unit("Goblin A".to_string()));
        world.add_relation::<Attack>(player, goblin_a);

        let goblin_b = world.create();
        world.add_component(goblin_b, Health(10));
        world.add_component(goblin_b, Unit("Goblin B".to_string()));
        world.add_relation::<Attack>(player, goblin_b);

        // this should not be matched by the query below
        // bad example I know, but I need something
        let trap = world.create();
        world.add_relation::<Attack>(trap, goblin_b);

        let origins_a: Vec<Entity> = world.relation_origins::<Attack>(goblin_a).collect();
        assert_eq!(&[player], origins_a.as_slice());
        let origins_b: Vec<Entity> = world.relation_origins::<Attack>(goblin_b).collect();
        assert_eq!(&[player, trap], origins_b.as_slice());

        let mut counter = 0;

        for (me, other, mut hp) in query!(world, Unit(me), Unit(other),
                                          mut Health(me), Attack(other, me))
        {
            println!("{me:?} attacked by {other:?}");
            hp.0 -= 5;
            println!("Hp now: {hp:?}");
            counter += 1;
        }
        assert_eq!(2, counter);
    }

    #[test]
    fn proc_query_trivial() {
        #[derive(Debug)]
        struct CompA(usize);
        #[derive(Debug)]
        struct CompB(String);
        struct CompC {}

        let mut world = World::new();
        let a = world.create();
        world.add_component(a, CompA(42));
        world.add_component(a, CompB("Hello".to_string()));
        let b = world.create();
        world.add_component(b, CompA(21));
        let c = world.create();
        world.add_component(c, CompA(42));
        world.add_component(c, CompB("Hello".to_string()));
        world.add_component(c, CompC {});

        let mut counter = 0;
        for (comp_a, comp_b) in query!(world, CompA, CompB) {
            println!("{comp_a:?}");
            println!("{comp_b:?}");
            assert_eq!(42, comp_a.0);
            assert_eq!("Hello", &comp_b.0);
            counter += 1;
        }
        assert_eq!(2, counter);
    }

    #[test]
    fn proc_query_uncomponent() {
        #[derive(Debug)]
        struct CompA(usize);
        #[derive(Debug)]
        struct CompB(String);
        struct CompC {}

        let mut world = World::new();
        let a = world.create();
        world.add_component(a, CompA(42));
        world.add_component(a, CompB("Hello".to_string()));
        let b = world.create();
        world.add_component(b, CompA(42));
        world.add_component(b, CompB("Hello".to_string()));
        world.add_component(b, CompC {});
        let c = world.create();
        world.add_component(c, CompA(42));
        world.add_component(c, CompB("Hello".to_string()));

        let mut counter = 0;
        for (comp_a, comp_b) in query!(world, CompA, CompB, !CompC) {
            println!("{comp_a:?}");
            println!("{comp_b:?}");
            assert_eq!(42, comp_a.0);
            assert_eq!("Hello", &comp_b.0);
            counter += 1;
        }
        assert_eq!(2, counter);
    }

    // TODO remove with hygiene
    use crate::entity_view_deferred::EntityViewDeferred;

    #[test]
    fn proc_query_outvar() {
        #[derive(Debug)]
        struct CompA(usize);
        #[derive(Debug)]
        struct CompB(String);
        struct CompC {}

        let mut world = World::new();
        let a = world.create();
        world.add_component(a, CompA(42));
        world.add_component(a, CompB("Hello".to_string()));
        let b = world.create();
        world.add_component(b, CompA(21));
        let c = world.create();
        world.add_component(c, CompA(42));
        world.add_component(c, CompB("Hello".to_string()));
        world.add_component(c, CompC {});

        let mut counter = 0;
        let mut c_counter = 0;
        for (this, comp_a, comp_b) in query!(world, &this, CompA, CompB) {
            println!("{comp_a:?}");
            println!("{comp_b:?}");
            assert_eq!(42, comp_a.0);
            assert_eq!("Hello", &comp_b.0);
            assert!(this.has::<CompA>());
            if this.has::<CompC>() {
                println!("I have CompC");
                c_counter += 1;
            }
            counter += 1;
        }
        assert_eq!(2, counter);
        assert_eq!(1, c_counter);
    }
}
