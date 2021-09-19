#![allow(clippy::all)]
use bevy::{
    asset::LoadState,
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    log::info,
    prelude::*,
    sprite::{TextureAtlas, TextureAtlasBuilder},
    utils::HashSet,
    window::WindowMode,
};
use bevy_tilemap::prelude::*;
use rand::Rng;

fn main() {
    App::build()
        .insert_resource(WindowDescriptor {
            title: "Drunk Stressed Dwarves".to_string(),
            width: 1024.,
            height: 1024.,
            vsync: false,
            resizable: true,
            mode: WindowMode::Windowed,
            ..Default::default()
        })
        .init_resource::<TileSpriteHandles>()
        .init_resource::<State>()
        .add_plugins(DefaultPlugins)
        .add_plugins(TilemapDefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_startup_system(setup_system.system())
        .add_system(load.system())
        .add_system(build_map.system())
        .add_system(drunk_stumbles.system())
        .add_system(counter.system())
        .run()
}

const DWARF_COUNT: usize = 10_000;

#[derive(Default, Clone)]
struct TileSpriteHandles {
    handles: Vec<HandleUntyped>,
    atlas_loaded: bool,
}

#[derive(Default, Copy, Clone, PartialEq)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Default)]
struct Render {
    sprite_index: usize,
    sprite_order: usize,
}

#[derive(Bundle)]
struct StressDwarfBundle {
    position: Position,
    render: Render,
}

#[derive(Default, Clone)]
struct State {
    map_loaded: bool,
    spawned: bool,
    collisions: HashSet<(i32, i32)>,
}

impl State {
    fn try_stumble(&mut self, position: &mut Position, delta_xy: (i32, i32)) {
        let new_pos = (position.x + delta_xy.0, position.y + delta_xy.1);
        if !self.collisions.contains(&new_pos) {
            position.x = new_pos.0;
            position.y = new_pos.1;
        }
    }
}

fn setup_system(
    mut tile_sprite_handles: ResMut<TileSpriteHandles>,
    asset_server: Res<AssetServer>,
) {
    tile_sprite_handles.handles = asset_server.load_folder("textures").unwrap();
}

fn load(
    mut commands: Commands,
    mut sprite_handles: ResMut<TileSpriteHandles>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut textures: ResMut<Assets<Texture>>,
    asset_server: Res<AssetServer>,
) {
    if sprite_handles.atlas_loaded {
        return;
    }

    // Lets load all our textures from our folder!
    let mut texture_atlas_builder = TextureAtlasBuilder::default();
    if let LoadState::Loaded =
        asset_server.get_group_load_state(sprite_handles.handles.iter().map(|handle| handle.id))
    {
        for handle in sprite_handles.handles.iter() {
            let texture = textures.get(handle).unwrap();
            texture_atlas_builder.add_texture(handle.clone_weak().typed::<Texture>(), &texture);
        }

        let texture_atlas = texture_atlas_builder.finish(&mut textures).unwrap();
        let atlas_handle = texture_atlases.add(texture_atlas);

        // These are fairly advanced configurations just to quickly showcase
        // them.
        let tilemap = Tilemap::builder()
            .dimensions(3, 3)
            .texture_dimensions(32, 32)
            .chunk_dimensions(32, 32, 1)
            .auto_chunk()
            .auto_spawn(2, 2)
            .add_layer(
                TilemapLayer {
                    kind: LayerKind::Dense,
                },
                0,
            )
            .add_layer(
                TilemapLayer {
                    kind: LayerKind::Sparse,
                },
                1,
            )
            .texture_atlas(atlas_handle)
            .finish()
            .unwrap();

        let tilemap_components = TilemapBundle {
            tilemap,
            visible: Visible {
                is_visible: true,
                is_transparent: true,
            },
            transform: Default::default(),
            global_transform: Default::default(),
        };

        commands
            .spawn()
            .insert_bundle(OrthographicCameraBundle::new_2d());
        commands
            .spawn()
            .insert_bundle(tilemap_components)
            .insert(Timer::from_seconds(0.075, true));

        sprite_handles.atlas_loaded = true;
    }
}

fn build_map(
    mut commands: Commands,
    mut state: ResMut<State>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    asset_server: Res<AssetServer>,
    mut query: Query<&mut Tilemap>,
) {
    if state.map_loaded {
        return;
    }

    for mut map in query.iter_mut() {
        let chunk_width = (map.width().unwrap() * map.chunk_width()) as i32;
        let chunk_height = (map.height().unwrap() * map.chunk_height()) as i32;

        let floor_sprite: Handle<Texture> = asset_server.get_handle("textures/square-floor.png");
        let wall_sprite: Handle<Texture> = asset_server.get_handle("textures/square-wall.png");
        let texture_atlas = texture_atlases.get(map.texture_atlas()).unwrap();
        let floor_idx = texture_atlas.get_texture_index(&floor_sprite).unwrap();
        let wall_idx = texture_atlas.get_texture_index(&wall_sprite).unwrap();

        let mut tiles = Vec::new();
        for y in (-chunk_height / 2)..(chunk_height / 2) {
            for x in (-chunk_width / 2)..(chunk_width / 2) {
                let tile = Tile {
                    point: (x, y),
                    sprite_index: floor_idx,
                    ..Default::default()
                };
                tiles.push(tile);
            }
        }

        for x in 0..chunk_width {
            let x = x - chunk_width / 2;
            let tile_a = (x, -chunk_height / 2);
            let tile_b = (x, chunk_height / 2 - 1);
            tiles.push(Tile {
                point: tile_a,
                sprite_index: wall_idx,
                ..Default::default()
            });
            tiles.push(Tile {
                point: tile_b,
                sprite_index: wall_idx,
                ..Default::default()
            });
            state.collisions.insert(tile_a);
            state.collisions.insert(tile_b);
        }

        // Then the wall tiles on the Y axis.
        for y in 0..chunk_height {
            let y = y - chunk_height / 2;
            let tile_a = (-chunk_width / 2, y);
            let tile_b = (chunk_width / 2 - 1, y);
            tiles.push(Tile {
                point: tile_a,
                sprite_index: wall_idx,
                ..Default::default()
            });
            tiles.push(Tile {
                point: tile_b,
                sprite_index: wall_idx,
                ..Default::default()
            });
            state.collisions.insert(tile_a);
            state.collisions.insert(tile_b);
        }

        let range = (chunk_width * chunk_height) as usize / 5;
        let mut rng = rand::thread_rng();
        for _ in 0..range {
            let x = rng.gen_range((-chunk_width / 2)..(chunk_width / 2));
            let y = rng.gen_range((-chunk_height / 2)..(chunk_height / 2));
            let coord = (x, y, 0i32);
            if coord != (0, 0, 0) {
                tiles.push(Tile {
                    point: (x, y),
                    sprite_index: wall_idx,
                    ..Default::default()
                });
                state.collisions.insert((x, y));
            }
        }

        let dwarf_sprite: Handle<Texture> = asset_server.get_handle("textures/square-dwarf.png");
        let dwarf_sprite_index = texture_atlas.get_texture_index(&dwarf_sprite).unwrap();
        let mut rng = rand::thread_rng();
        info!("Spawning {} drunken dwarves.", DWARF_COUNT);
        for _ in 0..DWARF_COUNT {
            let position = Position {
                x: rng.gen_range((-chunk_width / 2 + 1)..(chunk_width / 2 - 1)),
                y: rng.gen_range((-chunk_height / 2 + 1)..(chunk_height / 2 - 1)),
            };

            commands.spawn().insert_bundle(StressDwarfBundle {
                position,
                render: Render {
                    sprite_index: dwarf_sprite_index,
                    sprite_order: 1,
                },
            });
        }
        info!("{} drunken dwarves spawned.", DWARF_COUNT);

        map.insert_tiles(tiles).unwrap();
        state.map_loaded = true;
    }
}

fn move_sprite(
    map: &mut Tilemap,
    previous_position: Position,
    position: Position,
    render: &Render,
) {
    map.clear_tile((previous_position.x, previous_position.y), 1)
        .unwrap();
    let tile = Tile {
        point: (position.x, position.y),
        sprite_index: render.sprite_index,
        sprite_order: render.sprite_order,
        ..Default::default()
    };
    map.insert_tile(tile).unwrap();
}

fn drunk_stumbles(
    mut state: ResMut<State>,
    mut map_query: Query<&mut Tilemap>,
    mut drunk_query: Query<(&mut Position, &Render)>,
) {
    if !state.map_loaded {
        return;
    }

    for mut map in map_query.iter_mut() {
        for (mut position, render) in drunk_query.iter_mut() {
            let previous_position = *position;
            let mut rng = rand::thread_rng();
            state.try_stumble(&mut position, (rng.gen_range(-1..2), rng.gen_range(-1..2)));
            if previous_position == *position {
                continue;
            }
            move_sprite(&mut map, previous_position, *position, render);
        }
    }
}

fn counter(diagnostics: Res<Diagnostics>, time: Res<Time>, mut query: Query<&mut Timer>) {
    if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
        for mut timer in query.iter_mut() {
            timer.tick(time.delta());
            if !timer.finished() {
                return;
            }
            if let Some(average) = fps.average() {
                info!("FPS average: {:.2}", average);
            }
        }
    }
}
