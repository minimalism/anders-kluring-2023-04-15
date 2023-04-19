use bevy::{prelude::*};
use super::tile::GlobalPos;

pub struct ShapePermutation {
    pub index: usize,
    pub rotation: u8,
    pub flipped: bool
}

pub struct Shape {
    pub index: usize,
    bounds: (u8, u8),
    tiles: Vec<(u8, u8)>,
}

impl Shape {
    fn from_string(
        index: usize,
        string: &str,
    ) -> Shape {

        let mut bounds_x = 0;
        let mut tiles = Vec::new();

        let mut y = 0;
        for line in string.lines() {
            let mut x = 0;
            for char in line.chars() {
                if !char.is_whitespace() {
                    tiles.push((x, y));
                }
                x += 1;
            }
            y += 1;
            bounds_x = bounds_x.max(x);
        }

        return Shape {
            index,
            tiles,
            bounds: (bounds_x, y),
        };
    }

    pub fn iter_pos(&self) -> impl Iterator<Item=&(u8, u8)> + '_ {
        self.tiles.iter()
    }

    pub fn iter_globalpos(&self, offs: GlobalPos) -> impl Iterator<Item=GlobalPos> + '_ {
        self.tiles.iter().map(move |(pos_x, pos_y)| {
            let x = offs.x + *pos_x as i32;
            let y = offs.y + *pos_y as i32;

            GlobalPos { x, y }
        })
    }
    
}

#[derive(Resource)]
pub struct ShapeBag {
    remaining: Vec<u16>,
    vec: Vec<Shape>,
}

impl ShapeBag {

    pub fn iter_available(&self) -> impl Iterator<Item=&Shape> {
        self.vec.iter().filter(|shape| self.remaining[shape.index] > 0)
    }

    pub fn try_pop(&mut self, shape_index: usize) {
        self.remaining[shape_index] -= 1;
    }

    pub fn reset(&mut self, count: u16) {
        for i in 0..self.remaining.len() {
            self.remaining[i] = count;
        }
    }

    pub fn iter_globalpos(&self, permutation: &ShapePermutation, offs: GlobalPos) -> impl Iterator<Item=GlobalPos> + '_ {
        self.vec[permutation.index].iter_globalpos(offs)
    }

    pub fn get_random_permutation(&self) -> ShapePermutation {

        // TODO: pick random from available...
        ShapePermutation {
            index: 2,
            flipped: true,
            rotation: 2,
        }
    }

    pub fn load(count: u16) -> ShapeBag {
        let shapes = vec![
            Shape::from_string(0, "X
X
X
XX
X
XX"),
            Shape::from_string(1, " XXX
XX
 XX
  X"),
            Shape::from_string(2, " XXX
XX
X
XX"),
            Shape::from_string(3, " X
XXXX
 X
 X
 X"),
            Shape::from_string(4, "XXX
  X
  XXX
  X"),
            Shape::from_string(5, "  X
XXX
 XXX
  X"),
        ];
    
        ShapeBag { 
            remaining: vec![count; 6],
            vec: shapes,
        }
    
    }
    
}

