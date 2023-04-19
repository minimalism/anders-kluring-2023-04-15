use std::collections::HashSet;
use bevy_ecs_tilemap::prelude::*;

use bevy::prelude::*;

pub struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(ChunkManager::default())
            .add_startup_system(create_terrain);
    }
}

#[derive(Default, Debug, Resource)]
pub struct ChunkManager {
    pub spawned_chunks: HashSet<IVec2>,
}

pub const TILE_SIZE: f32 = 16.;
pub const CHUNK_SIZE: u32 = 64;

pub fn create_chunk(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    chunk_pos: IVec2
) {
    let texture_handle: Handle<Image> = asset_server.load("tiles.png");

    let w = CHUNK_SIZE;
    let h = CHUNK_SIZE;
    let map_size = TilemapSize { x: w, y: h };
    let tile_storage = TileStorage::empty(map_size);
    let tilemap_entity = commands.spawn_empty().id();

    let tile_size = TilemapTileSize { x: TILE_SIZE, y: TILE_SIZE };
    let grid_size = tile_size.into();
    let map_type = TilemapType::default();

    // get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0)
    const CHUNK_SIZEF: f32 = CHUNK_SIZE as f32 * TILE_SIZE;
    let transform = Transform::from_translation(Vec3::new(
        chunk_pos.x as f32 * CHUNK_SIZEF - CHUNK_SIZEF / 2.,
        chunk_pos.y as f32 * CHUNK_SIZEF - CHUNK_SIZEF / 2.,
        0.0,
    ));

    commands.entity(tilemap_entity).insert(TilemapBundle {
        grid_size,
        map_type,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(texture_handle),
        tile_size,
        frustum_culling: bevy_ecs_tilemap::FrustumCulling(false),
        transform,
        ..Default::default()
    });
}

fn create_terrain(
    commands: Commands,
    asset_server: Res<AssetServer>,
) {
    create_chunk(commands, asset_server, IVec2 { x: 0, y: 0 });
}

/*pub fn get_or_create_chunk(
    chunk_pos: IVec2,
) -> (&mut TileStorage, Entity){

}*/

pub fn to_chunk_pos(global: &GlobalPos) -> (IVec2, TilePos) {
    const CHUNK_SIZEI: i32 = CHUNK_SIZE as i32;
    let chunk_pos_x = global.x.div_euclid(CHUNK_SIZEI);
    let chunk_pos_y = global.y.div_euclid(CHUNK_SIZEI);

    let tile_pos_x = global.x.rem_euclid(CHUNK_SIZEI);
    let tile_pos_y = global.y.rem_euclid(CHUNK_SIZEI);

    (
        IVec2 {
            x: chunk_pos_x,
            y: chunk_pos_y,
        },
        TilePos {
            x: tile_pos_x as u32,
            y: tile_pos_y as u32,
        }
    )
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct GlobalPos {
    pub x: i32,
    pub y: i32,
}

#[derive(Component)]
pub struct BorderTile {
    pub adjacency_score: i32,
    pub distance_score: i32,
    pub global_pos: GlobalPos,
}

impl BorderTile {
    pub fn score(&self) -> i32 {
        self.adjacency_score + self.distance_score
    }
}