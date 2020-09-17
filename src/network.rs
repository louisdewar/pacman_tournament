use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::io::Write;
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::mpsc::{channel, Receiver, Sender};

use serde::{Deserialize, Serialize};

use crate::{Action, BaseTile, Bucket, Direction, Entity, EntityIndex, Map, Mob, Player};

pub struct NetworkManager {
    listener: TcpListener,
    clients: HashMap<usize, TcpStream>,
    unallocated_clients: Bucket<TcpStream>,
    tx: Sender<NetworkMessage>,
    rx: Receiver<GameMessage>,
}

impl NetworkManager {
    pub fn start<A: ToSocketAddrs>(
        addr: A,
    ) -> io::Result<(Sender<GameMessage>, Receiver<NetworkMessage>)> {
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
            let temporary_id = self.unallocated_clients.add(stream);
            self.tx
                .send(NetworkMessage::ClientConnect { temporary_id })
                .expect("Transmitter error");
        }
    }

    fn handle_incoming_data(&mut self) {
        use std::io::{ErrorKind, Read};
        let mut to_remove = Vec::new();
        for (player_id, stream) in self.clients.iter_mut() {
            let mut buf = [0_u8; 512];
            match stream.read(&mut buf) {
                Ok(n_read) => {
                    if n_read == 0 {
                        to_remove.push(*player_id);
                        continue;
                    }

                    // if buf[1] != b'\n' {
                    //     println!(
                    //         "Invalid message format (newline) `{}` from {}",
                    //         String::from_utf8_lossy(&buf[0..n_read]),
                    //         player_id
                    //     );
                    //     continue;
                    // }

                    let action = match buf[0] {
                        b'F' => Action::Forward,
                        b'L' => Action::TurnLeft,
                        b'R' => Action::TurnRight,
                        b'E' => Action::Eat,
                        b'S' => Action::Stay,
                        _ => {
                            println!(
                                "Invalid message char `{}` from {}",
                                String::from_utf8_lossy(&buf[0..n_read]),
                                player_id
                            );
                            continue;
                        }
                    };

                    self.tx
                        .send(NetworkMessage::PlayerAction {
                            id: *player_id,
                            action,
                        })
                        .unwrap();
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => {}
                Err(err) => {
                    println!("Error reading from socket of player {}: {}", player_id, err);
                    to_remove.push(*player_id);
                }
            }
        }

        for id in to_remove {
            println!("Disconnecting {}", id);
            self.tx
                .send(NetworkMessage::ClientDisconnect { id })
                .unwrap();
            self.clients.remove(&id);
        }
    }

    fn handle_game_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            use GameMessage::*;
            match msg {
                PlayerSpawned { temporary_id, id } => {
                    let stream = self
                        .unallocated_clients
                        .remove(temporary_id)
                        .expect("Temporary id didn't exist");
                    self.clients.insert(id, stream);
                }
                ProcessTick {
                    map,
                    entities,
                    players,
                    mobs,
                } => {
                    for (player_id, player) in players.iter() {
                        let player = player.borrow();
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

                            if new_x < 0
                                || new_y < 0
                                || new_x >= map.width as i32
                                || new_y >= map.height as i32
                            {
                                return TileView {
                                    base: BaseTile::Wall,
                                    mob: None,
                                    player: None,
                                };
                            }

                            let (x, y) = (new_x as u16, new_y as u16);

                            let (mob, player) = if let Some(entity_index) =
                                &entities[map.flatten_coordinate(x as usize, y as usize)]
                            {
                                use crate::entity::EntityType;
                                match entity_index.entity_type {
                                    EntityType::Mob => (
                                        Some(mobs.get(entity_index.index).unwrap().borrow().into()),
                                        None,
                                    ),
                                    EntityType::Player => {
                                        let player =
                                            players.get(entity_index.index).unwrap().borrow();
                                        let player_view = PlayerView {
                                            direction: player.direction(),
                                            health: player.health(),
                                            is_invulnerable: player.is_invulnerable(),
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
                                player,
                                mob,
                            }
                        };

                        let view = [
                            [c(-1, 2), c(-1, 1), c(-1, 0), c(-1, -1)],
                            [c(0, 2), c(0, 1), c(0, 0), c(0, -1)],
                            [c(1, 2), c(1, 1), c(1, 0), c(1, -1)],
                        ];

                        if let Some(mut stream) = self.clients.get_mut(player_id) {
                            if let Err(e) = serde_json::to_writer(&mut stream, &view) {
                                println!("Failed to serialize: {}", e);
                            }
                            let _ = stream.write(b"\n");
                        } else {
                            println!(
                                "Player id {} didn't have a stream but was in the update tick",
                                player_id
                            );
                        }
                    }
                }
                GameMessage::PlayerDied {
                    player_id,
                    final_score,
                } => {
                    let mut stream = self.clients.remove(&player_id).unwrap();
                    if let Err(e) =
                        serde_json::to_writer(&mut stream, &PlayerDiedMessage { final_score })
                    {
                        println!("Failed to serialize: {}", e);
                    }
                    let _ = stream.write(b"\n");
                    println!(
                        "Player {} disconnected with final score {}",
                        player_id, final_score
                    );
                }
            }
        }
    }
}

#[derive(Serialize, Debug)]
struct PlayerView {
    direction: Direction,
    health: u8,
    is_invulnerable: bool,
    is_current_player: bool,
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
}

#[derive(Serialize)]
struct PlayerDiedMessage {
    final_score: usize,
}

pub enum GameMessage {
    PlayerSpawned {
        temporary_id: usize,
        id: usize,
    },
    ProcessTick {
        map: Map,
        entities: Vec<Option<EntityIndex>>,
        players: Bucket<RefCell<Player>>,
        mobs: Bucket<RefCell<Mob>>,
    },
    PlayerDied {
        player_id: usize,
        final_score: usize,
    },
}

pub enum NetworkMessage {
    ClientConnect { temporary_id: usize },
    ClientDisconnect { id: usize },
    PlayerAction { id: usize, action: Action },
}
