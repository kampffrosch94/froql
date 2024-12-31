#![allow(dead_code)]
use fast_queries::world::World;

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
    let a = world.create();
    let b = world.create();
}
