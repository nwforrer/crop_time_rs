use bevy::{
    core::FixedTimestep,
    prelude::*,
};
use rand::distributions::{Distribution, Uniform};

pub struct GamePlugin;

#[derive(Component)]
struct Player;

#[derive(Component, Debug)]
struct Growable {
    growth_state: u32,
    max_growth_state: u32,
}

#[derive(Component)]
struct Animation;

#[derive(Default)]
struct TextureHandles{
    crops: Handle<TextureAtlas>,
}

const TIME_STEP: f32 = 1.0 / 60.0;
impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(TextureHandles { ..Default::default() })
            .add_startup_system(setup_player)
            .add_startup_system(setup_tiles)
            .add_startup_system(setup_crop_textures)
            .add_system(animate_sprite_system)
            .add_system(grow_system)
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(FixedTimestep::step(TIME_STEP as f64))
                    .with_system(player_movement_system)
            );
    }
}

fn animate_sprite_system(
    time: Res<Time>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    mut query: Query<(&mut Timer, &mut TextureAtlasSprite, &Handle<TextureAtlas>), With<Animation>>,
) {
    for (mut timer, mut sprite, texture_atlas_handle) in query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            let texture_atlas = texture_atlases.get(texture_atlas_handle).unwrap();
            sprite.index = (sprite.index + 1) % texture_atlas.textures.len();
        }
    }
}

fn grow_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Timer, &mut TextureAtlasSprite, &mut Growable)>,
) {
    for (entity, mut timer, mut sprite, mut growable) in query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            if growable.growth_state < growable.max_growth_state {
                growable.growth_state += 1;
                sprite.index = growable.growth_state as usize;
            } else {
                commands.entity(entity).remove::<Growable>();
            }
        }
    }
}

fn action_pressed(name: &str, keyboard_input: &Res<Input<KeyCode>>) -> bool {
    match name {
        "move_left" => {
            keyboard_input.pressed(KeyCode::Left) || 
            keyboard_input.pressed(KeyCode::A)
        },
        "move_right" => {
            keyboard_input.pressed(KeyCode::Right) || 
            keyboard_input.pressed(KeyCode::D)
        },
        "move_up" => {
            keyboard_input.pressed(KeyCode::Up) || 
            keyboard_input.pressed(KeyCode::W)
        },
        "move_down" => {
            keyboard_input.pressed(KeyCode::Down) || 
            keyboard_input.pressed(KeyCode::S)
        },
        _ => {
            false
        },
    }
}

fn player_movement_system(
    mut commands: Commands, 
    texture_handles: Res<TextureHandles>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&Player, &mut Transform)>,
) {
    let (_player, mut transform) = query.single_mut();
    let mut direction = Vec3::ZERO;
    if action_pressed("move_left", &keyboard_input) {
        direction.x -= 1.0;
    }
    if action_pressed("move_right", &keyboard_input) {
        direction.x += 1.0;
    }
    if action_pressed("move_up", &keyboard_input) {
        direction.y += 1.0;
    }
    if action_pressed("move_down", &keyboard_input) {
        direction.y -= 1.0;
    }

    let translation = &mut transform.translation;
    *translation = *translation + direction.normalize_or_zero() * 250.0 * TIME_STEP;

    if keyboard_input.just_pressed(KeyCode::Return) {
        commands.spawn_bundle(SpriteSheetBundle {
            texture_atlas: texture_handles.crops.to_owned(),
            transform: Transform {
                translation: *translation,
                scale: Vec3::splat(3.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Growable {
            growth_state: 0,
            max_growth_state: 2,
        })
        .insert(Timer::from_seconds(5.0, true));
    }
}

fn setup_player(
    mut commands: Commands, 
    asset_server: Res<AssetServer>, 
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let player_texture_handle = asset_server.load("player.png");
    let player_texture_atlas = TextureAtlas::from_grid(player_texture_handle, Vec2::new(32.0, 32.0), 4, 1);
    let player_texture_atlas_handle = texture_atlases.add(player_texture_atlas);
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: player_texture_atlas_handle,
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, 1.0),
                scale: Vec3::splat(3.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Timer::from_seconds(0.1, true))
        .insert(Animation)
        .insert(Player);
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
    let tiles_texture_atlas = TextureAtlas::from_grid(tiles_texture_handle, Vec2::new(16.0, 16.0), 2, 2);
    let tiles_texture_atlas_handle = texture_atlases.add(tiles_texture_atlas);

    let window = get_primary_window_size(windows);

    let scale = 3.0;
    let tile_size = scale * 16.0;
    let columns = (window.x / tile_size) as i32 + 2;
    let rows = (window.y / tile_size) as i32 + 2;
    let between = Uniform::from(0..100);
    let mut rng = rand::thread_rng();
    for row in 0..rows {
        for column in 0..columns {
            let position = Vec3::new(
                column as f32 * tile_size - window.x / 2.0,
                row as f32 * tile_size - window.y / 2.0,
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

            commands
                .spawn_bundle(SpriteSheetBundle {
                    texture_atlas: tiles_texture_atlas_handle.to_owned(),
                    transform: Transform {
                        translation: position,
                        scale: Vec3::splat(scale),
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
    let window = Vec2::new(window.width() as f32, window.height() as f32);
    window
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
