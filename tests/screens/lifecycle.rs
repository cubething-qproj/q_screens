use crate::prelude::*;

macro_rules! gen_fns {
    ($($name:ident),*) => {
        $(
            fn $name(mut r: ResMut<LifecycleStatus>) {
                r.$name = true;
            }
        )*
        #[derive(Resource, Debug, Default)]
        pub struct LifecycleStatus {
            $(pub $name:bool,)*
        }
        impl LifecycleStatus {
            pub fn ok(&self) -> bool {
                $(self.$name) &&*
            }
        }
    }
}
gen_fns!(
    loading,
    update,
    fixed_update,
    unloading,
    load,
    ready,
    unload,
    unloaded
);

macro_rules! gen_test_fns {
    ($($name:ident),*) => {
        #[derive(Resource, Default, Debug)]
        struct TestRes {
            $($name: bool,)*
        }
        impl TestRes {
            pub fn ok(&self) -> bool {
                $(self.$name) &&*
            }
        }
    }
}
macro_rules! impl_test_fns {
    ($app:expr, $($name:ident),*) => {
        $(
            $app.add_systems(
                $name::<LifecycleScreen>(),
                |mut r: ResMut<TestRes>| {r.$name = true;}
            );
        )*
    }
}

gen_test_fns!(
    on_screen_load,
    on_screen_ready,
    on_screen_unloaded,
    on_screen_cleanup
);

macro_rules! progress_by {
    ($name:ident) => {
        |mut data: ScreenInfoMut<LifecycleScreen>| {
            data.$name();
        }
    };
}

/// The main [Screen] implementation.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Reflect)]
pub struct LifecycleScreen;
impl Screen for LifecycleScreen {
    fn builder(mut builder: ScreenScopeBuilder<Self>) -> ScreenScopeBuilder<Self> {
        builder
            .add_systems(
                ScreenSchedule::Loading,
                (
                    loading,
                    progress_by!(finish_loading),
                    |current_screen: Res<CurrentScreen>,
                     id: ScreenIdFor<LifecycleScreen>,
                     screens: Screens,
                     mut commands: Commands| {
                        if current_screen.get_id() != Some(*id) {
                            let s = screens.get_by_id(*id).unwrap();
                            error!("while loading, current screen was {}", s.name());
                            commands.write_message(AppExit::error());
                        }
                    },
                ),
            )
            // progress to unload by loading in EmptyScreen
            .add_systems(
                ScreenSchedule::Update,
                (update, |r: Res<LifecycleStatus>, mut commands: Commands| {
                    if r.fixed_update && r.update {
                        commands.trigger(switch_to_screen::<EmptyScreen>());
                    }
                }),
            )
            .add_systems(ScreenSchedule::FixedUpdate, fixed_update)
            .add_systems(
                ScreenSchedule::Unloading,
                (unloading, progress_by!(finish_unloading)),
            )
            .add_systems(ScreenSchedule::OnLoad, load)
            .add_systems(ScreenSchedule::OnReady, ready)
            .add_systems(ScreenSchedule::OnUnload, unload)
            .add_systems(
                ScreenSchedule::OnUnloaded,
                (
                    unloaded,
                    |current_screen: Res<CurrentScreen>,
                     id: ScreenIdFor<Self>,
                     mut commands: Commands| {
                        if current_screen.get_id() == Some(*id) {
                            error!("Found LifecycleScreen in component hierarchy after unload!");
                            commands.write_message(AppExit::error());
                        }
                    },
                )
                    .chain(),
            );
        builder
    }
}

type Scr = LifecycleScreen;
#[test]
fn lifecycle() {
    let mut app = get_test_app::<Scr>();
    app.register_screen::<EmptyScreen>();
    app.init_resource::<LifecycleStatus>();
    app.init_resource::<TestRes>();
    impl_test_fns!(
        app,
        on_screen_load,
        on_screen_ready,
        on_screen_unloaded,
        on_screen_cleanup
    );
    app.add_systems(
        on_screen_ready::<EmptyScreen>(),
        |r: Res<LifecycleStatus>, r2: Res<TestRes>, mut commands: Commands| {
            let ok = r.ok() && r2.ok();
            if ok {
                info!("OK!");
                commands.write_message(AppExit::Success);
            } else {
                error!("Did not reach all expected points.");
                error!(?r);
                error!(?r2);
                commands.write_message(AppExit::error());
            }
        },
    );
    assert!(app.run().is_success());
}
