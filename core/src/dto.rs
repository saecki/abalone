use serde_derive::{Deserialize, Serialize};

use crate::{Abalone, Dir, Move, Pos2};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientMsg {
    /// Create a new room
    CreateRoom(String),
    /// Request a list of rooms.
    ListRooms,
    /// Request to join the room.
    RequestJoinRoom(RoomId),
    /// Allow someone to join the room.
    AllowJoinRoom(TransactionId),
    /// Allow someone to join the room.
    JoinRoom(RoomId, TransactionId),
    /// Leave a room.
    LeaveRoom,
    /// Request a sync message from the server.
    Sync,
    /// Make a move.
    MakeMove { first: Pos2, last: Pos2, dir: Dir },
    /// Request the opponent to undo the last move.
    RequestUndo,
    /// Allow the opponent to undo the last move.
    AllowUndo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerMsg {
    /// Connection acknowledge
    Welcome(User),
    /// List of open rooms.
    OpenRooms(Vec<OpenRoom>),
    /// Someone requested to join the room.
    JoinRoomRequested(TransactionId),
    /// Someone allowed you to join their room.
    JoinRoomAllowed(OpenRoom, TransactionId),
    /// Synchronize game state.
    Sync(Room),
    /// Synchronize game state, but there isn't any.
    SyncEmpty,
    /// A move was made.
    AppliedMove(Move),
    /// An undo was requested by the opponent.
    UndoRequested,
    /// An error occurred.
    Error(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Room {
    pub id: RoomId,
    pub name: String,
    pub game: Abalone,
    pub players: [Option<User>; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpenRoom {
    pub id: RoomId,
    pub name: String,
    pub players: [Option<User>; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TransactionId(pub uuid::Uuid);

impl std::fmt::Display for TransactionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.simple().fmt(f)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RoomId(pub u64);

impl std::fmt::Display for RoomId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UserId(pub uuid::Uuid);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.simple().fmt(f)
    }
}
