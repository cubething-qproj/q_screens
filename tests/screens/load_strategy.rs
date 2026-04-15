use crate::prelude::*;

#[derive(Resource, Debug, Default)]
struct FinalValue(u32);

#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Reflect)]
struct LoadStrategyScreen<const BLOCKING: bool>;

impl<const BLOCKING: bool> Screen for LoadStrategyScreen<BLOCKING> {
    fn builder(mut builder: ScreenScopeBuilder<Self>) -> ScreenScopeBuilder<Self> {
        builder
            .with_load_strategy(if BLOCKING {
                LoadStrategy::Blocking
            } else {
                LoadStrategy::Nonblocking
            })
            .add_systems(ScreenSchedule::Update, Self::update)
            .add_systems(ScreenSchedule::Loading, Self::load)
            .add_systems(ScreenSchedule::OnUnloaded, Self::unloaded);
        builder
    }
}

impl<const BLOCKING: bool> LoadStrategyScreen<BLOCKING> {
    fn load(mut count: Local<u32>, mut data: ScreenInfoMut<Self>) {
        *count += 1;
        if *count == 100 {
            data.finish_loading();
            info!("Finished loading!");
        }
    }

    fn update(
        mut count: Local<u32>,
        data: ScreenInfoRef<Self>,
        mut commands: Commands,
        mut value: ResMut<FinalValue>,
    ) {
        *count += 1;
        if data.data().state() == ScreenState::Ready {
            value.0 = *count;
            commands.trigger(switch_to_screen::<EmptyScreen>());
        }
    }

    fn unloaded(data: ScreenInfoRef<Self>, value: Res<FinalValue>, mut commands: Commands) {
        let expected_value = match data.data().load_strategy() {
            LoadStrategy::Nonblocking => 100,
            LoadStrategy::Blocking => 1,
        };
        info!("Got {}, expected {}", value.0, expected_value);
        if value.0 != expected_value {
            commands.write_message(AppExit::error());
        } else {
            commands.write_message(AppExit::Success);
        }
    }
}

#[test]
fn blocking() {
    let mut app = get_test_app::<LoadStrategyScreen<true>>();
    app.init_resource::<FinalValue>();
    app.register_screen::<EmptyScreen>();
    assert!(app.run().is_success());
}

#[test]
fn nonblocking() {
    let mut app = get_test_app::<LoadStrategyScreen<false>>();
    app.init_resource::<FinalValue>();
    app.register_screen::<EmptyScreen>();
    assert!(app.run().is_success());
}
