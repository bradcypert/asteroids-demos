use super::*;
use rand::{SeedableRng, rngs::StdRng};

fn rng() -> StdRng {
    StdRng::seed_from_u64(0x5EED_CAFE)
}

fn body_at(position: Vec2) -> AsteroidBody {
    AsteroidBody::new(position, Vec2::ZERO, Radians::ZERO, 0.0)
}

/// A session with no asteroids yet, so tests can set up the exact world
/// they need. [`PlayingGame::new`] additionally spawns the first wave.
fn empty_playing(config: GameConfig) -> PlayingGame {
    PlayingGame {
        config,
        player: Player::new(config),
        asteroids: Vec::new(),
        bullets: Vec::new(),
        score: Score::ZERO,
        wave: Wave::FIRST,
    }
}

#[test]
fn score_saturates_instead_of_overflowing() {
    let mut score = Score::new(u32::MAX - 1);
    score += Score::new(5);
    assert_eq!(score.value(), u32::MAX);
}

#[test]
fn wave_starts_at_one_and_increments() {
    assert_eq!(Wave::FIRST.value(), 1);
    assert_eq!(Wave::FIRST.next().unwrap().value(), 2);
}

#[test]
fn a_new_game_starts_with_the_first_wave() {
    let config = GameConfig {
        starting_asteroids: 3,
        ..GameConfig::default()
    };
    let playing = PlayingGame::new(config, &mut rng());

    assert_eq!(playing.wave().value(), 1);
    assert_eq!(playing.asteroids().len(), 3);
    assert!(
        playing
            .asteroids()
            .iter()
            .all(|a| a.kind() == AsteroidKind::Large)
    );
    // The ship spawns with invulnerability (and its blink) active.
    assert!(matches!(
        playing.player().ship,
        ShipState::Invulnerable { .. }
    ));
}

#[test]
fn losing_the_last_life_is_game_over() {
    assert!(matches!(
        NonZeroLives::new(1).unwrap().lose_one(),
        LifeLoss::GameOver
    ));
    assert!(matches!(
        NonZeroLives::new(3).unwrap().lose_one(),
        LifeLoss::Remaining(_)
    ));
}

#[test]
fn screen_wraps_and_bounds() {
    let screen = Screen::new(100.0, 100.0);

    assert_eq!(screen.wrap(Vec2::new(-1.0, 50.0)), Vec2::new(99.0, 50.0));
    assert_eq!(screen.wrap(Vec2::new(101.0, 50.0)), Vec2::new(1.0, 50.0));
    assert_eq!(screen.wrap(Vec2::new(-201.0, 50.0)), Vec2::new(99.0, 50.0));
    assert_eq!(screen.wrap(Vec2::new(301.0, 50.0)), Vec2::new(1.0, 50.0));
    assert_eq!(screen.wrap(Vec2::new(50.0, -1.0)), Vec2::new(50.0, 99.0));

    assert!(screen.contains(Vec2::new(100.0, 100.0)));
    assert!(!screen.contains(Vec2::new(100.5, 50.0)));
    assert!(!screen.contains(Vec2::new(50.0, -0.5)));
}

#[test]
fn radians_support_arithmetic_and_trigonometry() {
    let quarter = Radians::new(std::f32::consts::FRAC_PI_2);
    let full = quarter + quarter + quarter + quarter;
    assert!((full.value() - std::f32::consts::TAU).abs() < 1e-6);

    assert!((quarter.sin() - 1.0).abs() < 1e-6);
    assert!(quarter.cos().abs() < 1e-6);
}

#[test]
fn ship_rotates_in_radians_per_second() {
    let config = GameConfig::default();
    let mut ship = Ship::spawn(&config);

    ship.rotate(Turn::Right, Duration::from_secs(1), &config);
    assert!((ship.heading().value() - config.rotation_speed).abs() < 1e-6);

    ship.rotate(Turn::Left, Duration::from_secs(1), &config);
    assert!(ship.heading().value().abs() < 1e-6);
}

#[test]
fn weapon_fires_along_the_ship_facing() {
    let config = GameConfig::default();
    let ship = Ship::spawn(&config);
    let mut weapon = Weapon::new(WeaponConfig::from(&config));

    // Heading 0 means facing straight up: (sin 0, -cos 0) = (0, -1).
    let bullet = weapon
        .fire(FiringPose::from(&ship))
        .expect("a ready weapon should fire");
    assert!((bullet.velocity().x).abs() < 1e-3);
    assert!((bullet.velocity().y + config.bullet_speed).abs() < 1e-3);
}

#[test]
fn ship_respawns_then_becomes_invulnerable_then_active() {
    let config = GameConfig::default();

    let mut state = ShipState::Respawning {
        remaining: Duration::from_millis(500),
    };
    state.update(Duration::from_millis(400), &config);
    assert!(matches!(state, ShipState::Respawning { .. }));

    state.update(Duration::from_millis(100), &config);
    assert!(matches!(state, ShipState::Invulnerable { .. }));
    assert!(state.ship().is_some());

    state.update(Duration::from_secs(3), &config);
    assert!(matches!(state, ShipState::Active(_)));
}

#[test]
fn invulnerable_ship_ignores_hits() {
    let mut player = Player::new(GameConfig::default());
    player.ship = ShipState::Invulnerable {
        ship: Ship::spawn(&player.config),
        remaining: Duration::from_secs(1),
    };

    assert!(!player.hit());
    assert_eq!(player.lives().value(), 3);
}

#[test]
fn weapon_respects_cooldown() {
    let config = GameConfig::default();
    let ship = Ship::spawn(&config);
    let mut weapon = Weapon::new(WeaponConfig::from(&config));

    let pose = FiringPose::from(&ship);
    assert!(weapon.fire(pose).is_some());
    assert!(weapon.fire(pose).is_none());

    weapon.update(Duration::from_millis(249));
    assert!(weapon.fire(pose).is_none());

    weapon.update(Duration::from_millis(1));
    assert!(weapon.fire(pose).is_some());
}

#[test]
fn bullet_expires_and_is_culled_offscreen() {
    let screen = Screen::new(800.0, 600.0);

    let mut bullet = Bullet::new(
        Vec2::new(400.0, 300.0),
        Vec2::ZERO,
        Duration::from_millis(100),
    );
    assert!(bullet.update(Duration::from_millis(99), screen));
    assert!(!bullet.update(Duration::from_millis(1), screen));

    let mut offscreen = Bullet::new(Vec2::new(801.0, 300.0), Vec2::ZERO, Duration::from_secs(1));
    assert!(!offscreen.update(Duration::ZERO, screen));
}

#[test]
fn large_asteroid_splits_into_two_mediums() {
    let config = GameConfig::default();
    let asteroid = Asteroid::new(AsteroidKind::Large, body_at(Vec2::new(100.0, 100.0)));

    match asteroid.destroy(&mut rng(), &config) {
        AsteroidDestruction::Fragments(fragments) => {
            assert_eq!(fragments.len(), 2);
            assert!(fragments.iter().all(|a| a.kind() == AsteroidKind::Medium));
            assert_eq!(fragments[0].position(), Vec2::new(100.0, 100.0));
            assert_eq!(fragments[1].position(), Vec2::new(100.0, 100.0));
        }
        AsteroidDestruction::Destroyed => panic!("a large asteroid must split"),
    }
}

#[test]
fn small_asteroid_is_destroyed_outright() {
    let config = GameConfig::default();
    let asteroid = Asteroid::new(AsteroidKind::Small, body_at(Vec2::ZERO));

    assert!(matches!(
        asteroid.destroy(&mut rng(), &config),
        AsteroidDestruction::Destroyed
    ));
}

#[test]
fn bullet_destroys_asteroid_and_scores() {
    let mut playing = empty_playing(GameConfig::default());
    playing.asteroids.push(Asteroid::new(
        AsteroidKind::Large,
        body_at(Vec2::new(200.0, 200.0)),
    ));
    playing.bullets.push(Bullet::new(
        Vec2::new(200.0, 200.0),
        Vec2::ZERO,
        Duration::from_secs(1),
    ));

    let outcome = playing.update(&Input::default(), Duration::ZERO, &mut rng());

    assert_eq!(outcome, PlayOutcome::Continued);
    assert_eq!(playing.score().value(), 20);
    assert!(playing.bullets().is_empty());
    assert_eq!(playing.asteroids().len(), 2);
    assert!(
        playing
            .asteroids()
            .iter()
            .all(|a| a.kind() == AsteroidKind::Medium)
    );
}

#[test]
fn ship_hit_drains_a_life_and_respawns() {
    let mut playing = empty_playing(GameConfig::default());
    // The ship spawns invulnerable; make it hittable for this test.
    playing.player.ship = ShipState::Active(Ship::spawn(&playing.config));
    let center = playing.config.screen.center();
    playing
        .asteroids
        .push(Asteroid::new(AsteroidKind::Small, body_at(center)));

    let outcome = playing.update(&Input::default(), Duration::ZERO, &mut rng());

    assert_eq!(outcome, PlayOutcome::Continued);
    assert_eq!(playing.player().lives().value(), 2);
    assert!(matches!(
        playing.player().ship,
        ShipState::Respawning { .. }
    ));
}

#[test]
fn losing_the_last_ship_ends_the_game() {
    let config = GameConfig {
        starting_lives: NonZeroLives::new(1).unwrap(),
        ..GameConfig::default()
    };
    let mut playing = empty_playing(config);
    // The ship spawns invulnerable; make it hittable for this test.
    playing.player.ship = ShipState::Active(Ship::spawn(&playing.config));
    let center = playing.config.screen.center();
    playing
        .asteroids
        .push(Asteroid::new(AsteroidKind::Small, body_at(center)));

    let outcome = playing.update(&Input::default(), Duration::ZERO, &mut rng());

    assert_eq!(outcome, PlayOutcome::PlayerKilled(Score::ZERO));
}

#[test]
fn clearing_a_wave_spawns_the_next() {
    let mut playing = empty_playing(GameConfig {
        starting_asteroids: 1,
        ..GameConfig::default()
    });
    playing.asteroids.push(Asteroid::new(
        AsteroidKind::Small,
        body_at(Vec2::new(200.0, 200.0)),
    ));
    playing.bullets.push(Bullet::new(
        Vec2::new(200.0, 200.0),
        Vec2::ZERO,
        Duration::from_secs(1),
    ));

    let outcome = playing.update(&Input::default(), Duration::ZERO, &mut rng());

    assert_eq!(outcome, PlayOutcome::Continued);
    // Wave 2 spawns `starting_asteroids + 1` large asteroids.
    assert_eq!(playing.wave().value(), 2);
    assert_eq!(playing.asteroids().len(), 2);
    assert!(
        playing
            .asteroids()
            .iter()
            .all(|a| a.kind() == AsteroidKind::Large)
    );
}

#[test]
fn firing_is_rate_limited_by_the_weapon() {
    let mut playing = empty_playing(GameConfig::default());

    let fire = Input {
        fire: true,
        ..Input::default()
    };
    playing.update(&fire, Duration::ZERO, &mut rng());
    assert_eq!(playing.bullets().len(), 1);

    // Holding fire during the cooldown produces no extra bullets.
    playing.update(&fire, Duration::from_millis(100), &mut rng());
    assert_eq!(playing.bullets().len(), 1);

    // This frame drains the last 150ms of cooldown, but the fire attempt
    // happens before the drain — matching the original game — so no shot.
    playing.update(&fire, Duration::from_millis(200), &mut rng());
    assert_eq!(playing.bullets().len(), 1);

    // Cooldown is over; the next frame fires again.
    playing.update(&fire, Duration::ZERO, &mut rng());
    assert_eq!(playing.bullets().len(), 2);
}
