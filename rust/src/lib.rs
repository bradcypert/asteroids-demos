//! The game domain for Asteroids, kept free of any rendering or windowing
//! dependency so it can be unit-tested and reused independently of the UI.
//!
//! The binary crate (`src/main.rs`) owns the raylib window, maps input and
//! draws the state exposed by this crate.

pub mod domain;
