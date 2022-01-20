use bevy::{core::FixedTimestep, prelude::*, sprite::collide_aabb::collide};
use rand::distributions::{Distribution, Uniform};

const SCALE: f32 = 3.0;
const TILE_SIZE: f32 = SCALE * 16.0;
const PLAYER_SIZE: f32 = SCALE * 32.0;

pub struct GamePlugin;

enum CollisionLayer {
    Environment,
    Characters,
    Tools,
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Crop;

#[derive(Component)]
struct CollisionConfig {
    layer: u32,
    mask: u32,
}

#[derive(Component, Debug)]
struct Growable {
    growth_state: u32,
    max_growth_state: u32,
}

#[derive(Component)]
struct Hydration(f32);

#[derive(Component)]
struct Animation(bool);

#[derive(Component)]
struct FollowTarget {
    target: Vec3,
    offset: Vec3,
    flip_x: bool,
    grid_snap: bool,
}

#[derive(Component)]
struct PlantSeedTool; // TODO: seed type

#[derive(Component)]
struct WaterPlantTool; // TODO: water amount (for upgraded watering can)

#[derive(Component)]
struct Highlight;

#[derive(Default)]
struct TextureHandles {
    crops: Handle<TextureAtlas>,
}

const TIME_STEP: f32 = 1.0 / 60.0;
impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TextureHandles {
            ..Default::default()
        })
        .add_startup_system(setup_player)
        .add_startup_system(setup_tiles)
        .add_startup_system(setup_crop_textures)
        .add_system(animate_sprite_system)
        .add_system(grow_system)
        .add_system(update_follow_system)
        .add_system(follow_system)
        .add_system(use_tool_system)
        .add_system(pickup_system)
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(TIME_STEP as f64))
                .with_system(player_movement_system),
        );
    }
}

fn animate_sprite_system(
    time: Res<Time>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    mut query: Query<(
        &mut Timer,
        &mut TextureAtlasSprite,
        &Handle<TextureAtlas>,
        &Animation,
    )>,
) {
    for (mut timer, mut sprite, texture_atlas_handle, animation) in query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            if animation.0 {
                let texture_atlas = texture_atlases.get(texture_atlas_handle).unwrap();
                sprite.index = (sprite.index + 1) % texture_atlas.textures.len();
            } else {
                sprite.index = 0;
            }
        }
    }
}

fn grow_system(
    time: Res<Time>,
    //mut commands: Commands,
    mut query: Query<(
        Entity,
        &mut Timer,
        &mut TextureAtlasSprite,
        &mut Growable,
        &mut Hydration,
    )>,
) {
    for (_entity, mut timer, mut sprite, mut growable, mut hydration) in query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            if growable.growth_state < growable.max_growth_state && hydration.0 >= 1.0 {
                growable.growth_state += 1;
                sprite.index = growable.growth_state as usize;
                hydration.0 = 0.0;
                //} else {
                //    commands.entity(entity).remove::<Growable>();
            }
        }
    }
}

fn action_pressed(name: &str, keyboard_input: &Res<Input<KeyCode>>) -> bool {
    match name {
        "move_left" => keyboard_input.pressed(KeyCode::Left) || keyboard_input.pressed(KeyCode::A),
        "move_right" => {
            keyboard_input.pressed(KeyCode::Right) || keyboard_input.pressed(KeyCode::D)
        }
        "move_up" => keyboard_input.pressed(KeyCode::Up) || keyboard_input.pressed(KeyCode::W),
        "move_down" => keyboard_input.pressed(KeyCode::Down) || keyboard_input.pressed(KeyCode::S),
        _ => false,
    }
}

fn update_follow_system(
    query: Query<(&TextureAtlasSprite, &Transform), With<Player>>,
    mut fq: Query<&mut FollowTarget>,
) {
    let (sprite, transform) = query.single();
    for mut follow in fq.iter_mut() {
        follow.target = transform.translation;
        follow.target.y += PLAYER_SIZE / 4.0;
        follow.target.x += PLAYER_SIZE / 4.0;
        follow.target.z = 1.0;
        follow.flip_x = sprite.flip_x;
    }
}

fn follow_system(mut query: Query<(&mut Transform, &FollowTarget)>) {
    for (mut transform, follow) in query.iter_mut() {
        let flip = if follow.flip_x { -1.0 } else { 1.0 };
        let offset = Vec3::new(
            flip * follow.offset.x - TILE_SIZE / 2.0,
            follow.offset.y,
            follow.offset.z,
        );
        let pos = follow.target + offset;
        if follow.grid_snap {
            transform.translation = pixel_to_tile_coord(pos);
        } else {
            transform.translation = pos;
        }
    }
}

fn use_tool_system(
    mut commands: Commands,
    texture_handles: Res<TextureHandles>,
    keyboard_input: Res<Input<KeyCode>>,
    query: Query<&Transform, With<Highlight>>,
    tool_query: Query<(Option<&WaterPlantTool>, Option<&PlantSeedTool>), With<FollowTarget>>,
    mut cq: Query<(&Transform, &mut Hydration), With<Crop>>,
) {
    for (water_tool, plant_tool) in tool_query.iter() {
        if plant_tool.is_some() {
            let transform = query.single();
            if keyboard_input.just_pressed(KeyCode::Return) {
                let mut free_slot = true;
                for (crop_tf, _) in cq.iter() {
                    let crop = Vec3::new(crop_tf.translation.x, crop_tf.translation.y, 0.0);
                    let new_crop = Vec3::new(transform.translation.x, transform.translation.y, 0.0);
                    if crop == new_crop {
                        free_slot = false;
                        break;
                    }
                }

                if free_slot {
                    commands
                        .spawn_bundle(SpriteSheetBundle {
                            texture_atlas: texture_handles.crops.to_owned(),
                            transform: Transform {
                                translation: transform.translation,
                                scale: Vec3::splat(SCALE),
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .insert(Growable {
                            growth_state: 0,
                            max_growth_state: 2,
                        })
                        .insert(Hydration(0.0))
                        .insert(Crop)
                        .insert(Timer::from_seconds(5.0, true));
                }
            }
        }

        if water_tool.is_some() {
            if keyboard_input.just_pressed(KeyCode::Return) {
                let transform = query.single();
                for (crop_tf, mut hydration) in cq.iter_mut() {
                    let crop = Vec3::new(crop_tf.translation.x, crop_tf.translation.y, 0.0);
                    let new_crop = Vec3::new(transform.translation.x, transform.translation.y, 0.0);
                    if crop == new_crop {
                        hydration.0 = 1.0;
                    }
                }
            }
        }
    }
}

fn pickup_system(
    mut commands: Commands,
    keyboard_input: Res<Input<KeyCode>>,
    player_query: Query<(&Transform, &CollisionConfig), With<Player>>,
    col_query: Query<(Entity, &Transform, &CollisionConfig), Without<FollowTarget>>,
    tool_query: Query<Entity, (With<FollowTarget>, Without<Highlight>)>,
) {
    if keyboard_input.just_pressed(KeyCode::E) {
        let (player_transform, player_col_config) = player_query.single();
        for (entity, transform, collision_config) in col_query.iter() {
            let collision = collide(
                player_transform.translation,
                Vec2::splat(PLAYER_SIZE),
                transform.translation,
                Vec2::splat(TILE_SIZE),
            );
            if collision.is_some()
                && collision_config.layer & player_col_config.mask != 0
                && collision_config.layer & CollisionLayer::Tools as u32 != 0
            {
                for active_tool in tool_query.iter() {
                    commands.entity(active_tool).remove::<FollowTarget>();
                }
                commands.entity(entity).insert(FollowTarget {
                    target: transform.translation,
                    offset: Vec3::new(-TILE_SIZE / 3.0 * SCALE, -TILE_SIZE / 2.0, 0.0),
                    flip_x: false,
                    grid_snap: false,
                });
            }
        }
    }
}

fn player_movement_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&mut Transform, &mut TextureAtlasSprite, &mut Animation), With<Player>>,
) {
    let (mut transform, mut sprite, mut animation) = query.single_mut();
    let mut direction = Vec3::ZERO;
    if action_pressed("move_left", &keyboard_input) {
        direction.x -= 1.0;
        sprite.flip_x = false;
    }
    if action_pressed("move_right", &keyboard_input) {
        direction.x += 1.0;
        sprite.flip_x = true;
    }
    if action_pressed("move_up", &keyboard_input) {
        direction.y += 1.0;
    }
    if action_pressed("move_down", &keyboard_input) {
        direction.y -= 1.0;
    }

    animation.0 = direction.length() > 0.1;

    let translation = &mut transform.translation;
    *translation += direction.normalize_or_zero() * 250.0 * TIME_STEP;
}

fn setup_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let player_texture_handle = asset_server.load("player.png");
    let player_texture_atlas =
        TextureAtlas::from_grid(player_texture_handle, Vec2::new(32.0, 32.0), 4, 1);
    let player_texture_atlas_handle = texture_atlases.add(player_texture_atlas);
    let player_size = 32.0;
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: player_texture_atlas_handle,
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, 5.0),
                scale: Vec3::splat(SCALE),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Timer::from_seconds(0.1, true))
        .insert(Animation(false))
        .insert(CollisionConfig {
            layer: CollisionLayer::Characters as u32,
            mask: CollisionLayer::Environment as u32 | CollisionLayer::Tools as u32,
        })
        .insert(Player);
    commands
        .spawn_bundle(SpriteBundle {
            texture: asset_server.load("highlight.png"),
            transform: Transform {
                scale: Vec3::splat(SCALE),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(FollowTarget {
            target: Vec3::splat(0.0),
            offset: Vec3::new(-player_size / 2.0 * SCALE, 0.0, 0.0),
            flip_x: false,
            grid_snap: true,
        })
        .insert(Highlight);

    commands
        .spawn_bundle(SpriteBundle {
            texture: asset_server.load("flower_seed_bag.png"),
            transform: Transform {
                scale: Vec3::splat(SCALE),
                translation: Vec3::new(-200.0, 0.0, 1.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(CollisionConfig {
            layer: CollisionLayer::Tools as u32,
            mask: 0,
        })
        .insert(PlantSeedTool);

    commands
        .spawn_bundle(SpriteBundle {
            texture: asset_server.load("watering_can.png"),
            transform: Transform {
                scale: Vec3::splat(SCALE),
                translation: Vec3::new(200.0, 0.0, 1.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(CollisionConfig {
            layer: CollisionLayer::Tools as u32,
            mask: 0,
        })
        .insert(WaterPlantTool);
}

fn setup_crop_textures(
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut texture_handles: ResMut<TextureHandles>,
) {
    let texture_handle = asset_server.load("corn.png");
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(16.0, 16.0), 4, 1);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    texture_handles.crops = texture_atlas_handle;
}

fn setup_tiles(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    windows: Res<Windows>,
) {
    let tiles_texture_handle = asset_server.load("tiles.png");
    let tiles_texture_atlas =
        TextureAtlas::from_grid(tiles_texture_handle, Vec2::new(16.0, 16.0), 2, 2);
    let tiles_texture_atlas_handle = texture_atlases.add(tiles_texture_atlas);

    let window = get_primary_window_size(windows);

    let columns = (window.x / TILE_SIZE) as i32 + 2;
    let rows = (window.y / TILE_SIZE) as i32 + 2;
    let between = Uniform::from(0..100);
    let mut rng = rand::thread_rng();
    for row in 0..rows {
        for column in 0..columns {
            let position = Vec3::new(
                column as f32 * TILE_SIZE - window.x / 2.0,
                row as f32 * TILE_SIZE - window.y / 2.0,
                0.0,
            );
            let sprite_id = between.sample(&mut rng);
            let sprite_id = if sprite_id < 95 {
                0
            } else if sprite_id < 98 {
                1
            } else {
                2
            };

            commands.spawn_bundle(SpriteSheetBundle {
                texture_atlas: tiles_texture_atlas_handle.to_owned(),
                transform: Transform {
                    translation: position,
                    scale: Vec3::splat(SCALE),
                    ..Default::default()
                },
                sprite: TextureAtlasSprite {
                    index: sprite_id,
                    ..Default::default()
                },
                ..Default::default()
            });
        }
    }
}

fn get_primary_window_size(windows: Res<Windows>) -> Vec2 {
    let window = windows.get_primary().unwrap();

    Vec2::new(window.width() as f32, window.height() as f32)
}

fn pixel_to_tile_coord(pos: Vec3) -> Vec3 {
    let tile = pos / TILE_SIZE;
    let tile_pos = tile.floor() * TILE_SIZE;
    Vec3::new(tile_pos.x, tile_pos.y, pos.z)
}

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "Crop Time".to_string(),
            width: 960.0,
            height: 540.0,
            vsync: true,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(GamePlugin)
        .run();
}
