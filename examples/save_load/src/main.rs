use std::{
    any::{type_name, TypeId},
    cell::RefCell,
    collections::HashMap,
};

use froql::{
    component::CASCADING_DESTRUCT, entity_store::EntityId, query,
    query_helper::trivial_query_one_component, world::World,
};
use macroquad::prelude::*;
use nanoserde::{DeJson, SerJson};

fn window_conf() -> Conf {
    Conf {
        window_title: "Save Load Example".to_string(),
        high_dpi: true,
        ..Default::default()
    }
}

/// Relationship: a connection between two shapes
enum Link {}

// often you'll need to define your own type if you want to derive serialization
// for example the macroquad Rect type does not have any serialization traits
#[derive(Debug, DeJson, SerJson)]
struct MyRect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

//trace_macros!(true);

#[derive(Default, Debug, DeJson, SerJson)]
struct SerializedState {
    // TypeName, Vec<(EntityId, ComponentPayload)>
    components: HashMap<String, Vec<(u32, String)>>,
    // TypeName, Vec<(Origin, Target)>
    relations: HashMap<String, Vec<(u32, u32)>>,
}

macro_rules! generate_register {
    (@rel $world:ident $ty:tt $flags:tt) => {
        $world.register_relation_flags::<$ty>($flags);
    };
    (@rel $world:ident $ty:tt) => {
        $world.register_relation::<$ty>();
    };
    (Components($($components:ty),*), Relations($($relations:tt $(($flags:tt))?),*)) => {
        fn register_components(world: &mut World) {
            $(world.register_component::<$components>();)*
            $(generate_register!(@rel world $relations $($flags)?);)*
        }
    };
}

generate_register!(Components(MyRect), Relations(Link(CASCADING_DESTRUCT)));

// fn register_components(world: &mut World) {
//     world.register_relation_flags::<Link>(CASCADING_DESTRUCT);
//     world.register_component::<MyRect>();
// }

fn save_world(world: &World) -> String {
    let mut state = SerializedState::default();

    let mut buffer = Vec::new();
    for id in trivial_query_one_component(world, TypeId::of::<RefCell<MyRect>>()) {
        let r = world.get_component_by_entityid::<MyRect>(id);
        let s = r.serialize_json();
        buffer.push((id.0, s));
    }
    state
        .components
        .insert(type_name::<MyRect>().to_string(), buffer);

    state.serialize_json()
}

fn load_world(s: &str) -> World {
    let mut world = World::new();
    register_components(&mut world);

    let state: SerializedState = SerializedState::deserialize_json(s).unwrap();

    for (ty, payloads) in &state.components {
        match ty.as_str() {
            var if var == type_name::<MyRect>() => {
                for (entity_id, payload) in payloads {
                    let val = MyRect::deserialize_json(payload).unwrap();
                    let e = world.ensure_alive(EntityId(*entity_id));
                    world.add_component(e, val);
                }
            }
            _ => panic!(),
        }
    }

    world
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut world = World::new();
    register_components(&mut world);
    rand::srand(12345);

    // executes deferred operations
    world.process();

    let mut saved_state = None;

    loop {
        clear_background(BLACK);

        let s = "Left Click to add a rectangle.";
        draw_text(s, 20.0, 20.0, 30.0, WHITE);
        let s = "Right Click to remove a rectangle.";
        draw_text(s, 20.0, 40.0, 30.0, WHITE);
        let s = "F5 saves. F9 loads.";
        draw_text(s, 20.0, 60.0, 30.0, WHITE);

        if is_key_released(KeyCode::F5) {
            println!("Save.");
            saved_state = Some(save_world(&world));
        }
        if is_key_released(KeyCode::F9) && saved_state.is_some() {
            println!("Load.");
            world = load_world(saved_state.as_ref().unwrap())
        }

        let mouse = mouse_position();
        if is_mouse_button_down(MouseButton::Left) {
            world.create_mut().add(MyRect {
                x: mouse.0,
                y: mouse.1,
                w: 200.,
                h: 50.,
            });
        }

        for (r,) in query!(world, MyRect) {
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 5., GREEN);
        }

        next_frame().await
    }
}
