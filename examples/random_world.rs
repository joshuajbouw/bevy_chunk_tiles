#![allow(clippy::all)]
use bevy::{
    asset::LoadState,
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
            title: "Random Hex World".to_string(),
            width: 1036.,
            height: 1036.,
            vsync: false,
            resizable: true,
            mode: WindowMode::Windowed,
            ..Default::default()
        })
        .init_resource::<SpriteHandles>()
        .init_resource::<GameState>()
        .add_plugins(DefaultPlugins)
        .add_plugins(TilemapDefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(load.system())
        .add_system(build_random_world.system())
        .add_system(character_movement.system())
        .run()
}

#[derive(Default, Clone)]
struct SpriteHandles {
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
    z_order: usize,
}

#[derive(Default)]
struct Player {}

#[derive(Bundle)]
struct PlayerBundle {
    player: Player,
    position: Position,
    render: Render,
}

#[derive(Default, Clone)]
struct GameState {
    map_loaded: bool,
    spawned: bool,
    collisions: HashSet<(i32, i32)>,
}

impl GameState {
    fn try_move_player(&mut self, position: &mut Position, delta_xy: (i32, i32)) {
        let new_pos = (position.x + delta_xy.0, position.y + delta_xy.1);
        if !self.collisions.contains(&new_pos) {
            position.x = new_pos.0;
            position.y = new_pos.1;
        }
    }
}

fn setup(mut tile_sprite_handles: ResMut<SpriteHandles>, asset_server: Res<AssetServer>) {
    tile_sprite_handles.handles = asset_server.load_folder("textures").unwrap();
}

fn load(
    mut commands: Commands,
    mut sprite_handles: ResMut<SpriteHandles>,
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
            .topology(GridTopology::HexEvenRows)
            .dimensions(1, 1)
            .chunk_dimensions(32, 38, 1)
            .texture_dimensions(32, 37)
            .add_layer(
                TilemapLayer {
                    kind: LayerKind::Dense,
                    ..Default::default()
                },
                0,
            )
            .add_layer(
                TilemapLayer {
                    kind: LayerKind::Sparse,
                    ..Default::default()
                },
                1,
            )
            .add_layer(
                TilemapLayer {
                    kind: LayerKind::Sparse,
                    ..Default::default()
                },
                2,
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

fn build_random_world(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    asset_server: Res<AssetServer>,
    mut query: Query<&mut Tilemap>,
) {
    if game_state.map_loaded {
        return;
    }

    for mut map in query.iter_mut() {
        // Since we did not `auto_chunk` in the builder, we must manually
        // insert a chunk. This will then communicate with us if we accidentally
        // insert a tile in a chunk we may not want. Also, we only expect to
        // have just 1 chunk.
        map.insert_chunk((0, 0)).unwrap();

        let chunk_width = (map.width().unwrap() * map.chunk_width()) as i32;
        let chunk_height = (map.height().unwrap() * map.chunk_height()) as i32;

        // Then we need to find out what the handles were to our textures we are going to use.
        let grass_floor: Handle<Texture> = asset_server.get_handle("textures/hex-floor-grass.png");
        let dirt_floor: Handle<Texture> = asset_server.get_handle("textures/hex-floor-dirt.png");
        let boulder: Handle<Texture> = asset_server.get_handle("textures/hex-boulder.png");
        let trees: Handle<Texture> = asset_server.get_handle("textures/hex-trees.png");
        let texture_atlas = texture_atlases.get(map.texture_atlas()).unwrap();
        let grass_index = texture_atlas.get_texture_index(&grass_floor).unwrap();
        let dirt_index = texture_atlas.get_texture_index(&dirt_floor).unwrap();
        let boulder_index = texture_atlas.get_texture_index(&boulder).unwrap();
        let trees_index = texture_atlas.get_texture_index(&trees).unwrap();

        // Now we fill the entire world with grass.
        let mut tiles = Vec::new();
        for y in 0..chunk_height {
            for x in 0..chunk_width {
                let y = y - chunk_height / 2;
                let x = x - chunk_width / 2;
                // By default tile sets the Z order at 0. Lower means that tile
                // will render lower than others. 0 is the absolute bottom
                // level which is perfect for backgrounds.
                let tile = Tile {
                    point: (x, y),
                    sprite_index: grass_index,
                    ..Default::default()
                };
                tiles.push(tile);
            }
        }

        // And lets surround our world with boulders.
        for x in 0..chunk_width {
            let x = x - chunk_width / 2;
            let tile_a = (x, -chunk_height / 2);
            let tile_b = (x, chunk_height / 2 - 1);
            tiles.push(Tile {
                point: tile_a,
                sprite_index: boulder_index,
                sprite_order: 1,
                ..Default::default()
            });
            tiles.push(Tile {
                point: tile_b,
                sprite_index: boulder_index,
                sprite_order: 1,
                ..Default::default()
            });
            game_state.collisions.insert(tile_a);
            game_state.collisions.insert(tile_b);
        }

        // Then the boulder tiles on the Y axis.
        for y in 0..chunk_height {
            let y = y - chunk_height / 2;
            let tile_a = (-chunk_width / 2, y);
            let tile_b = (chunk_width / 2 - 1, y);
            tiles.push(Tile {
                point: tile_a,
                sprite_index: boulder_index,
                sprite_order: 1,
                ..Default::default()
            });
            tiles.push(Tile {
                point: tile_b,
                sprite_index: boulder_index,
                sprite_order: 1,
                ..Default::default()
            });
            game_state.collisions.insert(tile_a);
            game_state.collisions.insert(tile_b);
        }
        // Lets just generate some random walls to sparsely place around the
        // world!
        let range = (chunk_width * chunk_height) as usize / 5;
        let mut rng = rand::thread_rng();
        for _ in 0..range {
            let x = rng.gen_range((-chunk_width / 2)..(chunk_width / 2));
            let y = rng.gen_range((-chunk_height / 2)..(chunk_height / 2));
            let coord = (x, y, 0i32);
            if coord != (0, 0, 0) {
                if rng.gen_bool(0.5) {
                    tiles.push(Tile {
                        point: (x, y),
                        sprite_index: boulder_index,
                        sprite_order: 1,
                        ..Default::default()
                    });
                } else {
                    tiles.push(Tile {
                        point: (x, y),
                        sprite_index: trees_index,
                        sprite_order: 1,
                        ..Default::default()
                    });
                }
                game_state.collisions.insert((x, y));
            }
        }
        // Lets finally vary it up and add some dirt patches.
        for _ in 0..range {
            let x = rng.gen_range((-chunk_width / 2)..(chunk_width / 2));
            let y = rng.gen_range((-chunk_height / 2)..(chunk_height / 2));
            tiles.push(Tile {
                point: (x, y),
                sprite_index: dirt_index,
                ..Default::default()
            });
        }

        // Now lets add in a dwarf friend!
        let dwarf_sprite: Handle<Texture> = asset_server.get_handle("textures/hex-dwarf.png");
        let dwarf_sprite_index = texture_atlas.get_texture_index(&dwarf_sprite).unwrap();
        // We add in a Z order of 2 to place the tile above the background on Z
        // order 0.
        let dwarf_tile = Tile {
            point: (0, 0),
            sprite_index: dwarf_sprite_index,
            sprite_order: 2,
            ..Default::default()
        };
        tiles.push(dwarf_tile);

        commands.spawn().insert_bundle(PlayerBundle {
            player: Player {},
            position: Position { x: 0, y: 0 },
            render: Render {
                sprite_index: dwarf_sprite_index,
                z_order: 2,
            },
        });

        // Now we pass all the tiles to our map.
        map.insert_tiles(tiles).unwrap();

        // Finally we spawn the chunk! In actual use this should be done in a
        // spawn system.
        map.spawn_chunk((0, 0)).unwrap();

        game_state.map_loaded = true;
    }
}

fn move_sprite(
    map: &mut Tilemap,
    previous_position: Position,
    position: Position,
    render: &Render,
) {
    // We need to first remove where we were prior.
    map.clear_tile((previous_position.x, previous_position.y), 2)
        .unwrap();
    // We then need to update where we are going!
    let tile = Tile {
        point: (position.x, position.y),
        sprite_index: render.sprite_index,
        sprite_order: render.z_order,
        ..Default::default()
    };
    map.insert_tile(tile).unwrap();
}

fn character_movement(
    mut game_state: ResMut<GameState>,
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut map_query: Query<(&mut Tilemap, &mut Timer)>,
    mut player_query: Query<(&mut Position, &Render, &Player)>,
) {
    if !game_state.map_loaded {
        return;
    }

    for (mut map, mut timer) in map_query.iter_mut() {
        timer.tick(time.delta());
        if !timer.finished() {
            continue;
        }

        for (mut position, render, _player) in player_query.iter_mut() {
            for key in keyboard_input.get_pressed() {
                // First we need to store our very current position.
                let previous_position = *position;

                // Of course we need to control where we are going to move our
                // dwarf friend. Since this map is hex, we most handle movement
                // different depending on if we are on an even Y axis or not.
                use KeyCode::*;
                if position.y % 2 == 0 {
                    match key {
                        W | Numpad8 | Up | Q | Numpad7 => {
                            game_state.try_move_player(&mut position, (0, 1));
                        }
                        A | Numpad4 | Left => {
                            game_state.try_move_player(&mut position, (-1, 0));
                        }
                        X | Numpad2 | Down | Z | Numpad1 => {
                            game_state.try_move_player(&mut position, (0, -1));
                        }
                        D | Numpad6 | Right => {
                            game_state.try_move_player(&mut position, (1, 0));
                        }

                        E | Numpad9 => {
                            game_state.try_move_player(&mut position, (1, 1));
                        }
                        C | Numpad3 => {
                            game_state.try_move_player(&mut position, (1, -1));
                        }

                        _ => {}
                    }
                } else {
                    match key {
                        W | Numpad8 | Up | E | Numpad9 => {
                            game_state.try_move_player(&mut position, (0, 1));
                        }
                        A | Numpad4 | Left => {
                            game_state.try_move_player(&mut position, (-1, 0));
                        }
                        X | Numpad2 | Down | C | Numpad3 => {
                            game_state.try_move_player(&mut position, (0, -1));
                        }
                        D | Numpad6 | Right => {
                            game_state.try_move_player(&mut position, (1, 0));
                        }

                        Q | Numpad7 => {
                            game_state.try_move_player(&mut position, (-1, 1));
                        }
                        Z | Numpad1 => {
                            game_state.try_move_player(&mut position, (-1, -1));
                        }

                        _ => {}
                    }
                }

                // If we are standing still or hit something, don't do anything.
                if previous_position == *position {
                    continue;
                }

                // This is a helpful function to make it easier to do stuff!
                move_sprite(&mut map, previous_position, *position, render);
            }
        }
    }
}
