use pumpkin_plugin_api::wit::pumpkin::plugin::data_components::DataComponent;
use pumpkin_plugin_api::{ItemStack, Player};

const DAMAGE_PER_LEVEL: f32 = 0.3;

pub(crate) fn wear_player_armor(player: &Player, level: u32) {
    let durability_loss = (level as f32 * DAMAGE_PER_LEVEL) as i32;
    if durability_loss == 0 {
        return;
    }

    let wear = |stack| wear_stack(stack, durability_loss);
    let inventory = player.get_inventory();
    if let Some(update) = inventory.get_helmet().and_then(wear) {
        inventory.set_helmet(update);
    }
    if let Some(update) = inventory.get_chestplate().and_then(wear) {
        inventory.set_chestplate(update);
    }
    if let Some(update) = inventory.get_leggings().and_then(wear) {
        inventory.set_leggings(update);
    }
    if let Some(update) = inventory.get_boots().and_then(wear) {
        inventory.set_boots(update);
    }
}

/// Returns the slot update, with an empty slot for broken armor.
fn wear_stack(stack: ItemStack, durability_loss: i32) -> Option<Option<ItemStack>> {
    let mut damage = 0;
    let mut max_damage = None;
    for component in stack.get_components() {
        match component.component {
            DataComponent::Damage => damage = decode_durability(&component.value)?,
            DataComponent::MaxDamage => max_damage = Some(decode_durability(&component.value)?),
            _ => {}
        }
    }

    // The host exposes component overrides, so an unmodified stack needs the item default.
    let max_damage = max_damage.or_else(|| vanilla_max_damage(&stack.get_registry_key()))?;
    if max_damage == 0 {
        return None;
    }
    let Some(new_damage) = damage_armor(damage, max_damage, durability_loss) else {
        return Some(None);
    };
    stack.set_component(DataComponent::Damage, &encode_durability(new_damage));
    Some(Some(stack))
}

fn damage_armor(damage: i32, max_damage: i32, durability_loss: i32) -> Option<i32> {
    damage
        .checked_add(durability_loss)
        .filter(|new_damage| *new_damage <= max_damage)
}

fn vanilla_max_damage(key: &str) -> Option<i32> {
    let key = key.strip_prefix("minecraft:").unwrap_or(key);
    if key == "elytra" {
        return Some(432);
    }

    let (material, piece) = key.split_once('_')?;
    let material_multiplier = match material {
        "leather" => 5,
        "golden" => 7,
        "copper" => 11,
        "chainmail" | "iron" => 15,
        "turtle" if piece == "helmet" => 25,
        "diamond" => 33,
        "netherite" => 37,
        _ => return None,
    };
    let piece_durability = match piece {
        "helmet" => 11,
        "chestplate" => 16,
        "leggings" => 15,
        "boots" => 13,
        _ => return None,
    };
    Some(material_multiplier * piece_durability)
}

fn decode_durability(bytes: &[u8]) -> Option<i32> {
    let mut value = 0u32;
    for (index, &byte) in bytes.iter().enumerate() {
        if index == 4 && byte > 0x07 {
            return None;
        }
        value |= u32::from(byte & 0x7f) << (index * 7);
        if byte & 0x80 == 0 {
            return (index + 1 == bytes.len()).then_some(value as i32);
        }
    }
    None
}

fn encode_durability(mut value: i32) -> Vec<u8> {
    let mut bytes = Vec::new();
    while value >= 0x80 {
        bytes.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    bytes.push(value as u8);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breaks_armor_only_when_remaining_durability_would_be_negative() {
        assert_eq!(damage_armor(149, 250, 100), Some(249));
        assert_eq!(damage_armor(150, 250, 100), Some(250));
        assert_eq!(damage_armor(151, 250, 100), None);
        assert_eq!(damage_armor(i32::MAX, i32::MAX, 1), None);
    }

    #[test]
    fn encodes_durability_components_as_varints() {
        assert_eq!(encode_durability(400), [0x90, 0x03]);
        for value in [0, 100, 127, 128, 400, i32::MAX] {
            assert_eq!(decode_durability(&encode_durability(value)), Some(value));
        }
        assert_eq!(decode_durability(&[0x80]), None);
        assert_eq!(decode_durability(&[0xff, 0xff, 0xff, 0xff, 0x08]), None);
    }
}
