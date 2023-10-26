use crate::{Dir, Error, Game, Pos, Success};

struct CheckState {
    game: Game,
}

fn start() -> CheckState {
    CheckState { game: Game::new() }
}

impl CheckState {
    fn check_push(self, first: impl Into<Pos>, dir: Dir, expected: Result<Success, Error>) -> Self {
        let res = self.game.push(first.into(), dir);
        assert_eq!(res, expected);
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
