//! The site model: believable synthetic ground for `--site` (ADR 0001).
//!
//! A preset ([`preset`]) tells the site's story; [`build`] generates holes and
//! strata from it with keyed draws ([`draw`]) over smooth fields ([`ground`]),
//! and lowers them into the model the emitter writes.

pub mod build;
pub mod cover;
mod draw;
mod ground;
pub mod preset;
pub mod properties;
mod results;
pub mod suite;
mod water;
