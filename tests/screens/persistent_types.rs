//! Screen cleanup and [PersistentTypes]: see the q_screens persistent-types design.

use bevy::{ecs::entity::Entities, window::Monitor};

use crate::prelude::*;

/// An entity and whether it should survive the first screen's cleanup.
#[derive(Resource, Default)]
struct Expectations(Vec<(&'static str, Entity, bool)>);

/// Spawns the entities under test while the first screen loads.
#[derive(Resource)]
struct Setup(fn(&mut Commands) -> Vec<(&'static str, Entity, bool)>);

#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Reflect)]
struct PersistenceScreen;
impl Screen for PersistenceScreen {
    fn builder(mut builder: ScreenScopeBuilder<Self>) -> ScreenScopeBuilder<Self> {
        builder.add_systems(
            ScreenSchedule::Loading,
            |mut commands: Commands,
             setup: Res<Setup>,
             mut expectations: ResMut<Expectations>,
             mut data: ScreenInfoMut<Self>| {
                expectations.0.extend((setup.0)(&mut commands));
                data.finish_loading();
            },
        );
        builder.add_systems(ScreenSchedule::OnReady, |mut commands: Commands| {
            commands.trigger(switch_to_screen::<EmptyScreen>());
        });
        builder.add_systems(ScreenSchedule::OnUnloaded, check_expectations);
        builder
    }
}

fn check_expectations(
    expectations: Res<Expectations>,
    entities: &Entities,
    mut commands: Commands,
) {
    let failures = expectations
        .0
        .iter()
        .filter(|(_, entity, survives)| entities.contains(*entity) != *survives)
        .map(|(label, _, survives)| format!("{label} (expected survives = {survives})"))
        .collect::<Vec<_>>();
    if commands.assert(
        failures.is_empty(),
        format!("Unexpected cleanup result: {failures:?}"),
    ) {
        commands.write_message(AppExit::Success);
    }
}

fn run(setup: fn(&mut Commands) -> Vec<(&'static str, Entity, bool)>, app_setup: fn(&mut App)) {
    let mut app = get_test_app::<PersistenceScreen>();
    app.insert_resource(Setup(setup))
        .init_resource::<Expectations>();
    app_setup(&mut app);
    assert!(app.run().is_success());
}

#[derive(Component)]
struct AnywhereMarker;

#[derive(Component)]
struct TopLevelMarker;

#[test]
fn registered_types_follow_their_scope() {
    run(
        |commands| {
            let parent = commands.spawn_empty().id();
            let anywhere_child = commands.spawn((AnywhereMarker, ChildOf(parent))).id();
            let top_level = commands.spawn(TopLevelMarker).id();
            let parent = commands.spawn_empty().id();
            let top_level_child = commands.spawn((TopLevelMarker, ChildOf(parent))).id();
            let explicitly_scoped = commands.spawn((AnywhereMarker, ScreenScoped)).id();
            let plain = commands.spawn_empty().id();
            vec![
                ("anywhere child", anywhere_child, true),
                ("top-level", top_level, true),
                ("top-level type as child", top_level_child, false),
                (
                    "registered type overrides ScreenScoped",
                    explicitly_scoped,
                    true,
                ),
                ("plain", plain, false),
            ]
        },
        |app| {
            app.register_persistent_type::<AnywhereMarker>(PersistentTypeScope::Anywhere)
                .register_persistent_type::<TopLevelMarker>(PersistentTypeScope::TopLevel);
        },
    );
}

#[test]
fn types_registered_before_screen_plugin_are_kept() {
    let mut app = App::new();
    app.register_persistent_type::<AnywhereMarker>(PersistentTypeScope::Anywhere)
        .add_plugins(ScreenPlugin);
    let id = app.world().component_id::<AnywhereMarker>().unwrap();
    assert!(
        app.world()
            .resource::<PersistentTypes>()
            .anywhere()
            .contains(&id)
    );
}

#[derive(Event)]
struct Ping;

#[test]
fn global_observers_survive_and_other_observers_die_with_their_entity() {
    run(
        |commands| {
            let global = commands.spawn(Observer::new(|_: On<Ping>| {})).id();
            let target = commands.spawn_empty().id();
            // Unparented: Bevy despawns it with its target, not q_screens' query.
            let entity_observer = commands
                .spawn(Observer::new(|_: On<Add, Name>| {}).with_entity(target))
                .id();
            let parent = commands.spawn_empty().id();
            let child_observer = commands
                .spawn((Observer::new(|_: On<Ping>| {}), ChildOf(parent)))
                .id();
            vec![
                ("global observer", global, true),
                (
                    "entity observer (despawned by Bevy)",
                    entity_observer,
                    false,
                ),
                ("child observer (TopLevel scope)", child_observer, false),
            ]
        },
        |_| {},
    );
}

#[derive(Component)]
struct SpawnedAtStartup;

#[test]
fn startup_entities_are_screen_scoped() {
    run(
        |_| vec![],
        |app| {
            app.add_systems(
                Startup,
                |mut commands: Commands, mut expectations: ResMut<Expectations>| {
                    let entity = commands.spawn(SpawnedAtStartup).id();
                    expectations.0.push(("startup entity", entity, false));
                },
            );
        },
    );
}

#[test]
fn unconditional_engine_types_follow_their_scope() {
    run(
        |commands| {
            let monitor = || Monitor {
                name: None,
                physical_height: 0,
                physical_width: 0,
                physical_position: IVec2::ZERO,
                refresh_rate_millihertz: None,
                scale_factor: 1.,
                video_modes: vec![],
            };
            let parent = commands.spawn_empty().id();
            Vec::from([
                ("window", commands.spawn(Window::default()).id(), true),
                (
                    "child window",
                    commands.spawn((Window::default(), ChildOf(parent))).id(),
                    false,
                ),
                ("monitor", commands.spawn(monitor()).id(), true),
                (
                    "child monitor",
                    commands.spawn((monitor(), ChildOf(parent))).id(),
                    true,
                ),
                (
                    "one-shot system",
                    commands.register_system(|| {}).entity(),
                    true,
                ),
            ])
        },
        |_| {},
    );
}
