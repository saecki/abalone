use std::{fmt, error};
use std::ops::{Add, Deref, DerefMut, Div, Mul, Sub};
use std::convert::TryFrom;

const UNIT_X: Pos = Pos { x: 1, y: 0 };
const UNIT_Y: Pos = Pos { x: 0, y: 1 };
const UNIT_Z: Pos = Pos { x: 1, y: 1 };

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
    pub pos: Pos,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Color {
    Black,
    White,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Dir {
    X,
    Y,
    Z,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Mag {
    Pos,
    Neg,
}

impl Mag {
    pub fn to_i32(&self) -> i32 {
        match self {
            Mag::Pos => 1,
            Mag::Neg => -1,
        }
    }
}

impl Dir {
    pub fn to_pos(&self) -> Pos {
        match self {
            Self::X => UNIT_X,
            Self::Y => UNIT_Y,
            Self::Z => UNIT_Z,
        }
    }
}

impl<'a, 'b> Add<&'b Pos> for &'a Pos {
    type Output = Pos;

    fn add(self, rhs: &'b Pos) -> Self::Output {
        Self::Output {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl<'a, 'b> Sub<&'b Pos> for &'a Pos {
    type Output = Pos;

    fn sub(self, rhs: &'b Pos) -> Self::Output {
        Self::Output {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl<'a> Mul<i32> for &'a Pos {
    type Output = Pos;

    fn mul(self, rhs: i32) -> Self::Output {
        Self::Output {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl<'a> Div<i32> for &'a Pos {
    type Output = Pos;

    fn div(self, rhs: i32) -> Self::Output {
        Self::Output {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Pos {
    pub fn push(&mut self, distance: i32, dir: &Dir) {
        match dir {
            Dir::X => self.x += distance,
            Dir::Y => self.y += distance,
            Dir::Z => {
                self.x += distance;
                self.y += distance;
            }
        }
    }

    pub fn pushed(&self, distance: i32, dir: &Dir) -> Self {
        match dir {
            Dir::X => Self {
                x: self.x + distance,
                y: self.y,
            },
            Dir::Y => Self {
                x: self.x,
                y: self.y + distance,
            },
            Dir::Z => Self {
                x: self.x + distance,
                y: self.y + distance,
            },
        }
    }

    pub fn abs(&self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }
}

impl Deref for Ball {
    type Target = Pos;

    fn deref(&self) -> &Pos {
        &self.pos
    }
}

impl DerefMut for Ball {
    fn deref_mut(&mut self) -> &mut Pos {
        &mut self.pos
    }
}

impl Ball {
    pub fn new(color: Color, x: i32, y: i32) -> Self {
        Ball {
            color,
            pos: Pos { x, y },
        }
    }

    pub fn black(x: i32, y: i32) -> Self {
        Ball {
            color: Color::Black,
            pos: Pos { x, y },
        }
    }

    pub fn white(x: i32, y: i32) -> Self {
        Ball {
            color: Color::White,
            pos: Pos { x, y },
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
    ///               0 1 2 3 4 5 6 7 8
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

    pub fn ball(&self, pos: &Pos) -> Option<&Ball> {
        self.balls.iter().find(|b| &b.pos == pos)
    }

    pub fn is_pushable(&self, ball: &Ball, mag: Mag, dir: &Dir) -> bool {
        let mut force = 1;
        let mut counterforce = 0;

        while let Some(b) = self.ball(&ball.pos.pushed(force * mag.to_i32(), dir)) {
            if b.color == ball.color {
                force += 1;
            } else {
                break;
            }
        }

        if force > 3 {
            return false;
        }

        let mut dist = (force + counterforce + 1) * mag.to_i32();
        while let Some(b) = self.ball(&ball.pos.pushed(dist, dir)) {
            if b.color != ball.color {
                counterforce += 1;
                dist += 1;
            } else {
                return false;
            }
        }

        force > counterforce
    }

    pub fn are_pushable(&self, balls: Vec<&Ball>, mag: Mag, dir: &Dir) -> bool {
        if balls.len() > 3 || balls.len() < 1 {
            return false;
        }

        if balls.len() == 3 {
            let dir1 = &balls[0].pos - &balls[1].pos;
            let dir2 = &balls[1].pos - &balls[2].pos;

            if dir1 != dir2 {
                return false;
            }
        }



        for b in balls {
            if let Some(_) = self.ball(&b.pos.pushed(mag.to_i32(), dir)) {
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
