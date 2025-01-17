use std::collections::HashMap;
use std::io;
use std::io::Write;
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::mpsc::{channel, Receiver, Sender};

use serde::{Deserialize, Serialize};

use crate::{Action, BaseTile, Bucket, Direction, Entity, Food, GameData, GameEvent, Mob};

/// Useful building block for custom network managers
/// It has a buffer of 1024 bytes with a TCP stream.
/// It will take incoming messages and fill them into the buffer producing messages
/// when it finds a '\n', if the buffer fills up before finding and newline then
/// it empties the buffer and logs an error.
pub struct Connection {
    stream: TcpStream,
    buf: [u8; 1024],
    buf_len: usize,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Connection {
        Connection {
            stream,
            buf: [0; 1024],
            buf_len: 0,
        }
    }

    /// First returns the oldest message (from start to newline (exclusive))
    /// by deserialising the bytes.
    /// If there is no message / an incomplete one it tries to fill the buffer as
    /// much as possible (only then next call of this method will deserialize the
    /// message in the buffer.
    /// This is non-blocking if the provided TcpStream is non-blocking.
    ///
    /// Returns an error if there is an issue reading from the stream (i.e. it is probably
    /// disconnected). In the event the message is malformatted it will log and return Ok(None).
    pub fn next_message<T: serde::de::DeserializeOwned>(
        &mut self,
        player_id: String,
    ) -> Result<Option<T>, ()> {
        use std::io::{ErrorKind, Read};
        let mut msg_end = None;
        for (i, byte) in self.buf.iter().enumerate() {
            if *byte == b'\n' {
                msg_end = Some(i);
                break;
            }
        }

        if let Some(end_index) = msg_end {
            let mut new_buf = [0_u8; 1024];
            &mut new_buf[0..(self.buf_len - end_index - 1)]
                .copy_from_slice(&self.buf[end_index + 1..self.buf_len]);
            std::mem::swap(&mut self.buf, &mut new_buf);
            self.buf_len -= end_index + 1;
            match serde_json::from_slice::<T>(&new_buf[0..end_index]) {
                Ok(value) => {
                    return Ok(Some(value));
                }
                Err(_err) => {
                    println!(
                        "Player {} sent malformatted message `{}`",
                        player_id,
                        String::from_utf8_lossy(&new_buf[0..end_index])
                    );
                }
            }
        } else {
            if self.buf_len == 1024 {
                // Message is over 1024 bytes (no \n found) it is probably malformatted so drop
                // it
                self.buf = [0; 1024];
                self.buf_len = 0;
                println!(
                    "Player {} filled the message buffer without a complete message (`{}`)",
                    player_id,
                    String::from_utf8_lossy(&self.buf)
                );
            }
            match self.stream.read(&mut self.buf[self.buf_len..]) {
                Ok(n_read) => {
                    if n_read == 0 {
                        return Err(());
                    }

                    self.buf_len += n_read;
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => {}
                Err(err) => {
                    println!("Error reading from socket of player {}: {}", player_id, err);
                    return Err(());
                }
            }
        }

        Ok(None)
    }
}

impl From<TcpStream> for Connection {
    fn from(stream: TcpStream) -> Connection {
        Connection {
            stream,
            buf: [0; 1024],
            buf_len: 0,
        }
    }
}

pub struct NetworkManager {
    listener: TcpListener,
    clients: HashMap<usize, Connection>,
    unallocated_clients: Bucket<Connection>,
    tx: Sender<NetworkMessage>,
    rx: Receiver<GameEvent>,
}

impl NetworkManager {
    pub fn start<A: ToSocketAddrs>(
        addr: A,
    ) -> io::Result<(Sender<GameEvent>, Receiver<NetworkMessage>)> {
        let (client_tx, server_rx) = channel();
        let (server_tx, client_rx) = channel();

        let mut manager = NetworkManager {
            listener: TcpListener::bind(addr)?,
            clients: HashMap::new(),
            unallocated_clients: Bucket::new(),
            tx: server_tx,
            rx: server_rx,
        };

        manager.listener.set_nonblocking(true)?;

        std::thread::spawn(move || loop {
            manager.process_accept_requests();
            manager.handle_incoming_data();
            manager.handle_game_messages();
        });

        Ok((client_tx, client_rx))
    }

    fn process_accept_requests(&mut self) {
        while let Ok((stream, addr)) = self.listener.accept() {
            println!("Incoming connection on {}", addr);
            stream
                .set_nonblocking(true)
                .expect("Couldn't set non-blocking on TCP connection");
            self.unallocated_clients.add(stream.into());
        }
    }

    fn handle_incoming_data(&mut self) {
        let mut clients_to_remove = Vec::new();
        let mut unallocated_to_remove = Vec::new();

        for (player_id, conn) in self.clients.iter_mut() {
            match conn.next_message::<ActionMessage>(format!("{}", player_id)) {
                Ok(Some(msg)) => {
                    self.tx
                        .send(NetworkMessage::PlayerAction {
                            id: *player_id,
                            action: msg.action,
                            tick: msg.tick,
                        })
                        .unwrap();
                }
                Ok(None) => {}
                Err(_) => clients_to_remove.push(*player_id),
            }
        }

        for (temporary_id, conn) in self.unallocated_clients.iter_mut() {
            match conn.next_message::<ClientConnectMessage>(format!("[temp_id] {}", temporary_id)) {
                Ok(Some(msg)) => {
                    self.tx
                        .send(NetworkMessage::ClientConnect {
                            temporary_id: *temporary_id,
                            username: msg.username,
                        })
                        .expect("Transmitter error");
                }
                Ok(None) => {}
                Err(_) => unallocated_to_remove.push(*temporary_id),
            }
        }

        for id in clients_to_remove {
            println!("Disconnecting {}", id);
            self.tx
                .send(NetworkMessage::ClientDisconnect { id })
                .unwrap();
            self.clients.remove(&id);
        }

        for id in unallocated_to_remove {
            println!("Disconnecting temp user {}", id);
            self.unallocated_clients.remove(id);
        }
    }

    fn handle_game_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            use GameEvent::*;
            match msg {
                PlayerSpawned { temporary_id, id } => {
                    let stream = self
                        .unallocated_clients
                        .remove(temporary_id)
                        .expect("Temporary id didn't exist");
                    self.clients.insert(id, stream);
                }
                ProcessTick { game_data, tick } => {
                    for (player_id, _) in game_data.players.iter() {
                        if let Some(conn) = self.clients.get_mut(player_id) {
                            let tick_msg = create_tick_message(&game_data, *player_id, tick);
                            let mut msg = serde_json::to_string(&tick_msg)
                                .expect("Couldn't serialize message");
                            msg.push('\n');

                            if let Err(e) = conn.stream.write(&msg.as_bytes()) {
                                println!("Failed to serialize: {}", e);
                            }
                        } else {
                            println!(
                                "Player id {} didn't have a stream but was in the update tick",
                                player_id
                            );
                        }
                    }
                }
                GameEvent::PlayerDied {
                    player_id,
                    final_score,
                } => {
                    let mut conn = self.clients.remove(&player_id).unwrap();
                    if let Err(e) =
                        serde_json::to_writer(&mut conn.stream, &PlayerDiedMessage { final_score })
                    {
                        println!("Failed to serialize: {}", e);
                    }
                    let _ = conn.stream.write(b"\n");
                    println!(
                        "Player {} disconnected with final score {}",
                        player_id, final_score
                    );
                }
            }
        }
    }
}

pub fn create_tick_message(game_data: &GameData, player_id: usize, tick: u32) -> TickMessage {
    let GameData {
        players,
        mobs,
        map,
        food,
        entities,
    } = game_data;

    let player = players.get(player_id).unwrap().borrow();
    let (player_x, player_y) = player.position();
    let player_x = player_x as i32;
    let player_y = player_y as i32;
    let direction = player.direction();

    let c = |x: i32, y: i32| {
        let is_current_player = x == 0 && y == 0;
        use Direction::*;
        let (new_x, new_y) = match direction {
            North => (player_x + x, player_y - y),
            East => (player_x + y, player_y + x),
            South => (player_x - x, player_y + y),
            West => (player_x - y, player_y - x),
        };

        if new_x < 0 || new_y < 0 || new_x >= map.width() as i32 || new_y >= map.height() as i32 {
            return TileView {
                base: BaseTile::Wall,
                mob: None,
                player: None,
                food: None,
            };
        }

        let (x, y) = (new_x as u16, new_y as u16);

        let (mob, player) = if let Some(entity_index) = &entities[x as usize][y as usize] {
            use crate::entity::EntityType;
            match entity_index.entity_type() {
                EntityType::Mob => (
                    Some(mobs.get(entity_index.index).unwrap().borrow().into()),
                    None,
                ),
                EntityType::Player => {
                    let player = players.get(entity_index.index).unwrap().borrow();
                    let player_view = PlayerView {
                        direction: player.direction(),
                        health: player.health(),
                        is_invulnerable: player.is_invulnerable(),
                        has_powerpill: player.has_powerpill(),
                        score: player.score(),
                        username: player.username().clone(),
                        is_current_player,
                    };
                    (None, Some(player_view))
                }
            }
        } else {
            (None, None)
        };

        TileView {
            base: map.base_tile(x as usize, y as usize).clone(),
            food: food[x as usize][y as usize].clone(),
            player,
            mob,
        }
    };

    let view = [
        [c(-1, 2), c(-1, 1), c(-1, 0), c(-1, -1)],
        [c(0, 2), c(0, 1), c(0, 0), c(0, -1)],
        [c(1, 2), c(1, 1), c(1, 0), c(1, -1)],
    ];

    TickMessage { view, tick }
}

#[derive(Serialize, Debug)]
struct PlayerView {
    direction: Direction,
    health: u8,
    has_powerpill: bool,
    is_invulnerable: bool,
    is_current_player: bool,
    score: u32,
    username: String,
}

#[derive(Serialize, Debug)]
struct MobView {
    direction: Direction,
}

impl<M: std::ops::Deref<Target = Mob>> From<M> for MobView {
    fn from(mob: M) -> MobView {
        MobView {
            direction: mob.direction(),
        }
    }
}

#[derive(Serialize, Debug)]
struct TileView {
    base: BaseTile,
    player: Option<PlayerView>,
    mob: Option<MobView>,
    food: Option<Food>,
}

#[derive(Serialize)]
pub struct TickMessage {
    view: [[TileView; 4]; 3],
    tick: u32,
}

#[derive(Serialize)]
pub struct PlayerDiedMessage {
    pub final_score: u32,
}

#[derive(Deserialize)]
/// Represents a message that a client might send to indicate their action for a particular tick
pub struct ActionMessage {
    pub tick: u32,
    pub action: Action,
}

#[derive(Deserialize)]
struct ClientConnectMessage {
    username: String,
}

/// An event produced by the front end (typically network manager)
pub enum NetworkMessage {
    ClientConnect {
        temporary_id: usize,
        username: String,
    },
    ClientDisconnect {
        id: usize,
    },
    PlayerAction {
        id: usize,
        action: Action,
        tick: u32,
    },
}
