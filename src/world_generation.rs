use crate::assets::Graphics;
use crate::item::WorldObject;
use crate::{Game, GameState, ImageAssets, Player, WORLD_SIZE};
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy::{math::Vec3Swizzles, utils::HashSet};
use bevy_ecs_tilemap::helpers::square_grid::neighbors::Neighbors;
use bevy_ecs_tilemap::prelude::*;
use bevy_inspector_egui::Inspectable;
use noise::{NoiseFn, Perlin, Seedable, Simplex};
use rand::rngs::ThreadRng;
use rand::Rng;
use serde::Deserialize;

pub struct WorldGenerationPlugin;

impl Plugin for WorldGenerationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ChunkManager::new()).add_system_set(
            SystemSet::on_update(GameState::Main)
                // .with_system(Self::spawn_chunk)
                .with_system(Self::spawn_chunks_around_camera), // .with_system(Self::despawn_outofrange_chunks),
        );
        // TODO: add updating code
        // .add_system_set(
        //     SystemSet::on_enter(GameState::Main)
        //         .with_system(Self::spawn_test_objects.after("graphics")),
        //     // .with_system(Self::world_object_growth),
        // );
    }
}
#[derive(Resource, Inspectable, Default)]
pub struct Data {
    pub chunk_bits: [[u8; 16]; 16],
}
#[derive(Debug, Resource)]
struct ChunkManager {
    pub spawned_chunks: HashSet<IVec2>,
    pub chunk_tile_bit_grid: HashMap<IVec2, [[u8; 16]; 16]>,
}

impl ChunkManager {
    fn new() -> Self {
        Self {
            spawned_chunks: HashSet::default(),
            chunk_tile_bit_grid: HashMap::new(),
        }
    }
}

impl WorldGenerationPlugin {
    fn spawn_chunk(
        commands: &mut Commands,
        sprite_sheet: &Res<ImageAssets>,
        game: &Res<Game>,
        chunk_pos: IVec2,
        data: &mut ResMut<Data>,
        chunk_manager: &mut ResMut<ChunkManager>,
    ) {
        let tilemap_size = TilemapSize {
            x: 16 as u32,
            y: 16 as u32,
        };
        let tile_size = TilemapTileSize { x: 32., y: 32. };
        let grid_size = tile_size.into();
        let map_type = TilemapType::default();

        let tilemap_entity = commands.spawn_empty().id();
        let mut tile_storage = TileStorage::empty(tilemap_size);

        let mut tile_bit_grid = [[0 as u8; 16]; 16];

        for y in 0..16 {
            for x in 0..16 {
                let tile_entity = commands
                    .spawn(TileBundle {
                        position: TilePos { x, y },
                        tilemap_id: TilemapId(tilemap_entity),
                        texture_index: TileTextureIndex(0b1111),
                        ..Default::default()
                    })
                    .id();
                commands.entity(tilemap_entity).add_child(tile_entity);
                tile_bit_grid[x as usize][y as usize] = 0b1111;
                tile_storage.set(&TilePos { x, y }, tile_entity);
            }
        }

        let noise_e = Perlin::new(1);
        let noise_e2 = Perlin::new(2);
        let noise_e3 = Perlin::new(3);

        let noise_m = Simplex::new(4);
        let noise_m2 = Simplex::new(5);
        let noise_m3 = Simplex::new(6);

        for y in 0..16 {
            for x in 0..16 {
                let tile_pos = TilePos {
                    x: x.try_into().unwrap(),
                    y: y.try_into().unwrap(),
                };
                let nx = (x + chunk_pos.x * 16) as f64 / 16. as f64 - 0.5;
                let ny = (y + chunk_pos.y * 16) as f64 / 16. as f64 - 0.5;
                // let e = noise_e.get([nx, ny]) + 0.5;
                let base_oct = 1. / 10.;
                let e1 = (noise_e.get([nx * base_oct, ny * base_oct]));
                let e2 = (noise_e2.get([nx * base_oct * 8., ny * base_oct * 8.]));
                let e3 = (noise_e3.get([nx * base_oct * 16., ny * base_oct * 16.]));

                let e = f64::min(e1, f64::min(e2, e3) + 0.4) + 0.5;
                let m = (noise_m.get([nx * base_oct, ny * base_oct]) + 0.5)
                    + 0.5 * (noise_m2.get([nx * base_oct * 2., ny * base_oct * 2.]) + 0.5)
                    + 0.25 * (noise_m3.get([nx * base_oct * 3., ny * base_oct * 3.]) + 0.5);

                // let e = f64::powf(e / (1. + 0.5 + 0.25), 1.);
                let m = f64::powf(m / (1. + 0.5 + 0.25), 1.);
                // print!("{:?}", e);
                let m = f64::powf(m, 1.);
                let mut block = if e <= game.world_generation_params.water_frequency {
                    WorldObject::Water
                }
                // else if e <= game.world_generation_params.sand_frequency {
                //     if m <= 0.35 {
                //         WorldObject::RedSand
                //     } else {
                //         WorldObject::Sand
                //     }
                // } else if e <= game.world_generation_params.dirt_frequency {
                //     if m > 0.6 {
                //         WorldObject::Dirt
                //     } else {
                //         WorldObject::Grass
                //     }
                // } else if e <= game.world_generation_params.stone_frequency {
                //     WorldObject::Stone
                // }
                else {
                    // if m > 0.75 {
                    //     WorldObject::DryGrass
                    // } else if m > 0.45 {
                    //     WorldObject::Grass
                    // } else {
                    WorldObject::Grass
                    // }
                };
                // if chunk_pos.x == 0 && chunk_pos.y == 0 {
                //     if y <= 8 {
                //         block = WorldObject::Grass
                //     } else {
                //         block = WorldObject::Dirt
                //     }
                // }
                let block_bits: u8 = if block == WorldObject::Grass {
                    0b0000
                } else {
                    println!("WATER BLOCK HERE: {:?}", tile_pos);
                    0b1111
                };

                // let texture_index = (graphics.item_map.get(&block).unwrap().1) as u32;
                let texture_index = block_bits as u32;
                let tile_entity = commands
                    .spawn(TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(tilemap_entity),
                        texture_index: TileTextureIndex(texture_index),
                        ..Default::default()
                    })
                    .id();
                commands.entity(tilemap_entity).add_child(tile_entity);

                tile_bit_grid[x as usize][y as usize] = block_bits;

                data.chunk_bits[x as usize][y as usize] = block_bits;
                tile_storage.set(&tile_pos, tile_entity);
                tile_bit_grid = Self::update_neighbour_tiles(
                    tile_pos,
                    block_bits,
                    commands,
                    &mut tile_storage,
                    tile_bit_grid,
                    data,
                    chunk_manager,
                    chunk_pos,
                );
            }
        }
        // Self::smooth_terrain(5, &mut tile_storage, tile_index_grid, commands);

        let transform = Transform::from_translation(Vec3::new(
            chunk_pos.x as f32 * 16. as f32 * 16.,
            chunk_pos.y as f32 * 16. as f32 * 16.,
            0.0,
        ));
        commands.entity(tilemap_entity).insert(TilemapBundle {
            grid_size,
            map_type,
            size: tilemap_size,
            storage: tile_storage,
            texture: TilemapTexture::Single(sprite_sheet.tiles_sheet.clone()),
            tile_size,
            transform,
            ..Default::default()
        });

        chunk_manager
            .chunk_tile_bit_grid
            .insert(chunk_pos, dbg!(tile_bit_grid));
    }

    fn update_neighbour_tiles(
        new_tile_pos: TilePos,
        new_tile_bits: u8,
        commands: &mut Commands,
        tile_storage: &mut TileStorage,
        mut tile_bit_grid: [[u8; 16]; 16],
        data: &mut ResMut<Data>,
        chunk_manager: &mut ResMut<ChunkManager>,
        chunk_pos: IVec2,
    ) -> [[u8; 16]; 16] {
        //TODO: figure out how to get neighbour chunks...
        let x = new_tile_pos.x as i8;
        let y = new_tile_pos.y as i8;

        for dy in -1i8..=1 {
            for dx in -1i8..=1 {
                let mut neighbour_tile_pos = TilePos {
                    x: (dx + x) as u32,
                    y: (dy + y) as u32,
                };
                let mut chunk_tile_bit_grid = tile_bit_grid;
                let mut is_other_chunk_tile = false;
                let mut skip_x = false;
                let mut skip_y = false;

                if x + dx < 0 {
                    skip_x = true;
                    is_other_chunk_tile = true;
                    if let Some(bit_grid) = chunk_manager.chunk_tile_bit_grid.get(&IVec2 {
                        x: chunk_pos.x - 1,
                        y: chunk_pos.y,
                    }) {
                        chunk_tile_bit_grid = *bit_grid;
                        neighbour_tile_pos.x = 15;
                        println!(
                            "1 {:?} {:?} {:?}",
                            chunk_pos, new_tile_pos, neighbour_tile_pos
                        );
                        skip_x = false;
                    }
                } else if x + dx >= 16 {
                    skip_x = true;
                    is_other_chunk_tile = true;
                    if let Some(bit_grid) = chunk_manager.chunk_tile_bit_grid.get(&IVec2 {
                        x: chunk_pos.x + 1,
                        y: chunk_pos.y,
                    }) {
                        chunk_tile_bit_grid = *bit_grid;
                        neighbour_tile_pos.x = 0;
                        println!(
                            "2 {:?} {:?} {:?}",
                            chunk_pos, new_tile_pos, neighbour_tile_pos
                        );
                        skip_x = false;
                    }
                }
                if y + dy < 0 {
                    skip_y = true;
                    is_other_chunk_tile = true;
                    if let Some(bit_grid) = chunk_manager.chunk_tile_bit_grid.get(&IVec2 {
                        x: chunk_pos.x,
                        y: chunk_pos.y - 1,
                    }) {
                        chunk_tile_bit_grid = *bit_grid;
                        neighbour_tile_pos.y = 15;
                        println!(
                            "3 {:?} {:?} {:?}",
                            chunk_pos, new_tile_pos, neighbour_tile_pos
                        );
                        skip_y = false;
                    }
                } else if y + dy >= 16 {
                    skip_y = true;
                    is_other_chunk_tile = true;
                    if let Some(bit_grid) = chunk_manager.chunk_tile_bit_grid.get(&IVec2 {
                        x: chunk_pos.x,
                        y: chunk_pos.y + 1,
                    }) {
                        chunk_tile_bit_grid = *bit_grid;
                        neighbour_tile_pos.y = 0;
                        println!(
                            "4 {:?} {:?} {:?}",
                            chunk_pos, new_tile_pos, neighbour_tile_pos
                        );
                        skip_y = false;
                    }
                }
                if !(dx == 0 && dy == 0) && !skip_x && !skip_y {
                    let neighbour_tile_bits = if is_other_chunk_tile {
                        chunk_tile_bit_grid[neighbour_tile_pos.x as usize]
                            [neighbour_tile_pos.y as usize]
                    } else {
                        tile_bit_grid[neighbour_tile_pos.x as usize][neighbour_tile_pos.y as usize]
                    };
                    if (dx + dy) as i8 == 1 || (dx + dy) as i8 == -1 {
                        let updated_bit_index =
                            Self::compute_tile_index(new_tile_bits, neighbour_tile_bits, (dx, dy));
                        if neighbour_tile_pos.y == 2 {
                            // println!(
                            //     "pos: {:?}, {:?} {:?} {:?}| {:?}",
                            //     neighbour_tile_pos,
                            //     new_tile_bits,
                            //     neighbour_tile_bits,
                            //     (dx, dy),
                            //     updated_bit_index
                            // );
                        }
                        commands
                            .entity(tile_storage.get(&neighbour_tile_pos).unwrap())
                            .insert(TileTextureIndex(updated_bit_index as u32));
                        if is_other_chunk_tile {
                            chunk_tile_bit_grid[neighbour_tile_pos.x as usize]
                                [neighbour_tile_pos.y as usize] = updated_bit_index;
                        } else {
                            tile_bit_grid[neighbour_tile_pos.x as usize]
                                [neighbour_tile_pos.y as usize] = updated_bit_index;
                        }
                        // chunk_tile_bit_grid[neighbour_tile_pos.x as usize]
                        //     [neighbour_tile_pos.y as usize] = updated_bit_index;

                        data.chunk_bits[neighbour_tile_pos.x as usize]
                            [neighbour_tile_pos.y as usize] = updated_bit_index;
                    } else {
                        let updated_bit_index = Self::compute_corner_index(
                            new_tile_bits,
                            neighbour_tile_bits,
                            (dx, dy),
                        );
                        if neighbour_tile_pos.y == 2 {
                            // println!(
                            //     "pos: {:?}, {:?} {:?} {:?}| {:?}",
                            //     neighbour_tile_pos,
                            //     new_tile_bits,
                            //     neighbour_tile_bits,
                            //     (dx, dy),
                            //     updated_bit_index
                            // );
                        }
                        commands
                            .entity(tile_storage.get(&neighbour_tile_pos).unwrap())
                            .insert(TileTextureIndex(updated_bit_index.into()));
                        if is_other_chunk_tile {
                            chunk_tile_bit_grid[neighbour_tile_pos.x as usize]
                                [neighbour_tile_pos.y as usize] = updated_bit_index;
                        } else {
                            tile_bit_grid[neighbour_tile_pos.x as usize]
                                [neighbour_tile_pos.y as usize] = updated_bit_index;
                        }

                        data.chunk_bits[neighbour_tile_pos.x as usize]
                            [neighbour_tile_pos.y as usize] = updated_bit_index;
                    }
                }
            }
        }
        tile_bit_grid
    }

    fn compute_tile_index(new_tile_bits: u8, neighbour_bits: u8, edge: (i8, i8)) -> u8 {
        let mut index = 0;
        // new tile will be 0b1111 i think
        if edge == (0, 1) {
            // Top edge needs b0 b1
            index |= (new_tile_bits & 0b0110);
            index |= (neighbour_bits & 0b1001);
        } else if edge == (1, 0) {
            // Right edge
            index |= (new_tile_bits & 0b0101);
            index |= (neighbour_bits & 0b1010);
        } else if edge == (0, -1) {
            // Bottom edge
            index |= (new_tile_bits & 0b0011);
            index |= (neighbour_bits & 0b1100);
        } else if edge == (-1, 0) {
            // Left edge
            index |= (new_tile_bits & 0b1010);
            index |= (neighbour_bits & 0b0101);
        }
        index
    }
    fn compute_corner_index(new_tile_bits: u8, neighbour_bits: u8, corner: (i8, i8)) -> u8 {
        let mut index = 0;
        if corner == (-1, 1) {
            // Top-left corner
            index |= new_tile_bits & 0b1000;
            index |= neighbour_bits & 0b0111;
        } else if corner == (1, 1) {
            // Top-right corner
            index |= new_tile_bits & 0b0100;
            index |= neighbour_bits & 0b1011;
        } else if corner == (-1, -1) {
            // Bottom-left corner
            index |= new_tile_bits & 0b0010;
            index |= neighbour_bits & 0b1101;
        } else if corner == (1, -1) {
            // Bottom-right corner
            index |= new_tile_bits & 0b0001;
            index |= neighbour_bits & 0b1110;
        }
        index
    }

    fn smooth_terrain(
        k: i8,
        tile_storage: &mut TileStorage,
        tile_index_grid: [[u32; 16]; 16],
        commands: &mut Commands,
    ) {
        // Create a new grid to hold the smoothed terrain
        let mut smooth_grid = [[10000; 16 as usize]; 16 as usize];

        // Loop over each tile in the grid
        for y in 0..16 {
            for x in 0..16 {
                let current_tile = tile_index_grid[x as usize][y as usize];
                // Count the number of adjacent tiles that are the same type as the current tile
                let mut adjacent_count = 0;
                let mut previous_tile: u32 = 10000;
                let mut smooth_tile: u32 = 10000;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if x + dx >= 0 && x + dx < 16 && y + dy >= 0 && y + dy < 16 {
                            let adj_tile = tile_index_grid[i32::abs(x + dx) as usize]
                                [i32::abs(y + dy) as usize];
                            if adj_tile == current_tile {
                                continue;
                            }
                            // tile_storage.get(&TilePos {
                            //     x: (x + dx as i8).try_into().unwrap(),
                            //     y: (y + dy as i8).try_into().unwrap(),
                            // });
                            if adj_tile == previous_tile {
                                adjacent_count += 1;
                                if adjacent_count >= k {
                                    smooth_tile = adj_tile;
                                }
                            } else {
                                previous_tile = adj_tile;
                            }
                        }
                    }
                }
                // If at least 5 adjacent tiles are the same type, set the smooth_grid value to 1
                // (indicating that this tile should be the same type as the current tile)
                if adjacent_count >= k {
                    smooth_grid[y as usize][x as usize] = smooth_tile;
                }
            }
        }

        // Use the smooth_grid to set the tile types in the tile_storage
        for y in 0..16 {
            for x in 0..16 {
                let tile_pos = TilePos {
                    x: x.try_into().unwrap(),
                    y: y.try_into().unwrap(),
                };
                if smooth_grid[y][x] < 1000 {
                    // tile_storage.get(&tile_pos, smoothed_tile);
                    commands
                        .entity(tile_storage.get(&tile_pos).unwrap())
                        .insert(TileTextureIndex(smooth_grid[y][x]));
                }
            }
        }
    }

    pub fn camera_pos_to_chunk_pos(camera_pos: &Vec2) -> IVec2 {
        let camera_pos = camera_pos.as_ivec2();
        let chunk_size: IVec2 = IVec2::new(16 as i32, 16 as i32);
        let tile_size: IVec2 = IVec2::new(32 as i32, 32 as i32);
        camera_pos / (chunk_size * tile_size)
    }
    pub fn camera_pos_to_block_pos(camera_pos: &Vec2) -> IVec2 {
        let camera_pos = camera_pos.as_ivec2();
        let chunk_size: IVec2 = IVec2::new(16 as i32, 16 as i32);
        let tile_size: IVec2 = IVec2::new(32 as i32, 32 as i32);
        camera_pos / (chunk_size * tile_size)
    }

    fn spawn_chunks_around_camera(
        mut commands: Commands,
        sprite_sheet: Res<ImageAssets>,
        camera_query: Query<&Transform, With<Camera>>,
        mut chunk_manager: ResMut<ChunkManager>,
        game: Res<Game>,
        mut data: ResMut<Data>,
    ) {
        if !chunk_manager.spawned_chunks.contains(&IVec2::new(0, 0)) {
            chunk_manager.spawned_chunks.insert(IVec2::new(0, 0));
            Self::spawn_chunk(
                &mut commands,
                &sprite_sheet,
                &game,
                IVec2::new(0, 0),
                &mut data,
                &mut chunk_manager,
            );
        }
        if !chunk_manager.spawned_chunks.contains(&IVec2::new(-1, 0)) {
            chunk_manager.spawned_chunks.insert(IVec2::new(-1, 0));
            Self::spawn_chunk(
                &mut commands,
                &sprite_sheet,
                &game,
                IVec2::new(-1, 0),
                &mut data,
                &mut chunk_manager,
            );
        }
        // for transform in camera_query.iter() {
        //     let camera_chunk_pos = Self::camera_pos_to_chunk_pos(&transform.translation.xy());
        //     for y in (camera_chunk_pos.y - 2)..(camera_chunk_pos.y + 2) {
        //         for x in (camera_chunk_pos.x - 2)..(camera_chunk_pos.x + 2) {
        //             if !chunk_manager.spawned_chunks.contains(&IVec2::new(x, y)) {
        //                 println!("spawning chunk at {:?} {:?}", x, y);
        //                 chunk_manager.spawned_chunks.insert(IVec2::new(x, y));
        //                 Self::spawn_chunk(
        //                     &mut commands,
        //                     &sprite_sheet,
        //                     &game,
        //                     IVec2::new(x, y),
        //                     &mut data,
        //                     &mut chunk_manager,
        //                 );
        //             }
        //         }
        //     }
        // }
    }

    fn despawn_outofrange_chunks(
        mut commands: Commands,
        camera_query: Query<&Transform, With<Camera>>,
        chunks_query: Query<(Entity, &Transform)>,
        mut chunk_manager: ResMut<ChunkManager>,
    ) {
        for camera_transform in camera_query.iter() {
            for (entity, chunk_transform) in chunks_query.iter() {
                let chunk_pos = chunk_transform.translation.xy();
                let distance = camera_transform.translation.xy().distance(chunk_pos);
                //TODO: calculate maximum possible distance for 2x2 chunksa
                if distance > 1250.0 {
                    let x = (chunk_pos.x as f32 / (16 as f32 * 16.)).floor() as i32;
                    let y = (chunk_pos.y as f32 / (16 as f32 * 16.)).floor() as i32;
                    println!("despawning chunk at {:?} {:?} d === {:?}", x, y, distance);
                    chunk_manager.spawned_chunks.remove(&IVec2::new(x, y));
                    commands.entity(entity).despawn_recursive();
                }
            }
        }
    }

    fn spawn_test_objects(mut commands: Commands, graphics: Res<Graphics>) {
        let mut tree_children = Vec::new();

        let tree_points = poisson_disk_sampling(4., 30, rand::thread_rng());
        for tp in tree_points {
            tree_children.push(WorldObject::Tree.spawn(
                &mut commands,
                &graphics,
                Vec3::new((tp.x as f32) * 16., (tp.y as f32) * 16., 0.1),
            ));
        }
        commands
            .spawn(SpatialBundle::default())
            // .insert(Name::new("Test Objects"))
            // .push_children(&children)
            .push_children(&tree_children);
    }
}

fn poisson_disk_sampling(r: f64, k: i8, mut rng: ThreadRng) -> Vec<Vec2> {
    // TODO: fix this to work w 4 quadrants -/+
    let n = 2.;
    // the final set of points to return
    let mut points: Vec<Vec2> = vec![];
    // the currently "Active" set of points
    let mut active: Vec<Vec2> = vec![];
    let p0 = Vec2::new(
        rng.gen_range(0..WORLD_SIZE) as f32,
        rng.gen_range(0..WORLD_SIZE) as f32,
    );

    let cell_size = f64::floor(r / f64::sqrt(n));
    let num_cell: usize = (f64::ceil(WORLD_SIZE as f64 / cell_size) + 1.) as usize;
    let mut grid: Vec<Vec<Option<Vec2>>> = vec![vec![None; num_cell]; num_cell];

    let insert_point = |g: &mut Vec<Vec<Option<Vec2>>>, p: Vec2| {
        let xi: usize = f64::floor(p.x as f64 / cell_size) as usize;
        let yi: usize = f64::floor(p.y as f64 / cell_size) as usize;
        g[xi][yi] = Some(p);
    };

    let is_valid_point = move |g: &Vec<Vec<Option<Vec2>>>, p: Vec2| -> bool {
        // make sure p is on screen
        if p.x < 0. || p.x > WORLD_SIZE as f32 || p.y < 0. || p.y > WORLD_SIZE as f32 {
            return false;
        }

        // check neighboring eight cells
        let xi: f64 = f64::floor(p.x as f64 / cell_size);
        let yi: f64 = f64::floor(p.y as f64 / cell_size);
        let i0 = usize::max((xi - 1.) as usize, 0);
        let i1 = usize::min((xi + 1.) as usize, num_cell - 1. as usize);
        let j0 = usize::max((yi - 1.) as usize, 0);
        let j1 = usize::min((yi + 1.) as usize, num_cell - 1. as usize);

        for i in i0..=i1 {
            for j in j0..=j1 {
                if let Some(sample_point) = g[i][j] {
                    if sample_point.distance(p) < r as f32 {
                        return false;
                    }
                }
            }
        }
        true
    };

    insert_point(&mut grid, p0);
    points.push(p0);
    active.push(p0);
    while active.len() > 0 {
        let i = rng.gen_range(0..=(active.len() - 1));
        let p = active.get(i).unwrap();
        let mut found = false;

        for _ in 0..k {
            // get a random angle
            let theta: f64 = rng.gen_range(0. ..360.);
            let new_r = rng.gen_range(r..(2. * r));

            // create new point from randodm angle r distance away from p
            let new_px = p.x as f64 + new_r * theta.to_radians().cos();
            let new_py = p.y as f64 + new_r * theta.to_radians().sin();
            let new_p = Vec2::new(new_px as f32, new_py as f32);

            if !is_valid_point(&grid, new_p) {
                continue;
            }

            //add the new point to our lists and break
            points.push(new_p);
            insert_point(&mut grid, new_p);
            active.push(new_p);
            found = true;
            break;
        }

        if !found {
            active.remove(i);
        }
    }

    points
}