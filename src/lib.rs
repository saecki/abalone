use std::{fmt, ops};

use crate::stackvec::StackVec;

pub mod stackvec;
#[cfg(test)]
mod test;

const UNIT_X: Vec2 = Vec2 { x: 1, y: 0 };
const UNIT_Y: Vec2 = Vec2 { x: 0, y: 1 };
const UNIT_Z: Vec2 = Vec2 { x: 1, y: 1 };

const SIZE: i8 = 9;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Success {
    /// Pushed opposing color, off the board.
    PushedOff {
        /// First ball that was pushed.
        first: Pos2,
        /// Last opposing ball that was pushed off.
        last: Pos2,
    },
    /// Pushed opposing color, but not off the board.
    PushedAway {
        /// First ball that was pushed.
        first: Pos2,
        /// Last opposing ball that was pushed away.
        last: Pos2,
    },
    /// Moved without resistance.
    Moved {
        dir: Dir,
        /// First ball that was pushed.
        first: Pos2,
        /// Last ball, of the same color, that was pushed.
        last: Pos2,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Selection(SelectionError),
    Move(MoveError),
}

impl From<SelectionError> for Error {
    fn from(value: SelectionError) -> Self {
        Self::Selection(value)
    }
}

impl From<MoveError> for Error {
    fn from(value: MoveError) -> Self {
        Self::Move(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectionError {
    /// It's the other color's turn.
    WrongTurn(Pos2),
    /// The first and last balls span an invalid set of balls, e.g. the vector
    /// last - first isn't a multiple of a unit vector (X, Y, Z).
    InvalidSet,
    /// The first and last balls span a mixed colored set of balls.
    /// the position is the color of the first offending ball.
    MixedSet(StackVec<2, Pos2>),
    /// No ball was found ad the position.
    NotABall(Pos2),
    /// More than 3 balls.
    TooMany,
    /// No matter what direction, there isn't any valid move.
    NoPossibleMove,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MoveError {
    /// Would push off your own ball.
    PushedOff(Pos2),
    /// A ball off your own color, is blocking you from pushing opposing balls.
    BlockedByOwn(Pos2),
    /// More than 3 balls of the same color were inferred,
    /// e.g. in the same direction.
    TooManyInferred {
        /// First opposing ball.
        first: Pos2,
        /// Last opposing ball.
        last: Pos2,
    },
    /// More or the same amount of opposing balls.
    TooManyOpposing {
        /// First opposing ball.
        first: Pos2,
        /// Last opposing ball.
        last: Pos2,
    },
    /// Field isn't free, only for sideward motion.
    NotFree(StackVec<3, Pos2>),
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
/// ```md
///               0 1 2 3 4 5 6 7 8
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
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pos2 {
    pub x: i8,
    pub y: i8,
}

impl<'a, 'b> ops::Add<Vec2> for Pos2 {
    type Output = Pos2;

    fn add(self, rhs: Vec2) -> Self::Output {
        Self::Output {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl<'a, 'b> ops::Sub<Pos2> for Pos2 {
    type Output = Vec2;

    fn sub(self, rhs: Pos2) -> Self::Output {
        Self::Output {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl<'a, 'b> ops::Sub<Vec2> for Pos2 {
    type Output = Pos2;

    fn sub(self, rhs: Vec2) -> Self::Output {
        Self::Output {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl From<(i8, i8)> for Pos2 {
    fn from((x, y): (i8, i8)) -> Self {
        Self { x, y }
    }
}

impl Pos2 {
    pub const ZERO: Self = Self { x: 0, y: 0 };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vec2 {
    pub x: i8,
    pub y: i8,
}

impl<'a, 'b> ops::Neg for Vec2 {
    type Output = Vec2;

    fn neg(self) -> Self::Output {
        Self::Output {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl<'a> ops::Mul<i8> for Vec2 {
    type Output = Vec2;

    fn mul(self, rhs: i8) -> Self::Output {
        Self::Output {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl From<(i8, i8)> for Vec2 {
    fn from((x, y): (i8, i8)) -> Self {
        Self { x, y }
    }
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0, y: 0 };

    pub fn new(x: i8, y: i8) -> Self {
        Self { x, y }
    }

    /// Magnitude of the vector.
    ///
    /// NOTE: diagonals in the Z direction are also counted as length 1.
    pub fn mag(&self) -> i8 {
        if self.x.signum() == self.y.signum() {
            self.x.abs().max(self.y.abs())
        } else {
            self.x.abs() + self.y.abs()
        }
    }

    pub fn abs(&self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }

    pub fn norm(&self) -> Self {
        Self {
            x: self.x.signum(),
            y: self.y.signum(),
        }
    }

    pub fn is_multiple_of_unit_vec(&self) -> bool {
        self.x == 0 || self.y == 0 || self.x == self.y
    }

    pub fn is_parallel(&self, other: Vec2) -> bool {
        if *self == Vec2::ZERO || other == Vec2::ZERO {
            return *self == other;
        }

        self.x * other.y == self.y * other.x
    }

    pub fn unit_vec(&self) -> Option<Dir> {
        let dir = match *self {
            v if v == UNIT_X => Dir::PosX,
            v if v == -UNIT_X => Dir::NegX,
            v if v == UNIT_Y => Dir::PosY,
            v if v == -UNIT_Y => Dir::NegY,
            v if v == UNIT_Z => Dir::PosZ,
            v if v == -UNIT_Z => Dir::NegZ,
            _ => return None,
        };
        Some(dir)
    }
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
    pub fn vec(&self) -> Vec2 {
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

#[derive(Clone, Debug, PartialEq)]
pub struct Abalone {
    pub balls: [[Option<Color>; SIZE as usize]; SIZE as usize],
    pub turn: Color,
}

impl fmt::Display for Abalone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for y in 0..SIZE {
            for _ in 0..(SIZE - y) {
                write!(f, " ")?;
            }
            for x in 0..SIZE {
                match self[(x, y)] {
                    Some(Color::Black) => write!(f, " b")?,
                    Some(Color::White) => write!(f, " w")?,
                    None => write!(f, " .")?,
                }
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

impl<P: Into<Pos2>> ops::Index<P> for Abalone {
    type Output = Option<Color>;

    fn index(&self, index: P) -> &Self::Output {
        let Pos2 { x, y } = index.into();
        &self.balls[y as usize][x as usize]
    }
}

impl<P: Into<Pos2>> ops::IndexMut<P> for Abalone {
    fn index_mut(&mut self, index: P) -> &mut Self::Output {
        let Pos2 { x, y } = index.into();
        &mut self.balls[y as usize][x as usize]
    }
}

impl Abalone {
    /// Returns a new game with the default start position as shown below:
    ///
    /// ```md
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
    /// ```
    pub fn new() -> Self {
        let mut game = Self {
            balls: [[None; SIZE as usize]; SIZE as usize],
            turn: Color::White,
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

        for i in 4..9 {
            game[(i, 8)] = Some(Color::White);
        }
        for i in 3..9 {
            game[(i, 7)] = Some(Color::White);
        }
        for i in 4..7 {
            game[(i, 6)] = Some(Color::White);
        }

        game
    }

    pub fn get(&self, pos: impl Into<Pos2>) -> Option<&Option<Color>> {
        let pos = pos.into();
        if !is_in_bounds(pos) {
            return None;
        }

        Some(&self[pos])
    }

    pub fn get_mut(&mut self, pos: impl Into<Pos2>) -> Option<&mut Option<Color>> {
        let pos = pos.into();
        if !is_in_bounds(pos) {
            return None;
        }

        Some(&mut self[pos])
    }

    pub fn iter(&self) -> impl Iterator<Item = (i8, i8, Option<Color>)> + '_ {
        (0..SIZE * SIZE).filter_map(move |i| {
            let y = i / SIZE;
            let x = i % SIZE;
            let val = *self.get((x, y))?;
            Some((x, y, val))
        })
    }

    pub fn check_selection(&self, selection: [Pos2; 2]) -> Result<(), SelectionError> {
        let dirs = [
            Dir::PosX,
            Dir::PosY,
            Dir::PosZ,
            Dir::NegX,
            Dir::NegY,
            Dir::NegZ,
        ];
        for dir in dirs {
            match self.check_move(selection, dir) {
                Ok(_) => return Ok(()),
                Err(Error::Selection(e)) => return Err(e),
                Err(Error::Move(_)) => continue,
            }
        }
        Err(SelectionError::NoPossibleMove)
    }

    pub fn check_move(&self, [mut first, mut last]: [Pos2; 2], dir: Dir) -> Result<Success, Error> {
        let Some(&Some(color)) = self.get(first) else {
            return Err(SelectionError::NotABall(first).into());
        };
        if color != self.turn {
            return Err(SelectionError::WrongTurn(first).into());
        }

        let mut vec = last - first;
        let norm = if vec != Vec2::ZERO {
            let mut norm = vec.norm();
            if !vec.is_multiple_of_unit_vec() {
                return Err(SelectionError::InvalidSet.into());
            }

            // flip things if pushing in reverse direction
            if -norm == dir.vec() {
                (first, last) = (last, first);
                vec = -vec;
                norm = -norm
            }

            norm
        } else {
            dir.vec()
        };

        let mag = vec.mag();
        if mag >= 3 {
            return Err(SelectionError::TooMany.into());
        }

        if norm == dir.vec() {
            // forward motion
            let mut force = 1;
            let opposing_first = loop {
                let p = first + dir.vec() * force;
                match self.get(p) {
                    Some(&Some(c)) if c != color => {
                        if force < mag {
                            let mut mixed_set = StackVec::new();
                            mixed_set.push(p);
                            for i in force + 1..=mag {
                                let p = first + dir.vec() * i;
                                mixed_set.push(p);
                            }

                            return Err(SelectionError::MixedSet(mixed_set).into());
                        } else {
                            break p;
                        }
                    }
                    Some(Some(_)) => {
                        if force >= 3 {
                            return Err(MoveError::TooManyInferred { first, last: p }.into());
                        }
                        force += 1;
                    }
                    Some(None) => {
                        let last = first + dir.vec() * (force - 1);
                        return Ok(Success::Moved { dir, first, last });
                    }
                    None => {
                        let last = first + dir.vec() * (force - 1);
                        return Err(MoveError::PushedOff(last).into());
                    }
                }
            };

            let opposing_color = color.opposite();
            let mut opposing_force = 1;
            loop {
                let p = opposing_first + dir.vec() * opposing_force;
                match self.get(p) {
                    Some(&Some(c)) => {
                        if c != opposing_color {
                            return Err(MoveError::BlockedByOwn(p).into());
                        }
                        if opposing_force >= force - 1 {
                            return Err(MoveError::TooManyOpposing {
                                first: opposing_first,
                                last: p,
                            }
                            .into());
                        }
                        opposing_force += 1;
                    }
                    Some(None) => {
                        let last = opposing_first + dir.vec() * (opposing_force - 1);
                        return Ok(Success::PushedAway { first, last });
                    }
                    None => {
                        let last = opposing_first + dir.vec() * (opposing_force - 1);
                        return Ok(Success::PushedOff { first, last });
                    }
                }
            }
        } else {
            // sideward motion
            let mut mixed_set = StackVec::new();
            for i in 1..=mag {
                let p = first + norm * i;
                match self.get(p) {
                    Some(&Some(c)) if c != color => mixed_set.push(p),
                    Some(Some(_)) => (),
                    Some(None) | None => return Err(SelectionError::NotABall(p).into()),
                }
            }

            if !mixed_set.is_empty() {
                return Err(SelectionError::MixedSet(mixed_set).into());
            }

            let mut non_free = StackVec::new();
            for i in 0..=mag {
                let current_pos = first + norm * i;
                let new_pos = current_pos + dir.vec();
                match self.get(new_pos) {
                    Some(&Some(_)) => non_free.push(new_pos),
                    Some(None) => (),
                    None => return Err(MoveError::PushedOff(current_pos).into()),
                }
            }

            if !non_free.is_empty() {
                return Err(MoveError::NotFree(non_free).into());
            }

            Ok(Success::Moved { dir, first, last })
        }
    }

    pub fn apply_move(&mut self, success: &Success) {
        match success {
            &Success::PushedOff { first, last } => {
                let vec = last - first;
                let num = vec.mag();
                let norm = vec.norm();

                for i in (0..num).rev() {
                    let pos = first + norm * i;
                    let new = pos + norm;
                    self[new] = self[pos];
                }
                self[first] = None;
            }
            &Success::PushedAway { first, last } => {
                let vec = last - first;
                let num = vec.mag();
                let norm = vec.norm();

                for i in (0..=num).rev() {
                    let pos = first + norm * i;
                    let new = pos + norm;
                    self[new] = self[pos];
                }
                self[first] = None;
            }
            &Success::Moved { dir, first, last } => {
                let vec = last - first;
                let num = vec.mag();
                let norm = vec.norm();

                for i in (0..=num).rev() {
                    let pos = first + norm * i;
                    let new = pos + dir.vec();
                    self[new] = self[pos];
                    self[pos] = None;
                }
            }
        }

        self.turn = self.turn.opposite();
    }
}

pub fn is_in_bounds(pos: impl Into<Pos2>) -> bool {
    let Pos2 { x, y } = pos.into();
    x >= 0 && x < SIZE && y >= 0 && y < SIZE && x - y < 5 && y - x < 5
}
