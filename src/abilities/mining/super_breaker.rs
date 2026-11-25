use pumpkin_plugin_api::{
    Attribute, AttributeModifier, ModifierOperation, Player, text::TextComponent,
};
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
    player.show_subtitle(TextComponent::text("已激活超级挖掘机"));
    player.show_title(TextComponent::text(""));
}

pub(crate) fn deactivate_super_breaker(player: &Player) {
    let Some(living) = player.as_entity().as_living() else {
        return;
    };
    player.show_subtitle(TextComponent::text("超级挖掘机技能已结束"));
    player.show_title(TextComponent::text(""));
    living.remove_attribute_modifier(Attribute::MiningEfficiency, SPEED_MODIFIER_ID);
}
