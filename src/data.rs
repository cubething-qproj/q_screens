use crate::prelude::*;
use bevy::ecs::{
    schedule::ScheduleLabel,
    system::{ReadOnlySystemParam, SystemParam},
};
use std::{any::TypeId, marker::PhantomData};

mod general_api {
    use thiserror::Error;

    use super::*;

    #[allow(missing_docs)]
    #[derive(Error, Debug)]
    pub enum ScreenError {
        #[error("Could not find screen {0}! Did you register it?")]
        NoSuchScreen(String),
        #[error("Could not find screen with ID {0:?}!")]
        NoSuchScreenId(ScreenId),
    }

    /// Call this when you want to switch screens. This will trigger a
    /// [SwitchToScreenMsg] with the screen's [ComponentId].
    #[derive(Event, Debug, PartialEq, Eq, Clone, Deref, Default)]
    pub struct SwitchToScreen<S: Screen>(PhantomData<S>);

    /// See [SwitchToScreen]
    pub fn switch_to_screen<S: Screen>() -> SwitchToScreen<S> {
        SwitchToScreen::<S>::default()
    }

    /// Switches to the given screen by its [ComponentId]. When possible, prefer
    /// to use [SwitchToScreen] to ensure type safety. This is a [Message] so we
    /// can buffer any [SwitchToScreenMsg]s to avoid conflicts. Only the last
    /// valid [SwitchToScreenMsg] will be read.
    #[derive(Message, Debug, PartialEq, Eq, Clone, Deref)]
    pub struct SwitchToScreenMsg(pub ScreenId);

    /// Signals that the current screen has changed.
    #[derive(Event, Debug, PartialEq, Eq, Clone)]
    pub struct ScreenChanged {
        #[allow(missing_docs)]
        pub from: Option<ScreenId>,
        #[allow(missing_docs)]
        pub to: ScreenId,
    }

    /// Will cause the given screen to finish loading. Has no effect if the
    /// screen is not currently loading.
    #[derive(Event, Debug, PartialEq, Eq, Clone, Deref, Default)]
    pub struct FinishLoading<S: Screen>(PhantomData<S>);

    /// See [FinishLoading]
    pub fn finish_loading<S: Screen>() -> FinishLoading<S> {
        FinishLoading::<S>::default()
    }

    /// Will cause the given screen to finish unloading. Has no effect if the
    /// screen is not currently unloading.
    #[derive(Event, Debug, PartialEq, Eq, Clone, Deref, Default)]
    pub struct FinishUnloading<S: Screen>(PhantomData<S>);

    /// See [FinishUnloading]
    pub fn finish_unloading<S: Screen>() -> FinishUnloading<S> {
        FinishUnloading::<S>::default()
    }

    /// Scopes an entity to the current screen. The entity will be cleaned up when
    /// the [Screen] state changes. By default, all entities _except_ top-level
    /// [Observer] and [Window] components are screen-scoped.
    ///
    /// Note: This is effectively used to skip the propagation of the
    /// [Persistent] component. Since screen scoping is the default behavior, it
    /// should not be necessary to add this component in other cases.
    #[derive(Component, Debug, Reflect, Clone, Copy, Default, PartialEq)]
    pub struct ScreenScoped;

    /// Marks an entity as screen-persistent, i.e., this entity will _not_ be
    /// automatically cleaned up when the screen changes. By default, all entites
    /// _except_ top-level [Observer] and [Window] components and are screen-scoped.
    ///
    /// In order to mark the children of this component as Persistent, you should
    /// use the [Propagate](bevy::app::Propagate) component.
    #[derive(Component, Debug, Reflect, Clone, Copy, Default, PartialEq)]
    pub struct Persistent;

    /// The first screen. Typically this will be a splash screen, a loading
    /// screen, or a main menu.
    #[derive(Resource, Default, Debug, Deref)]
    pub struct InitialScreen(Option<String>);
    impl InitialScreen {
        #[allow(missing_docs)]
        pub fn new<S: Screen>() -> Self {
            Self(Some(S::name()))
        }
        #[allow(missing_docs)]
        pub fn from_name(name: String) -> Self {
            Self(Some(name))
        }
    }
}
pub use general_api::*;

mod screens {
    use bevy::{ecs::change_detection::Tick, utils::TypeIdMap};

    use super::*;

    /// Generates [`ScreenId`]s.
    #[derive(Resource, Debug, Default)]
    pub(crate) struct ScreenIds {
        next: bevy::platform::sync::atomic::AtomicUsize,
    }

    impl ScreenIds {
        pub fn next(&self) -> ScreenId {
            ScreenId(
                self.next
                    .fetch_add(1, bevy::platform::sync::atomic::Ordering::Relaxed),
            )
        }
    }
    /// The screen's ID.
    #[derive(Clone, Debug, PartialEq, Eq, Deref, Copy, Reflect)]
    pub struct ScreenId(pub(crate) usize);

    /// The screen registry holds a map between the screen's type id and it's [ScreenId].
    #[derive(Resource, Debug, Deref, DerefMut, Default)]
    pub struct ScreenRegistry(TypeIdMap<ScreenId>);
    impl ScreenRegistry {
        #[allow(missing_docs)]
        pub fn get(&self, id: &TypeId) -> Result<ScreenId, ScreenError> {
            self.0
                .get(id)
                .copied()
                .ok_or(ScreenError::NoSuchScreen(format!("{:?}", id)))
        }
    }

    /// Efficiently accessible vec of screen data.
    /// Do not use this directly. Instead prefer to use [ScreenDataRef] or [ScreenDataMut]
    #[derive(Resource, Debug, Deref, DerefMut, Default)]
    pub struct ScreenData(Vec<Option<ScreenInfo>>);
    impl ScreenData {
        #[allow(missing_docs)]
        pub fn get(&self, id: ScreenId) -> Result<&ScreenInfo, ScreenError> {
            self.0
                .get(*id)
                .and_then(|v| v.as_ref())
                .ok_or(ScreenError::NoSuchScreenId(id))
        }
        #[allow(missing_docs)]
        pub fn get_mut(&mut self, id: ScreenId) -> Result<&mut ScreenInfo, ScreenError> {
            self.0
                .get_mut(*id)
                .and_then(|v| v.as_mut())
                .ok_or(ScreenError::NoSuchScreenId(id))
        }
        #[allow(missing_docs)]
        pub fn iter_some(&self) -> impl Iterator<Item = &ScreenInfo> {
            self.0.iter().filter_map(|v| v.as_ref())
        }
        #[allow(missing_docs)]
        pub fn iter_some_mut(&mut self) -> impl Iterator<Item = &mut ScreenInfo> {
            self.0.iter_mut().filter_map(|v| v.as_mut())
        }
    }

    /// The current screen's ID.
    #[derive(Resource, Debug, Deref, DerefMut, Default)]
    pub struct CurrentScreen(Option<ScreenId>);
    impl CurrentScreen {
        /// Gets the [ScreenId] for the given [Screen].
        /// This will usually be populated, unless you have yet to switch to any screen.
        pub fn get_id(&self) -> Option<ScreenId> {
            self.0
        }
    }

    /// Data about a given screen. This is where all the screen's identifying information lives, including it's [ScreenState].
    #[derive(Debug)]
    pub struct ScreenInfo {
        /// Serialized name of the [Screen]
        name: String,
        state: ScreenState,
        /// [TypeId] of the underlying [Screen] component
        type_id: TypeId,
        /// [ScreenId] of the underlying [Screen] component
        screen_id: ScreenId,
        /// Indicates that the state has changed and needs to run the corresponding state schedule.
        pub(crate) needs_update: bool,
        pub(crate) changed_at: Tick,
        pub(crate) initialized: bool,
        /// Should the Update schedule run even while loading?
        load_strategy: LoadStrategy,
        /// Initialize directly into Ready.
        skip_load: bool,
        /// Deinitialize immediately
        skip_unload: bool,
    }
    impl ScreenInfo {
        #[allow(missing_docs)]
        pub fn new<S: Screen>(screen_id: ScreenId, tick: Tick) -> Self {
            Self {
                name: S::name(),
                state: ScreenState::Unloaded,
                type_id: TypeId::of::<S>(),
                needs_update: true,
                skip_load: true,
                skip_unload: true,
                load_strategy: LoadStrategy::Blocking,
                changed_at: tick,
                initialized: false,
                screen_id,
            }
        }

        pub(crate) fn load(&mut self, tick: Tick) {
            if matches!(
                self.state,
                ScreenState::Unloaded | ScreenState::Unloading | ScreenState::LoadQueued
            ) {
                if self.skip_load {
                    self.state = ScreenState::Ready
                } else {
                    self.state = ScreenState::Loading;
                }
                self.needs_update = true;
                self.changed_at = tick;
            }
        }

        /// Unloads the screen.
        /// Has no effect if already in Unloading or Unloaded states.
        pub fn unload(&mut self, tick: Tick) {
            if matches!(self.state, ScreenState::Loading | ScreenState::Ready) {
                if self.skip_unload {
                    self.state = ScreenState::Cleanup;
                } else {
                    self.state = ScreenState::Unloading;
                }
                self.needs_update = true;
                self.changed_at = tick;
            }
        }
        /// Queues the screen to load, unloading any other screens and waiting
        /// until their cleanup is complete.
        /// Only works if in the `Unloaded` state.
        pub fn queue_load(&mut self, tick: Tick) {
            if matches!(self.state, ScreenState::Unloaded) {
                self.state = ScreenState::LoadQueued;
                self.needs_update = true;
                self.changed_at = tick;
            }
        }
        /// Finishes loading the screen.
        /// Has no effect if already in Loading or Ready states.
        pub fn finish_loading(&mut self, tick: Tick) {
            if matches!(self.state, ScreenState::Loading) {
                self.state = ScreenState::Ready;
                self.needs_update = true;
                self.changed_at = tick;
            }
        }
        /// Finishes loading the screen.
        /// Has no effect if already in Loading or Ready states.
        pub fn finish_unloading(&mut self, tick: Tick) {
            if matches!(self.state, ScreenState::Unloading) {
                self.state = ScreenState::Cleanup;
                self.needs_update = true;
                self.changed_at = tick;
            }
        }

        pub(crate) fn finish_cleanup(&mut self, tick: Tick) {
            if matches!(self.state, ScreenState::Cleanup) {
                self.state = ScreenState::Unloaded;
                self.needs_update = true;
                self.changed_at = tick;
            }
        }

        #[allow(missing_docs)]
        pub fn load_strategy(&self) -> LoadStrategy {
            self.load_strategy
        }

        #[allow(missing_docs)]
        pub fn skip_load(&self) -> bool {
            self.skip_load
        }

        #[allow(missing_docs)]
        pub fn skip_unload(&self) -> bool {
            self.skip_unload
        }

        #[allow(missing_docs)]
        pub fn set_skip_unload(&mut self, skip_unload: bool) {
            self.skip_unload = skip_unload;
        }

        #[allow(missing_docs)]
        pub fn set_skip_load(&mut self, skip_load: bool) {
            self.skip_load = skip_load;
        }

        #[allow(missing_docs)]
        pub fn set_load_strategy(&mut self, load_strategy: LoadStrategy) {
            self.load_strategy = load_strategy;
        }

        #[allow(missing_docs)]
        pub fn initialized(&self) -> bool {
            self.initialized
        }

        #[allow(missing_docs)]
        pub fn changed_at(&self) -> Tick {
            self.changed_at
        }

        #[allow(missing_docs)]
        pub fn needs_update(&self) -> bool {
            self.needs_update
        }

        #[allow(missing_docs)]
        pub fn type_id(&self) -> TypeId {
            self.type_id
        }

        #[allow(missing_docs)]
        pub fn state(&self) -> ScreenState {
            self.state
        }

        #[allow(missing_docs)]
        pub fn name(&self) -> &str {
            &self.name
        }

        #[allow(missing_docs)]
        pub fn screen_id(&self) -> ScreenId {
            self.screen_id
        }
    }
}
pub use screens::*;

mod schedules {
    use super::*;
    /// Describes a screen's [Schedule]. All systems added to this schedule
    /// will be scoped to this screen's lifetime.
    /// To use as a schedule, wrap it with [ScreenScheduleLabel].
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter)]
    pub enum ScreenSchedule {
        /// Runs on [Update] when the screen has [ScreenState::Ready]
        Update,
        /// Runs on [FixedUpdate] when the screen has [ScreenState::Ready]
        FixedUpdate,
        /// Runs on [Update] when the screen has [ScreenState::Loading]
        Loading,
        /// Runs on [Update] when the screen has [ScreenState::Unloading]
        Unloading,
        /// For internal use! Runs after [ScreenState::Unloading]. Used to clean up screen-scoped entities.
        Cleanup,
        /// Can also be specified as [on_screen_load]
        OnLoad,
        /// Can also be specified as [on_screen_ready]
        OnReady,
        /// Can also be specified as [on_screen_unload]
        OnUnload,
        /// Can also be specified as [on_screen_unloaded]
        OnUnloaded,
    }

    // TODO: This should use ScreenId internally.
    /// Wrapper around [ScreenSchedule]. Needed to make schedules unique per type.
    #[derive(ScheduleLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ScreenScheduleLabel {
        id: TypeId,
        kind: ScreenSchedule,
    }
    impl ScreenScheduleLabel {
        #[allow(missing_docs)]
        pub fn new<S: Screen>(kind: ScreenSchedule) -> Self {
            Self {
                id: TypeId::of::<S>(),
                kind,
            }
        }
        #[allow(missing_docs)]
        pub fn from_id(kind: ScreenSchedule, id: TypeId) -> Self {
            Self { id, kind }
        }
    }
}
pub use schedules::*;

/// Describes the current state of a screen. Not an actual [State].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ScreenState {
    #[default]
    /// The screen is currently down.
    Unloaded,
    /// The screen is waiting for cleanup to finish.
    LoadQueued,
    /// The screen is running any custom load systems.
    Loading,
    /// The screen is fully loaded and ready to execute its main systems.
    Ready,
    /// The screen is running any custom unload systems.
    Unloading,
    /// For internal use. Cleaning up screen-scoped entities.
    Cleanup,
}

mod system_params {
    use bevy::ecs::change_detection::Tick;

    use super::*;

    /// Read-only [SystemParam] for easy access to a screen's [ScreenInfo]
    pub struct ScreenInfoRef<'w, S: Screen> {
        data: &'w ScreenInfo,
        _ghost: PhantomData<S>,
    }
    impl<'w, S: Screen> ScreenInfoRef<'w, S> {
        #[allow(missing_docs)]
        pub fn data(&self) -> &'w ScreenInfo {
            self.data
        }
    }

    unsafe impl<'w, S: Screen> SystemParam for ScreenInfoRef<'w, S> {
        type State = ();
        type Item<'world, 'state> = ScreenInfoRef<'world, S>;

        fn init_state(_world: &mut World) -> Self::State {}

        fn init_access(
            _state: &Self::State,
            _system_meta: &mut bevy::ecs::system::SystemMeta,
            component_access_set: &mut bevy::ecs::query::FilteredAccessSet,
            world: &mut World,
        ) {
            component_access_set
                .add_unfiltered_resource_read(world.resource_id::<ScreenRegistry>().unwrap());
        }

        unsafe fn get_param<'world, 'state>(
            _state: &'state mut Self::State,
            _system_meta: &bevy::ecs::system::SystemMeta,
            world: bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell<'world>,
            _change_tick: bevy::ecs::change_detection::Tick,
        ) -> Self::Item<'world, 'state> {
            let registry = unsafe { world.get_resource::<ScreenRegistry>().unwrap() };
            let idx = registry.get(&TypeId::of::<S>()).unwrap();
            let data_res = unsafe { world.get_resource::<ScreenData>().unwrap() };
            let data = data_res
                .get(idx)
                .map_err(|_| ScreenError::NoSuchScreen(S::name()))
                .unwrap();
            ScreenInfoRef {
                _ghost: PhantomData,
                data,
            }
        }
    }
    unsafe impl<'w, S: Screen> ReadOnlySystemParam for ScreenInfoRef<'w, S> {}

    /// [SystemParam] for easy mutable access to the given screen's data.
    /// All functionality happens through helper functions for API sanity.
    pub struct ScreenInfoMut<'w, S: Screen> {
        _ghost: PhantomData<S>,
        data: Mut<'w, ScreenInfo>,
        change_tick: Tick,
    }
    impl<'w, S: Screen> ScreenInfoMut<'w, S> {
        /// Loads the screen. Has no effect if the screen is already Loaded or Ready.
        pub fn load(&mut self) {
            let tick = self.change_tick;
            self.data.load(tick);
        }
        /// Unloads the screen. Has no effect if the screen is already Loaded or Ready.
        pub fn unload(&mut self) {
            let tick = self.change_tick;
            self.data.unload(tick);
        }
        /// Loads the screen. Has no effect if the screen is not Loading.
        pub fn finish_loading(&mut self) {
            let tick = self.change_tick;
            self.data.finish_loading(tick);
        }
        /// Loads the screen. Has no effect if the screen is not Loading.
        pub fn finish_unloading(&mut self) {
            let tick = self.change_tick;
            self.data.finish_unloading(tick);
        }
        pub(crate) fn finish_cleanup(&mut self) {
            let tick = self.change_tick;
            self.data.finish_cleanup(tick);
        }
        #[allow(missing_docs)]
        pub fn data(&self) -> &ScreenInfo {
            self.data.as_ref()
        }
    }
    unsafe impl<'w, S: Screen> SystemParam for ScreenInfoMut<'w, S> {
        type State = ();
        type Item<'world, 'state> = ScreenInfoMut<'world, S>;

        fn init_state(_world: &mut World) -> Self::State {}

        fn init_access(
            _state: &Self::State,
            _system_meta: &mut bevy::ecs::system::SystemMeta,
            component_access_set: &mut bevy::ecs::query::FilteredAccessSet,
            world: &mut World,
        ) {
            component_access_set
                .add_unfiltered_resource_write(world.resource_id::<ScreenRegistry>().unwrap());
        }

        unsafe fn get_param<'world, 'state>(
            _state: &'state mut Self::State,
            _system_meta: &bevy::ecs::system::SystemMeta,
            world: bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell<'world>,
            change_tick: bevy::ecs::change_detection::Tick,
        ) -> Self::Item<'world, 'state> {
            let registry = unsafe { world.get_resource_mut::<ScreenRegistry>().unwrap() };
            let data_res = unsafe { world.get_resource_mut::<ScreenData>().unwrap() };
            let screen_id = registry
                .get(&TypeId::of::<S>())
                .map_err(|_| ScreenError::NoSuchScreen(S::name()))
                .unwrap();
            let data = data_res.map_unchanged(|res| res.get_mut(screen_id).unwrap());
            Self::Item {
                data,
                _ghost: PhantomData,
                change_tick,
            }
        }
    }

    /// Gets the [ScreenId] for the given [Screen]
    #[derive(Debug, Copy, Clone, Deref)]
    pub struct ScreenIdFor<S: Screen> {
        #[deref]
        id: ScreenId,
        _ghost: PhantomData<S>,
    }
    unsafe impl<S: Screen> SystemParam for ScreenIdFor<S> {
        type Item<'world, 'state> = ScreenIdFor<S>;
        type State = ();

        fn init_state(_: &mut World) -> Self::State {}

        fn init_access(
            _state: &Self::State,
            _system_meta: &mut bevy::ecs::system::SystemMeta,
            _component_access_set: &mut bevy::ecs::query::FilteredAccessSet,
            _world: &mut World,
        ) {
        }

        unsafe fn get_param<'world, 'state>(
            _state: &'state mut Self::State,
            _system_meta: &bevy::ecs::system::SystemMeta,
            world: bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell<'world>,
            _change_tick: Tick,
        ) -> Self::Item<'world, 'state> {
            let registry = unsafe { world.get_resource::<ScreenRegistry>().unwrap() };
            let id = registry.get(&TypeId::of::<S>()).unwrap();
            Self {
                id,
                _ghost: PhantomData,
            }
        }
    }
    unsafe impl<S: Screen> ReadOnlySystemParam for ScreenIdFor<S> {}

    /// [SystemParam] for easy access to all screen info.
    #[derive(SystemParam)]
    pub struct Screens<'w> {
        registry: Res<'w, ScreenRegistry>,
        data: Res<'w, ScreenData>,
    }
    impl<'w> Screens<'w> {
        /// Gets the [ScreenInfo] for the first [Screen] with a matching name.
        /// Note that screen name uniqueness is not enforced.
        pub fn get_by_name(&self, name: &str) -> Result<&ScreenInfo, ScreenError> {
            self.data
                .iter_some()
                .find(|v| v.name() == name)
                .ok_or(ScreenError::NoSuchScreen(name.into()))
        }
        /// Gets the [ScreenInfo] for the [Screen] with the corresponding [TypeId]
        pub fn get_by_type_id(&self, id: &TypeId) -> Result<&ScreenInfo, ScreenError> {
            self.registry.get(id).and_then(|id| self.data.get(id))
        }
        /// Gets the [ScreenInfo] for the [Screen] with the corresponding [TypeId]
        pub fn get_by_id(&self, id: ScreenId) -> Result<&ScreenInfo, ScreenError> {
            self.data.get(id)
        }
        /// Gets the [ScreenInfo] for this [Screen]. Alternative to calling [ScreenInfoRef]
        pub fn get<S: Screen>(&self) -> Result<&ScreenInfo, ScreenError> {
            self.registry
                .get(&TypeId::of::<S>())
                .and_then(|id| self.data.get(id))
        }
    }
    /// [SystemParam] for easy mutable access to all screen info.
    #[derive(SystemParam)]
    pub struct ScreensMut<'w> {
        registry: ResMut<'w, ScreenRegistry>,
        data: ResMut<'w, ScreenData>,
    }
    impl<'w> ScreensMut<'w> {
        /// Gets the [ScreenInfo] for the first [Screen] with a matching name.
        /// Note that screen name uniqueness is not enforced.
        pub fn get_by_name(&self, name: &str) -> Result<&ScreenInfo, ScreenError> {
            self.data
                .iter_some()
                .find(|v| v.name() == name)
                .ok_or(ScreenError::NoSuchScreen(name.into()))
        }
        /// Gets the [ScreenInfo] for the [Screen] with the corresponding [TypeId]
        pub fn get_by_type_id(&self, id: &TypeId) -> Result<&ScreenInfo, ScreenError> {
            self.registry.get(id).and_then(|id| self.data.get(id))
        }
        /// Gets the [ScreenInfo] for the [Screen] with the corresponding [TypeId]
        pub fn get_by_id(&self, id: ScreenId) -> Result<&ScreenInfo, ScreenError> {
            self.data.get(id)
        }
        /// Gets the [ScreenInfo] for this [Screen]. Alternative to calling [ScreenInfoRef]
        pub fn get<S: Screen>(&self) -> Result<&ScreenInfo, ScreenError> {
            self.registry
                .get(&TypeId::of::<S>())
                .and_then(|id| self.data.get(id))
        }
        /// Mutably gets the [ScreenInfo] for the first [Screen] with a matching name.
        /// Note that screen name uniqueness is not enforced.
        pub fn get_by_name_mut(&mut self, name: &str) -> Result<&mut ScreenInfo, ScreenError> {
            self.data
                .iter_some_mut()
                .find(|v| v.name() == name)
                .ok_or(ScreenError::NoSuchScreen(name.into()))
        }
        /// Mutably gets the [ScreenInfo] for the [Screen] with the corresponding [TypeId].
        pub fn get_by_type_id_mut(&mut self, id: &TypeId) -> Result<&mut ScreenInfo, ScreenError> {
            self.registry.get(id).and_then(|id| self.data.get_mut(id))
        }
        /// Mutably gets the [ScreenInfo] for this [Screen]. Alternative to calling [ScreenInfoRef]
        pub fn get_mut<S: Screen>(&mut self) -> Result<&mut ScreenInfo, ScreenError> {
            self.registry
                .get(&TypeId::of::<S>())
                .and_then(|id| self.data.get_mut(id))
        }
        /// Gets the [ScreenInfo] for the [Screen] with the corresponding [TypeId]
        pub fn get_by_id_mut(&mut self, id: ScreenId) -> Result<&mut ScreenInfo, ScreenError> {
            self.data.get_mut(id)
        }
    }
}

pub use system_params::*;

mod helpers {
    pub use super::*;

    /// Condition, like [in_state], but for screens.
    pub fn screen_has_state<S: Screen>(
        state: ScreenState,
    ) -> impl FnMut(ScreenInfoRef<S>) -> bool + Clone {
        move |data: ScreenInfoRef<S>| data.data().state() == state
    }
    /// Is the screen still loading?
    pub fn screen_loading<S: Screen>() -> impl FnMut(ScreenInfoRef<S>) -> bool + Clone {
        |data: ScreenInfoRef<S>| matches!(data.data().state(), ScreenState::Loading)
    }
    /// Has the screen finished loading?
    pub fn screen_ready<S: Screen>() -> impl FnMut(ScreenInfoRef<S>) -> bool + Clone {
        |data: ScreenInfoRef<S>| matches!(data.data().state(), ScreenState::Ready)
    }
    /// Is the screen currently unloading?
    pub fn screen_unloading<S: Screen>() -> impl FnMut(ScreenInfoRef<S>) -> bool + Clone {
        |data: ScreenInfoRef<S>| matches!(data.data().state(), ScreenState::Unloading)
    }
    /// Has the screen finished unloading?
    pub fn screen_unloaded<S: Screen>() -> impl FnMut(ScreenInfoRef<S>) -> bool + Clone {
        |data: ScreenInfoRef<S>| matches!(data.data().state(), ScreenState::Unloaded)
    }

    /// Label of a schedule which fires when the screen has begun to load.
    #[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone, Copy)]
    pub struct OnScreenLoad(pub TypeId);

    /// See [OnScreenLoad]
    pub fn on_screen_load<S: Screen>() -> impl ScheduleLabel {
        OnScreenLoad(TypeId::of::<S>())
    }

    /// Label of a schedule which fires when the screen has begun its cleanup schedule.
    /// Try to avoid spawning anything during this schedule.
    #[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone, Copy)]
    pub struct OnScreenCleanup(pub TypeId);
    /// See [OnScreenCleanup]
    pub fn on_screen_cleanup<S: Screen>() -> impl ScheduleLabel {
        OnScreenCleanup(TypeId::of::<S>())
    }
    /// Label of a schedule which fires when the screen is waiting to load.
    #[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone, Copy)]
    pub struct OnScreenLoadQueued(pub TypeId);
    /// See [OnScreenLoadQueued]
    pub fn on_screen_load_queued<S: Screen>() -> impl ScheduleLabel {
        OnScreenLoadQueued(TypeId::of::<S>())
    }

    /// Label of a schedule which fires when the screen has finished loading.
    #[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone, Copy)]
    pub struct OnScreenReady(pub TypeId);

    /// See [OnScreenReady]
    pub fn on_screen_ready<S: Screen>() -> impl ScheduleLabel {
        OnScreenReady(TypeId::of::<S>())
    }

    /// Label of a schedule which fires when the screen is beginning to unload. Not to be confused with [OnScreenUnloaded].
    #[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone, Copy)]
    pub struct OnScreenUnload(pub TypeId);

    /// See [OnScreenUnload]
    pub fn on_screen_unload<S: Screen>() -> impl ScheduleLabel {
        OnScreenUnload(TypeId::of::<S>())
    }

    /// Label of a schedule which fires when the screen has finished unloading and is no longer active.
    #[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone, Copy)]
    pub struct OnScreenUnloaded(pub TypeId);

    /// See [OnScreenUnloaded]
    pub fn on_screen_unloaded<S: Screen>() -> impl ScheduleLabel {
        OnScreenUnloaded(TypeId::of::<S>())
    }
}
pub use helpers::*;
