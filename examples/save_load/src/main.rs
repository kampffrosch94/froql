use std::{any::TypeId, cell::RefCell, collections::HashMap};

use froql::{
    component::CASCADING_DESTRUCT, query, query_helper::trivial_query_one_component, world::World,
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

#[derive(Debug, DeJson, SerJson)]
struct SerializedState {
    // TypeName, Vec<(EntityId, ComponentPayload)>
    components: HashMap<String, Vec<(u32, String)>>,
    // TypeName, Vec<(Origin, Target)>
    relations: HashMap<String, Vec<(u32, u32)>>,
}

macro_rules! generate_register {
    (@rel $world:ident $ty:tt ($flags:expr)) => {
        $world.register_relation_flags::<$ty>($flags);
    };
    (@rel $world:ident $ty:tt) => {
        $world.register_relation::<$ty>();
    };
    (Components[$($components:ty),*], Relations[$($relations:tt ($flags:expr)),*]) => {
        fn register_components(world: &mut World) {
            $(world.register_component::<$components>();)*
            $(generate_register!(@rel world $relations))*;
        }
    };
}

generate_register!(Components[MyRect], Relations[Link(CASCADING_DESTRUCT)]);

// fn register_components(world: &mut World) {
//     world.register_relation_flags::<Link>(CASCADING_DESTRUCT);
//     world.register_component::<MyRect>();
// }

fn save_world(world: &World) -> String {
    let mut result: Vec<String> = Vec::new();

    for id in trivial_query_one_component(world, TypeId::of::<RefCell<MyRect>>()) {
        let r = world.get_component_by_entityid::<MyRect>(id);
        let s = r.serialize_json();
        result.push(s);
    }
    result.serialize_json()
}

fn load_world(s: &str) -> World {
    let mut world = World::new();
    register_components(&mut world);

    let buffer: Vec<String> = Vec::deserialize_json(s).unwrap();

    for rect_s in buffer {
        let rect = MyRect::deserialize_json(&rect_s).unwrap();
        world.create_mut().add(rect);
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
