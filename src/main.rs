use std::{fmt, ops};

#[cfg(test)]
mod test;

const UNIT_X: Pos = Pos { x: 1, y: 0 };
const UNIT_Y: Pos = Pos { x: 0, y: 1 };
const UNIT_Z: Pos = Pos { x: 1, y: 1 };

const SIZE: i8 = 9;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Success {
    /// Pushed opposing color, off the board.
    PushedOff {
        /// First ball that was pushed.
        first: Pos,
        /// Last opposing ball that was pushed off.
        last: Pos,
    },
    /// Pushed opposing color, but not off the board.
    PushedAway {
        /// First ball that was pushed.
        first: Pos,
        /// Last opposing ball that was away off.
        last: Pos,
    },
    /// Moved without resistance.
    Moved {
        /// First ball that was pushed.
        first: Pos,
        /// Last ball, of the same color, that was pushed.
        last: Pos,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// No ball was found ad the position.
    NotABall(Pos),
    /// Would push off your own ball.
    PushedOff(Pos),
    /// A ball off your own color, is blocking you from pushing away opposing balls.
    Mixed(Pos),
    /// More than 3 balls.
    TooMany {
        /// First own ball.
        first: Pos,
        /// The fourth own ball,
        fourth: Pos,
    },
    /// More or the same amount of opposing balls.
    TooManyOpposing {
        /// First opposing ball.
        first: Pos,
        /// Last opposing ball.
        last: Pos,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Game {
    pub balls: [[Option<Color>; SIZE as usize]; SIZE as usize],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Black,
    White,
}

impl Color {
    fn opposite(&self) -> Self {
        match self {
            Self::Black => Self::White,
            Self::White => Self::Black,
        }
    }
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pos {
    pub x: i8,
    pub y: i8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    PosX,
    PosY,
    PosZ,
    NegX,
    NegY,
    NegZ,
}

impl Dir {
    pub fn vec(&self) -> Pos {
        match self {
            Self::PosX => UNIT_X,
            Self::PosY => UNIT_Y,
            Self::PosZ => UNIT_Z,
            Self::NegX => -UNIT_X,
            Self::NegY => -UNIT_Y,
            Self::NegZ => -UNIT_Z,
        }
    }
}

impl<'a, 'b> ops::Neg for Pos {
    type Output = Pos;

    fn neg(self) -> Self::Output {
        Self::Output {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl<'a, 'b> ops::Add<Pos> for Pos {
    type Output = Pos;

    fn add(self, rhs: Pos) -> Self::Output {
        Self::Output {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl<'a, 'b> ops::Sub<Pos> for Pos {
    type Output = Pos;

    fn sub(self, rhs: Pos) -> Self::Output {
        Self::Output {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl<'a> ops::Mul<i8> for Pos {
    type Output = Pos;

    fn mul(self, rhs: i8) -> Self::Output {
        Self::Output {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl From<(i8, i8)> for Pos {
    fn from((x, y): (i8, i8)) -> Self {
        Self { x, y }
    }
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let buf = String::new();

        f.write_str(&buf)
    }
}

impl<P: Into<Pos>> ops::Index<P> for Game {
    type Output = Option<Color>;

    fn index(&self, index: P) -> &Self::Output {
        let Pos { x, y } = index.into();
        &self.balls[y as usize][x as usize]
    }
}

impl<P: Into<Pos>> ops::IndexMut<P> for Game {
    fn index_mut(&mut self, index: P) -> &mut Self::Output {
        let Pos { x, y } = index.into();
        &mut self.balls[y as usize][x as usize]
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
        let mut game = Self {
            balls: [[None; SIZE as usize]; SIZE as usize],
        };

        for i in 0..5 {
            game[(i, 0)] = Some(Color::Black);
        }
        for i in 0..6 {
            game[(i, 1)] = Some(Color::Black);
        }
        for i in 2..5 {
            game[(i, 2)] = Some(Color::Black);
        }

        for i in 0..5 {
            game[(i, 8)] = Some(Color::White);
        }
        for i in 0..6 {
            game[(i, 7)] = Some(Color::White);
        }
        for i in 4..7 {
            game[(i, 6)] = Some(Color::White);
        }

        game
    }

    pub fn get(&self, pos: impl Into<Pos>) -> Option<&Option<Color>> {
        let pos = pos.into();
        if !is_in_bounds(pos) {
            return None;
        }

        Some(&self[pos])
    }

    pub fn get_mut(&mut self, pos: impl Into<Pos>) -> Option<&mut Option<Color>> {
        let pos = pos.into();
        if !is_in_bounds(pos) {
            return None;
        }

        Some(&mut self[pos])
    }

    pub fn push(&self, first: Pos, dir: Dir) -> Result<Success, Error> {
        let mut force = 1;

        let Some(Some(color)) = self.get(first) else {
            return Err(Error::NotABall(first));
        };

        let opposing_first = loop {
            let p = first + dir.vec() * force;
            if let Some(c) = self.get(p) {
                if let Some(c) = c {
                    if c != color {
                        break p;
                    }
                    if force >= 4 {
                        return Err(Error::TooMany { first, fourth: p });
                    }
                    force += 1;
                } else {
                    let last = first + dir.vec() * (force - 1);
                    return Ok(Success::Moved { first, last });
                }
            } else {
                return Err(Error::PushedOff(p));
            }
        };

        let opposing_color = color.opposite();
        let mut opposing_force = 1;
        loop {
            let p = opposing_first + dir.vec() * opposing_force;
            if let Some(&c) = self.get(p) {
                if let Some(c) = c {
                    if c != opposing_color {
                        return Err(Error::Mixed(p));
                    }
                    if opposing_force >= force {
                        return Err(Error::TooManyOpposing {
                            first: opposing_first,
                            last: p,
                        });
                    }
                    opposing_force += 1;
                } else {
                    let last = opposing_first + dir.vec() * (force - 1);
                    return Ok(Success::PushedAway { first, last });
                }
            } else {
                let last = opposing_first + dir.vec() * (force - 1);
                return Ok(Success::PushedOff { first, last });
            }
        }
    }
}

fn is_in_bounds(pos: impl Into<Pos>) -> bool {
    let Pos { x, y } = pos.into();
    x >= 0 && x < SIZE && y >= 0 && y < SIZE && x - y < 5 && y - x < 5
}

fn main() {
    let game = Game::new();

    println!("{:#?}", game);
}
