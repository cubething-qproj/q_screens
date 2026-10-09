//! This module contains all systems, including observers.

use std::any::TypeId;

use crate::prelude::*;
use bevy::ecs::system::SystemChangeTick;

fn handle_switch_msg(
    mut reader: MessageReader<SwitchToScreenMsg>,
    mut registry: ResMut<ScreenData>,
    tick: SystemChangeTick,
    mut commands: Commands,
    mut current_screen: ResMut<CurrentScreen>,
) {
    // get the most recent valid message, then load it and unload all others
    if reader.is_empty() {
        return;
    }
    let vec = reader.read().collect::<Vec<_>>();
    let msg_key = vec.iter().rev().find(|msg| registry.get(msg.0).is_ok());
    let msg_key = rq!(msg_key);
    for (key, value) in registry.iter_mut().enumerate() {
        if let Some(data) = value {
            if key == *msg_key.0 {
                data.queue_load(tick.this_run());
            } else {
                data.unload(tick.this_run());
            }
        }
    }
    commands.trigger(ScreenChanged {
        from: **current_screen,
        to: msg_key.0,
    });
    **current_screen = Some(msg_key.0)
}
/// NOTE: This is registered in scope.rs
pub(crate) fn on_switch_screen<S: Screen>(
    _trigger: On<SwitchToScreen<S>>,
    registry: Res<ScreenRegistry>,
    mut commands: Commands,
) {
    let id = registry
        .get(&TypeId::of::<S>())
        .map_err(|_| ScreenError::NoSuchScreen(S::name()));
    commands.write_message(SwitchToScreenMsg(id.unwrap()));
}

pub(crate) fn on_finish_loading<S: Screen>(
    _trigger: On<FinishLoading<S>>,
    mut data: ScreenInfoMut<S>,
) {
    data.finish_loading();
}
pub(crate) fn on_finish_unloading<S: Screen>(
    _trigger: On<FinishUnloading<S>>,
    mut data: ScreenInfoMut<S>,
) {
    data.finish_unloading();
}

fn run_schedules(mut data: ResMut<ScreenData>, mut commands: Commands, tick: SystemChangeTick) {
    let all_clear = data.iter().filter_map(|info| info.as_ref()).all(|info| {
        matches!(
            info.state(),
            ScreenState::Unloaded | ScreenState::LoadQueued
        )
    });

    for info in data.iter_mut().filter_map(|info| info.as_mut()) {
        match info.state() {
            ScreenState::Unloaded => {
                if !info.initialized {
                    info.initialized = true;
                    info.needs_update = false;
                    info.changed_at = tick.this_run();
                }
                if info.needs_update {
                    commands.run_schedule(OnScreenUnloaded(info.type_id()));
                    info.needs_update = false;
                    info.changed_at = tick.this_run();
                }
            }
            ScreenState::LoadQueued => {
                if info.needs_update {
                    commands.run_schedule(OnScreenLoadQueued(info.type_id()));
                    info.needs_update = false;
                    info.changed_at = tick.this_run();
                }
                if all_clear {
                    info.load(tick.this_run());
                }
            }
            ScreenState::Loading => {
                commands.run_schedule(ScreenScheduleLabel::from_id(
                    ScreenSchedule::Loading,
                    info.type_id(),
                ));
                if info.needs_update {
                    commands.run_schedule(OnScreenLoad(info.type_id()));
                    info.needs_update = false;
                    info.changed_at = tick.this_run();
                }
                if matches!(info.load_strategy(), LoadStrategy::Nonblocking) {
                    run_update_schedule(&mut commands, info, ScreenSchedule::Update);
                }
            }
            ScreenState::Ready => {
                run_update_schedule(&mut commands, info, ScreenSchedule::Update);
            }
            ScreenState::Unloading => {
                commands.run_schedule(ScreenScheduleLabel::from_id(
                    ScreenSchedule::Unloading,
                    info.type_id(),
                ));
                if info.needs_update {
                    commands.run_schedule(OnScreenUnload(info.type_id()));
                    info.needs_update = false;
                    info.changed_at = tick.this_run();
                }
            }
            ScreenState::Cleanup => {
                if info.needs_update {
                    commands.run_schedule(OnScreenCleanup(info.type_id()));
                    info.needs_update = false;
                    info.changed_at = tick.this_run();
                }
            }
        }
    }
}

fn run_fixed_schedules(mut registry: ResMut<ScreenData>, mut commands: Commands) {
    for data in registry.iter_mut().filter_map(|d| d.as_mut()) {
        match data.state() {
            ScreenState::Loading => {
                if matches!(data.load_strategy(), LoadStrategy::Nonblocking) {
                    run_update_schedule(&mut commands, data, ScreenSchedule::FixedUpdate);
                }
            }
            ScreenState::Ready => {
                run_update_schedule(&mut commands, data, ScreenSchedule::FixedUpdate);
            }
            _ => {}
        }
    }
}

/// Completes pending Ready initialization before either screen update schedule.
/// Check at command execution: a queued Loading schedule can finish loading first.
fn run_update_schedule(commands: &mut Commands, info: &ScreenInfo, schedule: ScreenSchedule) {
    let screen_id = info.screen_id();
    let type_id = info.type_id();
    commands.queue(move |world: &mut World| {
        let tick = world.change_tick();
        let needs_ready = {
            let mut data = world.resource_mut::<ScreenData>();
            let info = r!(data.get_mut(screen_id));
            if info.state() == ScreenState::Ready && info.needs_update {
                info.needs_update = false;
                info.changed_at = tick;
                true
            } else {
                false
            }
        };
        if needs_ready {
            r!(world.try_run_schedule(OnScreenReady(type_id)));
        }
    });
    commands.run_schedule(ScreenScheduleLabel::from_id(schedule, type_id));
}

pub(crate) fn initial_screen(
    mut commands: Commands,
    initial_screen: Res<InitialScreen>,
    screens: Screens,
) {
    if let Some(initial_screen) = (*initial_screen).as_ref() {
        let info = r!(screens.get_by_name(initial_screen));
        commands.write_message(SwitchToScreenMsg(info.screen_id()));
    }
}

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(Startup, initial_screen);
    app.add_systems(PostUpdate, handle_switch_msg);
    app.add_systems(Update, run_schedules);
    app.add_systems(FixedUpdate, run_fixed_schedules);
}
