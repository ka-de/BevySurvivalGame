use item_abilities::{ add_ability_to_item_drops, handle_item_abilitiy_on_attack };
use rand::Rng;
use serde::{ Deserialize, Serialize };
use std::ops::{ Range, RangeInclusive };

use bevy::{ ecs::system::EntityCommands, prelude::* };
use bevy_proto::prelude::{ ReflectSchematic, Schematic };
pub mod health_regen;
pub mod modifiers;
use crate::{
    animations::AnimatedTextureMaterial,
    attributes::attribute_helpers::{ build_item_stack_with_parsed_attributes, get_rarity_rng },
    client::GameOverEvent,
    colors::{ LIGHT_BLUE, LIGHT_GREEN, LIGHT_GREY, LIGHT_RED },
    inventory::{ Inventory, ItemStack },
    item::{ Equipment, EquipmentType },
    player::{ stats::PlayerStats, Limb },
    proto::proto_param::ProtoParam,
    ui::{
        DropOnSlotEvent,
        InventoryState,
        RemoveFromSlotEvent,
        ShowInvPlayerStatsEvent,
        UIElement,
    },
    CustomFlush,
    GameParam,
    GameState,
    Player,
};
use modifiers::*;
pub mod attribute_helpers;
pub mod hunger;
use hunger::*;
pub mod item_abilities;

use self::health_regen::{ handle_health_regen, handle_mana_regen };
pub struct AttributesPlugin;

#[derive(Resource, Reflect, Default, Bundle)]
pub struct BlockAttributeBundle {
    pub health: CurrentHealth,
}
#[derive(
    Component,
    PartialEq,
    Clone,
    Reflect,
    FromReflect,
    Schematic,
    Default,
    Debug,
    Serialize,
    Deserialize
)]
#[reflect(Schematic, Default)]
pub struct ItemAttributes {
    pub health: i32,
    pub attack: i32,
    pub durability: i32,
    pub max_durability: i32,
    pub attack_cooldown: f32,
    pub invincibility_cooldown: f32,
    pub crit_chance: i32,
    pub crit_damage: i32,
    pub bonus_damage: i32,
    pub health_regen: i32,
    pub healing: i32,
    pub thorns: i32,
    pub dodge: i32,
    pub speed: i32,
    pub lifesteal: i32,
    pub defense: i32,
    pub xp_rate: i32,
    pub loot_rate: i32,
}

impl ItemAttributes {
    pub fn get_tooltips(&self) -> Vec<String> {
        let mut tooltips: Vec<String> = vec![];
        let is_positive = |val: i32| val > 0;
        if self.health != 0 {
            tooltips.push(
                format!("{}{} HP", if is_positive(self.health) { "+" } else { "" }, self.health)
            );
        }
        if self.defense != 0 {
            tooltips.push(
                format!(
                    "{}{} Defense",
                    if is_positive(self.defense) {
                        "+"
                    } else {
                        ""
                    },
                    self.defense
                )
            );
        }
        if self.attack != 0 {
            tooltips.push(
                format!("{}{} DMG", if is_positive(self.attack) { "+" } else { "" }, self.attack)
            );
        }
        if self.attack_cooldown != 0.0 {
            tooltips.push(format!("{:.2} Hits/s", 1.0 / self.attack_cooldown));
        }
        if self.crit_chance != 0 {
            tooltips.push(
                format!(
                    "{}{}% Crit",
                    if is_positive(self.crit_chance) {
                        "+"
                    } else {
                        ""
                    },
                    self.crit_chance
                )
            );
        }
        if self.crit_damage != 0 {
            tooltips.push(
                format!(
                    "{}{}% Crit DMG",
                    if is_positive(self.crit_damage) {
                        "+"
                    } else {
                        ""
                    },
                    self.crit_damage
                )
            );
        }
        if self.bonus_damage != 0 {
            tooltips.push(
                format!(
                    "{}{} DMG",
                    if is_positive(self.bonus_damage) {
                        "+"
                    } else {
                        ""
                    },
                    self.bonus_damage
                )
            );
        }
        if self.health_regen != 0 {
            tooltips.push(
                format!(
                    "{}{} HP Regen",
                    if is_positive(self.health_regen) {
                        "+"
                    } else {
                        ""
                    },
                    self.health_regen
                )
            );
        }
        if self.healing != 0 {
            tooltips.push(
                format!(
                    "{}{}% Healing",
                    if is_positive(self.healing) {
                        "+"
                    } else {
                        ""
                    },
                    self.healing
                )
            );
        }
        if self.thorns != 0 {
            tooltips.push(
                format!(
                    "{}{}% Thorns",
                    if is_positive(self.thorns) {
                        "+"
                    } else {
                        ""
                    },
                    self.thorns
                )
            );
        }
        if self.dodge != 0 {
            tooltips.push(
                format!("{}{}% Dodge", if is_positive(self.dodge) { "+" } else { "" }, self.dodge)
            );
        }
        if self.speed != 0 {
            tooltips.push(
                format!("{}{}% Speed", if is_positive(self.speed) { "+" } else { "" }, self.speed)
            );
        }
        if self.lifesteal != 0 {
            tooltips.push(
                format!(
                    "{}{} Lifesteal",
                    if is_positive(self.lifesteal) {
                        "+"
                    } else {
                        ""
                    },
                    self.lifesteal
                )
            );
        }

        if self.xp_rate != 0 {
            tooltips.push(
                format!("{}{}% XP", if is_positive(self.xp_rate) { "+" } else { "" }, self.xp_rate)
            );
        }
        if self.loot_rate != 0 {
            tooltips.push(
                format!(
                    "{}{}% Loot",
                    if is_positive(self.loot_rate) {
                        "+"
                    } else {
                        ""
                    },
                    self.loot_rate
                )
            );
        }

        tooltips
    }
    pub fn get_stats_summary(&self) -> Vec<(String, String)> {
        let mut tooltips: Vec<(String, String)> = vec![];
        tooltips.push(("HP:       ".to_string(), format!("{}", self.health)));
        tooltips.push(("Att:      ".to_string(), format!("{}", self.attack + self.bonus_damage)));
        tooltips.push(("Defense:  ".to_string(), format!("{}", self.defense)));
        tooltips.push(("Crit:     ".to_string(), format!("{}", self.crit_chance)));
        tooltips.push(("Crit DMG: ".to_string(), format!("{}", self.crit_damage)));
        tooltips.push(("HP Regen: ".to_string(), format!("{}", self.health_regen)));
        tooltips.push(("Healing:  ".to_string(), format!("{}", self.healing)));
        tooltips.push(("Thorns:   ".to_string(), format!("{}", self.thorns)));
        tooltips.push(("Dodge:    ".to_string(), format!("{}", self.dodge)));
        tooltips.push(("Speed:    ".to_string(), format!("{}", self.speed)));
        tooltips.push(("Leech:    ".to_string(), format!("{}", self.lifesteal)));

        // tooltips.push(format!("XP: {}", self.xp_rate));
        // tooltips.push(format!("Loot: {}", self.loot_rate));

        tooltips
    }
    pub fn get_durability_tooltip(&self) -> String {
        format!("{}/{}", self.durability, self.max_durability)
    }
    pub fn add_attribute_components(&self, entity: &mut EntityCommands) {
        if self.health > 0 {
            entity.insert(MaxHealth(self.health));
        }
        if self.attack_cooldown > 0.0 {
            entity.insert(AttackCooldown(self.attack_cooldown));
        } else {
            entity.remove::<AttackCooldown>();
        }
        entity.insert(Attack(self.attack));
        entity.insert(CritChance(self.crit_chance));
        entity.insert(CritDamage(self.crit_damage));
        entity.insert(BonusDamage(self.bonus_damage));
        entity.insert(HealthRegen(self.health_regen));
        entity.insert(Healing(self.healing));
        entity.insert(Thorns(self.thorns));
        entity.insert(Dodge(self.dodge));
        entity.insert(Speed(self.speed));
        entity.insert(Lifesteal(self.lifesteal));
        entity.insert(Defense(self.defense));
        entity.insert(XpRateBonus(self.xp_rate));
        entity.insert(LootRateBonus(self.loot_rate));
    }
    pub fn change_attribute(&mut self, modifier: AttributeModifier) -> &Self {
        match modifier.modifier.as_str() {
            "health" => {
                self.health += modifier.delta;
            }
            "attack" => {
                self.attack += modifier.delta;
            }
            "durability" => {
                self.durability += modifier.delta;
            }
            "max_durability" => {
                self.max_durability += modifier.delta;
            }
            "attack_cooldown" => {
                self.attack_cooldown += modifier.delta as f32;
            }
            "invincibility_cooldown" => {
                self.invincibility_cooldown += modifier.delta as f32;
            }
            _ => warn!("Got an unexpected attribute: {:?}", modifier.modifier),
        }
        self
    }
    pub fn combine(&self, other: &ItemAttributes) -> ItemAttributes {
        ItemAttributes {
            health: self.health + other.health,
            attack: self.attack + other.attack,
            durability: self.durability + other.durability,
            max_durability: self.max_durability + other.max_durability,
            attack_cooldown: self.attack_cooldown + other.attack_cooldown,
            invincibility_cooldown: self.invincibility_cooldown + other.invincibility_cooldown,
            crit_chance: self.crit_chance + other.crit_chance,
            crit_damage: self.crit_damage + other.crit_damage,
            bonus_damage: self.bonus_damage + other.bonus_damage,
            health_regen: self.health_regen + other.health_regen,
            healing: self.healing + other.healing,
            thorns: self.thorns + other.thorns,
            dodge: self.dodge + other.dodge,
            speed: self.speed + other.speed,
            lifesteal: self.lifesteal + other.lifesteal,
            defense: self.defense + other.defense,
            xp_rate: self.xp_rate + other.xp_rate,
            loot_rate: self.loot_rate + other.loot_rate,
        }
    }
}
macro_rules! setup_raw_bonus_attributes {
    (struct $name:ident { $($field_name:ident: $field_type:ty,)* }) => {
        #[derive(Component, PartialEq, Clone, Reflect, FromReflect, Schematic, Default, Debug)]
        #[reflect(Schematic, Default)]
        pub struct $name {
            pub $($field_name: $field_type,)*
        }

        impl $name {

            pub fn into_item_attributes(
                &self,
                rarity: ItemRarity,
                item_type: &EquipmentType
            ) -> ItemAttributes {
                // take fields of Range<i32> into one i32
                let mut rng = rand::thread_rng();
                let num_bonus_attributes = rarity.get_num_bonus_attributes(item_type);
                let num_attributes = rng.gen_range(num_bonus_attributes);
                let mut item_attributes = ItemAttributes::default();
                let valid_attributes = {
                    let mut v = Vec::new();
                    $(
                        if self.$field_name.is_some() {
                            v.push(stringify!($field_name))
                        }
                    )*
                    v
                };
                let num_valid_attributes = valid_attributes.len();
                let mut already_picked_attributes = Vec::new();
                for _ in 0..num_attributes {
                    let picked_attribute_index = rng.gen_range(0..num_valid_attributes);
                    let mut picked_attribute = valid_attributes[picked_attribute_index];
                    while already_picked_attributes.contains(&picked_attribute) {
                        let picked_attribute_index = rng.gen_range(0..num_valid_attributes);
                        picked_attribute = valid_attributes[picked_attribute_index];
                    }
                    already_picked_attributes.push(picked_attribute);
                    $(
                        {
                            if stringify!($field_name) == picked_attribute {
                                let value = rng.gen_range(self.$field_name.clone().unwrap());
                                item_attributes.$field_name = value + rarity.get_rarity_attributes_bonus();
                            }
                        }
                    )*
                }

                item_attributes
            }
        }
    };
}
macro_rules! setup_raw_base_attributes {
    (struct $name:ident { $($field_name:ident: $field_type:ty,)* }) => {
        #[derive(Component, PartialEq, Clone, Reflect, FromReflect, Schematic, Default, Debug)]
        #[reflect(Schematic, Default)]
        pub struct $name {
            pub $($field_name: $field_type,)*
        }

        impl $name {

            pub fn into_item_attributes(
                &self,
                attack_cooldown: f32,
            ) -> ItemAttributes {
                // take pick an i32 attribute value from fields of Range<i32>
                let mut rng = rand::thread_rng();
                let mut item_attributes = ItemAttributes{ attack_cooldown, ..default()};
                let valid_attributes = {
                    let mut v = Vec::new();
                    $(
                        if self.$field_name.is_some() {
                            v.push(stringify!($field_name))
                        }
                    )*
                    v
                };
                for att in valid_attributes.iter() {
                    $(
                        {
                            if stringify!($field_name) == *att {
                                let value = rng.gen_range(self.$field_name.clone().unwrap());
                                item_attributes.$field_name = value;
                            }
                        }
                    )*
                }

                item_attributes
            }
        }
    };
}

setup_raw_bonus_attributes! {
    struct RawItemBonusAttributes {
        attack: Option<Range<i32>>,
        health: Option<Range<i32>>,
        defense: Option<Range<i32>>,
        durability: Option<Range<i32>>,
        max_durability: Option<Range<i32>>,
        //
        crit_chance: Option<Range<i32>>,
        crit_damage: Option<Range<i32>>,
        bonus_damage: Option<Range<i32>>,
        health_regen: Option<Range<i32>>,
        healing: Option<Range<i32>>,
        thorns: Option<Range<i32>>,
        dodge: Option<Range<i32>>,
        speed: Option<Range<i32>>,
        lifesteal: Option<Range<i32>>,
        xp_rate: Option<Range<i32>>,
        loot_rate: Option<Range<i32>>,
    }
}

setup_raw_base_attributes! {
    struct RawItemBaseAttributes {
        attack: Option<Range<i32>>,
        health: Option<Range<i32>>,
        defense: Option<Range<i32>>,
        durability: Option<Range<i32>>,
        max_durability: Option<Range<i32>>,
        //
        crit_chance: Option<Range<i32>>,
        crit_damage: Option<Range<i32>>,
        bonus_damage: Option<Range<i32>>,
        health_regen: Option<Range<i32>>,
        healing: Option<Range<i32>>,
        thorns: Option<Range<i32>>,
        dodge: Option<Range<i32>>,
        speed: Option<Range<i32>>,
        lifesteal: Option<Range<i32>>,
        xp_rate: Option<Range<i32>>,
        loot_rate: Option<Range<i32>>,
    }
}

#[derive(
    Component,
    Reflect,
    FromReflect,
    Debug,
    Schematic,
    Clone,
    Default,
    Eq,
    PartialEq,
    Serialize,
    Deserialize
)]
#[reflect(Component, Schematic)]
pub enum ItemRarity {
    #[default]
    Common,
    Uncommon,
    Rare,
    Legendary,
}

impl ItemRarity {
    fn get_num_bonus_attributes(&self, eqp_type: &EquipmentType) -> RangeInclusive<i32> {
        let acc_offset = if eqp_type.is_accessory() { 1 } else { 0 };
        match self {
            ItemRarity::Common => 0 + acc_offset..=1 + acc_offset,
            ItemRarity::Uncommon => 1 + acc_offset..=2 + acc_offset,
            ItemRarity::Rare => 2 + acc_offset..=3 + acc_offset,
            ItemRarity::Legendary => 4 + acc_offset..=5 + acc_offset,
        }
    }
    fn get_rarity_attributes_bonus(&self) -> i32 {
        match self {
            ItemRarity::Common => 1,
            ItemRarity::Uncommon => 2,
            ItemRarity::Rare => 3,
            ItemRarity::Legendary => 5,
        }
    }

    pub fn get_tooltip_ui_element(&self) -> UIElement {
        match self {
            ItemRarity::Common => UIElement::LargeTooltipCommon,
            ItemRarity::Uncommon => UIElement::LargeTooltipUncommon,
            ItemRarity::Rare => UIElement::LargeTooltipRare,
            ItemRarity::Legendary => UIElement::LargeTooltipLegendary,
        }
    }
    pub fn get_color(&self) -> Color {
        match self {
            ItemRarity::Common => LIGHT_GREY,
            ItemRarity::Uncommon => LIGHT_GREEN,
            ItemRarity::Rare => LIGHT_BLUE,
            ItemRarity::Legendary => LIGHT_RED,
        }
    }
    pub fn get_next_rarity(&self) -> ItemRarity {
        match self {
            ItemRarity::Common => ItemRarity::Uncommon,
            ItemRarity::Uncommon => ItemRarity::Rare,
            ItemRarity::Rare => ItemRarity::Legendary,
            ItemRarity::Legendary => ItemRarity::Legendary,
        }
    }
}

#[derive(Reflect, FromReflect, Default, Component, Clone, Debug, Copy)]
#[reflect(Component)]
pub struct ItemLevel(pub u8);

pub struct AttributeModifier {
    pub modifier: String,
    pub delta: i32,
}

#[derive(Debug, Clone, Default)]
pub struct AttributeChangeEvent;

#[derive(Bundle, Clone, Debug, Copy, Default)]
pub struct PlayerAttributeBundle {
    pub health: MaxHealth,
    pub mana: Mana,
    pub attack: Attack,
    pub attack_cooldown: AttackCooldown,
    pub defense: Defense,
    pub crit_chance: CritChance,
    pub crit_damage: CritDamage,
    pub bonus_damage: BonusDamage,
    pub health_regen: HealthRegen,
    pub healing: Healing,
    pub thorns: Thorns,
    pub dodge: Dodge,
    pub speed: Speed,
    pub lifesteal: Lifesteal,
    pub xp_rate: XpRateBonus,
    pub mana_regen: ManaRegen,
    pub loot_rate: LootRateBonus,
}

//TODO: Add max health vs curr health
#[derive(
    Reflect,
    FromReflect,
    Default,
    Schematic,
    Component,
    Clone,
    Debug,
    Copy,
    Serialize,
    Deserialize
)]
#[reflect(Component, Schematic)]
pub struct CurrentHealth(pub i32);
#[derive(Reflect, FromReflect, Default, Schematic, Component, Clone, Debug, Copy)]
#[reflect(Component, Schematic)]
pub struct Mana {
    pub max: i32,
    pub current: i32,
}
impl Mana {
    pub fn new(max: i32) -> Self {
        Self { max, current: max }
    }
}

#[derive(Reflect, FromReflect, Default, Schematic, Component, Clone, Debug, Copy)]
#[reflect(Component, Schematic)]
pub struct MaxHealth(pub i32);
#[derive(Reflect, FromReflect, Default, Schematic, Component, Clone, Debug, Copy)]
#[reflect(Component, Schematic)]
pub struct Attack(pub i32);
#[derive(Reflect, FromReflect, Default, Component, Clone, Debug, Copy)]
#[reflect(Component)]
pub struct Durability(pub i32);

#[derive(Reflect, FromReflect, Default, Component, Clone, Debug, Copy)]
#[reflect(Component)]
pub struct AttackCooldown(pub f32);
#[derive(Reflect, FromReflect, Default, Component, Clone, Debug, Copy)]
#[reflect(Component)]
pub struct InvincibilityCooldown(pub f32);

#[derive(Default, Component, Clone, Debug, Copy)]
pub struct CritChance(pub i32);
#[derive(Default, Component, Clone, Debug, Copy)]
pub struct CritDamage(pub i32);
#[derive(Default, Component, Clone, Debug, Copy)]
pub struct BonusDamage(pub i32);
#[derive(Default, Component, Clone, Debug, Copy)]
pub struct HealthRegen(pub i32);
#[derive(Default, Component, Clone, Debug, Copy)]
pub struct Healing(pub i32);
#[derive(Default, Component, Clone, Debug, Copy)]
pub struct Thorns(pub i32);
#[derive(Default, Component, Clone, Debug, Copy)]
pub struct Dodge(pub i32);
#[derive(Default, Component, Clone, Debug, Copy)]
pub struct Speed(pub i32);
#[derive(Default, Component, Clone, Debug, Copy)]
pub struct Lifesteal(pub i32);
#[derive(Default, Component, Clone, Debug, Copy)]
pub struct Defense(pub i32);
#[derive(Default, Component, Clone, Debug, Copy)]
pub struct XpRateBonus(pub i32);
#[derive(Default, Component, Clone, Debug, Copy)]
pub struct LootRateBonus(pub i32);

#[derive(Reflect, FromReflect, Default, Schematic, Component, Clone, Debug, Copy)]
#[reflect(Component, Schematic)]
pub struct ManaRegen(pub i32);

impl Plugin for AttributesPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<AttributeChangeEvent>()
            .add_event::<ModifyHealthEvent>()
            .add_event::<ModifyManaEvent>()
            .add_systems(
                (
                    clamp_health,
                    clamp_mana,
                    handle_actions_drain_hunger,
                    tick_hunger,
                    handle_modify_health_event.before(clamp_health),
                    handle_modify_mana_event.before(clamp_mana),
                    add_current_health_with_max_health,
                    handle_health_regen,
                    handle_mana_regen,
                    update_attributes_with_held_item_change,
                    update_attributes_and_sprite_with_equipment_change,
                    update_sprite_with_equipment_removed,
                    handle_item_abilitiy_on_attack,
                    handle_new_items_raw_attributes.before(CustomFlush),
                    handle_player_item_attribute_change_events.after(CustomFlush),
                ).in_set(OnUpdate(GameState::Main))
            );
    }
}

fn clamp_health(
    mut health: Query<(&mut CurrentHealth, &MaxHealth), With<Player>>,
    mut game_over_event: EventWriter<GameOverEvent>
) {
    for (mut h, max_h) in health.iter_mut() {
        if h.0 <= 0 {
            h.0 = 0;
            game_over_event.send_default();
        } else if h.0 > max_h.0 {
            h.0 = max_h.0;
        }
    }
}
fn clamp_mana(mut health: Query<&mut Mana, With<Player>>) {
    for mut m in health.iter_mut() {
        if m.current < 0 {
            m.current = 0;
        } else if m.current > m.max {
            m.current = m.max;
        }
    }
}
fn handle_player_item_attribute_change_events(
    mut commands: Commands,
    player: Query<(Entity, &Inventory, &PlayerStats), With<Player>>,
    eqp_attributes: Query<&ItemAttributes, With<Equipment>>,
    mut att_events: EventReader<AttributeChangeEvent>,
    mut stats_event: EventWriter<ShowInvPlayerStatsEvent>,
    player_atts: Query<&ItemAttributes, With<Player>>
) {
    for _event in att_events.iter() {
        let mut new_att = player_atts.single().clone();
        let (player, inv, stats) = player.single();
        let equips: Vec<ItemAttributes> = inv.equipment_items.items
            .iter()
            .chain(inv.accessory_items.items.iter())
            .flatten()
            .map(|e| e.item_stack.attributes.clone())
            .collect();

        for a in eqp_attributes.iter().chain(equips.iter()) {
            new_att = new_att.combine(a);
        }
        if new_att.attack_cooldown == 0.0 {
            new_att.attack_cooldown = 0.4;
        }
        new_att = stats.apply_stats_to_player_attributes(new_att.clone());
        new_att.add_attribute_components(&mut commands.entity(player));
        stats_event.send(ShowInvPlayerStatsEvent);
    }
}

/// Adds a current health component to all entities with a max health component
pub fn add_current_health_with_max_health(
    mut commands: Commands,
    mut health: Query<(Entity, &MaxHealth), (Changed<MaxHealth>, Without<CurrentHealth>)>
) {
    for (entity, max_health) in health.iter_mut() {
        commands.entity(entity).insert(CurrentHealth(max_health.0));
    }
}

///Tracks player held item changes, spawns new held item entity and updates player attributes
fn update_attributes_with_held_item_change(
    mut commands: Commands,
    mut game_param: GameParam,
    inv_state: Res<InventoryState>,
    mut inv: Query<&mut Inventory>,
    item_stack_query: Query<&ItemAttributes>,
    mut att_event: EventWriter<AttributeChangeEvent>,
    proto: ProtoParam
) {
    let active_hotbar_slot = inv_state.active_hotbar_slot;
    let active_hotbar_item = inv.single_mut().items.items[active_hotbar_slot].clone();
    let player_data = game_param.player_mut();
    let prev_held_item_data = &player_data.main_hand_slot;
    if let Some(new_item) = active_hotbar_item {
        let new_item_stack = new_item.item_stack.clone();
        if let Some(current_item) = prev_held_item_data {
            let curr_attributes = item_stack_query.get(current_item.entity).unwrap();
            let new_attributes = &new_item.item_stack.attributes;
            if new_item_stack != current_item.item_stack {
                new_item.spawn_item_on_hand(&mut commands, &mut game_param, &proto);
                att_event.send(AttributeChangeEvent);
            } else if curr_attributes != new_attributes {
                commands.entity(current_item.entity).insert(new_attributes.clone());
                att_event.send(AttributeChangeEvent);
            }
        } else {
            new_item.spawn_item_on_hand(&mut commands, &mut game_param, &proto);
            att_event.send(AttributeChangeEvent);
        }
    } else if let Some(current_item) = prev_held_item_data {
        commands.entity(current_item.entity).despawn();
        player_data.main_hand_slot = None;
        att_event.send(AttributeChangeEvent);
    }
}
///Tracks player equip or accessory inventory slot changes,
///spawns new held equipment entity, and updates player attributes
fn update_attributes_and_sprite_with_equipment_change(
    player_limbs: Query<(&mut Handle<AnimatedTextureMaterial>, &Limb)>,
    asset_server: Res<AssetServer>,
    proto_param: ProtoParam,
    mut materials: ResMut<Assets<AnimatedTextureMaterial>>,
    mut att_event: EventWriter<AttributeChangeEvent>,
    mut events: EventReader<DropOnSlotEvent>
) {
    for drop in events.iter() {
        if
            drop.drop_target_slot_state.r#type.is_equipment() ||
            drop.drop_target_slot_state.r#type.is_accessory()
        {
            let slot = drop.drop_target_slot_state.slot_index;
            let Some(eqp_type) = proto_param.get_component::<EquipmentType, _>(
                drop.dropped_item_stack.obj_type
            ) else {
                continue;
            };
            if !eqp_type.is_equipment() || !eqp_type.get_valid_slots().contains(&slot) {
                continue;
            }
            att_event.send(AttributeChangeEvent);
            if drop.drop_target_slot_state.r#type.is_equipment() {
                for (mat, limb) in player_limbs.iter() {
                    if Limb::from_slot(slot).contains(limb) {
                        let mat = materials.get_mut(mat).unwrap();
                        let armor_texture_handle = asset_server.load(
                            format!(
                                "textures/player/{}.png",
                                drop.dropped_item_stack.obj_type.to_string()
                            )
                        );
                        mat.lookup_texture = Some(armor_texture_handle);
                    }
                }
            }
        }
    }
}
///Tracks player equip or accessory inventory slot changes,
///spawns new held equipment entity, and updates player attributes
fn update_sprite_with_equipment_removed(
    mut removed_inv_item: EventReader<RemoveFromSlotEvent>,
    player_limbs: Query<(&mut Handle<AnimatedTextureMaterial>, &Limb)>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<AnimatedTextureMaterial>>
) {
    for item in removed_inv_item.iter() {
        if item.removed_slot_state.r#type.is_equipment() {
            for (mat, limb) in player_limbs.iter() {
                if Limb::from_slot(item.removed_slot_state.slot_index).contains(limb) {
                    let mat = materials.get_mut(mat).unwrap();
                    let armor_texture_handle = asset_server.load(
                        format!("textures/player/player-texture-{}.png", if
                            limb == &Limb::Torso ||
                            limb == &Limb::Hands
                        {
                            Limb::Torso.to_string().to_lowercase()
                        } else {
                            limb.to_string().to_lowercase()
                        })
                    );
                    mat.lookup_texture = Some(armor_texture_handle);
                }
            }
        }
    }
}
fn handle_new_items_raw_attributes(
    mut commands: Commands,
    new_items: Query<
        (
            Entity,
            &ItemStack,
            Option<&RawItemBonusAttributes>,
            &RawItemBaseAttributes,
            &EquipmentType,
            Option<&ItemLevel>,
        ),
        Or<(Added<RawItemBaseAttributes>, Added<RawItemBonusAttributes>)>
    >
) {
    for (e, stack, raw_bonus_att_option, raw_base_att, eqp_type, item_level) in new_items.iter() {
        let rarity = get_rarity_rng(rand::thread_rng());
        let mut new_stack = build_item_stack_with_parsed_attributes(
            stack,
            raw_base_att,
            raw_bonus_att_option,
            rarity,
            eqp_type,
            item_level.map(|l| l.0)
        );
        if new_stack.obj_type.is_weapon() {
            add_ability_to_item_drops(&mut new_stack);
        }
        commands.entity(e).insert(new_stack);
    }
}
