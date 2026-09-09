const std = @import("std");
const rl = @import("raylib");

const SCREEN_WIDTH = 800;
const SCREEN_HEIGHT = 600;
const SHIP_SIZE = 20;
const SHIP_ROTATION_SPEED = 3.5;
const SHIP_THRUST = 220;
const SHIP_DRAG = 0.60;
const MAX_SHIP_SPEED = 380;
const SHIP_COLLISION_RADIUS = 12;
const BULLET_SPEED = 520;
const BULLET_LIFETIME = 1.1;
const FIRE_COOLDOWN = 0.25;
const MAX_BULLETS = 32;
const BULLET_RADIUS = 2;
const MAX_ASTEROIDS = 64;
const ASTEROID_SPEED_MIN = 40;
const ASTEROID_SPEED_MAX = 110;
const ASTEROID_SPLIT_COUNT = 2;
const STARTING_LIVES = 3;
const STARTING_ASTEROIDS = 4;
const INVULNERABILITY_TIME = 2;
const RESPAWN_DELAY = 1.5;

const AsteroidSize = enum(u8) {
    large = 0,
    medium = 1,
    small = 2,
};

const ASTEROID_RADIUS = [3]f32{ 40, 22, 12 };
const ASTEROID_SCORE = [3]i32{ 20, 50, 100 };

const Vec2 = struct {
    x: f32,
    y: f32,

    pub fn add(a: Vec2, b: Vec2) Vec2 {
        return .{ .x = a.x + b.x, .y = a.y + b.y };
    }

    pub fn scale(a: Vec2, s: f32) Vec2 {
        return .{ .x = a.x * s, .y = a.y * s };
    }

    pub fn length(a: Vec2) f32 {
        return @sqrt(a.x * a.x + a.y * a.y);
    }
};

const Ship = struct {
    position: Vec2 = .{ .x = SCREEN_WIDTH / 2, .y = SCREEN_HEIGHT / 2 },
    velocity: Vec2 = .{ .x = 0, .y = 0 },
    rotation: f32 = 0,
    alive: bool = true,
    invulnerable_timer: f32 = INVULNERABILITY_TIME,
};

const Bullet = struct {
    position: Vec2,
    velocity: Vec2,
    lifetime: f32,
    active: bool,
};

const Asteroid = struct {
    position: Vec2,
    velocity: Vec2,
    size: AsteroidSize,
    rotation: f32,
    rotation_speed: f32,
    active: bool,
};

const GameState = enum {
    playing,
    game_over,
};

const Game = struct {
    ship: Ship = .{},
    bullets: [MAX_BULLETS]Bullet = @splat(std.mem.zeroes(Bullet)),
    asteroids: [MAX_ASTEROIDS]Asteroid = @splat(std.mem.zeroes(Asteroid)),
    score: i32 = 0,
    lives: i32 = STARTING_LIVES,
    wave: i32 = 1,
    fire_cooldown: f32 = 0,
    respawn_timer: f32 = 0,
    state: GameState = .playing,
    rng: std.Random,

    pub fn init(g: *Game) void {
        const rng = g.rng;
        g.* = .{ .rng = rng };
        g.spawn_wave();
    }

    fn spawn_wave(g: *Game) void {
        const count = STARTING_ASTEROIDS + (g.wave - 1);
        var i: i32 = 0;
        while (i < count) : (i += 1) {
            var pos: Vec2 = undefined;
            if (g.rand_bool()) {
                pos.x = if (g.rand_bool()) 0 else SCREEN_WIDTH;
                pos.y = g.rand_float(0, SCREEN_HEIGHT);
            } else {
                pos.x = g.rand_float(0, SCREEN_WIDTH);
                pos.y = if (g.rand_bool()) 0 else SCREEN_HEIGHT;
            }
            spawn_asteroid(g, pos, .large);
        }
    }

    pub fn draw(g: *const Game) void {
        rl.beginDrawing();
        defer rl.endDrawing();

        rl.clearBackground(rl.Color.black);

        draw_ship(g);

        for (&g.asteroids) |*a| {
            if (!a.active) continue;
            rl.drawCircleLines(@intFromFloat(a.position.x), @intFromFloat(a.position.y), ASTEROID_RADIUS[@intFromEnum(a.size)], rl.Color.gray);
        }

        for (&g.bullets) |*b| {
            if (!b.active) continue;
            rl.drawCircleV(.{ .x = b.position.x, .y = b.position.y }, BULLET_RADIUS, rl.Color.yellow);
        }

        rl.drawText(rl.textFormat("SCORE %d", .{g.score}), 10, 10, 20, rl.Color.white);
        rl.drawText(rl.textFormat("LIVES %d", .{g.lives}), 10, 34, 20, rl.Color.white);
        rl.drawText(rl.textFormat("WAVE %d", .{g.wave}), 10, 58, 20, rl.Color.white);

        if (g.state == .game_over) {
            const msg = "GAME OVER";
            const w = rl.measureText(msg, 40);
            rl.drawText(msg, @divTrunc(SCREEN_WIDTH, 2) - @divTrunc(w, 2), @divTrunc(SCREEN_HEIGHT, 2) - 40, 40, rl.Color.red);
            const sub = "Press ENTER to restart";
            const w2 = rl.measureText(sub, 20);
            rl.drawText(sub, @divTrunc(SCREEN_WIDTH, 2) - @divTrunc(w2, 2), @divTrunc(SCREEN_HEIGHT, 2) + 10, 20, rl.Color.white);
        }
    }

    fn update(g: *Game, dt: f32) void {
        if (g.state == .game_over) {
            if (rl.isKeyPressed(.enter)) g.init();
            return;
        }

        if (g.respawn_timer > 0) {
            g.respawn_timer -= dt;
            if (g.respawn_timer <= 0) {
                g.ship = .{};
            }
        } else {
            if (rl.isKeyDown(.a) or rl.isKeyDown(.left)) g.ship.rotation -= SHIP_ROTATION_SPEED * dt;
            if (rl.isKeyDown(.d) or rl.isKeyDown(.right)) g.ship.rotation += SHIP_ROTATION_SPEED * dt;

            if (rl.isKeyDown(.w) or rl.isKeyDown(.up)) {
                const facing = Vec2{ .x = @sin(g.ship.rotation), .y = -@cos(g.ship.rotation) };
                g.ship.velocity = g.ship.velocity.add(facing.scale(SHIP_THRUST * dt));
            }

            if (rl.isKeyDown(.space)) fire_bullet(g);

            const speed = g.ship.velocity.length();
            if (speed > MAX_SHIP_SPEED) {
                g.ship.velocity = g.ship.velocity.scale(MAX_SHIP_SPEED / speed);
            }

            const drag_factor = std.math.pow(f32, SHIP_DRAG, dt);
            g.ship.velocity = g.ship.velocity.scale(drag_factor);

            g.ship.position = g.ship.position.add(g.ship.velocity.scale(dt));
            g.ship.position = wrap_position(g.ship.position);
        }

        if (g.fire_cooldown > 0) g.fire_cooldown -= dt;

        for (&g.bullets) |*b| {
            if (!b.active) continue;
            b.position = b.position.add(b.velocity.scale(dt));
            b.lifetime -= dt;
            if (b.lifetime <= 0 or
                b.position.x < 0 or b.position.x > SCREEN_WIDTH or
                b.position.y < 0 or b.position.y > SCREEN_HEIGHT)
            {
                b.active = false;
            }
        }

        for (&g.asteroids) |*a| {
            if (!a.active) continue;
            a.position = a.position.add(a.velocity.scale(dt));
            a.position = wrap_position(a.position);
            a.rotation += a.rotation_speed * dt;
        }

        for (&g.bullets) |*b| {
            if (!b.active) continue;
            for (&g.asteroids) |*a| {
                if (!a.active) continue;
                if (circle_collide(b.position, BULLET_RADIUS, a.position, ASTEROID_RADIUS[@intFromEnum(a.size)])) {
                    b.active = false;
                    split_asteroid(g, a);
                    break;
                }
            }
        }

        if (g.ship.alive and g.ship.invulnerable_timer <= 0) {
            for (&g.asteroids) |*a| {
                if (!a.active) continue;
                if (circle_collide(g.ship.position, SHIP_COLLISION_RADIUS, a.position, ASTEROID_RADIUS[@intFromEnum(a.size)])) {
                    g.lives -= 1;
                    g.ship.alive = false;
                    if (g.lives <= 0) {
                        g.state = .game_over;
                    } else {
                        g.respawn_timer = RESPAWN_DELAY;
                    }
                    break;
                }
            }
        }

        if (g.ship.invulnerable_timer > 0) g.ship.invulnerable_timer -= dt;

        var any_asteroids = false;
        for (&g.asteroids) |*a| {
            if (a.active) {
                any_asteroids = true;
                break;
            }
        }
        if (!any_asteroids and g.state == .playing) {
            g.wave += 1;
            g.spawn_wave();
        }
    }

    fn rand_float(g: *Game, min: f32, max: f32) f32 {
        return min + (max - min) * g.rng.float(f32);
    }

    fn rand_bool(g: *Game) bool {
        return g.rng.boolean();
    }
};

fn wrap_position(pos: Vec2) Vec2 {
    var p = pos;
    if (p.x < 0) p.x += SCREEN_WIDTH;
    if (p.x > SCREEN_WIDTH) p.x -= SCREEN_WIDTH;
    if (p.y < 0) p.y += SCREEN_HEIGHT;
    if (p.y > SCREEN_HEIGHT) p.y -= SCREEN_HEIGHT;
    return p;
}

fn circle_collide(a: Vec2, ra: f32, b: Vec2, rb: f32) bool {
    const dx = a.x - b.x;
    const dy = a.y - b.y;
    const dist = @sqrt(dx * dx + dy * dy);
    return dist < (ra + rb);
}

fn spawn_asteroid(g: *Game, position: Vec2, size: AsteroidSize) void {
    for (&g.asteroids) |*a| {
        if (!a.active) {
            const angle = g.rand_float(0, 2 * std.math.pi);
            const speed = g.rand_float(ASTEROID_SPEED_MIN, ASTEROID_SPEED_MAX);
            a.* = .{
                .position = position,
                .velocity = .{ .x = @cos(angle) * speed, .y = @sin(angle) * speed },
                .size = size,
                .rotation = g.rand_float(0, 2 * std.math.pi),
                .rotation_speed = g.rand_float(-2, 2),
                .active = true,
            };
            return;
        }
    }
}

fn fire_bullet(g: *Game) void {
    if (g.fire_cooldown > 0) return;
    for (&g.bullets) |*b| {
        if (!b.active) {
            const facing = Vec2{ .x = @sin(g.ship.rotation), .y = -@cos(g.ship.rotation) };
            const nose = g.ship.position.add(facing.scale(SHIP_SIZE));
            b.* = .{
                .position = nose,
                .velocity = facing.scale(BULLET_SPEED),
                .lifetime = BULLET_LIFETIME,
                .active = true,
            };
            g.fire_cooldown = FIRE_COOLDOWN;
            return;
        }
    }
}

fn split_asteroid(g: *Game, a: *Asteroid) void {
    g.score += ASTEROID_SCORE[@intFromEnum(a.size)];
    if (a.size != .small) {
        const next: AsteroidSize = @enumFromInt(@intFromEnum(a.size) + 1);
        var i: usize = 0;
        while (i < ASTEROID_SPLIT_COUNT) : (i += 1) {
            spawn_asteroid(g, a.position, next);
        }
    }
    a.active = false;
}

fn draw_ship(g: *const Game) void {
    if (!g.ship.alive) return;
    if (g.ship.invulnerable_timer > 0 and @mod(@as(i64, @intFromFloat(rl.getTime() * 10)), 2) == 0) return;

    const nose = Vec2{
        .x = g.ship.position.x + @sin(g.ship.rotation) * SHIP_SIZE,
        .y = g.ship.position.y - @cos(g.ship.rotation) * SHIP_SIZE,
    };
    const left = Vec2{
        .x = g.ship.position.x + @sin(g.ship.rotation + 2.5) * SHIP_SIZE,
        .y = g.ship.position.y - @cos(g.ship.rotation + 2.5) * SHIP_SIZE,
    };
    const right = Vec2{
        .x = g.ship.position.x + @sin(g.ship.rotation - 2.5) * SHIP_SIZE,
        .y = g.ship.position.y - @cos(g.ship.rotation - 2.5) * SHIP_SIZE,
    };

    rl.drawTriangleLines(
        .{ .x = nose.x, .y = nose.y },
        .{ .x = left.x, .y = left.y },
        .{ .x = right.x, .y = right.y },
        .white,
    );
}

pub fn main(init: std.process.Init) void {
    rl.initWindow(SCREEN_WIDTH, SCREEN_HEIGHT, "Asteroids - Zig");
    defer rl.closeWindow();
    rl.setTargetFPS(60);

    var rng = std.Random.DefaultPrng.init(@bitCast(std.Io.Clock.real.now(init.io).toMilliseconds()));
    var game: Game = .{ .rng = rng.random() };
    game.init();

    while (!rl.windowShouldClose()) {
        const dt = rl.getFrameTime();
        game.update(dt);
        game.draw();
    }
}
