use strum::IntoEnumIterator;

use crate::prelude::*;

#[derive(Component, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Reflect, Default, Copy, Clone)]
struct ConditionalsScreen;

#[derive(Resource, Default)]
struct Conditionals {
    screen_has_state: bool,
    screen_loading: bool,
    screen_ready: bool,
    screen_unloading: bool,
    screen_unloaded: bool,
}

macro_rules! add_system {
    ($builder:expr, $schedule:expr, $event:expr) => {
        $builder.add_systems(
            $schedule,
            move |mut count: Local<u8>, mut commands: Commands| {
                *count += 1;
                info!("{:?} {}", $schedule, *count);
                if *count > 100 {
                    commands.trigger($event)
                }
            },
        );
    };
}

impl Screen for ConditionalsScreen {
    fn builder(mut builder: ScreenScopeBuilder<Self>) -> ScreenScopeBuilder<Self> {
        ScreenSchedule::iter().for_each(|schedule| match schedule {
            ScreenSchedule::Update => {
                add_system!(builder, schedule, switch_to_screen::<EmptyScreen>());
            }
            ScreenSchedule::Loading => {
                add_system!(builder, schedule, finish_loading::<Self>());
            }
            ScreenSchedule::Unloading => {
                add_system!(builder, schedule, finish_unloading::<Self>());
            }
            _ => {}
        });
        builder
    }
}
macro_rules! impl_updates {
    ($app:expr, $($name:ident),*) => {
        $(
            $app.add_systems(Update, (|mut data: ResMut<Conditionals>| {
                data.$name = true;
            }).run_if($name::<ConditionalsScreen>()));
        )*
    };
}
#[test]
fn test_conditionals() {
    let mut app = get_test_app::<ConditionalsScreen>();
    app.init_resource::<Conditionals>();
    impl_updates!(
        app,
        screen_loading,
        screen_ready,
        screen_unloading,
        screen_unloaded
    );
    app.add_systems(
        Update,
        (|mut res: ResMut<Conditionals>| {
            res.screen_has_state = true;
        })
        .run_if(screen_has_state::<ConditionalsScreen>(ScreenState::Ready)),
    );
    app.add_systems(
        on_screen_unloaded::<ConditionalsScreen>(),
        |res: Res<Conditionals>, mut commands: Commands| {
            let mut ok = true;
            ok = ok && res.screen_has_state;
            ok = ok && res.screen_loading;
            ok = ok && res.screen_ready;
            ok = ok && res.screen_unloading;
            ok = ok && res.screen_unloaded;
            if ok {
                commands.write_message(AppExit::Success);
            } else {
                commands.write_message(AppExit::error());
            }
        },
    );
    assert!(app.run().is_success());
}
