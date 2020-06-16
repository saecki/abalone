use std::fmt;
use std::ops::{Deref, DerefMut};

const UNIT_X: Position = Position { x: 1, y: 0 };
const UNIT_Y: Position = Position { x: 0, y: 1 };
const UNIT_Z: Position = Position { x: 1, y: 1 };

#[derive(Clone, Debug, PartialEq)]
pub struct Game {
    pub balls: Vec<Ball>,
}

/// Coordinates representing the position of a ball in the following coordinate
/// system where ```*``` represents all possible positions.
///
///              0 1 2 3 4 5 6 7 8
///            #------------------ x
///         0 / * * * * * . . . .
///        1 / * * * * * * . . .
///       2 / * * * * * * * . .
///      3 / * * * * * * * * .
///     4 / * * * * * * * * *
///    5 / . * * * * * * * *
///   6 / . . * * * * * * *
///  7 / . . . * * * * * *
/// 8 / . . . . * * * * *
///  y
#[derive(Clone, Debug, PartialEq)]
pub struct Ball {
    pub color: Color,
    pub pos: Position,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Color {
    Black,
    White,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Direction {
    X,
    Y,
    Z,
}

impl Position {
    pub fn push(&mut self, distance: i32, dir: &Direction) {
        match dir {
            Direction::X => self.x += distance,
            Direction::Y => self.y += distance,
            Direction::Z => {
                self.x += distance;
                self.y += distance;
            }
        }
    }

    pub fn pushed(&self, distance: i32, dir: &Direction) -> Self {
        match dir {
            Direction::X => Self {
                x: self.x + distance,
                y: self.y,
            },
            Direction::Y => Self {
                x: self.x,
                y: self.y + distance,
            },
            Direction::Z => Self {
                x: self.x + distance,
                y: self.y + distance,
            },
        }
    }
}

impl Deref for Ball {
    type Target = Position;

    fn deref(&self) -> &Position {
        &self.pos
    }
}

impl DerefMut for Ball {
    fn deref_mut(&mut self) -> &mut Position {
        &mut self.pos
    }
}

impl Ball {
    pub fn new(color: Color, x: i32, y: i32) -> Self {
        Ball {
            color,
            pos: Position { x, y },
        }
    }

    pub fn black(x: i32, y: i32) -> Self {
        Ball {
            color: Color::Black,
            pos: Position { x, y },
        }
    }

    pub fn white(x: i32, y: i32) -> Self {
        Ball {
            color: Color::White,
            pos: Position { x, y },
        }
    }
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let buf = String::new();


        f.write_str(&buf)
    }
}

impl Game {
    /// Returns a new game with the default start position as shown below:
    ///
    ///              0 1 2 3 4 5 6 7 8
    ///            # - - - - - - - - - x
    ///         0 / b b b b b . . . .
    ///        1 / b b b b b b . . .
    ///       2 / * * b b b * * . .
    ///      3 / * * * * * * * * .
    ///     4 / * * * * * * * * *
    ///    5 / . * * * * * * * *
    ///   6 / . . * * w w w * *
    ///  7 / . . . w w w w w w
    /// 8 / . . . . w w w w w
    ///  y
    pub fn new() -> Self {
        let mut balls = Vec::new();

        for i in 0..5 {
            balls.push(Ball::black(i, 0));
        }
        for i in 0..6 {
            balls.push(Ball::black(i, 1));
        }
        for i in 2..5 {
            balls.push(Ball::black(i, 2));
        }

        for i in 0..5 {
            balls.push(Ball::white(i, 8));
        }
        for i in 0..6 {
            balls.push(Ball::white(i, 7));
        }
        for i in 4..7 {
            balls.push(Ball::white(i, 6));
        }

        Game { balls }
    }

    pub fn ball(&self, pos: &Position) -> Option<&Ball> {
        self.balls.iter()
            .find(|b| &b.pos == pos)
    }

    pub fn is_pushable(&self, ball: Ball, dir: &Direction) -> bool {
        let mut force = 1;
        let mut counterforce = 0;

        while let Some(b) = self.ball(&ball.pos.pushed(force, dir)) {
            if b.color == ball.color {
                force += 1;
            } else {
                break;
            }
        }

        if force > 3 {
            return false;
        }

        while let Some(b) = self.ball(&ball.pos.pushed(force + counterforce + 1, dir)) {
            if b.color != ball.color {
                counterforce += 1;
            } else {
                return false;
            }
        }

        force > counterforce
    }

    pub fn are_pushable(&self, balls: Vec<Ball>, dir: Direction) -> bool {
        if balls.len() > 3 && balls.len() < 1 {
            return false;
        }

        for b in balls {
            if let Some(_) = self.ball(&b.pos.pushed(1, &dir)) {
                return false;
            }
        }

        true
    }
}

fn main() {
    let game = Game::new();

    println!("{:#?}", game);
}
