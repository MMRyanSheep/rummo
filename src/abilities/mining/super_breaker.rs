use pumpkin_plugin_api::{
    Attribute, AttributeModifier, ModifierOperation, Player, text::TextComponent,
};

use crate::i18n;

const SPEED_MODIFIER_ID: &str = "rummo:super_breaker";

pub(crate) fn activate_super_breaker(player: &Player) {
    let Some(living) = player.as_entity().as_living() else {
        return;
    };
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
