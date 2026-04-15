mod screens;
pub mod prelude {
    pub use super::get_test_app;
    pub use super::screens::prelude::*;
    pub use bevy::prelude::*;
    pub use bevy_asset_loader::prelude::*;
    pub use q_screens::prelude::*;
    pub use q_test_harness::prelude::*;
}

use prelude::*;
pub fn get_test_app<S: Screen>() -> App {
    let mut app = App::new();
    app.add_plugins((TestRunnerPlugin::default(), ScreenPlugin));
    app.register_screen::<S>();
    app.register_screen::<EmptyScreen>();
    app.insert_resource(InitialScreen::new::<S>());
    app
}
