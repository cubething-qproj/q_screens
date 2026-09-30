#![doc = include_str!("../README.md")]
#![feature(register_tool)]
#![register_tool(bevy)]
#![allow(bevy::panicking_methods)]
#![deny(missing_docs)]

#[allow(unused_imports, reason = "used in docs")]
use prelude::*;
/// Resources, components, states, etc.
pub mod data;
pub mod persistent;
mod plugin;
/// The [ScreenScopeBuilder] and friends.
pub mod scope;
mod systems;
/// The [Screen] trait.
pub mod trait_impl;

/// The main export.
pub mod prelude {
    pub use super::data::*;
    pub use super::persistent::*;
    pub use super::plugin::*;
    pub use super::scope::*;
    pub(crate) use super::systems::*;
    pub use super::trait_impl::*;
    pub(crate) use bevy::prelude::*;
    pub(crate) use std::marker::PhantomData;
    pub(crate) use tiny_bail::prelude::*;
}
