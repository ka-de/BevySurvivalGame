use crate::{
    animations::DoneAnimation,
    attributes::{
        modifiers::ModifyHealthEvent,
        Attack,
        Defense,
        Dodge,
        InvincibilityCooldown,
        Lifesteal,
        Thorns,
    },
    enemy::{ Mob, MobIsAttacking },
    inventory::{ Inventory, ItemStack },
    item::{
        projectile::{ EnemyProjectile, Projectile, ProjectileState },
        Equipment,
        MainHand,
        WorldObject,
    },
    ui::damage_numbers::DodgeEvent,
    CustomFlush,
    GameParam,
    GameState,
    Player,
};
use bevy::prelude::*;
use bevy_rapier2d::prelude::{ CollisionEvent, RapierContext };
use rand::Rng;

use super::{ HitEvent, HitMarker, InvincibilityTimer };
pub struct CollisionPlugion;

impl Plugin for CollisionPlugion {
    fn build(&self, app: &mut App) {
        app.add_systems(
            (
                check_melee_hit_collisions,
                check_mob_to_player_collisions,
                check_projectile_hit_mob_collisions,
                check_projectile_hit_player_collisions,
                check_item_drop_collisions.after(CustomFlush),
            ).in_set(OnUpdate(GameState::Main))
        );
    }
}

fn check_melee_hit_collisions(
    _commands: Commands,
    context: ResMut<RapierContext>,
    weapons: Query<
        (Entity, &Parent, &GlobalTransform, &WorldObject),
        (Without<HitMarker>, With<MainHand>)
    >,
    mut hit_event: EventWriter<HitEvent>,
    game: GameParam,
    world_obj: Query<Entity, (With<WorldObject>, Without<MainHand>)>,
    lifesteal: Query<&Lifesteal>,
    mut modify_health_events: EventWriter<ModifyHealthEvent>,
    mob_txfms: Query<&GlobalTransform, With<Mob>>,
    mut hit_tracker: Local<Vec<Entity>>
) {
    if !game.game.player_state.is_attacking {
        hit_tracker.clear();
    }
    if let Ok((weapon_e, weapon_parent, weapon_t, weapon_obj)) = weapons.get_single() {
        let hits_this_frame = context
            .intersection_pairs()
            .filter(|c| {
                (c.0 == weapon_e && c.1 != weapon_parent.get()) ||
                    (c.1 == weapon_e && c.0 != weapon_parent.get())
            });
        for hit in hits_this_frame {
            let hit_entity = if hit.0 == weapon_e { hit.1 } else { hit.0 };
            if
                !game.game.player_state.is_attacking ||
                world_obj.get(hit_entity).is_ok() ||
                hit_tracker.contains(&hit_entity)
            {
                continue;
            }

            hit_tracker.push(hit_entity);
            let damage = game.calculate_player_damage().0 as i32;
            let Ok(mob_txfm) = mob_txfms.get(hit_entity) else {
                continue;
            };
            let delta = weapon_t.translation() - mob_txfm.translation();
            if let Ok(lifesteal) = lifesteal.get(game.game.player) {
                modify_health_events.send(
                    ModifyHealthEvent(
                        f32::floor(((damage as f32) * (lifesteal.0 as f32)) / 100.0) as i32
                    )
                );
            }
            hit_event.send(HitEvent {
                hit_entity,
                damage,
                dir: delta.normalize_or_zero().truncate() * -1.0,
                hit_with_melee: Some(*weapon_obj),
                hit_with_projectile: None,
            });
        }
    }
}
fn check_projectile_hit_mob_collisions(
    mut commands: Commands,
    player_attack: Query<(Entity, &Children, Option<&Lifesteal>), With<Player>>,
    allowed_targets: Query<Entity, (Without<ItemStack>, Without<MainHand>, Without<Projectile>)>,
    mut hit_event: EventWriter<HitEvent>,
    mut collisions: EventReader<CollisionEvent>,
    mut projectiles: Query<
        (Entity, &mut ProjectileState, &Projectile, &Attack, Option<&DoneAnimation>),
        Without<EnemyProjectile>
    >,
    is_world_obj: Query<&WorldObject>,
    mut children: Query<&Parent>,
    mut modify_health_events: EventWriter<ModifyHealthEvent>
) {
    for evt in collisions.iter() {
        let CollisionEvent::Started(e1, e2, _) = evt else {
            continue;
        };
        for (e1, e2) in [
            (e1, e2),
            (e2, e1),
        ] {
            let (proj_entity, mut state, proj, att, anim_option) = if
                let Ok(e) = children.get_mut(*e1)
            {
                if
                    let Ok((proj_entity, state, proj, att, anim_option)) = projectiles.get_mut(
                        e.get()
                    )
                {
                    (proj_entity, state, proj, att, anim_option)
                } else {
                    continue;
                }
            } else if
                let Ok((proj_entity, state, proj, att, anim_option)) = projectiles.get_mut(*e1)
            {
                (proj_entity, state, proj, att, anim_option)
            } else {
                continue;
            };
            let Ok((player_e, children, lifesteal)) = player_attack.get_single() else {
                continue;
            };
            if player_e == *e2 || children.contains(e2) || !allowed_targets.contains(*e2) {
                continue;
            }
            if state.hit_entities.contains(e2) {
                continue;
            }
            state.hit_entities.push(*e2);
            let damage = att.0;
            if let Some(lifesteal) = lifesteal {
                if !is_world_obj.contains(*e2) {
                    modify_health_events.send(
                        ModifyHealthEvent(
                            f32::floor(((damage as f32) * (lifesteal.0 as f32)) / 100.0) as i32
                        )
                    );
                }
            }
            hit_event.send(HitEvent {
                hit_entity: *e2,
                damage,
                dir: state.direction,
                hit_with_melee: None,
                hit_with_projectile: Some(proj.clone()),
            });
            //non-animating sprites are despawned immediately
            if anim_option.is_none() {
                commands.entity(proj_entity).despawn_recursive();
            }
        }
    }
}
fn check_projectile_hit_player_collisions(
    mut commands: Commands,
    enemy_attack: Query<(Entity, &Attack), With<Mob>>,
    allowed_targets: Query<
        Option<&WorldObject>,
        (Or<(With<Player>, With<WorldObject>)>, (Without<Projectile>, Without<MainHand>))
    >,
    mut hit_event: EventWriter<HitEvent>,
    mut collisions: EventReader<CollisionEvent>,
    mut projectiles: Query<
        (
            Entity,
            &mut ProjectileState,
            Option<&DoneAnimation>,
            &Projectile,
            &Attack,
            &EnemyProjectile,
        ),
        With<EnemyProjectile>
    >,
    mut children: Query<&Parent>
) {
    for evt in collisions.iter() {
        let CollisionEvent::Started(e1, e2, _) = evt else {
            continue;
        };
        for (e1, e2) in [
            (e1, e2),
            (e2, e1),
        ] {
            let (proj_entity, mut state, anim_option, proj, att, enemy_proj) = if
                let Ok(e) = children.get_mut(*e1)
            {
                if
                    let Ok((proj_entity, state, anim_option, proj, att, enemy_proj)) =
                        projectiles.get_mut(e.get())
                {
                    (proj_entity, state, anim_option, proj, att, enemy_proj)
                } else {
                    continue;
                }
            } else if
                let Ok((proj_entity, state, anim_option, proj, att, enemy_proj)) =
                    projectiles.get_mut(*e1)
            {
                (proj_entity, state, anim_option, proj, att, enemy_proj)
            } else {
                continue;
            };
            let Ok((enemy_e, _attack)) = enemy_attack.get(enemy_proj.entity) else {
                continue;
            };
            if enemy_e == *e2 || !allowed_targets.contains(*e2) {
                continue;
            }
            if let Some(obj) = allowed_targets.get(*e2).unwrap() {
                if
                    [
                        WorldObject::Grass,
                        WorldObject::Grass2,
                        WorldObject::Grass3,
                        WorldObject::RedFlower,
                        WorldObject::PinkFlower,
                        WorldObject::YellowFlower,
                        WorldObject::RedMushroom,
                        WorldObject::BrownMushroom,
                        WorldObject::Stick,
                    ].contains(obj)
                {
                    continue;
                }
            }
            if state.hit_entities.contains(e2) {
                continue;
            }
            state.hit_entities.push(*e2);

            hit_event.send(HitEvent {
                hit_entity: *e2,
                damage: att.0,
                dir: state.direction,
                hit_with_melee: None,
                hit_with_projectile: Some(proj.clone()),
            });
            if anim_option.is_none() {
                commands.entity(proj_entity).despawn_recursive();
            }
        }
    }
}
pub fn check_item_drop_collisions(
    mut commands: Commands,
    player: Query<Entity, With<Player>>,
    allowed_targets: Query<Entity, (With<ItemStack>, Without<MainHand>, Without<Equipment>)>,
    rapier_context: Res<RapierContext>,
    items_query: Query<&ItemStack>,
    mut game: GameParam,
    mut inv: Query<&mut Inventory>
) {
    if !game.player().is_moving {
        return;
    }
    let player_e = player.single();
    for (e1, e2, _) in rapier_context.intersections_with(player_e) {
        for (e1, e2) in [
            (e1, e2),
            (e2, e1),
        ] {
            //if the player is colliding with an entity...
            let Ok(_) = player.get(e1) else {
                continue;
            };
            if !allowed_targets.contains(e2) {
                continue;
            }
            let item_stack = items_query.get(e2).unwrap().clone();
            // ...and the entity is an item stack...
            let inv_container = inv.single().items.clone();
            if
                inv_container.get_first_empty_slot().is_none() &&
                inv_container
                    .get_slot_for_item_in_container_with_space(&item_stack.obj_type, None)
                    .is_none()
            {
                return;
            }
            // ...and inventory has room, add it to the player's inventory

            item_stack.add_to_inventory(&mut inv.single_mut().items, &mut game.inv_slot_query);

            commands.entity(e2).despawn();
        }
    }
}
fn check_mob_to_player_collisions(
    mut commands: Commands,
    player: Query<
        (Entity, &Transform, &Thorns, &Defense, &Dodge, &InvincibilityCooldown),
        With<Player>
    >,
    dmg_source: Query<(&Transform, &Attack, Option<&MobIsAttacking>), Without<Player>>,
    rapier_context: Res<RapierContext>,
    mut hit_event: EventWriter<HitEvent>,
    mut dodge_event: EventWriter<DodgeEvent>,
    in_i_frame: Query<&InvincibilityTimer>
) {
    let (player_e, player_txfm, thorns, defense, dodge, i_frames) = player.single();
    let mut hit_this_frame = false;
    for (e1, e2, _) in rapier_context.intersections_with(player_e) {
        for (e1, e2) in [
            (e1, e2),
            (e2, e1),
        ] {
            if hit_this_frame {
                continue;
            }
            //if the player is colliding with an entity...
            let Ok(_) = player.get(e1) else {
                continue;
            };

            if !dmg_source.contains(e2) {
                continue;
            }
            let (mob_txfm, attack, is_attacking) = dmg_source.get(e2).unwrap();

            // mobs can only hit player during their attack animations
            if is_attacking.is_none() {
                continue;
            }

            let delta = player_txfm.translation - mob_txfm.translation;
            hit_this_frame = true;

            let mut rng = rand::thread_rng();
            if rng.gen_ratio(dodge.0.try_into().unwrap_or(0), 100) && !in_i_frame.contains(e1) {
                dodge_event.send(DodgeEvent { entity: e1 });
                commands
                    .entity(e1)
                    .insert(InvincibilityTimer(Timer::from_seconds(i_frames.0, TimerMode::Once)));
                continue;
            }
            hit_event.send(HitEvent {
                hit_entity: e1,
                damage: f32::round((attack.0 as f32) * (0.99_f32).powi(defense.0)) as i32,
                dir: delta.normalize_or_zero().truncate(),
                hit_with_melee: None,
                hit_with_projectile: None,
            });
            // hit back to attacker if we have Thorns
            if thorns.0 > 0 && in_i_frame.get(e1).is_err() {
                hit_event.send(HitEvent {
                    hit_entity: e2,
                    damage: f32::ceil(((attack.0 as f32) * (thorns.0 as f32)) / 100.0) as i32,
                    dir: delta.normalize_or_zero().truncate(),
                    hit_with_melee: None,
                    hit_with_projectile: None,
                });
            }
        }
    }
}
