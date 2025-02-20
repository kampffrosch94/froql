use std::{
    any::{type_name, TypeId},
    cell::RefCell,
    collections::HashMap,
};

use froql::{
    component::CASCADING_DESTRUCT, entity_store::EntityId, query,
    query_helper::trivial_query_one_component, relation::Relation, world::World,
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

impl MyRect {
    fn center(&self) -> Vec2 {
        vec2(self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
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
    (Components($($components:tt $([persist])?),*),
     Relations($($relations:tt $(($flags:expr))? $([persist])? ),*)) => {
        fn register_components(world: &mut World) {
            $(world.register_component::<$components>();)*
            $(generate_register!(@rel world $relations $($flags)?);)*
        }
    };
}

macro_rules! generate_save {
    (@rel $world:ident $state:ident $ty:tt persist) => {
        $state.relations.insert(
            type_name::<$ty>().to_string(),
            $world
                .bookkeeping
                .relation_pairs(TypeId::of::<Relation<$ty>>())
                .into_iter()
                .map(|(o, t)| (o.id.0, t.id.0))
                .collect(),
        );
    };
    (@rel $world:ident $state:ident $ty:tt ) => {};
    (@comp $world:ident $state:ident $ty:tt persist) => {
        let mut buffer = Vec::new();
        for id in trivial_query_one_component($world, TypeId::of::<RefCell<$ty>>()) {
            let r = $world.get_component_by_entityid::<$ty>(id);
            let s = r.serialize_json();
            buffer.push((id.0, s));
        }
        $state
            .components
            .insert(type_name::<$ty>().to_string(), buffer);
    };
    (@comp $world:ident $state:ident $ty:tt) => {};
    (Components($($components:tt $([$persist_comp:tt])?),*),
     Relations($($relations:tt $(($flags:expr))? $([$persist_rel:tt])?),*)) => {
        fn save_world(world: &World) -> String {
            let mut state = SerializedState::default();
            $(generate_save!(@comp world state $components $($persist_comp)?);)*
            $(generate_save!(@rel world state $relations $($persist_rel)?);)*
            state.serialize_json()
        }
    };
}

generate_register!(Components( MyRect [persist]), Relations(Link(CASCADING_DESTRUCT) [persist]));

generate_save!(
    Components(MyRect[persist]),
    Relations(Link(CASCADING_DESTRUCT|TRANSITIVE)[persist])
);

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
            var => panic!("Unknown component type: {var}"),
        }
    }

    for (ty, pairs) in &state.relations {
        match ty.as_str() {
            var if var == type_name::<Link>() => {
                for (origin, target) in pairs {
                    let a = world.ensure_alive(EntityId(*origin));
                    let b = world.ensure_alive(EntityId(*target));
                    world.add_relation::<Link>(a, b);
                }
            }
            var => panic!("Unknown relationship type: {var}"),
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

    let mut prev = None;

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
        if is_mouse_button_pressed(MouseButton::Left) {
            let e = world.create_mut().add(MyRect {
                x: mouse.0,
                y: mouse.1,
                w: 200.,
                h: 50.,
            });
            let id = e.id;
            if prev.is_some() {
                let prev = e.world.ensure_alive(prev.unwrap());
                e.relate_from::<Link>(prev);
            }
            prev = Some(id.id);
        }

        for (r,) in query!(world, MyRect) {
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 5., GREEN);
        }

        for (a, b) in query!(world, MyRect(a), MyRect(b), Link(a, b)) {
            let a = a.center();
            let b = b.center();
            draw_line(a.x, a.y, b.x, b.y, 2.0, YELLOW);
        }

        next_frame().await
    }
}
