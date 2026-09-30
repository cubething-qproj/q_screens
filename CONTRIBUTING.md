# Contributing to q_screens

## Auditing persistent types on a Bevy bump

Screen cleanup despawns every entity that isn't `Persistent`, except entities
with a registered persistent type (`src/persistent.rs`). Bevy spawns some
entities of its own, and cleanup must not kill those. On every Bevy version bump:

1. Run `just audit-bevy-spawns <old> <new>` (e.g. `0.19.1 0.20.0`). It prints
   the main-world spawn sites Bevy added or removed between the two versions,
   each with 3 lines of surrounding code (`-C <n>` to change, like `rg -C`).
   Both versions must be in the cargo registry: run `cargo fetch` in a project
   on each version if needed.
2. Classify each `ADDED` site (and every site of a `NEW CRATE`) with the rule of
   thumb below.
3. For each `REMOVED` site, check whether a registered type is still spawned
   anywhere. If not, drop its registration and test.
4. Update the type table and the "audited against Bevy" version in the
   `persistent` module docs.

### Rule of thumb: should this entity survive screen changes?

Work through these in order. The first one that applies decides.

1. **It isn't a main-world entity: ignore it.** This covers spawns in render-app
   or `ExtractSchedule` systems, `TemporaryRenderEntity`, and render-world sync.
   Screen cleanup never sees the render world. Futures spawned on a task pool
   aren't entities at all.
2. **The app caused it: it's screen-scoped (the default).** If the entity exists
   only because the app asked for it, the app owns its lifetime. Examples:
   `spawn`/`with_children`/`with_child`/`spawn_batch`, relationship helpers,
   entity cloning, scene and glTF spawning, BRP requests, widget children under
   the app's own UI. Do nothing.
3. **Bevy despawns it itself: ignore it.** Transient entities with an engine
   managed lifetime (e.g. a `Screenshot` that despawns after capture) don't need
   exempting; losing one mid-screen-change is acceptable.
4. **The engine keeps a reference to it: it must persist.** Look for the entity
   stored in a resource or map, e.g. `bevy_winit`'s monitor map, gilrs's gamepad
   map, `LineGizmoEntities`. Despawning it leaves the engine holding a dead
   entity (`OnMonitor ... relates to an entity that does not exist`). This is
   the strongest signal.
5. **The engine spawns it on its own: it should persist.** Entities from a
   plugin's `Startup` system or `FromWorld` impl, or from hardware and OS events
   (monitor hotplug, gamepad connect, touch start), aren't the app's to scope.
6. **It's global by intent: it should persist.** Plugin-level observers
   (`add_observer`) and one-shot systems registered by plugins.

### Registering a type

- **Pick the component** that marks exactly those entities. It must be public,
  and it must not also appear on entities the app spawns for itself. A
  registered type survives on every entity that has it.
- **Pick the scope.** Use `Anywhere` unless children with the type belong to
  their parent. `Observer` is `TopLevel` because an observer parented to a
  screen entity must die with the screen.
- **Gate it.** If the type comes from an optional Bevy crate, register it under
  a `q_screens` feature named after that crate (e.g.
  `bevy_picking = ["bevy/bevy_picking"]`).
- **Test it** in `src/persistent.rs`: spawn it top-level and as a child, and
  check survival against its scope.
- **No usable marker?** If the only marker is private (e.g.
  `GizmoOverlayCamera`) or there is none (plain `Node` roots), it can't be
  registered. List it as "not covered yet" in the `persistent` module docs and
  file a Bevy issue asking for a public marker component.
