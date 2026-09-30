<div align="center">
<img src="https://raw.githubusercontent.com/ada-x64/qproj/refs/heads/main/.doc/q_screens.png" height=300 alt="Illustration of a bowerbird with text 'q_screens'" title="tfw logo" />
</div>


[![Coverage Status](https://coveralls.io/repos/github/cubething-qproj/q_screens/badge.svg)](https://coveralls.io/github/cubething-qproj/q_screens)

Screen implementation for Bevy.

_This continues the work started [here.](https://github.com/ada-x64/tfw)_

## Features

- [x] Basic screen lifecycle (load, ready, unload, unloaded)
- [x] Screen-scoped and persistent entities
- [x] App-level screen registration
- [x] Friendly and bevyish API
- [x] Well-tested

## Screen scoping

When a screen changes, every entity that isn't `Persistent` is despawned.
`ScreenPlugin` exempts Bevy's engine-internal entities (a few, from gizmos and
dev-tools overlays, aren't covered yet), and apps can exempt whole component
types with `app.register_persistent_type::<T>(scope)`. See the `persistent`
module docs for details.

Some engine types come from optional Bevy crates. Enable the matching feature
for each one your app uses:

- `bevy_picking`: pointers (including app-spawned `PointerId::Custom` pointers,
  which then outlive screens)
- `bevy_gilrs`: gamepads

## About the bird

"Like all bowerbirds, the satin bowerbird shows highly complex courtship behaviour. ... Males build specialised stick structures, called bowers, which they decorate with blue, yellow, and shiny objects, including berries, flowers, snail shells, and plastic items such as ballpoint pens, drinking straws and clothes pegs." ([wikipedia](https://en.wikipedia.org/wiki/Satin_bowerbird))
