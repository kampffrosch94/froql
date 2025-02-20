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
    fn center(&self) -> Center {
        Center {
            x: self.x + self.w / 2.0,
            y: self.y + self.h / 2.0,
        }
    }
}

#[derive(Debug, DeJson, SerJson)]
struct Center {
    x: f32,
    y: f32,
}

#[derive(Debug, DeJson, SerJson)]
struct MyCircle {
    x: f32,
    y: f32,
    r: f32,
}

impl MyCircle {
    fn center(&self) -> Center {
        Center {
            x: self.x,
            y: self.y,
        }
    }
}

#[derive(Debug, DeJson, SerJson)]
struct Previous {}

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

macro_rules! generate_load {
    (@rel ($world:expr) $var:ident $pairs:ident $ty:tt persist) => {
        if $var == type_name::<$ty>() {
            for (origin, target) in $pairs {
                let a = $world.ensure_alive(EntityId(*origin));
                let b = $world.ensure_alive(EntityId(*target));
                $world.add_relation::<$ty>(a, b);
            }
            continue;
        }
    };
    (@rel ($world:expr) $var:ident $payloads:ident $ty:tt) => {};
    (@comp ($world:expr) $var:ident $payloads:ident $ty:tt persist) => {
        if $var == type_name::<$ty>() {
            for (entity_id, payload) in $payloads {
                let val = $ty::deserialize_json(payload).unwrap();
                let e = $world.ensure_alive(EntityId(*entity_id));
                $world.add_component(e, val);
            }
            continue;
        }
    };
    (@comp ($world:expr) $var:ident $payloads:ident $ty:tt) => {};
    (Components($($components:tt $([$persist_comp:tt])?),*),
     Relations($($relations:tt $(($flags:expr))? $([$persist_rel:tt])?),*)) => {
        fn load_world(s: &str) -> World {
            let mut world = World::new();
            register_components(&mut world);
            let state: SerializedState = SerializedState::deserialize_json(s).unwrap();

            //$(generate_load!(@comp world state $components $($persist_comp)?);)*
            //$(generate_load!(@rel world state $relations $($persist_rel)?);)*

            for (ty, payloads) in &state.components {
                let var = ty.as_str();
                $(generate_load!(@comp (&mut world) var payloads $components $($persist_comp)?);)*
                panic!("Unknown component type: {var}");
            }

            for (ty, pairs) in &state.relations {
                let var = ty.as_str();
                $(generate_load!(@rel (&mut world) var pairs $relations $($persist_rel)?);)*
                panic!("Unknown relationship type: {var}");
            }

            world
        }
    };
}

macro_rules! ecs_types {
    ($($tokens:tt)+) => {
        generate_register!($($tokens)+);
        generate_save!($($tokens)+);
        generate_load!($($tokens)+);
    }
}

ecs_types!(
    Components(
        MyRect[persist],
        MyCircle[persist],
        Center[persist],
        Previous[persist]
    ),
    Relations(Link(CASCADING_DESTRUCT)[persist])
);

enum CurrentShape {
    Rect,
    Circle,
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut world = World::new();
    register_components(&mut world);
    rand::srand(12345);

    // executes deferred operations
    world.process();

    let mut saved_state = None;

    let mut current_shape = CurrentShape::Rect;

    loop {
        clear_background(BLACK);

        let s = "Left Click to add a shape.";
        draw_text(s, 20.0, 20.0, 30.0, WHITE);
        let s = "Right Click to remove a shape.";
        draw_text(s, 20.0, 45.0, 30.0, WHITE);
        let s = "Q/W to switch shape.";
        draw_text(s, 20.0, 70.0, 30.0, WHITE);
        let s = "F5 saves. F9 loads.";
        draw_text(s, 20.0, 95.0, 30.0, WHITE);

        if is_key_released(KeyCode::F5) {
            println!("Save.");
            saved_state = Some(save_world(&world));
        }
        if is_key_released(KeyCode::F9) && saved_state.is_some() {
            println!("Load.");
            world = load_world(saved_state.as_ref().unwrap())
        }

        if is_key_released(KeyCode::Q) {
            current_shape = CurrentShape::Rect;
        }

        if is_key_released(KeyCode::W) {
            current_shape = CurrentShape::Circle;
        }

        let mouse = mouse_position();
        if is_mouse_button_pressed(MouseButton::Left) {
            let e = {
                match current_shape {
                    CurrentShape::Rect => {
                        let r = MyRect {
                            x: mouse.0,
                            y: mouse.1,
                            w: 200.,
                            h: 50.,
                        };
                        let center = r.center();
                        world.create_mut().add(r).add(center).id
                    }
                    CurrentShape::Circle => {
                        let c = MyCircle {
                            x: mouse.0,
                            y: mouse.1,
                            r: 50.,
                        };
                        let center = c.center();
                        world.create_mut().add(c).add(center).id
                    }
                }
            };
            for (prev,) in query!(world, &this, _ Previous) {
                prev.relate_to::<Link>(e);
                prev.remove::<Previous>();
            }
            world.process();
            world.add_component(e, Previous {});
        }

        if is_mouse_button_pressed(MouseButton::Right) {
            let mut destroyed = Vec::new();
            for (e, r) in query!(world, &this, MyRect) {
                if Rect::new(r.x, r.y, r.w, r.h).contains(vec2(mouse.0, mouse.1)) {
                    e.destroy();
                    destroyed.push(e.id);
                }
            }
            for (e, c) in query!(world, &this, MyCircle) {
                if Circle::new(c.x, c.y, c.r).contains(&vec2(mouse.0, mouse.1)) {
                    e.destroy();
                    destroyed.push(e.id);
                }
            }

            // fix up previous
            for e in destroyed {
                for (prev,) in query!(world, &prev, Link(prev, *e)) {
                    prev.add(Previous {});
                }
            }
            world.process();
        }

        
        for (r,) in query!(world, MyRect) {
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 5., GREEN);
        }

        for (c,) in query!(world, MyCircle) {
            draw_circle_lines(c.x, c.y, c.r, 5., BLUE);
        }

        for (a, b) in query!(world, Center(a), Center(b), Link(a, b)) {
            draw_line(a.x, a.y, b.x, b.y, 2.0, YELLOW);
        }

        next_frame().await
    }
}
