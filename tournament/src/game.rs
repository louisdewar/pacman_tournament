use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};

use model::{Bucket, GameEvent, Model, NetworkMessage};

pub struct Player {
    username: String,
    best_score: usize,
}

/// Manages all the games on the server
pub struct GlobalManager {
    // In future this will be true, but for now we're not using a database for scores
    // /// All the players currently in the game (important for live scoreboard updates without
    // /// querying the database each time).
    /// All players that have logged on at some point (doesn't necessarily have to be alive
    /// players).
    /// The id here is the **global** user id, it will not be the same id as the one generated when
    /// a user joins a game.
    players: HashMap<usize, Player>,
    games: Bucket<LocalManager>,
}

/// Manages a single game
struct LocalManager {
    model: Model<Box<Fn(GameEvent)>>,
    rx: Receiver<NetworkMessage>,
}

#[derive(Clone, Debug)]
pub enum AuthenticationFailedReason {
    PlayerInGame,
    BadCode,
}

pub enum ManagerEvents {
    Authenticated {
        temporary_id: usize,
        id: usize,
    },
    BadAuthentication {
        temporary_id: usize,
        reason: AuthenticationFailedReason,
    },
    NewGame {
        id: usize,
        rx: Receiver<GameEvent>,
        tx: Sender<NetworkMessage>,
    },
    GameClosed {
        id: usize,
    },
}
