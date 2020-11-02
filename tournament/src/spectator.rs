use std::net::TcpStream;
use tungstenite::WebSocket;

use model::Bucket;

enum ListenFilter {
    /// All games + top 10 leaderboard
    AllGames,
    /// A specific game (given by the game id)
    Game(usize),
}

struct Spectator {
    socket: WebSocket<TcpStream>,
    filter: ListenFilter,
}

/// Manages connections to the websocket clients (spectators)
pub struct Manager {
    spectators: Bucket<Spectator>,
}
