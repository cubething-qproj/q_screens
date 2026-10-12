---
name: q-screens
description: Deep dive on q_screens (lib/q_screens), the Bevy screen system — the Screen trait and ScreenScopeBuilder, ScreenSchedule variants, ScreenState lifecycle (LoadQueued/Loading/Ready/Unloading/Cleanup/Unloaded), switching screens, finishing load/unload, run conditions and system params (ScreenInfoRef/Mut, Screens), screen-scoped entity cleanup, Persistent/ScreenScoped, and PersistentTypes. Use when adding or modifying a screen in quell/vn-jun-26, debugging entities disappearing on screen change, or touching q_screens itself.
---

# q_screens

Source: `lib/q_screens/src/{trait_impl.rs, scope.rs, data.rs, systems.rs,
persistent.rs}`. `CONTRIBUTING.md` covers auditing persistent types on Bevy
bumps (`just audit-bevy-spawns <old> <new>` in the crate). Real usage:
`bin/quell/src/screen/*/screen.rs`. Tests: `tests/screens/*.rs`,
`tests/ready_ordering.rs`. `use q_screens::prelude::*;`.

## Defining a screen

```rust
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Reflect)]
pub struct WorldScreen;
impl Screen for WorldScreen {
    fn builder(mut b: ScreenScopeBuilder<Self>) -> ScreenScopeBuilder<Self> {
        b.add_systems(ScreenSchedule::Loading, load_world);   // presence enables Loading
        b.add_systems(ScreenSchedule::OnReady, init);
        b.add_systems(ScreenSchedule::Update, tick);
        b
    }
}
fn load_world(mut screen: ScreenInfoMut<WorldScreen>, /* … */) {
    if ready { screen.finish_loading(); }
}
app.add_plugins(ScreenPlugin);
app.register_screen::<WorldScreen>();                 // re-registering warns and no-ops
app.insert_resource(InitialScreen::new::<SplashScreen>()); // switched to at Startup
```

`Screen: Component + Reflect + Default`; `Screen::name()` defaults to the
short type path (used by `Screens::get_by_name`, not uniqueness-checked).

## Schedules (`ScreenSchedule`)

- Per frame: `Update` (when Ready; also Loading if `LoadStrategy::Nonblocking`),
  `FixedUpdate` (same rule), `Loading`, `Unloading`.
- On transition: `OnLoad`, `OnReady`, `OnUnload`, `OnUnloaded`. Also
  available as labels `on_screen_load::<S>()` etc. for `app.add_systems`.
- `Cleanup` is internal.

Builder flags: `with_skip_load(bool)` / `with_skip_unload(bool)` — default is
skip **unless** the `Loading`/`Unloading` schedule has systems. Adding only
`OnLoad`/`OnUnload` does *not* enable loading. `with_load_strategy(...)`
(default `Blocking`).

## Lifecycle

```text
Unloaded → LoadQueued → (wait until every screen is Unloaded/LoadQueued)
        → Loading → Ready → Unloading → Cleanup → Unloaded
```

- Switch: `commands.trigger(switch_to_screen::<S>())` (typed) or
  `commands.write_message(SwitchToScreenMsg(id))`. Messages are handled in
  `PostUpdate`; only the last valid one per frame wins. `ScreenChanged
  { from, to }` is triggered.
- Finish: `ScreenInfoMut::<S>::finish_loading()/finish_unloading()`, or
  `commands.trigger(finish_loading::<S>())` / `finish_unloading::<S>()`.
- Screen schedules run via `commands.run_schedule` from `Update`
  (`run_schedules`) and `FixedUpdate`. `OnReady` is guaranteed to run before
  the first `Update`/`FixedUpdate` of the screen.

Introspection: `ScreenInfoRef<S>` / `ScreenInfoMut<S>` (panic if `S` isn't
registered), `Screens` / `ScreensMut` (by type, id, or name), `ScreenIdFor<S>`,
`CurrentScreen`, `ScreenData`. Run conditions: `screen_ready::<S>()`,
`screen_loading`, `screen_unloading`, `screen_unloaded`,
`screen_has_state::<S>(state)`.

## Entity scoping — the part that bites

On `Cleanup`, **every entity is despawned** except:

- entities with `Persistent` (and no `ScreenScoped`);
- entities with a registered persistent type
  (`app.register_persistent_type::<T>(PersistentTypeScope::{TopLevel, Anywhere})`).
  Built in: resource entities, top-level `Observer`s and one-shot systems,
  `Window` (TopLevel), `Monitor`; `Gamepad` with feature `bevy_gilrs`,
  `PointerId` with `bevy_picking`.

Cleanup is per entity, not per hierarchy. To keep a subtree, use
`Propagate(Persistent)` (`HierarchyPropagatePlugin` is installed in
`PostUpdate`); `ScreenScoped` opts a child back out. Gizmo and dev-tools
overlay entities are not covered — mark them yourself.

Things that commonly vanish unexpectedly: cameras, UI roots, q_term
terminals/shells, anything spawned in `Startup` by another plugin.

## When changing q_screens

`#![deny(missing_docs)]` is on. Follow `CONTRIBUTING.md` to classify new Bevy
spawn sites and update the persistent-type table in `persistent.rs` docs.
