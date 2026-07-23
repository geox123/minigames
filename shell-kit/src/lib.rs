//! Shared shell primitives every Game in the Collection reuses: a crisp pixel
//! font, the logical-canvas scaling, audio synthesis, and a fixed-timestep
//! accumulator.
//!
//! This crate holds only the stable, Game-agnostic pieces — nothing about any
//! particular Game's rules, screens or feel. Those stay in each Game's own
//! shell. Everything here is parameterised by the Game's logical resolution, so
//! Pong's landscape field and Breakout's portrait field both work.

pub mod font;
pub mod screen;
pub mod synth;
pub mod timestep;
