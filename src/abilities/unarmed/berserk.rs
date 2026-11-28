use pumpkin_plugin_api::{
    Attribute, AttributeModifier, ModifierOperation, Player, text::TextComponent,
};

use crate::abilities::ability_definition::AbilityDefinition;
use crate::abilities::ability_manager::AbilityId;
use crate::abilities::active_ability_manager::{self, show_cooldown};
use crate::abilities::cooldown_manager::CooldownManager;
use crate::i18n;
use crate::skills::skill_manager::player_uuid;

const SPEED_MODIFIER_ID: &str = "rummo:berserk";
/// Added to the total break speed, so 1.0 doubles it.
const BREAK_SPEED_BONUS: f64 = 1.0;
const DAMAGE_MULTIPLIER: f32 = 1.5;

pub(crate) fn activate_berserk(
    player: &Player,
    player_id: u128,
    definition: &AbilityDefinition,
) -> Result<bool, &'static str> {
    let Some(living) = player.as_entity().as_living() else {
        return Ok(false);
    };
    println!("123");
    let ability_id = AbilityId::Berserk;
    if let Some(remaining) =
        CooldownManager::start_cooldown(player, player_id, ability_id, definition)?
    {
        show_cooldown(player, ability_id, remaining);
        return Ok(false);
    }
    living.add_attribute_modifier(
        Attribute::BlockBreakSpeed,
        &AttributeModifier {
            id: SPEED_MODIFIER_ID.into(),
            amount: BREAK_SPEED_BONUS,
            operation: ModifierOperation::MultiplyTotal,
        },
    );
    let messages = i18n::for_player(player);
    player.show_subtitle(TextComponent::text(
        messages.text("ability.berserk.activated"),
    ));
    player.show_title(TextComponent::text(""));
    Ok(true)
}

pub(crate) fn deactivate_berserk(player: &Player) {
    let messages = i18n::for_player(player);
    player.show_subtitle(TextComponent::text(
        messages.text("ability.berserk.expired"),
    ));
    player.show_title(TextComponent::text(""));
    remove_speed_modifier(player);
}

/// Removes a saved speed bonus left by a player who was offline when Berserk expired.
pub(crate) fn remove_stale_speed_modifier(player: &Player) {
    if !active_ability_manager::is_active(player_uuid(player), AbilityId::Berserk) {
        remove_speed_modifier(player);
    }
}

/// Applies the damage multiplier while Berserk is active.
pub(crate) fn boost_damage(player: &Player, damage: f32) -> f32 {
    if active_ability_manager::is_active(player_uuid(player), AbilityId::Berserk) {
        damage * DAMAGE_MULTIPLIER
    } else {
        damage
    }
}

fn remove_speed_modifier(player: &Player) {
    if let Some(living) = player.as_entity().as_living() {
        living.remove_attribute_modifier(Attribute::BlockBreakSpeed, SPEED_MODIFIER_ID);
    }
}
