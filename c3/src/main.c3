module asteroids;
import raylib6::rl;
import std::math;
import std::math::random;

const int SCREEN_WIDTH = 800;
const int SCREEN_HEIGHT = 600;
const float SHIP_SIZE = 20.0;
const float SHIP_ROTATION_SPEED = 3.5;
const float SHIP_THRUST = 220.0;
const float SHIP_DRAG = 0.60;
const float MAX_SHIP_SPEED = 380.0;
const float SHIP_COLLISION_RADIUS = 12.0;
const float BULLET_SPEED = 520.0;
const float BULLET_LIFETIME = 1.1;
const float FIRE_COOLDOWN = 0.25;
const int MAX_BULLETS = 32;
const float BULLET_RADIUS = 2.0;
const int MAX_ASTEROIDS = 64;
const float ASTEROID_SPEED_MIN = 40.0;
const float ASTEROID_SPEED_MAX = 110.0;
const int ASTEROID_SPLIT_COUNT = 2;
const int STARTING_LIVES = 3;
const int STARTING_ASTEROIDS = 4;
const float INVULNERABILITY_TIME = 2.0;
const float RESPAWN_DELAY = 1.5;

enum AsteroidSize : int (float radius, int score)
{
    LARGE  { 40,  20 },
    MEDIUM { 22,  50 },
    SMALL  { 12, 100 },
}

enum GameState : int
{
    PLAYING,
    GAME_OVER,
}

alias Vec2 = float[<2>];

struct Ship
{
    Vec2 position;
    Vec2 velocity;
    float rotation;
    bool alive;
    float invulnerable_timer;
}

struct Bullet
{
    Vec2 position;
    Vec2 velocity;
    float lifetime;
    bool active;
}

struct Asteroid
{
    Vec2 position;
    Vec2 velocity;
    AsteroidSize size;
    float rotation;
    float rotation_speed;
    bool active;
}

struct Game
{
    Ship ship;
    Bullet[MAX_BULLETS] bullets;
    Asteroid[MAX_ASTEROIDS] asteroids;
    int score;
    int lives;
    int wave;
    float fire_cooldown;
    float respawn_timer;
    GameState state;
    random::Sfc64Random rng;
}

fn float rand_float(Game* g, float min, float max)
{
    return min + (max - min) * random::next_float(&g.rng);
}

fn Vec2 wrap_position(Vec2 pos)
{
    if (pos.x < 0) pos.x += SCREEN_WIDTH;
    if (pos.x > SCREEN_WIDTH) pos.x -= SCREEN_WIDTH;
    if (pos.y < 0) pos.y += SCREEN_HEIGHT;
    if (pos.y > SCREEN_HEIGHT) pos.y -= SCREEN_HEIGHT;
    return pos;
}

fn bool circle_collide(Vec2 a, float ra, Vec2 b, float rb)
{
    return a.distance(b) < (ra + rb);
}

fn void spawn_asteroid(Game* g, Vec2 position, AsteroidSize size)
{
    foreach (&a : g.asteroids)
    {
        if (!a.active)
        {
            float angle = rand_float(g, 0.0, math::TAU);
            float speed = rand_float(g, ASTEROID_SPEED_MIN, ASTEROID_SPEED_MAX);
            *a = {
                .position = position,
                .velocity = { math::cos(angle) * speed, math::sin(angle) * speed },
                .size = size,
                .rotation = rand_float(g, 0.0, math::TAU),
                .rotation_speed = rand_float(g, -2.0, 2.0),
                .active = true,
            };
            return;
        }
    }
}

fn void spawn_wave(Game* g)
{
    int count = STARTING_ASTEROIDS + (g.wave - 1);
    for (int i = 0; i < count; i++)
    {
        Vec2 pos;
        if (random::rand(2) == 0)
        {
            pos.x = (random::rand(2) == 0) ? 0 : SCREEN_WIDTH;
            pos.y = rand_float(g, 0, SCREEN_HEIGHT);
        }
        else
        {
            pos.x = rand_float(g, 0.0, SCREEN_WIDTH);
            pos.y = (random::rand(2) == 0) ? 0 : SCREEN_HEIGHT;
        }
        spawn_asteroid(g, pos, AsteroidSize.LARGE);
    }
}

fn void init_game(Game* g)
{
    random::Sfc64Random saved_rng = g.rng;
    *g = {
        .rng = saved_rng,
        .ship = {
            .position = { SCREEN_WIDTH / 2.0, SCREEN_HEIGHT / 2.0 },
            .alive = true,
            .invulnerable_timer = INVULNERABILITY_TIME
        },
        .score = 0,
        .lives = STARTING_LIVES,
        .wave = 1,
        .state = PLAYING
    };
    spawn_wave(g);
}

fn void fire_bullet(Game* g)
{
    if (g.fire_cooldown > 0.0) return;
    foreach (&b : g.bullets)
    {
        if (!b.active)
        {
            Vec2 facing = { math::sin(g.ship.rotation), -math::cos(g.ship.rotation) };
            Vec2 nose = g.ship.position + SHIP_SIZE * facing;
            *b = {
                .position = nose,
                .velocity = BULLET_SPEED * facing,
                .lifetime = BULLET_LIFETIME,
                .active = true,
            };
            g.fire_cooldown = FIRE_COOLDOWN;
            return;
        }
    }
}

fn void split_asteroid(Game* g, Asteroid* a)
{
    g.score += a.size.score;
    if (a.size != SMALL)
    {
        AsteroidSize next = ++a.size;
        for (int i = 0; i < ASTEROID_SPLIT_COUNT; i++)
        {
            spawn_asteroid(g, a.position, next);
        }
    }
    a.active = false;
}

fn void update(Game* g, float dt)
{
    if (g.state == GAME_OVER)
    {
        if (rl::is_key_pressed(ENTER)) init_game(g);
        return;
    }

    if (g.respawn_timer > 0.0)
    {
        g.respawn_timer -= dt;
        if (g.respawn_timer <= 0.0)
        {
            g.ship.position = { SCREEN_WIDTH / 2.0, SCREEN_HEIGHT / 2.0 };
            g.ship.velocity = { 0, 0 };
            g.ship.rotation = 0.0;
            g.ship.alive = true;
            g.ship.invulnerable_timer = INVULNERABILITY_TIME;
        }
    }
    else
    {
        if (rl::is_key_down(A) || rl::is_key_down(LEFT))
        {
            g.ship.rotation -= SHIP_ROTATION_SPEED * dt;
        }
        if (rl::is_key_down(D) || rl::is_key_down(RIGHT))
        {
            g.ship.rotation += SHIP_ROTATION_SPEED * dt;
        }

        if (rl::is_key_down(W) || rl::is_key_down(UP))
        {
            Vec2 facing = { math::sin(g.ship.rotation), -math::cos(g.ship.rotation) };
            g.ship.velocity +=  SHIP_THRUST * dt * facing;
        }

        if (rl::is_key_down(SPACE)) fire_bullet(g);

        float speed = g.ship.velocity.length();
        if (speed > MAX_SHIP_SPEED)
        {
            g.ship.velocity = g.ship.velocity * MAX_SHIP_SPEED / speed;
        }

        g.ship.velocity *= math::pow(SHIP_DRAG, dt);

        g.ship.position += dt *g.ship.velocity;
        g.ship.position = wrap_position(g.ship.position);
    }

    if (g.fire_cooldown > 0.0) g.fire_cooldown -= dt;

    foreach (&b : g.bullets)
    {
        if (!b.active) continue;
        b.position += b.velocity * dt;
        b.lifetime -= dt;
        if (b.lifetime <= 0.0 ||
            b.position.x < 0 || b.position.x > SCREEN_WIDTH ||
            b.position.y < 0 || b.position.y > SCREEN_HEIGHT)
        {
            b.active = false;
        }
    }

    foreach (&a : g.asteroids)
    {
        if (!a.active) continue;
        a.position += dt *a.velocity;
        a.position = wrap_position(a.position);
        a.rotation += a.rotation_speed * dt;
    }

    foreach (&b : g.bullets)
    {
        if (!b.active) continue;
        foreach (&a : g.asteroids)
        {
            if (!a.active) continue;
            if (circle_collide(b.position, BULLET_RADIUS, a.position, a.size.radius))
            {
                b.active = false;
                split_asteroid(g, a);
                break;
            }
        }
    }

    if (g.ship.alive && g.ship.invulnerable_timer <= 0.0)
    {
        foreach (&a : g.asteroids)
        {
            if (!a.active) continue;
            if (circle_collide(g.ship.position, SHIP_COLLISION_RADIUS, a.position, a.size.radius))
            {
                g.lives -= 1;
                g.ship.alive = false;
                if (g.lives <= 0)
                {
                    g.state = GAME_OVER;
                }
                else
                {
                    g.respawn_timer = RESPAWN_DELAY;
                }
                break;
            }
        }
    }

    if (g.ship.invulnerable_timer > 0.0) g.ship.invulnerable_timer -= dt;

    bool any_asteroids = false;
    foreach (&a : g.asteroids)
    {
        if (a.active) { any_asteroids = true; break; }
    }
    if (!any_asteroids && g.state == PLAYING)
    {
        g.wave += 1;
        spawn_wave(g);
    }
}

fn void draw_ship(Game* g)
{
    if (!g.ship.alive) return;
    if (g.ship.invulnerable_timer > 0.0 && ((int)(rl::get_time() * 10.0) % 2 == 0)) return;

    Vec2 nose = {
        g.ship.position.x + math::sin(g.ship.rotation) * SHIP_SIZE,
        g.ship.position.y - math::cos(g.ship.rotation) * SHIP_SIZE,
    };
    Vec2 left = {
        g.ship.position.x + math::sin(g.ship.rotation + 2.5f) * SHIP_SIZE,
        g.ship.position.y - math::cos(g.ship.rotation + 2.5f) * SHIP_SIZE,
    };
    Vec2 right = {
        g.ship.position.x + math::sin(g.ship.rotation - 2.5f) * SHIP_SIZE,
        g.ship.position.y - math::cos(g.ship.rotation - 2.5f) * SHIP_SIZE,
    };

    rl::draw_triangle_lines(nose, left, right, rl::WHITE);
}

fn void draw(Game* g)
{
    rl::begin_drawing();
    defer rl::end_drawing();
    rl::clear_background(rl::BLACK);

    draw_ship(g);

    foreach (&a : g.asteroids)
    {
        if (!a.active) continue;
        rl::draw_circle_lines((int)a.position.x, (int)a.position.y, a.size.radius, rl::GRAY);
    }

    foreach (&b : g.bullets)
    {
        if (!b.active) continue;
        rl::draw_circle_v(b.position, BULLET_RADIUS, rl::YELLOW);
    }

    rl::draw_text(rl::text_format("SCORE %d", g.score), 10, 10, 20, rl::WHITE);
    rl::draw_text(rl::text_format("LIVES %d", g.lives), 10, 34, 20, rl::WHITE);
    rl::draw_text(rl::text_format("WAVE %d", g.wave), 10, 58, 20, rl::WHITE);

    if (g.state == GAME_OVER)
    {
        ZString msg = "GAME OVER";
        int w = rl::measure_text(msg, 40);
        rl::draw_text(msg, SCREEN_WIDTH / 2 - w / 2, SCREEN_HEIGHT / 2 - 40, 40, rl::RED);
        ZString sub = "Press ENTER to restart";
        int w2 = rl::measure_text(sub, 20);
        rl::draw_text(sub, SCREEN_WIDTH / 2 - w2 / 2, SCREEN_HEIGHT / 2 + 10, 20, rl::WHITE);
    }
}

fn int main(String[] args)
{
    rl::init_window(SCREEN_WIDTH, SCREEN_HEIGHT, "Asteroids - C3");
    defer rl::close_window();
    rl::set_target_fps(60);

    Game game;
    random::seed_entropy(&game.rng);
    init_game(&game);

    while (!rl::window_should_close())
    {
        float dt = rl::get_frame_time();
        update(&game, dt);
        draw(&game);
    }

    return 0;
}
