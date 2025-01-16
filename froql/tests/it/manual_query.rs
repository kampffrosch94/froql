#![allow(dead_code)]
use froql::world::World;

pub struct NPC {
    name: String,
}

pub struct Health {
    current: i32,
    max: i32,
}

pub enum Faction {}
pub enum Relative {}

#[test]
fn manual_query_simple() {
    let mut world = World::new();
    let _a = world.create();
    let _b = world.create();
}
