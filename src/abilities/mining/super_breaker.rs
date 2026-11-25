use pumpkin_plugin_api::{
    Attribute, AttributeModifier, ModifierOperation, Player, events::player_respawn,
};
const SPEED_MODIFIER_ID: &str = "rummo:mining_speed";

pub(crate) fn activate_super_breaker(player: &Player) {
    let Some(living) = player.as_entity().as_living() else {
        return;
    };
    living.add_attribute_modifier(
        Attribute::BlockBreakSpeed,
        &AttributeModifier {
            id: SPEED_MODIFIER_ID.into(),
            amount: 4.0,
            operation: ModifierOperation::MultiplyTotal,
        },
    );
    return;
}

pub(crate) fn deactivate_super_breaker(player: &Player) {
    let Some(living) = player.as_entity().as_living() else {
        return;
    };
    living.remove_attribute_modifier(Attribute::BlockBreakSpeed, SPEED_MODIFIER_ID);
}
