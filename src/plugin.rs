use bevy::app::HierarchyPropagatePlugin;

use crate::prelude::*;
/// The main export plugin for TFW. `Screens` should be an enum with screen
/// names. Refer to the template documentation for more details.
/// The template parameter refers to the initial screen.
#[derive(Default, Debug)]
pub struct ScreenPlugin;
impl Plugin for ScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScreenRegistry>();
        app.init_resource::<ScreenData>();
        app.init_resource::<InitialScreen>();
        app.init_resource::<CurrentScreen>();
        app.add_message::<SwitchToScreenMsg>();
        app.add_plugins((
            HierarchyPropagatePlugin::<Persistent>::new(PostUpdate),
            HierarchyPropagatePlugin::<ScreenScoped>::new(PostUpdate),
        ));
        app.add_plugins(super::systems::plugin);
    }
}
