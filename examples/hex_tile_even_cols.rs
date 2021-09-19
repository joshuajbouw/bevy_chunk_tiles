use bevy::{asset::LoadState, prelude::*, sprite::TextureAtlasBuilder, window::WindowMode};
use bevy_tilemap::prelude::*;

fn main() {
    App::build()
        .insert_resource(WindowDescriptor {
            title: "Hex Even Columns".to_string(),
            width: 1024.,
            height: 720.,
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
        .add_system(build_world.system())
        .run()
}

#[derive(Default, Clone)]
struct SpriteHandles {
    handles: Vec<HandleUntyped>,
    atlas_loaded: bool,
}

#[derive(Default, Clone)]
struct GameState {
    map_loaded: bool,
    spawned: bool,
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

        let tilemap = Tilemap::builder()
            .auto_chunk()
            .auto_spawn(2, 2)
            .topology(GridTopology::HexEvenCols)
            .dimensions(3, 3)
            .chunk_dimensions(8, 4, 1)
            .texture_dimensions(37, 32)
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

fn build_world(
    mut game_state: ResMut<GameState>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    asset_server: Res<AssetServer>,
    mut query: Query<&mut Tilemap>,
) {
    if game_state.map_loaded {
        return;
    }

    for mut map in query.iter_mut() {
        let chunk_width = (map.width().unwrap() * map.chunk_width()) as i32;
        let chunk_height = (map.height().unwrap() * map.chunk_height()) as i32;

        let grass_floor: Handle<Texture> =
            asset_server.get_handle("textures/hex-floor-grass_alt.png");
        let texture_atlas = texture_atlases.get(map.texture_atlas()).unwrap();
        let grass_index = texture_atlas.get_texture_index(&grass_floor).unwrap();

        let mut tiles = Vec::new();
        for y in 0..chunk_height {
            for x in 0..chunk_width {
                let y = y - chunk_height / 2;
                let x = x - chunk_width / 2;
                let tile = Tile {
                    point: (x, y),
                    sprite_index: grass_index,
                    ..Default::default()
                };
                tiles.push(tile);
            }
        }
        map.insert_tiles(tiles).unwrap();

        map.spawn_chunk((-1, 0)).unwrap();
        map.spawn_chunk((0, 0)).unwrap();
        map.spawn_chunk((1, 0)).unwrap();
        map.spawn_chunk((-1, 1)).unwrap();
        map.spawn_chunk((0, 1)).unwrap();
        map.spawn_chunk((1, 1)).unwrap();
        map.spawn_chunk((-1, -1)).unwrap();
        map.spawn_chunk((0, -1)).unwrap();
        map.spawn_chunk((1, -1)).unwrap();

        game_state.map_loaded = true;
    }
}
