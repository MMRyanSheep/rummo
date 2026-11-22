use std::sync::{Arc, Mutex};
use std::time::Duration;

use pumpkin_plugin_api::Context;
use pumpkin_plugin_api::scheduler::SchedulerExt;

use super::bossbar_manager::BossBarManager;
use super::bossbar_render;

const UPDATE_INTERVAL_TICKS: u64 = 4;
const HIDE_AFTER: Duration = Duration::from_secs(5);

pub(crate) fn register(context: &Context, manager: &Arc<BossBarManager>) {
    for player in context.get_server().get_all_players() {
        manager.add_player(&player);
    }

    let manager = Arc::downgrade(manager);
    let updating = Mutex::new(());
    context.schedule_repeating_task(
        UPDATE_INTERVAL_TICKS,
        UPDATE_INTERVAL_TICKS,
        move |server| {
            // A server call can yield, so avoid overlapping passes over the same bars.
            let Ok(_update) = updating.try_lock() else {
                return;
            };
            if let Some(manager) = manager.upgrade() {
                manager.hide_expired(HIDE_AFTER);
                bossbar_render::render_dirty(&manager, &server);
            }
        },
    );
}
