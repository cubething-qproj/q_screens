use bevy::{
    app::Propagate,
    ecs::{lifecycle::HookContext, world::DeferredWorld},
};
use itertools::Itertools;

use crate::prelude::*;

// TODO: Test that screens which spawn entities in `load` are able to access those entities.
// Don't run `load` until all the scoped entities are removed!

#[derive(Component, PartialEq, Debug, Copy, Clone)]
#[component(on_insert = Self::on_insert)]
enum Target {
    PersistentParent,
    PersistentChild,
    PersistentGrandchild,
    PersistentGreatGrandchild,
    ScopedGrandchild,
    ScopedParent,
    ScopedChild,
}
impl Target {
    fn on_insert(mut world: DeferredWorld, ctx: HookContext) {
        let this = *world.get::<Self>(ctx.entity).unwrap();
        world
            .commands()
            .entity(ctx.entity)
            .insert(Name::new(format!("{:?}", this)));
    }
}

#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Reflect)]
pub struct ScopedEntitiesScreen;
impl Screen for ScopedEntitiesScreen {
    fn builder(mut builder: ScreenScopeBuilder<Self>) -> ScreenScopeBuilder<Self> {
        builder.add_systems(
            ScreenSchedule::Loading,
            |mut commands: Commands, mut data: ScreenInfoMut<Self>| {
                commands.spawn((
                    Target::PersistentParent,
                    Propagate(Persistent),
                    children![
                        Target::PersistentChild,
                        children![(
                            Target::ScopedGrandchild,
                            ScreenScoped,
                            children![Target::PersistentGreatGrandchild]
                        )]
                    ],
                ));
                commands.spawn((
                    Target::ScopedParent,
                    children![
                        Target::ScopedChild,
                        children![(Target::PersistentGrandchild, Persistent)]
                    ],
                ));
                commands.log_hierarchy();
                data.finish_loading();
            },
        );
        builder.add_systems(ScreenSchedule::OnReady, |mut commands: Commands| {
            commands.trigger(switch_to_screen::<EmptyScreen>());
        });
        builder.add_systems(
            ScreenSchedule::OnUnloaded,
            |mut commands: Commands, query: Query<&Target>| {
                let mut ok = true;
                ok = ok && query.iter().contains(&Target::PersistentParent);
                ok = ok && query.iter().contains(&Target::PersistentChild);
                ok = ok && query.iter().contains(&Target::PersistentGrandchild);
                ok = ok && query.iter().contains(&Target::PersistentGreatGrandchild);
                ok = ok && !query.iter().contains(&Target::ScopedParent);
                ok = ok && !query.iter().contains(&Target::ScopedChild);
                ok = ok && !query.iter().contains(&Target::ScopedGrandchild);
                if ok {
                    commands.log_hierarchy();
                    commands.write_message(AppExit::Success);
                } else {
                    error!("Bad hierarchy.");
                    commands.log_hierarchy();
                    commands.write_message(AppExit::error());
                }
            },
        );
        builder
    }
}

type Scr = ScopedEntitiesScreen;
#[test]
fn test_scoped_entities() {
    let mut app = get_test_app::<Scr>();
    app.register_screen::<EmptyScreen>();
    assert!(app.run().is_success());
}
