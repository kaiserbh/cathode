//! Source adapters: turn a provider's wire format into the normalized model.
//!
//! Each source lives in its own submodule. They all emit the same `Stream` /
//! `Category` types; the only place a source's identity is allowed to surface is
//! URL resolution, which is modelled on the source itself.

pub mod xtream;
