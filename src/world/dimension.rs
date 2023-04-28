use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilemapTexture;

use crate::{
    enemy::Enemy,
    item::{Equipment, WorldObject},
    WorldGeneration,
};

use super::ChunkManager;

#[derive(Component, Debug)]
pub struct Dimension;
impl Dimension {}

#[derive(Component, Debug)]
pub struct GenerationSeed {
    pub seed: u32,
}

#[derive(Component, Debug)]
pub struct SpawnDimension;
pub struct DimensionSpawnEvent {
    pub generation_params: WorldGeneration,
    pub seed: Option<u32>,
    pub swap_to_dim_now: bool,
}
#[derive(Component)]

pub struct ActiveDimension;
pub struct DimensionPlugin;

impl Plugin for DimensionPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ChunkManager::new())
            .add_event::<DimensionSpawnEvent>()
            .add_system_to_stage(
                CoreStage::PreUpdate,
                Self::handle_dimension_swap_events.after(Self::new_dim_with_params),
            )
            .add_system_to_stage(CoreStage::PreUpdate, Self::new_dim_with_params);
    }
}
impl DimensionPlugin {
    ///spawns the initial world dimension entity
    pub fn new_dim_with_params(
        mut commands: Commands,
        mut spawn_event: EventReader<DimensionSpawnEvent>,
    ) {
        for new_dim in spawn_event.iter() {
            println!("SPAWNING NEW DIMENSION");
            let mut cm = ChunkManager::new();
            cm.world_generation_params = new_dim.generation_params.clone();
            let dim_e = commands
                .spawn((
                    Dimension,
                    GenerationSeed {
                        seed: new_dim.seed.unwrap_or(0),
                    },
                    cm,
                ))
                .id();
            if new_dim.swap_to_dim_now {
                commands.entity(dim_e).insert(SpawnDimension);
            }
        }
    }
    pub fn handle_dimension_swap_events(
        new_dim: Query<Entity, Added<SpawnDimension>>,
        mut commands: Commands,
        entity_query: Query<
            Entity,
            (
                Or<(With<WorldObject>, With<Enemy>, With<TilemapTexture>)>,
                Without<Equipment>,
            ),
        >,
        old_dim: Query<Entity, With<ActiveDimension>>,
        cm: Query<&ChunkManager>,
        old_cm: Res<ChunkManager>,
    ) {
        // event sent out when we enter a new dimension
        for d in new_dim.iter() {
            //despawn all entities with positions, except the player
            println!("DESPAWNING EVERYTHING!!! {:?}", entity_query.iter().len());
            for e in entity_query.iter() {
                commands.entity(e).despawn_recursive();
            }
            // clean up old dimension, remove active tag, and update its chunk manager
            if let Ok(old_dim) = old_dim.get_single() {
                commands
                    .entity(old_dim)
                    .remove::<ActiveDimension>()
                    .insert(old_cm.clone());
            }
            println!("inserting new chunk manager/dim {:?}", cm.iter().len());
            //give the new dimension active tag, and use its chunk manager as the game resource
            commands
                .entity(d)
                .insert(ActiveDimension)
                .remove::<SpawnDimension>();
            commands.insert_resource(cm.get(d).unwrap().clone());
        }
    }
}