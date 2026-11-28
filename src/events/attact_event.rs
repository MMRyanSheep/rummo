use pumpkin_plugin_api::common::Hand;
use pumpkin_plugin_api::events::{
    EntityDamageByEntityEvent, EventData, EventHandler, EventPriority,
};
use pumpkin_plugin_api::{Context, Item, ItemStack, ItemStackExt, Player, Result, Server};

use crate::abilities::unarmed::berserk;
use crate::skills::axes::{armor, effects, is_axe};
use crate::skills::experience_for_damage;
use crate::skills::skill_manager::{SkillManager, SkillType};
use crate::skills::unarmed;

pub(crate) fn register(context: &Context) -> Result<()> {
    context.register_event_handler(CombatAttack, EventPriority::Normal, true)?;
    Ok(())
}

struct CombatAttack;

impl EventHandler<EntityDamageByEntityEvent> for CombatAttack {
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

        let stack = attacker.get_item_in_hand(Hand::Right);
        let Some(skill) = attack_skill(stack.as_ref()) else {
            return data;
        };

        let experience = experience_for_damage(data.damage);
        if experience > 0
            && let Err(error) = SkillManager::add_exp(attacker, skill, experience)
        {
            tracing::error!(attacker_id = data.damager_id, skill = skill.id(), %error, "Failed to add combat experience");
        }

        match skill {
            SkillType::Axes => {
                let target = players
                    .iter()
                    .find(|player| player.as_entity().get_id() as i32 == data.entity_id);
                data.damage = apply_axe_effects(attacker, target, data.damage);
            }
            SkillType::Unarmed => {
                match SkillManager::get_skill(attacker, SkillType::Unarmed) {
                    Ok(skill) => {
                        data.damage = unarmed::effects::add_bonus_damage(data.damage, skill.level);
                        if let Some(target) = players
                            .iter()
                            .find(|player| player.as_entity().get_id() as i32 == data.entity_id)
                        {
                            unarmed::disarm::try_disarm(target, skill.level);
                        }
                    }
                    Err(error) => {
                        tracing::error!(attacker_id = data.damager_id, %error, "Failed to read unarmed level");
                    }
                }
                data.damage = berserk::boost_damage(attacker, data.damage);
            }
            SkillType::Mining => {}
        }
        data
    }
}

/// The combat skill trained by the held stack, if any.
fn attack_skill(stack: Option<&ItemStack>) -> Option<SkillType> {
    let Some(stack) = stack else {
        return Some(SkillType::Unarmed);
    };
    // Custom items yield `None` here and train nothing.
    match stack.get_item()? {
        Item::Air => Some(SkillType::Unarmed),
        item if is_axe(item) => Some(SkillType::Axes),
        _ => None,
    }
}

/// Rolls the axe critical hit and armor wear, returning the damage to deal.
fn apply_axe_effects(attacker: &Player, target: Option<&Player>, damage: f32) -> f32 {
    match SkillManager::get_skill(attacker, SkillType::Axes) {
        Ok(skill) => {
            let damage = effects::multiply_damage(damage, target.is_some(), skill.level);
            if let Some(target) = target {
                armor::wear_player_armor(target, skill.level);
            }
            damage
        }
        Err(error) => {
            tracing::error!(attacker_id = attacker.as_entity().get_id(), %error, "Failed to read axes level for critical hit");
            damage
        }
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
