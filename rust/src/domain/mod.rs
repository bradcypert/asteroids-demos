use std::{
    num::{NonZeroU8, NonZeroU32},
    ops::{Add, AddAssign, Sub},
    time::Duration,
};

use glam::Vec2;
use rand::RngExt;

/// An angle in radians.
///
/// A dedicated type keeps the degrees-vs-radians mistake impossible: the only
/// way to feed an angle to trigonometry or combine it with other angles is
/// through [`Radians`], so a raw `f32` can never silently be treated as
/// degrees. Angular *rates* (radians per second) are deliberately not this
/// type — an angle is not a rate.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Radians(f32);

impl Radians {
    const ZERO: Self = Self(0.0);

    pub const fn new(radians: f32) -> Self {
        Self(radians)
    }

    const fn value(self) -> f32 {
        self.0
    }

    fn random(rng: &mut impl RngExt) -> Self {
        Self(rng.random_range(0.0..std::f32::consts::TAU))
    }

    pub fn sin(self) -> f32 {
        self.0.sin()
    }

    pub fn cos(self) -> f32 {
        self.0.cos()
    }

    fn sin_cos(self) -> (f32, f32) {
        self.0.sin_cos()
    }
}

impl Add for Radians {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for Radians {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Radians {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

/// The playfield rectangle, in world units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Screen {
    width: f32,
    height: f32,
}
impl Screen {
    const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub const fn width(self) -> f32 {
        self.width
    }

    pub const fn height(self) -> f32 {
        self.height
    }

    const fn center(self) -> Vec2 {
        Vec2::new(self.width / 2.0, self.height / 2.0)
    }

    /// Wrap a position around the edges, classic-Asteroids style.
    fn wrap(self, mut pos: Vec2) -> Vec2 {
        if !(0.0..=self.width).contains(&pos.x) {
            pos.x = pos.x.rem_euclid(self.width);
        }
        if !(0.0..=self.height).contains(&pos.y) {
            pos.y = pos.y.rem_euclid(self.height);
        }
        pos
    }

    /// Whether a point is inside the playfield (inclusive of the edges).
    fn contains(self, pos: Vec2) -> bool {
        pos.x >= 0.0 && pos.x <= self.width && pos.y >= 0.0 && pos.y <= self.height
    }

    /// A random point on the screen edge, where new asteroids appear.
    fn random_edge(self, rng: &mut impl RngExt) -> Vec2 {
        if rng.random_bool(0.5) {
            let x = if rng.random_bool(0.5) {
                0.0
            } else {
                self.width
            };
            let y = rng.random_range(0.0..self.height);
            Vec2::new(x, y)
        } else {
            let x = rng.random_range(0.0..self.width);
            let y = if rng.random_bool(0.5) {
                0.0
            } else {
                self.height
            };
            Vec2::new(x, y)
        }
    }
}

/// A range of `f32` values for random sampling; `max` is exclusive, matching
/// `random_range(min..max)`.
#[derive(Debug, Clone, Copy)]
struct FloatRange {
    min: f32,
    max: f32,
}

impl FloatRange {
    const fn new(min: f32, max: f32) -> Self {
        Self { min, max }
    }

    fn sample(self, rng: &mut impl RngExt) -> f32 {
        rng.random_range(self.min..self.max)
    }
}

/// The game's gameplay tuning.
#[derive(Debug, Clone, Copy)]
pub struct GameConfig {
    screen: Screen,
    starting_lives: NonZeroLives,
    starting_asteroids: usize,
    respawn_time: Duration,
    invulnerability_time: Duration,
    /// Ship rotation speed, in radians per second.
    rotation_speed: f32,
    thrust: f32,
    drag: f32,
    max_speed: f32,
    ship_size: f32,
    ship_collision_radius: f32,
    bullet_speed: f32,
    bullet_lifetime: Duration,
    bullet_radius: f32,
    fire_cooldown: Duration,
    asteroid_speed: FloatRange,
    /// Asteroid angular velocity, in radians per second.
    asteroid_rotation: FloatRange,
}

impl GameConfig {
    pub const fn screen(&self) -> Screen {
        self.screen
    }

    pub const fn ship_size(&self) -> f32 {
        self.ship_size
    }

    pub const fn bullet_radius(&self) -> f32 {
        self.bullet_radius
    }
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            screen: Screen::new(800.0, 600.0),
            starting_lives: NonZeroLives::new(3).expect("3 lives is non-zero"),
            starting_asteroids: 4,
            respawn_time: Duration::from_millis(1_500),
            invulnerability_time: Duration::from_secs(2),
            rotation_speed: 3.5,
            thrust: 220.0,
            drag: 0.60,
            max_speed: 380.0,
            ship_size: 20.0,
            ship_collision_radius: 12.0,
            bullet_speed: 520.0,
            bullet_lifetime: Duration::from_millis(1_100),
            bullet_radius: 2.0,
            fire_cooldown: Duration::from_millis(250),
            asteroid_speed: FloatRange::new(40.0, 110.0),
            asteroid_rotation: FloatRange::new(-2.0, 2.0),
        }
    }
}

/// The top-level game: either a live session or the game-over screen.
#[derive(Debug)]
pub struct Game {
    config: GameConfig,
    state: GameState,
}

impl Game {
    pub fn new(config: GameConfig, rng: &mut impl RngExt) -> Self {
        Self {
            config,
            state: GameState::Playing(PlayingGame::new(config, rng)),
        }
    }

    pub fn config(&self) -> &GameConfig {
        &self.config
    }

    pub fn playing(&self) -> Option<&PlayingGame> {
        match &self.state {
            GameState::Playing(playing) => Some(playing),
            GameState::GameOver(_) => None,
        }
    }

    pub fn final_score(&self) -> Option<Score> {
        match self.state {
            GameState::Playing(_) => None,
            GameState::GameOver(game_over) => Some(game_over.final_score()),
        }
    }

    pub fn is_game_over(&self) -> bool {
        matches!(self.state, GameState::GameOver(_))
    }

    /// Start a fresh game with the same configuration.
    pub fn restart(&mut self, rng: &mut impl RngExt) {
        self.state = GameState::Playing(PlayingGame::new(self.config, rng));
    }

    /// Advance the simulation by one frame. No-op while on the game-over
    /// screen; start a new game with [`Self::restart`] instead.
    pub fn update(&mut self, input: &Input, dt: Duration, rng: &mut impl RngExt) {
        let outcome = match &mut self.state {
            GameState::Playing(playing) => playing.update(input, dt, rng),
            GameState::GameOver(_) => return,
        };

        if let PlayOutcome::PlayerKilled(score) = outcome {
            self.state = GameState::GameOver(GameOver { final_score: score });
        }
    }
}

/// The two mutually exclusive phases of a game session.
///
/// `PlayingGame` is far larger than `GameOver`, but the enum is constructed
/// once per session and mutated in place, never copied or stored in bulk, so
/// boxing the large variant would only add an allocation.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum GameState {
    Playing(PlayingGame),
    GameOver(GameOver),
}

/// The game-over screen. Owns only the score: the world is gone.
#[derive(Debug, Clone, Copy)]
struct GameOver {
    final_score: Score,
}

impl GameOver {
    fn final_score(&self) -> Score {
        self.final_score
    }
}

/// What the player asked for this frame, already translated from whatever
/// input devices exist.
#[derive(Debug, Clone, Copy, Default)]
pub struct Input {
    pub turn: Option<Turn>,
    pub thrust: bool,
    pub fire: bool,
}

/// How a frame of play ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayOutcome {
    /// The session continues.
    Continued,
    /// The player lost their last ship; `score` is the final score.
    PlayerKilled(Score),
}

/// A live session: the player, every asteroid and every bullet in flight.
#[derive(Debug)]
pub struct PlayingGame {
    config: GameConfig,
    player: Player,
    asteroids: Vec<Asteroid>,
    bullets: Vec<Bullet>,
    score: Score,
    wave: Wave,
}

impl PlayingGame {
    /// A fresh session with the first wave already spawned.
    fn new(config: GameConfig, rng: &mut impl RngExt) -> Self {
        let mut game = Self {
            config,
            player: Player::new(config),
            asteroids: Vec::with_capacity(16),
            bullets: Vec::with_capacity(8),
            score: Score::ZERO,
            wave: Wave::FIRST,
        };
        game.spawn_wave(rng);
        game
    }

    pub fn player(&self) -> &Player {
        &self.player
    }

    pub fn asteroids(&self) -> &[Asteroid] {
        &self.asteroids
    }

    pub fn bullets(&self) -> &[Bullet] {
        &self.bullets
    }

    pub fn score(&self) -> Score {
        self.score
    }

    pub fn wave(&self) -> Wave {
        self.wave
    }

    /// Advance one frame: input, physics, collisions, wave progression.
    fn update(&mut self, input: &Input, dt: Duration, rng: &mut impl RngExt) -> PlayOutcome {
        let screen = self.config.screen;

        if let Some(turn) = input.turn {
            self.player.rotate(turn, dt);
        }
        if input.thrust {
            self.player.accelerate(dt);
        }
        if input.fire
            && let Some(bullet) = self.player.fire()
        {
            self.bullets.push(bullet);
        }

        self.player.update(dt);

        self.bullets.retain_mut(|bullet| bullet.update(dt, screen));
        for asteroid in &mut self.asteroids {
            asteroid.update(dt, screen);
        }

        self.resolve_bullet_hits(rng);

        if self.ship_is_hit() && self.player.hit() {
            return PlayOutcome::PlayerKilled(self.score);
        }

        if self.asteroids.is_empty() {
            if let Some(next) = self.wave.next() {
                self.wave = next;
            }
            self.spawn_wave(rng);
        }

        PlayOutcome::Continued
    }

    fn ship_is_hit(&self) -> bool {
        self.player.ship().is_some_and(|ship| {
            self.asteroids.iter().any(|asteroid| {
                circle_collide(
                    ship.position(),
                    self.config.ship_collision_radius,
                    asteroid.position(),
                    asteroid.radius(),
                )
            })
        })
    }

    /// Destroy every asteroid a bullet touches, scoring the hits and spawning
    /// fragments. Bullets and asteroids are removed in place, so the frame
    /// performs no allocations, and fragments spawned mid-frame are
    /// immediately hittable — matching the original game.
    fn resolve_bullet_hits(&mut self, rng: &mut impl RngExt) {
        let bullet_radius = self.config.bullet_radius;

        let mut bullet_index = 0;
        while bullet_index < self.bullets.len() {
            let bullet_position = self.bullets[bullet_index].position();

            let hit = self.asteroids.iter().position(|asteroid| {
                circle_collide(
                    bullet_position,
                    bullet_radius,
                    asteroid.position(),
                    asteroid.radius(),
                )
            });

            if let Some(asteroid_index) = hit {
                let asteroid = self.asteroids.swap_remove(asteroid_index);
                self.score += asteroid.score();
                match asteroid.destroy(rng, &self.config) {
                    AsteroidDestruction::Fragments(parts) => self.asteroids.extend(parts),
                    AsteroidDestruction::Destroyed => {}
                }
                self.bullets.swap_remove(bullet_index);
            } else {
                bullet_index += 1;
            }
        }
    }

    fn spawn_wave(&mut self, rng: &mut impl RngExt) {
        let count = self.config.starting_asteroids + (self.wave.value() as usize - 1);
        for _ in 0..count {
            let body =
                AsteroidBody::random_at(self.config.screen.random_edge(rng), rng, &self.config);
            self.asteroids
                .push(Asteroid::new(AsteroidKind::Large, body));
        }
    }
}

/// The player's score, a non-negative value that saturates instead of
/// overflowing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Score(u32);

impl Score {
    const ZERO: Self = Self(0);

    const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

impl AddAssign for Score {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_add(rhs.0);
    }
}

/// A wave number, always at least 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Wave(NonZeroU32);

impl Wave {
    const FIRST: Self = Self(NonZeroU32::MIN);

    pub const fn value(self) -> u32 {
        self.0.get()
    }

    fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// Remaining lives, guaranteed non-zero: a game over is represented by
/// [`LifeLoss::GameOver`], not by a zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NonZeroLives(NonZeroU8);

impl NonZeroLives {
    pub fn new(value: u8) -> Option<Self> {
        NonZeroU8::new(value).map(Self)
    }

    pub const fn value(self) -> u8 {
        self.0.get()
    }

    fn lose_one(self) -> LifeLoss {
        NonZeroU8::new(self.0.get() - 1)
            .map(Self)
            .map_or(LifeLoss::GameOver, LifeLoss::Remaining)
    }
}

enum LifeLoss {
    Remaining(NonZeroLives),
    GameOver,
}

/// The player: their remaining lives, the ship (whatever state it is in) and
/// the ship's weapon.
#[derive(Debug)]
pub struct Player {
    config: GameConfig,
    lives: NonZeroLives,
    ship: ShipState,
    weapon: Weapon,
}

impl Player {
    fn new(config: GameConfig) -> Self {
        Self {
            config,
            lives: config.starting_lives,
            ship: ShipState::Invulnerable {
                ship: Ship::spawn(&config),
                remaining: config.invulnerability_time,
            },
            weapon: Weapon::new(WeaponConfig::from(&config)),
        }
    }

    pub fn lives(&self) -> NonZeroLives {
        self.lives
    }

    pub fn ship(&self) -> Option<&Ship> {
        self.ship.ship()
    }

    pub fn is_invulnerable(&self) -> bool {
        matches!(self.ship, ShipState::Invulnerable { .. })
    }

    fn update(&mut self, dt: Duration) {
        self.ship.update(dt, &self.config);
        self.weapon.update(dt);
    }

    /// The ship was hit by an asteroid: it loses a life and starts respawning,
    /// unless that was the last life. Invulnerable and respawning ships are
    /// unaffected. Returns `true` when the player has no lives left — i.e. the
    /// game is over.
    fn hit(&mut self) -> bool {
        if !matches!(self.ship, ShipState::Active(_)) {
            return false;
        }

        match self.lives.lose_one() {
            LifeLoss::Remaining(lives) => {
                self.lives = lives;
                self.ship = ShipState::Respawning {
                    remaining: self.config.respawn_time,
                };
                false
            }
            LifeLoss::GameOver => true,
        }
    }

    fn rotate(&mut self, turn: Turn, dt: Duration) {
        if let Some(ship) = self.ship.ship_mut() {
            ship.rotate(turn, dt, &self.config);
        }
    }

    fn accelerate(&mut self, dt: Duration) {
        if let Some(ship) = self.ship.ship_mut() {
            ship.accelerate(dt, &self.config);
        }
    }

    fn fire(&mut self) -> Option<Bullet> {
        let ship = self.ship.ship()?;
        self.weapon.fire(FiringPose::from(ship))
    }
}

/// The three states a ship can be in. There is no "dead" state: a dead ship is
/// either respawning or the game is over.
#[derive(Debug)]
enum ShipState {
    Active(Ship),

    /// Recently respawned; the ship exists but collisions are ignored.
    Invulnerable {
        ship: Ship,
        remaining: Duration,
    },

    /// Waiting to respawn; there is no ship at all.
    Respawning {
        remaining: Duration,
    },
}

impl ShipState {
    fn update(&mut self, dt: Duration, config: &GameConfig) {
        if let Self::Active(ship) = self {
            ship.update(dt, config);
            return;
        }

        if let Self::Invulnerable { ship, remaining } = self {
            ship.update(dt, config);
            if Self::tick(remaining, dt) {
                let ship = std::mem::replace(ship, Ship::spawn(config));
                *self = Self::Active(ship);
            }
            return;
        }

        if let Self::Respawning { remaining } = self
            && Self::tick(remaining, dt)
        {
            *self = Self::Invulnerable {
                ship: Ship::spawn(config),
                remaining: config.invulnerability_time,
            };
        }
    }

    fn tick(remaining: &mut Duration, dt: Duration) -> bool {
        *remaining = remaining.saturating_sub(dt);
        remaining.is_zero()
    }

    /// The ship, if one currently exists.
    fn ship(&self) -> Option<&Ship> {
        match self {
            Self::Active(ship) | Self::Invulnerable { ship, .. } => Some(ship),
            Self::Respawning { .. } => None,
        }
    }

    fn ship_mut(&mut self) -> Option<&mut Ship> {
        match self {
            Self::Active(ship) | Self::Invulnerable { ship, .. } => Some(ship),
            Self::Respawning { .. } => None,
        }
    }
}

/// The player's ship: a position, a velocity and a heading.
#[derive(Debug)]
pub struct Ship {
    position: Vec2,
    velocity: Vec2,
    heading: Radians,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Turn {
    Left,
    Right,
}

impl Turn {
    /// Rotation direction on screen: +1 for right, -1 for left.
    fn direction(self) -> f32 {
        match self {
            Turn::Left => -1.0,
            Turn::Right => 1.0,
        }
    }
}

impl Ship {
    fn spawn(config: &GameConfig) -> Self {
        Self {
            position: config.screen.center(),
            velocity: Vec2::ZERO,
            heading: Radians::ZERO,
        }
    }

    pub fn position(&self) -> Vec2 {
        self.position
    }

    /// The ship's heading; it faces `(sin θ, -cos θ)`.
    pub fn heading(&self) -> Radians {
        self.heading
    }

    fn update(&mut self, dt: Duration, config: &GameConfig) {
        let dt = dt.as_secs_f32();
        self.velocity *= config.drag.powf(dt);
        self.position += self.velocity * dt;
        self.position = config.screen.wrap(self.position);
    }

    fn rotate(&mut self, turn: Turn, dt: Duration, config: &GameConfig) {
        self.heading += Radians::new(turn.direction() * config.rotation_speed * dt.as_secs_f32());
    }

    fn accelerate(&mut self, dt: Duration, config: &GameConfig) {
        self.velocity += self.facing() * (config.thrust * dt.as_secs_f32());
        self.limit_speed(config.max_speed);
    }

    fn facing(&self) -> Vec2 {
        let (sin, cos) = self.heading.sin_cos();
        Vec2::new(sin, -cos)
    }

    fn limit_speed(&mut self, max_speed: f32) {
        let speed = self.velocity.length();
        if speed > max_speed {
            self.velocity *= max_speed / speed;
        }
    }
}

/// The position and direction needed to fire a weapon.
#[derive(Debug, Clone, Copy)]
struct FiringPose {
    origin: Vec2,
    direction: Vec2,
}

impl From<&Ship> for FiringPose {
    fn from(ship: &Ship) -> Self {
        Self {
            origin: ship.position,
            direction: ship.facing(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct WeaponConfig {
    cooldown: Duration,
    muzzle_offset: f32,
    bullet_speed: f32,
    bullet_lifetime: Duration,
}

impl WeaponConfig {
    const fn new(
        cooldown: Duration,
        muzzle_offset: f32,
        bullet_speed: f32,
        bullet_lifetime: Duration,
    ) -> Self {
        Self {
            cooldown,
            muzzle_offset,
            bullet_speed,
            bullet_lifetime,
        }
    }
}

impl From<&GameConfig> for WeaponConfig {
    fn from(config: &GameConfig) -> Self {
        Self::new(
            config.fire_cooldown,
            config.ship_size,
            config.bullet_speed,
            config.bullet_lifetime,
        )
    }
}

/// The ship's weapon: ready to fire, or cooling down.
#[derive(Debug)]
struct Weapon {
    config: WeaponConfig,
    state: WeaponState,
}

#[derive(Debug)]
enum WeaponState {
    Ready,
    CoolingDown { remaining: Duration },
}

impl Weapon {
    fn new(config: WeaponConfig) -> Self {
        Self {
            config,
            state: WeaponState::Ready,
        }
    }

    /// Try to fire: creates a bullet and consumes the cooldown when ready.
    fn fire(&mut self, pose: FiringPose) -> Option<Bullet> {
        if !matches!(self.state, WeaponState::Ready) {
            return None;
        }

        let facing = pose.direction;
        self.state = WeaponState::CoolingDown {
            remaining: self.config.cooldown,
        };
        Some(Bullet::new(
            pose.origin + facing * self.config.muzzle_offset,
            facing * self.config.bullet_speed,
            self.config.bullet_lifetime,
        ))
    }

    fn update(&mut self, dt: Duration) {
        if let WeaponState::CoolingDown { remaining } = &mut self.state {
            *remaining = remaining.saturating_sub(dt);
            if remaining.is_zero() {
                self.state = WeaponState::Ready;
            }
        }
    }
}

/// A bullet in flight. Dies when its lifetime expires or it leaves the screen
/// (bullets do not wrap, matching the original game).
#[derive(Debug, Clone, Copy)]
pub struct Bullet {
    position: Vec2,
    velocity: Vec2,
    remaining: Duration,
}

impl Bullet {
    fn new(position: Vec2, velocity: Vec2, remaining: Duration) -> Self {
        Self {
            position,
            velocity,
            remaining,
        }
    }

    pub fn position(&self) -> Vec2 {
        self.position
    }

    #[cfg(test)]
    fn velocity(&self) -> Vec2 {
        self.velocity
    }

    /// Advance one frame; returns false when the bullet should be removed.
    fn update(&mut self, dt: Duration, screen: Screen) -> bool {
        self.position += self.velocity * dt.as_secs_f32();
        self.remaining = self.remaining.saturating_sub(dt);
        !self.remaining.is_zero() && screen.contains(self.position)
    }
}

/// Kinematic state shared by every asteroid, regardless of size.
#[derive(Debug)]
struct AsteroidBody {
    position: Vec2,
    velocity: Vec2,
    rotation: Radians,
    /// Angular velocity, in radians per second.
    angular_velocity: f32,
}

impl AsteroidBody {
    #[cfg(test)]
    fn new(position: Vec2, velocity: Vec2, rotation: Radians, angular_velocity: f32) -> Self {
        Self {
            position,
            velocity,
            rotation,
            angular_velocity,
        }
    }

    fn random_at(position: Vec2, rng: &mut impl RngExt, config: &GameConfig) -> Self {
        Self {
            position,
            velocity: Vec2::from_angle(Radians::random(rng).value())
                * config.asteroid_speed.sample(rng),
            rotation: Radians::random(rng),
            angular_velocity: config.asteroid_rotation.sample(rng),
        }
    }

    fn position(&self) -> Vec2 {
        self.position
    }

    fn update(&mut self, dt: Duration) {
        let dt = dt.as_secs_f32();
        self.position += self.velocity * dt;
        self.rotation += Radians::new(self.angular_velocity * dt);
    }
}

/// The size class of an asteroid; determines how it splits and what it scores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AsteroidKind {
    Large,
    Medium,
    Small,
}

impl AsteroidKind {
    fn radius(self) -> f32 {
        match self {
            Self::Large => 40.0,
            Self::Medium => 22.0,
            Self::Small => 12.0,
        }
    }

    fn score(self) -> Score {
        match self {
            Self::Large => Score::new(20),
            Self::Medium => Score::new(50),
            Self::Small => Score::new(100),
        }
    }

    fn next_smaller(self) -> Option<Self> {
        match self {
            Self::Large => Some(Self::Medium),
            Self::Medium => Some(Self::Small),
            Self::Small => None,
        }
    }
}

/// An asteroid with a size kind and shared kinematic state.
#[derive(Debug)]
pub struct Asteroid {
    kind: AsteroidKind,
    body: AsteroidBody,
}

/// The result of destroying an asteroid.
enum AsteroidDestruction {
    /// The asteroid split into two fragments of the next-smaller size.
    Fragments([Asteroid; 2]),
    /// Small asteroids are destroyed outright.
    Destroyed,
}

impl Asteroid {
    /// Spawning is domain-internal; gameplay uses [`PlayingGame`] to create
    /// asteroids.
    fn new(kind: AsteroidKind, body: AsteroidBody) -> Self {
        Self { kind, body }
    }

    #[cfg(test)]
    fn kind(&self) -> AsteroidKind {
        self.kind
    }

    pub fn position(&self) -> Vec2 {
        self.body.position()
    }

    pub fn radius(&self) -> f32 {
        self.kind.radius()
    }

    fn score(&self) -> Score {
        self.kind.score()
    }

    fn update(&mut self, dt: Duration, screen: Screen) {
        self.body.update(dt);
        self.body.position = screen.wrap(self.body.position);
    }

    fn destroy(self, rng: &mut impl RngExt, config: &GameConfig) -> AsteroidDestruction {
        let Self { kind, body } = self;
        let Some(fragment_kind) = kind.next_smaller() else {
            return AsteroidDestruction::Destroyed;
        };

        AsteroidDestruction::Fragments(std::array::from_fn(|_| {
            Self::new(
                fragment_kind,
                AsteroidBody::random_at(body.position, rng, config),
            )
        }))
    }
}

fn circle_collide(a: Vec2, radius_a: f32, b: Vec2, radius_b: f32) -> bool {
    (a - b).length() < radius_a + radius_b
}

#[cfg(test)]
mod tests;
