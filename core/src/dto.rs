use serde_derive::{Deserialize, Serialize};

use crate::{Abalone, Dir, Move, Pos2};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientMsg {
    /// Create a new room
    CreateRoom(String),
    /// Request a list of rooms.
    ListRooms,
    /// Request a sync message from the server.
    Sync,
    /// Join a room.
    Join(u64),
    /// Leave a room.
    Leave,
    /// Make a move.
    MakeMove { first: Pos2, last: Pos2, dir: Dir },
    /// Request the opponent to undo the last move.
    RequestUndo,
    /// Allow the opponent to undo the last move.
    AllowUndo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerMsg {
    /// List of open rooms.
    OpenRooms(Vec<Room>),
    /// Synchronize game state.
    Sync(Sync),
    /// Synchronize game state, but there isn't any.
    SyncEmpty,
    /// A move was made.
    UpdateMove(Move),
    /// An undo was requested by the opponent.
    UndoRequested,
    /// An error occurred.
    Error(),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sync {
    pub room: Room,
    pub game: Abalone,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Room {
    pub id: u64,
    pub name: String,
}
