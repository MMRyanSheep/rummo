use pumpkin_plugin_api::{
    Attribute, AttributeModifier, ModifierOperation, Player, text::TextComponent,
};

use crate::abilities::ability_definition::AbilityDefinition;
use crate::abilities::ability_manager::AbilityId;
use crate::abilities::active_ability_manager::show_cooldown;
use crate::abilities::cooldown_manager::CooldownManager;
use crate::i18n;

const SPEED_MODIFIER_ID: &str = "rummo:super_breaker";

pub(crate) fn activate_super_breaker(
    player: &Player,
    player_id: u128,
    definition: &AbilityDefinition,
) -> Result<bool, &'static str> {
    let Some(living) = player.as_entity().as_living() else {
        return Ok(false);
    };
    let ability_id = AbilityId::SuperBreaker;
    if let Some(remaining) =
        CooldownManager::start_cooldown(player, player_id, ability_id, definition)?
    {
        show_cooldown(player, ability_id, remaining);
        return Ok(false);
    }
    living.add_attribute_modifier(
        Attribute::MiningEfficiency,
        &AttributeModifier {
            id: SPEED_MODIFIER_ID.into(),
            amount: 444.0,
            operation: ModifierOperation::Add,
        },
    );
    let messages = i18n::for_player(player);
    player.show_subtitle(TextComponent::text(
        messages.text("ability.super_breaker.activated"),
    ));
    player.show_title(TextComponent::text(""));
    Ok(true)
}

pub(crate) fn deactivate_super_breaker(player: &Player) {
    let Some(living) = player.as_entity().as_living() else {
        return;
    };
    let messages = i18n::for_player(player);
    player.show_subtitle(TextComponent::text(
        messages.text("ability.super_breaker.expired"),
    ));
    player.show_title(TextComponent::text(""));
    living.remove_attribute_modifier(Attribute::MiningEfficiency, SPEED_MODIFIER_ID);
}
