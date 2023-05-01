use std::collections::{HashSet, HashMap, hash_map::Entry};

use bevy::{prelude::*};
use bevy_ecs_tilemap::prelude::*;

use crate::kluring::{shape::Permutation, tile::{TILEMAP_SIZE, create_chunk}};

use self::{
    shape::{ShapeBag, ShapePermutation},
    ui::{ShowUiPlugin, InputFieldsState}, 
    tile::{TilePlugin, ChunkManager, BorderTile, CHUNK_SIZE, GlobalPos}
};

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
                scored_positions: HashMap::new(),
                bounds: Bounds::new(),
                attempts: 0,
            })
            .add_systems(
                (
                    find_best_shape,
                    place_shape,
                )
                .chain()
                .in_base_set(CoreSet::PreUpdate)
            )
            .add_system(
                update_boundary_score,
            )/*.run_if(on_event::<PlaceShapeEvent>())*/
            //.add_system(.run_if(on_event::<RecalculateBoundary>()))
            .add_systems(
                (
                    reset.run_if(on_event::<RestartEvent>()),
                    apply_system_buffers,
                )
            )
        ;
    }
}

pub struct PlaceShapeEvent {
    permutation: ShapePermutation,
    pos: GlobalPos,
}

#[derive(Resource)]
pub struct BoardState {
    scored_positions: HashMap<GlobalPos, i32>,
    bounds: Bounds,
    attempts: usize,
}

impl BoardState {
    fn is_taken(&self, pos: &GlobalPos) -> bool {
        self.scored_positions.get(pos).map_or(true, |x| *x == BLOCKED)
    }
}

const INITIAL: GlobalPos = GlobalPos {
    x: (CHUNK_SIZE / 2) as i32,
    y: (CHUNK_SIZE / 2) as i32,
};

const BLOCKED: i32 = i32::MIN;

fn find_best_shape(
    mut state: ResMut<BoardState>,
    bag: Res<ShapeBag>,
    border_query: Query<&BorderTile>,
    mut place_shape_event: EventWriter<PlaceShapeEvent>,
) {
    if state.scored_positions.len() == 0 {

        // degenerate case: just place any ole tile first.
        if let Some(permutation) = bag.get_random_permutation() {
            place_shape_event.send(PlaceShapeEvent {
                permutation,
                pos: INITIAL.clone(),
            });
        }


    } else {

        const MAX_ATTEMPTS: usize = 0;

        let best_positions = collect_candidate_positions(border_query);
        let mut attempts_count = 0;
        let mut best_attempts = Vec::new();

        'outer: for shape in bag.iter_available() {
            
            let permutation = ShapePermutation {
                index: shape.index,
                permutation: Permutation {
                    rotation: 0,
                    flipped: false,
                }
            };

            // Iterate every edge position
            for border_pos in best_positions.iter() {
                
                // Iterate every position in the shape as anchor
                let shape_positions = bag.iter_pos(&permutation);
                for shape_tile_pos in &shape_positions {
                    
                    let attempt_pos = *border_pos - *shape_tile_pos;
                    
                    if MAX_ATTEMPTS > 0 
                        && attempts_count > MAX_ATTEMPTS 
                        && best_attempts.len() > 1 {
                        break 'outer;
                    }

                    attempts_count += 1;
    
                    if let Some(score) = get_placement_score(
                        &attempt_pos,
                        &shape_positions,
                        &state,
                    ) {
                        best_attempts.push((score, permutation.clone(), attempt_pos));
                    }
                }
            }
        }

        // Take best attempt...
        best_attempts.sort_by_key(|x| x.0);
        if let Some((_, permutation, attempt_pos)) = best_attempts.pop() {
            place_shape_event.send(PlaceShapeEvent {
                permutation,
                pos: attempt_pos,
            });
        }

        state.attempts += attempts_count;
    }

}

fn place_shape(
    mut tilemap: Query<&mut TileStorage>,
    mut bag: ResMut<ShapeBag>,
    mut place_shape_events: EventReader<PlaceShapeEvent>,
    mut commands: Commands,
    mut state: ResMut<BoardState>,
    mut chunk_manager: ResMut<ChunkManager>,
    asset_server: Res<AssetServer>,
) {
    let mut border = HashSet::new();
    
    // First, group all tiles we want to place by their appropriate chunk...
    
    let mut tiles_per_chunk: HashMap<IVec2, Vec<(TilePos, usize)>> = HashMap::new();

    fn place_tile(
        shape_index: usize,
        tiles_per_chunk: &mut HashMap<IVec2, Vec<(TilePos, usize)>>,
        global_pos: &GlobalPos,
    ) {
        let (chunk_pos, tile_pos) = global_pos.to_chunk_pos();

        match tiles_per_chunk.entry(chunk_pos) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().push((tile_pos, shape_index));
            },
            Entry::Vacant(entry) => {
                entry.insert(vec![(tile_pos, shape_index)]);
            },
        }
    }

    for place_shape_event in place_shape_events.iter() {

        let shape = &place_shape_event.permutation;
        let attempt_pos = place_shape_event.pos;

        if !bag.try_pop(shape.index) {
            panic!("Tried to place shape that was unavailable.");
        }

        for shape_pos in bag.iter_pos(&shape) {
            
            let global_pos = shape_pos + attempt_pos;
            place_tile(shape.index, &mut tiles_per_chunk, &global_pos);
       
            for neighbor_pos in iter_moore(global_pos) {
                border.insert(neighbor_pos);
            }
    
            state.bounds.expand(&global_pos);

            if let Some(prev) = state.scored_positions.insert(global_pos, BLOCKED) {
                if prev == BLOCKED {
                    panic!("Overlapped old tile!");
                }
            }
        }
    }

    // update border...
    const BORDER_INDEX: usize = 6;
    for border_pos in border.iter() {

        //let (chunk_pos, neighbor) = to_chunk_pos(&border_pos);
        if !state.scored_positions.contains_key(&border_pos) {
            place_tile(BORDER_INDEX, &mut tiles_per_chunk, &border_pos);
        }
    }

    fn create_tiles(
        commands: &mut Commands,
        tilemap_entity: Entity,
        chunk_pos: IVec2,
        placed_tiles: Vec<(TilePos, usize)>,
    ) -> Vec<(TilePos, Entity)> {
        let mut placed = Vec::new();
        for (tile_pos, shape_index) in placed_tiles {

            //println!("Placing tile {} at {}, {}", shape_index, tile_pos.x, tile_pos.y);

            // in with the new
            let mut new_tile_commands = commands
                .spawn((
                    TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(tilemap_entity),
                        texture_index: TileTextureIndex(shape_index as u32),
                        color: TileColor(Color::Rgba { red: 1., green: 1., blue: 1., alpha: 1. }),
                        ..Default::default()
                    },
                ));

            if shape_index == BORDER_INDEX {
                new_tile_commands.insert(BorderTile {
                    adjacency_score: 0,
                    distance_score: 0,
                    global_pos: GlobalPos::from_chunk_tile(&chunk_pos, &tile_pos)
                });
            }

            let new_tile_id = new_tile_commands.id();

            placed.push((tile_pos, new_tile_id));
            
        }
        return placed;
    }

    // Then, step through all chunks and allocate tiles in the right chunk
    for (chunk_pos, placed_tiles) in tiles_per_chunk {
        
        if let Some(tilemap_entity) = chunk_manager.spawned_chunks.get(&chunk_pos) {
            //println!("Loading old chunk at {}, {}", chunk_pos.x, chunk_pos.y);
            // Get chunk by pos...
            let mut tile_storage = tilemap.get_mut(*tilemap_entity).unwrap();
            let placed_tiles = create_tiles(
                &mut commands,
                *tilemap_entity,
                chunk_pos,
                placed_tiles);

            for (tile_pos, new_tile_id) in placed_tiles {
                // out with the old
                if let Some(old_tile) = tile_storage.get(&tile_pos) {
                    //println!("Despawning old tile at {}, {}", tile_pos.x, tile_pos.y);
                    commands.entity(old_tile).despawn_recursive();
                    tile_storage.remove(&tile_pos);
                }
                //println!("Spawning new tile at {}, {}", tile_pos.x, tile_pos.y);
                tile_storage.set(&tile_pos, new_tile_id);
            }
        } else {
            //println!("Creating new chunk at {}, {}", chunk_pos.x, chunk_pos.y);
            // Create new chunk
            let mut tile_storage = TileStorage::empty(TILEMAP_SIZE);

            let tilemap_entity = commands.spawn_empty().id();

            chunk_manager.spawned_chunks.insert(chunk_pos, tilemap_entity);

            let placed_tiles = create_tiles(
                &mut commands,
                tilemap_entity,
                chunk_pos,
                placed_tiles);

            for (tile_pos, new_tile_id) in placed_tiles {
                // out with the old
                if let Some(old_tile) = tile_storage.get(&tile_pos) {
                    //println!("Despawning old tile at {}, {}", tile_pos.x, tile_pos.y);
                    commands.entity(old_tile).despawn_recursive();
                    tile_storage.remove(&tile_pos);
                }
                //println!("Spawning new tile at {}, {}", tile_pos.x, tile_pos.y);
                tile_storage.set(&tile_pos, new_tile_id);
            }

            create_chunk(tilemap_entity, tile_storage, &mut commands, &asset_server, chunk_pos);
        }
    }
}

fn update_boundary_score(
    mut state: ResMut<BoardState>,
    mut border_query: Query<(&mut BorderTile, &mut TileColor)>,
) {

    const MAX_ADJACENCY_SCORE: f32 = 4.;

    let center_of_mass = INITIAL.clone();   // todo

    if state.bounds.is_default() {
        return;
    }

    let max_distance = ((state.bounds.width().pow(2) + state.bounds.height().pow(2)) as f32).sqrt();

    for (mut border, mut color) in border_query.iter_mut() {

        border.adjacency_score = 0;
        for neighbor in iter_moore(border.global_pos) {
            if state.is_taken(&neighbor) {
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

        let score = border.distance_score + border.adjacency_score;
        if let Some(prev) = state.scored_positions.insert(border.global_pos, score) {
            if prev == BLOCKED {
                panic!("Overwrote blocked position at {}, {}", border.global_pos.x, border.global_pos.y);
            }
        }
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
    shape_positions: &Vec<GlobalPos>,
    state: &BoardState,
) -> Option<i32> {

    let mut score_sum = 0;

    let mut expanded_bounds = state.bounds.clone();

    for tile_pos in shape_positions { 
        let global_pos = *tile_pos + *offset;
        if let Some(score) = state.scored_positions.get(&global_pos) {
            if *score == BLOCKED {
                return None;
            }
            score_sum += score;
        }

        expanded_bounds.expand(tile_pos);
    }

    // sum empty tiles in bounds...?
    if false {
        let mut emptiness = 0;
        for x in expanded_bounds.min_x..expanded_bounds.max_x + 1 {
            for y in expanded_bounds.min_y..expanded_bounds.max_y + 1 {
                let pos = GlobalPos { x, y };
                if state.is_taken(&pos) {
                    emptiness += 1;
                } else {
                    emptiness -= 1;
                }
            }
        }

        score_sum += emptiness;
    }

    return Some(score_sum);
}

#[derive(Clone)]
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
    mut state: ResMut<BoardState>,
    chunk_manager: ResMut<ChunkManager>,
    mut tilemap: Query<&mut TileStorage>,
    input_fields: Query<&InputFieldsState>,
) {

    println!("=== RESET ===");

    //let (tile_storage_entity, mut tile_storage) = tilemap
    //    .get_single_mut().expect("Need a tilemap");

    // clear tiles
    for (global_pos, _score) in state.scored_positions.drain() {

        let (chunk_pos, tile_pos) = global_pos.to_chunk_pos();

        let tile_storage_entity = chunk_manager.spawned_chunks.get(&chunk_pos).unwrap();

        let mut tile_storage = tilemap.get_mut(*tile_storage_entity).unwrap();

        if let Some(entity) = tile_storage.get(&tile_pos) {
            commands.entity(entity).despawn_recursive();
            tile_storage.remove(&tile_pos);
        } else {
            panic!("Could not retrieve tile from storage");
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

    //state.scored_positions = HashMap::new();
    state.attempts = 0;
    state.bounds = Bounds::new();
}
