use std::collections::HashSet;

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use self::{shape::{ShapeBag, Shape}, ui::ShowUiPlugin};

mod shape;
mod ui;

pub struct KluringPlugin;

impl Plugin for KluringPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugin(TilemapPlugin)
            .add_plugin(ShowUiPlugin)
            .add_startup_system(create_terrain)
            .insert_resource(ShapeBag::load(1))
            .insert_resource(BoardState { 
                positions: HashSet::new(),
                bounds: Bounds::new(),
            })
            .add_system(place_shape/*.run_if(on_event::<PlaceShapeEvent>())*/)
        ;
    }
}

#[derive(Resource)]
pub struct BoardState {
    positions: HashSet<TilePos>,
    bounds: Bounds,
}

struct GlobalPos {
    x: i32,
    y: i32,
}

fn place_shape(
    mut shapes: ResMut<ShapeBag>,
    mut commands: Commands,
    mut tilemap: Query<(&mut TileStorage, Entity)>,
    mut state: ResMut<BoardState>,
    border: Query<(&BorderTile, &TilePos)>,
) {
    let (mut tile_storage, tilemap_entity) = tilemap
        .get_single_mut().expect("Need a tilemap");

    let candidate_positions = collect_candidate_positions(border);

    let available_shapes: Vec<&Shape> = shapes.iter_available().collect();

    let mut placed_shape: Option<usize> = None;
    
    'outer: for shape in available_shapes {
        for border_pos in candidate_positions.iter() {
            
            for origin_pos in shape.iter_pos() {

                let attempt_pos = TilePos {
                    x: border_pos.x + origin_pos.0 as u32,
                    y: border_pos.y + origin_pos.1 as u32,
                };

                if is_valid_placement(
                    &attempt_pos,
                    shape,
                    &state,
                ) {

                    let mut border = HashSet::new();
            
                    for tile_pos in shape.iter_tilepos(attempt_pos) {
            
                        // out with the old
                        if let Some(old_tile) = tile_storage.get(&tile_pos) {
                            commands.entity(old_tile).despawn_recursive();
                            tile_storage.remove(&tile_pos);
                        }
                
                        // in with the new
                        let new_tile = commands
                            .spawn((
                                TileBundle {
                                    position: tile_pos,
                                    tilemap_id: TilemapId(tilemap_entity),
                                    texture_index: TileTextureIndex(shape.index as u32),
                                    ..Default::default()
                                },
                            ))
                            .id();
            
                        tile_storage.set(&tile_pos, new_tile);
                        commands.entity(tilemap_entity).add_child(new_tile); // add tile as a child of tilemap to keep inspector view clean
            
                        for neighbor_pos in iter_moore(tile_pos) {
                            border.insert(neighbor_pos);
                        }

                        // record in state...
                        let global_pos = GlobalPos {
                            x: tile_pos.x as i32,
                            y: tile_pos.y as i32,
                        };

                        state.positions.insert(tile_pos);

                        state.bounds.expand(&global_pos);
                    }
            
                    for neighbor in border.iter() {
                        if tile_storage.get(&neighbor).is_none() {
            
                            let boundary_tile = commands
                                .spawn((
                                    TileBundle {
                                        position: *neighbor,
                                        tilemap_id: TilemapId(tilemap_entity),
                                        texture_index: TileTextureIndex(6),
                                        ..Default::default()
                                    },
                                ))
                                .insert(BorderTile {
            
                                })
                                .id();
            
            
                            tile_storage.set(neighbor, boundary_tile)
                        }
                    }

                    placed_shape = Some(shape.index);
                    break 'outer;
                }
            }
        }
    }

    if let Some(shape_index) = placed_shape {
        shapes.try_pop(shape_index);
    }
}


fn collect_candidate_positions(border_query: Query<(&BorderTile, &TilePos)>) -> Vec<TilePos> {
    let mut border: Vec<TilePos> = border_query.iter().map(|(_border, pos)| *pos).collect();

    if border.len() == 0 {

        // start at center...
        let x = MAP_WIDTH / 2;
        let y = MAP_HEIGHT / 2;

        border.push(TilePos { x, y });
    }

    return border;
}

fn iter_moore(tile_pos: TilePos) -> impl Iterator<Item = TilePos> {
    const NEIGHBORHOOD: [(i32, i32); 4] = [
        ( 1, 0),
        ( 0, 1),
        (-1, 0),
        ( 0,-1),
    ];

    NEIGHBORHOOD.iter().map(move |xy| TilePos { 
        x: (tile_pos.x as i32 + xy.0) as u32,
        y: (tile_pos.y as i32 + xy.1) as u32,
     })
}

fn is_valid_placement(
    offset: &TilePos,
    shape: &Shape,
    state: &BoardState,
) -> bool {
    for tile_pos in shape.iter_tilepos(*offset) { 
        if state.positions.get(&tile_pos).is_some() {
            return false;
        }
    }
    return true;
}

pub const MAP_WIDTH: u32 = 64;
pub const MAP_HEIGHT: u32 = 64;
pub const TILE_SIZE: f32 = 16.;

fn create_terrain(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let texture_handle: Handle<Image> = asset_server.load("tiles.png");


    let w = MAP_WIDTH;
    let h = MAP_HEIGHT;
    let map_size = TilemapSize { x: w, y: h };
    let tile_storage = TileStorage::empty(map_size);
    let tilemap_entity = commands.spawn_empty().id();

    let tile_size = TilemapTileSize { x: TILE_SIZE, y: TILE_SIZE };
    let grid_size = tile_size.into();
    let map_type = TilemapType::default();

    commands.entity(tilemap_entity).insert(TilemapBundle {
        grid_size,
        map_type,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(texture_handle),
        tile_size,
        frustum_culling: bevy_ecs_tilemap::FrustumCulling(false),
        transform: get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0),
        ..Default::default()
    });
}


#[derive(Component)]
struct BorderTile {

}

pub struct Bounds {
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
}

impl Bounds {
    fn new() -> Bounds {
        Bounds {
            min_x: i32::MAX,
            max_x: i32::MIN,
            min_y: i32::MAX,
            max_y: i32::MIN,
        }
    }

    fn is_default(&self) -> bool {
        self.min_x == i32::MAX &&
        self.max_x == i32::MIN &&
        self.min_y == i32::MAX &&
        self.max_y == i32::MIN

    }

    fn expand(&mut self, global_pos: &GlobalPos) {

        self.max_x = global_pos.x.max(self.max_x);
        self.max_y = global_pos.y.max(self.max_y);
        self.min_x = global_pos.x.min(self.min_x);
        self.min_y = global_pos.y.min(self.min_y);
    }    
}