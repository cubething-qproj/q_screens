//! Screen cleanup despawns every entity that isn't [`Persistent`]. This module
//! lets whole component types opt out: register a type with
//! [`RegisterPersistentType::register_persistent_type`] and a
//! [`PersistentTypeScope`], and entities with it survive cleanup.
//!
//! - [`PersistentTypeScope::TopLevel`]: only entities without a parent
//!   ([`ChildOf`]) survive.
//! - [`PersistentTypeScope::Anywhere`]: every entity with the type survives.
//!
//! Cleanup is per entity: children of an exempt entity survive only if they are
//! exempt (or [`Persistent`]) themselves.
//!
//! [`ScreenPlugin`] registers Bevy's engine-internal types, audited against Bevy
//! 0.19.1:
//!
//! | Type | Scope | Feature |
//! | --- | --- | --- |
//! | [`IsResource`] (resource entities) | Anywhere | always |
//! | [`Observer`] (global observers) | TopLevel | always |
//! | [`SystemIdMarker`] (one-shot systems) | TopLevel | always |
//! | [`Window`] | TopLevel | always |
//! | [`Monitor`] | Anywhere | always |
//! | `Gamepad` | Anywhere | `bevy_gilrs` |
//! | `PointerId` | Anywhere | `bevy_picking` |
//!
//! Observers: an entity observer (`.observe(...)`) is despawned by Bevy along
//! with the last entity it watches, and an observer parented to a screen entity
//! ([`ChildOf`]) is cleaned up with the screen. Only unparented, global
//! observers survive.
//!
//! Registration is per type, not per value: every `PointerId` survives,
//! including `PointerId::Custom` pointers an app spawns itself. Despawn those
//! yourself when a screen ends.
//!
//! Not covered yet: entities from `bevy_gizmos_render` (`LineGizmoEntities`, the
//! transform gizmo) and `bevy_dev_tools` overlays use private or no marker
//! types, so they are still cleaned up on screen changes. Mark them
//! [`Persistent`] yourself if you use them.
//!
//! Audit this list on each Bevy bump with `just audit-bevy-spawns <old> <new>`,
//! which prints the main-world spawn sites Bevy added or removed between the two
//! versions. `CONTRIBUTING.md` has the procedure and a rule of thumb for which
//! entities should survive screen changes.

use bevy::{
    ecs::{component::ComponentId, resource::IsResource, system::SystemIdMarker},
    window::Monitor,
};

use crate::prelude::*;

/// Where a persistent type exempts its entities from screen cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PersistentTypeScope {
    /// Only entities without a parent ([`ChildOf`]) are exempt.
    TopLevel,
    /// Every entity with the type is exempt, parented or not.
    Anywhere,
}

/// Component types whose entities survive screen cleanup: a type-level opt-out
/// from the screen-scoped-by-default rule (see [`ScreenScoped`], [`Persistent`]
/// and [`ScreenScopeBuilder`] for how scoping works). See the [module
/// docs](self) for the engine-internal types [`ScreenPlugin`] registers here.
/// Register your own with [`RegisterPersistentType::register_persistent_type`].
#[derive(Resource, Default, Debug)]
pub struct PersistentTypes {
    top_level: Vec<ComponentId>,
    anywhere: Vec<ComponentId>,
}

impl PersistentTypes {
    /// Registered component ids exempt only at the top level.
    pub fn top_level(&self) -> &[ComponentId] {
        &self.top_level
    }

    /// Registered component ids exempt anywhere in a hierarchy.
    pub fn anywhere(&self) -> &[ComponentId] {
        &self.anywhere
    }

    fn insert(&mut self, id: ComponentId, scope: PersistentTypeScope) {
        let ids = match scope {
            PersistentTypeScope::TopLevel => &mut self.top_level,
            PersistentTypeScope::Anywhere => &mut self.anywhere,
        };
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
}

/// Registers component types in [`PersistentTypes`]. Registration can happen
/// before or after [`ScreenPlugin`] is added, and before or after any entity
/// with the type exists.
pub trait RegisterPersistentType {
    /// Entities with `T` survive screen cleanup within `scope`.
    fn register_persistent_type<T: Component>(&mut self, scope: PersistentTypeScope) -> &mut Self;
}

impl RegisterPersistentType for App {
    fn register_persistent_type<T: Component>(&mut self, scope: PersistentTypeScope) -> &mut Self {
        let world = self.world_mut();
        let id = world.register_component::<T>();
        world
            .get_resource_or_init::<PersistentTypes>()
            .insert(id, scope);
        self
    }
}

/// Registers Bevy's engine-internal persistent types (see the module docs).
pub(crate) fn plugin(app: &mut App) {
    use PersistentTypeScope::*;
    app.register_persistent_type::<IsResource>(Anywhere)
        .register_persistent_type::<Observer>(TopLevel)
        .register_persistent_type::<SystemIdMarker>(TopLevel)
        .register_persistent_type::<Window>(TopLevel)
        .register_persistent_type::<Monitor>(Anywhere);
    #[cfg(feature = "bevy_gilrs")]
    app.register_persistent_type::<bevy::input::gamepad::Gamepad>(Anywhere);
    #[cfg(feature = "bevy_picking")]
    app.register_persistent_type::<bevy::picking::pointer::PointerId>(Anywhere);
}

#[cfg(test)]
mod tests {
    use crate::{prelude::*, scope::scoped_entities};

    /// Spawns `component` top-level and as a child; both must survive cleanup.
    #[allow(dead_code, reason = "only used by feature-gated tests")]
    fn assert_survives_anywhere<C: Bundle>(component: impl Fn() -> C) {
        let mut app = App::new();
        app.add_plugins(super::plugin);
        let world = app.world_mut();
        let top_level = world.spawn(component()).id();
        let parent = world.spawn_empty().id();
        let child = world.spawn((component(), ChildOf(parent))).id();
        let scoped = scoped_entities(world);
        assert!(
            !scoped.contains(&top_level),
            "top-level entity is cleaned up"
        );
        assert!(!scoped.contains(&child), "child entity is cleaned up");
    }

    #[cfg(feature = "bevy_gilrs")]
    #[test]
    fn gamepads_survive_cleanup() {
        assert_survives_anywhere(bevy::input::gamepad::Gamepad::default);
    }

    #[cfg(feature = "bevy_picking")]
    #[test]
    fn pointers_survive_cleanup() {
        assert_survives_anywhere(|| bevy::picking::pointer::PointerId::Mouse);
    }
}
