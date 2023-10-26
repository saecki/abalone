use crate::{Dir, Error, Game, Pos2, Success};

struct CheckState {
    game: Game,
}

fn start() -> CheckState {
    CheckState { game: Game::new() }
}

impl CheckState {
    fn check_move(
        mut self,
        first: impl Into<Pos2>,
        last: impl Into<Pos2>,
        dir: Dir,
        expected: Result<Success, Error>,
    ) -> Self {
        let res = self.game.check_move(first.into(), last.into(), dir);
        if let Ok(s) = &res {
            println!("{}", self.game);
            self.game.apply_move(s);
        }
        assert_eq!(res, expected, "\n{}", self.game);
        self
    }
    fn assert_move(mut self, first: impl Into<Pos2>, last: impl Into<Pos2>, dir: Dir) -> Self {
        let res = self.game.check_move(first.into(), last.into(), dir);
        if let Ok(s) = &res {
            println!("{}", self.game);
            self.game.apply_move(s);
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
            Err(Error::TooManyOpposing {
                first: (6, 6).into(),
                last: (8, 8).into(),
            }),
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
        Err(Error::NotFree(
            [(2, 1).into(), (3, 1).into(), (4, 1).into()].into(),
        )),
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
            Err(Error::NotFree(
                [(4, 6).into(), (5, 6).into(), (6, 6).into()].into(),
            )),
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
            Err(Error::MixedSet([(6, 6).into(), (7, 7).into()].into())),
        );
}
