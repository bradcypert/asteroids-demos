use std::time::Duration;

use asteroids::domain::{Game, GameConfig, Input, Radians, Ship, Turn};
use glam::Vec2;
use raylib::prelude::*;

/// Half-angle between the nose and each wing of the ship triangle.
const WING_SPREAD: Radians = Radians::new(2.5);

fn to_rl(v: Vec2) -> Vector2 {
    Vector2::new(v.x, v.y)
}

fn main() {
    let config = GameConfig::default();

    let (mut rl, thread) = raylib::init()
        .size(
            config.screen().width() as i32,
            config.screen().height() as i32,
        )
        .title("Asteroids - Rust")
        .build();

    rl.set_target_fps(60);

    let mut rng = rand::rng();
    let mut game = Game::new(config, &mut rng);

    while !rl.window_should_close() {
        let dt = Duration::from_secs_f32(rl.get_frame_time());
        let time = rl.get_time();

        let input = Input {
            turn: if rl.is_key_down(KeyboardKey::KEY_A) || rl.is_key_down(KeyboardKey::KEY_LEFT) {
                Some(Turn::Left)
            } else if rl.is_key_down(KeyboardKey::KEY_D) || rl.is_key_down(KeyboardKey::KEY_RIGHT) {
                Some(Turn::Right)
            } else {
                None
            },
            thrust: rl.is_key_down(KeyboardKey::KEY_W) || rl.is_key_down(KeyboardKey::KEY_UP),
            fire: rl.is_key_down(KeyboardKey::KEY_SPACE),
        };

        game.update(&input, dt, &mut rng);

        if rl.is_key_pressed(KeyboardKey::KEY_ENTER) && game.is_game_over() {
            game.restart(&mut rng);
        }

        let mut d = rl.begin_drawing(&thread);
        draw(&mut d, &game, time);
    }
}

fn draw(d: &mut RaylibDrawHandle, game: &Game, time: f64) {
    d.clear_background(Color::BLACK);

    if let Some(playing) = game.playing() {
        let config = game.config();

        let player = playing.player();
        draw_ship(
            d,
            player.ship(),
            player.is_invulnerable(),
            config.ship_size(),
            time,
        );

        for asteroid in playing.asteroids() {
            d.draw_circle_lines(
                asteroid.position().x as i32,
                asteroid.position().y as i32,
                asteroid.radius(),
                Color::GRAY,
            );
        }

        for bullet in playing.bullets() {
            d.draw_circle_v(
                to_rl(bullet.position()),
                config.bullet_radius(),
                Color::YELLOW,
            );
        }

        let score = playing.score().value();
        let lives = playing.player().lives().value();
        let wave = playing.wave().value();

        d.draw_text(&format!("SCORE {score}"), 10, 10, 20, Color::WHITE);
        d.draw_text(&format!("LIVES {lives}"), 10, 34, 20, Color::WHITE);
        d.draw_text(&format!("WAVE {wave}"), 10, 58, 20, Color::WHITE);
        return;
    }

    let Some(final_score) = game.final_score() else {
        return;
    };
    let screen = game.config().screen();

    let msg = "GAME OVER";
    let w = d.measure_text(msg, 40);
    d.draw_text(
        msg,
        screen.width() as i32 / 2 - w / 2,
        screen.height() as i32 / 2 - 60,
        40,
        Color::RED,
    );

    let score = format!("FINAL SCORE {}", final_score.value());
    let w2 = d.measure_text(&score, 20);
    d.draw_text(
        &score,
        screen.width() as i32 / 2 - w2 / 2,
        screen.height() as i32 / 2,
        20,
        Color::WHITE,
    );

    let sub = "Press ENTER to restart";
    let w3 = d.measure_text(sub, 20);
    d.draw_text(
        sub,
        screen.width() as i32 / 2 - w3 / 2,
        screen.height() as i32 / 2 + 40,
        20,
        Color::WHITE,
    );
}

fn draw_ship(
    d: &mut RaylibDrawHandle,
    ship: Option<&Ship>,
    invulnerable: bool,
    size: f32,
    time: f64,
) {
    let Some(ship) = ship else {
        return;
    };

    if invulnerable && ((time * 10.0) as i64) % 2 == 0 {
        return;
    }

    let heading = ship.heading();
    let position = ship.position();

    let nose = position + Vec2::new(heading.sin(), -heading.cos()) * size;
    let left = position
        + Vec2::new(
            (heading + WING_SPREAD).sin(),
            -(heading + WING_SPREAD).cos(),
        ) * size;
    let right = position
        + Vec2::new(
            (heading - WING_SPREAD).sin(),
            -(heading - WING_SPREAD).cos(),
        ) * size;

    d.draw_triangle_lines(to_rl(nose), to_rl(left), to_rl(right), Color::WHITE);
}
