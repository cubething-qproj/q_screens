//! Ready initialization must finish before updates in the full app lifecycle.

use std::time::Duration;

use bevy::{prelude::*, time::TimeUpdateStrategy};
use q_screens::prelude::*;
use q_test_harness::prelude::*;

#[derive(Component)]
struct ReadyEntity;

#[derive(Resource, Default)]
struct Lifecycle {
    entities: Vec<Entity>,
    ready_updates: [usize; 2],
    loading_updates: [usize; 2],
    finish_loading: bool,
}

#[derive(Component, Default, Reflect)]
struct ReadyScreen<const SKIP: bool, const NONBLOCKING: bool>;

impl<const SKIP: bool, const NONBLOCKING: bool> Screen for ReadyScreen<SKIP, NONBLOCKING> {
    fn builder(mut builder: ScreenScopeBuilder<Self>) -> ScreenScopeBuilder<Self> {
        builder
            .with_skip_load(SKIP)
            .with_skip_unload(true)
            .with_load_strategy(if NONBLOCKING {
                LoadStrategy::Nonblocking
            } else {
                LoadStrategy::Blocking
            })
            .add_systems(ScreenSchedule::Loading, Self::loading)
            .add_systems(ScreenSchedule::OnReady, Self::ready)
            .add_systems(ScreenSchedule::Update, Self::update::<0>)
            .add_systems(ScreenSchedule::FixedUpdate, Self::update::<1>);
        builder
    }
}

impl<const SKIP: bool, const NONBLOCKING: bool> ReadyScreen<SKIP, NONBLOCKING> {
    fn loading(mut lifecycle: ResMut<Lifecycle>, mut commands: Commands) {
        if std::mem::take(&mut lifecycle.finish_loading) {
            commands.trigger(finish_loading::<Self>());
        }
    }

    fn ready(mut commands: Commands, mut lifecycle: ResMut<Lifecycle>) {
        lifecycle.entities.push(commands.spawn(ReadyEntity).id());
    }

    fn update<const INDEX: usize>(
        info: ScreenInfoRef<Self>,
        entities: Query<Entity, With<ReadyEntity>>,
        mut lifecycle: ResMut<Lifecycle>,
    ) {
        match info.data().state() {
            ScreenState::Ready => {
                assert_eq!(
                    entities.single().expect("OnReady Commands must be applied"),
                    *lifecycle.entities.last().expect("OnReady must run first")
                );
                lifecycle.ready_updates[INDEX] += 1;
            }
            ScreenState::Loading => {
                assert!(NONBLOCKING);
                assert!(entities.is_empty());
                lifecycle.loading_updates[INDEX] += 1;
            }
            state => panic!("Unexpected update state: {state:?}"),
        }
    }
}

#[derive(Component, Default, Reflect)]
struct AwayScreen;

impl Screen for AwayScreen {
    fn builder(builder: ScreenScopeBuilder<Self>) -> ScreenScopeBuilder<Self> {
        builder
    }
}

struct Fixture<const SKIP: bool, const NONBLOCKING: bool> {
    app: App,
}

impl<const SKIP: bool, const NONBLOCKING: bool> Fixture<SKIP, NONBLOCKING> {
    fn new() -> Self {
        let mut app = App::new();
        app.add_plugins((TestRunnerPlugin::default(), ScreenPlugin))
            .init_resource::<Lifecycle>()
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                10,
            )))
            .insert_resource(Time::<Fixed>::from_duration(Duration::from_millis(10)))
            .insert_resource(InitialScreen::new::<ReadyScreen<SKIP, NONBLOCKING>>())
            .register_screen::<ReadyScreen<SKIP, NONBLOCKING>>()
            .register_screen::<AwayScreen>();
        Self { app }
    }

    fn frame(&mut self) {
        self.app.update();
    }

    fn state(&self) -> ScreenState {
        self.app
            .world()
            .resource::<ScreenData>()
            .iter_some()
            .find(|info| info.type_id() == std::any::TypeId::of::<ReadyScreen<SKIP, NONBLOCKING>>())
            .unwrap()
            .state()
    }

    fn reach(&mut self, state: ScreenState) {
        for _ in 0..10 {
            if self.state() == state {
                return;
            }
            self.frame();
        }
        panic!("Did not reach {state:?}; at {:?}", self.state());
    }

    fn lifecycle(&self) -> &Lifecycle {
        self.app.world().resource::<Lifecycle>()
    }

    fn finish_loading(&mut self) {
        let entries = self.lifecycle().entities.len();
        self.reach(ScreenState::Loading);
        self.frame();
        assert_eq!(self.lifecycle().entities.len(), entries);
        if NONBLOCKING {
            assert!(
                self.lifecycle()
                    .loading_updates
                    .iter()
                    .all(|&count| count > 0)
            );
        } else {
            assert_eq!(self.lifecycle().loading_updates, [0, 0]);
        }
        // Finish inside Loading, before the already queued nonblocking Update.
        self.app
            .world_mut()
            .resource_mut::<Lifecycle>()
            .finish_loading = true;
        self.frame();
        assert_eq!(self.state(), ScreenState::Ready);
    }

    fn assert_entry(&mut self, entry: usize) {
        let before = self.lifecycle().ready_updates;
        self.frame();
        assert!(
            self.lifecycle()
                .ready_updates
                .iter()
                .zip(before)
                .all(|(&after, before)| after > before)
        );
        assert_eq!(self.lifecycle().entities.len(), entry);
        self.frame();
        assert_eq!(
            self.lifecycle().entities.len(),
            entry,
            "OnReady runs only once"
        );
    }

    fn reenter(&mut self) {
        let old_entity = *self.lifecycle().entities.last().unwrap();
        self.app
            .world_mut()
            .trigger(switch_to_screen::<AwayScreen>());
        self.reach(ScreenState::Unloaded);
        assert!(self.app.world().get_entity(old_entity).is_err());
        self.app
            .world_mut()
            .trigger(switch_to_screen::<ReadyScreen<SKIP, NONBLOCKING>>());
        if SKIP {
            self.reach(ScreenState::Ready);
        } else {
            self.finish_loading();
        }
        self.assert_entry(2);
    }
}

#[test]
fn skipped_loading_initializes_before_fixed_update_and_reentry() {
    let mut fixture = Fixture::<true, false>::new();
    fixture.reach(ScreenState::Ready);
    assert!(fixture.lifecycle().entities.is_empty());
    fixture.assert_entry(1);
    fixture.reenter();
}

#[test]
fn skipped_loading_initializes_before_update_without_a_fixed_tick() {
    let mut fixture = Fixture::<true, false>::new();
    fixture
        .app
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    fixture.reach(ScreenState::Ready);
    fixture.frame();
    assert_eq!(fixture.lifecycle().ready_updates, [1, 0]);
    assert_eq!(fixture.lifecycle().entities.len(), 1);
    fixture
        .app
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            10,
        )));
    fixture.assert_entry(1);
}

#[test]
fn blocking_finish_loading_initializes_before_fixed_update_and_reentry() {
    let mut fixture = Fixture::<false, false>::new();
    fixture.finish_loading();
    assert!(fixture.lifecycle().entities.is_empty());
    fixture.assert_entry(1);
    fixture.reenter();
}

#[test]
fn nonblocking_finish_loading_initializes_before_queued_update_and_reentry() {
    let mut fixture = Fixture::<false, true>::new();
    fixture.finish_loading();
    assert_eq!(fixture.lifecycle().ready_updates, [1, 0]);
    assert_eq!(fixture.lifecycle().entities.len(), 1);
    fixture.assert_entry(1);
    fixture.reenter();
}
