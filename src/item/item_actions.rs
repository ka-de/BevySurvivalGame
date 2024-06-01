use std::marker::PhantomData;

use crate::{
    attributes::{
        hunger::Hunger,
        modifiers::{ModifyHealthEvent, ModifyManaEvent},
    },
    inputs::CursorPos,
    inventory::Inventory,
    juice::UseItemEvent,
    player::MovePlayerEvent,
    proto::proto_param::ProtoParam,
    ui::{ChestContainer, FurnaceContainer, InventoryState},
    world::{
        dimension::DimensionSpawnEvent,
        world_helpers::{can_object_be_placed_here, world_pos_to_tile_pos},
    },
    GameParam,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_proto::prelude::{ReflectSchematic, Schematic};

use super::{CraftingTracker, PlaceItemEvent, Recipes, WorldObject};

#[derive(Component, Reflect, FromReflect, Clone, Schematic, Default, PartialEq)]
#[reflect(Component, Schematic)]
pub enum ItemAction {
    #[default]
    None,
    ModifyHealth(i32),
    ModifyMana(i32),
    TeleportHome,
    PlacesInto(WorldObject),
    Eat(i8),
}

#[derive(Component, Reflect, FromReflect, Schematic, Default)]
#[reflect(Component, Schematic)]
pub struct ItemActions {
    pub actions: Vec<ItemAction>,
}
#[derive(Component, Reflect, FromReflect, Schematic, Default)]
#[reflect(Component, Schematic)]
pub struct ManaCost(pub i32);
#[derive(Component, Reflect, FromReflect, Schematic, Default)]
#[reflect(Component, Schematic)]
pub struct ConsumableItem;

pub struct ActionSuccessEvent {
    pub obj: WorldObject,
}
#[derive(SystemParam)]
pub struct ItemActionParam<'w, 's> {
    pub move_player_event: EventWriter<'w, MovePlayerEvent>,
    pub use_item_event: EventWriter<'w, UseItemEvent>,
    pub modify_health_event: EventWriter<'w, ModifyHealthEvent>,
    pub dim_event: EventWriter<'w, DimensionSpawnEvent>,
    pub modify_mana_event: EventWriter<'w, ModifyManaEvent>,
    pub place_item_event: EventWriter<'w, PlaceItemEvent>,
    pub action_success_event: EventWriter<'w, ActionSuccessEvent>,
    pub cursor_pos: Res<'w, CursorPos>,
    pub hunger_query: Query<'w, 's, &'static mut Hunger>,
    pub chest_query: Query<'w, 's, &'static ChestContainer>,
    pub furnace_query: Query<'w, 's, &'static FurnaceContainer>,
    pub crafting_tracker: Res<'w, CraftingTracker>,
    pub recipes: Res<'w, Recipes>,

    #[system_param(ignore)]
    marker: PhantomData<&'s ()>,
}

impl ItemActions {
    pub fn run_action(
        &self,
        obj: WorldObject,
        item_action_param: &mut ItemActionParam,
        game: &mut GameParam,
        proto_param: &ProtoParam,
    ) {
        for action in &self.actions {
            match action {
                ItemAction::ModifyHealth(delta) => {
                    item_action_param
                        .modify_health_event
                        .send(ModifyHealthEvent(*delta));
                    item_action_param.use_item_event.send(UseItemEvent(obj));
                }
                ItemAction::ModifyMana(delta) => {
                    item_action_param
                        .modify_mana_event
                        .send(ModifyManaEvent(*delta));
                    item_action_param.use_item_event.send(UseItemEvent(obj));
                }
                ItemAction::TeleportHome => {
                    if let Some(pos) = game.game.home_pos {
                        item_action_param
                            .move_player_event
                            .send(MovePlayerEvent { pos });
                        item_action_param.use_item_event.send(UseItemEvent(obj));
                    }
                }
                ItemAction::PlacesInto(obj) => {
                    let pos = item_action_param.cursor_pos.world_coords.truncate();
                    if game.player().position.truncate().distance(pos)
                        > game.player().reach_distance * 32.
                    {
                        return;
                    }
                    if !can_object_be_placed_here(
                        world_pos_to_tile_pos(pos),
                        game,
                        *obj,
                        &proto_param,
                    ) {
                        return;
                    }
                    item_action_param.place_item_event.send(PlaceItemEvent {
                        obj: *obj,
                        pos,
                        placed_by_player: true,
                        override_existing_obj: false,
                    });
                }
                ItemAction::Eat(delta) => {
                    for mut hunger in item_action_param.hunger_query.iter_mut() {
                        hunger.modify_hunger(*delta);
                    }
                    item_action_param.use_item_event.send(UseItemEvent(obj));
                }
                _ => {}
            }
        }

        item_action_param
            .action_success_event
            .send(ActionSuccessEvent { obj });
    }
}

pub fn handle_item_action_success(
    mut success_events: EventReader<ActionSuccessEvent>,
    mut inv: Query<&mut Inventory>,
    inv_state: Res<InventoryState>,
    proto_param: ProtoParam,
) {
    for e in success_events.iter() {
        if proto_param
            .get_component::<ConsumableItem, _>(e.obj)
            .is_some()
        {
            let hotbar_slot = inv_state.active_hotbar_slot;
            let held_item_option = inv.single().items.items[hotbar_slot].clone();
            inv.single_mut().items.items[hotbar_slot] = held_item_option.unwrap().modify_count(-1);
        }
    }
}
