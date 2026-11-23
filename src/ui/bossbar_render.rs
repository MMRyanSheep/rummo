use pumpkin_plugin_api::Server;
use pumpkin_plugin_api::text::TextComponent;
use pumpkin_plugin_api::uuid::Uuid;

use super::bossbar_manager::BossBarManager;
use crate::skills::skill_manager::{SkillInfo, SkillType};

pub(super) fn render_dirty(manager: &BossBarManager, server: &Server) {
    for (bar, skill, info) in manager.dirty() {
        let (title, progress) = presentation(skill, info);
        bar.bar.set_title(TextComponent::text(&title));
        bar.bar.set_health(progress);
        if !bar.is_visible() {
            let uuid = Uuid {
                high: (info.player_uuid >> 64) as u64,
                low: info.player_uuid as u64,
            };
            let Some(player) = server.get_player_by_uuid(uuid) else {
                continue;
            };
            bar.show(player);
        }
        manager.clear_rendered(&bar, skill, info);
    }
}

fn presentation(skill: SkillType, info: SkillInfo) -> (String, f32) {
    let name = skill.display_name();
    let level = info.level;
    match info.required_experience {
        Some(required) => (
            format!(
                "{name} Lv.{level} — {} / {} XP",
                format_xp(info.experience),
                format_xp(required),
            ),
            (f64::from(info.experience) / f64::from(required)).clamp(0.0, 1.0) as f32,
        ),
        None => (format!("{name} Lv.{level} — 已满级"), 1.0),
    }
}

fn format_xp(experience: u32) -> String {
    let digits = experience.to_string();
    let mut formatted = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            formatted.push(',');
        }
        formatted.push(digit);
    }
    formatted
}
