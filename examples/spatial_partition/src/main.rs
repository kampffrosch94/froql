use froql::{query, world::World};
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Spatial Partion Example".to_string(),
        high_dpi: true,
        ..Default::default()
    }
}

/// Relationship that signifies that a Rectangle contains a Point
enum Inside {}

#[macroquad::main(window_conf)]
async fn main() {
    let mut world = World::new();
    world.register_relation::<Inside>();
    rand::srand(12345);

    for _ in 0..100 {
        let circle = world.create_mut();
        circle.add(Circle::new(
            rand::gen_range(0., 1.),
            rand::gen_range(0., 1.),
            0.01,
        ));
    }
    for x in [0., 0.55] {
        for y in [0., 0.55] {
            let rect = world.create_mut();
            rect.add(Rect {
                x,
                y,
                w: 0.45,
                h: 0.45,
            });
        }
    }

    for (e_rect, rect) in query!(world, &this, Rect) {
        for (e_circle, circle) in query!(world, &this, Circle) {
            if circle.overlaps_rect(&rect) {
                // adding a relation here is deferred
                // we can't mutate what we are iterating over
                // so that we don't accidentally invalide our iterator
                e_circle.relate_to::<Inside>(*e_rect);
            }
        }
    }
    // executes deferred operations
    world.process();

    loop {
        clear_background(BLACK);
        let circle_scale = (screen_height() + screen_width()) / 2.0;

        for (r,) in query!(world, Rect) {
            draw_rectangle_lines(
                r.x * screen_width(),
                r.y * screen_height(),
                r.w * screen_width(),
                r.h * screen_height(),
                5.,
                GREEN,
            );
        }

        for (c,) in query!(world, Circle, Inside(this, _)) {
            let r = c.r * circle_scale;
            draw_circle(c.x * screen_width(), c.y * screen_height(), r, YELLOW);
        }

        for (c,) in query!(world, Circle, !Inside(this, _)) {
            let r = c.r * circle_scale;
            draw_circle(c.x * screen_width(), c.y * screen_height(), r, BLUE);
        }

        let mouse = vec2(
            mouse_position().0 / screen_width(),
            mouse_position().1 / screen_height(),
        );

        /*
        for (e_rect, rect) in query!(world, &this, Rect) {
            if !rect.contains(mouse) {
                continue; // so efficient, wow 🐕
            }
            let e = *e_rect; // TODO fix this via into
            let should_highlight =
                query!(world, Circle, Inside(this, *e)).any(|(c,)| c.contains(&mouse));
            if should_highlight {
                for (c,) in query!(world, Circle, Inside(this, *e)) {
                    let r = c.r * circle_scale;
                    draw_circle(c.x * screen_width(), c.y * screen_height(), r, RED);
                }
            }
        }
        */

        for (e_circle, _) in query!(world, &this, Circle).filter(|(_, c)| c.contains(&mouse)) {
            // for (c,) in query!(world, Circle, Inside(this, rect), !Inside(*e_circle, rect)) {
            for (c,) in query!(world, Circle, Inside(this, rect), !Inside(*e_circle, rect)) {
                let r = c.r * circle_scale;
                draw_circle(c.x * screen_width(), c.y * screen_height(), r, RED);
            }
        }

        draw_circle(
            mouse.x * screen_width(),
            mouse.y * screen_height(),
            0.005 * circle_scale,
            WHITE,
        );

        let text = &format!("screen: ({},{})", screen_width(), screen_height());
        draw_text(text, 20.0, 20.0, 30.0, WHITE);
        let text = &format!("fps: {}", get_fps());
        draw_text(text, 20.0, 50.0, 30.0, WHITE);
        let text = &format!("dpi: {}", screen_dpi_scale());
        draw_text(text, 20.0, 70.0, 30.0, WHITE);

        next_frame().await
    }
}
