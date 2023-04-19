use std::collections::HashSet;

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use self::{shape::{ShapeBag, Shape, ShapePermutation}, ui::{ShowUiPlugin, InputFieldsState}, tile::{TilePlugin, ChunkManager, BorderTile, CHUNK_SIZE, GlobalPos, to_chunk_pos}};

mod shape;
mod ui;
mod tile;

pub struct KluringPlugin;

impl Plugin for KluringPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<RestartEvent>()
            .add_plugin(TilemapPlugin)
            .add_plugin(ShowUiPlugin)
            .add_plugin(TilePlugin)
            .add_event::<PlaceShapeEvent>()
            .insert_resource(ShapeBag::load(1))
            .insert_resource(BoardState { 
                positions: HashSet::new(),
                bounds: Bounds::new(),
                attempts: 0,
            })
            .add_systems((find_best_shape, place_shape, update_boundary_score).chain())/*.run_if(on_event::<PlaceShapeEvent>())*/
            .add_system(reset.run_if(on_event::<RestartEvent>()))
        ;
    }
}

pub struct PlaceShapeEvent {
    permutation: ShapePermutation,
    pos: GlobalPos,
}

#[derive(Resource)]
pub struct BoardState {
    positions: HashSet<GlobalPos>,
    bounds: Bounds,
    attempts: usize,
}

const INITIAL: GlobalPos = GlobalPos {
    x: (CHUNK_SIZE / 2) as i32,
    y: (CHUNK_SIZE / 2) as i32,
};

fn find_best_shape(
    mut state: ResMut<BoardState>,
    bag: Res<ShapeBag>,
    border_query: Query<&BorderTile>,
    mut place_shape_event: EventWriter<PlaceShapeEvent>,
) {
    if state.positions.len() == 0 {

        // degenerate case: just place any ole tile first.
        let permutation = bag.get_random_permutation();

        place_shape_event.send(PlaceShapeEvent {
            permutation,
            pos: INITIAL.clone(),
        });

    } else {

        let best_positions = collect_candidate_positions(border_query);

        for shape in bag.iter_available() {

            let permutation = ShapePermutation {
                index: shape.index,
                rotation: 0,
                flipped: false,
            };

            for border_pos in best_positions.iter() {
                
                for attempt_pos in bag.iter_globalpos(&permutation, border_pos.clone()) {
    
                    state.attempts += 1;
    
                    if get_placement_score(
                        &attempt_pos,
                        &permutation,
                        &bag,
                        &state,
                    ).is_some() {
                        place_shape_event.send(PlaceShapeEvent {
                            permutation,
                            pos: attempt_pos,
                        });

                        return;
                    }
                }
            }
        }
    }

}

fn place_shape(
    mut tilemap: Query<(&mut TileStorage, Entity)>,
    mut bag: ResMut<ShapeBag>,
    mut place_shape_events: EventReader<PlaceShapeEvent>,
    mut commands: Commands,
    mut state: ResMut<BoardState>,
) {
    let mut border = HashSet::new();

    let (mut tile_storage, tilemap_entity) = tilemap
        .get_single_mut().expect("Need a tilemap");

    for place_shape_event in place_shape_events.iter() {

        let shape = &place_shape_event.permutation;
        let attempt_pos = place_shape_event.pos;

        bag.try_pop(shape.index);

        for global_pos in bag.iter_globalpos(&shape, attempt_pos.clone()) {
    
            let (chunk_pos, tile_pos) = to_chunk_pos(&global_pos);
    
            // let (tile_storage, tilemap_entity) = get_or_create_chunk(chunk_pos);
    
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
    
            for neighbor_pos in iter_moore(global_pos) {
                border.insert(neighbor_pos);
            }
    
            state.bounds.expand(&global_pos);
            state.positions.insert(global_pos);
        }
    
    }

    // update border...
    for border_pos in border.iter() {
    
        let (chunk_pos, neighbor) = to_chunk_pos(&border_pos);

        if tile_storage.get(&neighbor).is_none() {

            let boundary_tile = commands
                .spawn((
                    TileBundle {
                        position: neighbor,
                        tilemap_id: TilemapId(tilemap_entity),
                        texture_index: TileTextureIndex(6),
                        // color: TileColor(Color::Rgba { red: 1., green: 0., blue: 0., alpha: 0.5 }),
                        ..Default::default()
                    },
                ))
                .insert(BorderTile {
                    adjacency_score: 0,
                    distance_score: 0,
                    global_pos: border_pos.clone()
                })
                .id();

            tile_storage.set(&neighbor, boundary_tile);
        }
    }

}

fn update_boundary_score(
    state: Res<BoardState>,
    mut border_query: Query<(&mut BorderTile, &mut TileColor)>,
) {

    const MAX_ADJACENCY_SCORE: f32 = 4.;

    let center_of_mass = INITIAL.clone();   // todo

    let max_distance = ((state.bounds.width().pow(2) + state.bounds.height().pow(2)) as f32).sqrt();

    for (mut border, mut color) in border_query.iter_mut() {

        border.adjacency_score = 0;
        for neighbor in iter_moore(border.global_pos) {
            if state.positions.contains(&neighbor) {
                border.adjacency_score += 1;
            }
        }

        let distance_x = (border.global_pos.x - center_of_mass.x).abs() as f32;
        let distance_y = (border.global_pos.y - center_of_mass.y).abs() as f32;
        let distance = (distance_x.powi(2) + distance_y.powi(2)).sqrt();

        let normalized_distance = (max_distance - distance) / max_distance;
        border.distance_score = (normalized_distance * 10.) as i32;
        
        // calculate adjacency score and update color
        color.0 = Color::rgba(
            border.adjacency_score as f32 / MAX_ADJACENCY_SCORE, 
            normalized_distance,
            0.,
            1.);
    }
}


fn collect_candidate_positions(border_query: Query<&BorderTile>) -> Vec<GlobalPos> {

    let mut border_tiles: Vec<&BorderTile> = border_query
        .iter()
        .collect();

    border_tiles.sort_by(|a, b| b.score().cmp(&a.score()));

    let mut border: Vec<GlobalPos> = border_tiles
        .iter()
        .map(|border| border.global_pos.clone())
        .collect();

    if border.len() == 0 {

        // start at center...
        let x = (CHUNK_SIZE / 2) as i32;
        let y = (CHUNK_SIZE / 2) as i32;

        border.push(GlobalPos { x, y });
    }

    return border;
}

fn iter_moore(tile_pos: GlobalPos) -> impl Iterator<Item = GlobalPos> {
    const NEIGHBORHOOD: [(i32, i32); 4] = [
        ( 1, 0),
        ( 0, 1),
        (-1, 0),
        ( 0,-1),
    ];

    NEIGHBORHOOD.iter().map(move |xy| GlobalPos { 
        x: (tile_pos.x + xy.0),
        y: (tile_pos.y + xy.1),
     })
}

fn get_placement_score(
    offset: &GlobalPos,
    shape: &ShapePermutation,
    bag: &ShapeBag,
    state: &BoardState,
) -> Option<i32> {

    let score = 1;

    for tile_pos in bag.iter_globalpos(&shape, offset.clone()) { 
        if state.positions.get(&tile_pos).is_some() {
            return None;
        }
    }
    return Some(score);
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

    fn width(&self) -> i32 {
        self.max_x - self.min_x + 1
    }
    
    fn height(&self) -> i32 {
        self.max_y - self.min_y + 1
    }

    fn expand(&mut self, global_pos: &GlobalPos) {

        self.max_x = global_pos.x.max(self.max_x);
        self.max_y = global_pos.y.max(self.max_y);
        self.min_x = global_pos.x.min(self.min_x);
        self.min_y = global_pos.y.min(self.min_y);
    }    
}


pub struct RestartEvent {}

fn reset(
    mut shapes: ResMut<ShapeBag>,
    mut commands: Commands,
    mut tilemap: Query<&mut TileStorage>,
    mut state: ResMut<BoardState>,
    border: Query<&TilePos, With<BorderTile>>,
    input_fields: Query<&InputFieldsState>,
) {
    
    let mut tile_storage = tilemap
        .get_single_mut().expect("Need a tilemap");

    // clear tiles
    for global_pos in state.positions.drain() {

        let (chunk_pos, tile_pos) = to_chunk_pos(&global_pos);

        if let Some(entity) = tile_storage.get(&tile_pos) {
            commands.entity(entity).despawn_recursive();
            tile_storage.remove(&tile_pos);
        }
    }

    // clear borders
    for tile_pos in border.iter() {
        if let Some(entity) = tile_storage.get(&tile_pos) {
            commands.entity(entity).despawn_recursive();
            tile_storage.remove(&tile_pos);
        }
    }

    // apparently we get one state per input widget
    // but whatever
    let mut count = 1;
    for input_field in input_fields.iter() {
        if let Ok(n) = input_field.n.parse::<u16>() {
            count = n;
        }
        break;
    }
    

    shapes.reset(count);

    state.positions = HashSet::new();
    state.attempts = 0;
    state.bounds = Bounds::new();
}