use bevy::{prelude::*};
use super::tile::GlobalPos;
use rand::Rng;

#[derive(Copy, Clone)]
pub struct ShapePermutation {
    pub index: usize,
    pub permutation: Permutation,
}

pub const PERMUTATIONS: u8 = 8;


#[derive(Copy, Clone)]
pub struct Permutation {
    pub rotation: u8,
    pub flipped: bool,
}

impl Permutation {
    pub fn from_index(index: u8) -> Permutation {
        Permutation { 
            rotation: index % 4,
            flipped: index >= 4,
        }
    }
}

pub struct Shape {
    pub index: usize,
    bounds: (i32, i32),
    pub tiles: Vec<GlobalPos>,
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
                    tiles.push(GlobalPos { x, y });
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

    pub fn try_pop(&mut self, shape_index: usize) -> bool {

        if self.remaining[shape_index] > 0 {
            self.remaining[shape_index] -= 1;
            return true;
        }
        return false;
    }

    pub fn reset(&mut self, count: u16) {
        for i in 0..self.remaining.len() {
            self.remaining[i] = count;
        }
    }

    pub fn iter_pos(&self, shape_permutation: &ShapePermutation) -> Vec<GlobalPos> {
        let mut ret = self.vec[shape_permutation.index].tiles.clone();

        let rotations = shape_permutation.permutation.rotation % 4;
        if rotations != 0 {
            let rotation_matrix = match rotations {
                1 => [[0, -1], [1, 0]],   // 90 degrees counterclockwise
                2 => [[-1, 0], [0, -1]],  // 180 degrees
                3 => [[0, 1], [-1, 0]],  // 270 degrees counterclockwise
                _ => [[1, 0], [0, 1]],   // 0 degrees (identity matrix)
            };

            for point in &mut ret {
                let x = point.x * rotation_matrix[0][0] + point.y * rotation_matrix[0][1];
                let y = point.x * rotation_matrix[1][0] + point.y * rotation_matrix[1][1];
                point.x = x;
                point.y = y;
            }
        }

        if shape_permutation.permutation.flipped {
            for point in &mut ret {
                point.x = -point.x;
                //point.y = y;
            }
        }

        ret
    }

    pub fn get_random_permutation(&self) -> Option<ShapePermutation> {
        let mut rng = rand::thread_rng();

        let index = rng.gen::<usize>() % self.remaining.len();
        let flipped = rng.gen::<i32>() % 2 == 0;
        let rotation = rng.gen::<u8>() % 4;

        if self.remaining[index] > 0 {
            return Some(ShapePermutation {
                index,
                permutation: Permutation {
                    flipped,
                    rotation,
                },
            });
        }
        return None;
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

