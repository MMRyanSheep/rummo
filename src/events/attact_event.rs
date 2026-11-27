use pumpkin_plugin_api::common::Hand;
use pumpkin_plugin_api::events::{
    EntityDamageByEntityEvent, EventData, EventHandler, EventPriority,
};
use pumpkin_plugin_api::{Context, ItemStackExt, Result, Server};

use crate::skills::axes::{effects, experience_for_damage, is_axe};
use crate::skills::skill_manager::{SkillManager, SkillType};

pub(crate) fn register(context: &Context) -> Result<()> {
    context.register_event_handler(AxesAttack, EventPriority::Normal, true)?;
    Ok(())
}

struct AxesAttack;

impl EventHandler<EntityDamageByEntityEvent> for AxesAttack {
    fn handle(
        &self,
        server: Server,
        mut data: EventData<EntityDamageByEntityEvent>,
    ) -> EventData<EntityDamageByEntityEvent> {
        if data.cancelled
            || !is_player_attack(&data.cause)
            || !data.damage.is_finite()
            || data.damage <= 0.0
        {
            return data;
        }

        let players = server.get_all_players();
        let Some(attacker) = players
            .iter()
            .find(|player| player.as_entity().get_id() as i32 == data.damager_id)
        else {
            return data;
        };

        if !attacker
            .get_item_in_hand(Hand::Right)
            .and_then(|stack| stack.get_item())
            .is_some_and(is_axe)
        {
            return data;
        }

        let experience = experience_for_damage(data.damage);
        let target_is_player = players
            .iter()
            .any(|player| player.as_entity().get_id() as i32 == data.entity_id);
        data.damage = effects::multiply_damage(data.damage, target_is_player);
        if experience > 0
            && let Err(error) = SkillManager::add_exp(attacker, SkillType::Axes, experience)
        {
            tracing::error!(attacker_id = data.damager_id, %error, "Failed to add axes experience");
        }
        data
    }
}

fn is_player_attack(cause: &str) -> bool {
    // Pumpkin 0.2.1's host formats its native DamageType with Debug.
    cause.starts_with("DamageType {") && cause.contains("message_id: \"player\",")
}

#[cfg(test)]
mod tests {
    use super::is_player_attack;

    #[test]
    fn recognizes_player_melee_damage_in_the_host_event() {
        let cause = r#"DamageType { death_message_type: Default, exhaustion: 0.1, effects: None, message_id: "player", scaling: WhenCausedByLivingNonPlayer, id: 34 }"#;
        assert!(is_player_attack(cause));
        for message_id in ["mob", "arrow", "trident", "explosion.player", "thorns"] {
            assert!(!is_player_attack(
                &cause.replace("\"player\"", &format!("\"{message_id}\""))
            ));
        }
    }
}
