use criterion::{criterion_group, criterion_main, Criterion};
use froql::query;
use froql::*;

use std::hint::black_box;
use world::World;

struct Pos(i32, i32);
struct Speed(i32, i32);
enum Rel {}
use std::cell::RefCell;

fn create_entities_simple(count: usize) -> World {
    let mut world = World::new();
    for _ in 0..=count {
        let e = world.create();
        world.add_component(e, Pos(30, 20));
        world.add_component(e, Speed(10, 5));
    }
    world
}

fn simple_query(world: &mut World) {
    for (mut pos, speed) in query!(world, mut Pos, Speed) {
        pos.0 += speed.0;
        pos.1 += speed.1;
    }
}

fn create_entities_relation(pairs: usize) -> World {
    let mut world = World::new();
    for _ in 0..=pairs {
        let a = world.create();
        let b = world.create();
        world.add_component(a, Pos(30, 20));
        world.add_component(b, Speed(10, 5));
        world.add_relation::<Rel>(a, b);
    }
    world
}

fn query_rel(world: &mut World) {
    for (mut pos, speed) in query!(world, mut Pos(a), Speed(b), Rel(a,b) ) {
        pos.0 += speed.0;
        pos.1 += speed.1;
    }
}
fn query_rel_empty_body(world: &mut World) {
    for (_pos, _speed) in query!(world, mut Pos(a), Speed(b), Rel(a,b) ) {}
}

fn criterion_benchmark(c: &mut Criterion) {
    {
        let mut world = create_entities_simple(10000);
        c.bench_function("iterate 10000 Pos, Speed", |b| {
            b.iter(|| simple_query(black_box(&mut world)))
        });
    }
    c.bench_function("create 10000 Pos, Speed", |b| {
        b.iter(|| create_entities_simple(black_box(10000)))
    });
    c.bench_function("create 10000 entities: Pos(a), Speed(b), Rel(a,b)", |b| {
        b.iter(|| create_entities_relation(black_box(5000)))
    });
    {
        let mut world = create_entities_relation(10000);
        c.bench_function("iterate 10000 Pos(a), Speed(b), Rel(a,b)", |b| {
            b.iter(|| query_rel(black_box(&mut world)))
        });
    }
    {
        let mut world = create_entities_relation(10000);
        c.bench_function("iterate (empty) 10000 Pos(a), Speed(b), Rel(a,b)", |b| {
            b.iter(|| query_rel_empty_body(black_box(&mut world)))
        });
    }
    {
        let mut world = create_entities_relation(1);
        c.bench_function("iterate 1 Pos(a), Speed(b), Rel(a,b)", |b| {
            b.iter(|| query_rel(black_box(&mut world)))
        });
    }
    {
        let mut world = create_entities_relation(5);
        c.bench_function("iterate 5 Pos(a), Speed(b), Rel(a,b)", |b| {
            b.iter(|| query_rel(black_box(&mut world)))
        });
    }
    {
        let mut world = create_entities_relation(500);
        c.bench_function("iterate 500 Pos(a), Speed(b), Rel(a,b)", |b| {
            b.iter(|| query_rel(black_box(&mut world)))
        });
    }
}

criterion_group!(
    name = benches;
    config = Criterion::default().noise_threshold(0.05);
    targets = criterion_benchmark);
criterion_main!(benches);
