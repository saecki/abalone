use crate::{Abalone, Dir, Error, MoveError, Pos2, SelectionError, Success, Vec2};

struct CheckState {
    game: Abalone,
}

fn start() -> CheckState {
    CheckState {
        game: Abalone::new(),
    }
}

impl CheckState {
    fn check_move(
        mut self,
        first: impl Into<Pos2> + Copy,
        last: impl Into<Pos2> + Copy,
        dir: Dir,
        expected: Result<Success, Error>,
    ) -> Self {
        let mut res = self.game.check_move([first.into(), last.into()], dir);
        if let Err(Error::Selection(SelectionError::WrongTurn(_))) = res {
            self.game.turn = self.game.turn.opposite();
            res = self.game.check_move([first.into(), last.into()], dir);
        }
        if let Ok(s) = res {
            println!("{}", self.game);
            self.game.submit_move(s);
        }
        assert_eq!(res, expected, "\n{}", self.game);
        self
    }

    fn assert_move(
        mut self,
        first: impl Into<Pos2> + Copy,
        last: impl Into<Pos2> + Copy,
        dir: Dir,
    ) -> Self {
        let mut res = self.game.check_move([first.into(), last.into()], dir);
        if let Err(Error::Selection(SelectionError::WrongTurn(_))) = res {
            self.game.turn = self.game.turn.opposite();
            res = self.game.check_move([first.into(), last.into()], dir);
        }
        if let Ok(s) = res {
            println!("{}", self.game);
            self.game.submit_move(s);
        }
        assert_eq!(res.err(), None, "\n{}", self.game);
        self
    }
}

#[test]
fn smooth_operator() {
    start().check_move(
        (0, 0),
        (2, 2),
        Dir::PosZ,
        Ok(Success::Moved {
            dir: Dir::PosZ,
            first: (0, 0).into(),
            last: (2, 2).into(),
        }),
    );
}

#[test]
fn too_many_opposing() {
    start()
        .assert_move((0, 0), (2, 2), Dir::PosZ)
        .assert_move((1, 1), (3, 3), Dir::PosZ)
        .assert_move((2, 2), (4, 4), Dir::PosZ)
        .check_move(
            (3, 3),
            (5, 5),
            Dir::PosZ,
            Err(Error::Move(MoveError::TooManyOpposing {
                first: (6, 6).into(),
                last: (8, 8).into(),
            })),
        );
}

#[test]
fn sideward_move() {
    start().check_move(
        (2, 2),
        (4, 2),
        Dir::PosY,
        Ok(Success::Moved {
            dir: Dir::PosY,
            first: (2, 2).into(),
            last: (4, 2).into(),
        }),
    );
}

#[test]
fn sideward_blocked_by_own() {
    start().check_move(
        (2, 2),
        (4, 2),
        Dir::NegY,
        Err(Error::Move(MoveError::NotFree(
            [(2, 1).into(), (3, 1).into(), (4, 1).into()].into(),
        ))),
    );
}

#[test]
fn sideward_not_free() {
    start()
        .assert_move((2, 2), (4, 2), Dir::PosZ)
        .assert_move((3, 3), (5, 3), Dir::PosY)
        .assert_move((3, 4), (5, 4), Dir::PosZ)
        .check_move(
            (4, 5),
            (6, 5),
            Dir::PosY,
            Err(Error::Move(MoveError::NotFree(
                [(4, 6).into(), (5, 6).into(), (6, 6).into()].into(),
            ))),
        );
}

#[test]
fn mixed_set_forward_motion() {
    start()
        .assert_move((0, 0), (2, 2), Dir::PosZ)
        .assert_move((1, 1), (3, 3), Dir::PosZ)
        .assert_move((2, 2), (4, 4), Dir::PosZ)
        .check_move(
            (5, 5),
            (7, 7),
            Dir::PosZ,
            Err(Error::Selection(SelectionError::MixedSet(
                [(6, 6).into(), (7, 7).into()].into(),
            ))),
        );
}

#[test]
fn one_vs_one_push_off() {
    start()
        .assert_move((0, 1), (0, 1), Dir::PosZ)
        .assert_move((1, 2), (1, 2), Dir::PosY)
        .assert_move((1, 3), (1, 3), Dir::PosZ)
        .assert_move((2, 4), (2, 4), Dir::PosY)
        .assert_move((2, 5), (2, 5), Dir::PosZ)
        .check_move(
            (3, 6),
            (3, 6),
            Dir::PosY,
            Err(Error::Move(MoveError::TooManyOpposing {
                first: (3, 7).into(),
                last: (3, 7).into(),
            })),
        );
}

#[test]
fn one_vs_one_push_away() {
    start()
        .assert_move((0, 1), (0, 1), Dir::PosZ)
        .assert_move((1, 2), (1, 2), Dir::PosY)
        .assert_move((1, 3), (1, 3), Dir::PosZ)
        .assert_move((2, 4), (2, 4), Dir::PosY)
        .assert_move((2, 5), (2, 5), Dir::PosZ)
        .check_move(
            (3, 7),
            (3, 7),
            Dir::NegY,
            Err(Error::Move(MoveError::TooManyOpposing {
                first: (3, 6).into(),
                last: (3, 6).into(),
            })),
        );
}

#[test]
fn parallel() {
    fn check(a: impl Into<Vec2>, b: impl Into<Vec2>) {
        let a = a.into();
        let b = b.into();
        assert!(a.is_parallel(b), "{:?} and {:?} not parallel", a, b);
    }

    fn check_not(a: impl Into<Vec2>, b: impl Into<Vec2>) {
        let a = a.into();
        let b = b.into();
        assert!(!a.is_parallel(b), "{:?} and {:?} parallel", a, b);
    }

    check((3, 2), (6, 4));
    check((10, 4), (5, 2));
    check((-4, 2), (8, -4));
    check((-4, -2), (8, 4));
    check((0, 0), (0, 0));

    check_not((1, 2), (2, 1));
    check_not((3, 2), (2, 1));
}
