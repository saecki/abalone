use crate::{Dir, Error, Game, Pos2, Success};

struct CheckState {
    game: Game,
}

fn start() -> CheckState {
    CheckState { game: Game::new() }
}

impl CheckState {
    fn check_push(
        mut self,
        first: impl Into<Pos2>,
        dir: Dir,
        expected: Result<Success, Error>,
    ) -> Self {
        let res = self.game.can_push(first.into(), dir);
        if let Ok(s) = &res {
            println!("{}", self.game);
            self.game.apply(s);
        }
        assert_eq!(res, expected, "\n{}", self.game);
        self
    }
}

#[test]
fn smooth_operator() {
    start().check_push(
        (0, 0),
        Dir::PosZ,
        Ok(Success::Moved {
            first: (0, 0).into(),
            last: (2, 2).into(),
        }),
    );
}

#[test]
fn too_many_opposing() {
    start()
        .check_push(
            (0, 0),
            Dir::PosZ,
            Ok(Success::Moved {
                first: (0, 0).into(),
                last: (2, 2).into(),
            }),
        )
        .check_push(
            (1, 1),
            Dir::PosZ,
            Ok(Success::Moved {
                first: (1, 1).into(),
                last: (3, 3).into(),
            }),
        )
        .check_push(
            (2, 2),
            Dir::PosZ,
            Ok(Success::Moved {
                first: (2, 2).into(),
                last: (4, 4).into(),
            }),
        )
        .check_push(
            (3, 3),
            Dir::PosZ,
            Err(Error::TooManyOpposing {
                first: (6, 6).into(),
                last: (8, 8).into(),
            }),
        );
}
