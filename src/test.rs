use crate::{Dir, Error, Game, Pos2, Success};

struct CheckState {
    game: Game,
}

fn start() -> CheckState {
    CheckState { game: Game::new() }
}

impl CheckState {
    fn assert_move(
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
}

#[test]
fn smooth_operator() {
    start().assert_move(
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
        .assert_move(
            (0, 0),
            (2, 2),
            Dir::PosZ,
            Ok(Success::Moved {
                dir: Dir::PosZ,
                first: (0, 0).into(),
                last: (2, 2).into(),
            }),
        )
        .assert_move(
            (1, 1),
            (3, 3),
            Dir::PosZ,
            Ok(Success::Moved {
                dir: Dir::PosZ,
                first: (1, 1).into(),
                last: (3, 3).into(),
            }),
        )
        .assert_move(
            (2, 2),
            (5, 5),
            Dir::PosZ,
            Ok(Success::Moved {
                dir: Dir::PosZ,
                first: (2, 2).into(),
                last: (4, 4).into(),
            }),
        )
        .assert_move(
            (3, 3),
            (6, 6),
            Dir::PosZ,
            Err(Error::TooManyOpposing {
                first: (6, 6).into(),
                last: (8, 8).into(),
            }),
        );
}

#[test]
fn sideward() {
    start().assert_move(
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
